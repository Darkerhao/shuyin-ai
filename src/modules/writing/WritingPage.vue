<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { RouterLink, onBeforeRouteLeave, onBeforeRouteUpdate, useRouter } from 'vue-router'
import * as api from '../../services/ai'
import { getProject, isDesktop } from '../../services/projects'
import { emptyPublishing, type ModelProvider, type PublishingContent, type WritingVersion } from '../../types/ai'
import { useOperation } from '../../composables/useOperation'
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
onBeforeUnmount(() => globalThis.removeEventListener('beforeunload', beforeUnload))
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
  try { await globalThis.navigator.clipboard.writeText(draft.value[field]); message.value = `已复制${label}。`; error.value = '' }
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
onMounted(() => { if (desktop) void load() })
</script>
<template>
  <header>
    <RouterLink to="/projects">
      ← 我的书籍
    </RouterLink>
    <h1>{{ title || '书籍' }}</h1>
    <p class="muted">
      发布文案、音频提示词和封面素材，一次准备好。
    </p>
    <RouterLink :to="`/projects/${id}`">
      修改书籍资料
    </RouterLink>
    <RouterLink
      class="cover-entry"
      :to="`/projects/${id}/covers`"
    >
      制作封面
    </RouterLink>
    <RouterLink
      class="cover-entry"
      :to="`/projects/${id}/publishing`"
    >
      发布准备
    </RouterLink>
  </header>
  <p
    v-if="!desktop"
    class="notice"
  >
    请在桌面应用中生成内容。
  </p>
  <p
    v-if="error"
    class="error"
    role="alert"
  >
    {{ error }}
  </p>
  <p
    v-if="message"
    class="notice"
    role="status"
  >
    {{ message }}
  </p>
  <button
    v-if="!ready"
    :disabled="!desktop || busy"
    @click="load"
  >
    {{ busy ? '读取中…' : '重新读取' }}
  </button>
  <template v-if="ready">
    <section class="panel">
      <h2>准备这一期内容</h2>
      <p class="muted">
        甲子 · 十分钟读懂一本书 · 简体中文 · 8—10 分钟 · 通俗幽默、有故事感
      </p>
      <template v-if="providers.length > 1">
        <label for="writing-provider">文本模型</label>
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
      <p
        v-if="!providers.length"
        class="notice"
      >
        先<RouterLink to="/settings/models">
          配置文本模型
        </RouterLink>，即可开始生成。
      </p>
      <div class="actions">
        <button
          :disabled="busy || !providerId"
          @click="generate"
        >
          {{ taskId ? '生成中…' : '一键生成' }}
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
      <p
        v-if="taskId"
        class="muted"
        role="status"
      >
        正在生成，最多等待 120 秒，已有内容保留。
      </p>
    </section>
    <form @submit.prevent="save">
      <section
        v-for="field in fields"
        :key="field.key"
        class="panel"
      >
        <div class="result-heading">
          <label :for="`result-${field.key}`">{{ field.label }}</label>
          <button
            type="button"
            class="secondary"
            :disabled="!draft[field.key].trim()"
            @click="copy(field.key, field.label)"
          >
            复制{{ field.label }}
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
          class="muted character-count"
        >
          {{ [...draft[field.key]].length }} / {{ field.limit }}
        </p>
      </section>
      <div class="actions wrap">
        <button
          type="submit"
          :disabled="busy || !dirty || !canSave"
        >
          保存修改
        </button>
        <button
          type="button"
          class="secondary"
          :disabled="busy || !canSave || !draft.coverTitle.trim() || !draft.coverSubtitle.trim()"
          @click="makeCover"
        >
          {{ dirty ? '保存并制作封面' : '使用这些内容制作封面' }}
        </button>
        <span class="muted">{{ dirty ? '有未保存的修改' : '已保存的内容会自动保留' }}</span>
      </div>
    </form>
    <details
      v-if="versions.length"
      class="history-panel"
    >
      <summary>历史版本（{{ versions.length }}）</summary>
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
