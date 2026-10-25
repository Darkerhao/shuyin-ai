<script setup lang="ts">
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '@/components/ui/collapsible'
import { Slider } from '@/components/ui/slider'
import { Input } from '@/components/ui/input'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { Textarea } from '@/components/ui/textarea'
import { computed, nextTick, onBeforeUnmount, onMounted, ref, shallowRef, watch, type ComponentPublicInstance } from 'vue'
import { RouterLink, onBeforeRouteLeave, onBeforeRouteUpdate, useRoute } from 'vue-router'
import * as api from '../../services/covers'
import { listProviders, listVersions, cancelGeneration } from '../../services/ai'
import { emptyPublishing, type ModelProvider, type PublishingContent } from '../../types/ai'
import { getProject, isDesktop, linkAsset, listAssets, openProjectDirectory } from '../../services/projects'
import { backgroundRect, coverPng, decodeBackground, renderCover } from '../../services/coverRenderer'
import { coverSizes, coverTemplates, newCover, type CoverConfig, type CoverVersion, type CoverPreset } from '../../types/cover'
import type { Asset } from '../../types/project'
import { useOperation } from '../../composables/useOperation'
import { useEditorDraft } from '../../composables/useEditorDraft'
import { readDraft } from '../../services/drafts'
import AppIcon from '../../components/AppIcon.vue'
import ProjectNavigation from '../../components/ProjectNavigation.vue'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import OperationFeedback from '../../components/OperationFeedback.vue'

const props = defineProps<{ id: string }>()
const route = useRoute()
const desktop = isDesktop()
const ready = ref(false)
const bookTitle = ref('')
const workspace = ref('')
const draft = ref(newCover())
const original = ref(JSON.stringify(draft.value))
const dirty = computed(() => original.value !== JSON.stringify(draft.value))
const versions = ref<CoverVersion[]>([])
const selected = ref<CoverVersion | null>(null)
const images = ref<Asset[]>([])
const canvas = ref<globalThis.HTMLCanvasElement | null>(null)
const previewCanvases: (globalThis.HTMLCanvasElement | null)[] = []
const presets = ref<CoverPreset[]>([])
const presetId = ref<string | null>(null)
const presetName = ref('')
const background = shallowRef<globalThis.HTMLImageElement | null>(null)
const backgroundLoading = ref(false)
const backgroundError = ref('')
const previewError = ref('')
const actualSizes = ref({ titleSize: 112, subtitleSize: 44 })
const { busy, error, message, run } = useOperation()
const useAi = ref(false)
const activeTab = ref('text')
const tabs = [{ value: 'text', label: '文字内容' }, { value: 'style', label: '排版样式' }, { value: 'background', label: '背景图片' }]
const imageModels = ref<ModelProvider[]>([])
const providerId = ref('')
const description = ref('')
const writing = ref<PublishingContent | null>(null)
const writingError = ref('')
const promptRatio = ref<'landscape' | 'portrait'>('landscape')
const suggestedPrompt = computed(() => (promptRatio.value === 'landscape' ? writing.value?.coverPromptLandscape : writing.value?.coverPromptPortrait) ?? '')
const taskId = ref('')
const cancelling = ref(false)
let loadedBackgroundId: string | null = null
let backgroundRequest = 0
let disposed = false
let fontsReady = false
const canDiscard = () => !dirty.value || globalThis.confirm('封面尚未保存，确定放弃修改？')
const autosave = useEditorDraft(() => props.id, 'cover', draft, () => selected.value?.id ?? null, dirty, ready, () => !busy.value)
const canLeave = async () => {
  if (busy.value) return false
  try { if (ready.value) await autosave.flush(); return true } catch { return false }
}
onBeforeRouteLeave(canLeave)
onBeforeRouteUpdate(canLeave)
function beforeUnload(event: globalThis.BeforeUnloadEvent) { if (dirty.value || busy.value) event.preventDefault() }
globalThis.addEventListener('beforeunload', beforeUnload)
onBeforeUnmount(() => { disposed = true; backgroundRequest++; globalThis.removeEventListener('beforeunload', beforeUnload) })

function accept(version: CoverVersion | null) {
  selected.value = version
  const config = { ...newCover(bookTitle.value), ...version?.config }
  const size = coverSizes.find(s => s.width === config.width && s.height === config.height) ?? coverSizes[config.width > config.height ? 0 : 1]
  if (size.width !== config.width || size.height !== config.height) message.value = `历史封面为 ${config.width} × ${config.height}，当前按${size.label}继续编辑，原版本和文件保留。`
  draft.value = { ...config, width: size.width, height: size.height }
  original.value = JSON.stringify(draft.value)
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
  const [project, saved, assets, recovery, styles] = await Promise.all([getProject(props.id), api.listCovers(props.id), listAssets(props.id), readDraft<CoverConfig>(props.id, 'cover'), api.listPresets(), loadWriting()])
  bookTitle.value = project.title; workspace.value = project.workspacePath
  versions.value = saved; images.value = assets.filter(a => a.assetType === 'image')
  presets.value = styles
  accept(recovery ? saved.find(v => v.id === recovery.basedOn) ?? null : saved[0] ?? null)
  if (recovery) { draft.value = { ...newCover(), ...recovery.content }; message.value = '已恢复自动暂存的封面草稿，尚未新增历史版本。' }
  else if (!saved.length && writing.value?.coverTitle && writing.value.coverSubtitle) applyWritingTitles()
  ready.value = true
  await nextTick(); paint()
})
function applyWritingTitles() {
  if (!writing.value) return
  draft.value.title = writing.value.coverTitle
  draft.value.subtitle = writing.value.coverSubtitle
  message.value = '已带入封面主副标题，确认预览后保存或导出。'
}
async function copyPrompt() {
  try { await globalThis.navigator.clipboard.writeText(suggestedPrompt.value); message.value = '已复制封面提示词。'; error.value = '' }
  catch { error.value = '复制失败，请选中文本后手动复制。' }
}
function usePrompt() {
  description.value = suggestedPrompt.value
  activeTab.value = 'background'
  useAi.value = true
  void loadImageModels()
}
async function loadBackground(id: string | null) {
  const request = ++backgroundRequest
  background.value = null; loadedBackgroundId = null; backgroundError.value = ''; backgroundLoading.value = !!id
  try {
    if (id) {
      const image = await decodeBackground(await api.readBackground(props.id, id))
      if (request === backgroundRequest) { background.value = image; loadedBackgroundId = id }
    }
  } catch (cause) {
    if (request === backgroundRequest) backgroundError.value = cause instanceof Error ? cause.message : String(cause)
  } finally {
    if (request === backgroundRequest) { backgroundLoading.value = false; paint() }
  }
}
function paint() {
  if (!canvas.value || !fontsReady || disposed) return
  previewError.value = ''
  try {
    actualSizes.value = renderCover(canvas.value, draft.value, background.value)
    coverSizes.forEach((size, index) => { const target = previewCanvases[index]; if (target) renderCover(target, { ...draft.value, width: size.width, height: size.height }, background.value) })
  }
  catch (cause) { previewError.value = cause instanceof Error ? cause.message : String(cause) }
}
function setPreviewCanvas(el: globalThis.Element | ComponentPublicInstance | null, index: number) {
  previewCanvases[index] = el instanceof globalThis.HTMLCanvasElement ? el : null
}
let drag: { x: number; y: number; positionX: number; positionY: number; overflowX: number; overflowY: number } | null = null
function startDrag(event: globalThis.PointerEvent) {
  if (busy.value || !background.value || !canvas.value || event.button !== 0) return
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
  Object.assign(draft.value, preset.style); presetId.value = id
  message.value = `已应用「${preset.name}」，可继续为本书调整。`
}
const savePreset = () => run(async () => {
  await api.savePreset(presetName.value.trim(), { ...draft.value })
  presets.value = await api.listPresets(); presetName.value = ''; message.value = '已保存到我的封面样式。'
})
const deletePreset = () => run(async () => {
  if (!presetId.value) return
  await api.deletePreset(presetId.value)
  presets.value = presets.value.filter(p => p.id !== presetId.value); presetId.value = null; message.value = '已删除个人样式，当前封面保持原样。'
})
watch(() => draft.value.backgroundAssetId, id => { if (!id || id !== loadedBackgroundId) void loadBackground(id) })
watch(draft, paint, { deep: true, flush: 'post' })

function applyTemplate(key: CoverConfig['templateId']) {
  Object.assign(draft.value, coverTemplates[key].style, { templateId: key })
}
function changeSize(index: number) {
  const size = coverSizes[index]!
  draft.value.width = size.width; draft.value.height = size.height
}
function switchVersion(id: string | null) {
  if (canDiscard()) accept(versions.value.find(v => v.id === id) ?? null)
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
  try {
    taskId.value = await api.startCoverGeneration(props.id, providerId.value, description.value)
    const asset = await api.executeCoverGeneration(taskId.value)
    if (disposed) return
    images.value = [asset, ...images.value]
    const image = await decodeBackground(await api.readBackground(props.id, asset.id))
    if (disposed) return
    // Apply only after reading and decoding succeeds; preserve all text and style edits.
    backgroundRequest++; loadedBackgroundId = asset.id
    background.value = image; backgroundLoading.value = false; backgroundError.value = ''
    draft.value.backgroundAssetId = asset.id
    message.value = 'AI 背景已生成，文字和样式已保留。确认预览后可导出全部比例封面。'
  } finally { taskId.value = '' }
})
async function cancelImage() {
  if (cancelling.value) return
  cancelling.value = true
  try { await cancelGeneration(taskId.value) }
  catch { error.value = '取消失败，请稍后重试。' }
  finally { cancelling.value = false }
}
const save = (exportPng = false) => run(async () => {
  await autosave.flush()
  const source = { ...draft.value }
  const configs = exportPng ? coverSizes.map(size => ({ ...source, width: size.width, height: size.height })) : [source]
  // Render all ratios before writing any file so layout failures keep the whole draft.
  const outputs = await Promise.all(configs.map(async config => ({ config, png: exportPng ? await coverPng(config, background.value) : null })))
  const saved: CoverVersion[] = []
  for (const { config, png } of outputs) {
    const version = await api.saveCover(props.id, config, png)
    versions.value = [version, ...versions.value]; saved.push(version)
    if (exportPng) message.value = `已导出 ${saved.length}/${outputs.length} 张封面：${saved.map(v => v.exportedPath).join('；')}`
  }
  accept(saved.find(v => v.config.width === source.width && v.config.height === source.height) ?? saved[0]!)
  await autosave.flush()
  message.value = exportPng ? `已导出 ${saved.length} 张封面：${saved.map((v, i) => `${coverSizes[i]!.label}：${v.exportedPath}`).join('；')}` : `编辑配置已保存为 v${saved[0]!.versionNo}。`
})
onMounted(async () => {
  await globalThis.document.fonts.ready
  fontsReady = true
  if (desktop && !disposed) void load()
})
</script>

<template>
  <header class="page-heading">
    <div>
      <RouterLink
        class="back-link"
        :to="`/projects/${id}/writing`"
      >
        <AppIcon name="back" />{{ bookTitle || '书籍内容' }}
      </RouterLink><h1>封面工坊</h1><p class="muted">
        让故事的第一眼，就有吸引力。
      </p>
    </div>
    <Button
      variant="outline"
      as-child
    >
      <RouterLink
        :to="`/projects/${id}/publishing`"
      >
        发布准备<AppIcon name="arrow" />
      </RouterLink>
    </Button>
  </header>
  <ProjectNavigation :id="id" />
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
  <p
    v-if="autosave.error.value"
    class="error"
    role="alert"
  >
    {{ autosave.error.value }}
  </p>
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
  <template v-if="ready">
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
    <div class="cover-workbench">
      <div class="cover-controls panel">
        <div class="section-heading">
          <h2><AppIcon name="settings" /> 编辑封面</h2><Badge
            :class="{ 'bg-amber-50 text-amber-800': dirty }"
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
                  rows="3"
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
                <div class="mt-3 flex flex-wrap gap-2">
                  <Input
                    v-model="presetName"
                    aria-label="新样式名称"
                    placeholder="给当前样式起个名字"
                    maxlength="60"
                    class="min-w-0 flex-1"
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
                    @click="deletePreset"
                  >
                    删除样式
                  </Button>
                </div>
                <p class="muted">
                  复用品牌文字、字体、字号、颜色和布局；每本书的标题、背景与裁切单独保留。
                </p>
                <label for="cover-template">封面模板</label><Select
                  :model-value="draft.templateId"
                  :disabled="busy"
                  @update:model-value="applyTemplate"
                >
                  <SelectTrigger
                    id="cover-template"
                    class="w-full min-w-0"
                  >
                    <SelectValue placeholder="请选择" />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem
                      v-for="(template, key) in coverTemplates"
                      :key="key"
                      :value="key"
                    >
                      {{ template.label }}
                    </SelectItem>
                  </SelectContent>
                </Select>
                <p class="muted">
                  切换模板会应用配色、字体与布局，文字和背景图片保持不变。
                </p>
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
                    <label for="cover-text-color">文字色</label><Input
                      id="cover-text-color"
                      v-model="draft.textColor"
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
              </TabsContent>
            </fieldset>
          </form>
          <TabsContent value="background">
            <fieldset :disabled="busy">
              <div
                class="local-background"
              >
                <label for="cover-background">背景来源</label><Select
                  v-model="draft.backgroundAssetId"
                  :disabled="busy"
                >
                  <SelectTrigger
                    id="cover-background"
                    class="w-full min-w-0"
                  >
                    <SelectValue placeholder="纯色背景" />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem :value="null">
                      纯色背景
                    </SelectItem>
                    <SelectItem
                      v-if="draft.backgroundAssetId && !images.some(a => a.id === draft.backgroundAssetId)"
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
                <Button
                  type="button"
                  variant="outline"
                  class="choose-image"
                  @click="chooseImage"
                >
                  <AppIcon name="upload" />选择本地图片
                </Button>
                <p class="muted">
                  支持 PNG、JPEG、WebP，可在预览画布拖动和缩放背景。
                </p>
                <template v-if="draft.backgroundAssetId">
                  <label for="cover-overlay">底色蒙层 <span class="optional">· {{ Math.round(draft.overlayOpacity * 100) }}%</span></label><Slider
                    id="cover-overlay"
                    aria-label="底色蒙层"
                    :model-value="[draft.overlayOpacity]"
                    :min="0"
                    :max="0.85"
                    :step="0.05"
                    :disabled="busy"
                    class="my-4"
                    @update:model-value="draft.overlayOpacity = $event![0]!"
                  />
                  <Button
                    type="button"
                    :disabled="backgroundLoading"
                    variant="outline"
                    @click="loadBackground(draft.backgroundAssetId)"
                  >
                    重新读取背景
                  </Button>
                </template>
              </div>
            </fieldset>
            <section
              class="ai-background"
              aria-label="可选 AI 背景"
            >
              <Button
                type="button"
                :disabled="busy"
                :aria-expanded="useAi"
                variant="outline"
                class="ai-toggle"
                @click="toggleAi"
              >
                <AppIcon name="sparkle" />{{ useAi ? '收起 AI 背景' : '使用 AI 生成背景（可选）' }}
              </Button>
              <template v-if="useAi">
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
                    <label for="cover-model">图片模型</label><Select
                      v-model="providerId"
                      required
                      :disabled="busy || !imageModels.length"
                    >
                      <SelectTrigger
                        id="cover-model"
                        class="w-full min-w-0"
                      >
                        <SelectValue placeholder="请选择图片模型" />
                      </SelectTrigger>
                      <SelectContent>
                        <SelectItem
                          v-for="model in imageModels"
                          :key="model.id"
                          :value="model.id"
                        >
                          {{ model.name }}
                        </SelectItem>
                      </SelectContent>
                    </Select>
                    <label for="cover-description">画面描述</label><Textarea
                      id="cover-description"
                      v-model="description"
                      rows="5"
                      maxlength="2000"
                      required
                    />
                    <p class="muted">
                      AI 生成无字背景，中文由本地排版。书名、作者和画面描述会发送至所选服务，可能产生费用。
                    </p>
                    <Button
                      type="submit"
                      :disabled="!description.trim() || !providerId"
                    >
                      <AppIcon name="sparkle" />生成 AI 背景
                    </Button>
                  </fieldset>
                </form>
                <Button
                  type="button"
                  :disabled="busy"
                  variant="ghost"
                  class="refresh-models"
                  @click="loadImageModels"
                >
                  <AppIcon name="refresh" />刷新图片模型
                </Button>
              </template>
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
            <Collapsible
              v-if="writing?.coverPromptLandscape || writing?.coverPromptPortrait"
              class="cover-suggestions"
              :unmount-on-hide="false"
            >
              <CollapsibleTrigger class="disclosure-trigger">
                本书的画面提示词
              </CollapsibleTrigger>
              <CollapsibleContent>
                <label for="cover-prompt-ratio">封面提示词比例</label><Select
                  v-model="promptRatio"
                >
                  <SelectTrigger
                    id="cover-prompt-ratio"
                    class="w-full min-w-0"
                  >
                    <SelectValue placeholder="请选择" />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="landscape">
                      横版 4:3 · 1440 × 1080
                    </SelectItem><SelectItem value="portrait">
                      竖版 3:4 · 1080 × 1440
                    </SelectItem>
                  </SelectContent>
                </Select>
                <label for="cover-suggested-prompt">书籍专属画面提示词</label><Textarea
                  id="cover-suggested-prompt"
                  :model-value="suggestedPrompt"
                  rows="5"
                  readonly
                />
                <div class="actions wrap">
                  <Button
                    type="button"
                    :disabled="!suggestedPrompt.trim()"
                    variant="outline"
                    @click="copyPrompt"
                  >
                    复制封面提示词
                  </Button><Button
                    type="button"
                    :disabled="busy || !suggestedPrompt.trim()"
                    variant="outline"
                    @click="usePrompt"
                  >
                    用于 AI 背景
                  </Button>
                </div>
                <p class="muted">
                  外部工具请选择对应比例；内置 AI 生成一张背景，再裁切排版导出 {{ coverSizes.length }} 种比例。切换提示词不会替换你的画面描述。
                </p>
              </CollapsibleContent>
            </Collapsible>
          </TabsContent>
        </Tabs>
      </div>
      <section
        class="cover-preview panel"
        aria-label="封面预览"
      >
        <div class="section-heading">
          <h2><i class="status-dot" /> 实时预览</h2><span class="preview-dimensions">{{ draft.width }} × {{ draft.height }}</span>
        </div>
        <label
          class="sr-only"
          for="cover-size"
        >预览比例</label><Select
          :model-value="coverSizes.findIndex(s => s.width === draft.width && s.height === draft.height)"
          :disabled="busy"
          @update:model-value="changeSize"
        >
          <SelectTrigger
            id="cover-size"
            class="mb-4 w-full min-w-0 sm:w-64"
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
        <p
          v-if="backgroundLoading"
          class="muted"
          role="status"
        >
          正在读取背景图…
        </p><p
          v-if="backgroundError || previewError"
          class="error"
          role="alert"
        >
          {{ backgroundError || previewError }}
        </p>
        <div class="canvas-stage">
          <canvas
            ref="canvas"
            :style="{ aspectRatio: `${draft.width} / ${draft.height}` }"
            aria-label="当前封面画布"
            role="img"
            :class="{ 'cursor-grab touch-none': !!background && !busy }"
            @pointerdown="startDrag"
            @pointermove="moveDrag"
            @pointerup="drag = null"
            @pointercancel="drag = null"
            @lostpointercapture="drag = null"
          />
        </div>
        <div
          v-if="draft.backgroundAssetId"
          class="mt-4 space-y-3"
        >
          <p class="muted">
            拖动画布调整背景，也可用下方滑块精确定位。
          </p>
          <label for="background-zoom">背景缩放 · {{ draft.backgroundZoom.toFixed(2) }} 倍</label>
          <Slider
            id="background-zoom"
            aria-label="背景缩放"
            :model-value="[draft.backgroundZoom]"
            :min="1"
            :max="3"
            :step="0.05"
            :disabled="busy"
            @update:model-value="draft.backgroundZoom = $event?.[0] ?? 1"
          />
          <label for="background-x">背景水平位置</label>
          <Slider
            id="background-x"
            aria-label="背景水平位置"
            :model-value="[draft.backgroundX]"
            :min="0"
            :max="1"
            :step="0.01"
            :disabled="busy"
            @update:model-value="draft.backgroundX = $event?.[0] ?? 0.5"
          />
          <label for="background-y">背景垂直位置</label>
          <Slider
            id="background-y"
            aria-label="背景垂直位置"
            :model-value="[draft.backgroundY]"
            :min="0"
            :max="1"
            :step="0.01"
            :disabled="busy"
            @update:model-value="draft.backgroundY = $event?.[0] ?? 0.5"
          />
          <Button
            type="button"
            variant="outline"
            :disabled="busy"
            @click="Object.assign(draft, { backgroundX: 0.5, backgroundY: 0.5, backgroundZoom: 1 })"
          >
            重置背景位置
          </Button>
        </div>
        <div
          class="mt-5 grid grid-cols-3 items-start gap-3"
          aria-label="三种比例并排预览"
        >
          <figure
            v-for="(size, index) in coverSizes"
            :key="size.label"
            class="min-w-0 space-y-2"
          >
            <canvas
              :ref="el => setPreviewCanvas(el, index)"
              class="w-full"
              :style="{ aspectRatio: `${size.width}/${size.height}` }"
              :aria-label="size.label"
              role="img"
            />
            <figcaption class="text-center text-xs text-muted-foreground">
              {{ size.width }} × {{ size.height }}
            </figcaption>
          </figure>
        </div>
        <p class="preview-caption">
          同一份内容，{{ coverSizes.length }} 种精心排版的封面
        </p>
        <div class="export-formats">
          <span><i class="ratio-icon landscape" />横版 4:3</span><span><i class="ratio-icon portrait" />竖版 3:4</span><span><i class="ratio-icon widescreen" />横版 16:9</span><Badge variant="secondary">
            高清 PNG
          </Badge>
        </div>
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
              导出目录：{{ workspace }} / covers
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
      </section>
    </div>
    <div class="sticky-actions">
      <span
        class="save-status"
        :class="{ 'is-dirty': dirty }"
      ><i class="status-dot" />{{ dirty ? autosave.status.value || '有未保存修改' : '编辑配置已保存' }}</span>
      <div class="actions wrap">
        <Button
          type="submit"
          form="cover-edit-form"
          :disabled="busy"
          variant="outline"
        >
          <AppIcon name="save" />保存编辑配置
        </Button><Button
          type="button"
          :disabled="busy || backgroundLoading || !!previewError || !!backgroundError"
          @click="save(true)"
        >
          <AppIcon name="download" />导出 {{ coverSizes.length }} 张封面
        </Button>
      </div>
    </div>
    <Collapsible
      class="history-panel"
      :unmount-on-hide="false"
    >
      <CollapsibleTrigger class="disclosure-trigger">
        封面历史 · {{ versions.length }} 个版本
      </CollapsibleTrigger>
      <CollapsibleContent>
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
        </Select><p
          v-if="selected?.exportedPath"
          class="file-path"
        >
          {{ selected.exportedPath }}
        </p>
      </CollapsibleContent>
    </Collapsible>
  </template>
</template>
<style scoped>
.cover-workbench { display: grid; grid-template-columns: minmax(310px, .85fr) minmax(0, 1.15fr); gap: 23px; align-items: start; }
.cover-controls, .cover-preview { padding: 22px; margin-bottom: 0; }
.cover-controls label { margin-top: 16px; }
.cover-controls .muted { font-size: 13px; }
.cover-controls .form-grid { gap: 0 12px; }
.cover-colors { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 12px; }
.cover-preview { position: sticky; top: 20px; }
.cover-preview h2 { font-size: 13px; }
.preview-dimensions { font-size: 10px; color: #8b9980; }
.canvas-stage { display: flex; align-items: center; justify-content: center; height: clamp(280px, 43vh, 470px); padding: 24px; background-color: #eef0e9; background-image: radial-gradient(#cbd4c0 0.8px, transparent 0.8px); background-size: 12px 12px; border: 1px solid #e4e8db; border-radius: 9px; overflow: hidden; }
canvas { display: block; max-width: 100%; max-height: 100%; width: auto; height: auto; box-shadow: 0 6px 18px #22352224; }
.preview-caption { text-align: center; color: var(--muted-foreground); font-size: 12px; margin: 17px 0 11px; }
.export-formats { display: flex; flex-wrap: wrap; align-items: center; justify-content: center; gap: 17px; }
.export-formats > span { display: inline-flex; align-items: center; gap: 6px; color: #829274; font-size: 10px; }
.ratio-icon { width: 17px; height: 13px; border: 1px solid #99ae8a; border-radius: 2px; }
.ratio-icon.portrait { width: 12px; height: 16px; }
.ratio-icon.widescreen { width: 21px; height: 12px; }
.cover-suggestions, .preview-details { margin-top: 21px; padding-top: 15px; border-top: 1px solid var(--line); color: var(--muted-foreground); font-size: 12px; }
.ai-background { border-top: 1px solid var(--line); padding-top: 17px; margin-top: 17px; }
.ai-toggle, .choose-image { width: 100%; margin-top: 11px; }
.refresh-models { margin-top: 8px; }
@media (max-width: 1180px) { .cover-workbench { gap: 18px; } .cover-controls, .cover-preview { padding: 18px; } }
@media (max-width: 980px) { .cover-workbench { grid-template-columns: 1fr; gap: 14px; } .cover-preview { position: static; } }
@media (max-width: 850px) { .cover-controls, .cover-preview { padding: 15px; } .canvas-stage { padding: 15px; height: 300px; } .export-formats { gap: 9px; } }
@media (max-width: 720px) { .cover-workbench { grid-template-columns: 1fr; } .cover-preview { position: static; } .canvas-stage { height: 320px; } }
</style>
