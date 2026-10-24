<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { RouterLink, onBeforeRouteLeave, onBeforeRouteUpdate, useRouter } from 'vue-router'
import * as api from '../../services/ai'
import { getProject, isDesktop } from '../../services/projects'
import { emptyPublishing, type ModelProvider, type PublishingContent, type WritingVersion } from '../../types/ai'
import { useOperation } from '../../composables/useOperation'
import AppIcon from '../../components/AppIcon.vue'
import ProjectNavigation from '../../components/ProjectNavigation.vue'
import EditorTabs from '../../components/EditorTabs.vue'
import OperationFeedback from '../../components/OperationFeedback.vue'
const props = defineProps<{ id: string }>()
const router = useRouter()
const desktop = isDesktop()
const title = ref('')
const ready = ref(false)
const providers = ref<ModelProvider[]>([])
const providerId = ref('')
const versions = ref<WritingVersion[]>([])
const selected = ref<WritingVersion | null>(null)
const draft = ref(emptyPublishing())
const original = ref(JSON.stringify(draft.value))
const dirty = computed(() => JSON.stringify(draft.value) !== original.value)
const taskId = ref('')
const cancelling = ref(false)
const activeTab = ref('publishing')
const copiedField = ref('')
let copyTimer: ReturnType<typeof globalThis.setTimeout> | undefined
const tabs = [
  { value: 'publishing', label: '发布文案' },
  { value: 'audio', label: '音频提示词' },
  { value: 'cover', label: '封面素材' },
]
const { busy, error, message, run } = useOperation()
function accept(version: WritingVersion | null) {
  selected.value = version
  draft.value = { ...emptyPublishing(), ...version?.content }
  original.value = JSON.stringify(draft.value)
}
function canDiscard() { return !dirty.value || globalThis.confirm('当前内容尚未保存，确定放弃修改？') }
const canLeave = () => !busy.value && canDiscard()
onBeforeRouteLeave(canLeave)
onBeforeRouteUpdate(canLeave)
function beforeUnload(event: globalThis.BeforeUnloadEvent) { if (dirty.value || busy.value) event.preventDefault() }
globalThis.addEventListener('beforeunload', beforeUnload)
onBeforeUnmount(() => { globalThis.removeEventListener('beforeunload', beforeUnload); globalThis.clearTimeout(copyTimer) })
const load = () => run(async () => {
  const [project, models, saved] = await Promise.all([getProject(props.id), api.listProviders(), api.listVersions(props.id, 'publishing')])
  title.value = project.title
  providers.value = models.filter(p => p.isEnabled && p.modelKind === 'text')
  providerId.value = providers.value.find(p => p.isDefault)?.id ?? providers.value[0]?.id ?? ''
  versions.value = saved; accept(saved[0] ?? null); ready.value = true
})
function switchVersion(event: globalThis.Event) {
  const select = event.target as globalThis.HTMLSelectElement
  if (canDiscard()) accept(versions.value.find(v => v.id === select.value) ?? null)
  else select.value = selected.value?.id ?? ''
}
const save = () => run(async () => {
  const saved = await api.saveWriting({ projectId: props.id, kind: 'publishing', content: { ...draft.value }, basedOn: selected.value?.id ?? null, isConfirmed: false })
  accept(saved); versions.value = [saved, ...versions.value]; message.value = '修改已保存。'
})
const generate = () => run(async () => {
  try {
    taskId.value = await api.startGeneration({ projectId: props.id, providerId: providerId.value })
    const saved = await api.executeGeneration(taskId.value)
    versions.value = [saved, ...versions.value]
    if (!dirty.value) accept(saved)
    message.value = dirty.value ? '生成完成，当前修改已保留。保存后可在历史版本中查看新结果。' : '发布文案、音频提示词、封面主副标题及两种比例的图片提示词已生成并保存。'
  } finally { taskId.value = '' }
})
async function cancel() {
  if (cancelling.value) return
  cancelling.value = true
  try { await api.cancelGeneration(taskId.value) }
  catch { error.value = '取消失败，请稍后重试。' }
  finally { cancelling.value = false }
}
async function makeCover() {
  if (dirty.value) await save()
  if (!dirty.value && selected.value) await router.push({ path: `/projects/${props.id}/covers`, query: { writingVersion: selected.value.id } })
}
async function copy(field: keyof Omit<PublishingContent, 'format'>, label: string) {
  try {
    await globalThis.navigator.clipboard.writeText(draft.value[field]); message.value = `已复制${label}。`; error.value = ''
    copiedField.value = field
    globalThis.clearTimeout(copyTimer)
    copyTimer = globalThis.setTimeout(() => { copiedField.value = '' }, 2000)
  }
  catch { error.value = '复制失败，请选中文本后手动复制。' }
}
const fields: { key: keyof Omit<PublishingContent, 'format'>; label: string; rows: number; limit: number; help?: string }[] = [
  { key: 'title', label: '发布标题', rows: 2, limit: 30 },
  { key: 'description', label: '发布内容', rows: 7, limit: 1000 },
  { key: 'audioPrompt', label: '音频生成提示词', rows: 18, limit: 20000, help: '复制到音频工具的自定义提示词中，并在该工具中选择对应书籍来源。' },
  { key: 'coverTitle', label: '封面主标题', rows: 2, limit: 30, help: '提炼本书的悬念或看点，与发布标题分开使用。旧版本没有封面内容，可再次一键生成，也可手动填写。' },
  { key: 'coverSubtitle', label: '封面副标题', rows: 2, limit: 60, help: '用书名和具体看点补充主标题，可直接带入制作封面。' },
  { key: 'coverPromptLandscape', label: '横版 4:3 封面提示词', rows: 7, limit: 2000, help: '用于 1440 × 1080 横版构图，复制到图片工具并选择 4:3。只生成无字背景，主副标题在制作封面时添加。' },
  { key: 'coverPromptPortrait', label: '竖版 3:4 封面提示词', rows: 7, limit: 2000, help: '用于 1080 × 1440 竖版构图，复制到图片工具并选择 3:4。与横版保持同一主题，分别安排主体和留白。' },
]
const canSave = computed(() => [draft.value.title, draft.value.description, draft.value.audioPrompt].every(value => value.trim()))
function fieldTab(key: string) { return key.startsWith('cover') ? 'cover' : key === 'audioPrompt' ? 'audio' : 'publishing' }
function saveShortcut(event: globalThis.KeyboardEvent) {
  if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 's') {
    event.preventDefault()
    if (ready.value && !busy.value && dirty.value && canSave.value) void save()
  }
}
onMounted(() => globalThis.addEventListener('keydown', saveShortcut))
onBeforeUnmount(() => globalThis.removeEventListener('keydown', saveShortcut))
onMounted(() => { if (desktop) void load() })
</script>
<template>
  <header class="page-heading">
    <div>
      <RouterLink
        class="back-link"
        to="/projects"
      >
        <AppIcon name="back" />我的书籍
      </RouterLink>
      <h1>{{ title || '书籍' }}</h1>
      <p class="muted">
        把读过的好书，变成想分享的内容。
      </p>
    </div>
    <RouterLink
      class="button secondary"
      :to="`/projects/${id}`"
    >
      <AppIcon name="pen" />修改书籍资料
    </RouterLink>
  </header>
  <ProjectNavigation :id="id" />
  <p
    v-if="!desktop"
    class="notice"
  >
    请在桌面应用中生成内容。
  </p>
  <OperationFeedback
    :error="error"
    :message="message"
    @dismiss="message = ''"
  />
  <button
    v-if="!ready"
    :disabled="!desktop || busy"
    @click="load"
  >
    <AppIcon
      name="refresh"
      :class="{ spin: busy }"
    />{{ busy ? '读取中…' : '重新读取' }}
  </button>
  <template v-if="ready">
    <section class="panel generation-panel">
      <div class="generation-info">
        <span class="generation-symbol"><AppIcon
          name="sparkle"
          :class="{ spin: !!taskId }"
        /></span>
        <div><h2>让灵感，成为这一期内容</h2><p>甲子 · 十分钟读懂一本书 · 8—10 分钟</p></div>
      </div>
      <div class="generation-actions">
        <template v-if="providers.length > 1">
          <label
            class="sr-only"
            for="writing-provider"
          >文本模型</label>
          <select
            id="writing-provider"
            v-model="providerId"
            :disabled="busy"
          >
            <option
              v-for="provider in providers"
              :key="provider.id"
              :value="provider.id"
            >
              {{ provider.name }}
            </option>
          </select>
        </template>
        <button
          :disabled="busy || !providerId"
          @click="generate"
        >
          <AppIcon name="sparkle" />{{ taskId ? '生成中…' : '一键生成' }}
        </button>
        <button
          v-if="taskId"
          class="secondary"
          :disabled="cancelling"
          @click="cancel"
        >
          取消生成
        </button>
      </div>
    </section>
    <p
      v-if="!providers.length"
      class="notice"
    >
      先 <RouterLink to="/settings/models">
        配置文本模型
      </RouterLink>，即可开始生成。
    </p>
    <p
      v-if="taskId"
      class="notice"
      role="status"
    >
      正在生成文案、音频提示词和封面素材，最多等待 120 秒。你可以继续编辑，当前修改会保留。
    </p>
    <form
      class="writing-editor"
      @submit.prevent="save"
    >
      <EditorTabs
        id="writing"
        v-model="activeTab"
        label="创作内容"
        :tabs="tabs"
      />
      <div
        v-for="tab in tabs"
        :id="`writing-panel-${tab.value}`"
        :key="tab.value"
        :hidden="activeTab !== tab.value"
        role="tabpanel"
        :aria-labelledby="`writing-tab-${tab.value}`"
        tabindex="0"
      >
        <p class="tab-intro">
          {{ tab.value === 'publishing' ? '一份有吸引力的标题，一段让人想点开的介绍。' : tab.value === 'audio' ? '复制到音频工具的自定义指导，并选择对应的书籍来源。' : '为同一本书准备两种构图，用文字引出故事的第一眼。' }}
        </p>
        <section
          v-for="field in fields.filter(item => fieldTab(item.key) === tab.value)"
          :key="field.key"
          class="panel editor-section"
        >
          <div class="result-heading">
            <label :for="`result-${field.key}`">{{ field.label }}</label>
            <button
              type="button"
              class="secondary"
              :class="{ copied: copiedField === field.key }"
              :disabled="!draft[field.key].trim()"
              :aria-label="`复制${field.label}`"
              @click="copy(field.key, field.label)"
            >
              <AppIcon :name="copiedField === field.key ? 'check' : 'copy'" />{{ copiedField === field.key ? '已复制' : `复制${field.label}` }}
            </button>
          </div>
          <p
            v-if="field.help"
            class="muted"
          >
            {{ field.help }}
          </p>
          <textarea
            :id="`result-${field.key}`"
            v-model="draft[field.key]"
            :rows="field.rows"
            :maxlength="field.limit"
            :disabled="busy && !taskId"
            :placeholder="`生成后在这里查看和修改${field.label}`"
          />
          <p
            v-if="field.key !== 'audioPrompt'"
            class="character-count"
          >
            {{ [...draft[field.key]].length }} / {{ field.limit }}
          </p>
        </section>
      </div>
      <div class="sticky-actions">
        <span
          class="save-status"
          :class="{ 'is-dirty': dirty }"
        ><i class="status-dot" />{{ dirty ? '有未保存的修改' : selected ? '当前版本已保存' : '生成后自动保存' }}<span class="shortcut-hint"> · Ctrl / ⌘ S 保存</span></span>
        <div class="actions wrap">
          <button
            type="submit"
            class="secondary"
            :disabled="busy || !dirty || !canSave"
          >
            <AppIcon name="save" />保存修改
          </button>
          <button
            type="button"
            :disabled="busy || !canSave || !draft.coverTitle.trim() || !draft.coverSubtitle.trim()"
            @click="makeCover"
          >
            {{ dirty ? '保存并制作封面' : '使用这些内容制作封面' }}<AppIcon name="arrow" />
          </button>
        </div>
      </div>
    </form>
    <details
      v-if="versions.length"
      class="history-panel"
    >
      <summary>历史版本（{{ versions.length }}） · 每次生成和保存，都为你保留</summary>
      <label for="writing-version">选择历史版本</label>
      <select
        id="writing-version"
        :value="selected?.id ?? ''"
        :disabled="busy"
        @change="switchVersion"
      >
        <option
          v-for="version in versions"
          :key="version.id"
          :value="version.id"
        >
          v{{ version.versionNo }} · {{ version.sourceType === 'ai' ? '生成' : '修改' }} · {{ new Date(version.createdAt).toLocaleString() }}
        </option>
      </select>
    </details>
  </template>
</template>
