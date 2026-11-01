<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { onBeforeRouteLeave, onBeforeRouteUpdate, RouterLink, useRoute } from 'vue-router'
import AppIcon from '../../components/AppIcon.vue'
import ProjectNavigation from '../../components/ProjectNavigation.vue'
import OperationFeedback from '../../components/OperationFeedback.vue'
import VideoJobProgress from './VideoJobProgress.vue'
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
import { videoStatuses, type VideoJob, type VideoScene, type VideoSettings } from '../../types/video'

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
const mediaError = ref('')
const progressError = ref('')
const jobPanel = ref<globalThis.HTMLElement>()
const selected = computed(() => jobs.value.find(job => job.id === selectedId.value))
const running = computed(() => jobs.value.some(job => job.status === 'running'))
const audios = computed(() => assets.value.filter(a => a.assetType === 'audio' && a.existsStatus === 'available'))
const subtitles = computed(() => assets.value.filter(a => a.assetType === 'subtitle' && /\.srt$/i.test(a.originalName) && a.existsStatus === 'available'))
const editable = computed(() => selected.value && !['running', 'succeeded'].includes(selected.value.status))
const completePlan = computed(() => scenes.value.length > 0 && scenes.value.at(-1)!.last + 1 === selected.value?.plan.cues.length)
const editorValue = () => JSON.stringify([subtitleText.value, scenes.value])
const jobLabel = (job: VideoJob) => `${videoStatuses[job.status]} · ${job.createdAt ? new Date(job.createdAt).toLocaleString() : job.id}`
const dirty = computed(() => loaded.value && editable.value && savedEditor.value !== editorValue())
const configDirty = computed(() => loaded.value && savedConfig.value !== JSON.stringify(config.value))
const runtimeFields: { key: keyof VideoSettings; label: string; directory: boolean; hint: string }[] = [
  { key: 'engineRoot', label: 'Frametell 引擎目录', directory: true, hint: '选择 whiteboard-teacher-video 内的 engine，当前适配 0.7.3。' },
  { key: 'nodePath', label: 'Node 程序', directory: false, hint: '选择 Node 22.19 或更新版本的 node.exe。' },
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
  scenes.value = job.plan.scenes.map(scene => ({ ...scene, labels: [...scene.labels] }))
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
    }
  } catch (cause) { if (!disposed && projectId === props.id) progressError.value = String(cause) }
  finally { polling = false }
}
const choosePath = (key: keyof VideoSettings, directory: boolean) => run(async () => {
  const path = await api.chooseRuntimePath(directory)
  if (typeof path === 'string') config.value[key] = path
})
const saveConfig = () => run(async () => { await api.saveSettings(config.value); savedConfig.value = JSON.stringify(config.value); message.value = '视频运行环境已保存，生成时会检查实际可用性。' })
const create = () => run(async () => {
  if (dirty.value || configDirty.value) throw new Error('请先保存或还原字幕、分镜和运行环境修改。')
  const job = await api.startJob(props.id, audioId.value, providerId.value, subtitleId.value === 'auto' ? null : subtitleId.value)
  put(job); await adopt(job)
  await nextTick()
  jobPanel.value?.scrollIntoView({ block: 'start' })
})
const resume = (render: boolean) => run(async () => {
  if (!selected.value) return
  if (configDirty.value) throw new Error('请先保存运行环境配置。')
  if (render) await api.savePlan(selected.value.id, subtitleText.value, scenes.value)
  else if (dirty.value) throw new Error('字幕已修改，请使用“保存字幕并重新生成分镜”。')
  savedEditor.value = editorValue()
  put(await api.continueJob(selected.value.id, render))
  await nextTick()
  jobPanel.value?.scrollIntoView({ block: 'start' })
})
const regenerate = () => run(async () => {
  if (!selected.value) return
  await api.savePlan(selected.value.id, subtitleText.value, [])
  scenes.value = []; savedEditor.value = editorValue()
  put(await api.continueJob(selected.value.id, false))
})
const stop = () => run(async () => { if (selected.value) { await api.cancelJob(selected.value.id); message.value = '正在停止本任务的识别或渲染进程，原音频不受影响。' } })
const switchJob = (id: unknown) => run(async () => { if (dirty.value) throw new Error('请先保存或还原当前修改。'); selectedId.value = String(id); await adopt(selected.value) })
const resetEditor = () => run(async () => { await adopt(selected.value) })
const saveEditor = () => run(async () => {
  if (!selected.value) return
  await api.savePlan(selected.value.id, subtitleText.value, scenes.value)
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
        横屏 1920 × 1080 · 24 fps · 书卷风。支持两小时以内音频，不重新配音、不加配乐。
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
            :disabled="busy || running"
            @click="resume(false)"
          >
            继续准备字幕与分镜
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
              保存字幕并重新生成分镜
            </Button><Button
              variant="ghost"
              :disabled="busy || !dirty"
              @click="resetEditor"
            >
              还原未保存修改
            </Button>
          </div>
          <div class="video-scenes">
            <article
              v-for="(scene, index) in scenes"
              :key="index"
              class="video-scene"
            >
              <p class="muted">
                第 {{ index + 1 }} 镜 · 字幕 {{ scene.first + 1 }}—{{ scene.last + 1 }} · {{ { book: '书籍', quote: '观点', compare: '对比', flow: '过程', timeline: '时间线' }[scene.kind] }}
              </p><Input
                v-model="scene.title"
                :disabled="busy"
                :aria-label="`第${index + 1}镜标题`"
                maxlength="20"
              /><div class="video-labels">
                <Input
                  v-for="(_, i) in scene.labels"
                  :key="i"
                  v-model="scene.labels[i]"
                  :disabled="busy"
                  :aria-label="`第${index + 1}镜标签${i + 1}`"
                  maxlength="12"
                />
              </div>
            </article>
          </div>
          <div class="video-render-action">
            <p class="muted">
              确认后按此字幕和分镜制作，原音频和旧成片保留。自动分镜采用图解模板，不等同于逐镜定制角色动画。
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
.video-inputs, .video-job, .video-runtime { display: grid; gap: 18px; padding: 24px; }
.video-form-grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 24px; }
.video-field { display: grid; align-content: start; gap: 8px; min-width: 0; }
.video-field .muted { margin: 4px 0 0; }
.video-path, .video-labels { display: flex; gap: 8px; min-width: 0; }
.video-path input, .video-labels input { min-width: 0; }
.video-job-heading { display: flex; align-items: center; justify-content: space-between; gap: 20px; }
.video-job-heading [role='combobox'] { max-width: 360px; }
.video-job { scroll-margin-top: calc(var(--workspace-sticky-top) + 20px); }
.video-runtime summary { cursor: pointer; font-weight: 600; }
.video-runtime summary span { font-weight: 400; margin-left: 12px; }
.video-runtime > :not(summary) { margin-top: 18px; }
.video-scenes { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 16px; max-height: 560px; overflow: auto; padding: 2px; }
.video-scene { display: grid; gap: 10px; padding: 16px; border: 1px solid var(--border); border-radius: 10px; }
.video-editor-heading { margin-top: 12px; }
.video-render-action { display: flex; align-items: center; justify-content: space-between; gap: 24px; border-top: 1px solid var(--border); padding-top: 20px; }
.video-render-action .muted { max-width: 600px; }
.video-error { overflow-wrap: anywhere; white-space: pre-wrap; }
audio { width: 100%; align-self: center; }
video { display: block; width: 100%; max-height: 640px; background: #151d18; border-radius: 10px; }
@media (max-width: 980px) { .video-form-grid, .video-scenes { grid-template-columns: minmax(0, 1fr); } .video-render-action, .video-job-heading { align-items: stretch; flex-direction: column; } .video-job-heading [role='combobox'] { max-width: none; } }
@media (max-width: 520px) { .video-inputs, .video-job, .video-runtime { padding: 16px; } .video-labels { flex-wrap: wrap; } }
</style>
