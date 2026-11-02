<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { onBeforeRouteLeave, onBeforeRouteUpdate, RouterLink, useRoute } from 'vue-router'
import AppIcon from '../../components/AppIcon.vue'
import ProjectNavigation from '../../components/ProjectNavigation.vue'
import OperationFeedback from '../../components/OperationFeedback.vue'
import VideoJobProgress from './VideoJobProgress.vue'
import { inspectStoryboard } from './storyboard'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Textarea } from '@/components/ui/textarea'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { useOperation } from '../../composables/useOperation'
import { useWindowCloseGuard } from '../../composables/useWindowCloseGuard'
import { getProject, isDesktop, listAssets, openAsset, openProjectDirectory } from '../../services/projects'
import { listProviders } from '../../services/ai'
import * as api from '../../services/video'
import type { Asset } from '../../types/project'
import type { ModelProvider } from '../../types/ai'
import { visualCatalog, videoStatuses, type VideoJob, type VideoScene, type VideoSettings } from '../../types/video'

const props = defineProps<{ id: string }>()
const route = useRoute()
const desktop = isDesktop()
const { busy, error, message, run } = useOperation()
const title = ref('')
const loaded = ref(false)
const assets = ref<Asset[]>([])
const providers = ref<ModelProvider[]>([])
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
const audioUrl = ref('')
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
const editable = computed(() => selected.value && !['running', 'succeeded'].includes(selected.value.status))
const legacyPlan = computed(() => scenes.value.some(scene => !scene.beats?.length))
const completePlan = computed(() => scenes.value.length > 0 && !legacyPlan.value && scenes.value.at(-1)!.last + 1 === selected.value?.plan.cues.length)
const storyboardNotes = computed(() => selected.value ? inspectStoryboard({ ...selected.value.plan, scenes: scenes.value }) : [])
const sceneKindLabel = (kind: VideoScene['kind']) => Object.hasOwn(visualCatalog.kinds, kind) ? visualCatalog.kinds[kind as keyof typeof visualCatalog.kinds] : '旧版文字图解'
const editorValue = () => JSON.stringify([subtitleText.value, scenes.value])
const jobLabel = (job: VideoJob) => `${videoStatuses[job.status]} · ${job.createdAt ? new Date(job.createdAt).toLocaleString() : job.id}`
const dirty = computed(() => loaded.value && editable.value && savedEditor.value !== editorValue())
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

function canLeave() {
  if (dirty.value || configDirty.value || busy.value) { error.value = '请先保存或还原当前修改，再离开视频制作。'; return false }
  return true
}
onBeforeRouteLeave(canLeave)
onBeforeRouteUpdate((to, from) => to.params.id === from.params.id || canLeave())
const closeGuard = useWindowCloseGuard(() => !!dirty.value || configDirty.value || busy.value, canLeave)

async function adopt(job: VideoJob | undefined) {
  if (!job) { subtitleText.value = ''; scenes.value = []; savedEditor.value = editorValue(); return }
  const text = await api.subtitles(job.id)
  if (disposed || selectedId.value !== job.id) return
  subtitleText.value = text
  scenes.value = JSON.parse(JSON.stringify(job.plan.scenes))
  savedEditor.value = editorValue()
}
function put(job: VideoJob) { jobs.value = [job, ...jobs.value.filter(j => j.id !== job.id)]; selectedId.value = job.id }
const load = () => run(async () => {
  const [book, files, models, settings, history] = await Promise.all([getProject(props.id), listAssets(props.id), listProviders(), api.settings(), api.listJobs(props.id)])
  if (disposed) return
  title.value = book.title; assets.value = files; providers.value = models.filter(p => p.modelKind === 'text' && p.isEnabled)
  config.value = settings; savedConfig.value = JSON.stringify(settings)
  jobs.value = history
  selectedId.value = history.find(j => j.id === route.query.job)?.id ?? history[0]?.id ?? ''
  audioId.value = audios.value.find(a => a.id === route.query.audio)?.id ?? audios.value[0]?.id ?? ''
  providerId.value = providers.value.find(p => p.isDefault)?.id ?? providers.value[0]?.id ?? ''
  await adopt(selected.value)
  loaded.value = true
})
async function poll() {
  if (!running.value || polling || busy.value || disposed) return
  const projectId = props.id
  polling = true
  try {
    const previous = selected.value
    const history = await api.listJobs(projectId)
    if (disposed || projectId !== props.id) return
    progressError.value = ''
    jobs.value = history
    if (previous?.status === 'running' && selected.value?.status !== 'running') {
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
const saveConfig = () => run(async () => { await api.saveSettings(config.value); savedConfig.value = JSON.stringify(config.value); message.value = '视频运行环境已保存，生成时会检查实际可用性。' })
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
  if (configDirty.value) throw new Error('请先保存运行环境配置。')
  if (render) { await api.savePlan(selected.value.id, subtitleText.value, scenes.value); selected.value.preview = null }
  else if (dirty.value) throw new Error('字幕已修改，请使用“保存字幕并重新生成分镜”。')
  savedEditor.value = editorValue()
  put(await api.continueJob(selected.value.id, render))
  await nextTick()
  jobPanel.value?.scrollIntoView({ block: 'start' })
})
const preview = () => run(async () => {
  if (!selected.value) return
  if (configDirty.value) throw new Error('请先保存运行环境配置。')
  await api.savePlan(selected.value.id, subtitleText.value, scenes.value)
  selected.value.preview = null
  savedEditor.value = editorValue()
  put(await api.previewJob(selected.value.id, Number(previewFirstScene.value), Number(previewSeconds.value)))
  await nextTick()
  jobPanel.value?.scrollIntoView({ block: 'start' })
})
function focusScene(index: number) {
  const card = jobPanel.value?.querySelector<globalThis.HTMLElement>(`#video-scene-${index}`)
  card?.scrollIntoView({ block: 'nearest' })
  card?.querySelector('input')?.focus({ preventScroll: true })
}
const regenerate = () => run(async () => {
  if (!selected.value) return
  if (configDirty.value) throw new Error('请先保存运行环境配置。')
  await api.savePlan(selected.value.id, subtitleText.value, [])
  selected.value.preview = null
  scenes.value = []; savedEditor.value = editorValue()
  put(await api.continueJob(selected.value.id, false))
})
const stop = () => run(async () => { if (selected.value) { await api.cancelJob(selected.value.id); message.value = '正在停止本任务的识别或渲染进程，原音频不受影响。' } })
const switchJob = (id: unknown) => run(async () => { if (dirty.value) throw new Error('请先保存或还原当前修改。'); selectedId.value = String(id); await adopt(selected.value) })
const resetEditor = () => run(async () => { await adopt(selected.value) })
const saveEditor = () => run(async () => {
  if (!selected.value) return
  await api.savePlan(selected.value.id, subtitleText.value, scenes.value)
  selected.value.preview = null
  savedEditor.value = editorValue()
  jobs.value = await api.listJobs(props.id)
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

watch(audioId, async id => {
  audioUrl.value = ''; mediaError.value = ''
  if (!id || !desktop) return
  try { const url = await api.mediaSource(props.id, id); if (!disposed && id === audioId.value) audioUrl.value = url }
  catch (cause) { mediaError.value = `音频无法预览：${String(cause)}` }
})
watch(() => selected.value?.videoAssetId, async id => {
  videoUrl.value = ''; mediaError.value = ''
  if (!id || !desktop) return
  try { const url = await api.mediaSource(props.id, id); if (!disposed && id === selected.value?.videoAssetId) videoUrl.value = url }
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
watch(() => props.id, () => { if (desktop) void load() })
onMounted(() => { if (desktop) { void load(); timer = globalThis.setInterval(() => { void poll() }, 1500) } })
onBeforeUnmount(() => { disposed = true; if (timer) globalThis.clearInterval(timer) })
</script>

<template>
  <header class="page-heading">
    <div>
      <RouterLink
        class="back-link"
        :to="`/projects/${id}/writing?tab=audio`"
      >
        <AppIcon name="back" />{{ title || '书籍音频' }}
      </RouterLink><h1>视频制作</h1><p class="muted">
        让书中的观点动起来，保留原来的声音。
      </p>
    </div>
    <Button
      variant="outline"
      :disabled="busy || !loaded"
      @click="showDirectory"
    >
      <AppIcon name="folder" />打开书籍目录
    </Button>
  </header>
  <ProjectNavigation :id="id" />
  <OperationFeedback
    :error="error || closeGuard.error.value"
    :message="message"
    @dismiss="message = ''"
  />
  <OperationFeedback :error="selected?.error || mediaError" />
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
    <section class="panel video-inputs">
      <h2>从本书音频开始</h2><p class="muted">
        横屏 1920 × 1080 · 24 fps · 同款书卷手绘。支持两小时以内音频，具体物体逐笔绘制；不重新配音、不加配乐。
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
          <label for="video-provider">分镜文本模型</label><Select
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
        <div class="video-field">
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
        <audio
          v-if="audioUrl"
          :src="audioUrl"
          controls
          preload="metadata"
          aria-label="所选原声音频"
          @error="mediaError = '音频预览失败，请核对文件或使用系统播放器。'"
        />
      </div>
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
        请先在模型设置中配置可用的文本模型，用于生成分镜。
      </p>
      <Button
        :disabled="running || busy || !audioId || !providerId || !config.engineRoot || !config.nodePath"
        @click="create"
      >
        <AppIcon name="sparkle" />生成字幕与分镜
      </Button>
    </section>

    <details
      class="panel video-runtime"
      :open="!config.engineRoot || !config.nodePath"
    >
      <summary>视频运行环境 <span class="muted">首次使用时配置</span></summary>
      <p class="muted">
        独立安装的引擎还需要 Chrome/Edge、FFmpeg 和字体。转写在本地 CPU 运行，模型建议放 D 盘；不自动下载，不向模型服务发送音频，仅将字幕发给所选文本模型生成分镜。
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
      <p class="muted">
        Frametell 为 AGPL-3.0 引擎，当前不随书引捆绑分发；首次准备步骤见 MG 视频使用文档。
      </p>
    </details>

    <section
      v-if="jobs.length"
      ref="jobPanel"
      class="panel video-job"
    >
      <div class="video-job-heading">
        <h2>任务与成片</h2><Select
          :model-value="selectedId"
          :disabled="busy || !!dirty"
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
      <template v-if="selected">
        <VideoJobProgress
          :job="selected"
          :sync-error="selected.status === 'running' ? progressError : ''"
        />
        <p
          v-if="selected.error"
          class="notice video-error"
        >
          {{ selected.error }}
        </p>
        <div class="actions">
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
            :disabled="busy"
            @click="stop"
          >
            停止当前任务
          </Button>
          <Button
            v-if="['draft', 'failed', 'cancelled'].includes(selected.status)"
            :disabled="busy || running || legacyPlan"
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
            as-child
          >
            <RouterLink :to="{ path: `/projects/${id}/publishing`, query: { task: 'new', videoJob: selected.id } }">
              用于发布<AppIcon name="arrow" />
            </RouterLink>
          </Button>
        </div>
        <video
          v-if="videoUrl"
          :src="videoUrl"
          controls
          preload="metadata"
          aria-label="原声 MG 视频成片"
          @error="mediaError = '成片预览失败，请刷新任务或使用系统播放器。'"
        />
        <template v-if="editable && selected.plan.cues.length">
          <div class="video-editor-heading">
            <h3>字幕与分镜校对</h3><p class="muted">
              以实际原声为准。人名、外语请试听核对；每条字幕最多32字。更改条数或内容后应重新生成分镜。
            </p>
          </div>
          <p
            v-if="legacyPlan"
            class="notice"
          >
            当前是旧版文字图解分镜，不会按旧模板继续渲染。请保留字幕并升级手绘分镜，原音频和历史成片不变。
          </p>
          <label for="video-subtitles">SRT 字幕</label><Textarea
            id="video-subtitles"
            v-model="subtitleText"
            :disabled="busy"
            rows="9"
            spellcheck="false"
          />
          <div class="actions">
            <Button
              variant="outline"
              :disabled="busy || !dirty || !completePlan"
              @click="saveEditor"
            >
              保存校对
            </Button>
            <Button
              variant="outline"
              :disabled="busy || running"
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
          <section
            v-if="completePlan"
            class="video-preview"
            aria-label="原声试片"
          >
            <div class="video-editor-heading">
              <h3>先看一小段，再做整片</h3>
              <p class="muted">
                使用当前校对和同一绘图流程。试片不是正式成片，不会登记到发布素材。
              </p>
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
                <button
                  v-for="frame in previewFrames"
                  :key="frame.sceneIndex"
                  type="button"
                  class="video-thumbnail"
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
                </button>
              </div>
              <p class="muted">
                缩略图来自本次试片，最多展示12镜。点击可定位校对；修改后需重新生成试片。
              </p>
            </template>
          </section>
          <aside
            v-if="storyboardNotes.length"
            class="video-storyboard-notes"
            aria-label="分镜观看提醒"
          >
            <h3>观看提醒</h3><p class="muted">
              不是格式错误，也不会自动改稿。请结合原声判断，必要的停留和回扣可以保留。
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
                </Button><span>{{ note.message }}</span>
              </li>
            </ul>
          </aside>
          <div class="video-scenes">
            <article
              v-for="(scene, index) in scenes"
              :id="`video-scene-${index}`"
              :key="index"
              class="video-scene"
            >
              <p class="muted">
                第 {{ index + 1 }} 镜 · 字幕 {{ scene.first + 1 }}—{{ scene.last + 1 }} · {{ sceneKindLabel(scene.kind) }}
              </p><Input
                v-model="scene.title"
                :disabled="busy || !scene.beats?.length"
                :aria-label="`第${index + 1}镜标题`"
                maxlength="20"
              /><div
                v-for="(beat, i) in scene.beats"
                :key="i"
                class="video-beat"
              >
                <label :for="`scene-${index}-beat-${i}`">字幕 {{ beat.cue + 1 }} · {{ visualCatalog.objects[beat.object] }} · {{ visualCatalog.actions[beat.action] }}</label>
                <Input
                  :id="`scene-${index}-beat-${i}`"
                  v-model="beat.label"
                  :disabled="busy"
                  :aria-label="`第${index + 1}镜物体${i + 1}标签`"
                  maxlength="8"
                />
                <details class="video-beat-settings">
                  <summary>调整物体、动作与字幕锚点</summary>
                  <div class="video-beat-controls">
                    <div class="video-field">
                      <label :for="`scene-${index}-object-${i}`">具体物体</label>
                      <Select
                        v-model="beat.object"
                        :disabled="busy"
                      >
                        <SelectTrigger
                          :id="`scene-${index}-object-${i}`"
                          :aria-label="`第${index + 1}镜物体${i + 1}`"
                        >
                          <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                          <SelectItem
                            v-for="(name, key) in visualCatalog.objects"
                            :key="key"
                            :value="key"
                          >
                            {{ name }}
                          </SelectItem>
                        </SelectContent>
                      </Select>
                    </div>
                    <div class="video-field">
                      <label :for="`scene-${index}-action-${i}`">语义动作</label>
                      <Select
                        v-model="beat.action"
                        :disabled="busy"
                      >
                        <SelectTrigger
                          :id="`scene-${index}-action-${i}`"
                          :aria-label="`第${index + 1}镜物体${i + 1}动作`"
                        >
                          <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                          <SelectItem
                            v-for="(name, key) in visualCatalog.actions"
                            :key="key"
                            :value="key"
                          >
                            {{ name }}
                          </SelectItem>
                        </SelectContent>
                      </Select>
                    </div>
                    <div class="video-field video-anchor">
                      <label :for="`scene-${index}-cue-${i}`">在哪句开始出现</label>
                      <Select
                        :model-value="String(beat.cue)"
                        :disabled="busy"
                        @update:model-value="beat.cue = Number($event)"
                      >
                        <SelectTrigger
                          :id="`scene-${index}-cue-${i}`"
                          :aria-label="`第${index + 1}镜物体${i + 1}字幕锚点`"
                        >
                          <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                          <SelectItem
                            v-for="(cue, offset) in selected.plan.cues.slice(scene.first, scene.last + 1)"
                            :key="offset"
                            :value="String(scene.first + offset)"
                          >
                            {{ scene.first + offset + 1 }} · {{ cue.text }}
                          </SelectItem>
                        </SelectContent>
                      </Select>
                    </div>
                  </div>
                </details>
              </div>
              <p
                v-if="!scene.beats?.length"
                class="muted"
              >
                {{ scene.labels?.join(' · ') }}
              </p>
              <div
                v-if="scene.beats?.length"
                class="video-scene-controls"
              >
                <div class="video-field video-scene-kind">
                  <label :for="`scene-${index}-kind`">叙事构图</label>
                  <Select
                    v-model="scene.kind"
                    :disabled="busy"
                  >
                    <SelectTrigger
                      :id="`scene-${index}-kind`"
                      :aria-label="`第${index + 1}镜构图`"
                    >
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem
                        v-for="(name, key) in visualCatalog.kinds"
                        :key="key"
                        :value="key"
                      >
                        {{ name }}
                      </SelectItem>
                    </SelectContent>
                  </Select>
                </div>
                <div class="video-field">
                  <label :for="`scene-${index}-environment`">环境场景</label>
                  <Select
                    :model-value="scene.environment ?? 'plain'"
                    :disabled="busy"
                    @update:model-value="scene.environment = $event as NonNullable<VideoScene['environment']>"
                  >
                    <SelectTrigger
                      :id="`scene-${index}-environment`"
                      :aria-label="`第${index + 1}镜场景`"
                    >
                      <SelectValue>{{ visualCatalog.environments[scene.environment ?? 'plain'] }}</SelectValue>
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem
                        v-for="(name, key) in visualCatalog.environments"
                        :key="key"
                        :value="key"
                      >
                        {{ name }}
                      </SelectItem>
                    </SelectContent>
                  </Select>
                </div>
                <div class="video-field">
                  <label :for="`scene-${index}-framing`">画面景别</label>
                  <Select
                    :model-value="scene.framing ?? 'reference'"
                    :disabled="busy"
                    @update:model-value="scene.framing = $event as NonNullable<VideoScene['framing']>"
                  >
                    <SelectTrigger
                      :id="`scene-${index}-framing`"
                      :aria-label="`第${index + 1}镜景别`"
                    >
                      <SelectValue>{{ visualCatalog.framings[scene.framing ?? 'reference'] }}</SelectValue>
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem
                        v-for="(name, key) in visualCatalog.framings"
                        :key="key"
                        :value="key"
                      >
                        {{ name }}
                      </SelectItem>
                    </SelectContent>
                  </Select>
                </div>
              </div>
            </article>
          </div>
          <div class="video-render-action">
            <p class="muted">
              共{{ Object.keys(visualCatalog.objects).length }}种手绘物体，每镜最多3个主体。场景只作环境示意，特写突出首个主体，对照保持等权；“重点物体＋特写”在时长允许时轻推近一次。请先看试片，确认后再生成整片，原音频与旧成片保留。
            </p><Button
              data-test="render-video"
              :disabled="busy || running || !completePlan"
              @click="resume(true)"
            >
              <AppIcon name="video" />确认并生成视频
            </Button>
          </div>
        </template>
      </template>
    </section>
  </div>
</template>

<style scoped>
.video-workspace { display: grid; gap: 24px; }
.video-inputs, .video-job, .video-runtime { display: grid; grid-template-columns: minmax(0, 1fr); gap: 18px; padding: 24px; }
.video-job > .actions { margin-top: 0; }
.video-form-grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 24px; }
.video-field { display: grid; align-content: start; gap: 8px; min-width: 0; }
.video-field [role='combobox'] { width: 100%; min-width: 0; }
.video-workspace :deep([data-slot='select-value']) { min-width: 0; text-align: left; }
.video-field .muted { margin: 4px 0 0; }
.video-scene-controls { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 12px; }
.video-scene-kind { grid-column: 1 / -1; }
.video-path { display: flex; gap: 8px; min-width: 0; }
.video-path input { min-width: 0; }
.video-beat { display: grid; gap: 6px; min-width: 0; }
.video-beat label { color: var(--muted-foreground); font-size: 12px; overflow-wrap: anywhere; }
.video-beat-settings summary { cursor: pointer; color: var(--primary); font-size: 13px; padding-block: 6px; }
.video-beat-controls { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 12px; padding-block: 10px; }
.video-anchor { grid-column: 1 / -1; }
.video-preview { display: grid; gap: 14px; padding: 20px; border: 1px solid var(--border); border-radius: 10px; background: var(--background); }
.video-preview .video-editor-heading { margin-top: 0; }
.video-preview-controls { display: grid; grid-template-columns: minmax(0, 2fr) minmax(100px, 1fr) auto; gap: 16px; align-items: end; }
.video-preview-caption { color: var(--primary); font-size: 14px; }
.video-thumbnails { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 12px; }
.video-thumbnail { display: grid; gap: 8px; text-align: left; border: 1px solid var(--border); border-radius: 8px; padding: 8px; background: var(--card); min-width: 0; }
.video-thumbnail img { width: 100%; height: auto; aspect-ratio: 16 / 9; border-radius: 4px; }
.video-thumbnail span { font-size: 13px; overflow-wrap: anywhere; }
.video-storyboard-notes { padding: 16px; border-left: 3px solid var(--primary); background: var(--accent); border-radius: 6px; }
.video-storyboard-notes ul { padding: 0; margin: 8px 0 0; max-height: 220px; overflow-y: auto; }
.video-storyboard-notes li { display: flex; gap: 10px; align-items: baseline; margin-top: 6px; font-size: 13px; }
.video-storyboard-notes button { padding: 0; height: auto; }
.video-job-heading { display: flex; align-items: center; justify-content: space-between; gap: 20px; }
.video-job-heading [role='combobox'] { width: 100%; min-width: 0; max-width: 360px; }
.video-job { scroll-margin-top: calc(var(--workspace-sticky-top) + 20px); }
.video-runtime summary { cursor: pointer; font-weight: 600; }
.video-runtime summary span { font-weight: 400; margin-left: 12px; }
.video-runtime > :not(summary) { margin-top: 18px; }
.video-scenes { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 16px; max-height: 560px; overflow: auto; padding: 2px; }
.video-scene { display: grid; gap: 10px; padding: 16px; border: 1px solid var(--border); border-radius: 10px; }
.video-scene { scroll-margin-block: 16px; min-width: 0; }
.video-editor-heading { margin-top: 12px; }
.video-render-action { display: flex; align-items: center; justify-content: space-between; gap: 24px; border-top: 1px solid var(--border); padding-top: 20px; }
.video-render-action .muted { max-width: 600px; }
.video-error { overflow-wrap: anywhere; white-space: pre-wrap; }
audio { width: 100%; align-self: center; }
video { display: block; width: 100%; max-height: 640px; background: #151d18; border-radius: 10px; }
@media (max-width: 980px) { .video-form-grid, .video-scenes { grid-template-columns: minmax(0, 1fr); } .video-render-action, .video-job-heading { align-items: stretch; flex-direction: column; } .video-job-heading [role='combobox'] { max-width: none; } }
@media (max-width: 520px) { .video-inputs, .video-job, .video-runtime { padding: 16px; } }
@media (max-width: 720px) { .video-preview-controls, .video-thumbnails { grid-template-columns: minmax(0, 1fr); } .video-preview { padding: 14px; } }
</style>
