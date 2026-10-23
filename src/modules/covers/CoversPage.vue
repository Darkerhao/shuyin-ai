<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, shallowRef, watch } from 'vue'
import { RouterLink, onBeforeRouteLeave, onBeforeRouteUpdate } from 'vue-router'
import * as api from '../../services/covers'
import { listProviders, cancelGeneration } from '../../services/ai'
import type { ModelProvider } from '../../types/ai'
import { getProject, isDesktop, linkAsset, listAssets, openProjectDirectory } from '../../services/projects'
import { coverPng, decodeBackground, renderCover } from '../../services/coverRenderer'
import { coverSizes, coverTemplates, newCover, type CoverConfig, type CoverVersion } from '../../types/cover'
import type { Asset } from '../../types/project'
import { useOperation } from '../../composables/useOperation'

const props = defineProps<{ id: string }>()
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
const imageModels = ref<ModelProvider[]>([])
const providerId = ref('')
const description = ref('')
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
const load = () => run(async () => {
  const [project, saved, assets] = await Promise.all([getProject(props.id), api.listCovers(props.id), listAssets(props.id)])
  bookTitle.value = project.title; workspace.value = project.workspacePath
  versions.value = saved; images.value = assets.filter(a => a.assetType === 'image')
  accept(saved[0] ?? null); ready.value = true
  await nextTick(); paint()
})
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
    if (!description.value) description.value = `为《${bookTitle.value}》设计简洁、有阅读氛围的封面背景，主体居中，四周留白。`
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
  <header>
    <RouterLink :to="`/projects/${id}/writing`">
      ← 发布文案与音频提示词
    </RouterLink>
    <h1>发布图片封面</h1>
    <p class="muted">
      {{ bookTitle }} · 制作发布作品时上传的两张图片：横版 4:3 和竖版 3:4。
    </p>
  </header>
  <p
    v-if="!desktop"
    class="notice"
  >
    请在桌面应用中制作封面。
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
    class="notice file-path"
    role="status"
  >
    {{ message }}
  </p>
  <button
    v-if="!ready && desktop"
    :disabled="busy"
    @click="load"
  >
    {{ busy ? '读取中…' : '重新读取' }}
  </button>
  <template v-if="ready">
    <section
      class="panel"
      aria-label="可选 AI 背景"
    >
      <button
        type="button"
        class="secondary"
        :disabled="busy"
        :aria-expanded="useAi"
        @click="toggleAi"
      >
        {{ useAi ? '收起 AI 背景' : '使用 AI 生成背景（可选）' }}
      </button>
      <p class="muted">
        本地模板可直接使用。AI 仅生成背景，两种方式均导出 4:3 和 3:4 两张图片。
      </p>
      <template v-if="useAi">
        <p
          v-if="!imageModels.length"
          class="notice"
        >
          尚无已启用的图片模型，可继续使用下方本地模板，或前往 <RouterLink to="/settings/models">
            模型设置
          </RouterLink> 添加图片模型。
        </p>
        <form @submit.prevent="generateBackground">
          <fieldset :disabled="busy || !imageModels.length">
            <label for="cover-model">图片模型</label>
            <select
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
            <label for="cover-description">画面描述</label>
            <textarea
              id="cover-description"
              v-model="description"
              rows="3"
              maxlength="2000"
              required
            />
            <p class="muted">
              生成一张无字背景，中文标题由本地排版。书名、作者和画面描述会发送至所选服务，可能产生费用。
            </p>
            <button
              type="submit"
              :disabled="!description.trim() || !providerId"
            >
              生成 AI 背景
            </button>
          </fieldset>
        </form>
        <button
          type="button"
          class="secondary"
          :disabled="busy"
          @click="loadImageModels"
        >
          刷新图片模型
        </button>
      </template>
      <div
        v-if="taskId"
        class="actions"
        role="status"
      >
        <span>正在生成背景，最多等待 180 秒…</span>
        <button
          type="button"
          class="secondary"
          :disabled="cancelling"
          @click="cancelImage"
        >
          {{ cancelling ? '正在取消…' : '取消生成' }}
        </button>
      </div>
    </section>
    <div class="cover-workbench">
      <form
        class="panel cover-controls"
        @submit.prevent="save(false)"
      >
        <fieldset :disabled="busy">
          <div class="form-grid">
            <div>
              <label for="cover-template">模板</label><select
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
            </div>
            <div>
              <label for="cover-size">预览比例</label><select
                id="cover-size"
                :value="coverSizes.findIndex(s => s.width === draft.width && s.height === draft.height)"
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
            </div>
          </div>
          <label for="cover-title">主标题 *</label><textarea
            id="cover-title"
            v-model="draft.title"
            rows="3"
            maxlength="200"
            required
          />
          <label for="cover-subtitle">副标题</label><textarea
            id="cover-subtitle"
            v-model="draft.subtitle"
            rows="2"
            maxlength="300"
          />
          <label for="cover-brand">品牌标识</label><input
            id="cover-brand"
            v-model="draft.brand"
            maxlength="60"
          >
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
              <label for="cover-align">对齐</label><select
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
          <label for="cover-background">背景图</label>
          <select
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
            class="secondary"
            @click="chooseImage"
          >
            选择本地图片
          </button>
          <template v-if="draft.backgroundAssetId">
            <label for="cover-overlay">底色蒙层 · {{ Math.round(draft.overlayOpacity * 100) }}%</label><input
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
          <div class="actions wrap">
            <button
              type="submit"
              class="secondary"
            >
              保存编辑配置
            </button>
            <button
              type="button"
              :disabled="backgroundLoading || !!previewError || !!backgroundError"
              @click="save(true)"
            >
              导出两张封面
            </button>
          </div>
        </fieldset>
      </form>
      <section
        class="cover-preview panel"
        aria-label="封面预览"
      >
        <div class="result-heading">
          <h2>实时预览</h2><span class="badge">{{ draft.width }} × {{ draft.height }}</span>
        </div>
        <p
          v-if="backgroundLoading"
          role="status"
        >
          正在读取背景图…
        </p>
        <p
          v-if="backgroundError || previewError"
          class="error"
          role="alert"
        >
          {{ backgroundError || previewError }}
        </p>
        <canvas
          ref="canvas"
          :style="{ aspectRatio: `${draft.width} / ${draft.height}` }"
          aria-label="当前封面画布"
          role="img"
        />
        <p class="muted">
          导出时分别生成 4:3（1440 × 1080）和 3:4（1080 × 1440）PNG，供发布作品时上传。
        </p>
        <p class="muted">
          文字自动换行，过长时缩小字号。当前标题 {{ actualSizes.titleSize }} px，副标题 {{ actualSizes.subtitleSize }} px。
        </p>
        <p class="muted">
          每次保存或导出都会新建版本。背景图居中裁切；原图片保持在原位置。
        </p>
        <p class="file-path">
          导出目录：{{ workspace }} / covers
        </p>
        <button
          class="secondary"
          :disabled="busy"
          @click="run(() => openProjectDirectory(id))"
        >
          打开项目目录
        </button>
      </section>
    </div>
    <details class="history-panel">
      <summary>封面历史 · {{ versions.length }} 个版本{{ dirty ? ' · 有未保存修改' : '' }}</summary>
      <label for="cover-history">恢复编辑配置</label>
      <select
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
        </option>
        <option
          v-for="version in versions"
          :key="version.id"
          :value="version.id"
        >
          v{{ version.versionNo }} · {{ version.config.width }} × {{ version.config.height }} · {{ version.exportedPath ? '已导出' : '编辑配置' }} · {{ new Date(version.createdAt).toLocaleString() }}
        </option>
      </select>
      <p
        v-if="selected?.exportedPath"
        class="file-path"
      >
        {{ selected.exportedPath }}
      </p>
    </details>
  </template>
</template>

<style scoped>
.cover-workbench { display: grid; grid-template-columns: minmax(280px, 1fr) minmax(240px, 1fr); gap: 20px; align-items: start; }
.cover-controls, .cover-preview { padding: 20px; }
.cover-controls label { margin-top: 14px; }
.cover-controls .form-grid { gap: 0 12px; }
.cover-controls button.secondary { margin-top: 10px; }
.cover-controls .actions button { margin-top: 0; }
.cover-colors { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 12px; }
.cover-colors input { padding: 4px; }
.cover-preview { position: sticky; top: 20px; }
.cover-preview h2 { margin: 0; }
canvas { display: block; width: 100%; height: auto; border: 1px solid #dde3d9; }
@media (max-width: 1050px) { .cover-workbench { grid-template-columns: 1fr; } .cover-preview { position: static; grid-row: 1; } canvas { max-height: 460px; object-fit: contain; } }
</style>
