<script setup lang="ts">
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '@/components/ui/collapsible'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { Textarea } from '@/components/ui/textarea'
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { RouterLink, onBeforeRouteLeave, onBeforeRouteUpdate, useRouter } from 'vue-router'
import * as api from '../../services/ai'
import { chooseAudio, getProject, isDesktop, linkAsset, listAssets, openAsset } from '../../services/projects'
import { emptyPublishing, type GenerationPreview, type ModelProvider, type PublishingContent, type PublishingField, type PublishingTextField, type WritingVersion } from '../../types/ai'
import CoverDesignPreview from '../../components/CoverDesignPreview.vue'
import type { Asset } from '../../types/project'
import { useOperation } from '../../composables/useOperation'
import { useEditorDraft } from '../../composables/useEditorDraft'
import { readDraft } from '../../services/drafts'
import AppIcon from '../../components/AppIcon.vue'
import ProjectNavigation from '../../components/ProjectNavigation.vue'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import OperationFeedback from '../../components/OperationFeedback.vue'
const props = defineProps<{ id: string }>()
const router = useRouter()
const desktop = isDesktop()
const title = ref('')
const ready = ref(false)
const providers = ref<ModelProvider[]>([])
const providerId = ref('')
const versions = ref<WritingVersion[]>([])
const audioAssets = ref<Asset[]>([])
const selected = ref<WritingVersion | null>(null)
const draft = ref(emptyPublishing())
const original = ref(JSON.stringify(draft.value))
const dirty = computed(() => JSON.stringify(draft.value) !== original.value)
const taskId = ref('')
const cancelling = ref(false)
const revisionOpen = ref(false)
const revisionFields = ref<PublishingField[]>(['title'])
const revisionInstruction = ref('')
const revising = ref(false)
const preview = ref<GenerationPreview | null>(null)
const revisionLocked = computed(() => revising.value || !!preview.value)
const activeTab = ref('publishing')
const copiedField = ref('')
let copyTimer: ReturnType<typeof globalThis.setTimeout> | undefined
const tabs = [
  { value: 'publishing', label: '发布文案' },
  { value: 'audio', label: '音频制作' },
  { value: 'cover', label: '封面素材' },
]
const { busy, error, message, run } = useOperation()
const autosave = useEditorDraft(() => props.id, 'writing', draft, () => selected.value?.id ?? null, dirty, ready, () => !busy.value && (!preview.value || canDiscard()))
function accept(version: WritingVersion | null) {
  selected.value = version
  draft.value = { ...emptyPublishing(), ...version?.content }
  original.value = JSON.stringify(draft.value)
}
function canDiscard() { return (!dirty.value && !preview.value) || globalThis.confirm(preview.value ? '候选结果尚未采用，确定放弃？' : '当前内容尚未保存，确定放弃修改？') }
const canLeave = async () => {
  if (busy.value || (preview.value && !canDiscard())) return false
  try { if (ready.value) await autosave.flush(); return true } catch { return false }
}
onBeforeRouteLeave(canLeave)
onBeforeRouteUpdate(canLeave)
function beforeUnload(event: globalThis.BeforeUnloadEvent) { if (dirty.value || busy.value || preview.value) event.preventDefault() }
globalThis.addEventListener('beforeunload', beforeUnload)
onBeforeUnmount(() => { globalThis.removeEventListener('beforeunload', beforeUnload); globalThis.clearTimeout(copyTimer) })
const load = () => run(async () => {
  const [project, models, saved, recovery, files] = await Promise.all([getProject(props.id), api.listProviders(), api.listVersions(props.id, 'publishing'), readDraft<PublishingContent>(props.id, 'writing'), listAssets(props.id)])
  title.value = project.title
  audioAssets.value = files.filter(asset => asset.assetType === 'audio')
  providers.value = models.filter(p => p.isEnabled && p.modelKind === 'text')
  providerId.value = providers.value.find(p => p.isDefault)?.id ?? providers.value[0]?.id ?? ''
  versions.value = saved
  accept(recovery ? saved.find(v => v.id === recovery.basedOn) ?? null : saved[0] ?? null)
  if (recovery) { draft.value = { ...emptyPublishing(), ...recovery.content }; message.value = '已恢复自动暂存的草稿，保存修改后才会新增历史版本。' }
  ready.value = true
})
function switchVersion(id: string | null) {
  if (canDiscard()) accept(versions.value.find(v => v.id === id) ?? null)
}
async function persistWriting() {
  await autosave.flush()
  const saved = await api.saveWriting({ projectId: props.id, kind: 'publishing', content: { ...draft.value }, basedOn: selected.value?.id ?? null, isConfirmed: false })
  accept(saved); versions.value = [saved, ...versions.value]
  await autosave.flush()
}
const save = () => run(async () => { await persistWriting(); message.value = '修改已保存。' })
const openAudioSite = () => run(async () => { await api.openAudioTool(); message.value = '已在浏览器打开 Gemini Notebook。下载音频后回到这里导入。' })
const importAudio = () => run(async () => {
  const path = await chooseAudio()
  if (!path) return
  await linkAsset(props.id, path)
  const files = await listAssets(props.id)
  audioAssets.value = files.filter(asset => asset.assetType === 'audio')
  message.value = '音频已导入。请到剪映生成字幕、添加视频背景并导出最终成片。'
})
const playAudio = (id: string) => run(() => openAsset(id))
const generate = () => run(async () => {
  try {
    taskId.value = await api.startGeneration({ projectId: props.id, providerId: providerId.value })
    const saved = await api.executeGeneration(taskId.value)
    if ('taskId' in saved) throw new Error('生成结果类型不符，当前内容保留。')
    versions.value = [saved, ...versions.value]
    if (!dirty.value) accept(saved)
    message.value = dirty.value ? '生成完成，当前修改已保留。保存后可在历史版本中查看新结果。' : '发布文案、音频提示词、封面主副标题及两种比例的图片提示词已生成并保存。'
  } finally { taskId.value = '' }
})
function openRevision() { revisionOpen.value = true }
function selectRevision(field: PublishingField, checked: boolean | 'indeterminate') {
  revisionFields.value = checked === true ? [...revisionFields.value, field] : revisionFields.value.filter(key => key !== field)
}
const generateRevision = () => run(async () => {
  revising.value = true
  try {
    if (dirty.value) await persistWriting()
    if (!selected.value) throw new Error('请先生成或保存一套内容。')
    taskId.value = await api.startGeneration({ projectId: props.id, providerId: providerId.value, revision: { basedOn: selected.value.id, fields: [...revisionFields.value], instruction: revisionInstruction.value.trim() } })
    const result = await api.executeGeneration(taskId.value)
    if (!('taskId' in result)) throw new Error('调整结果类型不符，当前内容保留。')
    preview.value = result
    message.value = '候选结果已生成。核对后采用，或放弃并保留原内容。'
  } finally { taskId.value = ''; revising.value = false }
})
const applyRevision = () => run(async () => {
  if (!preview.value) return
  const saved = await api.applyGeneration(preview.value.taskId)
  accept(saved); versions.value = [saved, ...versions.value.filter(v => v.id !== saved.id)]
  preview.value = null; revisionOpen.value = false
  message.value = '已采用选中内容并保存新版本，其余内容保持原样。'
})
function closeRevision() { preview.value = null; revisionOpen.value = false }
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
async function copy(field: PublishingTextField, label: string) {
  try {
    await globalThis.navigator.clipboard.writeText(draft.value[field]); message.value = `已复制${label}。`; error.value = ''
    copiedField.value = field
    globalThis.clearTimeout(copyTimer)
    copyTimer = globalThis.setTimeout(() => { copiedField.value = '' }, 2000)
  }
  catch { error.value = '复制失败，请选中文本后手动复制。' }
}
const fields: { key: PublishingTextField; label: string; rows: number; limit: number; help?: string }[] = [
  { key: 'title', label: '发布标题', rows: 2, limit: 30 },
  { key: 'description', label: '发布内容', rows: 7, limit: 1000 },
  { key: 'audioPrompt', label: '音频生成提示词', rows: 18, limit: 20000, help: '复制到音频工具的自定义提示词中，并在该工具中选择对应书籍来源。生成音频后导入下方音频素材，再交给剪映制作。' },
  { key: 'coverTitle', label: '封面主标题', rows: 2, limit: 30, help: '提炼本书的悬念或看点，与发布标题分开使用。旧版本没有封面内容，可再次一键生成，也可手动填写。' },
  { key: 'coverSubtitle', label: '封面副标题', rows: 2, limit: 60, help: '用书名和具体看点补充主标题，可直接带入制作封面。' },
  { key: 'coverPromptLandscape', label: '横版 4:3 封面提示词', rows: 7, limit: 2000, help: '用于 1440 × 1080 横版构图，复制到图片工具并选择 4:3。只生成无字背景，主副标题在制作封面时添加。' },
  { key: 'coverPromptPortrait', label: '竖版 3:4 封面提示词', rows: 7, limit: 2000, help: '用于 1080 × 1440 竖版构图，复制到图片工具并选择 3:4。与横版保持同一主题，分别安排主体和留白。' },
]
const revisionOptions: { key: PublishingField; label: string }[] = [...fields, { key: 'coverDesign', label: '封面字号与配色' }]
const canSave = computed(() => [draft.value.title, draft.value.description, draft.value.audioPrompt].every(value => value.trim()))
function fieldTab(key: string) { return key.startsWith('cover') ? 'cover' : key === 'audioPrompt' ? 'audio' : 'publishing' }
function saveShortcut(event: globalThis.KeyboardEvent) {
  if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 's') {
    event.preventDefault()
    if (ready.value && !busy.value && !revisionLocked.value && dirty.value && canSave.value) void save()
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
    <Button
      variant="outline"
      as-child
    >
      <RouterLink
        :to="`/projects/${id}`"
      >
        <AppIcon name="pen" />修改书籍资料
      </RouterLink>
    </Button>
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
  <p
    v-if="autosave.error.value"
    class="error"
    role="alert"
  >
    {{ autosave.error.value }}
  </p>
  <Button
    v-if="!ready"
    :disabled="!desktop || busy"
    type="button"
    @click="load"
  >
    <AppIcon
      name="refresh"
      :class="{ spin: busy }"
    />{{ busy ? '读取中…' : '重新读取' }}
  </Button>
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
          <Select
            v-model="providerId"
            :disabled="busy"
          >
            <SelectTrigger
              id="writing-provider"
              class="w-full min-w-0 sm:w-44"
            >
              <SelectValue placeholder="请选择" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem
                v-for="provider in providers"
                :key="provider.id"
                :value="provider.id"
              >
                {{ provider.name }}
              </SelectItem>
            </SelectContent>
          </Select>
        </template>
        <Button
          :disabled="busy || !providerId || !!preview"
          type="button"
          @click="generate"
        >
          <AppIcon name="sparkle" />{{ taskId ? '生成中…' : '一键生成' }}
        </Button>
        <Button
          variant="outline"
          :disabled="busy || !selected || !!preview || !providerId"
          type="button"
          @click="openRevision"
        >
          按项调整
        </Button>
        <Button
          v-if="taskId"
          :disabled="cancelling"
          variant="outline"
          type="button"
          @click="cancel"
        >
          取消生成
        </Button>
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
      {{ revising ? '正在调整选中的内容，最多等待 120 秒。完成后先预览，再决定是否采用。' : '正在生成文案、音频提示词和封面素材，最多等待 120 秒。你可以继续编辑，当前修改会保留。' }}
    </p>
    <section
      v-if="revisionOpen"
      class="panel mb-6 space-y-5"
      aria-labelledby="revision-heading"
    >
      <div>
        <h2 id="revision-heading">
          按项调整
        </h2>
        <p class="muted">
          只重新生成选中的内容。生成前会保存当前修改，候选结果经你采用后才成为新版本。
        </p>
      </div>
      <fieldset :disabled="busy || !!preview">
        <legend class="mb-3 text-sm font-medium">
          选择要调整的内容
        </legend>
        <div class="grid gap-3 sm:grid-cols-2">
          <label
            v-for="field in revisionOptions"
            :key="field.key"
            :for="`revision-${field.key}`"
            class="flex items-center gap-2 text-sm"
          >
            <Checkbox
              :id="`revision-${field.key}`"
              :model-value="revisionFields.includes(field.key)"
              :disabled="busy || !!preview"
              @update:model-value="selectRevision(field.key, $event)"
            />{{ field.label }}
          </label>
        </div>
      </fieldset>
      <div>
        <label for="revision-instruction">修改要求 <span class="optional">· 选填</span></label>
        <Textarea
          id="revision-instruction"
          v-model="revisionInstruction"
          :disabled="busy || !!preview"
          :maxlength="2000"
          :rows="3"
          placeholder="例如：标题更有悬念但不剧透；发布内容再简短一些。"
        />
      </div>
      <template v-if="preview">
        <section
          v-if="preview.fields.includes('coverDesign')"
          class="space-y-3 border-t pt-4"
          aria-label="封面字号与配色调整对比"
        >
          <h3 class="text-sm font-semibold">
            封面字号与配色
          </h3>
          <div class="grid gap-4 lg:grid-cols-2">
            <div class="space-y-3">
              <p>原推荐</p>
              <CoverDesignPreview
                v-if="preview.before.coverDesign"
                :design="preview.before.coverDesign"
              />
              <p v-else>
                此版本还没有排版推荐。
              </p>
            </div>
            <div class="space-y-3">
              <p>候选推荐</p>
              <CoverDesignPreview
                v-if="preview.content.coverDesign"
                :design="preview.content.coverDesign"
              />
            </div>
          </div>
        </section>
        <section
          v-for="field in fields.filter(item => preview?.fields.includes(item.key))"
          :key="field.key"
          class="space-y-3 border-t pt-4"
          :aria-label="`${field.label}调整对比`"
        >
          <h3 class="text-sm font-semibold">
            {{ field.label }}
          </h3>
          <div class="grid gap-4 lg:grid-cols-2">
            <div>
              <label :for="`revision-before-${field.key}`">原内容</label>
              <Textarea
                :id="`revision-before-${field.key}`"
                :model-value="preview.before[field.key]"
                :rows="Math.min(field.rows, 8)"
                readonly
              />
            </div>
            <div>
              <label :for="`revision-after-${field.key}`">候选内容</label>
              <Textarea
                :id="`revision-after-${field.key}`"
                :model-value="preview.content[field.key]"
                :rows="Math.min(field.rows, 8)"
                readonly
              />
            </div>
          </div>
        </section>
      </template>
      <div class="actions wrap">
        <Button
          v-if="preview"
          :disabled="busy"
          type="button"
          @click="applyRevision"
        >
          采用并保存新版本
        </Button>
        <Button
          v-else
          :disabled="busy || !revisionFields.length || !canSave || !providerId"
          type="button"
          @click="generateRevision"
        >
          {{ dirty ? '保存并生成预览' : '生成预览' }}
        </Button>
        <Button
          :disabled="busy"
          variant="outline"
          type="button"
          @click="closeRevision"
        >
          {{ preview ? '放弃候选结果' : '取消调整' }}
        </Button>
      </div>
    </section>
    <form
      class="writing-editor"
      @submit.prevent="save"
    >
      <Tabs
        v-model="activeTab"
        :unmount-on-hide="false"
      >
        <TabsList
          class="w-full"
          aria-label="创作内容"
        >
          <TabsTrigger
            v-for="tab in tabs"
            :key="tab.value"
            :value="tab.value"
          >
            {{ tab.label }}
          </TabsTrigger>
        </TabsList>
        <TabsContent
          v-for="tab in tabs"
          :key="tab.value"
          :value="tab.value"
        >
          <p class="tab-intro">
            {{ tab.value === 'publishing' ? '一份有吸引力的标题，一段让人想点开的介绍。' : tab.value === 'audio' ? '生成音频后导入项目，再交给剪映生成字幕、添加背景并导出最终成片。' : '为同一本书准备两种构图，用文字引出故事的第一眼。' }}
          </p>
          <section
            v-if="tab.value === 'cover' && draft.coverDesign"
            class="panel editor-section"
            aria-label="AI 推荐排版"
          >
            <h2 class="mb-4 text-base font-semibold">
              AI 推荐字号与配色
            </h2>
            <CoverDesignPreview :design="draft.coverDesign" />
            <p class="muted">
              制作新封面时自动带入；已有封面可在「排版样式」中应用，并单独调整每种颜色。
            </p>
          </section>
          <section
            v-for="field in fields.filter(item => fieldTab(item.key) === tab.value)"
            :key="field.key"
            class="panel editor-section"
          >
            <div class="result-heading">
              <label :for="`result-${field.key}`">{{ field.label }}</label>
              <Button
                type="button"
                :class="{ 'border-primary/30 bg-accent text-primary': copiedField === field.key }"
                :disabled="!draft[field.key].trim()"
                :aria-label="`复制${field.label}`"
                variant="outline"
                @click="copy(field.key, field.label)"
              >
                <AppIcon :name="copiedField === field.key ? 'check' : 'copy'" />{{ copiedField === field.key ? '已复制' : `复制${field.label}` }}
              </Button>
            </div>
            <p
              v-if="field.help"
              class="muted"
            >
              {{ field.help }}
            </p>
            <Textarea
              :id="`result-${field.key}`"
              v-model="draft[field.key]"
              :rows="field.rows"
              :maxlength="field.limit"
              :disabled="(busy && !taskId) || revisionLocked"
              :placeholder="`生成后在这里查看和修改${field.label}`"
            />
            <p
              v-if="field.key !== 'audioPrompt'"
              class="character-count"
            >
              {{ [...draft[field.key]].length }} / {{ field.limit }}
            </p>
          </section>
          <section
            v-if="tab.value === 'audio'"
            class="panel editor-section"
            aria-labelledby="audio-assets-title"
          >
            <div class="result-heading">
              <h2 id="audio-assets-title">
                音频成品与剪映制作
              </h2>
              <Badge variant="secondary">
                {{ audioAssets.length }} 个音频
              </Badge>
            </div>
            <p class="muted">
              外部生成并下载音频后导入这里；随后在剪映生成字幕、添加视频背景，导出最终成片，再到发布准备关联成片。
            </p>
            <div class="actions wrap">
              <Button
                type="button"
                :disabled="!desktop || busy"
                variant="outline"
                @click="openAudioSite"
              >
                <AppIcon name="headphones" />打开 Gemini Notebook
              </Button>
              <Button
                type="button"
                :disabled="!desktop || busy"
                @click="importAudio"
              >
                <AppIcon name="upload" />导入音频成品
              </Button>
            </div>
            <p
              v-if="!audioAssets.length"
              class="muted"
            >
              还没有音频成品。支持 MP3、M4A、WAV、AAC、FLAC 和 OGG。
            </p>
            <article
              v-for="asset in audioAssets"
              :key="asset.id"
              class="asset-row"
            >
              <h3>
                {{ asset.originalName }} <Badge variant="secondary">
                  音频
                </Badge>
              </h3>
              <p class="file-path">
                {{ asset.filePath }}
              </p>
              <p :class="asset.existsStatus === 'available' ? 'muted' : 'error'">
                {{ asset.existsStatus === 'available' ? '文件可用' : '文件已丢失，请到发布准备页重新定位' }} · {{ (asset.fileSize / 1024 / 1024).toFixed(2) }} MB
              </p>
              <Button
                type="button"
                variant="outline"
                :disabled="busy || asset.existsStatus !== 'available'"
                @click="playAudio(asset.id)"
              >
                打开音频
              </Button>
            </article>
          </section>
        </TabsContent>
      </Tabs>
      <div class="sticky-actions">
        <span
          class="save-status"
          :class="{ 'is-dirty': dirty }"
        ><i class="status-dot" />{{ dirty ? autosave.status.value || '有未保存的修改' : selected ? '当前版本已保存' : '生成后自动保存' }}<span class="shortcut-hint"> · Ctrl / ⌘ S 保存</span></span>
        <div class="actions wrap">
          <Button
            type="submit"
            :disabled="busy || revisionLocked || !dirty || !canSave"
            variant="outline"
          >
            <AppIcon name="save" />保存修改
          </Button>
          <Button
            type="button"
            :disabled="busy || revisionLocked || !canSave || !draft.coverTitle.trim() || !draft.coverSubtitle.trim()"
            @click="makeCover"
          >
            {{ dirty ? '保存并制作封面' : '使用这些内容制作封面' }}<AppIcon name="arrow" />
          </Button>
        </div>
      </div>
    </form>
    <Collapsible
      v-if="versions.length"
      class="history-panel"
      :unmount-on-hide="false"
    >
      <CollapsibleTrigger class="disclosure-trigger">
        历史版本（{{ versions.length }}） · 每次生成和保存，都为你保留
      </CollapsibleTrigger>
      <CollapsibleContent>
        <label for="writing-version">选择历史版本</label>
        <Select
          :model-value="selected?.id ?? null"
          :disabled="busy || !!preview"
          @update:model-value="switchVersion"
        >
          <SelectTrigger
            id="writing-version"
            class="w-full min-w-0"
          >
            <SelectValue placeholder="请选择" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem
              v-for="version in versions"
              :key="version.id"
              :value="version.id"
            >
              v{{ version.versionNo }} · {{ version.sourceType === 'ai' ? '生成' : '修改' }} · {{ new Date(version.createdAt).toLocaleString() }}
            </SelectItem>
          </SelectContent>
        </Select>
      </CollapsibleContent>
    </Collapsible>
  </template>
</template>
