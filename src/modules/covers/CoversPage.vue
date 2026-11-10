<script setup lang="ts">
import { displayPath } from '@/lib/displayPath'
import { Badge } from '@/components/ui/badge'
import { Checkbox } from '@/components/ui/checkbox'
import { Button } from '@/components/ui/button'
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '@/components/ui/collapsible'
import { Slider } from '@/components/ui/slider'
import { Input } from '@/components/ui/input'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { Textarea } from '@/components/ui/textarea'
import { computed, nextTick, onBeforeUnmount, onMounted, ref, shallowRef, watch, type ComponentPublicInstance } from 'vue'
import { RouterLink, onBeforeRouteLeave, onBeforeRouteUpdate, useRoute } from 'vue-router'
import * as api from '../../services/covers'
import { listProviders, listVersions } from '../../services/ai'
import { generationJobs, generationKey, isGenerating, startBackgroundGeneration, cancelGenerationJob, dismissGeneration } from '../../services/generation'
import { emptyPublishing, type ModelProvider, type PublishingContent } from '../../types/ai'
import { getProject, isDesktop, linkAsset, listAssets, openProjectDirectory } from '../../services/projects'
import { backgroundRect, coverPng, decodeBackground, renderCover, type CoverTextBounds } from '../../services/coverRenderer'
import { coverSizes, coverTemplates, newCover, coverWorkspace, recoverCoverWorkspace, normalizeCover, coverSizeIndex, type CoverWorkspace, type CoverConfig, type CoverVersion, type CoverPreset } from '../../types/cover'
import type { Asset } from '../../types/project'
import { useOperation } from '../../composables/useOperation'
import { useEditorDraft } from '../../composables/useEditorDraft'
import { readDraft } from '../../services/drafts'
import AppIcon from '../../components/AppIcon.vue'
import ProjectNavigation from '../../components/ProjectNavigation.vue'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import OperationFeedback from '../../components/OperationFeedback.vue'
import ConfirmAction from '../../components/ConfirmAction.vue'
import { requestConfirmation } from '../../composables/useConfirmation'
import { useQueryState } from '../../composables/useQueryState'
import CoverDesignPreview from '../../components/CoverDesignPreview.vue'
import CoverThumbnail from '../../components/CoverThumbnail.vue'

const props = defineProps<{ id: string }>()
const route = useRoute()
const desktop = isDesktop()
const ready = ref(false)
const bookTitle = ref('')
const workspace = ref('')
const editor = ref(coverWorkspace([]))
const draft = computed(() => editor.value.covers[editor.value.activeSize]!)
const currentSize = computed(() => coverSizes[editor.value.activeSize]!)
const original = ref(JSON.stringify(editor.value.covers))
const dirty = computed(() => original.value !== JSON.stringify(editor.value.covers))
const versions = ref<CoverVersion[]>([])
const historyOpen = ref(false)
const recentExports = computed(() => versions.value.filter(version => version.exportedPath).slice(0, 6))
const linkedCover = computed(() => versions.value.find(version => version.id === route.query.coverVersion))
const selected = ref<CoverVersion | null>(null)
const images = ref<Asset[]>([])
const canvas = ref<globalThis.HTMLCanvasElement | null>(null)
const previewCanvases: (globalThis.HTMLCanvasElement | null)[] = []
const presets = ref<CoverPreset[]>([])
const presetId = ref<string | null>(null)
const presetName = ref('')
const deletingPreset = ref(false)
const selectedPreset = computed(() => presets.value.find(p => p.id === presetId.value))
const backgrounds = shallowRef<Record<string, { image: globalThis.HTMLImageElement | null; error: string }>>({})
const backgroundTasks = new Map<string, Promise<void>>()
const background = computed(() => backgrounds.value[draft.value.backgroundAssetId ?? '']?.image ?? null)
const backgroundLoading = computed(() => !!draft.value.backgroundAssetId && !backgrounds.value[draft.value.backgroundAssetId])
const backgroundError = computed(() => backgrounds.value[draft.value.backgroundAssetId ?? '']?.error ?? '')
const exportIndexes = ref(coverSizes.map((_, index) => index))
const exportReady = computed(() => exportIndexes.value.length > 0 && exportIndexes.value.every(index => {
  const config = editor.value.covers[index]!
  return !config.backgroundAssetId || backgrounds.value[config.backgroundAssetId]?.image
}))
function selectExport(index: number, checked: boolean) {
  exportIndexes.value = checked ? [...exportIndexes.value, index] : exportIndexes.value.filter(value => value !== index)
}
const previewError = ref('')
const actualSizes = ref<{ titleSize: number; subtitleSize: number; bounds: Record<'title' | 'subtitle', CoverTextBounds> | null }>({ titleSize: 112, subtitleSize: 44, bounds: null })
const inlineField = ref<'title' | 'subtitle' | null>(null)
const inlineStyle = ref({ left: '0px', top: '0px', width: '160px' })
let inlineOriginal = ''
let resizeObserver: globalThis.ResizeObserver | undefined
let previewAnimation: globalThis.Animation | undefined
function positionInline() {
  if (!inlineField.value || !canvas.value) return
  const rect = canvas.value.getBoundingClientRect()
  const stage = canvas.value.parentElement!.getBoundingClientRect()
  const bounds = actualSizes.value.bounds?.[inlineField.value]
  if (!bounds) return
  const width = Math.min(stage.width - 16, Math.max(130, (bounds.width || draft.value.width * .82) * rect.width / draft.value.width))
  const left = Math.max(8, Math.min(rect.left - stage.left + bounds.x * rect.width / draft.value.width, stage.width - width - 8))
  inlineStyle.value = { left: `${left}px`, top: `${rect.top - stage.top + bounds.y * rect.height / draft.value.height}px`, width: `${width}px` }
}
async function editInline(field: 'title' | 'subtitle') {
  if (busy.value) return
  inlineField.value = field; inlineOriginal = draft.value[field]; drag = null
  positionInline()
  await nextTick()
  canvas.value?.parentElement?.querySelector<globalThis.HTMLInputElement>('#cover-inline-text')?.focus()
}
function endInline(cancel = false) {
  if (cancel && inlineField.value) draft.value[inlineField.value] = inlineOriginal
  inlineField.value = null
}
function inlineKey(event: globalThis.KeyboardEvent) {
  if (event.isComposing || event.keyCode === 229) return
  if (event.key === 'Enter' || event.key === 'Escape') { event.preventDefault(); endInline(event.key === 'Escape') }
}
function textAtPointer(event: globalThis.MouseEvent) {
  if (!canvas.value || !actualSizes.value.bounds) return
  const rect = canvas.value.getBoundingClientRect()
  const x = (event.clientX - rect.left) * draft.value.width / rect.width
  const y = (event.clientY - rect.top) * draft.value.height / rect.height
  return (['title', 'subtitle'] as const).find(key => {
    const bounds = actualSizes.value.bounds![key]
    return x >= bounds.x && x <= bounds.x + bounds.width && y >= bounds.y && y <= bounds.y + bounds.height
  })
}
function editAtPointer(event: globalThis.MouseEvent) {
  const field = textAtPointer(event)
  if (field) void editInline(field)
}
function fadePreview() {
  previewAnimation?.cancel()
  if (!globalThis.matchMedia('(prefers-reduced-motion: reduce)').matches) previewAnimation = canvas.value?.animate?.([{ opacity: .45 }, { opacity: 1 }], { duration: 160 })
}
watch(canvas, element => {
  resizeObserver?.disconnect()
  if (element) { resizeObserver = new globalThis.ResizeObserver(positionInline); resizeObserver.observe(element) }
}, { flush: 'post' })
onBeforeUnmount(() => { resizeObserver?.disconnect(); previewAnimation?.cancel() })
const { busy: operationBusy, error, message, operation, run } = useOperation()
const generation = computed(() => generationJobs.get(generationKey(props.id, 'cover')))
const generating = computed(() => isGenerating(generation.value))
const busy = computed(() => operationBusy.value || generating.value)
const generatedImage = computed(() => {
  const result = generation.value?.result
  return result && 'assetType' in result ? result : null
})
const activeTab = useQueryState('tab', 'background', ['text', 'style', 'background'])
const useAi = ref(activeTab.value === 'background')
const coverView = ref('edit')
const tabs = [{ value: 'text', label: '文字内容' }, { value: 'style', label: '排版样式' }, { value: 'background', label: '背景图片' }]
const imageModels = ref<ModelProvider[]>([])
const providerId = ref('')
const selectedImageModel = computed(() => imageModels.value.find(model => model.id === providerId.value))
const previewWidth = computed(() => [480, 300, 520][editor.value.activeSize]!)
const backgroundOffset = computed(() => {
  if (!background.value) return { x: 0, y: 0 }
  const rect = backgroundRect(draft.value, background.value)
  const scale = previewWidth.value / draft.value.width
  return { x: Math.round((rect.x - (draft.value.width - rect.width) / 2) * scale), y: Math.round((rect.y - (draft.value.height - rect.height) / 2) * scale) }
})
const description = computed({ get: () => draft.value.backgroundPrompt, set: value => { draft.value.backgroundPrompt = value } })
const writing = ref<PublishingContent | null>(null)
const writingError = ref('')
const suggestedPrompt = computed(() => writing.value?.[currentSize.value.promptField] ?? '')
const taskId = computed(() => generating.value ? generation.value?.id ?? '' : '')
const cancelling = computed(() => generation.value?.cancelling ?? false)
let disposed = false
let fontsReady = false
const canDiscard = () => !dirty.value || requestConfirmation('封面尚未保存，确定放弃修改？', 'cover-history')
// A saved version contains one size; retain the workspace to recover all sizes and the active selection.
const autosave = useEditorDraft(() => props.id, 'cover', editor, () => selected.value?.id ?? null, dirty, ready, () => !operationBusy.value, true)
const canLeave = async () => {
  if (operationBusy.value) return false
  try { if (ready.value) await autosave.flush(); return true } catch { return false }
}
onBeforeRouteLeave(canLeave)
onBeforeRouteUpdate((to, from) => to.params.id === from.params.id || canLeave())
function beforeUnload(event: globalThis.BeforeUnloadEvent) { if (dirty.value || busy.value) event.preventDefault() }
globalThis.addEventListener('beforeunload', beforeUnload)
onBeforeUnmount(() => { disposed = true; globalThis.removeEventListener('beforeunload', beforeUnload) })

function accept(version: CoverVersion | null) {
  selected.value = version
  const config = version?.config ?? newCover(bookTitle.value)
  const index = coverSizeIndex(config)
  const size = coverSizes[index]!
  if (size.width !== config.width || size.height !== config.height) message.value = `历史封面为 ${config.width} × ${config.height}，当前按${size.label}继续编辑，原版本和文件保留。`
  editor.value.activeSize = index
  editor.value.covers[index] = normalizeCover(config)
}
async function loadWriting() {
  writingError.value = ''
  try {
    const textVersions = await listVersions(props.id, 'publishing')
    const textVersion = route.query.writingVersion ? textVersions.find(v => v.id === route.query.writingVersion) : textVersions[0]
    writing.value = textVersion ? { ...emptyPublishing(), ...textVersion.content } : null
  } catch {
    writingError.value = '封面文案读取失败，可重新读取；仍可使用本地模板。'
  }
}
const load = () => run(async () => {
  const [project, saved, assets, recovery, styles] = await Promise.all([getProject(props.id), api.listCovers(props.id), listAssets(props.id), readDraft<CoverWorkspace | CoverConfig>(props.id, 'cover'), api.listPresets(), loadWriting()])
  bookTitle.value = project.title; workspace.value = project.workspacePath
  versions.value = saved; images.value = assets.filter(a => a.assetType === 'image')
  presets.value = styles
  editor.value = coverWorkspace(saved, newCover(bookTitle.value))
  accept(saved[0] ?? null)
  original.value = JSON.stringify(editor.value.covers)
  if (recovery) {
    selected.value = saved.find(v => v.id === recovery.basedOn) ?? null
    editor.value = recoverCoverWorkspace(recovery.content)
    if (dirty.value) message.value = '已恢复各尺寸封面编辑配置，未新增历史版本。'
  }
  else if (!saved.length && writing.value?.coverTitle && writing.value.coverSubtitle) {
    applyWritingTitles()
    if (writing.value.coverDesign) applyWritingDesign()
    editor.value.covers = coverSizes.map((_, index) => normalizeCover(draft.value, index))
  }
  ready.value = true
  await nextTick(); paint()
})
function applyWritingTitles() {
  if (!writing.value) return
  draft.value.title = writing.value.coverTitle
  draft.value.subtitle = writing.value.coverSubtitle
  message.value = '已带入封面主副标题，确认预览后保存或导出。'
}
function applyWritingDesign() {
  if (!writing.value?.coverDesign) return
  Object.assign(draft.value, writing.value.coverDesign)
  presetId.value = null
  message.value = '已应用 AI 推荐字号、主副标题配色、点缀色和蒙层。可继续调整，确认预览后保存或导出。'
}
async function copyText(text: string, label: string) {
  error.value = ''; message.value = ''
  try { await globalThis.navigator.clipboard.writeText(text); message.value = `已复制${label}。`; error.value = '' }
  catch { error.value = '复制失败，请选中文本后手动复制。' }
}
function usePrompt() {
  description.value = suggestedPrompt.value
  activeTab.value = 'background'
  useAi.value = true
  void loadImageModels()
}
async function loadBackground(id: string | null) {
  if (!id) return
  if (backgroundTasks.has(id)) return backgroundTasks.get(id)
  const task = (async () => {
    try {
      const image = await decodeBackground(await api.readBackground(props.id, id))
      if (!disposed) backgrounds.value = { ...backgrounds.value, [id]: { image, error: '' } }
    } catch (cause) {
      if (!disposed) backgrounds.value = { ...backgrounds.value, [id]: { image: null, error: cause instanceof Error ? cause.message : String(cause) } }
    } finally {
      backgroundTasks.delete(id)
      paint()
    }
  })()
  backgroundTasks.set(id, task)
  await task
}
function paint() {
  if (!canvas.value || !fontsReady || disposed) return
  previewError.value = ''
  try {
    actualSizes.value = renderCover(canvas.value, draft.value, background.value)
    positionInline()
  }
  catch (cause) { previewError.value = cause instanceof Error ? cause.message : String(cause) }
  editor.value.covers.forEach((config, index) => {
    const target = previewCanvases[index]
    if (!target) return
    try { renderCover(target, config, backgrounds.value[config.backgroundAssetId ?? '']?.image ?? null) }
    catch { target.width = config.width; target.height = config.height }
  })
}
function setPreviewCanvas(el: globalThis.Element | ComponentPublicInstance | null, index: number) {
  previewCanvases[index] = el instanceof globalThis.HTMLCanvasElement ? el : null
}
let drag: { x: number; y: number; positionX: number; positionY: number; overflowX: number; overflowY: number } | null = null
function startDrag(event: globalThis.PointerEvent) {
  if (busy.value || inlineField.value || textAtPointer(event) || !background.value || !canvas.value || event.button !== 0) return
  const rect = backgroundRect(draft.value, background.value)
  const bounds = canvas.value.getBoundingClientRect()
  drag = { x: event.clientX, y: event.clientY, positionX: draft.value.backgroundX, positionY: draft.value.backgroundY, overflowX: (rect.width - draft.value.width) * bounds.width / draft.value.width, overflowY: (rect.height - draft.value.height) * bounds.height / draft.value.height }
  canvas.value.setPointerCapture(event.pointerId)
  event.preventDefault()
}
function moveDrag(event: globalThis.PointerEvent) {
  if (!drag) return
  const clamp = (value: number) => Math.min(1, Math.max(0, value))
  if (drag.overflowX > 0) draft.value.backgroundX = clamp(drag.positionX - (event.clientX - drag.x) / drag.overflowX)
  if (drag.overflowY > 0) draft.value.backgroundY = clamp(drag.positionY - (event.clientY - drag.y) / drag.overflowY)
}
function applyPreset(id: string | null) {
  const preset = presets.value.find(p => p.id === id)
  if (!preset) return
  Object.assign(draft.value, preset.style, { subtitleColor: preset.style.subtitleColor ?? preset.style.textColor }); presetId.value = id
  fadePreview()
  message.value = `已应用「${preset.name}」，可继续为本书调整。`
}
const savePreset = () => run(async () => {
  await api.savePreset(presetName.value.trim(), { ...draft.value })
  presets.value = await api.listPresets(); presetName.value = ''; message.value = '已保存到我的封面样式。'
})
const deletePreset = () => run(async () => {
  if (!presetId.value) return
  await api.deletePreset(presetId.value)
  presets.value = presets.value.filter(p => p.id !== presetId.value); presetId.value = null; deletingPreset.value = false; message.value = '已删除个人样式，当前封面保持原样。'
})
watch(() => editor.value.covers.map(config => config.backgroundAssetId), ids => {
  for (const id of new Set(ids)) if (id && !backgrounds.value[id]) void loadBackground(id)
})
watch(editor, paint, { deep: true, flush: 'post' })

function applyTemplate(key: CoverConfig['templateId']) {
  endInline()
  Object.assign(draft.value, coverTemplates[key].style, { templateId: key })
  presetId.value = null
  fadePreview()
}
function isTemplateApplied(key: CoverConfig['templateId']) {
  return draft.value.templateId === key && Object.entries(coverTemplates[key].style).every(([field, value]) => draft.value[field as keyof CoverConfig] === value)
}
function changeSize(index: number) {
  if (busy.value) return
  endInline()
  editor.value.activeSize = index
  presetId.value = null
  drag = null
}
async function shareCover(backgroundOnly: boolean) {
  if (!await requestConfirmation(backgroundOnly ? '将当前背景和裁切应用到其他尺寸？其他尺寸的文字、排版和提示词保持不变。' : '将当前封面的文字、排版、背景、裁切和画面描述应用到其他尺寸？将替换其他尺寸的编辑，可继续分别调整；历史版本保留。')) return
  const source = { ...draft.value }
  editor.value.covers = editor.value.covers.map((config, index) => normalizeCover(backgroundOnly
    ? { ...config, backgroundAssetId: source.backgroundAssetId, backgroundColor: source.backgroundColor, overlayOpacity: source.overlayOpacity, backgroundX: source.backgroundX, backgroundY: source.backgroundY, backgroundZoom: source.backgroundZoom }
    : source, index))
  message.value = backgroundOnly ? '全部尺寸已共用当前背景，仍可分别调整。' : '已应用到全部尺寸，仍可分别调整；导出时按各自尺寸裁切排版。'
}
async function switchVersion(id: string | null) {
  if (await canDiscard()) accept(versions.value.find(v => v.id === id) ?? null)
}
const chooseImage = () => run(async () => {
  const path = await api.chooseBackground()
  if (!path) return
  const id = await linkAsset(props.id, path)
  images.value = (await listAssets(props.id)).filter(a => a.assetType === 'image')
  draft.value.backgroundAssetId = id
})
const loadImageModels = () => run(async () => {
  imageModels.value = (await listProviders()).filter(p => p.isEnabled && p.modelKind === 'image')
  providerId.value = imageModels.value.find(p => p.id === providerId.value)?.id ?? imageModels.value.find(p => p.isDefault)?.id ?? imageModels.value[0]?.id ?? ''
})
function toggleAi() {
  useAi.value = !useAi.value
  if (useAi.value) {
    if (!description.value) description.value = suggestedPrompt.value || `为《${bookTitle.value}》设计简洁、有阅读氛围的封面背景，主体居中，四周留白。`
    void loadImageModels()
  }
}
const generateBackground = () => run(async () => {
  await autosave.flush()
  await startBackgroundGeneration(props.id, bookTitle.value, providerId.value, description.value, draft.value.width, draft.value.height)
})
watch([ready, () => generation.value?.status], () => {
  const job = generation.value
  if (!ready.value || !job || isGenerating(job)) return
  if (job.error) { error.value = job.error; dismissGeneration(job) }
  if (generatedImage.value) {
    images.value = [generatedImage.value, ...images.value.filter(image => image.id !== generatedImage.value!.id)]
    void loadBackground(generatedImage.value.id)
  }
})
const applyGeneratedImage = () => run(async () => {
  const job = generation.value, asset = generatedImage.value
  if (!job?.size || !asset) return
  await loadBackground(asset.id)
  if (!backgrounds.value[asset.id]?.image) throw new Error('生成背景读取失败，请重试，当前封面保持不变。')
  const target = editor.value.covers.find(config => config.width === job.size!.width && config.height === job.size!.height)!
  Object.assign(target, { backgroundAssetId: asset.id, backgroundX: 0.5, backgroundY: 0.5, backgroundZoom: 1 })
  await autosave.flush()
  dismissGeneration(job)
  message.value = '已采用生成背景到原请求尺寸，其他尺寸保持不变。'
})
async function cancelImage() {
  if (!generation.value) return
  try { await cancelGenerationJob(generation.value) }
  catch { error.value = '取消失败，请稍后重试。' }
}
const save = (exportPng = false, currentOnly = false) => run(async () => {
  if (exportPng && !currentOnly && !exportIndexes.value.length) return
  operation.value = '正在暂存编辑…'
  await autosave.flush()
  const source = { ...draft.value }
  const configs = (currentOnly ? [source] : editor.value.covers.filter((_, index) => !exportPng || exportIndexes.value.includes(index))).map(config => ({ ...config }))
  // Render all ratios before writing any file so layout failures keep the whole draft.
  operation.value = exportPng ? '正在读取背景并渲染…' : '正在保存全部编辑配置…'
  const outputs = await Promise.all(configs.map(async config => {
    if (exportPng && config.backgroundAssetId) {
      await loadBackground(config.backgroundAssetId)
      const loaded = backgrounds.value[config.backgroundAssetId]
      if (!loaded?.image) throw new Error(`${coverSizes[coverSizeIndex(config)]!.label}：${loaded?.error || '背景尚未读取'}`)
    }
    return { config, png: exportPng ? await coverPng(config, backgrounds.value[config.backgroundAssetId ?? '']?.image ?? null) : null }
  }))
  const saved: CoverVersion[] = []
  for (const { config, png } of outputs) {
    operation.value = exportPng ? `正在导出 ${saved.length + 1}/${outputs.length} 张封面…` : '正在保存编辑配置…'
    const version = await api.saveCover(props.id, config, png)
    versions.value = [version, ...versions.value]; saved.push(version)
    if (exportPng) message.value = `已导出 ${saved.length}/${outputs.length} 张封面：${saved.map(v => v.exportedPath).join('；')}`
  }
  selected.value = saved.find(v => v.config.width === source.width && v.config.height === source.height) ?? saved[0]!
  const baseline = JSON.parse(original.value) as CoverConfig[]
  for (const config of configs) baseline[coverSizeIndex(config)] = config
  original.value = JSON.stringify(baseline)
  await autosave.flush()
  message.value = exportPng ? `已导出 ${saved.length} 张封面，可在本书目录查看。` : `全部尺寸编辑配置已保存，共 ${saved.length} 个版本。`
})
async function showPreview() {
  coverView.value = 'preview'; await nextTick()
  const preview = globalThis.document.getElementById('cover-preview')
  preview?.scrollIntoView({ block: 'start' }); preview?.focus({ preventScroll: true })
}
async function showEditor() {
  coverView.value = 'edit'; await nextTick()
  globalThis.document.querySelector('.cover-controls')?.scrollIntoView({ block: 'start' })
  globalThis.document.querySelector<globalThis.HTMLButtonElement>('[aria-label="封面编辑"] [data-state="active"]')?.focus({ preventScroll: true })
}
function saveShortcut(event: globalThis.KeyboardEvent) {
  if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 's') {
    event.preventDefault()
    if (globalThis.document.querySelector('[role="dialog"], [role="alertdialog"]')) return
    if (ready.value && !busy.value && !deletingPreset.value) void save(false)
  }
}
globalThis.addEventListener('keydown', saveShortcut)
onBeforeUnmount(() => globalThis.removeEventListener('keydown', saveShortcut))
onMounted(async () => {
  await globalThis.document.fonts.ready
  fontsReady = true
  if (desktop && !disposed) {
    await load()
    if (ready.value && useAi.value && !disposed) void loadImageModels()
  }
})
</script>

<template>
  <div class="covers-page">
    <div class="page-return-row">
      <RouterLink
        class="back-link"
        :to="`/projects/${id}/writing`"
      >
        <AppIcon name="back" />返回内容创作 · {{ bookTitle || '书籍内容' }}
      </RouterLink>
      <Button
        variant="outline"
        as-child
      >
        <RouterLink :to="`/projects/${id}/publishing`">
          发布准备<AppIcon name="arrow" />
        </RouterLink>
      </Button>
    </div>
    <header class="page-heading">
      <div>
        <div class="cover-page-title">
          <h1>封面工坊</h1><Badge variant="outline">
            {{ !ready || previewError || backgroundError || backgroundLoading ? '排版待核对' : '当前尺寸排版就绪' }}
          </Badge>
        </div><p class="muted">
          让故事的第一眼，在全网信息流中拥有强烈的视觉抓力。
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
      请在桌面应用中制作封面。
    </p>
    <OperationFeedback
      :error="error"
      :message="message"
      @dismiss="message = ''"
    />
    <OperationFeedback :error="autosave.error.value" />
    <section
      v-if="linkedCover"
      class="mb-4 rounded-xl border bg-card p-4"
      aria-label="本次流程成品封面"
    >
      <p class="text-sm text-muted-foreground">
        本次流程已导出的封面 · 下方编辑草稿保留
      </p>
      <CoverThumbnail
        :cover="linkedCover"
        expandable
      />
    </section>
    <OperationFeedback
      v-else-if="ready && route.query.coverVersion"
      error="指定封面版本不存在，请核对原流程。"
    />
    <section
      v-if="generatedImage && generation"
      class="panel"
      aria-label="生成的封面背景"
    >
      <h2>背景已生成 · {{ generation.size?.width }} × {{ generation.size?.height }}</h2>
      <p class="muted">
        当前封面保持原样，采用后只修改本次请求的尺寸。生成图片已保存在本书素材中。
      </p>
      <img
        v-if="backgrounds[generatedImage.id]?.image"
        :src="backgrounds[generatedImage.id]!.image!.src"
        alt="生成的封面背景候选"
        class="my-3 max-h-48 max-w-full rounded-lg object-contain"
      >
      <div class="actions">
        <Button
          :disabled="busy"
          @click="applyGeneratedImage"
        >
          采用到该尺寸
        </Button>
        <Button
          variant="outline"
          :disabled="busy"
          @click="dismissGeneration(generation)"
        >
          保留当前封面
        </Button>
      </div>
    </section>
    <p
      v-if="generating"
      class="notice"
      role="status"
    >
      背景正在生成，可切换页面继续创作，请保持应用运行。完成后可在此采用背景。
    </p>
    <OperationFeedback
      :message="writingError"
      variant="warning"
    />
    <Button
      v-if="!ready && desktop"
      :disabled="busy"
      type="button"
      @click="load"
    >
      <AppIcon
        name="refresh"
        :class="{ spin: busy }"
      />{{ busy ? '读取中…' : '重新读取' }}
    </Button>
    <ConfirmAction
      v-if="deletingPreset && selectedPreset"
      :title="`删除个人样式「${selectedPreset.name}」？`"
      :busy="busy"
      @confirm="deletePreset"
      @cancel="deletingPreset = false"
    >
      <p>删除后无法恢复此个人样式，当前封面和已保存的封面历史保持原样。</p>
    </ConfirmAction>
    <template v-if="ready">
      <section
        class="panel cover-size-bar"
        aria-label="封面尺寸设置"
      >
        <div class="cover-size-description">
          <label
            for="cover-size"
          >当前编辑尺寸</label><Select
            :model-value="editor.activeSize"
            :disabled="busy"
            @update:model-value="changeSize"
          >
            <SelectTrigger
              id="cover-size"
              class="w-full min-w-0 sm:w-64"
            >
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem
                v-for="(size, index) in coverSizes"
                :key="index"
                :value="index"
              >
                {{ size.label }}
              </SelectItem>
            </SelectContent>
          </Select>

          <p class="muted">
            文字、排版、背景和画面描述只修改当前尺寸。需要共用时主动应用，之后仍可分别调整。
          </p>
        </div>
        <div class="actions wrap">
          <Button
            type="button"
            variant="outline"
            :disabled="busy"
            @click="shareCover(true)"
          >
            仅共用当前背景
          </Button>
          <Button
            type="button"
            variant="outline"
            :disabled="busy"
            @click="shareCover(false)"
          >
            将当前封面应用到全部尺寸
          </Button>
        </div>
      </section>
      <div class="cover-jump actions">
        <Button
          variant="outline"
          :aria-pressed="coverView === 'preview'"
          @click="showPreview"
        >
          查看实时预览
        </Button>
        <Button
          variant="ghost"
          :aria-pressed="coverView === 'edit'"
          @click="showEditor"
        >
          返回编辑
        </Button>
      </div>
      <p
        v-if="writingError"
        class="notice"
      >
        {{ writingError }}<Button
          :disabled="busy"
          variant="outline"
          type="button"
          @click="loadWriting"
        >
          重新读取
        </Button>
      </p>
      <div
        class="cover-workbench"
        :data-view="coverView"
      >
        <div class="cover-controls panel">
          <div class="studio-editor-heading">
            <h2><AppIcon name="sliders" /> 编辑封面</h2><Badge
              class="editor-save-status"
              :data-dirty="dirty"
              variant="secondary"
            >
              {{ dirty ? '未保存' : '已保存' }}
            </Badge>
          </div>
          <Tabs
            v-model="activeTab"
            :unmount-on-hide="false"
          >
            <TabsList
              class="w-full"
              aria-label="封面编辑"
            >
              <TabsTrigger
                v-for="tab in tabs"
                :key="tab.value"
                :value="tab.value"
              >
                {{ tab.label }}
              </TabsTrigger>
            </TabsList>
            <form
              id="cover-edit-form"
              @submit.prevent="save(false)"
            >
              <fieldset :disabled="busy">
                <TabsContent value="text">
                  <label for="cover-title">主标题 *</label><Textarea
                    id="cover-title"
                    v-model="draft.title"
                    rows="2"
                    maxlength="200"
                    required
                    placeholder="一句话，让人想了解这本书"
                  />
                  <label for="cover-subtitle">副标题</label><Textarea
                    id="cover-subtitle"
                    v-model="draft.subtitle"
                    rows="2"
                    maxlength="300"
                    placeholder="补充书名或这一期的看点"
                  />
                  <label for="cover-brand">品牌标识</label><Input
                    id="cover-brand"
                    v-model="draft.brand"
                    maxlength="60"
                    placeholder="你的名字或栏目名称"
                  />
                  <Collapsible
                    class="cover-suggestions"
                    :unmount-on-hide="false"
                  >
                    <CollapsibleTrigger class="disclosure-trigger">
                      <AppIcon name="sparkle" /> 使用本书生成的封面文案
                    </CollapsibleTrigger>
                    <CollapsibleContent>
                      <section aria-label="本书封面素材">
                        <template v-if="writing?.coverTitle && writing.coverSubtitle">
                          <p>{{ writing.coverTitle }}</p><p class="muted">
                            {{ writing.coverSubtitle }}
                          </p><Button
                            type="button"
                            :disabled="busy"
                            variant="outline"
                            @click="applyWritingTitles"
                          >
                            使用这组主副标题
                          </Button>
                        </template>
                        <p
                          v-else
                          class="muted"
                        >
                          还没有封面文案，可前往 <RouterLink :to="`/projects/${id}/writing`">
                            书籍内容
                          </RouterLink> 一键生成，也可以直接填写。
                        </p>
                      </section>
                    </CollapsibleContent>
                  </Collapsible>
                </TabsContent>
                <TabsContent value="style">
                  <section
                    v-if="writing?.coverDesign"
                    class="cover-recommendation"
                    aria-label="AI 推荐排版"
                  >
                    <h3>
                      <AppIcon name="sparkle" />AI 为本书推荐排版方案
                    </h3>
                    <CoverDesignPreview
                      :design="writing.coverDesign"
                      compact
                    >
                      <template #hint>
                        <span class="sr-only">推荐底色蒙层 {{ Math.round(writing.coverDesign.overlayOpacity * 100) }}%</span>
                      </template>
                    </CoverDesignPreview>
                    <Button
                      type="button"
                      variant="outline"
                      :disabled="busy"
                      @click="applyWritingDesign"
                    >
                      应用 AI 推荐排版
                    </Button>
                  </section>
                  <p
                    v-else
                    class="muted"
                  >
                    一键生成内容时会同时推荐字号与配色；也可直接选择下方样式。
                  </p>
                  <h3 class="preset-heading">
                    精选推荐样式
                  </h3>
                  <div
                    class="preset-grid"
                    aria-label="默认封面样式"
                  >
                    <Button
                      v-for="(template, key) in coverTemplates"
                      v-show="key !== 'ocean'"
                      :key="key"
                      type="button"
                      variant="ghost"
                      class="style-card"
                      :style="{ backgroundColor: template.style.backgroundColor, color: template.style.textColor }"
                      :aria-label="template.label"
                      :aria-pressed="isTemplateApplied(key)"
                      :disabled="busy"
                      @click="applyTemplate(key)"
                    >
                      <span
                        class="preset-accent"
                        :style="{ backgroundColor: template.style.accentColor }"
                      />
                      <strong>故事有引力</strong><span :style="{ color: template.style.subtitleColor }">一本书，一种新视角</span><b>{{ template.label }} · {{ template.description.split(' · ')[0] }}</b>
                    </Button>
                  </div>
                  <div class="typography-controls">
                    <div class="form-grid">
                      <div>
                        <label for="cover-font">字体</label><Select
                          v-model="draft.fontFamily"
                          :disabled="busy"
                        >
                          <SelectTrigger
                            id="cover-font"
                            class="w-full min-w-0"
                          >
                            <SelectValue placeholder="请选择" />
                          </SelectTrigger>
                          <SelectContent>
                            <SelectItem value="serif">
                              宋体 / 系统衬线
                            </SelectItem><SelectItem value="sans">
                              黑体 / 系统无衬线
                            </SelectItem>
                          </SelectContent>
                        </Select>
                      </div>
                      <div>
                        <label for="cover-align">对齐方式</label><Select
                          v-model="draft.align"
                          :disabled="busy"
                        >
                          <SelectTrigger
                            id="cover-align"
                            class="w-full min-w-0"
                          >
                            <SelectValue placeholder="请选择" />
                          </SelectTrigger>
                          <SelectContent>
                            <SelectItem value="left">
                              左对齐
                            </SelectItem><SelectItem value="center">
                              居中
                            </SelectItem>
                          </SelectContent>
                        </Select>
                      </div>
                      <div>
                        <label for="cover-title-size">标题字号</label><Input
                          id="cover-title-size"
                          v-model.number="draft.titleSize"
                          type="number"
                          min="40"
                          max="160"
                          required
                        />
                      </div>
                      <div>
                        <label for="cover-subtitle-size">副标题字号</label><Input
                          id="cover-subtitle-size"
                          v-model.number="draft.subtitleSize"
                          type="number"
                          min="24"
                          max="80"
                          required
                        />
                      </div>
                    </div>
                    <div class="cover-colors">
                      <div>
                        <label for="cover-text-color">主标题色</label><Input
                          id="cover-text-color"
                          v-model="draft.textColor"
                          type="color"
                          class="cursor-pointer p-1"
                        />
                      </div>
                      <div>
                        <label for="cover-subtitle-color">副标题色</label><Input
                          id="cover-subtitle-color"
                          v-model="draft.subtitleColor"
                          type="color"
                          class="cursor-pointer p-1"
                        />
                      </div>
                    </div>
                  </div>
                  <Collapsible
                    class="cover-suggestions"
                    :unmount-on-hide="false"
                  >
                    <CollapsibleTrigger class="disclosure-trigger">
                      更多排版设置与个人样式
                    </CollapsibleTrigger>
                    <CollapsibleContent>
                      <label for="cover-position">文字位置</label><Select
                        v-model="draft.position"
                        :disabled="busy"
                      >
                        <SelectTrigger
                          id="cover-position"
                          class="w-full min-w-0"
                        >
                          <SelectValue placeholder="请选择" />
                        </SelectTrigger>
                        <SelectContent>
                          <SelectItem value="top">
                            靠上
                          </SelectItem><SelectItem value="center">
                            居中
                          </SelectItem><SelectItem value="bottom">
                            靠下
                          </SelectItem>
                        </SelectContent>
                      </Select>
                      <div class="cover-colors">
                        <div>
                          <label for="cover-bg-color">底色</label><Input
                            id="cover-bg-color"
                            v-model="draft.backgroundColor"
                            type="color"
                            class="cursor-pointer p-1"
                          />
                        </div>
                        <div>
                          <label for="cover-accent-color">点缀色</label><Input
                            id="cover-accent-color"
                            v-model="draft.accentColor"
                            type="color"
                            class="cursor-pointer p-1"
                          />
                        </div>
                      </div>
                      <label for="cover-preset">我的封面样式</label>
                      <Select
                        :model-value="presetId"
                        :disabled="busy"
                        @update:model-value="applyPreset"
                      >
                        <SelectTrigger
                          id="cover-preset"
                          class="w-full min-w-0"
                        >
                          <SelectValue placeholder="选择个人样式" />
                        </SelectTrigger>
                        <SelectContent>
                          <SelectItem
                            v-for="preset in presets"
                            :key="preset.id"
                            :value="preset.id"
                          >
                            {{ preset.name }}
                          </SelectItem>
                        </SelectContent>
                      </Select>
                      <div class="mt-3 flex flex-wrap items-center gap-3">
                        <Input
                          v-model="presetName"
                          aria-label="新样式名称"
                          placeholder="给当前样式起个名字"
                          maxlength="60"
                          class="min-w-0 flex-1 basis-40"
                        />
                        <Button
                          type="button"
                          variant="outline"
                          :disabled="busy || !presetName.trim()"
                          @click="savePreset"
                        >
                          保存为我的样式
                        </Button>
                        <Button
                          v-if="presetId"
                          type="button"
                          variant="outline"
                          :disabled="busy"
                          @click="deletingPreset = true"
                        >
                          删除样式
                        </Button>
                      </div>
                      <p class="muted">
                        复用品牌文字、字体、字号、颜色和布局；每本书的标题、背景与裁切单独保留。
                      </p>
                      <Button
                        type="button"
                        variant="outline"
                        :disabled="busy"
                        @click="applyTemplate('ocean')"
                      >
                        蓝调纪实
                      </Button>
                    </CollapsibleContent>
                  </Collapsible>
                </TabsContent>
              </fieldset>
            </form>
            <TabsContent
              value="background"
              class="background-panel"
            >
              <div class="background-source">
                <label for="cover-background">背景来源</label>
                <Select
                  v-model="draft.backgroundAssetId"
                  :disabled="busy"
                >
                  <SelectTrigger
                    id="cover-background"
                    class="w-full"
                    :title="images.find(image => image.id === draft.backgroundAssetId)?.originalName"
                  >
                    <SelectValue placeholder="纯色背景" />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem :value="null">
                      纯色背景
                    </SelectItem>
                    <SelectItem
                      v-if="draft.backgroundAssetId && !images.some(image => image.id === draft.backgroundAssetId)"
                      :value="draft.backgroundAssetId"
                    >
                      背景关联已失效，请重新选择
                    </SelectItem>
                    <SelectItem
                      v-for="asset in images"
                      :key="asset.id"
                      :value="asset.id"
                    >
                      {{ asset.originalName }}{{ asset.existsStatus !== 'available' ? '（文件不可用）' : '' }}
                    </SelectItem>
                  </SelectContent>
                </Select>
              </div>
              <Button
                type="button"
                variant="outline"
                class="choose-image"
                :disabled="busy"
                title="支持 PNG、JPEG、WebP"
                @click="chooseImage"
              >
                <AppIcon name="download" />选择本地图片替换
              </Button>
              <div
                v-if="draft.backgroundAssetId"
                class="cover-mask"
              >
                <div class="studio-field-heading">
                  <label for="cover-overlay">底色蒙层遮罩</label><strong>{{ Math.round(draft.overlayOpacity * 100) }}%</strong>
                </div>
                <Slider
                  id="cover-overlay"
                  aria-label="底色蒙层"
                  :model-value="[draft.overlayOpacity]"
                  :min="0"
                  :max="0.85"
                  :step="0.05"
                  :disabled="busy"
                  @update:model-value="draft.overlayOpacity = $event![0]!"
                />
              </div>
              <div class="background-actions">
                <Button
                  type="button"
                  variant="outline"
                  :disabled="busy || backgroundLoading || !draft.backgroundAssetId"
                  @click="loadBackground(draft.backgroundAssetId)"
                >
                  重新读取背景
                </Button>
                <Button
                  type="button"
                  variant="outline"
                  class="ai-toggle"
                  :disabled="busy"
                  :aria-expanded="useAi"
                  @click="toggleAi"
                >
                  <AppIcon name="sparkle" />{{ useAi ? '收起 AI 背景' : '使用 AI 生成背景（可选）' }}
                </Button>
              </div>
              <section
                v-if="useAi"
                class="ai-background"
                aria-label="可选 AI 背景"
              >
                <div class="ai-heading">
                  <h3>
                    <i
                      class="status-dot animate-pulse"
                      aria-hidden="true"
                    />AI 神经生图终端
                  </h3>
                  <Button
                    type="button"
                    variant="outline"
                    class="ai-collapse"
                    aria-label="收起 AI 背景面板"
                    :disabled="busy"
                    @click="useAi = false"
                  >
                    <AppIcon name="sparkle" />收起 AI 背景
                  </Button>
                </div>
                <p
                  v-if="!imageModels.length"
                  class="notice"
                >
                  尚无已启用的图片模型。可继续使用本地模板，或前往 <RouterLink to="/settings/models">
                    模型设置
                  </RouterLink> 添加。
                </p>
                <form @submit.prevent="generateBackground">
                  <fieldset :disabled="busy || !imageModels.length">
                    <div class="ai-model-field">
                      <label for="cover-model">图片模型</label>
                      <Select
                        v-model="providerId"
                        required
                        :disabled="busy || !imageModels.length"
                      >
                        <SelectTrigger
                          id="cover-model"
                          class="w-full"
                        >
                          <SelectValue placeholder="请选择图片模型">
                            <i
                              v-if="selectedImageModel"
                              class="status-dot"
                            /><span>{{ selectedImageModel ? `${selectedImageModel.name} (${selectedImageModel.modelName})` : '请选择图片模型' }}</span>
                          </SelectValue>
                        </SelectTrigger>
                        <SelectContent>
                          <SelectItem
                            v-for="model in imageModels"
                            :key="model.id"
                            :value="model.id"
                          >
                            {{ model.name }} ({{ model.modelName }})
                          </SelectItem>
                        </SelectContent>
                      </Select>
                    </div>
                    <div class="ai-description-field">
                      <div class="studio-field-heading">
                        <label for="cover-description">画面描述</label><Button
                          type="button"
                          variant="ghost"
                          :disabled="!description.trim()"
                          @click="copyText(description, '画面描述')"
                        >
                          复制描述
                        </Button>
                      </div>
                      <Textarea
                        id="cover-description"
                        v-model="description"
                        rows="4"
                        maxlength="2000"
                        required
                        :placeholder="suggestedPrompt || '描述画面主体、场景、氛围和文字留白区域'"
                      />
                    </div>
                    <p class="ai-usage">
                      仅为 <strong>{{ currentSize.label }}</strong> 生成无文字背景，中文由本地排版。图片服务按横竖方向生成，最终按目标尺寸裁切。书名、作者和画面描述会发送至所选服务，可能产生模型费用。
                    </p>
                    <div class="ai-generate-actions">
                      <Button
                        type="submit"
                        class="generate-background"
                        :disabled="!description.trim() || !providerId"
                      >
                        <AppIcon name="bolt" />生成当前尺寸背景
                      </Button>
                    </div>
                  </fieldset>
                </form>
                <Button
                  type="button"
                  variant="outline"
                  class="refresh-models"
                  :disabled="busy"
                  @click="loadImageModels"
                >
                  <AppIcon name="refresh" />刷新图片模型
                </Button>
                <div
                  v-if="taskId"
                  class="notice"
                  role="status"
                >
                  正在生成背景，最多等待 180 秒…<Button
                    type="button"
                    :disabled="cancelling"
                    variant="outline"
                    @click="cancelImage"
                  >
                    {{ cancelling ? '正在取消…' : '取消生成' }}
                  </Button>
                </div>
              </section>
              <section
                class="cover-prompt-source"
                aria-label="本书的画面提示词"
              >
                <div class="studio-field-heading">
                  <strong>{{ currentSize.label.split(' · ')[0] }} · 专属画面提示词</strong><Button
                    type="button"
                    variant="ghost"
                    aria-label="复制封面提示词"
                    :disabled="!suggestedPrompt.trim()"
                    @click="copyText(suggestedPrompt, '封面提示词')"
                  >
                    复制
                  </Button>
                </div>
                <p
                  v-if="!suggestedPrompt"
                  class="muted"
                >
                  该尺寸暂无提示词，可在 <RouterLink :to="`/projects/${id}/writing?tab=cover`">
                    内容创作的按项调整
                  </RouterLink> 中单独生成，或自行填写画面描述。
                </p>
                <Textarea
                  id="cover-suggested-prompt"
                  aria-label="书籍专属画面提示词"
                  :model-value="suggestedPrompt"
                  rows="3"
                  readonly
                />
                <Button
                  type="button"
                  variant="secondary"
                  :disabled="busy || !suggestedPrompt.trim()"
                  @click="usePrompt"
                >
                  用于 AI 背景
                </Button>
              </section>
            </TabsContent>
          </Tabs>
        </div>
        <section
          id="cover-preview"
          tabindex="-1"
          class="cover-preview panel"
          aria-label="封面预览"
        >
          <div class="section-heading">
            <h2>
              <i
                class="status-dot"
                aria-hidden="true"
              /> 封面展示台
            </h2><span class="preview-dimensions">{{ draft.width }} × {{ draft.height }}</span>
          </div>
          <p
            v-if="backgroundLoading"
            class="muted"
            role="status"
          >
            正在读取背景图…
          </p><OperationFeedback :error="backgroundError || previewError" />
          <div class="canvas-stage">
            <canvas
              ref="canvas"
              :style="{ width: `${previewWidth}px`, aspectRatio: `${draft.width} / ${draft.height}` }"
              aria-label="当前封面画布"
              role="img"
              :class="{ 'cursor-grab touch-none': !!background && !busy && !inlineField }"
              @dblclick="editAtPointer"
              @pointerdown="startDrag"
              @pointermove="moveDrag"
              @pointerup="drag = null"
              @pointercancel="drag = null"
              @lostpointercapture="drag = null"
            />
            <Input
              v-if="inlineField"
              id="cover-inline-text"
              class="cover-inline-editor"
              :style="inlineStyle"
              :model-value="draft[inlineField]"
              :aria-label="inlineField === 'title' ? '画布主标题' : '画布副标题'"
              :maxlength="inlineField === 'title' ? 200 : 300"
              @update:model-value="draft[inlineField!] = String($event)"
              @keydown="inlineKey"
              @blur="endInline()"
            />
            <span
              v-if="!inlineField && !busy"
              class="canvas-hint"
              aria-hidden="true"
            >双击文字可直接在画布修改{{ background ? ' · 拖拽调整背景' : '' }}</span>
          </div>
          <div class="cover-inline-actions">
            <div>
              <Button
                variant="ghost"
                :disabled="busy"
                @click="editInline('title')"
              >
                在画布编辑主标题
              </Button><span>·</span><Button
                variant="ghost"
                :disabled="busy"
                @click="editInline('subtitle')"
              >
                在画布编辑副标题
              </Button>
            </div>
            <span>可双击文字 · Enter 完成 · Esc 撤销</span>
          </div>
          <div class="background-adjustments">
            <div class="studio-field-heading">
              <strong>拖动画布调整背景，也可用下方滑块精确微调</strong><Button
                type="button"
                variant="ghost"
                :disabled="busy || !background"
                @click="Object.assign(draft, { backgroundX: 0.5, backgroundY: 0.5, backgroundZoom: 1 })"
              >
                重置背景位置
              </Button>
            </div>
            <div class="adjustment-field">
              <div class="studio-field-heading">
                <label for="background-zoom">背景缩放</label><span>{{ draft.backgroundZoom.toFixed(2) }} 倍</span>
              </div>
              <Slider
                id="background-zoom"
                aria-label="背景缩放"
                :model-value="[draft.backgroundZoom]"
                :min="1"
                :max="3"
                :step="0.05"
                :disabled="busy || !background"
                @update:model-value="draft.backgroundZoom = $event?.[0] ?? 1"
              />
            </div>
            <div class="adjustment-field">
              <div class="studio-field-heading">
                <label for="background-x">背景水平位置</label><span>{{ backgroundOffset.x }} px</span>
              </div>
              <Slider
                id="background-x"
                aria-label="背景水平位置"
                :model-value="[draft.backgroundX]"
                :min="0"
                :max="1"
                :step="0.01"
                :disabled="busy || !background"
                @update:model-value="draft.backgroundX = $event?.[0] ?? 0.5"
              />
            </div>
            <div class="adjustment-field">
              <div class="studio-field-heading">
                <label for="background-y">背景垂直位置</label><span>{{ backgroundOffset.y }} px</span>
              </div>
              <Slider
                id="background-y"
                aria-label="背景垂直位置"
                :model-value="[draft.backgroundY]"
                :min="0"
                :max="1"
                :step="0.01"
                :disabled="busy || !background"
                @update:model-value="draft.backgroundY = $event?.[0] ?? 0.5"
              />
            </div>
          </div>
          <div class="cover-previews">
            <div class="studio-field-heading">
              <strong>{{ coverSizes.length }} 个独立封面 · 点击缩略图切换编辑</strong><span class="png-format"><i class="status-dot" />高清 PNG 格式</span>
            </div>
            <div
              class="cover-thumbnails"
              aria-label="三种比例并排预览"
            >
              <Button
                v-for="(size, index) in coverSizes"
                :key="size.label"
                type="button"
                variant="ghost"
                class="size-preview"
                :aria-label="`编辑${size.label}`"
                :aria-pressed="editor.activeSize === index"
                :disabled="busy"
                @click="changeSize(index)"
              >
                <span class="size-preview-stage">
                  <canvas
                    :ref="el => setPreviewCanvas(el, index)"
                    :style="{ aspectRatio: `${size.width}/${size.height}` }"
                    :aria-label="size.label"
                    role="img"
                  />
                  <span
                    v-if="editor.activeSize === index"
                    class="size-preview-selected"
                  ><AppIcon name="check" /><span class="sr-only">当前编辑</span></span>
                </span>
                <strong>{{ size.label.split(' · ')[0] }}</strong><span>{{ size.width }} × {{ size.height }}</span>
                <span
                  v-if="editor.covers[index]!.backgroundAssetId && !backgrounds[editor.covers[index]!.backgroundAssetId!]?.image"
                  class="text-destructive"
                >背景待读取或不可用</span>
              </Button>
            </div>
            <div
              class="export-formats"
              aria-label="批量导出尺寸"
            >
              <div>
                <label
                  v-for="(size, index) in coverSizes"
                  :key="size.label"
                  :for="`cover-export-${index}`"
                ><Checkbox
                  :id="`cover-export-${index}`"
                  :model-value="exportIndexes.includes(index)"
                  :disabled="busy"
                  @update:model-value="selectExport(index, $event === true)"
                />{{ size.label.split(' · ')[0] }}</label>
              </div>
              <span>批量导出选中</span>
            </div>
          </div>
        </section>
      </div>
      <Collapsible
        v-model:open="historyOpen"
        class="history-panel"
        :unmount-on-hide="false"
      >
        <CollapsibleTrigger class="disclosure-trigger history-trigger">
          <AppIcon name="clock" /><span>封面历史 · {{ versions.length }} 个版本</span><span class="history-expand">展开历史</span>
        </CollapsibleTrigger>
        <div
          v-if="selected"
          class="history-current"
        >
          <div>
            <strong>v{{ selected.versionNo }} · {{ selected.config.width }} × {{ selected.config.height }} · {{ selected.exportedPath ? '已导出' : '编辑配置' }} · {{ new Date(selected.createdAt).toLocaleString() }}</strong><p
              v-if="selected.exportedPath"
              class="file-path"
              :title="displayPath(selected.exportedPath)"
            >
              {{ displayPath(selected.exportedPath) }}
            </p>
          </div>
          <Button
            type="button"
            variant="outline"
            :disabled="busy"
            @click="switchVersion(selected.id)"
          >
            恢复此配置
          </Button>
        </div>
        <CollapsibleContent>
          <div
            v-if="historyOpen && recentExports.length"
            class="cover-history-gallery"
            aria-label="最近导出的封面"
          >
            <p>最近导出 · 点击封面放大查看，恢复编辑请使用下方版本选择。</p>
            <div class="cover-history-grid">
              <CoverThumbnail
                v-for="version in recentExports"
                :key="version.id"
                :cover="version"
                expandable
              />
            </div>
          </div>
          <label for="cover-history">恢复编辑配置</label><Select
            :model-value="selected?.id ?? null"
            :disabled="busy"
            @update:model-value="switchVersion"
          >
            <SelectTrigger
              id="cover-history"
              class="w-full min-w-0"
            >
              <SelectValue placeholder="尚未保存" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem
                :value="null"
                disabled
              >
                尚未保存
              </SelectItem><SelectItem
                v-for="version in versions"
                :key="version.id"
                :value="version.id"
              >
                v{{ version.versionNo }} · {{ version.config.width }} × {{ version.config.height }} · {{ version.exportedPath ? '已导出' : '编辑配置' }} · {{ new Date(version.createdAt).toLocaleString() }}
              </SelectItem>
            </SelectContent>
          </Select>
          <Collapsible
            class="preview-details"
            :unmount-on-hide="false"
          >
            <CollapsibleTrigger class="disclosure-trigger">
              排版与导出信息
            </CollapsibleTrigger>
            <CollapsibleContent>
              <p class="muted">
                文字自动换行，过长时缩小字号。当前标题 {{ actualSizes.titleSize }} px，副标题 {{ actualSizes.subtitleSize }} px。每次保存或导出都会新建版本。
              </p><p class="file-path">
                导出目录：{{ displayPath(workspace) }} / covers
              </p><Button
                :disabled="busy"
                variant="outline"
                type="button"
                @click="run(() => openProjectDirectory(id))"
              >
                <AppIcon name="folder" />打开项目目录
              </Button>
            </CollapsibleContent>
          </Collapsible>
        </CollapsibleContent>
      </Collapsible>
      <div class="sticky-actions">
        <span
          class="save-status"
          :class="{ 'is-dirty': dirty }"
        ><i
          class="status-dot animate-pulse"
          aria-hidden="true"
        />{{ operation || (dirty ? autosave.status.value || '有未保存修改' : '编辑配置已保存') }}</span>
        <div class="actions wrap">
          <Button
            type="submit"
            form="cover-edit-form"
            :disabled="busy"
            variant="outline"
          >
            <AppIcon name="save" />保存全部配置
          </Button><Button
            type="button"
            :disabled="busy || backgroundLoading || !!previewError || !!backgroundError"
            variant="outline"
            @click="save(true, true)"
          >
            导出当前封面
          </Button><Button
            type="button"
            :disabled="busy || !exportReady"
            @click="save(true)"
          >
            <AppIcon name="download" />导出 {{ exportIndexes.length }} 张封面
          </Button>
        </div>
      </div>
    </template>
  </div>
</template>
<style scoped>
.covers-page { color: var(--foreground); }
.covers-page .panel { box-shadow: var(--panel-shadow); }
.covers-page .page-return-row { margin: 0 0 24px; }
.covers-page .page-return-row > :deep(a[data-slot='button']) { padding: 8px 16px; color: var(--cover-accent); border-color: color-mix(in srgb, var(--input-border) 80%, transparent); }
.covers-page .page-heading { margin: 0 0 24px; }
.cover-page-title { display: flex; align-items: center; flex-wrap: wrap; gap: 12px; }
.cover-page-title :deep([data-slot='badge']) { padding: 2px 10px; border: 1px solid color-mix(in srgb, var(--cover-accent) 30%, transparent); border-radius: 999px; background: color-mix(in srgb, var(--cover-accent) 15%, transparent); color: var(--cover-accent); font: 400 12px/16px var(--font-mono); letter-spacing: -.75px; }
.covers-page .page-heading > div { width: 100%; }
.covers-page :deep(.project-navigation) { margin: 0 0 24px; }
.covers-page :deep(.project-navigation .is-current) { color: var(--cover-accent); border-color: color-mix(in srgb, var(--cover-accent) 40%, transparent); background: color-mix(in srgb, var(--cover-accent) 15%, transparent); box-shadow: 0 0 15px #f59e0b40; }
.covers-page :deep(.project-navigation .is-current .step-state) { background: var(--cover-accent); }
.covers-page :deep([data-slot='button']) { font-family: var(--font-mono); }
.covers-page :deep([data-slot='select-trigger']) { height: 38px; padding: 10px 14px; font: 400 12px/16px var(--font-mono); border-radius: 12px; }
.covers-page :deep([data-slot='select-value']) { min-width: 0; overflow: hidden; text-overflow: ellipsis; }
.covers-page :deep([data-slot='input']) { height: 38px; padding: 10px 16px; font: 400 12px/16px var(--font-mono); border-radius: 12px; }
.covers-page label { margin: 0 0 6px; font: 400 12px/16px var(--font-mono); }
.cover-size-bar { display: flex; align-items: center; justify-content: space-between; gap: 16px; padding: 20px; margin: 0 0 24px; }
.cover-size-description { min-width: 0; }
.cover-size-description :deep([data-slot='select-trigger']) { color: var(--cover-accent); font-weight: 700; }
.cover-size-description :deep(.app-icon) { color: var(--cover-accent); }
.cover-size-description .muted { margin: 6px 0 0; font: 400 11px/16.5px var(--font-mono); }
.cover-size-bar .actions { flex-shrink: 0; gap: 12px; margin: 0; }
.cover-size-bar .actions button { padding: 10px 16px; }
.cover-size-bar .actions button:last-child { border-color: color-mix(in srgb, var(--cover-accent) 50%, transparent); color: var(--cover-accent); }
.cover-workbench { display: grid; grid-template-columns: repeat(12, minmax(0, 1fr)); gap: 24px; align-items: start; }
.cover-controls { grid-column: span 5; padding: 20px; margin: 0; }
.cover-controls :deep([data-slot='tabs']) { gap: 0; }
.cover-preview { grid-column: span 7; padding: 24px; margin: 0; position: relative; overflow: visible; scroll-margin-top: var(--workspace-sticky-top); }
.studio-editor-heading { display: flex; align-items: center; justify-content: space-between; padding-bottom: 12px; border-bottom: 1px solid var(--border); margin-bottom: 20px; }
.studio-editor-heading h2 { display: flex; align-items: center; gap: 8px; margin: 0; color: var(--heading); font: 700 12px/16px var(--font-sans); letter-spacing: .6px; }
.studio-editor-heading .app-icon { color: var(--cover-accent); }
.editor-save-status { font: 400 10px/15px var(--font-mono); color: var(--primary); background: color-mix(in srgb, var(--primary) 15%, transparent); border: 1px solid color-mix(in srgb, var(--primary) 30%, transparent); padding: 2px 8px; border-radius: 4px; }
.editor-save-status[data-dirty='true'] { color: var(--cover-accent); background: color-mix(in srgb, var(--cover-accent) 15%, transparent); border-color: color-mix(in srgb, var(--cover-accent) 40%, transparent); }
.cover-controls :deep([data-slot='tabs-list']) { display: flex; width: 100%; height: 44px; min-height: 44px; padding: 4px; margin: 0 0 20px; border: 1px solid color-mix(in srgb, var(--secondary) 80%, transparent); border-radius: 12px; gap: 0; background: color-mix(in srgb, var(--terminal) 80%, transparent); backdrop-filter: none; }
.cover-controls :deep([data-slot='tabs-trigger']) { height: 34px; min-height: 34px; padding: 8px 0; border: 1px solid transparent; border-radius: 8px; font: 500 12px/16px var(--font-mono); }
.cover-controls :deep([data-slot='tabs-trigger'][data-state='active']) { color: var(--cover-accent); background: var(--secondary); border-color: color-mix(in srgb, var(--cover-accent) 40%, transparent); box-shadow: 0 1px 2px #0000000d; font-weight: 700; }
.cover-controls :deep([data-slot='tabs-content'][data-state='active']) { display: block; }
.cover-controls :deep(textarea) { min-height: 0; padding: 12px; font: 400 12px/16px var(--font-sans); border-radius: 12px; }
.cover-controls :deep(#cover-title) { color: var(--heading); font-size: 14px; line-height: 20px; font-weight: 700; }
.cover-controls :deep([data-slot='tabs-content'][data-state='active']:not(.background-panel)) > label { margin-top: 16px; }
.cover-controls :deep([data-slot='tabs-content'][data-state='active']:not(.background-panel)) > label:first-child { margin-top: 0; }
.cover-controls .muted { font: 400 11px/18px var(--font-mono); }
.cover-controls .form-grid, .cover-colors { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 16px 12px; }
.cover-controls .form-grid > *, .cover-colors > * { min-width: 0; }
.cover-controls .form-grid label, .cover-colors label { margin-top: 16px; }
.background-panel > * + * { margin-top: 16px; }
.background-panel .choose-image { display: flex; width: 100%; padding: 10px 0; color: var(--text-secondary); }
.background-panel .choose-image .app-icon { color: var(--cover-accent); }
.cover-mask { padding: 14px; border: 1px solid var(--border); border-radius: 12px; background: color-mix(in srgb, var(--terminal) 70%, transparent); }
.studio-field-heading { display: flex; align-items: center; justify-content: space-between; gap: 8px; font: 400 12px/16px var(--font-mono); color: var(--muted-foreground); }
.studio-field-heading label, .studio-field-heading strong { margin: 0; font: inherit; }
.studio-field-heading strong { font-weight: 700; }
.studio-field-heading button { padding: 0; border: 0; height: auto; font: inherit; background: transparent; color: inherit; }
.cover-mask .studio-field-heading strong { color: var(--cover-accent); }
.covers-page :deep([data-slot='slider']) { height: 16px; margin-top: 8px; }
.covers-page :deep([data-slot='slider-track']) { height: 6px; background: var(--input-border); }
.covers-page :deep([data-slot='slider-range']) { background: var(--cover-accent); }
.covers-page :deep([data-slot='slider-thumb']) { width: 16px; height: 16px; border: 0; background: var(--cover-accent); box-shadow: none; }
.background-actions { display: flex; gap: 8px; }
.background-actions > button { min-width: 0; flex: 1; padding: 8px 0; color: var(--text-secondary); }
.background-actions > .ai-toggle { gap: 6px; font-weight: 700; border-color: color-mix(in srgb, var(--cover-accent) 60%, transparent); color: var(--cover-accent); background: var(--input-surface); box-shadow: 0 0 12px #f59e0b40; }
.background-actions .ai-toggle[aria-expanded='false'] { background: linear-gradient(90deg,#f59e0b,#fb923c); color: var(--on-bright); }
.ai-background { position: relative; padding: 16px; border: 1px solid color-mix(in srgb, var(--cover-accent) 40%, transparent); border-radius: 12px; background: color-mix(in srgb, var(--terminal) 90%, transparent); box-shadow: var(--panel-shadow); }
.ai-heading { display: flex; justify-content: space-between; align-items: center; gap: 8px; }
.ai-heading h3 { display: flex; align-items: center; gap: 8px; margin: 0; font: 700 12px/16px var(--font-mono); color: var(--cover-accent); letter-spacing: .6px; }
.ai-heading .status-dot, .ai-model-field .status-dot { width: 8px; height: 8px; background: var(--cover-accent); }
.ai-heading > .ai-collapse { gap: 6px; padding: 4px 10px; font-weight: 600; border-radius: 8px; color: var(--cover-accent); background: color-mix(in srgb, var(--cover-accent) 10%, transparent); border-color: color-mix(in srgb, var(--cover-accent) 30%, transparent); box-shadow: 0 1px 2px #0000000d; }
.ai-heading .app-icon { width: 12px; height: 12px; color: var(--cover-accent); }
.ai-background form { margin-top: 14px; }
.ai-background fieldset > * + * { margin-top: 14px; }
.ai-background label { font: 400 11px/16.5px var(--font-mono); margin: 0 0 6px; }
.ai-model-field :deep([data-slot='select-trigger']) { height: 34px; padding: 8px 12px; }
.ai-model-field :deep([data-slot='select-trigger'] > .app-icon) { color: var(--cover-accent); }
.ai-model-field :deep([data-slot='select-value']) { font-weight: 500; }
.ai-description-field { padding-bottom: 6px; }
.ai-description-field .studio-field-heading { height: 16.5px; margin-bottom: 6px; }
.ai-description-field label { margin: 0; color: var(--text-secondary); }
.ai-description-field button { font: 400 10px/15px var(--font-mono); }
.ai-description-field :deep(textarea) { display: block; padding: 12px; font: 400 12px/19.5px var(--font-mono); border-color: color-mix(in srgb, var(--input-border) 80%, transparent); }
.ai-usage { font: 400 10px/16.25px var(--font-mono); color: var(--muted-foreground); margin: 0; }
.ai-usage strong { color: var(--cover-accent); }
.ai-generate-actions { padding-top: 4px; }
.ai-generate-actions > .generate-background { display: flex; width: 100%; padding: 10px 0; gap: 8px; font-weight: 700; background: var(--action-gradient); color: var(--primary-foreground); box-shadow: var(--primary-glow); }
.ai-background > .refresh-models { display: flex; width: 100%; margin-top: 8px; padding: 8px 0; gap: 6px; color: var(--text-secondary); }
.refresh-models .app-icon { width: 14px; height: 14px; color: var(--cover-accent); }
.cover-prompt-source { padding: 14px; border: 1px solid var(--border); border-radius: 12px; background: var(--terminal); font: 400 12px/16px var(--font-mono); }
.cover-prompt-source .studio-field-heading strong { color: var(--cover-accent); }
.cover-prompt-source .studio-field-heading button { color: var(--muted-foreground); }
.cover-prompt-source :deep(textarea) { display: block; margin-top: 8px; padding: 10px; border-color: var(--border); border-radius: 8px; color: var(--text-secondary); font: 400 11px/17.875px var(--font-mono); }
.cover-prompt-source > button { margin-top: 12px; padding: 6px 12px; border-radius: 8px; color: var(--cover-accent); font: 400 11px/16px var(--font-mono); }
.cover-preview > .section-heading { margin: 0 0 20px; }
.cover-preview h2 { display: flex; align-items: center; gap: 8px; font: 700 12px/16px var(--font-mono); }
.cover-preview h2 .status-dot { width: 8px; height: 8px; background: var(--primary); }
.preview-dimensions { font: 400 12px/16px var(--font-mono); color: var(--muted-foreground); }
.canvas-stage { position: relative; display: flex; align-items: center; justify-content: center; padding: 28px 20px; background: radial-gradient(ellipse at 50% 0%, color-mix(in srgb, var(--cover-accent) 10%, transparent), transparent 70%), var(--terminal); border: 1px solid color-mix(in srgb, var(--cover-accent) 20%, var(--border)); border-radius: 16px; overflow: hidden; }
.canvas-stage > canvas { display: block; max-width: 100%; height: auto; border-radius: 4px; box-shadow: 0 22px 40px -16px #00000080, 0 0 0 1px #ffffff12; }
.canvas-hint { position: absolute; left: 50%; bottom: 16px; transform: translateX(-50%); width: max-content; max-width: calc(100% - 24px); padding: 4px 12px; border: 1px solid color-mix(in srgb, var(--cover-accent) 30%, transparent); border-radius: 999px; background: #000000bf; color: #fcd34d; font: 400 10px/15px var(--font-mono); text-align: center; pointer-events: none; opacity: 0; transition: opacity 150ms; }
.canvas-stage:hover .canvas-hint, .canvas-stage:focus-within .canvas-hint { opacity: 1; }
.cover-inline-editor { position: absolute; z-index: 2; min-height: 38px; background: var(--card); color: var(--foreground); }
.cover-inline-actions { display: flex; justify-content: space-between; align-items: center; gap: 12px; margin-top: 20px; padding-top: 4px; color: var(--muted-foreground); font: 400 12px/16px var(--font-mono); }
.cover-inline-actions > div { display: flex; align-items: center; gap: 12px; }
.cover-inline-actions button { border: 0; background: transparent; padding: 0; color: inherit; font: inherit; height: auto; }
.cover-inline-actions button:hover { color: var(--cover-accent); }
.cover-inline-actions > span { font-size: 11px; }
.background-adjustments { margin-top: 20px; padding: 16px; border: 1px solid var(--border); border-radius: 12px; background: color-mix(in srgb, var(--input-surface) 60%, transparent); font: 400 12px/16px var(--font-mono); }
.background-adjustments > .studio-field-heading { color: var(--text-secondary); }
.background-adjustments > .studio-field-heading strong { font-weight: 600; }
.background-adjustments > .studio-field-heading button { color: var(--cover-accent); font: 400 12px/16px var(--font-mono); }
.adjustment-field { margin-top: 12px; }
.adjustment-field .studio-field-heading { font-size: 11px; }
.adjustment-field label { margin: 0; font: inherit; }
.adjustment-field :deep([data-slot='slider']) { height: 12px; margin-top: 4px; }
.adjustment-field :deep([data-slot='slider-track']) { height: 4px; }
.cover-previews { margin-top: 20px; padding-top: 8px; border-top: 1px solid var(--border); }
.cover-previews > .studio-field-heading { color: var(--text-secondary); }
.png-format { display: inline-flex; align-items: center; gap: 4px; color: var(--primary); font-weight: 700; white-space: nowrap; }
.png-format .status-dot { width: 6px; height: 6px; }
.cover-thumbnails { display: grid; grid-template-columns: repeat(3,minmax(0,1fr)); gap: 12px; margin-top: 12px; }
.cover-thumbnails > .size-preview { display: flex; flex-direction: column; align-items: stretch; justify-content: flex-start; gap: 0; width: 100%; min-width: 0; height: auto; padding: 8px; border: 1px solid var(--border); border-radius: 12px; background: var(--input-surface); color: var(--muted-foreground); text-align: center; white-space: normal; transition: transform 200ms, border-color 200ms, box-shadow 200ms, background-color 200ms; }
.size-preview[aria-pressed='true'] { border-color: var(--cover-accent); color: var(--cover-accent); box-shadow: inset 0 0 0 1px var(--cover-accent), var(--cover-glow); background: color-mix(in srgb, var(--cover-accent) 6%, var(--card)); }
.size-preview-stage { position: relative; display: flex; align-items: center; justify-content: center; height: clamp(88px, 12vw, 148px); padding: 10px; background: var(--terminal); border-radius: 6px; }
.size-preview canvas { display: block; max-width: 100%; max-height: 100%; width: auto; height: auto; object-fit: contain; border-radius: 2px; box-shadow: 0 8px 12px -5px #00000066; }
.size-preview-selected { position: absolute; right: 4px; bottom: 4px; display: grid; place-items: center; width: 20px; height: 20px; border-radius: 50%; color: var(--card); background: var(--cover-accent); }
.size-preview-selected .app-icon { width: 13px; height: 13px; }
.size-preview > strong { display: block; margin-top: 10px; font: 700 12px/18px var(--font-mono); }
.size-preview > span:not(.size-preview-stage) { display: block; font: 400 11px/16px var(--font-mono); color: var(--muted-foreground); }
@media (hover: hover) and (prefers-reduced-motion: no-preference) { .size-preview:hover:not(:disabled), .size-preview[aria-pressed='true'] { transform: translateY(-3px); } }
.cover-history-gallery { margin: 16px 0; }
.cover-history-gallery > p { font-size: 12px; color: var(--muted-foreground); }
.cover-history-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(180px, 1fr)); gap: 12px; }
.cover-history-grid :deep(figure) { min-width: 0; margin: 0; }
.export-formats { display: flex; align-items: center; justify-content: space-between; gap: 12px; margin-top: 12px; padding-top: 8px; font: 400 12px/16px var(--font-mono); }
.export-formats > div { display: flex; flex-wrap: wrap; gap: 16px; }
.export-formats label { display: flex; align-items: center; gap: 6px; margin: 0; color: var(--text-secondary); font: inherit; cursor: pointer; }
.export-formats > span { color: var(--cover-accent); font-size: 11px; white-space: nowrap; }
.export-formats :deep([data-slot='checkbox'][data-state='checked']) { background: var(--cover-accent); border-color: var(--cover-accent); color: var(--primary-foreground); }
.cover-suggestions, .preview-details { margin-top: 16px; padding: 12px; border: 1px solid var(--border); border-radius: 12px; background: color-mix(in srgb, var(--terminal) 60%, transparent); color: var(--muted-foreground); font-size: 12px; }
.cover-recommendation { padding: 16px; border: 1px solid color-mix(in srgb, var(--cover-accent) 30%, transparent); border-radius: 12px; background: var(--terminal); }
.cover-recommendation h3 { display: flex; align-items: center; gap: 6px; margin: 0 0 12px; color: var(--cover-accent); font: 700 12px/16px var(--font-mono); }
.cover-recommendation > button { width: 100%; margin-top: 12px; padding: 8px 0; border-color: color-mix(in srgb, var(--cover-accent) 40%, transparent); color: var(--cover-accent); background: color-mix(in srgb, var(--cover-accent) 15%, transparent); font-weight: 700; }
.preset-heading { margin: 16px 0 8px; color: var(--text-secondary); font: 600 12px/16px var(--font-mono); }
.preset-grid { display: grid; grid-template-columns: repeat(2,minmax(0,1fr)); gap: 10px; }
.cover-controls .style-card { display: block; height: auto; min-width: 0; padding: 12px; border: 1px solid transparent; border-radius: 12px; text-align: left; white-space: normal; font-family: var(--font-mono); transition: border-color 150ms; }
.style-card .preset-accent { display: block; width: 20px; height: 2px; margin-bottom: 6px; }
.style-card strong { display: block; font-size: 12px; line-height: 16px; }
.style-card > span:not(.preset-accent) { display: block; font-size: 9px; line-height: 13.5px; }
.style-card b { display: block; margin-top: 8px; font-size: 10px; line-height: 18.5px; }
.style-card:hover, .style-card[aria-pressed='true'] { border-color: var(--cover-accent); }
.typography-controls { margin-top: 16px; padding: 16px; border: 1px solid var(--border); border-radius: 12px; background: color-mix(in srgb, var(--input-surface) 80%, transparent); font: 400 12px/16px var(--font-mono); }
.typography-controls .form-grid { gap: 12px; }
.typography-controls .form-grid label, .typography-controls .cover-colors label { margin: 0 0 4px; }
.typography-controls .cover-colors { margin-top: 12px; padding-top: 4px; border-top: 1px solid var(--border); }
.covers-page .typography-controls :deep([data-slot='input']), .covers-page .typography-controls :deep([data-slot='select-trigger']) { height: 30px; padding: 6px 10px; border-radius: 8px; font: inherit; }
.covers-page .typography-controls :deep(input[type='number']) { text-align: center; }
.covers-page .typography-controls :deep(input[type='color']) { height: 34px; padding: 2px; cursor: pointer; }
.covers-page > .history-panel { margin-top: 24px; padding: 16px; border: 1px solid var(--border); border-radius: 16px; background: var(--panel-surface); }
.history-trigger { display: flex; width: 100%; align-items: center; gap: 8px; font: 400 12px/16px var(--font-mono); }
.history-trigger .app-icon { color: var(--cover-accent); }
.history-trigger::after { margin-left: 0; color: var(--cover-accent); }
.history-expand { margin-left: auto; color: var(--cover-accent); font-size: 11px; }
.history-current { display: flex; justify-content: space-between; align-items: center; gap: 12px; margin-top: 12px; padding: 12px; border: 1px solid var(--border); border-radius: 12px; background: color-mix(in srgb, var(--terminal) 70%, transparent); font: 400 11px/16px var(--font-mono); }
.history-current > div { min-width: 0; }
.history-current strong { color: var(--editor-foreground); line-height: 17px; }
.history-current .file-path { margin: 2px 0 0; padding: 0; border-radius: 0; background: transparent; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: var(--muted-foreground); font-size: 10px; }
.history-current > button { flex-shrink: 0; height: auto; padding: 6px 12px; border-radius: 8px; color: var(--cover-accent); font: inherit; }
.covers-page > .sticky-actions { margin-top: 24px; border-color: color-mix(in srgb, var(--cover-accent) 40%, transparent); bottom: 16px; box-shadow: var(--panel-shadow); background: var(--panel-surface); }
.sticky-actions .save-status { color: var(--muted-foreground); }
.sticky-actions .status-dot { width: 8px; height: 8px; background: var(--cover-accent); }
.sticky-actions .actions { gap: 12px; }
.sticky-actions .actions button:nth-child(2) { color: var(--cover-accent); border-color: color-mix(in srgb, var(--cover-accent) 50%, transparent); }
.sticky-actions .actions > button:last-child { font-weight: 700; background: linear-gradient(90deg,#fbbf24,#fb923c,#f59e0b); color: var(--on-bright); box-shadow: var(--cover-glow); }
.cover-jump { display: none; }
@media (prefers-reduced-transparency: reduce) { .covers-page .panel, .covers-page .sticky-actions { backdrop-filter: none; background: var(--card); } }
@media (max-width: 1279px) { .cover-size-bar { flex-wrap: wrap; } .cover-size-description { flex-basis: 100%; } .cover-inline-actions { flex-wrap: wrap; } }
@media (max-width: 1023px) { .cover-workbench { grid-template-columns: minmax(0,1fr); } .cover-controls, .cover-preview { grid-column: auto; } .cover-jump { display: flex; margin: 0 0 16px; position: sticky; top: var(--workspace-sticky-top); z-index: 6; padding: 8px 12px; border: 1px solid var(--border); border-radius: 12px; background: var(--card); } .cover-workbench[data-view='edit'] .cover-preview, .cover-workbench[data-view='preview'] .cover-controls { display: none; } .cover-jump [aria-pressed='true'] { color: var(--cover-accent); background: color-mix(in srgb, var(--cover-accent) 15%, transparent); } .cover-controls, .cover-preview { scroll-margin-top: calc(var(--workspace-sticky-top) + 64px); } }
@media (max-width: 600px) { .cover-size-bar, .cover-controls, .cover-preview { padding: 16px; } .cover-size-bar .actions { flex-shrink: 1; flex-wrap: wrap; } .cover-size-bar .actions button { white-space: normal; } .studio-field-heading { flex-wrap: wrap; } .ai-heading { flex-wrap: wrap; } .export-formats { flex-wrap: wrap; } .cover-page-title { gap: 8px; } .cover-previews .studio-field-heading > strong { font-size: 11px; } .cover-inline-actions { font-size: 11px; } }
</style>
