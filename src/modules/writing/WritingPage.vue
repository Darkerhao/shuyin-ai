<script setup lang="ts">
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '@/components/ui/collapsible'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { Textarea } from '@/components/ui/textarea'
import { displayPath } from '@/lib/displayPath'
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { RouterLink, onBeforeRouteLeave, onBeforeRouteUpdate, useRouter, useRoute } from 'vue-router'
import * as api from '../../services/ai'
import * as audioApi from '../../services/audio'
import { chooseAudio, chooseSourceDocument, getProject, getChapter, isDesktop, linkAsset, listAssets } from '../../services/projects'
import { errorMessage } from '../../services/verification'
import { emptyPublishing, type ModelProvider, type PublishingContent, type PublishingField, type PublishingTextField, type WritingVersion } from '../../types/ai'
import { generationJobs, generationKey, isGenerating, startWritingGeneration, cancelGenerationJob, dismissGeneration } from '../../services/generation'
import { automationRuns, refreshAutomation, stages } from '../../services/automation'
import CoverDesignPreview from '../../components/CoverDesignPreview.vue'
import type { Asset } from '../../types/project'
import type { AudioFormat, AudioJob, AudioJobAction, AudioJobStatus, AudioJobView } from '../../types/audio'
import { useOperation } from '../../composables/useOperation'
import { useQueryState } from '../../composables/useQueryState'
import { useEditorDraft } from '../../composables/useEditorDraft'
import { readDraft } from '../../services/drafts'
import AppIcon from '../../components/AppIcon.vue'
import AudioPlayer from '../../components/AudioPlayer.vue'
import ProjectNavigation from '../../components/ProjectNavigation.vue'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import OperationFeedback from '../../components/OperationFeedback.vue'
import ConfirmAction from '../../components/ConfirmAction.vue'
import { requestConfirmation } from '../../composables/useConfirmation'
import { useCopyFeedback } from '../../composables/useCopyFeedback'
import CopyButton from '../../components/CopyButton.vue'
import { chapterIdFromQuery, chapterOwner, chapterQuery, belongsToChapter, type Chapter } from '../../types/chapter'
const props = defineProps<{ id: string }>()
const route = useRoute()
const chapterId = computed(() => chapterIdFromQuery(route.query.chapter))
const chapter = ref<Chapter | null>(null)
const scopeInput = computed(() => chapterId.value ? { chapterId: chapterId.value } : {})
const readAudioView = () => audioApi.getAudioJobView(props.id, typeof route.query.audioJob === 'string' ? route.query.audioJob : undefined, chapterId.value)
const router = useRouter()
const desktop = isDesktop()
const title = ref('')
const generation = computed(() => generationJobs.get(generationKey(props.id, 'writing', chapterId.value)))
const automaticRun = computed(() => automationRuns.value.find(item => item.projectId === props.id && belongsToChapter(item, chapterId.value) && item.input.workflowVersion === 2 && !['completed', 'cancelled'].includes(item.status)))
const generating = computed(() => isGenerating(generation.value))
const ready = ref(false)
const providers = ref<ModelProvider[]>([])
const providerId = ref('')
const versions = ref<WritingVersion[]>([])
const audioAssets = ref<Asset[]>([])
const documents = ref<Asset[]>([])
const sourceNotes = ref('')
const audioView = ref<AudioJobView | null>(null)
const audioConfirmation = ref<{ job: AudioJob; action: 'cancel' | 'confirm_notebook'; url?: string } | null>(null)
const audioSourceKey = ref('')
const audioFormat = ref<AudioFormat>('deep_dive')
const clock = ref(Date.now())
let audioPollTimer: ReturnType<typeof globalThis.setInterval> | undefined
let clockTimer: ReturnType<typeof globalThis.setInterval> | undefined
const selected = ref<WritingVersion | null>(null)
const draft = ref(emptyPublishing())
const original = ref(JSON.stringify(draft.value))
const dirty = computed(() => JSON.stringify(draft.value) !== original.value)
const taskId = computed(() => generating.value ? generation.value?.id ?? '' : '')
const liveContent = computed(() => generation.value?.preview ?? {})
const cancelling = computed(() => generation.value?.cancelling ?? false)
const revisionOpen = ref(false)
const revisionFields = ref<PublishingField[]>(['title'])
const revisionInstruction = ref('')
const revising = computed(() => generating.value && generation.value?.mode === 'revision')
const preview = computed(() => {
  const result = generation.value?.result
  return result && 'taskId' in result ? result : null
})
const revisionLocked = computed(() => revising.value || !!preview.value)
const activeTab = useQueryState('tab', 'publishing', ['publishing', 'audio', 'cover'])
const clipboard = useCopyFeedback()
const copiedField = clipboard.copied
const copyLabel = ref('')
const historyOpen = ref(false)
const historyVersionId = ref<string | null>(null)
const historyVersion = computed(() => versions.value.find(version => version.id === historyVersionId.value))
const historyAudioCompatible = computed(() => !chapterId.value || (!!historyVersion.value?.sourceSnapshot && !!selected.value?.sourceSnapshot
  && JSON.stringify({ ...historyVersion.value.sourceSnapshot, targetDurationSeconds: null }) === JSON.stringify({ ...selected.value.sourceSnapshot, targetDurationSeconds: null })))
const candidateVersion = computed(() => versions.value[0]?.sourceType === 'ai' && versions.value[0]?.id !== selected.value?.id ? versions.value[0] : null)
const candidateVisible = computed(() => !preview.value && (generating.value || !!candidateVersion.value))
const candidateContent = computed(() => generating.value ? liveContent.value : candidateVersion.value?.content ?? {})
const candidateStatus = computed(() => generating.value ? Object.keys(liveContent.value).length ? '正在接收' : '等待返回' : '生成完成')
const versionSource = (version: WritingVersion) => version.sourceType === 'ai' ? 'AI 生成' : version.sourceType === 'mixed' ? 'AI 生成后修改' : '手动修改'
async function viewCandidate() {
  historyVersionId.value = candidateVersion.value?.id ?? null
  historyOpen.value = true
  await nextTick()
  globalThis.document.getElementById('writing-version')?.focus()
}
const savedFlash = ref(false)
const waitingSeconds = ref(0)
let savedTimer: ReturnType<typeof globalThis.setTimeout> | undefined
let waitingTimer: ReturnType<typeof globalThis.setInterval> | undefined
watch(taskId, id => {
  globalThis.clearInterval(waitingTimer)
  if (!id) return
  const started = generation.value?.startedAt ?? Date.now(); waitingSeconds.value = Math.floor((Date.now() - started) / 1000)
  waitingTimer = globalThis.setInterval(() => { waitingSeconds.value = Math.floor((Date.now() - started) / 1000) }, 1000)
}, { immediate: true })
function flashSave() {
  globalThis.clearTimeout(savedTimer); savedFlash.value = true
  savedTimer = globalThis.setTimeout(() => { savedFlash.value = false }, 1000)
}
const tabs = [
  { value: 'publishing', label: '发布文案', icon: 'pen' },
  { value: 'audio', label: '音频制作', icon: 'headphones' },
  { value: 'cover', label: '封面素材', icon: 'image' },
] as const
const { busy: operationBusy, error, message, operation, run } = useOperation()
const busy = computed(() => operationBusy.value || generating.value)
// Preserve the current source when a newer history entry has not been adopted.
const draftRequired = computed(() => dirty.value || selected.value?.id !== versions.value[0]?.id || !!generation.value)
const autosave = useEditorDraft(() => chapterOwner(props.id, chapterId.value), 'writing', draft, () => selected.value?.id ?? null, draftRequired, ready, prepareLeave)
const audioBrowser = computed(() => audioView.value?.browser ?? null)
const audioJob = computed(() => audioView.value?.job ?? null)
const activeAudioJob = computed(() => {
  const job = audioView.value?.activeJob
  return job && !['succeeded', 'cancelled'].includes(job.status) ? job : null
})
const otherActiveAudioJob = computed(() => activeAudioJob.value && (activeAudioJob.value.projectId !== props.id || !belongsToChapter(activeAudioJob.value, chapterId.value)) ? activeAudioJob.value : null)
const shownAudioJob = computed(() => route.query.audioJob && audioJob.value ? audioJob.value : otherActiveAudioJob.value ?? audioJob.value)
const completedAudioId = computed(() => shownAudioJob.value?.projectId === props.id && shownAudioJob.value.status === 'succeeded' ? shownAudioJob.value.assetId : null)
const completedPlayer = ref<InstanceType<typeof AudioPlayer>>()
const selectedDocument = computed(() => documents.value.find(item => `asset:${item.id}` === audioSourceKey.value) ?? null)
const audioSourceReady = computed(() => audioSourceKey.value === 'text' || audioSourceKey.value === 'chapter'
  ? !!sourceNotes.value.trim()
  : !!selectedDocument.value && selectedDocument.value.existsStatus === 'available')
const audioFieldsReady = computed(() => [draft.value.title, draft.value.description, draft.value.audioPrompt].every(value => value.trim()))
const canStartAudio = computed(() => desktop && ready.value && !busy.value && !!audioBrowser.value?.supported
  && !activeAudioJob.value && !automaticRun.value && audioFieldsReady.value
  && audioSourceReady.value)
const liveAudioStatuses = new Set<AudioJobStatus>(['created', 'waiting_login', 'preparing', 'generating', 'downloading', 'importing'])
const audioStatusLabels: Record<AudioJobStatus, string> = {
  created: '等待接续', waiting_login: '等待 Google 登录', preparing: '准备 Notebook', generating: '正在生成',
  downloading: '正在下载', importing: '正在归档', succeeded: '音频已保存', waiting_user: '等待人工核对',
  paused: '已暂停跟进', failed: '任务失败', cancelled: '已停止跟进',
}
const audioStageLabels: Record<string, string> = {
  opening_site: '正在打开网站', awaiting_extension: '等待在应用内继续', login: '等待 Google 登录', creating_notebook: '正在创建笔记本',
  adding_source: '添加原书来源', source_processing: '等待来源处理', configuring_audio: '填写音频指导',
  submitting: '提交音频生成', generating: 'Gemini Notebook 正在生成音频', downloading: '获取音频下载',
  importing: '复制并登记音频', verify_remote: '等待人工核对远端任务', completed: '归档完成', error: '任务已停止',
}
function accept(version: WritingVersion | null) {
  selected.value = version
  draft.value = { ...emptyPublishing(), ...version?.content }
  original.value = JSON.stringify(draft.value)
}
function canDiscard(returnFocusId?: string) { return (!dirty.value && !preview.value) || requestConfirmation(preview.value ? '候选结果尚未采用，确定放弃？' : '当前内容尚未保存，确定放弃修改？', returnFocusId) }
function prepareLeave() { return !operationBusy.value }
const canLeave = async () => {
  if (!prepareLeave()) return false
  try { if (ready.value) await autosave.flush(); return true } catch { return false }
}
onBeforeRouteLeave(canLeave)
onBeforeRouteUpdate((to, from) => (to.params.id === from.params.id && to.query.chapter === from.query.chapter && to.query.audioJob === from.query.audioJob && to.query.writingVersion === from.query.writingVersion) || canLeave())
function beforeUnload(event: globalThis.BeforeUnloadEvent) { if (dirty.value || busy.value || preview.value) event.preventDefault() }
globalThis.addEventListener('beforeunload', beforeUnload)
onBeforeUnmount(() => { globalThis.removeEventListener('beforeunload', beforeUnload); globalThis.clearTimeout(savedTimer); globalThis.clearInterval(waitingTimer) })
function setAssets(files: Asset[]) {
  documents.value = files.filter(asset => asset.assetType === 'document')
  audioAssets.value = files.filter(asset => asset.assetType === 'audio')
  if (chapterId.value) { audioSourceKey.value = 'chapter'; return }
  if (audioSourceKey.value.startsWith('asset:') && documents.value.some(item => `asset:${item.id}` === audioSourceKey.value)) return
  if (audioSourceKey.value === 'text') {
    if (sourceNotes.value.trim()) return
    audioSourceKey.value = ''
  }
  const available = documents.value.find(item => item.existsStatus === 'available')
  audioSourceKey.value = available ? `asset:${available.id}` : sourceNotes.value.trim() ? 'text' : ''
}
async function refreshAudioAssets() {
  setAssets(await listAssets(props.id, chapterId.value))
}
function applyAudioView(next: AudioJobView, refreshOnSuccess = true) {
  const previous = audioView.value
  audioView.value = next
  const nextJob = shownAudioJob.value
  const previousJob = [previous?.job, previous?.activeJob].find(job => job?.id === nextJob?.id)
  if (previous && nextJob?.errorMessage && ['waiting_user', 'failed'].includes(nextJob.status) &&
    (previousJob?.errorMessage !== nextJob.errorMessage || previousJob?.errorCode !== nextJob.errorCode || previousJob?.runId !== nextJob.runId)) {
    error.value = `《${nextJob.projectTitle}》的音频任务：${nextJob.errorMessage}`
  }
  if (refreshOnSuccess && next.job?.status === 'succeeded' &&
    (previous?.job?.status !== 'succeeded' || previous?.job?.assetId !== next.job.assetId)) {
    void refreshAudioAssets().catch(cause => { error.value = errorMessage(cause) })
  }
}
async function refreshAudio(showFailure = false) {
  try { applyAudioView(await readAudioView()) }
  catch (cause) { if (showFailure) error.value = errorMessage(cause) }
}
function startAudioTimers() {
  if (!desktop || audioPollTimer) return
  audioPollTimer = globalThis.setInterval(() => {
    if (!globalThis.document.hidden) void refreshAudio()
  }, 15_000)
  clockTimer = globalThis.setInterval(() => { clock.value = Date.now() }, 30_000)
}
function stopAudioTimers() {
  globalThis.clearInterval(audioPollTimer)
  globalThis.clearInterval(clockTimer)
  audioPollTimer = undefined
  clockTimer = undefined
}
function refreshAudioOnFocus() { if (desktop) void refreshAudio() }
function refreshAudioOnVisibility() { if (desktop && !globalThis.document.hidden) void refreshAudio() }
const load = () => run(async () => {
  ready.value = false
  const [project, models, saved, recovery, files, savedAudio, sourceChapter] = await Promise.all([getProject(props.id), api.listProviders(), api.listVersions(props.id, 'publishing', chapterId.value), readDraft<PublishingContent>(chapterOwner(props.id, chapterId.value), 'writing'), listAssets(props.id, chapterId.value), readAudioView(), chapterId.value ? getChapter(props.id, chapterId.value) : null])
  if (chapterId.value && !sourceChapter) throw new Error('章节内容无法读取，请返回章节资料检查。')
  chapter.value = sourceChapter
  title.value = sourceChapter ? `${project.title} · ${sourceChapter.title}` : project.title
  sourceNotes.value = sourceChapter ? sourceChapter.content : project.sourceNotes
  setAssets(files)
  // Restore task history without replaying past errors as new notifications.
  audioView.value = savedAudio
  providers.value = models.filter(p => p.isEnabled && p.modelKind === 'text')
  providerId.value = providers.value.find(p => p.isDefault)?.id ?? providers.value[0]?.id ?? ''
  versions.value = saved
  accept(recovery ? saved.find(v => v.id === recovery.basedOn) ?? null : saved[0] ?? null)
  if (recovery) {
    draft.value = { ...emptyPublishing(), ...recovery.content }
    if (dirty.value) message.value = '已恢复自动暂存的草稿，保存修改后才会新增历史版本。'
  }
  if (typeof route.query.writingVersion === 'string') {
    const requested = saved.find(version => version.id === route.query.writingVersion)
    if (!requested) error.value = '本次创作版本不可读取，请保留当前稿并检查任务。'
    else if (recovery) {
      historyVersionId.value = requested.id; historyOpen.value = true
      message.value = '本次创作方案已在历史对照中打开，当前暂存稿保持不变。'
    } else accept(requested)
  }
  if (generation.value?.revision) {
    revisionOpen.value = true
    revisionFields.value = [...generation.value.revision.fields]
    revisionInstruction.value = generation.value.revision.instruction
  }
  ready.value = true
})
watch([ready, () => generation.value?.status], () => {
  const job = generation.value
  if (!ready.value || !job || isGenerating(job)) return
  if (job.error) {
    error.value = job.error
    dismissGeneration(job)
    return
  }
  const result = job.result
  if (!result || 'assetType' in result) return
  if ('taskId' in result) {
    revisionOpen.value = true
    revisionFields.value = [...result.fields]
    return
  }
  versions.value = [result, ...versions.value.filter(version => version.id !== result.id)]
  const first = !selected.value && !dirty.value
  if (first) accept(result)
  historyVersionId.value = result.id; historyOpen.value = !first
  message.value = first ? `已生成并保存 v${result.versionNo}，可继续编辑。` : `已保存备选 v${result.versionNo}。当前内容保留，可在版本对照中取用。`
  dismissGeneration(job)
})
async function switchVersion(id: string | null) {
  if (await canDiscard('writing-version')) accept(versions.value.find(v => v.id === id) ?? null)
}
async function persistWriting() {
  await autosave.flush()
  const saved = await api.saveWriting({ projectId: props.id, ...scopeInput.value, kind: 'publishing', content: { ...draft.value }, basedOn: selected.value?.id ?? null, isConfirmed: false })
  accept(saved); versions.value = [saved, ...versions.value]
  await autosave.flush()
}
const save = () => run(async () => { await persistWriting(); flashSave() })
function audioStatusLabel(job: AudioJob) { return audioStatusLabels[job.status] }
function audioStageLabel(job: AudioJob) { return audioStageLabels[job.stage] ?? job.stage }
function audioElapsed(job: AudioJob) {
  const started = Date.parse(job.submittedAt ?? job.createdAt)
  if (!Number.isFinite(started)) return '等待时间未知'
  const seconds = Math.max(0, Math.floor((clock.value - started) / 1000))
  if (seconds < 60) return `${seconds} 秒`
  const minutes = Math.floor(seconds / 60)
  if (minutes < 60) return `${minutes} 分钟`
  return `${Math.floor(minutes / 60)} 小时 ${minutes % 60} 分钟`
}
function audioCheckedAt(job: AudioJob) { return job.lastCheckedAt ? new Date(job.lastCheckedAt).toLocaleString() : '尚未检查' }
function isLiveAudio(job: AudioJob) { return liveAudioStatuses.has(job.status) }
function canResumeAudio(job: AudioJob) {
  return ['paused', 'waiting_user', 'failed', 'cancelled'].includes(job.status)
    && (!activeAudioJob.value || activeAudioJob.value.id === job.id)
}
function needsDownloadRetry(job: AudioJob) { return job.status === 'waiting_user' && !!job.errorCode?.includes('download') }
const addSourceDocument = () => run(async () => {
  const path = await chooseSourceDocument()
  if (!path) return
  const id = await linkAsset(props.id, path)
  await refreshAudioAssets()
  const linked = documents.value.find(item => item.id === id)
  if (!linked) throw new Error('所选文件不是支持的书籍资料。')
  audioSourceKey.value = `asset:${linked.id}`
  message.value = '原书文件已加入本书来源，启动自动任务时会核验文件指纹。'
})
const startAudio = () => run(async () => {
  if (!audioBrowser.value?.supported) throw new Error(audioBrowser.value?.message || '当前平台暂不支持应用内自动音频。')
  if (!audioSourceReady.value) throw new Error('请选择可用的原书文件或书籍资料文本。')
  if (dirty.value || !selected.value) await persistWriting()
  if (!selected.value) throw new Error('请先保存一套音频提示词。')
  const source = chapterId.value ? { kind: 'chapter' as const, chapterId: chapterId.value } : audioSourceKey.value === 'text'
    ? { kind: 'text' as const }
    : { kind: 'asset' as const, assetId: selectedDocument.value!.id }
  applyAudioView(await audioApi.startAudioJob({ projectId: props.id, writingVersionId: selected.value.id, source, format: audioFormat.value }))
  message.value = '自动音频任务已启动。请按提示完成 Google 登录，后续生成和下载会持续跟进。'
})
const controlAudio = (job: AudioJob, action: AudioJobAction) => {
  if (action === 'cancel') { audioConfirmation.value = { job, action }; return }
  return run(async () => {
    if (action === 'resume' && !job.notebookUrl && ['creating_notebook', 'verify_remote'].includes(job.stage)) {
      const view = await readAudioView()
      applyAudioView(view)
      const url = view.browser.url
      if (!view.browser.open || !url) throw new Error('请先点击“查看网站”，打开原任务的空白笔记本，再继续跟进。')
      audioConfirmation.value = { job, action: 'confirm_notebook', url }
      return
    }
    applyAudioView(await audioApi.controlAudioJob(job.id, action))
    message.value = action === 'open_browser' ? '已显示应用内任务网站。'
      : action === 'refresh_browser' || action === 'reopen_browser' ? job.status === 'succeeded' ? '已重新加载任务网站。' : '已重新加载任务网站。请核对页面，准备好后点击“继续跟进”。'
      : action === 'pause' ? '已暂停本地跟进，Google 中的生成状态不受影响。'
        : action === 'resume' ? '已继续跟进当前任务。'
          : '已停止本地跟进。Google 中已提交的生成可能仍在继续。'
  }, action === 'resume' ? 'resumeAudio' : '')
}
const confirmAudio = () => run(async () => {
  const pending = audioConfirmation.value
  if (!pending) return
  applyAudioView(await (pending.url
    ? audioApi.controlAudioJob(pending.job.id, pending.action, pending.url)
    : audioApi.controlAudioJob(pending.job.id, pending.action)))
  audioConfirmation.value = null
  message.value = pending.action === 'cancel' ? '已停止本地跟进。Google 中已提交的生成可能仍在继续。' : '已关联当前笔记本并继续原任务。'
})
const importAudio = () => run(async () => {
  const path = await chooseAudio()
  if (!path) return
  await linkAsset(props.id, path, null, chapterId.value)
  await refreshAudioAssets()
  message.value = '音频已导入。可在视频制作中识别字幕并生成原声 MG 视频，也可使用外部剪辑工具。'
})
function playCompletedAudio() {
  completedPlayer.value?.$el.scrollIntoView?.({ block: 'center' })
  void completedPlayer.value?.play()
}
const generate = () => run(async () => {
  await startWritingGeneration({ projectId: props.id, ...scopeInput.value, providerId: providerId.value }, title.value)
})
function openRevision() { revisionOpen.value = true }
function selectRevision(field: PublishingField, checked: boolean | 'indeterminate') {
  revisionFields.value = checked === true ? [...revisionFields.value, field] : revisionFields.value.filter(key => key !== field)
}
const generateRevision = () => run(async () => {
  if (dirty.value) await persistWriting()
  if (!selected.value) throw new Error('请先生成或保存一套内容。')
  await startWritingGeneration({ projectId: props.id, ...scopeInput.value, providerId: providerId.value, revision: { basedOn: selected.value.id, fields: [...revisionFields.value], instruction: revisionInstruction.value.trim() } }, title.value)
})
const applyRevision = () => run(async () => {
  if (!preview.value) return
  const saved = await api.applyGeneration(preview.value.taskId)
  accept(saved); versions.value = [saved, ...versions.value.filter(v => v.id !== saved.id)]
  if (generation.value) dismissGeneration(generation.value)
  revisionOpen.value = false
  message.value = '已采用选中内容并保存新版本，其余内容保持原样。'
})
function closeRevision() { if (generation.value) dismissGeneration(generation.value); revisionOpen.value = false }
async function cancel() {
  if (!generation.value) return
  try { await cancelGenerationJob(generation.value) }
  catch { error.value = '取消失败，请稍后重试。' }
}
async function makeCover() {
  if (dirty.value) await save()
  if (!dirty.value && selected.value) await router.push({ path: `/projects/${props.id}/covers`, query: { ...chapterQuery(chapterId.value), writingVersion: selected.value.id } })
}
async function copy(field: PublishingTextField, label: string) {
  copyLabel.value = label
  error.value = ''; message.value = ''
  await clipboard.copy(draft.value[field], field)
  error.value = clipboard.error.value
}
const fields: { key: PublishingTextField; label: string; rows: number; limit: number; help?: string }[] = [
  { key: 'title', label: '发布标题', rows: 2, limit: 30 },
  { key: 'description', label: '发布内容', rows: 7, limit: 1000 },
  { key: 'tags', label: '发布标签', rows: 2, limit: 500, help: 'AI 会生成与书籍和本期主题相关的 #标签，可修改或清空。选择文案来源后自动带入发布准备，辅助发布时追加到抖音内容末尾。旧文案可通过按项调整单独生成标签。' },
  { key: 'audioPrompt', label: '音频生成提示词', rows: 18, limit: 20000, help: '自动任务会把这里已保存的完整指导写入 Gemini Notebook；也可手动复制到网站。网站格式和时间档位不保证精确分钟或单人效果。' },
  { key: 'coverTitle', label: '封面主标题', rows: 2, limit: 30, help: '提炼本书的悬念或看点，与发布标题分开使用。旧版本没有封面内容，可再次一键生成，也可手动填写。' },
  { key: 'coverSubtitle', label: '封面副标题', rows: 2, limit: 60, help: '用书名和具体看点补充主标题，可直接带入制作封面。' },
  { key: 'coverPromptLandscape', label: '横版 4:3 封面提示词', rows: 7, limit: 2000, help: '用于 1440 × 1080 横版构图，复制到图片工具并选择 4:3。只生成无字背景，主副标题在制作封面时添加。' },
  { key: 'coverPromptPortrait', label: '竖版 3:4 封面提示词', rows: 7, limit: 2000, help: '用于 1080 × 1440 竖版构图，复制到图片工具并选择 3:4。与横版保持同一主题，分别安排主体和留白。' },
  { key: 'coverPromptWidescreen', label: '横版 16:9 封面提示词', rows: 7, limit: 2000, help: '用于 1920 × 1080 宽屏构图，独立安排横向叙事与文字留白。旧内容可通过按项调整单独补充，不改动其他结果。' },
]
const revisionOptions: { key: PublishingField; label: string }[] = [...fields, { key: 'coverDesign', label: '封面字号与配色' }]
const canSave = computed(() => [draft.value.title, draft.value.description, draft.value.audioPrompt].every(value => value.trim()))
const historyFields = computed(() => fields.filter(field => fieldTab(field.key) === activeTab.value))
function takeHistoryField(field: PublishingTextField) {
  if (field === 'audioPrompt' && !historyAudioCompatible.value) return
  if (historyVersion.value && !busy.value && !revisionLocked.value) draft.value[field] = historyVersion.value.content[field]
}
function fieldTab(key: string) { return key.startsWith('cover') ? 'cover' : key === 'audioPrompt' ? 'audio' : 'publishing' }
function saveShortcut(event: globalThis.KeyboardEvent) {
  if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 's') {
    event.preventDefault()
    if (ready.value && !busy.value && !revisionLocked.value && canSave.value) {
      if (dirty.value) void save()
      else flashSave()
    }
  }
}
watch([() => route.query.audioJob, () => route.query.writingVersion], () => { if (desktop) void load() })
watch([ready, () => automaticRun.value?.writingVersionId], async ([loaded, id]) => {
  if (!loaded || !id || versions.value.some(version => version.id === id)) return
  try {
    const saved = await api.listVersions(props.id, 'publishing', chapterId.value)
    if (automaticRun.value?.writingVersionId !== id) return
    const generated = saved.find(version => version.id === id)
    if (!generated) return
    versions.value = saved
    if (!selected.value && !dirty.value) accept(generated)
    else { historyVersionId.value = id; historyOpen.value = true }
    message.value = '本次创作方案已保存，自动流程将使用该版本制作音频。'
  } catch (cause) { error.value = errorMessage(cause) }
})
onMounted(() => globalThis.addEventListener('keydown', saveShortcut))
onMounted(() => {
  if (!desktop) return
  globalThis.addEventListener('focus', refreshAudioOnFocus)
  globalThis.document.addEventListener('visibilitychange', refreshAudioOnVisibility)
  startAudioTimers()
})
onBeforeUnmount(() => globalThis.removeEventListener('keydown', saveShortcut))
onBeforeUnmount(() => {
  stopAudioTimers()
  globalThis.removeEventListener('focus', refreshAudioOnFocus)
  globalThis.document.removeEventListener('visibilitychange', refreshAudioOnVisibility)
})
onMounted(async () => {
  if (!desktop) return
  await refreshAutomation()
  await load()
})
</script>
<template>
  <div class="page-return-row">
    <RouterLink
      class="back-link"
      to="/projects"
    >
      <AppIcon name="back" />返回我的书籍
    </RouterLink>
    <Button
      variant="outline"
      size="sm"
      class="h-[30px] px-3 font-mono"
      as-child
    >
      <RouterLink :to="{ path: `/projects/${id}`, query: chapterQuery(chapterId) }">
        <AppIcon name="pen" />{{ chapterId ? '修改章节资料' : '修改书籍资料' }}
      </RouterLink>
    </Button>
  </div>
  <header class="page-heading writing-heading">
    <div>
      <div class="flex flex-wrap items-center gap-3">
        <h1>{{ title || '书籍' }}</h1><span class="creation-chip">{{ taskId ? 'AI 正在生成' : 'AI 内容创作' }}</span>
      </div>
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
    请在桌面应用中生成内容。
  </p>
  <OperationFeedback
    :error="error"
    :message="message"
    @dismiss="message = ''"
  />
  <OperationFeedback :error="autosave.error.value" />
  <p
    v-if="automaticRun"
    class="notice"
    role="status"
  >
    自动制作：{{ stages[automaticRun.stage] || '准备中' }}。
    已保存的方案会直接用于音频；当前手工草稿保留，查看本次方案可使用全局任务入口。
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
          name="bolt"
          :class="{ spin: !!taskId }"
        /></span>
        <div><h2>让灵感，成为这一期内容</h2><p>甲子 · {{ chapterId ? '读懂一章' : '读懂一本书' }} · 按内容自然展开</p></div>
      </div>
      <div class="generation-actions">
        <template v-if="providers.length">
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
              class="writing-model w-64 max-sm:w-full min-w-0 py-2.5 data-[size=default]:h-[44px]"
              :title="providers.find(provider => provider.id === providerId)?.name"
            >
              <span
                class="status-dot"
                aria-hidden="true"
              /><SelectValue placeholder="请选择">
                <span class="font-bold text-info">{{ providers.find(provider => provider.id === providerId)?.name }}</span>
              </SelectValue><span class="model-kind-badge">文本</span>
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
          class="primary-gradient"
          :disabled="busy || !!automaticRun || !providerId || !!preview"
          type="button"
          @click="generate"
        >
          <AppIcon name="sparkle" />{{ taskId ? '生成中…' : versions.length ? '生成备选' : '一键生成' }}
        </Button>
        <Button
          class="px-4"
          variant="outline"
          :disabled="busy || !!automaticRun || !selected || !!preview || !providerId"
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
      {{ revising ? '正在调整选中内容，完成后先预览再采用。' : '正在生成备选，你可以继续编辑，当前内容保留。' }}可切换页面继续创作，请保持应用运行。已等待 {{ waitingSeconds }} 秒，单次最多等待 5 分钟。
    </p>
    <section
      v-if="revisionOpen"
      class="panel revision-panel mb-6 space-y-5"
      aria-labelledby="revision-heading"
    >
      <div>
        <h2
          id="revision-heading"
          class="studio-section-title"
        >
          <AppIcon name="sliders" />按项调整
        </h2>
        <p class="muted">
          只重新生成选中的内容。生成前会保存当前修改，候选结果经你采用后才成为新版本。
        </p>
      </div>
      <fieldset :disabled="busy || !!preview">
        <legend class="mb-3 text-sm font-medium">
          选择要调整的内容
        </legend>
        <div class="revision-options">
          <label
            v-for="field in revisionOptions"
            :key="field.key"
            :for="`revision-${field.key}`"
            class="revision-option"
            :class="{ 'is-selected': revisionFields.includes(field.key) }"
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
          class="comparison-section"
          aria-label="封面字号与配色调整对比"
        >
          <h3 class="text-sm font-semibold">
            封面字号与配色
          </h3>
          <div class="version-comparison">
            <div class="comparison-current space-y-3">
              <p>原推荐</p>
              <CoverDesignPreview
                v-if="preview.before.coverDesign"
                :design="preview.before.coverDesign"
              />
              <p v-else>
                此版本还没有排版推荐。
              </p>
            </div>
            <div class="comparison-candidate space-y-3">
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
          class="comparison-section"
          :aria-label="`${field.label}调整对比`"
        >
          <h3 class="text-sm font-semibold">
            {{ field.label }}
          </h3>
          <div class="version-comparison">
            <div class="comparison-current">
              <label :for="`revision-before-${field.key}`">原内容</label>
              <Textarea
                :id="`revision-before-${field.key}`"
                :model-value="preview.before[field.key]"
                :rows="Math.min(field.rows, 8)"
                readonly
              />
            </div>
            <div class="comparison-candidate">
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
      <div class="actions wrap revision-actions">
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
    <Collapsible
      v-model:open="historyOpen"
      class="writing-workspace"
      :class="{ 'has-candidate': candidateVisible }"
      :unmount-on-hide="false"
    >
      <form
        class="writing-editor"
        @submit.prevent="save"
      >
        <div class="writing-editor-heading">
          <div>
            <h2 class="studio-section-title">
              <AppIcon name="pen" />当前稿
            </h2>
            <p>{{ selected ? `基于 v${selected.versionNo} · ${versionSource(selected)}` : '从这一期想分享的内容开始' }}</p>
          </div>
          <CollapsibleTrigger as-child>
            <Button
              type="button"
              variant="ghost"
              :disabled="!versions.length || busy"
            >
              <AppIcon name="clock" />历史版本<span class="history-count">{{ versions.length }}</span>
            </Button>
          </CollapsibleTrigger>
        </div>
        <Tabs
          v-model="activeTab"
          class="gap-0"
          :unmount-on-hide="false"
        >
          <TabsList
            class="mb-2 w-full"
            aria-label="创作内容"
          >
            <TabsTrigger
              v-for="tab in tabs"
              :key="tab.value"
              :value="tab.value"
            >
              <AppIcon :name="tab.icon" />{{ tab.label }}
            </TabsTrigger>
          </TabsList>
          <TabsContent
            v-for="tab in tabs"
            :key="tab.value"
            :value="tab.value"
          >
            <p class="tab-intro sr-only">
              {{ tab.value === 'publishing' ? '准备标题、介绍和话题标签，让感兴趣的观众找到你的作品。' : tab.value === 'audio' ? '自动生成或手动导入音频，再到视频制作识别字幕、生成原声 MG 成片。' : '为同一本书准备两种构图，用文字引出故事的第一眼。' }}
            </p>
            <section
              v-if="tab.value === 'cover' && draft.coverDesign"
              class="panel editor-section"
              aria-label="AI 推荐排版"
            >
              <div class="result-heading">
                <h2 class="flex items-center gap-2 text-xs font-bold">
                  <AppIcon
                    name="palette"
                    class="text-warning"
                  />AI 推荐字号与配色
                </h2>
                <span class="text-[10px] font-mono text-info">建议 {{ Math.round(draft.coverDesign.overlayOpacity * 100) }}% 底色蒙层自动适配</span>
              </div>
              <CoverDesignPreview :design="draft.coverDesign">
                <template #hint>
                  <p class="muted !m-0">
                    建议底色蒙层 {{ Math.round(draft.coverDesign.overlayOpacity * 100) }}%，突出文字；长文字仍会按各比例自动缩小。制作新封面时自动带入；已有封面可在「排版样式」中应用。
                  </p>
                </template>
              </CoverDesignPreview>
            </section>
            <div
              class="editor-fields"
              :data-tab="tab.value"
              :class="{ 'cover-fields': tab.value === 'cover' }"
            >
              <section
                v-for="field in fields.filter(item => fieldTab(item.key) === tab.value)"
                :key="field.key"
                :data-field="field.key"
                class="panel editor-section"
              >
                <div class="result-heading">
                  <div class="flex items-center gap-2">
                    <span
                      v-if="field.key.startsWith('coverPrompt')"
                      class="ratio-tag"
                      :data-ratio="field.key"
                    >{{ field.key === 'coverPromptLandscape' ? '4:3 RATIO' : field.key === 'coverPromptPortrait' ? '3:4 RATIO' : '16:9 WIDE' }}</span><label :for="`result-${field.key}`">{{ field.label }}</label>
                  </div>
                  <CopyButton
                    compact
                    :label="`复制${field.label}`"
                    :copied="copiedField === field.key"
                    :disabled="!draft[field.key]"
                    @click="copy(field.key, field.label)"
                  />
                </div>
                <div class="editor-input-wrap">
                  <Textarea
                    :id="`result-${field.key}`"
                    v-model="draft[field.key]"
                    :class="['editor-input text-sm leading-[1.8]', field.rows === 2 ? 'block min-h-0 resize-none' : '', field.key === 'title' ? 'h-16 py-4 pr-24 text-xl leading-[30px] font-semibold text-foreground' : field.rows === 2 ? 'h-[42px] py-2.5 leading-5 font-medium' : '', field.key === 'coverTitle' || field.key === 'coverSubtitle' ? 'pr-20' : '', field.key === 'description' ? 'min-h-60 text-[15px] leading-[1.9]' : '', tab.value === 'publishing' ? 'bg-transparent' : '', field.key === 'tags' ? 'font-mono font-normal text-info' : '', field.key === 'audioPrompt' ? 'block h-56 bg-terminal p-4 leading-relaxed' : field.key.startsWith('coverPrompt') ? 'p-3.5' : '']"
                    :rows="field.rows === 2 ? 1 : field.key === 'description' ? 5 : field.key.startsWith('coverPrompt') ? 3 : field.rows"
                    :wrap="field.rows === 2 ? 'off' : undefined"
                    :maxlength="field.limit"
                    :disabled="(busy && !taskId) || revisionLocked"
                    :aria-describedby="field.help ? `result-help-${field.key}` : undefined"
                    :placeholder="field.key === 'tags' ? '#读书 #好书分享 #书籍主题' : `生成后在这里查看和修改${field.label}`"
                  />
                  <p
                    v-if="field.key !== 'audioPrompt'"
                    class="character-count"
                  >
                    {{ [...draft[field.key]].length }} / {{ field.limit }}
                  </p>
                </div>
                <p
                  v-if="field.help"
                  :id="`result-help-${field.key}`"
                  class="muted editor-field-help"
                >
                  {{ field.help }}
                </p>
              </section>
            </div>
            <section
              v-if="tab.value === 'audio'"
              class="panel editor-section"
              aria-labelledby="audio-automation-title"
            >
              <div class="result-heading">
                <h2 id="audio-automation-title">
                  自动生成音频
                </h2>
                <Badge variant="secondary">
                  Gemini Notebook
                </Badge>
              </div>
              <p class="muted">
                书引 AI 将在应用内打开网站，自动添加所选来源、填写指导、生成并下载音频。首次使用需完成 Google 登录，请保持应用运行。
              </p>
              <div
                v-if="audioBrowser && !audioBrowser.supported"
                class="error"
                role="alert"
              >
                <AppIcon name="info" />{{ audioBrowser.message || '当前平台暂不支持自动音频，请使用手动导入。' }}
              </div>
              <template v-else>
                <div class="audio-setup-grid">
                  <div class="audio-setup-field">
                    <label for="audio-source">本次原书来源</label>
                    <Select
                      v-model="audioSourceKey"
                      :disabled="busy || !!activeAudioJob"
                    >
                      <SelectTrigger
                        id="audio-source"
                        class="w-full bg-card *:data-[slot=select-value]:block *:data-[slot=select-value]:truncate"
                        :title="selectedDocument?.originalName"
                        aria-describedby="audio-source-hint"
                      >
                        <SelectValue placeholder="选择原书文件或资料文本" />
                      </SelectTrigger>
                      <SelectContent>
                        <SelectItem
                          v-for="document in chapterId ? [] : documents"
                          :key="document.id"
                          :value="`asset:${document.id}`"
                          :disabled="document.existsStatus !== 'available'"
                        >
                          {{ document.originalName }} · {{ document.existsStatus === 'available' ? '文件可用' : '文件已丢失' }}
                        </SelectItem>
                        <SelectItem
                          v-if="!chapterId"
                          value="text"
                          :disabled="!sourceNotes.trim()"
                        >
                          书籍资料文本（用户提供，不标为完整原著）
                        </SelectItem>
                        <SelectItem
                          v-if="chapterId"
                          value="chapter"
                        >
                          {{ chapter?.title }} · 完整章节正文
                        </SelectItem>
                      </SelectContent>
                    </Select>
                    <p
                      v-if="chapterId"
                      id="audio-source-hint"
                      class="muted"
                    >
                      将通过“复制的文字”上传本章。{{ selected?.sourceSnapshot?.content !== chapter?.content ? '所选版本使用此前保存的章节内容。' : '音频提示词与正文来自同一次章节创作。' }}
                    </p>
                    <p
                      v-else-if="selectedDocument"
                      id="audio-source-hint"
                      class="muted file-path"
                    >
                      {{ displayPath(selectedDocument.filePath) }}
                    </p>
                    <p
                      v-else-if="audioSourceKey === 'text' && sourceNotes.trim()"
                      id="audio-source-hint"
                      class="muted"
                    >
                      将使用「修改书籍资料」中保存的资料文本，不会标记为完整原著。
                    </p>
                    <p
                      v-else
                      id="audio-source-hint"
                      class="muted"
                    >
                      还没有可选来源。请添加原书文件，或先在书籍资料中填写资料文本。
                    </p>
                    <Button
                      v-if="!chapterId"
                      type="button"
                      variant="outline"
                      :disabled="busy || !!activeAudioJob"
                      @click="addSourceDocument"
                    >
                      <AppIcon name="plus" />添加原书文件
                    </Button>
                  </div>
                  <div class="audio-setup-field">
                    <label for="audio-format">Gemini Notebook 音频格式</label>
                    <Select
                      v-model="audioFormat"
                      :disabled="busy || !!activeAudioJob"
                    >
                      <SelectTrigger
                        id="audio-format"
                        class="w-full bg-card"
                        aria-describedby="audio-format-hint"
                      >
                        <SelectValue placeholder="选择格式" />
                      </SelectTrigger>
                      <SelectContent>
                        <SelectItem value="deep_dive">
                          深入探索
                        </SelectItem>
                        <SelectItem value="brief">
                          简报
                        </SelectItem>
                      </SelectContent>
                    </Select>
                    <p
                      id="audio-format-hint"
                      class="muted"
                    >
                      「深入探索」通常是双主持人；「简报」较短。网站实际档位和提示词只影响创作要求，不保证精确分钟或单人效果。
                    </p>
                  </div>
                </div>
                <div class="actions audio-start-actions">
                  <Button
                    id="start-audio-job"
                    type="button"
                    :disabled="!canStartAudio"
                    @click="startAudio"
                  >
                    <AppIcon name="headphones" />开始生成音频
                  </Button>
                  <span class="muted">启动前会保存当前提示词，再冻结来源与版本。</span>
                </div>
              </template>
              <div
                v-if="otherActiveAudioJob"
                class="error"
                role="alert"
              >
                当前已有《{{ otherActiveAudioJob.projectTitle }}》的音频任务。完成或停止该任务后才能开始本书任务。
              </div>
            </section>
            <section
              v-if="tab.value === 'audio'"
              class="panel editor-section audio-output"
            >
              <article
                v-if="shownAudioJob"
                class="audio-task"
                aria-labelledby="audio-job-title"
              >
                <div class="audio-task-meta">
                  <div>
                    <div class="result-heading">
                      <h3 id="audio-job-title">
                        自动任务 · {{ shownAudioJob.projectTitle }}
                      </h3>
                      <Badge variant="secondary">
                        {{ audioStatusLabel(shownAudioJob) }}
                      </Badge>
                    </div>
                    <p class="muted">
                      停留阶段：{{ audioStageLabel(shownAudioJob) }} · 任务创建于 {{ new Date(shownAudioJob.createdAt).toLocaleString() }}<span v-if="shownAudioJob.outputSize"> · 大小：{{ (shownAudioJob.outputSize / 1024 / 1024).toFixed(2) }} MB</span><span v-if="isLiveAudio(shownAudioJob)"> · 已等待 {{ audioElapsed(shownAudioJob) }} · 上次检查 {{ audioCheckedAt(shownAudioJob) }}</span>
                    </p>
                  </div>
                  <div class="flex items-start gap-2">
                    <Button
                      type="button"
                      variant="outline"
                      size="sm"
                      class="h-[30px] px-3 py-1.5"
                      :disabled="busy"
                      @click="controlAudio(shownAudioJob, 'open_browser')"
                    >
                      查看网站
                    </Button>
                    <Button
                      type="button"
                      variant="outline"
                      size="sm"
                      class="h-[30px] px-3 py-1.5"
                      :disabled="busy || !audioBrowser?.supported"
                      @click="controlAudio(shownAudioJob, 'refresh_browser')"
                    >
                      刷新网站
                    </Button>
                  </div>
                </div>
                <p
                  v-if="audioBrowser?.url && shownAudioJob.status !== 'succeeded'"
                  class="muted break-all"
                >
                  当前网站：{{ audioBrowser.url }}
                </p>
                <p
                  v-if="shownAudioJob.status === 'waiting_login'"
                  class="notice"
                >
                  请在应用内网站窗口完成 Google 登录。完成后无需在网站手动点击生成，书引 AI 会继续跟进。
                </p>
                <p
                  v-if="shownAudioJob.errorMessage"
                  class="error"
                  role="alert"
                >
                  {{ displayPath(shownAudioJob.errorMessage) }}
                </p>
                <p
                  v-if="shownAudioJob.status === 'waiting_user'"
                  class="notice"
                >
                  {{ needsDownloadRetry(shownAudioJob) ? '音频已生成，但下载或归档未完成。继续后会只重试当前任务的下载和归档。' : '远端状态需要人工核对。确认原笔记本后继续跟进，不会自动重复点击生成。' }}
                </p>
                <AudioPlayer
                  v-if="completedAudioId"
                  ref="completedPlayer"
                  :project-id="id"
                  :asset-id="completedAudioId"
                  :title="shownAudioJob.projectTitle"
                  :format="audioAssets.find(asset => asset.id === completedAudioId)?.originalName.split('.').pop()?.toUpperCase()"
                  subtitle="已归档音频 · 本地原声"
                  aria-label="已下载音频播放器"
                  :disabled="activeTab !== 'audio'"
                />
                <p
                  v-if="shownAudioJob.status === 'cancelled'"
                  class="notice"
                >
                  已停止本地跟进，可点击“继续跟进”恢复原任务。Gemini Notebook 中已经提交的生成可能仍在继续。
                </p>
                <p
                  v-if="shownAudioJob.status !== 'succeeded'"
                  class="muted"
                >
                  页面空白或报错时可刷新网站；仍无法加载可重新打开原笔记本（尚未创建时打开首页）。刷新会暂停正在执行的任务，核对页面后继续跟进。登录、验证码及地区限制请按网站提示处理。
                </p>
                <ConfirmAction
                  v-if="audioConfirmation && audioConfirmation.job.id === shownAudioJob.id"
                  :title="audioConfirmation.action === 'cancel' ? '确认停止跟进？' : '确认关联当前笔记本？'"
                  :confirm-label="audioConfirmation.action === 'cancel' ? '确认停止跟进' : '确认关联并继续'"
                  :destructive="false"
                  :busy="busy"
                  @confirm="confirmAudio"
                  @cancel="audioConfirmation = null"
                >
                  <p v-if="audioConfirmation.action === 'cancel'">
                    停止后可点击“继续跟进”恢复原任务。停止跟进不等于取消 Google 生成。
                  </p>
                  <template v-else>
                    <p>原任务未保存笔记本地址。请确认此网站是要用于《{{ audioConfirmation.job.projectTitle }}》的空白笔记本。</p>
                    <p class="muted break-all">
                      {{ audioConfirmation.url }}
                    </p>
                    <p>确认后继续命名、添加来源和生成。网站必须明确显示 0 个来源且没有音频生成结果。</p>
                  </template>
                </ConfirmAction>
                <div
                  v-else-if="shownAudioJob.status !== 'succeeded'"
                  class="actions wrap"
                >
                  <Button
                    v-if="canResumeAudio(shownAudioJob)"
                    type="button"
                    :disabled="busy"
                    :aria-busy="busy && operation === 'resumeAudio'"
                    @click="controlAudio(shownAudioJob, 'resume')"
                  >
                    {{ busy && operation === 'resumeAudio' ? '正在继续…' : needsDownloadRetry(shownAudioJob) ? '重试下载并归档' : '继续跟进' }}
                  </Button>
                  <Button
                    type="button"
                    variant="outline"
                    :disabled="busy || !audioBrowser?.supported"
                    @click="controlAudio(shownAudioJob, 'reopen_browser')"
                  >
                    重新打开
                  </Button>
                  <Button
                    v-if="isLiveAudio(shownAudioJob)"
                    type="button"
                    variant="outline"
                    :disabled="busy"
                    @click="controlAudio(shownAudioJob, 'pause')"
                  >
                    暂停跟进
                  </Button>
                  <Button
                    v-if="shownAudioJob.status === 'failed'"
                    type="button"
                    variant="outline"
                    :disabled="busy"
                    @click="controlAudio(shownAudioJob, 'cancel')"
                  >
                    停止跟进以允许新任务
                  </Button>
                  <Button
                    v-else-if="shownAudioJob.status !== 'cancelled'"
                    type="button"
                    variant="outline"
                    :disabled="busy"
                    @click="controlAudio(shownAudioJob, 'cancel')"
                  >
                    停止跟进
                  </Button>
                </div>
              </article>
              <section
                class="audio-assets"
                aria-labelledby="audio-assets-title"
              >
                <div class="result-heading">
                  <h2 id="audio-assets-title">
                    音频成品与视频制作
                  </h2>
                  <Badge variant="secondary">
                    {{ audioAssets.length }} 个音频
                  </Badge>
                </div>
                <p class="muted">
                  自动任务成功后，音频会出现在这里。也可手动导入外部音频，选择「制作 MG 视频」生成字幕与动画；外部剪辑工具导出的成片仍可在发布准备中关联。
                </p>
                <div class="actions wrap">
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
                    {{ displayPath(asset.filePath) }}
                  </p>
                  <p :class="asset.existsStatus === 'available' ? 'muted' : 'error'">
                    {{ asset.existsStatus === 'available' ? '文件可用' : '文件已丢失，请到发布准备页重新定位' }} · {{ (asset.fileSize / 1024 / 1024).toFixed(2) }} MB
                  </p>
                  <AudioPlayer
                    v-if="asset.existsStatus === 'available' && asset.id !== completedAudioId"
                    :project-id="id"
                    :asset-id="asset.id"
                    :title="title"
                    :format="asset.originalName.split('.').pop()?.toUpperCase()"
                    :subtitle="`${asset.originalName.split('.').pop()?.toUpperCase()} · ${(asset.fileSize / 1024 / 1024).toFixed(2)} MB · 本地原声`"
                    :aria-label="`音频成品播放器：${asset.originalName}`"
                    :disabled="activeTab !== 'audio'"
                  />
                  <div class="actions">
                    <Button
                      v-if="asset.id === completedAudioId"
                      type="button"
                      variant="outline"
                      :disabled="asset.existsStatus !== 'available'"
                      @click="playCompletedAudio"
                    >
                      <AppIcon name="headphones" />试听音频
                    </Button>
                    <Button
                      type="button"
                      variant="outline"
                      :disabled="busy || asset.existsStatus !== 'available'"
                      @click="router.push({ path: `/projects/${id}/video`, query: { ...chapterQuery(chapterId), audio: asset.id } })"
                    >
                      <AppIcon name="video" />制作 MG 视频
                    </Button>
                  </div>
                </article>
              </section>
            </section>
          </TabsContent>
        </Tabs>
        <div class="sticky-actions">
          <span
            class="save-status"
            :class="{ 'is-dirty': dirty, 'is-saved': savedFlash }"
            role="status"
            aria-live="polite"
          ><i class="status-dot" /><CollapsibleTrigger as-child><Button
            type="button"
            variant="ghost"
            class="save-history-trigger inline-block h-auto p-0 font-mono text-xs font-normal"
            aria-label="预览历史版本"
            :disabled="!versions.length || busy"
          ><template v-if="dirty">{{ autosave.error.value || autosave.status.value || '有未保存的修改' }}</template><template v-else-if="selected">已保存为版本 <strong class="text-primary">v{{ selected.versionNo }}</strong></template><template v-else>尚无保存版本</template><span class="sr-only">历史版本（{{ versions.length }}）</span></Button></CollapsibleTrigger><span class="shortcut-hint"> · Ctrl / ⌘ S 快速保存当前状态</span></span>
          <div class="actions wrap">
            <Button
              type="submit"
              size="sm"
              :disabled="busy || revisionLocked || !dirty || !canSave"
              variant="outline"
            >
              <AppIcon name="save" />保存修改
            </Button>
            <Button
              v-if="activeTab === 'audio'"
              type="button"
              size="sm"
              class="px-5"
              :disabled="busy || revisionLocked"
              @click="router.push({ path: `/projects/${id}/video`, query: chapterQuery(chapterId) })"
            >
              下一步：制作视频<AppIcon name="arrow" />
            </Button>
            <Button
              v-else
              type="button"
              size="sm"
              class="px-5"
              :disabled="busy || revisionLocked || !canSave || !draft.coverTitle.trim() || !draft.coverSubtitle.trim()"
              @click="makeCover"
            >
              {{ dirty ? '保存并制作封面' : '使用这些内容制作封面' }}<AppIcon name="arrow" />
            </Button>
          </div>
        </div>
      </form>
      <Transition name="candidate-panel">
        <aside
          v-if="candidateVisible"
          class="writing-candidate panel"
          :aria-label="generating ? '生成中的候选预览' : '已完成的生成候选'"
          :data-state="generating ? 'receiving' : 'complete'"
        >
          <div class="candidate-heading">
            <h2 class="studio-section-title">
              <AppIcon name="sparkle" />生成候选
            </h2>
            <span
              class="candidate-status"
              role="status"
            ><AppIcon :name="generating ? 'clock' : 'check'" />{{ candidateStatus }}</span>
          </div>
          <p class="candidate-help">
            {{ generating ? '以下内容尚未校验或保存，停止或失败后会清除。当前稿和历史版本保留。' : `已保存为 v${candidateVersion?.versionNo}，当前稿保留。对照后选择需要的内容。` }}
          </p>
          <div
            v-if="!Object.keys(candidateContent).length"
            class="candidate-waiting"
          >
            <AppIcon name="pen" />
            <p>{{ revising && revisionFields.every(field => field === 'coverDesign') ? '正在调整封面字号与配色…' : '等待正文返回…' }}</p>
            <span>返回的内容将在这里显示</span>
          </div>
          <div
            v-else
            class="candidate-content"
            tabindex="0"
            aria-label="候选内容，可滚动阅读"
          >
            <section
              v-for="field in fields.filter(item => candidateContent[item.key])"
              :key="field.key"
              :aria-label="`候选${field.label}`"
            >
              <h3>{{ field.label }}</h3>
              <p>{{ candidateContent[field.key] }}</p>
            </section>
          </div>
          <div class="candidate-footer">
            <span v-if="generating">已等待 {{ waitingSeconds }} 秒 · 最长 5 分钟</span>
            <Button
              v-else
              type="button"
              variant="outline"
              @click="viewCandidate"
            >
              <AppIcon name="eye" />查看对照并取用
            </Button>
          </div>
        </aside>
      </Transition>
      <section
        v-if="versions.length"
        v-show="historyOpen"
        class="history-panel"
        aria-label="历史版本对照"
      >
        <CollapsibleContent>
          <div class="history-heading">
            <div>
              <h2 class="studio-section-title">
                <AppIcon name="clock" />历史版本对照
              </h2><p>查看只做对照，取用后才会改变当前稿。</p>
            </div>
            <CollapsibleTrigger as-child>
              <Button
                type="button"
                variant="ghost"
                size="icon"
                aria-label="收起历史版本"
              >
                <AppIcon name="close" />
              </Button>
            </CollapsibleTrigger>
          </div>
          <label for="writing-version">预览历史版本</label>
          <Select
            v-model="historyVersionId"
            :disabled="busy || !!preview"
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
                v{{ version.versionNo }} · {{ versionSource(version) }} · {{ version.modelName || '手动保存' }} · {{ new Date(version.createdAt).toLocaleString() }}{{ selected?.id === version.id ? ' · 当前编辑来源' : '' }}
              </SelectItem>
            </SelectContent>
          </Select>
          <template v-if="historyVersion">
            <div class="history-version-meta">
              <span><Badge variant="secondary">{{ versionSource(historyVersion) }}</Badge> v{{ historyVersion.versionNo }} <span v-if="historyVersion.id === selected?.id">· 当前编辑来源</span></span>
              <time :datetime="historyVersion.createdAt">{{ new Date(historyVersion.createdAt).toLocaleString() }}</time>
            </div>
            <div class="actions history-actions">
              <Button
                variant="outline"
                :disabled="busy || revisionLocked"
                @click="switchVersion(historyVersion.id)"
              >
                用于当前编辑
              </Button>
              <span class="muted text-sm">预览不会改变当前稿；取用单项后保存为新版本。</span>
            </div>
            <p
              v-if="activeTab === 'audio' && !historyAudioCompatible"
              class="text-sm text-muted-foreground"
            >
              此版本的章节来源与当前稿不同，请使用“用于当前编辑”切换完整版本，保留对应正文和音频指导。
            </p>
            <div
              v-for="field in historyFields"
              :key="field.key"
              class="version-comparison"
            >
              <div class="comparison-current">
                <label :for="`history-current-${field.key}`">当前稿 · {{ field.label }}</label><Textarea
                  :id="`history-current-${field.key}`"
                  :model-value="draft[field.key]"
                  readonly
                  :rows="Math.min(field.rows, 5)"
                />
              </div>
              <div class="comparison-candidate">
                <label :for="`history-candidate-${field.key}`">v{{ historyVersion.versionNo }} · {{ field.label }}</label><Textarea
                  :id="`history-candidate-${field.key}`"
                  :model-value="historyVersion.content[field.key]"
                  readonly
                  :rows="Math.min(field.rows, 5)"
                /><Button
                  variant="ghost"
                  size="sm"
                  :disabled="busy || revisionLocked || draft[field.key] === historyVersion.content[field.key] || (field.key === 'audioPrompt' && !historyAudioCompatible)"
                  @click="takeHistoryField(field.key)"
                >
                  取用{{ field.label }}
                </Button>
              </div>
            </div>
          </template>
        </CollapsibleContent>
      </section>
    </Collapsible>
    <span
      class="sr-only"
      role="status"
      aria-live="polite"
    >{{ copiedField ? `已复制${copyLabel}` : '' }}</span>
  </template>
</template>
