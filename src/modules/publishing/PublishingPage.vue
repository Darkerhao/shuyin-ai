<script setup lang="ts">
import { displayPath } from '@/lib/displayPath'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { DateTimePicker } from '@/components/ui/date-time-picker'
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '@/components/ui/collapsible'
import { Input } from '@/components/ui/input'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { Textarea } from '@/components/ui/textarea'
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { RouterLink, onBeforeRouteLeave, onBeforeRouteUpdate, useRoute, useRouter } from 'vue-router'
import * as api from '../../services/publishing'
import { getProject, isDesktop, listAssets, openAsset, videoAssetMetadata, type VideoMetadata } from '../../services/projects'
import { listVersions } from '../../services/ai'
import { listCovers, readCoverExport } from '../../services/covers'
import { listJobs as listVideoJobs } from '../../services/video'
import { publishInput, publishScheduleError, publishStatuses, type PublishInput, type PublishQueueItem, type PublishTask } from '../../types/publishing'
import type { Asset } from '../../types/project'
import type { CoverVersion } from '../../types/cover'
import type { WritingVersion } from '../../types/ai'
import { useOperation } from '../../composables/useOperation'
import { useEditorDraft } from '../../composables/useEditorDraft'
import { readDraft } from '../../services/drafts'
import ProjectAssets from '../projects/ProjectAssets.vue'
import AppIcon from '../../components/AppIcon.vue'
import ProjectNavigation from '../../components/ProjectNavigation.vue'
import OperationFeedback from '../../components/OperationFeedback.vue'
import CoverThumbnail from '../../components/CoverThumbnail.vue'
import DouyinVerification from './DouyinVerification.vue'
import ConfirmAction from '../../components/ConfirmAction.vue'
import { requestConfirmation } from '../../composables/useConfirmation'
import { useCopyFeedback } from '../../composables/useCopyFeedback'
import CopyButton from '../../components/CopyButton.vue'
const clipboard = useCopyFeedback()
const copiedField = clipboard.copied

const props = defineProps<{ id: string; taskId?: string }>()
const router = useRouter()
const route = useRoute()
const assistant = ref<InstanceType<typeof DouyinVerification> | null>(null)
const nextTasks = ref<PublishQueueItem[]>([])
const nextTaskId = ref<string | null>(null)
const desktop = isDesktop()
const title = ref('')
const loaded = ref(false)
const tasks = ref<PublishTask[]>([])
const selected = ref<PublishTask | null>(null)
const assets = ref<Asset[]>([])
const covers = ref<CoverVersion[]>([])
const writing = ref<WritingVersion[]>([])
const draft = ref<PublishInput>({ projectId: props.id, writingVersionId: null, title: '', description: '', tags: '', videoAssetId: null, audioAssetId: null, landscapeCoverId: null, portraitCoverId: null, titleConfirmed: false, coversConfirmed: false, allowDownload: true, scheduledAt: null })
const original = ref(JSON.stringify(draft.value))
const dirty = computed(() => JSON.stringify(draft.value) !== original.value)
const published = computed(() => selected.value?.status === 'published')
const publishMode = computed({
  get: () => draft.value.scheduledAt === null ? 'immediate' : 'scheduled',
  set: (mode: string) => { draft.value.scheduledAt = mode === 'scheduled' ? '' : null },
})
const scheduledTime = computed({
  get: () => draft.value.scheduledAt?.slice(0, 16) ?? '',
  set: (value: string | number) => { draft.value.scheduledAt = value ? `${value}:00+08:00` : '' },
})
const scheduleError = computed(() => published.value ? '' : publishScheduleError(draft.value.scheduledAt))
const videos = computed(() => assets.value.filter(a => a.assetType === 'video'))
const audios = computed(() => assets.value.filter(a => a.assetType === 'audio'))
const landscape = computed(() => covers.value.filter(c => c.exportedPath && c.config.width === 1440 && c.config.height === 1080))
const portrait = computed(() => covers.value.filter(c => c.exportedPath && c.config.width === 1080 && c.config.height === 1440))
const video = computed(() => videos.value.find(a => a.id === draft.value.videoAssetId))
const audio = computed(() => audios.value.find(a => a.id === draft.value.audioAssetId))
const selectedLandscape = computed(() => landscape.value.find(c => c.id === draft.value.landscapeCoverId))
const selectedPortrait = computed(() => portrait.value.find(c => c.id === draft.value.portraitCoverId))
const videoInfo = ref<VideoMetadata | null>(null)
const videoInfoError = ref('')
let videoInfoRequest = Promise.resolve()
watch(() => [draft.value.videoAssetId, video.value?.filePath, video.value?.modifiedAt, video.value?.fileSize] as const, ([id], _, onCleanup) => {
  let stale = false
  onCleanup(() => { stale = true })
  videoInfo.value = null; videoInfoError.value = ''
  videoInfoRequest = id ? videoAssetMetadata(props.id, id).then(
    info => { if (!stale) videoInfo.value = info },
    () => { if (!stale) videoInfoError.value = '成片无法读取，请重新关联有效的视频。' },
  ) : Promise.resolve()
})
const titleCount = computed(() => [...draft.value.title].length)
const descriptionCount = computed(() => [...draft.value.description].length)
const savedChecksPassed = computed(() => !!selected.value?.checklist.length && selected.value.checklist.every(item => item.passed))
const canReview = computed(() => !scheduleError.value && titleCount.value > 0 && titleCount.value <= 30 && descriptionCount.value > 0 && descriptionCount.value <= 1000 && audio.value?.existsStatus === 'available' && video.value?.existsStatus === 'available' && !!videoInfo.value?.hasAudio && videoInfo.value.width > 0 && videoInfo.value.height > 0 && videoInfo.value.durationSeconds > 0 && !!selectedLandscape.value && !!selectedPortrait.value)
const recordOpen = ref(false)
const assetsOpen = ref(false)
const record = ref({ publishedAt: '', postUrl: '', notes: '' })
const recordTimeZone = Intl.DateTimeFormat().resolvedOptions().timeZone
const { busy: operationBusy, error, message, run } = useOperation()
const douyinBusy = ref(false)
const busy = computed(() => operationBusy.value || douyinBusy.value)
const restoringDraft = ref(false)
const draftOwner = computed(() => `${props.id}:${selected.value?.id ?? 'new'}`)
const draftReady = computed(() => loaded.value && !published.value && !restoringDraft.value)
const autosave = useEditorDraft(() => draftOwner.value, 'publishing', draft, () => selected.value?.updatedAt ?? null, dirty, draftReady, prepareLeave)

const taskLabel = (task: PublishTask) => `${task.title || '未填写标题'} · ${publishStatuses[task.status]} · ${new Date(task.createdAt).toLocaleString()}`
const assetLabel = (asset: Asset) => `${asset.originalName}${asset.existsStatus !== 'available' ? '（文件不可用）' : ''}`

function defaults(): PublishInput {
  return { projectId: props.id, writingVersionId: null, title: '', description: '', tags: '', videoAssetId: videos.value.find(v => v.existsStatus === 'available')?.id ?? null, audioAssetId: audios.value.find(v => v.existsStatus === 'available')?.id ?? null, landscapeCoverId: landscape.value[0]?.id ?? null, portraitCoverId: portrait.value[0]?.id ?? null, titleConfirmed: false, coversConfirmed: false, allowDownload: true, scheduledAt: null }
}
async function selectWriting(id: string | null) {
  if (id === draft.value.writingVersionId) return
  const version = writing.value.find(v => v.id === id)
  if (version && (draft.value.title || draft.value.description || draft.value.tags) && !await requestConfirmation('使用此版本将替换当前发布标题、内容和标签，确定继续？', 'publish-writing-version')) return
  draft.value.writingVersionId = id
  if (version) { draft.value.title = version.content.title; draft.value.description = version.content.description; draft.value.tags = version.content.tags }
  draft.value.titleConfirmed = false
}
function accept(task: PublishTask | null) {
  selected.value = task
  draft.value = task ? publishInput(task) : defaults()
  original.value = JSON.stringify(draft.value)
}
function update(task: PublishTask) {
  tasks.value = [task, ...tasks.value.filter(t => t.id !== task.id)]
  accept(task)
}
function prepareLeave() { return !busy.value && (!recordOpen.value || requestConfirmation('发布结果尚未记录，确定放弃本次结果填写？')) }
const canLeave = async () => {
  if (!await prepareLeave()) return false
  try { await autosave.flush(); return true } catch { return false }
}
async function restoreDraft(task: PublishTask | null) {
  restoringDraft.value = true
  accept(task)
  try {
    if (!published.value) {
      const recovery = await readDraft<PublishInput>(draftOwner.value, 'publishing')
      if (recovery) {
        draft.value = { ...draft.value, ...recovery.content, projectId: props.id }
        message.value = '已恢复本任务的编辑暂存；保存或确认后才更新发布准备。'
      }
    }
    autosave.reset()
  } catch (cause) {
    loaded.value = false
    throw cause
  } finally { restoringDraft.value = false }
}
onBeforeRouteLeave(canLeave)
let syncingTask = false
onBeforeRouteUpdate((to, from) => syncingTask || (to.params.id === from.params.id && to.query.task === from.query.task) || canLeave())
function beforeUnload(event: globalThis.BeforeUnloadEvent) { if (dirty.value || busy.value || recordOpen.value) event.preventDefault() }
globalThis.addEventListener('beforeunload', beforeUnload)
onBeforeUnmount(() => globalThis.removeEventListener('beforeunload', beforeUnload))

const load = () => run(async () => {
  loaded.value = false
  const [project, saved, files, exported, versions] = await Promise.all([getProject(props.id), api.listPublishTasks(props.id), listAssets(props.id), listCovers(props.id), listVersions(props.id, 'publishing')])
  const target = props.taskId === 'new' ? null : props.taskId ? saved.find(task => task.id === props.taskId) : saved[0] ?? null
  if (props.taskId && props.taskId !== 'new' && !target) throw new Error('此任务不属于当前书籍或已不存在，请返回发布任务列表。')
  title.value = project.title; tasks.value = saved; assets.value = files; covers.value = exported; writing.value = versions
  await restoreDraft(target ?? null)
  if (props.taskId === 'new' && typeof route.query.videoJob === 'string') {
    const job = (await listVideoJobs(props.id)).find(job => job.id === route.query.videoJob && job.status === 'succeeded')
    if (!job || !files.some(asset => asset.id === job.videoAssetId && asset.assetType === 'video' && asset.existsStatus === 'available') || !files.some(asset => asset.id === job.audioAssetId && asset.assetType === 'audio' && asset.existsStatus === 'available')) throw new Error('所选视频任务或原音频不可用，请回到视频制作检查素材。')
    draft.value.videoAssetId = job.videoAssetId
    draft.value.audioAssetId = job.audioAssetId
    draft.value.titleConfirmed = false
  }
  loaded.value = true
})
async function loadTask() {
  await load()
  if (loaded.value && route.query.assist === '1') {
    // Consume the explicit next-task action before running, so refresh cannot replay it.
    const query = { ...route.query }
    delete query.assist
    await router.replace({ path: route.path, query })
    await nextTick()
    await assistant.value?.start()
  }
}
watch(() => props.taskId, () => {
  if (!desktop || syncingTask || (loaded.value && (props.taskId === selected.value?.id || (props.taskId === 'new' && !selected.value)))) return
  void loadTask()
})
const refresh = () => run(async () => {
  const [files, exported, saved] = await Promise.all([listAssets(props.id), listCovers(props.id), api.listPublishTasks(props.id)])
  assets.value = files; covers.value = exported; tasks.value = saved
  if (selected.value) {
    const current = saved.find(t => t.id === selected.value?.id)
    if (!current) { error.value = '此任务已不存在，请重新打开这本书。'; return }
    if (dirty.value) selected.value = current
    else accept(current)
  }
  message.value = '素材与任务状态已刷新。'
})
async function syncTaskUrl(id: string | null) {
  if (route.query.task === (id ?? 'new')) return
  syncingTask = true
  try { await router.replace({ query: { ...route.query, task: id ?? 'new' } }) }
  finally { syncingTask = false }
}
function newTask() { void switchTask(null) }
async function switchTask(id: string | null) {
  if (!await canLeave()) return
  await run(async () => {
    await restoreDraft(tasks.value.find(t => t.id === id) ?? null)
    recordOpen.value = false
    await syncTaskUrl(id)
  })
}
async function persist() {
  const invalid = publishScheduleError(draft.value.scheduledAt)
  if (invalid) throw new Error(invalid)
  if (!selected.value || dirty.value) {
    await autosave.flush()
    const saved = await api.savePublishTask(selected.value?.id ?? null, { ...draft.value })
    autosave.reset()
    update(saved)
  }
  await syncTaskUrl(selected.value!.id)
  return selected.value!.id
}
async function persistReviewed() {
  await videoInfoRequest
  const invalid = publishScheduleError(draft.value.scheduledAt)
  if (invalid) throw new Error(invalid)
  if (!canReview.value) throw new Error('请先补齐文案、有效原声和成片，以及两张发布封面。')
  const snapshot = JSON.stringify(draft.value)
  await Promise.all([readCoverExport(props.id, draft.value.landscapeCoverId!), readCoverExport(props.id, draft.value.portraitCoverId!)])
  if (JSON.stringify(draft.value) !== snapshot) throw new Error('本次文字或素材已变化，请重新核对后开始。')
  draft.value.titleConfirmed = true; draft.value.coversConfirmed = true
  return persist()
}
function isConfirmationCheck(key: string) { return key === 'covers' || (key === 'title' && !!draft.value.title.trim()) }
function locateMissing(key: string) {
  const fields: Record<string, string> = { title: 'publish-title', description: 'publish-description', audio: 'publish-audio', video: 'publish-video', landscape: 'publish-landscape', portrait: 'publish-portrait', package: 'publish-export', schedule: 'publish-scheduled-at' }
  const confirmation = isConfirmationCheck(key)
  const target = confirmation ? 'publish-confirm' : fields[key] ?? 'publish-title'
  const action = confirmation || key === 'package'
  if (action) globalThis.document.getElementById('publish-preflight')?.scrollIntoView({ block: 'start' })
  globalThis.document.getElementById(action && !canReview.value ? 'publish-confirmation-help' : target)?.focus()
}
function resetReview() { draft.value.titleConfirmed = false; draft.value.coversConfirmed = false }
async function showAssets() {
  assetsOpen.value = true
  await nextTick()
  globalThis.document.getElementById('publish-assets')?.scrollIntoView({ block: 'start' })
  globalThis.document.querySelector<globalThis.HTMLButtonElement>('#publish-assets button')?.focus({ preventScroll: true })
}
async function prepareDouyinTask() {
  const id = await persistReviewed()
  if (selected.value?.status !== 'ready') update(await api.preparePublishTask(id))
  return selected.value!
}
const save = () => run(async () => { await persist(); message.value = '发布草稿已保存。' })
const prepare = () => run(async () => {
  const id = await persistReviewed()
  update(await api.preparePublishTask(id))
  message.value = '任务包已生成。请检查平台预览并手动发布，成功后回来记录结果。'
})
const open = (item: Parameters<typeof api.openPublishItem>[1]) => run(async () => {
  if (selected.value) await api.openPublishItem(selected.value.id, item)
})
async function copy(field: 'title' | 'description' | 'tags' | 'all') {
  error.value = ''; message.value = ''
  const text = field === 'all' ? [draft.value.title, draft.value.description, draft.value.tags].filter(Boolean).join('\n\n') : draft.value[field]
  await clipboard.copy(text, field)
  error.value = clipboard.error.value
}
function confirmPublish() {
  const now = new Date()
  record.value = { publishedAt: new Date(now.getTime() - now.getTimezoneOffset() * 60000).toISOString().slice(0, 16), postUrl: '', notes: '' }
  recordOpen.value = true
  nextTaskId.value = null; nextTasks.value = []
  void api.listPublishQueue().then(tasks => { if (recordOpen.value) nextTasks.value = tasks.filter(task => task.status === 'ready' && task.id !== selected.value?.id) }).catch(cause => { error.value = String(cause) })
}
async function recordSuccess(startNext = false) {
  const target = startNext ? nextTasks.value.find(task => task.id === nextTaskId.value) : null
  if (startNext && !target) return
  let recorded = false
  await run(async () => {
    if (!selected.value) return
    if (!record.value.publishedAt) throw new Error('请选择实际发布时间。')
    const saved = await api.recordPublishSuccess(selected.value.id, { ...record.value, publishedAt: new Date(record.value.publishedAt).toISOString(), confirmed: true })
    update(saved); recordOpen.value = false; message.value = '已记录发布成功。'; recorded = true
  })
  if (recorded && target) await router.push({ path: `/projects/${target.projectId}/publishing`, query: { task: target.id, assist: '1' } })
}
onMounted(() => { if (desktop) void loadTask() })
</script>

<template>
  <div
    class="publishing-page"
    :class="{ 'is-published': published }"
  >
    <div class="page-return-row publish-return">
      <RouterLink
        class="back-link"
        to="/projects"
      >
        <AppIcon name="back" />返回我的书籍
      </RouterLink>
      <div class="publish-return-actions">
        <RouterLink
          to="/publishing"
          class="back-link"
        >
          {{ published ? '继续处理下一篇' : '发布任务列表' }}
        </RouterLink>
        <Badge class="publish-stage-badge">
          <AppIcon name="upload" />发布前的最后一步
        </Badge>
      </div>
    </div>
    <header class="page-heading">
      <div>
        <div class="publish-title-row">
          <h1>准备好，分享你的作品</h1>
          <Badge class="publish-step-badge">
            Step 5 · 交付
          </Badge>
        </div>
        <p class="muted">
          {{ title || '书籍' }} · 选好素材与文案，开始辅助发布，检查后亲自发布。
        </p>
      </div>
    </header>
    <ProjectNavigation
      :id="id"
      :editing="dirty"
      :busy="busy"
    />
    <p
      v-if="!desktop"
      class="notice"
    >
      请在桌面应用中准备发布。
    </p>
    <OperationFeedback
      :error="error || autosave.error.value"
      :message="message"
      @dismiss="message = ''"
    />
    <Button
      v-if="!loaded"
      :disabled="busy || !desktop"
      type="button"
      @click="load"
    >
      {{ busy ? '读取中…' : '重新读取' }}
    </Button>
    <template v-if="loaded">
      <section class="panel publish-task-panel">
        <div class="result-heading">
          <h2>本书发布任务</h2><Badge
            class="publish-task-status"
            :data-status="selected?.status ?? 'draft'"
          >
            {{ selected ? publishStatuses[selected.status] : '新任务' }}
          </Badge>
        </div>
        <label
          v-if="tasks.length"
          for="publish-task"
        >选择任务</label>
        <Select
          v-if="tasks.length"
          :model-value="selected?.id ?? null"
          :disabled="busy || recordOpen"
          @update:model-value="switchTask($event)"
        >
          <SelectTrigger
            id="publish-task"
            class="w-full min-w-0"
          >
            <SelectValue>{{ selected ? taskLabel(selected) : '新任务' }}</SelectValue>
          </SelectTrigger>
          <SelectContent class="w-(--reka-select-trigger-width)">
            <SelectItem :value="null">
              新任务
            </SelectItem>
            <SelectItem
              v-for="task in tasks"
              :key="task.id"
              :value="task.id"
            >
              {{ taskLabel(task) }}
            </SelectItem>
          </SelectContent>
        </Select>
        <div class="actions">
          <Button
            :disabled="busy || recordOpen"
            variant="outline"
            type="button"
            @click="newTask"
          >
            新建发布任务
          </Button><Button
            :disabled="busy || recordOpen"
            variant="outline"
            type="button"
            @click="refresh"
          >
            刷新素材与状态
          </Button>
        </div>
      </section>
      <Collapsible
        v-if="!published"
        v-show="assetsOpen"
        id="publish-assets"
        v-model:open="assetsOpen"
        class="panel publish-assets-panel"
        :unmount-on-hide="false"
      >
        <CollapsibleTrigger class="disclosure-trigger">
          关联视频 / 重新定位素材
        </CollapsibleTrigger>
        <CollapsibleContent>
          <fieldset :disabled="busy || recordOpen">
            <ProjectAssets
              :project-id="id"
              @changed="refresh"
            />
          </fieldset>
        </CollapsibleContent>
      </Collapsible>
      <form @submit.prevent="save">
        <fieldset
          class="publish-form-grid"
          :disabled="busy || published || recordOpen"
        >
          <section class="panel publish-copy-panel">
            <h2>发布文字</h2>
            <label for="publish-writing-version">文案来源版本</label>
            <Select
              :model-value="draft.writingVersionId"
              :disabled="busy || published || recordOpen"
              @update:model-value="selectWriting"
            >
              <SelectTrigger
                id="publish-writing-version"
                class="w-full min-w-0"
              >
                <SelectValue placeholder="请选择历史文案，或手动填写" />
              </SelectTrigger>
              <SelectContent class="w-(--reka-select-trigger-width)">
                <SelectItem :value="null">
                  手动填写 / 旧任务未记录来源
                </SelectItem>
                <SelectItem
                  v-for="version in writing"
                  :key="version.id"
                  :value="version.id"
                >
                  v{{ version.versionNo }} · {{ version.content.title }} · {{ new Date(version.createdAt).toLocaleString() }}
                </SelectItem>
              </SelectContent>
            </Select>
            <p class="muted">
              选择后带入标题、内容和标签，仍可编辑；保存任务后固定文字快照与来源版本。
            </p>
            <div class="publish-field-label">
              <label for="publish-title">发布标题</label>
              <span id="publish-title-count">{{ titleCount }} / 30</span>
            </div>
            <Input
              id="publish-title"
              v-model="draft.title"
              maxlength="30"
              aria-describedby="publish-title-count"
              @input="draft.titleConfirmed = false"
            />
            <div class="publish-field-label">
              <label for="publish-description">发布内容</label>
              <span id="publish-description-count">{{ descriptionCount }} / 1000</span>
            </div>
            <Textarea
              id="publish-description"
              v-model="draft.description"
              rows="6"
              maxlength="1000"
              aria-describedby="publish-description-count"
            />
            <label for="publish-tags">发布标签（可编辑）</label>
            <Input
              id="publish-tags"
              v-model="draft.tags"
              maxlength="500"
              placeholder="#读书 #好书分享"
              aria-describedby="publish-tags-help"
            />
            <p
              id="publish-tags-help"
              class="muted"
            >
              从所选文案版本带入，多个 #标签 用空格分隔；辅助发布时自动追加到抖音内容末尾。旧文案可在内容创作中按项生成标签，也可留空。
            </p>
          </section>
          <section class="panel publish-media-panel">
            <h2>音频、成片与封面</h2>
            <label for="publish-audio">音频成品</label>
            <Select
              v-model="draft.audioAssetId"
              :disabled="busy || published || recordOpen"
              @update:model-value="resetReview"
            >
              <SelectTrigger
                id="publish-audio"
                class="w-full min-w-0"
              >
                <SelectValue>{{ audio ? assetLabel(audio) : '请选择用于制作的音频' }}</SelectValue>
              </SelectTrigger>
              <SelectContent class="w-(--reka-select-trigger-width)">
                <SelectItem :value="null">
                  请选择用于制作的音频
                </SelectItem>
                <SelectItem
                  v-for="file in audios"
                  :key="file.id"
                  :value="file.id"
                >
                  {{ assetLabel(file) }}
                </SelectItem>
              </SelectContent>
            </Select>
            <p
              v-if="audio"
              class="file-path"
              :title="displayPath(audio.filePath)"
            >
              {{ displayPath(audio.filePath) }}
            </p>
            <p
              v-if="audio && audio.existsStatus !== 'available'"
              class="error"
            >
              音频文件不可用，请点击「关联或重新定位素材」。
            </p>
            <p
              v-if="!audios.length"
              class="muted"
            >
              先在 <RouterLink :to="{ path: `/projects/${id}/writing`, query: { tab: 'audio' } }">
                音频制作
              </RouterLink> 导入音频成品；音频仅用于制作记录，不会上传抖音。
            </p>
            <label for="publish-video">最终成片</label>
            <Select
              v-model="draft.videoAssetId"
              :disabled="busy || published || recordOpen"
              @update:model-value="resetReview"
            >
              <SelectTrigger
                id="publish-video"
                class="w-full min-w-0"
              >
                <SelectValue>{{ video ? assetLabel(video) : '请选择最终成片' }}</SelectValue>
              </SelectTrigger>
              <SelectContent class="w-(--reka-select-trigger-width)">
                <SelectItem :value="null">
                  请选择最终成片
                </SelectItem>
                <SelectItem
                  v-for="file in videos"
                  :key="file.id"
                  :value="file.id"
                >
                  {{ assetLabel(file) }}
                </SelectItem>
              </SelectContent>
            </Select>
            <p
              v-if="video"
              class="file-path"
              :title="displayPath(video.filePath)"
            >
              {{ displayPath(video.filePath) }}
            </p>
            <p
              v-if="video && video.existsStatus !== 'available'"
              class="error"
            >
              视频文件不可用，请点击「关联或重新定位素材」。
            </p>
            <p
              v-if="!videos.length"
              class="muted"
            >
              可在视频制作中生成并选用成片，也可关联外部成片。发布检查会验证 MP4/MOV 文件、可读取画面和音轨。
            </p>
            <Button
              v-if="!published"
              type="button"
              variant="outline"
              @click="showAssets"
            >
              关联或重新定位素材
            </Button>
            <div class="form-grid publish-cover-selects">
              <div>
                <label for="publish-landscape">横版封面 · 4:3</label><Select
                  v-model="draft.landscapeCoverId"
                  :disabled="busy || published || recordOpen"
                  @update:model-value="draft.coversConfirmed = false"
                >
                  <SelectTrigger
                    id="publish-landscape"
                    class="w-full min-w-0"
                  >
                    <SelectValue placeholder="请选择" />
                  </SelectTrigger>
                  <SelectContent class="w-(--reka-select-trigger-width)">
                    <SelectItem :value="null">
                      请选择已导出封面
                    </SelectItem><SelectItem
                      v-for="cover in landscape"
                      :key="cover.id"
                      :value="cover.id"
                    >
                      v{{ cover.versionNo }} · {{ cover.config.title }}
                    </SelectItem>
                  </SelectContent>
                </Select>
              </div>
              <div>
                <label for="publish-portrait">竖版封面 · 3:4</label><Select
                  v-model="draft.portraitCoverId"
                  :disabled="busy || published || recordOpen"
                  @update:model-value="draft.coversConfirmed = false"
                >
                  <SelectTrigger
                    id="publish-portrait"
                    class="w-full min-w-0"
                  >
                    <SelectValue placeholder="请选择" />
                  </SelectTrigger>
                  <SelectContent class="w-(--reka-select-trigger-width)">
                    <SelectItem :value="null">
                      请选择已导出封面
                    </SelectItem><SelectItem
                      v-for="cover in portrait"
                      :key="cover.id"
                      :value="cover.id"
                    >
                      v{{ cover.versionNo }} · {{ cover.config.title }}
                    </SelectItem>
                  </SelectContent>
                </Select>
              </div>
            </div>
            <p
              v-if="!landscape.length || !portrait.length"
              class="muted"
            >
              请先在 <RouterLink :to="`/projects/${id}/covers`">
                封面工坊
              </RouterLink> 一次导出横版和竖版两张封面。
            </p>
          </section>
          <section
            class="panel publish-settings-panel"
            aria-labelledby="publish-settings-heading"
          >
            <h2 id="publish-settings-heading">
              抖音发布设置
            </h2>
            <div class="form-grid">
              <div>
                <label
                  class="check-label"
                  for="publish-allow-download"
                >
                  <Checkbox
                    id="publish-allow-download"
                    v-model="draft.allowDownload"
                    :disabled="busy || published || recordOpen"
                    aria-describedby="publish-download-help"
                  />允许保存视频
                </label>
                <p
                  id="publish-download-help"
                  class="muted"
                >
                  允许其他用户将本条作品保存到本地；取消后会在抖音设置为「不允许」。
                </p>
              </div>
              <div>
                <label for="publish-mode">发布时间</label>
                <Select
                  v-model="publishMode"
                  :disabled="busy || published || recordOpen"
                >
                  <SelectTrigger id="publish-mode">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="immediate">
                      立即发布
                    </SelectItem>
                    <SelectItem value="scheduled">
                      定时发布
                    </SelectItem>
                  </SelectContent>
                </Select>
                <template v-if="publishMode === 'scheduled'">
                  <label for="publish-scheduled-at">定时发布时间（北京时间）</label>
                  <DateTimePicker
                    id="publish-scheduled-at"
                    v-model="scheduledTime"
                    :disabled="busy || published || recordOpen"
                    :aria-invalid="!!scheduleError"
                    aria-describedby="publish-schedule-help publish-schedule-error"
                  />
                  <p
                    v-show="!!scheduleError"
                    id="publish-schedule-error"
                    class="error"
                    role="status"
                  >
                    {{ scheduleError }}
                  </p>
                </template>
                <p
                  id="publish-schedule-help"
                  class="muted"
                >
                  设置随本次任务保存并填入官方页，定时范围以当前抖音账号为准。请亲自核对并提交；定时作品实际发布后再记录成功。
                </p>
              </div>
            </div>
          </section>
          <div
            v-if="!published"
            class="publish-save"
          >
            <Button
              type="submit"
              :disabled="!!selected && !dirty"
              variant="outline"
            >
              <AppIcon name="save" />保存发布草稿
            </Button>
            <span
              class="muted"
              role="status"
            >{{ dirty ? autosave.error.value || autosave.status.value || '等待暂存…' : '每次生成都保留已有任务包' }}</span>
            <Button
              v-if="autosave.error.value"
              type="button"
              variant="outline"
              @click="run(() => autosave.flush())"
            >
              重试暂存
            </Button>
          </div>
        </fieldset>
      </form>
      <section class="panel publish-actions">
        <h2>复制发布文字</h2>
        <div class="actions wrap">
          <CopyButton
            label="复制标题"
            :copied="copiedField === 'title'"
            :disabled="!draft.title"
            @click="copy('title')"
          /><CopyButton
            label="复制内容"
            :copied="copiedField === 'description'"
            :disabled="!draft.description"
            @click="copy('description')"
          /><CopyButton
            label="复制标签"
            :copied="copiedField === 'tags'"
            :disabled="!draft.tags"
            @click="copy('tags')"
          /><CopyButton
            label="一键复制全部"
            class="publish-copy-all"
            :copied="copiedField === 'all'"
            :disabled="!draft.title || !draft.description"
            @click="copy('all')"
          />
        </div>
        <span
          class="sr-only"
          role="status"
          aria-live="polite"
        >{{ copiedField ? '已复制' : '' }}</span>
      </section>
      <section
        id="publish-preflight"
        class="panel publish-preflight"
        aria-labelledby="publish-preflight-title"
      >
        <div class="preflight-heading">
          <h2 id="publish-preflight-title">
            发布前自检
          </h2>
          <span
            class="preflight-status"
            :class="{ 'check-pending': dirty || !savedChecksPassed }"
          >
            <span class="status-dot" />
            {{ dirty ? '当前修改待保存' : savedChecksPassed ? '已保存检查全部通过' : canReview ? '素材已选齐，待确认' : '请补齐发布所需素材' }}
          </span>
        </div>
        <dl class="preflight-summary">
          <div>
            <dt>发布标题</dt><dd>{{ titleCount }} / 30 字 · {{ draft.titleConfirmed ? '已核对' : '待核对' }}</dd>
          </div>
          <div>
            <dt>发布内容</dt><dd>{{ descriptionCount }} / 1000 字</dd>
          </div>
          <div>
            <dt>保存权限</dt><dd>{{ draft.allowDownload ? '允许保存视频' : '不允许保存视频' }}</dd>
          </div>
          <div>
            <dt>发布时间</dt><dd>{{ draft.scheduledAt ? `${scheduledTime.replace('T', ' ')}（北京时间）` : publishMode === 'scheduled' ? '待选择定时时间' : '立即发布' }}</dd>
          </div>
          <div>
            <dt>制作原声</dt><dd>
              <span>{{ audio ? assetLabel(audio) : '尚未选择' }}</span><Button
                v-if="audio"
                variant="link"
                size="sm"
                :disabled="busy || audio.existsStatus !== 'available'"
                @click="run(() => openAsset(audio!.id))"
              >
                试听原声
              </Button>
            </dd>
          </div>
          <div>
            <dt>最终成片</dt><dd>
              <span>{{ video ? assetLabel(video) : '尚未选择' }}</span><span
                v-if="videoInfo"
                class="block"
              >{{ videoInfo.width }} × {{ videoInfo.height }} · {{ Math.round(videoInfo.durationSeconds) }} 秒 · {{ videoInfo.hasAudio ? '含音轨' : '缺少音轨' }}</span><span
                v-if="videoInfoError"
                class="block text-destructive"
              >{{ videoInfoError }}</span><Button
                v-if="video"
                variant="link"
                size="sm"
                :disabled="busy || video.existsStatus !== 'available'"
                @click="run(() => openAsset(video!.id, true))"
              >
                在文件夹中定位
              </Button>
            </dd>
          </div>
          <div>
            <dt>发布封面</dt><dd :class="{ 'preflight-ready': draft.coversConfirmed }">
              已选 {{ Number(!!selectedLandscape) + Number(!!selectedPortrait) }} / 2 · {{ draft.coversConfirmed ? '已核对' : '待核对横竖两张图' }}
            </dd>
          </div>
          <div>
            <dt>任务包</dt><dd :class="{ 'preflight-ready': !dirty && selected?.status !== 'draft' && selected }">
              {{ dirty ? '修改后需重新准备' : selected?.status === 'ready' ? '已就绪' : selected?.status === 'published' ? '已归档发布记录' : '尚未准备' }}
            </dd>
          </div>
        </dl>
        <div
          class="preflight-covers"
          aria-label="本次发布封面"
        >
          <div
            v-if="selectedLandscape"
            class="preflight-cover preflight-landscape"
          >
            <CoverThumbnail :cover="selectedLandscape" /><p>横版 4:3 · v{{ selectedLandscape.versionNo }}</p>
          </div>
          <div
            v-if="selectedPortrait"
            class="preflight-cover preflight-portrait"
          >
            <CoverThumbnail :cover="selectedPortrait" /><p>竖版 3:4 · v{{ selectedPortrait.versionNo }}</p>
          </div>
        </div>
        <template v-if="!published">
          <p
            id="publish-confirmation-help"
            class="notice"
            tabindex="-1"
          >
            <strong class="block">确认发布标题和两张封面</strong>
            核对本次标题、内容、成片与横竖两张封面后，点击「确认并开始辅助发布」或「确认并导出任务包」，即可一并确认并保存。
          </p>
          <p class="muted">
            应用辅助上传和填写，最终由你在抖音页面发布。
          </p>
          <p
            v-if="!canReview"
            class="text-sm text-muted-foreground"
          >
            {{ scheduleError || '请补齐文案、有效音视频与双封面，再开始发布准备。' }}
            <template v-if="scheduleError && selected?.status === 'ready'">
              若作品已经实际发布，可直接记录发布成功。
            </template>
          </p>
          <div class="actions">
            <Button
              id="publish-confirm"
              class="publish-primary-action"
              aria-describedby="publish-confirmation-help"
              :disabled="busy || recordOpen || !canReview"
              @click="assistant?.start()"
            >
              <AppIcon name="upload" />确认并开始辅助发布
            </Button>
            <Button
              id="publish-export"
              aria-describedby="publish-confirmation-help"
              variant="outline"
              :disabled="busy || recordOpen || !canReview"
              @click="prepare"
            >
              <AppIcon name="download" />确认并导出任务包
            </Button>
          </div>
          <DouyinVerification
            ref="assistant"
            class="publish-assistant"
            :desktop="desktop"
            :task="selected"
            :disabled="operationBusy || recordOpen || published"
            :prepare-task="prepareDouyinTask"
            :hide-start="true"
            @working="douyinBusy = $event"
          >
            <template #summary>
              <p class="muted">
                {{ title }} · 成片：{{ video?.originalName || '尚未选择' }}
              </p>
            </template>
          </DouyinVerification>
        </template>
        <p
          v-if="dirty && selected"
          class="notice"
        >
          当前修改尚未保存，下列检查对应已保存的任务。
        </p>
        <Collapsible
          v-if="selected"
          :key="`${selected.id}-${selected.status}`"
          :default-open="!savedChecksPassed"
          class="preflight-details"
        >
          <CollapsibleTrigger class="disclosure-trigger">
            已保存任务检查 · {{ savedChecksPassed ? '已通过' : '有待处理项目' }}
          </CollapsibleTrigger>
          <CollapsibleContent>
            <ul class="publish-checklist">
              <li
                v-for="item in selected.checklist"
                :key="item.key"
                :class="{ 'check-pending': !item.passed }"
              >
                {{ item.passed ? '✓' : '○' }} {{ item.label }}
                <Button
                  v-if="!item.passed && !published"
                  variant="ghost"
                  size="sm"
                  :disabled="busy || recordOpen"
                  @click="locateMissing(item.key)"
                >
                  {{ isConfirmationCheck(item.key) ? '去确认' : item.key === 'package' ? '去准备任务包' : '去补齐' }}
                </Button>
              </li>
            </ul>
          </CollapsibleContent>
        </Collapsible>
        <p
          v-if="selected?.taskDirectory"
          class="file-path"
        >
          {{ displayPath(selected?.taskDirectory) }}
        </p>
        <div class="actions wrap">
          <Button
            v-if="selected?.taskDirectory"
            :disabled="busy"
            variant="outline"
            type="button"
            @click="open('directory')"
          >
            打开任务包
          </Button>
          <Button
            v-if="video"
            :disabled="busy || video.existsStatus !== 'available' || dirty"
            variant="outline"
            type="button"
            @click="open('video')"
          >
            打开视频
          </Button>
          <Button
            v-if="selected?.taskDirectory"
            :disabled="busy"
            variant="outline"
            type="button"
            @click="open('landscape')"
          >
            查看横版封面
          </Button>
          <Button
            v-if="selected?.taskDirectory"
            :disabled="busy"
            variant="outline"
            type="button"
            @click="open('portrait')"
          >
            查看竖版封面
          </Button>
          <Button
            :disabled="busy"
            variant="outline"
            class="publish-portal"
            type="button"
            @click="open('portal')"
          >
            打开抖音创作者中心
          </Button>
          <Button
            v-if="!published"
            :disabled="busy || dirty || recordOpen || selected?.status !== 'ready'"
            type="button"
            @click="confirmPublish"
          >
            记录发布成功
          </Button>
        </div>
        <p class="muted">
          任务包保留视频原路径，上传和最终发布由你在平台完成。打开入口或生成任务包不会标记已发布。
        </p>
        <div
          v-if="selected?.record"
          class="notice publish-success"
        >
          <strong><AppIcon name="check" />已记录发布成功</strong><p>{{ new Date(selected.record.publishedAt).toLocaleString() }}</p><p
            v-if="selected.record.postUrl"
            class="file-path"
          >
            {{ selected.record.postUrl }}
          </p><p v-if="selected.record.notes">
            {{ selected.record.notes }}
          </p>
        </div>
      </section>
      <ConfirmAction
        v-if="recordOpen"
        class="publish-record-dialog"
        title="记录发布成功"
        confirm-label="确认已发布并保存"
        form="publish-record-form"
        :destructive="false"
        :disabled="!record.publishedAt"
        :busy="busy"
        @cancel="recordOpen = false"
      >
        <p class="publish-record-description">
          请在抖音看到作品发布成功后再保存，记录本次发布结果。
        </p>
        <form
          id="publish-record-form"
          @submit.prevent="recordSuccess()"
        >
          <fieldset
            class="publish-record-fields"
            :disabled="busy"
          >
            <div class="publish-record-field">
              <label for="published-at">实际发布时间</label>
              <DateTimePicker
                id="published-at"
                v-model="record.publishedAt"
                :time-zone="recordTimeZone"
                :disabled="busy"
              />
            </div>
            <div class="publish-record-field">
              <label for="post-url">作品链接 <span class="optional">选填</span></label>
              <Input
                id="post-url"
                v-model="record.postUrl"
                type="url"
                maxlength="2048"
                placeholder="粘贴已发布作品的链接，如 https://…"
              />
            </div>
            <div class="publish-record-field">
              <label for="publish-notes">备注 <span class="optional">选填</span></label>
              <Textarea
                id="publish-notes"
                v-model="record.notes"
                rows="3"
                maxlength="2000"
                placeholder="补充本次发布的说明，方便以后回顾"
              />
            </div>
            <div
              v-if="nextTasks.length"
              class="publish-record-next publish-record-field"
            >
              <label for="publish-next-task">接下来处理的作品 <span class="optional">选填</span></label>
              <Select
                v-model="nextTaskId"
                :disabled="busy"
              >
                <SelectTrigger
                  id="publish-next-task"
                  class="w-full min-w-0"
                  aria-describedby="publish-next-help"
                >
                  <SelectValue placeholder="选择下一篇待发布作品" />
                </SelectTrigger>
                <SelectContent class="w-(--reka-select-trigger-width)">
                  <SelectItem
                    v-for="task in nextTasks"
                    :key="task.id"
                    :value="task.id"
                  >
                    {{ task.projectTitle }} · {{ task.title }}
                  </SelectItem>
                </SelectContent>
              </Select>
              <p
                id="publish-next-help"
                class="publish-record-help"
              >
                先保存本次记录，再开始所选作品的辅助发布。
              </p>
            </div>
          </fieldset>
        </form>
        <template #actions>
          <Button
            v-if="nextTasks.length"
            id="publish-record-next"
            type="button"
            form="publish-record-form"
            variant="outline"
            :disabled="busy || !nextTaskId || !record.publishedAt"
            @click="($event.currentTarget as globalThis.HTMLButtonElement).form?.reportValidity() && recordSuccess(true)"
          >
            确认记录并准备下一篇
            <AppIcon name="arrow" />
          </Button>
        </template>
      </ConfirmAction>
    </template>
  </div>
</template>

<style scoped>
:global(.publish-record-dialog) { max-width: 600px; gap: 0; border-radius: 16px; background: var(--card); }
:global(.publish-record-dialog > h2) { margin: 0; font-size: 20px; line-height: 1.5; }
:global(.publish-record-dialog > .actions) { justify-content: flex-end; gap: 10px; margin-top: 24px; padding-top: 20px; border-top: 1px solid var(--border); }
.publish-record-description { margin: 8px 0 24px; color: var(--muted-foreground); font-size: 13px; line-height: 1.7; }
.publish-record-fields { display: grid; gap: 20px; }
.publish-record-field { display: grid; min-width: 0; gap: 8px; }
.publish-record-field label { display: flex; align-items: baseline; gap: 8px; margin: 0; color: var(--foreground); }
.publish-record-field .optional { margin-left: auto; }
.publish-record-next { padding: 16px; border: 1px solid var(--border); border-radius: 12px; background: var(--accent); }
.publish-record-help { margin: 0; color: var(--muted-foreground); font-size: 12px; line-height: 1.6; }
@media (max-width: 560px) {
  :global(.publish-record-dialog) { padding: 20px; }
  :global(.publish-record-dialog > .actions) { flex-direction: column; align-items: stretch; }
}

.publishing-page { display: grid; gap: 24px; min-width: 0; padding-bottom: 80px; }
.publishing-page > .page-return-row, .publishing-page > .page-heading, .publishing-page > :deep(.project-navigation) { margin-bottom: 0; }
.publishing-page .panel { margin: 0; padding: 24px; box-shadow: var(--panel-shadow); }
.publishing-page h2 { color: var(--heading); font-family: var(--font-mono); }
.publishing-page .actions { margin-top: 16px; }
.publishing-page :deep(button), .publishing-page input, .publishing-page textarea { font-family: var(--font-mono); }
.publishing-page :deep(button[data-slot='button']) { border-radius: 12px; }
.publish-return { flex-wrap: wrap; }
.publish-return-actions, .publish-title-row { display: flex; align-items: center; flex-wrap: wrap; gap: 12px; }
.publish-return-actions .back-link { font-size: 12px; }
.publish-stage-badge, .publish-step-badge, .publish-task-status { border: 1px solid var(--active-border); background: var(--accent); color: var(--primary); font: 600 12px/16px var(--font-mono); }
.publish-stage-badge { gap: 6px; padding: 4px 12px; border-radius: 999px; font-weight: 400; }
.publish-stage-badge .app-icon { width: 14px; height: 14px; }
.publish-step-badge { border-radius: 4px; padding: 2px 10px; }
.publish-task-status { padding: 2px 10px; border-radius: 4px; }
.publish-task-status[data-status='draft'] { color: var(--warning); background: var(--warning-surface); border-color: var(--cover-border); }
.publish-task-status[data-status='ready'] { color: var(--info); background: color-mix(in srgb, var(--info) 10%, transparent); border-color: color-mix(in srgb, var(--info) 35%, transparent); }
.publish-task-panel .result-heading { margin-bottom: 16px; }
.publish-task-panel label { margin: 0 0 6px; }
.publish-task-panel :deep([data-slot='select-trigger']) { height: 44px; }
.publishing-page :deep([data-slot='select-trigger']), .publishing-page input, .publishing-page textarea { border-radius: 12px; background: var(--input-surface); border-color: var(--input-border); }
.publishing-page :deep([data-slot='select-value']) { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; display: block; }
.publishing-page :deep([data-slot='select-trigger']:focus-visible), .publishing-page input:focus-visible, .publishing-page textarea:focus-visible { border-color: var(--info); }
.publish-form-grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 24px; }
.publish-form-grid .panel { box-shadow: none; }
.publish-form-grid h2 { margin: 0 0 18px; }
.publish-form-grid label { margin: 16px 0 6px; }
.publish-form-grid h2 + label { margin-top: 0; }
.publish-settings-panel { grid-column: 1 / -1; }
.publish-settings-panel .form-grid > div { min-width: 0; }
.publish-settings-panel .form-grid label:first-child { margin-top: 0; }
.publish-form-grid .muted { margin: 6px 0 0; font: 400 11px/1.65 var(--font-mono); }
.publish-form-grid input { padding: 10px 14px; font-size: 12px; font-weight: 400; }
.publish-form-grid textarea { display: block; padding: 14px; min-height: 146px; }
.publish-copy-panel :deep(#publish-writing-version) { color: var(--info); }
.publish-field-label { display: flex; align-items: baseline; justify-content: space-between; gap: 12px; margin: 16px 0 6px; }
.publish-field-label label { margin: 0; color: var(--foreground); font-weight: 700; }
.publish-field-label > span { color: var(--muted-foreground); font: 400 11px/16px var(--font-mono); }
.publish-copy-panel label[for='publish-tags'] { color: var(--foreground); font-weight: 700; }
#publish-tags { color: var(--primary); }
.publish-media-panel > button { margin-top: 16px; }
.publish-cover-selects { margin-top: 4px; }
.publish-cover-selects > div { min-width: 0; }
.publish-cover-selects :deep([data-slot='select-trigger']) { color: var(--cover-accent); }
.publishing-page .file-path { padding: 12px; margin: 10px 0 0; border: 1px solid var(--border); border-radius: 12px; background: color-mix(in srgb, var(--terminal) 80%, transparent); color: var(--muted-foreground); font: 400 11px/1.65 var(--font-mono); overflow-wrap: anywhere; }
.publish-save { grid-column: 1 / -1; display: flex; align-items: center; flex-wrap: wrap; justify-content: space-between; gap: 12px; }
.publish-save .muted { margin: 0; font: 400 11px/1.65 var(--font-mono); }
.is-published .publish-form-grid :deep(:disabled) { opacity: 1; }
.publish-actions h2 { margin: 0; }
.publish-actions :deep(.app-icon) { color: var(--info); }
.publishing-page :deep(.publish-copy-all), .publishing-page .publish-primary-action { border-color: transparent; background: var(--action-gradient); color: var(--primary-foreground); font-weight: 800; box-shadow: var(--primary-glow); }
.publish-actions :deep(.publish-copy-all .app-icon) { color: inherit; }
.publishing-page :deep(.publish-copy-all:hover), .publishing-page .publish-primary-action:hover { filter: brightness(1.1); }
.publish-preflight { scroll-margin-top: 90px; }
.preflight-heading { display: flex; flex-wrap: wrap; align-items: center; justify-content: space-between; gap: 12px; padding-bottom: 12px; border-bottom: 1px solid var(--border); }
.preflight-heading h2 { margin: 0; }
.preflight-status { display: inline-flex; align-items: center; gap: 6px; color: var(--primary); font: 400 12px/18px var(--font-mono); }
.preflight-status .status-dot { background: currentColor; }
.preflight-summary { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); margin: 24px 0 0; gap: 12px 32px; font: 400 12px/1.65 var(--font-mono); }
.preflight-summary > div { display: flex; justify-content: space-between; align-items: baseline; gap: 16px; min-width: 0; padding: 4px 0; border-bottom: 1px solid color-mix(in srgb, var(--border) 50%, transparent); }
.preflight-summary dt { flex-shrink: 0; color: var(--muted-foreground); }
.preflight-summary dd { min-width: 0; margin: 0; text-align: right; overflow-wrap: anywhere; }
.preflight-summary dd > span:first-child { display: block; }
.preflight-summary :deep(button) { height: auto; padding: 0; color: var(--info); font-size: 12px; }
.preflight-ready { color: var(--primary); font-weight: 700; }
.preflight-covers { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); align-items: start; gap: 24px; max-width: 672px; margin-top: 28px; }
.preflight-covers:empty { display: none; }
.preflight-cover { min-width: 0; }
.preflight-cover :deep(figure) { margin: 0; padding: 14px; border-radius: 12px; border-color: var(--input-border); background: var(--terminal); }
.preflight-cover :deep(img) { width: 100%; max-height: none; border-radius: 8px; object-fit: contain; }
.preflight-landscape :deep(img) { aspect-ratio: 4 / 3; }
.preflight-portrait :deep(img) { aspect-ratio: 3 / 4; }
.preflight-cover :deep(figcaption) { margin-top: 12px; color: var(--foreground); font: 700 12px/18px var(--font-mono); }
.preflight-cover :deep(figcaption span) { font-size: 11px; font-weight: 400; }
.preflight-cover > p { margin: 8px 4px 0; color: var(--muted-foreground); font: 400 12px/18px var(--font-mono); }
#publish-confirmation-help { margin-top: 24px; }
.publish-preflight .publish-assistant { margin-top: 24px; padding: 20px; background: var(--input-surface); box-shadow: none; }
.publish-assistant :deep(.douyin-steps) { font-size: 12px; }
.preflight-details { margin-top: 28px; }
.preflight-details .disclosure-trigger { min-height: 32px; }
.publish-checklist { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 8px 20px; margin: 12px 0 0; padding: 0; list-style: none; font: 400 12px/2 var(--font-mono); }
.publish-checklist li { padding: 8px 0; overflow-wrap: anywhere; }
.check-pending { color: var(--warning); }
.publish-preflight .publish-portal { background: linear-gradient(90deg, #e11d48, #db2777, #9333ea); border-color: transparent; color: #fff; font-weight: 700; box-shadow: var(--video-glow); }
.publish-preflight .publish-portal:hover { filter: brightness(1.1); }
.publish-preflight > .muted { font: 400 11px/1.65 var(--font-mono); }
.publish-success { margin: 24px 0 0; font-family: var(--font-mono); }
.publish-success strong { display: flex; align-items: center; gap: 8px; }
.publish-success p { margin: 4px 0 0; font-size: 11px; }
@media (max-width: 1100px) {
  .publish-form-grid { grid-template-columns: minmax(0, 1fr); }
  .preflight-summary { grid-template-columns: minmax(0, 1fr); }
}
@media (max-width: 560px) {
  .publishing-page { gap: 20px; padding-bottom: 32px; }
  .publishing-page .panel { padding: 20px; }
  .publish-return-actions { width: 100%; justify-content: space-between; }
  .publish-return-actions .back-link, .publish-stage-badge { font-size: 11px; }
  .publish-title-row h1 { font-size: 26px; line-height: 1.4; }
  .publish-cover-selects, .preflight-covers, .publish-checklist { grid-template-columns: minmax(0, 1fr); }
  .preflight-covers { max-width: 320px; }
  .preflight-summary > div { gap: 10px; }
  .publishing-page .actions > :deep(button), .publish-save > button { flex: 1 1 auto; }
}
@media (prefers-reduced-transparency: reduce) {
  .publishing-page .panel { background: var(--card); backdrop-filter: none; }
}
</style>
