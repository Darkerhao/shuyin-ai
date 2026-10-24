<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, shallowRef, watch } from 'vue'
import { RouterLink, onBeforeRouteLeave, onBeforeRouteUpdate, useRoute } from 'vue-router'
import * as api from '../../services/covers'
import { listProviders, listVersions, cancelGeneration } from '../../services/ai'
import { emptyPublishing, type ModelProvider, type PublishingContent } from '../../types/ai'
import { getProject, isDesktop, linkAsset, listAssets, openProjectDirectory } from '../../services/projects'
import { coverPng, decodeBackground, renderCover } from '../../services/coverRenderer'
import { coverSizes, coverTemplates, newCover, type CoverConfig, type CoverVersion } from '../../types/cover'
import type { Asset } from '../../types/project'
import { useOperation } from '../../composables/useOperation'
import AppIcon from '../../components/AppIcon.vue'
import ProjectNavigation from '../../components/ProjectNavigation.vue'
import EditorTabs from '../../components/EditorTabs.vue'
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
const canLeave = () => !busy.value && canDiscard()
onBeforeRouteLeave(canLeave)
onBeforeRouteUpdate(canLeave)
function beforeUnload(event: globalThis.BeforeUnloadEvent) { if (dirty.value || busy.value) event.preventDefault() }
globalThis.addEventListener('beforeunload', beforeUnload)
onBeforeUnmount(() => { disposed = true; backgroundRequest++; globalThis.removeEventListener('beforeunload', beforeUnload) })

function accept(version: CoverVersion | null) {
  selected.value = version
  const config = version ? { ...version.config } : newCover(bookTitle.value)
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
  const [project, saved, assets] = await Promise.all([getProject(props.id), api.listCovers(props.id), listAssets(props.id), loadWriting()])
  bookTitle.value = project.title; workspace.value = project.workspacePath
  versions.value = saved; images.value = assets.filter(a => a.assetType === 'image')
  accept(saved[0] ?? null)
  if (!saved.length && writing.value?.coverTitle && writing.value.coverSubtitle) applyWritingTitles()
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
  try { actualSizes.value = renderCover(canvas.value, draft.value, background.value) }
  catch (cause) { previewError.value = cause instanceof Error ? cause.message : String(cause) }
}
watch(() => draft.value.backgroundAssetId, id => { if (!id || id !== loadedBackgroundId) void loadBackground(id) })
watch(draft, paint, { deep: true, flush: 'post' })

function applyTemplate(event: globalThis.Event) {
  const key = (event.target as globalThis.HTMLSelectElement).value as CoverConfig['templateId']
  Object.assign(draft.value, coverTemplates[key].style, { templateId: key })
}
function changeSize(event: globalThis.Event) {
  const size = coverSizes[Number((event.target as globalThis.HTMLSelectElement).value)]!
  draft.value.width = size.width; draft.value.height = size.height
}
function switchVersion(event: globalThis.Event) {
  const select = event.target as globalThis.HTMLSelectElement
  if (canDiscard()) accept(versions.value.find(v => v.id === select.value) ?? null)
  else select.value = selected.value?.id ?? ''
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
    message.value = 'AI 背景已生成，文字和样式已保留。确认预览后可导出两张封面。'
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
  const source = { ...draft.value }
  const configs = exportPng ? coverSizes.map(size => ({ ...source, width: size.width, height: size.height })) : [source]
  // Render both ratios before writing either file so layout failures keep the whole draft.
  const outputs = await Promise.all(configs.map(async config => ({ config, png: exportPng ? await coverPng(config, background.value) : null })))
  const saved: CoverVersion[] = []
  for (const { config, png } of outputs) {
    const version = await api.saveCover(props.id, config, png)
    versions.value = [version, ...versions.value]; saved.push(version)
    if (exportPng) message.value = `已导出 ${saved.length}/2 张发布封面：${saved.map(v => v.exportedPath).join('；')}`
  }
  accept(saved.find(v => v.config.width === source.width && v.config.height === source.height) ?? saved[0]!)
  message.value = exportPng ? `已导出两张发布封面：${saved.map((v, i) => `${coverSizes[i]!.label}：${v.exportedPath}`).join('；')}` : `编辑配置已保存为 v${saved[0]!.versionNo}。`
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
    <RouterLink
      class="button secondary"
      :to="`/projects/${id}/publishing`"
    >
      发布准备<AppIcon name="arrow" />
    </RouterLink>
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
  <button
    v-if="!ready && desktop"
    :disabled="busy"
    @click="load"
  >
    <AppIcon
      name="refresh"
      :class="{ spin: busy }"
    />{{ busy ? '读取中…' : '重新读取' }}
  </button>
  <template v-if="ready">
    <p
      v-if="writingError"
      class="notice"
    >
      {{ writingError }}<button
        class="secondary"
        :disabled="busy"
        @click="loadWriting"
      >
        重新读取
      </button>
    </p>
    <div class="cover-workbench">
      <div class="cover-controls panel">
        <div class="section-heading">
          <h2><AppIcon name="settings" /> 编辑封面</h2><span
            class="badge"
            :class="{ warm: dirty }"
          >{{ dirty ? '未保存' : '已保存' }}</span>
        </div>
        <EditorTabs
          id="cover"
          v-model="activeTab"
          label="封面编辑"
          :tabs="tabs"
        />
        <form
          id="cover-edit-form"
          @submit.prevent="save(false)"
        >
          <fieldset :disabled="busy">
            <div
              id="cover-panel-text"
              :hidden="activeTab !== 'text'"
              role="tabpanel"
              aria-labelledby="cover-tab-text"
              tabindex="0"
            >
              <label for="cover-title">主标题 *</label><textarea
                id="cover-title"
                v-model="draft.title"
                rows="3"
                maxlength="200"
                required
                placeholder="一句话，让人想了解这本书"
              />
              <label for="cover-subtitle">副标题</label><textarea
                id="cover-subtitle"
                v-model="draft.subtitle"
                rows="2"
                maxlength="300"
                placeholder="补充书名或这一期的看点"
              />
              <label for="cover-brand">品牌标识</label><input
                id="cover-brand"
                v-model="draft.brand"
                maxlength="60"
                placeholder="你的名字或栏目名称"
              >
              <details class="cover-suggestions">
                <summary><AppIcon name="sparkle" /> 使用本书生成的封面文案</summary>
                <section aria-label="本书封面素材">
                  <template v-if="writing?.coverTitle && writing.coverSubtitle">
                    <p>{{ writing.coverTitle }}</p><p class="muted">
                      {{ writing.coverSubtitle }}
                    </p><button
                      type="button"
                      class="secondary"
                      :disabled="busy"
                      @click="applyWritingTitles"
                    >
                      使用这组主副标题
                    </button>
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
              </details>
            </div>
            <div
              id="cover-panel-style"
              :hidden="activeTab !== 'style'"
              role="tabpanel"
              aria-labelledby="cover-tab-style"
              tabindex="0"
            >
              <label for="cover-template">封面模板</label><select
                id="cover-template"
                :value="draft.templateId"
                @change="applyTemplate"
              >
                <option
                  v-for="(template, key) in coverTemplates"
                  :key="key"
                  :value="key"
                >
                  {{ template.label }}
                </option>
              </select>
              <p class="muted">
                切换模板会应用配色、字体与布局，文字和背景图片保持不变。
              </p>
              <div class="form-grid">
                <div>
                  <label for="cover-font">字体</label><select
                    id="cover-font"
                    v-model="draft.fontFamily"
                  >
                    <option value="serif">
                      宋体 / 系统衬线
                    </option><option value="sans">
                      黑体 / 系统无衬线
                    </option>
                  </select>
                </div>
                <div>
                  <label for="cover-align">对齐方式</label><select
                    id="cover-align"
                    v-model="draft.align"
                  >
                    <option value="left">
                      左对齐
                    </option><option value="center">
                      居中
                    </option>
                  </select>
                </div>
                <div>
                  <label for="cover-title-size">标题字号</label><input
                    id="cover-title-size"
                    v-model.number="draft.titleSize"
                    type="number"
                    min="40"
                    max="160"
                    required
                  >
                </div>
                <div>
                  <label for="cover-subtitle-size">副标题字号</label><input
                    id="cover-subtitle-size"
                    v-model.number="draft.subtitleSize"
                    type="number"
                    min="24"
                    max="80"
                    required
                  >
                </div>
              </div>
              <label for="cover-position">文字位置</label><select
                id="cover-position"
                v-model="draft.position"
              >
                <option value="top">
                  靠上
                </option><option value="center">
                  居中
                </option><option value="bottom">
                  靠下
                </option>
              </select>
              <div class="cover-colors">
                <div>
                  <label for="cover-bg-color">底色</label><input
                    id="cover-bg-color"
                    v-model="draft.backgroundColor"
                    type="color"
                  >
                </div>
                <div>
                  <label for="cover-text-color">文字色</label><input
                    id="cover-text-color"
                    v-model="draft.textColor"
                    type="color"
                  >
                </div>
                <div>
                  <label for="cover-accent-color">点缀色</label><input
                    id="cover-accent-color"
                    v-model="draft.accentColor"
                    type="color"
                  >
                </div>
              </div>
            </div>
            <div
              v-show="activeTab === 'background'"
              class="local-background"
            >
              <label for="cover-background">背景来源</label><select
                id="cover-background"
                v-model="draft.backgroundAssetId"
              >
                <option :value="null">
                  纯色背景
                </option>
                <option
                  v-if="draft.backgroundAssetId && !images.some(a => a.id === draft.backgroundAssetId)"
                  :value="draft.backgroundAssetId"
                >
                  背景关联已失效，请重新选择
                </option>
                <option
                  v-for="asset in images"
                  :key="asset.id"
                  :value="asset.id"
                >
                  {{ asset.originalName }}{{ asset.existsStatus !== 'available' ? '（文件不可用）' : '' }}
                </option>
              </select>
              <button
                type="button"
                class="secondary choose-image"
                @click="chooseImage"
              >
                <AppIcon name="upload" />选择本地图片
              </button>
              <p class="muted">
                支持 PNG、JPEG、WebP，图片将居中裁切。
              </p>
              <template v-if="draft.backgroundAssetId">
                <label for="cover-overlay">底色蒙层 <span class="optional">· {{ Math.round(draft.overlayOpacity * 100) }}%</span></label><input
                  id="cover-overlay"
                  v-model.number="draft.overlayOpacity"
                  type="range"
                  min="0"
                  max="0.85"
                  step="0.05"
                >
                <button
                  type="button"
                  class="secondary"
                  :disabled="backgroundLoading"
                  @click="loadBackground(draft.backgroundAssetId)"
                >
                  重新读取背景
                </button>
              </template>
            </div>
          </fieldset>
        </form>
        <div
          id="cover-panel-background"
          :hidden="activeTab !== 'background'"
          role="tabpanel"
          aria-labelledby="cover-tab-background"
          tabindex="0"
        >
          <section
            class="ai-background"
            aria-label="可选 AI 背景"
          >
            <button
              type="button"
              class="secondary ai-toggle"
              :disabled="busy"
              :aria-expanded="useAi"
              @click="toggleAi"
            >
              <AppIcon name="sparkle" />{{ useAi ? '收起 AI 背景' : '使用 AI 生成背景（可选）' }}
            </button>
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
                  <label for="cover-model">图片模型</label><select
                    id="cover-model"
                    v-model="providerId"
                    required
                  >
                    <option
                      v-for="model in imageModels"
                      :key="model.id"
                      :value="model.id"
                    >
                      {{ model.name }}
                    </option>
                  </select>
                  <label for="cover-description">画面描述</label><textarea
                    id="cover-description"
                    v-model="description"
                    rows="5"
                    maxlength="2000"
                    required
                  />
                  <p class="muted">
                    AI 生成无字背景，中文由本地排版。书名、作者和画面描述会发送至所选服务，可能产生费用。
                  </p>
                  <button
                    type="submit"
                    :disabled="!description.trim() || !providerId"
                  >
                    <AppIcon name="sparkle" />生成 AI 背景
                  </button>
                </fieldset>
              </form>
              <button
                type="button"
                class="ghost refresh-models"
                :disabled="busy"
                @click="loadImageModels"
              >
                <AppIcon name="refresh" />刷新图片模型
              </button>
            </template>
            <div
              v-if="taskId"
              class="notice"
              role="status"
            >
              正在生成背景，最多等待 180 秒…<button
                type="button"
                class="secondary"
                :disabled="cancelling"
                @click="cancelImage"
              >
                {{ cancelling ? '正在取消…' : '取消生成' }}
              </button>
            </div>
          </section>
          <details
            v-if="writing?.coverPromptLandscape || writing?.coverPromptPortrait"
            class="cover-suggestions"
          >
            <summary>本书的画面提示词</summary>
            <label for="cover-prompt-ratio">封面提示词比例</label><select
              id="cover-prompt-ratio"
              v-model="promptRatio"
            >
              <option value="landscape">
                横版 4:3 · 1440 × 1080
              </option><option value="portrait">
                竖版 3:4 · 1080 × 1440
              </option>
            </select>
            <label for="cover-suggested-prompt">书籍专属画面提示词</label><textarea
              id="cover-suggested-prompt"
              :value="suggestedPrompt"
              rows="5"
              readonly
            />
            <div class="actions wrap">
              <button
                type="button"
                class="secondary"
                :disabled="!suggestedPrompt.trim()"
                @click="copyPrompt"
              >
                复制封面提示词
              </button><button
                type="button"
                class="secondary"
                :disabled="busy || !suggestedPrompt.trim()"
                @click="usePrompt"
              >
                用于 AI 背景
              </button>
            </div>
            <p class="muted">
              外部工具请选择对应比例；内置 AI 生成一张可双向裁切的背景，再排版导出两种比例。切换提示词不会替换你的画面描述。
            </p>
          </details>
        </div>
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
        >预览比例</label><select
          id="cover-size"
          :value="coverSizes.findIndex(s => s.width === draft.width && s.height === draft.height)"
          :disabled="busy"
          @change="changeSize"
        >
          <option
            v-for="(size, index) in coverSizes"
            :key="index"
            :value="index"
          >
            {{ size.label }}
          </option>
        </select>
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
          />
        </div>
        <p class="preview-caption">
          同一份内容，两种精心排版的封面
        </p>
        <div class="export-formats">
          <span><i class="ratio-icon landscape" />横版 4:3</span><span><i class="ratio-icon portrait" />竖版 3:4</span><span class="badge">高清 PNG</span>
        </div>
        <details class="preview-details">
          <summary>排版与导出信息</summary><p class="muted">
            文字自动换行，过长时缩小字号。当前标题 {{ actualSizes.titleSize }} px，副标题 {{ actualSizes.subtitleSize }} px。每次保存或导出都会新建版本。
          </p><p class="file-path">
            导出目录：{{ workspace }} / covers
          </p><button
            class="secondary"
            :disabled="busy"
            @click="run(() => openProjectDirectory(id))"
          >
            <AppIcon name="folder" />打开项目目录
          </button>
        </details>
      </section>
    </div>
    <div class="sticky-actions">
      <span
        class="save-status"
        :class="{ 'is-dirty': dirty }"
      ><i class="status-dot" />{{ dirty ? '有未保存修改' : '编辑配置已保存' }}</span>
      <div class="actions wrap">
        <button
          type="submit"
          form="cover-edit-form"
          class="secondary"
          :disabled="busy"
        >
          <AppIcon name="save" />保存编辑配置
        </button><button
          type="button"
          :disabled="busy || backgroundLoading || !!previewError || !!backgroundError"
          @click="save(true)"
        >
          <AppIcon name="download" />导出两张封面
        </button>
      </div>
    </div>
    <details class="history-panel">
      <summary>封面历史 · {{ versions.length }} 个版本</summary><label for="cover-history">恢复编辑配置</label><select
        id="cover-history"
        :value="selected?.id ?? ''"
        :disabled="busy"
        @change="switchVersion"
      >
        <option
          value=""
          disabled
        >
          尚未保存
        </option><option
          v-for="version in versions"
          :key="version.id"
          :value="version.id"
        >
          v{{ version.versionNo }} · {{ version.config.width }} × {{ version.config.height }} · {{ version.exportedPath ? '已导出' : '编辑配置' }} · {{ new Date(version.createdAt).toLocaleString() }}
        </option>
      </select><p
        v-if="selected?.exportedPath"
        class="file-path"
      >
        {{ selected.exportedPath }}
      </p>
    </details>
  </template>
</template>
<style scoped>
.cover-workbench { display: grid; grid-template-columns: minmax(270px, .85fr) minmax(0, 1.15fr); gap: 23px; align-items: start; }
.cover-controls, .cover-preview { padding: 22px; margin-bottom: 0; }
.cover-controls .editor-tabs { margin-bottom: 7px; }
.cover-controls label { margin-top: 16px; }
.cover-controls .muted { font-size: 11px; }
.cover-controls textarea { font-size: 12px; }
.cover-controls .form-grid { gap: 0 12px; }
.cover-controls .editor-tabs :deep(button) { padding: 8px 4px; font-size: 11px; }
.cover-colors { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 12px; }
.cover-preview { position: sticky; top: 20px; }
.cover-preview h2 { font-size: 13px; }
.preview-dimensions { font-size: 10px; color: #8b9980; }
#cover-size { font-size: 11px; min-height: 35px; max-width: 235px; padding: 7px 10px; margin-bottom: 15px; }
.canvas-stage { display: flex; align-items: center; justify-content: center; height: clamp(280px, 43vh, 470px); padding: 24px; background-color: #eef0e9; background-image: radial-gradient(#cbd4c0 0.8px, transparent 0.8px); background-size: 12px 12px; border: 1px solid #e4e8db; border-radius: 9px; overflow: hidden; }
canvas { display: block; max-width: 100%; max-height: 100%; width: auto; height: auto; box-shadow: 0 6px 18px #22352224; }
.preview-caption { text-align: center; color: #8d9b81; font-size: 11px; margin: 17px 0 11px; }
.export-formats { display: flex; align-items: center; justify-content: center; gap: 17px; }
.export-formats > span { display: inline-flex; align-items: center; gap: 6px; color: #829274; font-size: 10px; }
.ratio-icon { width: 17px; height: 13px; border: 1px solid #99ae8a; border-radius: 2px; }
.ratio-icon.portrait { width: 12px; height: 16px; }
.cover-suggestions, .preview-details { margin-top: 21px; padding-top: 15px; border-top: 1px solid var(--line); color: #8c9b7f; font-size: 11px; }
.cover-suggestions summary, .preview-details summary { cursor: pointer; }
.cover-suggestions summary .app-icon { width: 14px; height: 14px; }
.ai-background { border-top: 1px solid var(--line); padding-top: 17px; margin-top: 17px; }
.ai-toggle, .choose-image { width: 100%; margin-top: 11px; }
.refresh-models { margin-top: 8px; }
@media (max-width: 1180px) { .cover-workbench { gap: 18px; } .cover-controls, .cover-preview { padding: 18px; } }
@media (max-width: 980px) { .cover-workbench { grid-template-columns: 1fr; gap: 14px; } .cover-preview { position: static; } }
@media (max-width: 850px) { .cover-controls, .cover-preview { padding: 15px; } .canvas-stage { padding: 15px; height: 300px; } .export-formats { gap: 9px; } }
@media (max-width: 720px) { .cover-workbench { grid-template-columns: 1fr; } .cover-preview { position: static; } .canvas-stage { height: 320px; } }
</style>
