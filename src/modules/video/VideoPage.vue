<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { onBeforeRouteLeave, onBeforeRouteUpdate, RouterLink, useRoute } from 'vue-router'
import AppIcon from '../../components/AppIcon.vue'
import AudioPlayer from '../../components/AudioPlayer.vue'
import ProjectNavigation from '../../components/ProjectNavigation.vue'
import OperationFeedback from '../../components/OperationFeedback.vue'
import VideoJobProgress from './VideoJobProgress.vue'
import VideoSceneEditor from './VideoSceneEditor.vue'
import VideoSubtitleEditor from './VideoSubtitleEditor.vue'
import { revealInScroller } from '../../lib/scroll'
import VideoEnvironmentCheck from './VideoEnvironmentCheck.vue'
import VideoSceneRevision from './VideoSceneRevision.vue'
import { initialPosition, type VideoEditorDraft } from './subtitles'
import { inspectStoryboard } from './storyboard'
import { Button } from '@/components/ui/button'
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '@/components/ui/collapsible'
import { Input } from '@/components/ui/input'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { useOperation } from '../../composables/useOperation'
import { useEditorDraft } from '../../composables/useEditorDraft'
import { readDraft, saveDraft } from '../../services/drafts'
import { getProject, isDesktop, listAssets, mediaSource, openAsset, openProjectDirectory } from '../../services/projects'
import { listProviders } from '../../services/ai'
import * as api from '../../services/video'
import type { Asset } from '../../types/project'
import type { ModelProvider } from '../../types/ai'
import { visualCatalog, videoStatuses, type VideoCue, type VideoJob, type VideoScene, type VideoSceneRevisionInput, type VideoSettings } from '../../types/video'

const props = defineProps<{ id: string }>()
const route = useRoute()
const desktop = isDesktop()
const { busy: operationBusy, error, message, run } = useOperation()
const imageWorking = ref(false)
const environmentWorking = ref(false)
const sceneRevision = ref<VideoSceneRevisionInput>()
const busy = computed(() => operationBusy.value || imageWorking.value || environmentWorking.value || !!sceneRevision.value)
const title = ref('')
const loaded = ref(false)
const assets = ref<Asset[]>([])
const providers = ref<ModelProvider[]>([])
const imageProviders = ref<ModelProvider[]>([])
const jobs = ref<VideoJob[]>([])
const selectedId = ref('')
const audioId = ref('')
const providerId = ref('')
const subtitleId = ref('auto')
const config = ref<VideoSettings>({ engineRoot: '', nodePath: '', pythonPath: '', modelPath: '' })
const savedConfig = ref('')
const subtitleText = ref('')
const scenes = ref<VideoScene[]>([])
const savedEditor = ref('')
const editorJobId = ref('')
const draftReady = ref(false)
const position = ref(initialPosition())
const originalCues = ref<VideoCue[]>([])
const originalError = ref('')
const subtitleEditor = ref<InstanceType<typeof VideoSubtitleEditor>>()
const playbackCue = ref(-1)
const playbackScene = computed(() => scenes.value.findIndex(scene => playbackCue.value >= scene.first && playbackCue.value <= scene.last))
async function onPlayback(index: number, follow: boolean) {
  playbackCue.value = index
  if (!follow || playbackScene.value < 0) return
  await nextTick()
  const row = globalThis.document.getElementById(`video-scene-${playbackScene.value}`)
  revealInScroller(row)
}
const videoUrl = ref('')
const previewUrl = ref('')
const previewFrames = ref<{ sceneIndex: number; src: string }[]>([])
const previewFirstScene = ref('0')
const previewSeconds = ref('30')
const mediaError = ref('')
const progressError = ref('')
const jobPanel = ref<globalThis.HTMLElement>()
const selected = computed(() => jobs.value.find(job => job.id === selectedId.value))
const running = computed(() => jobs.value.some(job => job.status === 'running'))
const audios = computed(() => assets.value.filter(a => a.assetType === 'audio' && a.existsStatus === 'available'))
const subtitles = computed(() => assets.value.filter(a => a.assetType === 'subtitle' && /\.srt$/i.test(a.originalName) && a.existsStatus === 'available'))
const editable = computed(() => selected.value && editorJobId.value === selected.value.id && !['running', 'succeeded'].includes(selected.value.status))
const legacyPlan = computed(() => scenes.value.some(scene => !scene.beats?.length && !scene.image))
const completePlan = computed(() => scenes.value.length > 0 && !legacyPlan.value && scenes.value.at(-1)!.last + 1 === selected.value?.plan.cues.length)
const storyboardNotes = computed(() => selected.value ? inspectStoryboard({ ...selected.value.plan, scenes: scenes.value }) : [])
const sceneSnapshot = computed(() => JSON.stringify(scenes.value))
const editorValue = () => JSON.stringify([subtitleText.value, sceneSnapshot.value])
const jobLabel = (job: VideoJob) => `${videoStatuses[job.status]} · ${job.createdAt ? new Date(job.createdAt).toLocaleString() : job.id}`
const dirty = computed(() => loaded.value && !!editable.value && savedEditor.value !== editorValue())
const draftContent = computed<VideoEditorDraft>(() => ({ subtitles: subtitleText.value, scenes: sceneSnapshot.value, position: position.value }))
const autosave = useEditorDraft(() => selectedId.value, 'video', draftContent, () => String(selected.value?.runId), dirty, draftReady, canPrepareLeave, true)
const configDirty = computed(() => loaded.value && savedConfig.value !== JSON.stringify(config.value))
const previewKey = computed(() => selected.value?.preview && !dirty.value ? `${props.id}:${selected.value.id}:${selected.value.runId}:${selected.value.preview.signature}` : '')
const runtimeFields: { key: keyof VideoSettings; label: string; directory: boolean; hint: string }[] = [
  { key: 'engineRoot', label: 'Frametell 引擎目录', directory: true, hint: '选择 whiteboard-teacher-video 内的 engine，当前适配 0.7.3。' },
  { key: 'nodePath', label: 'Node 程序', directory: false, hint: '选择 Node.js 24.13.1 或更新的 24.x LTS（推荐 24.22.2）的 node.exe，避免旧版中文路径崩溃。' },
  { key: 'pythonPath', label: '字幕识别程序', directory: false, hint: '选择已安装 faster-whisper 1.2.1 的环境内 python.exe。使用已有 SRT 时不需要。' },
  { key: 'modelPath', label: '本地语音模型目录', directory: true, hint: '选择包含 model.bin 的多语言 small 模型。不会隐式下载模型。' },
]
let disposed = false
let polling = false
let timer: ReturnType<typeof globalThis.setInterval> | undefined

function canPrepareLeave() {
  if (configDirty.value || busy.value) { error.value = '请先保存运行环境或等待当前操作完成，再离开视频制作。'; return false }
  return true
}
async function canLeave() {
  if (!canPrepareLeave()) return false
  try { await autosave.flush(); return true } catch { return false }
}
onBeforeRouteLeave(canLeave)
onBeforeRouteUpdate((to, from) => (to.params.id === from.params.id && to.query.job === from.query.job) || canLeave())

async function adopt(job: VideoJob | undefined, restoreDraft = true) {
  draftReady.value = false
  editorJobId.value = ''
  autosave.reset()
  position.value = initialPosition(); originalCues.value = []; originalError.value = ''
  if (!job) { subtitleText.value = ''; scenes.value = []; savedEditor.value = editorValue(); return }
  const projectId = props.id
  const [text, draft] = await Promise.all([api.subtitles(job.id), restoreDraft && !['running', 'succeeded'].includes(job.status) ? readDraft<VideoEditorDraft>(job.id, 'video') : null])
  if (disposed || selectedId.value !== job.id || projectId !== props.id) return
  subtitleText.value = text
  scenes.value = JSON.parse(JSON.stringify(job.plan.scenes))
  savedEditor.value = editorValue()
  if (draft && draft.basedOn === String(job.runId)) {
    subtitleText.value = draft.content.subtitles
    scenes.value = JSON.parse(draft.content.scenes)
    position.value = draft.content.position
    autosave.status.value = '已恢复上次暂存的编辑位置和内容'
  }
  editorJobId.value = job.id
  if (job.plan.cues.length) {
    try {
      const cues = await api.originalCues(job.id)
      if (!disposed && selectedId.value === job.id && projectId === props.id) originalCues.value = cues
    } catch (cause) {
      if (!disposed && selectedId.value === job.id && projectId === props.id) originalError.value = String(cause)
    }
  }
  await nextTick()
  if (disposed || selectedId.value !== job.id || projectId !== props.id) return
  draftReady.value = !!editable.value
  if (draft) {
    if (position.value.section === 'scene') focusScene(position.value.index)
    else subtitleEditor.value?.restorePosition()
  }
  return !!draft
}
function put(job: VideoJob) { jobs.value = [job, ...jobs.value.filter(j => j.id !== job.id)]; selectedId.value = job.id }
const load = () => run(async () => {
  loaded.value = false; draftReady.value = false
  const projectId = props.id
  const [book, files, models, settings, history] = await Promise.all([getProject(props.id), listAssets(props.id), listProviders(), api.settings(), api.listJobs(props.id)])
  if (disposed || projectId !== props.id) return
  title.value = book.title; assets.value = files; providers.value = models.filter(p => p.modelKind === 'text' && p.isEnabled)
  imageProviders.value = models.filter(p => p.modelKind === 'image' && p.isEnabled)
  config.value = settings; savedConfig.value = JSON.stringify(settings)
  jobs.value = history
  const recentDraft = history.filter(j => j.draftUpdatedAt).sort((a, b) => b.draftUpdatedAt!.localeCompare(a.draftUpdatedAt!))[0]
  selectedId.value = history.find(j => j.id === route.query.job)?.id ?? recentDraft?.id ?? history[0]?.id ?? ''
  audioId.value = audios.value.find(a => a.id === route.query.audio)?.id ?? audios.value[0]?.id ?? ''
  providerId.value = providers.value.find(p => p.isDefault)?.id ?? providers.value[0]?.id ?? ''
  const restored = await adopt(selected.value)
  loaded.value = true
  await nextTick()
  if (restored) {
    if (position.value.section === 'scene') focusScene(position.value.index)
    else subtitleEditor.value?.restorePosition()
  }
})
async function poll() {
  if (!loaded.value || polling || busy.value || disposed) return
  const projectId = props.id
  polling = true
  try {
    const previous = selected.value
    const history = await api.listJobs(projectId)
    if (disposed || projectId !== props.id) return
    progressError.value = ''
    jobs.value = history
    if (previous && (previous.status !== selected.value?.status || previous.runId !== selected.value?.runId)) {
      if (selected.value?.status === 'running') {
        draftReady.value = false
        autosave.reset()
        return
      }
      await adopt(selected.value)
      const files = await listAssets(projectId)
      if (disposed || projectId !== props.id) return
      assets.value = files
      if (selected.value?.status === 'succeeded') message.value = '原声 MG 视频已保存，可直接播放或用于发布。'
      else if (selected.value?.preview) message.value = '试片已就绪，未登记为正式成片。请播放检查，再确认完整视频。'
    }
  } catch (cause) { if (!disposed && projectId === props.id) progressError.value = String(cause) }
  finally { polling = false }
}
const choosePath = (key: keyof VideoSettings, directory: boolean) => run(async () => {
  const path = await api.chooseRuntimePath(directory)
  if (typeof path === 'string') config.value[key] = path
})
const saveConfig = () => run(async () => { await api.saveSettings(config.value); savedConfig.value = JSON.stringify(config.value); message.value = '视频运行环境已保存，可点击“一键检查环境”验证。' })
const begin = (sourceAudio: string, sourceSubtitle: string | null) => run(async () => {
  if (dirty.value || configDirty.value) throw new Error('请先保存或还原字幕、分镜和运行环境修改。')
  const job = await api.startJob(props.id, sourceAudio, providerId.value, sourceSubtitle)
  put(job); await adopt(job)
  await nextTick()
  jobPanel.value?.scrollIntoView({ block: 'start' })
})
const create = () => begin(audioId.value, subtitleId.value === 'auto' ? null : subtitleId.value)
const remake = () => { const job = selected.value; if (job?.audioAssetId && job.subtitleAssetId) return begin(job.audioAssetId, job.subtitleAssetId) }
const resume = (render: boolean) => run(async () => {
  if (!selected.value) return
  if (!render && !providerId.value) throw new Error('请选择已启用的文本模型生成分镜。')
  if (configDirty.value) throw new Error('请先保存运行环境配置。')
  if (render) { await persistPlan(scenes.value); selected.value.preview = null }
  else if (dirty.value) throw new Error('字幕已修改，请使用“保存字幕并重新生成分镜”。')
  await startSavedWork(() => api.continueJob(selected.value!.id, render, render ? null : providerId.value))
  await nextTick()
  jobPanel.value?.scrollIntoView({ block: 'start' })
})
const preview = () => run(async () => {
  if (!selected.value) return
  if (configDirty.value) throw new Error('请先保存运行环境配置。')
  await persistPlan(scenes.value)
  selected.value.preview = null
  await startSavedWork(() => api.previewJob(selected.value!.id, Number(previewFirstScene.value), Number(previewSeconds.value)))
  await nextTick()
  jobPanel.value?.scrollIntoView({ block: 'start' })
})
async function refreshImages() { assets.value = await listAssets(props.id) }
const reviseScene = (sceneIndex: number) => run(async () => {
  if (!selected.value || !editable.value || !completePlan.value || running.value || !providerId.value) return
  await autosave.flush()
  position.value = { ...position.value, section: 'scene', index: sceneIndex }
  sceneRevision.value = { jobId: selected.value.id, runId: selected.value.runId, providerId: providerId.value, sceneIndex, subtitles: subtitleText.value, scenes: JSON.parse(sceneSnapshot.value), instruction: '' }
})
function adoptScene(candidate: VideoScene) {
  const snapshot = sceneRevision.value
  if (!snapshot) return
  if (!editable.value || selected.value?.id !== snapshot.jobId || selected.value.runId !== snapshot.runId || subtitleText.value !== snapshot.subtitles || sceneSnapshot.value !== JSON.stringify(snapshot.scenes)) {
    error.value = '编辑内容或任务已变化，候选已过期，请重新生成这一镜。'
    sceneRevision.value = undefined
    return
  }
  scenes.value[snapshot.sceneIndex] = candidate
  sceneRevision.value = undefined
  message.value = `已采用第${snapshot.sceneIndex + 1}镜，其余分镜和字幕保留。请保存校对并查看试片。`
}
function focusScene(index: number) {
  position.value = { ...position.value, section: 'scene', index }
  const card = jobPanel.value?.querySelector<globalThis.HTMLElement>(`#video-scene-${index}`)
  card?.scrollIntoView?.({ block: 'nearest' })
  card?.querySelector('input')?.focus({ preventScroll: true })
}
function goToSection(section: 'subtitles' | 'storyboard' | 'preview') {
  const target = jobPanel.value?.querySelector<globalThis.HTMLElement>(`#video-workbench-${section}`)
  target?.scrollIntoView({ block: 'start' })
  target?.focus({ preventScroll: true })
}
const regenerate = () => run(async () => {
  if (!selected.value) return
  if (!providerId.value) throw new Error('请选择已启用的文本模型生成分镜。')
  if (configDirty.value) throw new Error('请先保存运行环境配置。')
  await persistPlan([])
  selected.value.preview = null
  await startSavedWork(() => api.continueJob(selected.value!.id, false, providerId.value))
})
const stop = () => run(async () => { if (selected.value) { await api.cancelJob(selected.value.id); message.value = '正在停止本任务的识别或渲染进程，原音频不受影响。' } })
const switchJob = (id: unknown) => run(async () => { await autosave.flush(); selectedId.value = String(id); await adopt(selected.value) })
const resetEditor = () => run(async () => {
  await autosave.flush()
  draftReady.value = false
  try { await saveDraft(selectedId.value, 'video', null); await adopt(selected.value, false) }
  finally { draftReady.value = !!editable.value }
})
async function persistPlan(nextScenes: VideoScene[]) {
  await autosave.flush()
  draftReady.value = false
  try {
    await api.savePlan(selectedId.value, subtitleText.value, nextScenes)
    scenes.value = nextScenes
    savedEditor.value = editorValue()
  } finally { await nextTick(); draftReady.value = !!editable.value }
}
async function startSavedWork(start: () => Promise<VideoJob>) {
  await autosave.flush()
  draftReady.value = false
  try { put(await start()) }
  finally { await nextTick(); draftReady.value = !!editable.value }
}
const saveEditor = () => run(async () => {
  if (!selected.value) return
  await persistPlan(scenes.value)
  selected.value.preview = null
  jobs.value = await api.listJobs(props.id)
  await autosave.flush()
  message.value = '字幕和分镜校对已保存，尚未启动渲染。'
})
const exportSrt = () => run(async () => {
  if (!selected.value || dirty.value) throw new Error('请先保存字幕校对。')
  const id = await api.exportSubtitles(selected.value.id)
  jobs.value = await api.listJobs(props.id)
  assets.value = await listAssets(props.id)
  await openAsset(id, true)
  message.value = '已将当前保存的字幕导出为独立 SRT。'
})
const showAsset = (id: string, directory = false) => run(() => openAsset(id, directory))
const showDirectory = () => run(() => openProjectDirectory(props.id))

watch(() => selected.value?.videoAssetId, async id => {
  videoUrl.value = ''; mediaError.value = ''
  if (!id || !desktop) return
  try { const url = await mediaSource(props.id, id); if (!disposed && id === selected.value?.videoAssetId) videoUrl.value = url }
  catch (cause) { mediaError.value = `视频无法预览：${String(cause)}` }
})
watch(selectedId, () => { previewFirstScene.value = '0' })
watch(previewKey, async (key, _previous, onCleanup) => {
  previewUrl.value = ''; previewFrames.value = []
  const job = selected.value
  if (!key || !job?.preview || !desktop) return
  let current = true
  onCleanup(() => { current = false })
  try {
    const [url, ...frames] = await Promise.all([
      api.previewSource(props.id, job.id, job.runId),
      ...job.preview.scenes.map(async sceneIndex => ({ sceneIndex, src: await api.previewSource(props.id, job.id, job.runId, sceneIndex) })),
    ])
    if (current && !disposed) { previewUrl.value = url; previewFrames.value = frames }
  } catch (cause) { if (current && !disposed) mediaError.value = `试片无法读取，请重新生成：${String(cause)}` }
})
watch(() => [props.id, route.query.job], () => { if (desktop) void load() })
onMounted(() => { if (desktop) { void load(); timer = globalThis.setInterval(() => { void poll() }, 1500) } })
onBeforeUnmount(() => { disposed = true; if (timer) globalThis.clearInterval(timer) })
</script>

<template>
  <div class="video-page">
    <div class="video-backbar">
      <RouterLink
        class="back-link"
        :to="`/projects/${id}/covers`"
      >
        <AppIcon name="back" />返回封面工坊<span v-if="title"> · {{ title }}</span>
      </RouterLink>
      <Button
        variant="outline"
        :disabled="busy || !loaded"
        @click="showDirectory"
      >
        <AppIcon name="folder" />打开书籍目录
      </Button>
    </div>
    <header class="video-heading">
      <div class="video-title-row">
        <h1>视频制作</h1>
        <span
          v-if="loaded"
          class="video-environment-badge"
        >{{ config.engineRoot && config.nodePath ? 'Frametell · 已配置路径' : '视频环境待配置' }}</span>
      </div>
      <p class="muted">
        听原声、校字幕、调分镜，让这本书成为一支完整的视频。
      </p>
    </header>
    <ProjectNavigation
      :id="id"
      :editing="dirty"
      :busy="busy"
    />
    <OperationFeedback
      :error="error || autosave.error.value"
      :message="message"
      @dismiss="message = ''"
    />
    <OperationFeedback :error="mediaError" />
    <VideoSceneRevision
      v-if="sceneRevision"
      :input="sceneRevision"
      :providers="providers"
      @adopt="adoptScene"
      @close="sceneRevision = undefined"
    />
    <p
      v-if="!desktop"
      class="notice"
    >
      请在 Windows 桌面应用中制作原声 MG 视频。
    </p>
    <Button
      v-else-if="!loaded"
      :disabled="busy"
      @click="load"
    >
      {{ busy ? '正在读取…' : '重新读取' }}
    </Button>
    <div
      v-else
      class="video-workspace"
    >
      <Collapsible
        v-slot="{ open: setupOpen }"
        class="panel video-setup"
        :default-open="!jobs.length"
        :disabled="busy"
        :unmount-on-hide="false"
      >
        <CollapsibleTrigger class="disclosure-trigger video-setup-trigger">
          <AppIcon name="headphones" />
          <span>
            <span class="video-section-title">制作设置</span>
            <span class="muted">{{ jobs.length ? '音频、模型与运行环境；从原声制作另一版视频。' : '选择原声与模型，开始制作第一支视频。' }}</span>
          </span>
          <AppIcon name="chevron" />
        </CollapsibleTrigger>
        <CollapsibleContent class="video-inputs">
          <p class="muted">
            横屏 1920 × 1080 · 24 fps · 同款书卷手绘。按原声音频的完整时间轴制作，具体物体逐笔绘制；不重新配音、不加配乐。
          </p>
          <div class="video-form-grid">
            <div class="video-field">
              <label for="video-audio">制作音频</label><Select
                v-model="audioId"
                :disabled="running || busy"
              >
                <SelectTrigger
                  id="video-audio"
                  class="w-full min-w-0"
                >
                  <SelectValue placeholder="选择本书音频" />
                </SelectTrigger><SelectContent>
                  <SelectItem
                    v-for="asset in audios"
                    :key="asset.id"
                    :value="asset.id"
                  >
                    {{ asset.originalName }}
                  </SelectItem>
                </SelectContent>
              </Select>
            </div>
            <div class="video-field">
              <label for="video-provider">字幕校对与分镜模型</label><Select
                v-model="providerId"
                :disabled="running || busy"
              >
                <SelectTrigger
                  id="video-provider"
                  class="w-full min-w-0"
                >
                  <SelectValue placeholder="选择已启用的文本模型" />
                </SelectTrigger><SelectContent>
                  <SelectItem
                    v-for="provider in providers"
                    :key="provider.id"
                    :value="provider.id"
                  >
                    {{ provider.name }}
                  </SelectItem>
                </SelectContent>
              </Select>
            </div>
            <div class="video-field video-subtitle-source">
              <label for="video-subtitle-source">字幕来源</label><Select
                v-model="subtitleId"
                :disabled="running || busy"
              >
                <SelectTrigger
                  id="video-subtitle-source"
                  class="w-full min-w-0"
                >
                  <SelectValue />
                </SelectTrigger><SelectContent>
                  <SelectItem value="auto">
                    从原声自动识别
                  </SelectItem><SelectItem
                    v-for="asset in subtitles"
                    :key="asset.id"
                    :value="asset.id"
                  >
                    {{ asset.originalName }}
                  </SelectItem>
                </SelectContent>
              </Select><p class="muted">
                已有 SRT 可在发布准备的素材管理中关联；使用已有字幕不需要语音模型。
              </p>
            </div>
          </div>
          <AudioPlayer
            v-if="audioId && setupOpen"
            :project-id="id"
            :asset-id="audioId"
            :title="title"
            :format="audios.find(asset => asset.id === audioId)?.originalName.split('.').pop()?.toUpperCase()"
            subtitle="制作原声 · 本地音频"
            aria-label="所选原声音频"
          />
          <p
            v-if="!audios.length"
            class="notice"
          >
            尚无可用音频。请先在音频制作中生成或导入成品。
          </p>
          <p
            v-if="!providers.length"
            class="notice"
          >
            请先在模型设置中配置可用的文本模型，用于字幕校对和生成分镜。
          </p>
          <Button
            class="video-primary-action"
            :disabled="running || busy || !audioId || !providerId || !config.engineRoot || !config.nodePath"
            @click="create"
          >
            <AppIcon name="bolt" />生成字幕与分镜
          </Button>
          <Collapsible
            class="panel video-runtime"
            :default-open="!config.engineRoot || !config.nodePath"
            :unmount-on-hide="false"
          >
            <CollapsibleTrigger class="disclosure-trigger video-runtime-trigger">
              <span>
                <span class="video-section-title">视频运行环境 · 首次使用时配置</span>
                <span class="muted">独立安装的引擎还需 Chrome/Edge、FFmpeg 和字体。转写在本地 CPU 运行，模型建议放 D 盘。</span>
              </span>
              <AppIcon name="chevron" />
            </CollapsibleTrigger>
            <CollapsibleContent class="video-runtime-content">
              <p class="muted">
                独立安装的引擎还需要 Chrome/Edge、FFmpeg 和字体。转写在本地 CPU 运行，模型建议放 D 盘；不自动下载。字幕生成后自动分批校对，再生成分镜，会增加文本模型用量与等待时间。仅发送字幕、书名、作者及已保存音频提示词的首尾参考，不发送音频。
              </p>
              <div class="video-form-grid">
                <div
                  v-for="field in runtimeFields"
                  :key="field.key"
                  class="video-field"
                >
                  <label :for="`video-${field.key}`">{{ field.label }}</label><div class="video-path">
                    <Input
                      :id="`video-${field.key}`"
                      v-model="config[field.key]"
                      :disabled="busy || running"
                      :aria-describedby="`hint-${field.key}`"
                    /><Button
                      variant="outline"
                      :disabled="busy || running"
                      :aria-label="`选择${field.label}`"
                      @click="choosePath(field.key, field.directory)"
                    >
                      选择
                    </Button>
                  </div><p
                    :id="`hint-${field.key}`"
                    class="muted"
                  >
                    {{ field.hint }}
                  </p>
                </div>
              </div>
              <div class="actions">
                <Button
                  :disabled="busy || running"
                  @click="saveConfig"
                >
                  保存运行环境
                </Button><Button
                  variant="ghost"
                  :disabled="busy || !configDirty"
                  @click="config = JSON.parse(savedConfig)"
                >
                  还原配置
                </Button>
              </div>
              <VideoEnvironmentCheck
                :settings="config"
                :has-subtitles="subtitleId !== 'auto' || !!selected?.plan.cues.length"
                :disabled="busy || running"
                @working="environmentWorking = $event"
                @apply="config = $event"
                @choose="choosePath($event, $event === 'engineRoot' || $event === 'modelPath')"
              />
              <p class="muted">
                Frametell 为 AGPL-3.0 引擎，当前不随书引捆绑分发；首次准备步骤见 MG 视频使用文档。
              </p>
            </CollapsibleContent>
          </Collapsible>
        </CollapsibleContent>
      </Collapsible>

      <div
        v-if="jobs.length"
        ref="jobPanel"
        class="video-task-workspace"
      >
        <section class="panel video-job">
          <div class="video-job-heading">
            <div>
              <h2 class="video-section-title">
                当前制作任务
              </h2>
              <p class="muted">
                进度与成果随任务更新，可切换查看历史制作。
              </p>
            </div><Select
              :model-value="selectedId"
              :disabled="busy"
              @update:model-value="switchJob"
            >
              <SelectTrigger aria-label="选择视频历史任务">
                <SelectValue>{{ selected ? jobLabel(selected) : '选择视频任务' }}</SelectValue>
              </SelectTrigger><SelectContent>
                <SelectItem
                  v-for="job in jobs"
                  :key="job.id"
                  :value="job.id"
                >
                  {{ jobLabel(job) }}
                </SelectItem>
              </SelectContent>
            </Select>
          </div>
          <VideoJobProgress
            v-if="selected"
            :job="selected"
            :sync-error="selected.status === 'running' ? progressError : ''"
          >
            <template #actions>
              <div class="actions video-job-actions">
                <Button
                  v-if="selected.plan.cues.length"
                  variant="outline"
                  :disabled="busy || !!dirty"
                  @click="exportSrt"
                >
                  导出已保存 SRT
                </Button>
                <Button
                  v-if="selected.status === 'running'"
                  variant="outline"
                  class="video-stop-action"
                  :disabled="busy"
                  @click="stop"
                >
                  停止当前任务
                </Button>
                <Button
                  v-if="['draft', 'failed', 'cancelled'].includes(selected.status)"
                  :disabled="busy || running || legacyPlan || !providerId"
                  @click="resume(false)"
                >
                  继续准备字幕与分镜
                </Button>
                <Button
                  v-if="selected.status === 'succeeded' && selected.audioAssetId && selected.subtitleAssetId"
                  variant="outline"
                  :disabled="busy || running || !providerId"
                  @click="remake"
                >
                  用原字幕制作新版手绘
                </Button>
                <Button
                  v-if="selected.subtitleAssetId"
                  variant="outline"
                  :disabled="busy"
                  @click="showAsset(selected.subtitleAssetId, true)"
                >
                  <AppIcon name="folder" />打开 SRT 所在目录
                </Button>
                <Button
                  v-if="selected.videoAssetId"
                  variant="outline"
                  :disabled="busy"
                  @click="showAsset(selected.videoAssetId)"
                >
                  用系统播放器打开
                </Button>
                <Button
                  v-if="selected.status === 'succeeded' && selected.videoAssetId"
                  class="video-publish-action"
                  as-child
                >
                  <RouterLink :to="{ path: `/projects/${id}/publishing`, query: { task: 'new', videoJob: selected.id } }">
                    用于发布<AppIcon name="arrow" />
                  </RouterLink>
                </Button>
              </div>
            </template>
            <video
              v-if="videoUrl"
              :src="videoUrl"
              controls
              preload="metadata"
              aria-label="原声 MG 视频成片"
              @error="mediaError = '成片预览失败，请刷新任务或使用系统播放器。'"
            />
          </VideoJobProgress>
        </section>
        <template v-if="selected">
          <nav
            v-if="editable && selected.plan.cues.length"
            class="video-section-nav"
            aria-label="视频编辑分区"
          >
            <span>继续校对</span>
            <Button
              variant="ghost"
              @click="goToSection('subtitles')"
            >
              <AppIcon name="headphones" />字幕校对
            </Button>
            <Button
              variant="ghost"
              @click="goToSection('storyboard')"
            >
              <AppIcon name="pen" />分镜精修
            </Button>
            <Button
              v-if="completePlan"
              variant="ghost"
              @click="goToSection('preview')"
            >
              <AppIcon name="video" />试片与成片
            </Button>
          </nav>
          <Button
            v-if="editorJobId !== selected.id && !['running', 'succeeded'].includes(selected.status)"
            variant="outline"
            :disabled="busy"
            @click="run(async () => { await adopt(selected) })"
          >
            重新读取字幕与草稿
          </Button>
          <template v-if="editable && selected.plan.cues.length">
            <section
              id="video-workbench-subtitles"
              class="panel video-proofread"
              tabindex="-1"
              aria-labelledby="video-subtitles-heading"
            >
              <div class="video-editor-heading">
                <h2
                  id="video-subtitles-heading"
                  class="video-section-title"
                >
                  字幕校对
                </h2><p class="muted">
                  以实际原声为准，AI 校对后仍需试听人名和外语；每条字幕最多32字。“保存字幕并重新生成分镜”会先重新校对全文。原始 SRT 保留在本书素材中。
                </p>
              </div>
              <p
                v-if="legacyPlan"
                class="notice"
              >
                当前是旧版文字图解分镜，不会按旧模板继续渲染。请保留字幕并升级手绘分镜，原音频和历史成片不变。
              </p>
              <div class="actions video-draft-status">
                <p
                  role="status"
                  aria-live="polite"
                  class="muted"
                >
                  {{ autosave.error.value || autosave.status.value || '编辑会自动暂存；正式保存后才用于制作。' }}
                </p>
                <Button
                  v-if="autosave.error.value"
                  variant="outline"
                  :disabled="busy"
                  @click="run(() => autosave.flush())"
                >
                  重试暂存
                </Button>
              </div>
              <VideoSubtitleEditor
                :key="selected.id"
                ref="subtitleEditor"
                v-model="subtitleText"
                v-model:position="position"
                :originals="originalCues"
                :original-error="originalError"
                :project-id="id"
                :audio-asset-id="selected.audioAssetId"
                :disabled="busy"
                @playback="onPlayback"
              />
              <div class="actions video-editor-actions">
                <Button
                  variant="outline"
                  :disabled="busy || !dirty || !completePlan"
                  @click="saveEditor"
                >
                  保存校对
                </Button>
                <Button
                  :disabled="busy || running || !providerId"
                  @click="regenerate"
                >
                  {{ legacyPlan ? '保留字幕并升级手绘分镜' : '保存字幕并重新生成分镜' }}
                </Button><Button
                  variant="ghost"
                  :disabled="busy || !dirty"
                  @click="resetEditor"
                >
                  还原未保存修改
                </Button>
              </div>
            </section>
            <section
              id="video-workbench-storyboard"
              class="panel video-refinement"
              tabindex="-1"
              aria-labelledby="video-storyboard-heading"
            >
              <header class="video-refinement-heading">
                <div class="video-editor-heading">
                  <h2
                    id="video-storyboard-heading"
                    class="video-section-title"
                  >
                    分镜精修
                  </h2>
                  <p class="muted">
                    按原声时间码逐镜调整，试听后再决定画面与文字。
                  </p>
                </div>
                <span class="video-scene-count">{{ scenes.length }} 镜</span>
              </header>
              <aside
                v-if="storyboardNotes.length"
                class="video-storyboard-notes"
                aria-label="分镜观看提醒"
              >
                <h3>观看提醒</h3>
                <p class="muted">
                  请结合原声判断，必要的停留和回扣可以保留；不会自动改稿。
                </p>
                <ul>
                  <li
                    v-for="note in storyboardNotes"
                    :key="`${note.sceneIndex}-${note.code}`"
                  >
                    <Button
                      variant="link"
                      @click="focusScene(note.sceneIndex)"
                    >
                      第{{ note.sceneIndex + 1 }}镜
                    </Button>
                    <span>{{ note.message }}</span>
                  </li>
                </ul>
              </aside>
              <div class="video-scenes">
                <VideoSceneEditor
                  v-for="(scene, index) in scenes"
                  :key="index"
                  :project-id="id"
                  :scene-index="index"
                  :scene="scene"
                  :cues="selected.plan.cues"
                  :assets="assets"
                  :providers="imageProviders"
                  :disabled="busy"
                  :image-disabled="busy || running || !completePlan"
                  :revision-disabled="running || !completePlan || !providerId"
                  :playing="playbackScene === index"
                  :audio-available="!!subtitleEditor?.audio"
                  @play="subtitleEditor?.play(scene.first)"
                  @revise="reviseScene(index)"
                  @focusin="position = { ...position, section: 'scene', index }"
                  @update:scene="scenes[index] = $event"
                  @refresh-assets="refreshImages"
                  @working="imageWorking = $event"
                />
              </div>
              <p class="muted">
                共{{ Object.keys(visualCatalog.objects).length }}种手绘物体，每镜最多3个主体。场景只作环境示意，特写突出首个主体，对照保持等权；“重点物体＋特写”在时长允许时轻推近一次。
              </p>
            </section>
            <section
              v-if="completePlan"
              id="video-workbench-preview"
              class="panel video-preview"
              tabindex="-1"
              aria-labelledby="video-preview-heading"
            >
              <div class="video-editor-heading">
                <h2
                  id="video-preview-heading"
                  class="video-section-title"
                >
                  试片与成片
                </h2>
                <p class="muted">
                  使用当前校对和同一绘图流程。试片不是正式成片，不会登记到发布素材。
                </p>
              </div>
              <div
                v-if="!previewKey"
                class="video-preview-empty"
              >
                <AppIcon name="video" />
                <strong>{{ dirty ? '校对有修改，生成试片查看最新画面' : '先看一小段，再做整片' }}</strong>
                <p>选择起始分镜，先检查原声、字幕与画面是否合拍。</p>
              </div>
              <div class="video-preview-controls">
                <div class="video-field">
                  <label for="preview-first-scene">从哪一镜开始</label>
                  <Select
                    v-model="previewFirstScene"
                    :disabled="busy || running"
                  >
                    <SelectTrigger id="preview-first-scene">
                      <SelectValue>第 {{ Number(previewFirstScene) + 1 }} 镜 · {{ scenes[Number(previewFirstScene)]?.title }}</SelectValue>
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem
                        v-for="(scene, index) in scenes"
                        :key="index"
                        :value="String(index)"
                      >
                        第 {{ index + 1 }} 镜 · {{ scene.title }}
                      </SelectItem>
                    </SelectContent>
                  </Select>
                </div>
                <div class="video-field">
                  <label for="preview-seconds">片段时长</label>
                  <Select
                    v-model="previewSeconds"
                    :disabled="busy || running"
                  >
                    <SelectTrigger id="preview-seconds">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="30">
                        30秒
                      </SelectItem><SelectItem value="60">
                        60秒
                      </SelectItem>
                    </SelectContent>
                  </Select>
                </div>
                <Button
                  data-test="preview-video"
                  variant="outline"
                  :disabled="busy || running"
                  @click="preview"
                >
                  <AppIcon name="video" />生成{{ previewSeconds }}秒试片
                </Button>
              </div>
              <p class="muted">
                从所选分镜的原声位置截取，片尾不足时使用剩余时长；原音频、字幕和已有成片保持不变。
              </p>
              <template v-if="previewKey && selected.preview">
                <p class="video-preview-caption">
                  本次试片：原声 {{ selected.preview.start.toFixed(2) }}—{{ selected.preview.end.toFixed(2) }} 秒。检查完仍需确认完整视频。
                </p>
                <video
                  v-if="previewUrl"
                  :src="previewUrl"
                  controls
                  preload="metadata"
                  aria-label="原声试片（非正式成片）"
                  @error="mediaError = '试片无法播放，请重新生成试片。'"
                />
                <p
                  v-else
                  class="muted"
                  role="status"
                >
                  正在读取本次试片…
                </p>
                <div class="video-thumbnails">
                  <Button
                    v-for="frame in previewFrames"
                    :key="frame.sceneIndex"
                    type="button"
                    variant="outline"
                    class="video-thumbnail h-auto whitespace-normal"
                    :aria-label="`查看第${frame.sceneIndex + 1}镜校对`"
                    @click="focusScene(frame.sceneIndex)"
                  >
                    <img
                      :src="frame.src"
                      :alt="`第${frame.sceneIndex + 1}镜渲染缩略图`"
                      loading="lazy"
                      width="1920"
                      height="1080"
                    >
                    <span>第 {{ frame.sceneIndex + 1 }} 镜 · {{ scenes[frame.sceneIndex]?.title }}</span>
                  </Button>
                </div>
                <p class="muted">
                  缩略图来自本次试片，最多展示12镜。点击可定位校对；修改后需重新生成试片。
                </p>
              </template>
              <div class="video-render-action">
                <div>
                  <h3>画面确认后，制作完整视频</h3>
                  <p class="muted">
                    使用当前校对与完整原声，保留原音频和已有成片。
                  </p>
                </div>
                <Button
                  data-test="render-video"
                  class="video-primary-action"
                  :disabled="busy || running || !completePlan"
                  @click="resume(true)"
                >
                  <AppIcon name="video" />确认并生成视频
                </Button>
              </div>
            </section>
          </template>
        </template>
      </div>
      <section
        v-else
        class="panel video-empty"
      >
        <AppIcon name="video" />
        <h2>还没有视频任务</h2>
        <p class="muted">
          选择本书音频和文本模型，生成字幕与分镜后，在这里查看进度和成片。
        </p>
      </section>
    </div>
  </div>
</template>

<style scoped>
.video-page, .video-workspace, .video-task-workspace { display: grid; grid-template-columns: minmax(0, 1fr); gap: 20px; min-width: 0; }
.video-backbar { display: flex; align-items: center; justify-content: space-between; gap: 16px; min-width: 0; }
.video-backbar .back-link { display: flex; flex-wrap: wrap; gap: 6px; margin: 0; font: 400 12px/18px var(--font-mono); }
.video-backbar .back-link:hover, .video-backbar button .app-icon { color: var(--video-accent); }
.video-backbar button { min-height: 30px; height: auto; padding: 6px 14px; flex-shrink: 0; font: 400 12px/16px var(--font-mono); }
.video-title-row { display: flex; align-items: center; flex-wrap: wrap; gap: 12px; }
.video-heading h1 { margin: 0; font-size: 24px; line-height: 32px; font-weight: 700; letter-spacing: -.5px; }
.video-heading p { margin: 4px 0 0; font-size: 14px; line-height: 20px; }
.video-environment-badge { padding: 2px 10px; border: 1px solid var(--video-border); border-radius: 999px; background: var(--video-surface); color: var(--video-accent); font: 400 12px/16px var(--font-mono); }
.video-page :deep(.project-navigation) { margin: 0; }
.video-page :deep(.project-navigation a:not(.is-current) .app-icon) { display: none; }
.video-inputs, .video-job, .video-proofread, .video-refinement, .video-preview { display: grid; grid-template-columns: minmax(0, 1fr); gap: 20px; padding: 24px; min-width: 0; }
.video-inputs { gap: 20px; }
.video-setup { padding: 0; }
.video-setup-trigger { display: flex; align-items: center; gap: 14px; width: 100%; padding: 18px 24px; text-align: left; }
.video-setup-trigger > span { flex: 1; min-width: 0; display: grid; gap: 4px; }
.video-setup-trigger > .app-icon:first-child { color: var(--video-accent); width: 22px; height: 22px; }
.video-setup-trigger > .app-icon:last-child { transform: rotate(90deg); transition: transform 180ms; }
.video-setup-trigger[data-state='open'] > .app-icon:last-child { transform: rotate(-90deg); }
.video-setup-trigger::after { display: none; }
.video-setup-trigger:hover { background: var(--accent); }
.video-setup-trigger .video-section-title { border: 0; padding: 0; }
.video-setup .video-inputs { border-top: 1px solid var(--border); }
.video-setup .video-inputs[data-state='closed'] { padding: 0; border: 0; }
.video-workspace .panel { margin: 0; background: var(--panel-surface); box-shadow: var(--panel-shadow); }
.video-workspace .muted { margin: 0; font: 400 12px/1.75 var(--font-sans); }
.video-inputs > .muted { line-height: 16px; }
.video-workspace :deep(button), .video-workspace :deep(input) { font-family: var(--font-sans); font-size: 13px; }
.video-workspace :deep(label) { margin: 0; }
.video-workspace :deep([data-slot='select-trigger']) { background: var(--input-surface); }
.video-section-title { display: flex; align-items: center; margin: 0; padding-left: 12px; border-left: 3px solid var(--video-accent); color: var(--foreground); font-size: 16px; line-height: 24px; font-weight: 650; }
.video-form-grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 20px 16px; }
.video-field { display: grid; align-content: start; gap: 6px; min-width: 0; }
.video-field label { color: var(--text-secondary); font: 500 13px/20px var(--font-sans); }
.video-field [role='combobox'] { width: 100%; min-width: 0; }
.video-inputs [role='combobox'] { height: 42px; padding: 12px; }
#video-audio { color: var(--video-accent); }
#video-provider { color: var(--primary); }
.video-subtitle-source { grid-column: 1 / -1; }
.video-workspace :deep([data-slot='select-value']) { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; text-align: left; }
.video-field .muted { font-size: 12px; line-height: 1.65; }
.video-inputs :deep(.audio-player:not([data-theme='oled'])) { background: var(--player-surface); border-color: var(--player-line); }
.video-workspace :deep(.audio-waveform) { padding: 12px; border: 1px solid var(--player-line); border-radius: 12px; background: var(--player-surface); }
.video-workspace :deep(.waveform-surface), .video-workspace :deep(.waveform-image) { height: 48px; }
.video-workspace :deep(.player-times) { padding-top: 8px; border-top: 1px solid var(--player-line); margin: 4px 0 0; font-size: 10px; line-height: 15px; }
.video-workspace :deep(.player-times > span:first-child) { color: var(--player-muted); }
.video-workspace :deep(.player-tools) { padding: 0; border: 0; margin-top: 16px; }
.video-workspace :deep(.player-art) { background: color-mix(in srgb, var(--player-accent) 10%, transparent); border-color: color-mix(in srgb, var(--player-accent) 30%, transparent); }
.video-workspace :deep(.player-state) { padding: 4px 12px; border: 1px solid color-mix(in srgb, var(--player-accent) 30%, transparent); border-radius: 999px; background: color-mix(in srgb, var(--player-accent) 10%, transparent); }
.video-workspace :deep(.player-jump) { width: auto; gap: 4px; background: var(--input-surface); }
.video-workspace :deep(.player-jump span) { display: inline; font-size: 10px; }
.video-primary-action { height: auto; min-height: 44px; padding: 12px 24px; border: 0; border-radius: 10px; color: var(--primary-foreground); font-weight: 650; background: var(--primary); }
.video-primary-action:hover:not(:disabled) { filter: brightness(1.08); }
.video-inputs > .video-primary-action { width: 100%; }
.video-runtime { padding: 0; overflow: hidden; }
.video-runtime-trigger { display: flex; width: 100%; align-items: center; justify-content: space-between; gap: 16px; padding: 20px; text-align: left; }
.video-runtime-trigger > span { display: grid; gap: 4px; min-width: 0; }
.video-runtime-trigger .video-section-title { font-size: 14px; }
.video-runtime-trigger .video-section-title { border-color: var(--info); }
.video-runtime-trigger .muted { padding-left: 15px; font-size: 12px; }
.video-runtime-trigger > .app-icon { transform: rotate(90deg); transition: transform 180ms; }
.video-runtime-trigger::after { display: none; }
.video-runtime-trigger[data-state='open'] > .app-icon { transform: rotate(-90deg); }
.video-runtime-trigger:hover { background: color-mix(in srgb, var(--secondary) 40%, transparent); }
.video-runtime-content { display: grid; gap: 20px; padding: 20px 24px 24px; border-top: 1px solid var(--border); }
.video-runtime-content[data-state='closed'] { padding: 0; border: 0; }
.video-path { display: flex; gap: 8px; min-width: 0; }
.video-path input { min-width: 0; }
.video-job-heading, .video-refinement-heading { display: flex; align-items: center; justify-content: space-between; gap: 20px; padding-bottom: 16px; border-bottom: 1px solid var(--border); }
.video-job-heading > div { min-width: 0; }
.video-job-heading p { margin-top: 4px; padding-left: 15px; font-size: 12px; }
.video-job-heading [role='combobox'] { width: auto; min-width: 0; max-width: 360px; }
.video-task-workspace { scroll-margin-top: calc(var(--workspace-sticky-top) + 20px); }
.video-section-nav { display: flex; align-items: center; flex-wrap: wrap; gap: 8px; padding: 8px 0; border-block: 1px solid var(--border); }
.video-section-nav > span { padding-inline: 12px; font-size: 12px; color: var(--muted-foreground); }
.video-section-nav button { gap: 8px; }
.video-section-nav .app-icon { color: var(--video-accent); }
.video-proofread, .video-refinement, .video-preview { scroll-margin-top: calc(var(--workspace-sticky-top) + 20px); }
.video-proofread:focus-visible, .video-refinement:focus-visible, .video-preview:focus-visible { outline: 2px solid var(--video-accent); outline-offset: 4px; }
.video-job-actions { gap: 12px; margin: 0; }
.video-stop-action { color: var(--destructive); border-color: color-mix(in srgb, var(--destructive) 33%, transparent); }
.video-publish-action { margin-left: auto; }
.video-proofread .video-section-title { border-color: var(--info); }
.video-scene-count { padding: 4px 12px; border-radius: 6px; color: var(--video-accent); background: var(--video-surface); font: 500 12px/20px var(--font-mono); white-space: nowrap; align-self: flex-start; }
.video-editor-heading { display: grid; gap: 6px; }
.video-draft-status { margin: -12px 0 0; }
.video-draft-status p { font-size: 12px; }
.video-editor-actions { margin: 0; padding-top: 16px; border-top: 1px solid var(--border); gap: 12px; }
.video-preview-controls { display: grid; grid-template-columns: minmax(0, 2fr) minmax(100px, 1fr) auto; gap: 16px; align-items: end; }
.video-preview .video-section-title { border-color: var(--primary); }
.video-preview-empty { display: grid; justify-items: center; align-content: center; gap: 10px; min-height: 200px; padding: 24px; border: 1px dashed var(--input-border); border-radius: 12px; background: var(--input-surface); text-align: center; }
.video-preview-empty > .app-icon { width: 28px; height: 28px; color: var(--video-accent); }
.video-preview-empty strong { font-size: 14px; font-weight: 500; }
.video-preview-empty p { margin: 0; color: var(--muted-foreground); font-size: 12px; line-height: 1.7; }
.video-preview-caption { margin: 0; color: var(--primary); font: 400 12px/1.65 var(--font-mono); }
.video-thumbnails { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 12px; }
.video-thumbnail { display: grid; gap: 8px; text-align: left; border: 1px solid var(--border); border-radius: 8px; padding: 8px; background: var(--card); min-width: 0; }
.video-thumbnail img { width: 100%; height: auto; aspect-ratio: 16 / 9; border-radius: 4px; }
.video-thumbnail span { font-size: 12px; overflow-wrap: anywhere; }
.video-storyboard-notes { display: grid; gap: 8px; padding: 16px; border-left: 3px solid var(--warning); background: var(--warning-surface); border-radius: 0 8px 8px 0; }
.video-storyboard-notes h3 { margin: 0; color: var(--warning); font-size: 14px; font-weight: 600; }
.video-storyboard-notes ul { padding: 0; margin: 8px 0 0; max-height: 220px; overflow-y: auto; }
.video-storyboard-notes li { display: flex; gap: 10px; align-items: baseline; margin-top: 6px; font: 400 13px/1.7 var(--font-sans); }
.video-storyboard-notes button { padding: 0; height: auto; color: var(--primary); flex-shrink: 0; }
.video-scenes { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); align-items: start; gap: 20px; max-height: 760px; overflow: auto; padding: 3px; }
.video-render-action { display: flex; align-items: center; justify-content: space-between; gap: 24px; border-top: 1px solid var(--border); padding-top: 16px; }
.video-render-action .muted { max-width: 640px; }
.video-render-action h3 { margin: 0 0 6px; font-size: 15px; font-weight: 600; }
.video-render-action button { flex-shrink: 0; }
.video-empty { display: grid; justify-items: center; gap: 12px; padding: 40px 24px; text-align: center; }
.video-empty > .app-icon { width: 32px; height: 32px; color: var(--video-accent); }
.video-empty h2 { margin: 0; font-size: 16px; }
video { display: block; width: 100%; max-height: 640px; aspect-ratio: 16 / 9; background: #020617; border: 1px solid var(--input-border); border-radius: 16px; }
@media (max-width: 1100px) { .video-job-heading, .video-refinement-heading, .video-render-action { align-items: stretch; flex-direction: column; gap: 12px; } .video-job-heading [role='combobox'] { width: 100%; max-width: none; } }
@media (max-width: 980px) { .video-form-grid, .video-scenes, .video-preview-controls { grid-template-columns: minmax(0, 1fr); } }
@media (max-width: 600px) { .video-backbar { align-items: flex-start; flex-direction: column; gap: 12px; } .video-backbar button { align-self: flex-end; } .video-publish-action { margin-left: 0; } .video-thumbnails { grid-template-columns: minmax(0, 1fr); } .video-job-actions, .video-editor-actions { align-items: stretch; flex-direction: column; } }
@media (max-width: 520px) { .video-inputs, .video-job, .video-proofread, .video-refinement, .video-preview { padding: 16px; } .video-runtime-content { padding: 16px; } .video-runtime-trigger, .video-setup-trigger { padding: 16px; } .video-runtime-trigger .muted { display: none; } .video-section-nav > span { flex-basis: 100%; } }
@media (prefers-reduced-motion: reduce) { .video-setup-trigger > .app-icon, .video-runtime-trigger > .app-icon { transition: none; } }
@media (prefers-reduced-transparency: reduce) { .video-workspace .panel { background: var(--card); } }
</style>
