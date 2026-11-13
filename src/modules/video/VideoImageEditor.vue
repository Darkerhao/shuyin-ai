<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Textarea } from '@/components/ui/textarea'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '@/components/ui/collapsible'
import ConfirmAction from '../../components/ConfirmAction.vue'
import OperationFeedback from '../../components/OperationFeedback.vue'
import { useOperation } from '../../composables/useOperation'
import { cancelGeneration } from '../../services/ai'
import * as api from '../../services/video'
import type { Asset } from '../../types/project'
import type { ModelProvider } from '../../types/ai'
import type { VideoSceneImage } from '../../types/video'

const props = defineProps<{ projectId: string; chapterId?: string | null; sceneIndex: number; image?: VideoSceneImage; assets: Asset[]; providers: ModelProvider[]; disabled: boolean }>()
const emit = defineEmits<{ 'update:image': [image: VideoSceneImage | undefined]; 'refresh-assets': []; working: [value: boolean] }>()
const { busy, error, message, run } = useOperation()
const prefix = computed(() => `video-image-${props.sceneIndex}`)
const images = computed(() => props.assets.filter(a => a.assetType === 'image'))
const selectedImageName = computed(() => images.value.find(a => a.id === props.image?.assetId)?.originalName ?? '图片关联已缺失，请重新选择')
const providerId = ref('')
const description = ref('')
const confirming = ref(false)
const taskId = ref('')
const cancelling = ref(false)
const candidate = ref<{ asset: Asset; model: string }>()
const imageUrl = ref('')
const candidateUrl = ref('')
const provider = computed(() => props.providers.find(p => p.id === providerId.value))
watch(() => props.providers, models => { if (!models.some(p => p.id === providerId.value)) providerId.value = models.find(p => p.isDefault)?.id ?? models[0]?.id ?? '' }, { immediate: true })
watch(busy, value => emit('working', value), { flush: 'sync' })
function selectImage(id: unknown) {
  if (id === 'none') { emit('update:image', undefined); return }
  emit('update:image', { assetId: String(id), source: '', rights: '', fit: 'contain', x: .5, y: .5 })
}
function update<K extends keyof VideoSceneImage>(key: K, value: VideoSceneImage[K]) {
  if (props.image) emit('update:image', { ...props.image, [key]: value })
}
const importImage = () => run(async () => {
  const path = await api.chooseImage()
  if (typeof path !== 'string') return
  const id = await api.importImage(props.projectId, path, props.chapterId ?? null)
  emit('refresh-assets'); selectImage(id)
  message.value = '图片已关联，原文件保留。请填写来源和授权，再检查裁切。'
})
const generate = () => {
  confirming.value = false
  return run(async () => {
    if (!provider.value) throw new Error('请选择已启用的图片模型。')
    const model = `${provider.value.name} / ${provider.value.modelName}`
    try {
      taskId.value = await api.startIllustration(props.projectId, providerId.value, description.value, props.chapterId ?? null)
      const asset = await api.executeIllustration(taskId.value)
      candidate.value = { asset, model }
      emit('refresh-assets')
      message.value = '插画已保存为本书素材。检查后主动采用，再填写使用权说明；原分镜尚未改变。'
    } finally { taskId.value = '' }
  })
}
async function cancel() {
  cancelling.value = true
  try { await cancelGeneration(taskId.value) } catch (cause) { error.value = String(cause) }
  finally { cancelling.value = false }
}
function adopt() {
  if (!candidate.value) return
  emit('update:image', { assetId: candidate.value.asset.id, source: `AI插画 · ${candidate.value.model}`, rights: '', fit: 'contain', x: .5, y: .5 })
  candidate.value = undefined
}
watch(() => [props.projectId, props.image?.assetId], async (_value, _old, onCleanup) => {
  imageUrl.value = ''
  if (!props.image) return
  let current = true
  let url = ''
  onCleanup(() => { current = false; if (url) globalThis.URL.revokeObjectURL(url) })
  try {
    const bytes = await api.readImage(props.projectId, props.image.assetId)
    if (current) { url = globalThis.URL.createObjectURL(new globalThis.Blob([bytes])); imageUrl.value = url }
  } catch (cause) { if (current) error.value = `图片无法读取：${String(cause)}` }
}, { immediate: true })
watch(candidate, async (value, _old, onCleanup) => {
  candidateUrl.value = ''
  if (!value) return
  let current = true
  let url = ''
  onCleanup(() => { current = false; if (url) globalThis.URL.revokeObjectURL(url) })
  try {
    const bytes = await api.readImage(props.projectId, value.asset.id)
    if (current) { url = globalThis.URL.createObjectURL(new globalThis.Blob([bytes])); candidateUrl.value = url }
  } catch (cause) { if (current) error.value = String(cause) }
})
onBeforeUnmount(() => emit('working', false))
</script>

<template>
  <Collapsible
    :default-open="!!image"
    class="scene-image-editor"
  >
    <CollapsibleTrigger as-child>
      <Button
        variant="ghost"
        :disabled="disabled"
      >
        {{ image ? '关键图片与来源' : '选用关键图片 / 生成插画' }}
      </Button>
    </CollapsibleTrigger>
    <CollapsibleContent class="scene-image-content">
      <p class="muted">
        图片替代本镜手绘主体，标题和字幕仍统一排版。只选有权使用的静态图片，不自动找图。
      </p>
      <OperationFeedback
        :error="error"
        :message="message"
      />
      <div class="video-field">
        <label :for="prefix">本书图片</label>
        <Select
          :model-value="image?.assetId ?? 'none'"
          :disabled="disabled"
          @update:model-value="selectImage"
        >
          <SelectTrigger :id="prefix">
            <SelectValue>{{ image ? selectedImageName : '使用手绘物体' }}</SelectValue>
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="none">
              使用手绘物体
            </SelectItem>
            <SelectItem
              v-for="asset in images"
              :key="asset.id"
              :value="asset.id"
              :disabled="asset.existsStatus !== 'available'"
            >
              {{ asset.originalName }}{{ asset.existsStatus === 'available' ? '' : '（文件缺失）' }}
            </SelectItem>
          </SelectContent>
        </Select>
      </div>
      <Button
        variant="outline"
        :disabled="disabled"
        @click="importImage"
      >
        导入本书图片
      </Button>
      <template v-if="image">
        <div
          v-if="imageUrl"
          class="scene-image-crop"
        >
          <img
            :src="imageUrl"
            :alt="`第${sceneIndex + 1}镜图片裁切预览`"
            :style="{ objectFit: image.fit, objectPosition: `${image.x * 100}% ${image.y * 100}%` }"
          >
        </div>
        <p class="muted">
          上方只预览裁切；最终排版和出场时机请生成原声试片检查。图片从本镜第一条字幕开始出现。
        </p>
        <div class="video-field">
          <label :for="`${prefix}-fit`">显示方式</label>
          <Select
            :model-value="image.fit"
            :disabled="disabled"
            @update:model-value="update('fit', $event as 'contain' | 'cover')"
          >
            <SelectTrigger :id="`${prefix}-fit`">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="contain">
                完整显示
              </SelectItem><SelectItem value="cover">
                裁切铺满
              </SelectItem>
            </SelectContent>
          </Select>
        </div>
        <template v-if="image.fit === 'cover'">
          <div
            v-for="axis in (['x', 'y'] as const)"
            :key="axis"
            class="video-field"
          >
            <label :for="`${prefix}-${axis}`">{{ axis === 'x' ? '水平' : '垂直' }}裁切位置 · {{ Math.round(image[axis] * 100) }}%</label>
            <Input
              :id="`${prefix}-${axis}`"
              type="range"
              min="0"
              max="1"
              step="0.01"
              :model-value="image[axis]"
              :disabled="disabled"
              @update:model-value="update(axis, Number($event))"
            />
          </div>
        </template>
        <div class="video-field">
          <label :for="`${prefix}-source`">图片来源（必填）</label><Input
            :id="`${prefix}-source`"
            :model-value="image.source"
            maxlength="1000"
            :disabled="disabled"
            placeholder="本人拍摄，或可核对的资料来源"
            @update:model-value="update('source', String($event))"
          />
        </div>
        <div class="video-field">
          <label :for="`${prefix}-creator`">创作者 / 权利人（已知时填写）</label><Input
            :id="`${prefix}-creator`"
            :model-value="image.creator ?? ''"
            maxlength="200"
            :disabled="disabled"
            @update:model-value="update('creator', String($event))"
          />
        </div>
        <div class="video-field">
          <label :for="`${prefix}-rights`">授权 / 自有声明（必填）</label><Textarea
            :id="`${prefix}-rights`"
            :model-value="image.rights"
            maxlength="1000"
            rows="2"
            :disabled="disabled"
            placeholder="说明允许本次视频使用的依据；AI生成也需核对服务条款"
            @update:model-value="update('rights', String($event))"
          />
        </div>
      </template>
      <Collapsible class="scene-image-generation">
        <CollapsibleTrigger as-child>
          <Button
            variant="ghost"
            :disabled="disabled"
          >
            主动生成一张关键插画
          </Button>
        </CollapsibleTrigger>
        <CollapsibleContent class="scene-image-content">
          <p class="muted">
            适合开场、转折或结尾的少量画面。每次1张，不上传图片或音频，只发送书名、作者和本次画面描述；不会自动采用或补图。
          </p>
          <div class="video-field">
            <label :for="`${prefix}-model`">插画图片模型</label><Select
              v-model="providerId"
              :disabled="disabled"
            >
              <SelectTrigger :id="`${prefix}-model`">
                <SelectValue placeholder="选择已启用的图片模型" />
              </SelectTrigger><SelectContent>
                <SelectItem
                  v-for="model in providers"
                  :key="model.id"
                  :value="model.id"
                >
                  {{ model.name }} / {{ model.modelName }}
                </SelectItem>
              </SelectContent>
            </Select>
          </div>
          <div class="video-field">
            <label :for="`${prefix}-description`">本镜插画描述</label><Textarea
              :id="`${prefix}-description`"
              v-model="description"
              maxlength="2000"
              rows="3"
              :disabled="disabled"
              placeholder="描述具体画面，避免未经核实的人物和史料细节；不要要求图片绘制文字"
            />
          </div>
          <div class="actions wrap">
            <Button
              :disabled="disabled || !provider || !description.trim()"
              @click="confirming = true"
            >
              生成前确认
            </Button><Button
              v-if="taskId"
              variant="outline"
              :disabled="cancelling"
              @click="cancel"
            >
              {{ cancelling ? '正在取消…' : '取消插画生成' }}
            </Button>
          </div>
          <p
            v-if="busy"
            role="status"
          >
            正在生成单张插画，请等待；已有图片和分镜保留。
          </p>
          <div
            v-if="candidate"
            class="scene-image-candidate"
          >
            <img
              v-if="candidateUrl"
              :src="candidateUrl"
              alt="尚未采用的视频插画"
            >
            <p>新插画 · {{ candidate.model }} · 尚未用于本镜</p>
            <Button
              :disabled="disabled || !candidateUrl"
              @click="adopt"
            >
              采用此插画
            </Button>
            <Button
              variant="ghost"
              :disabled="disabled"
              @click="candidate = undefined"
            >
              保留为素材，暂不采用
            </Button>
          </div>
        </CollapsibleContent>
      </Collapsible>
    </CollapsibleContent>
  </Collapsible>
  <ConfirmAction
    v-if="confirming"
    title="确认生成视频插画"
    confirm-label="确认生成1张"
    :destructive="false"
    @cancel="confirming = false"
    @confirm="generate"
  >
    <p>模型：{{ provider?.name }} / {{ provider?.modelName }}。本次生成1张横屏插画，可能产生服务费用；价格以所选服务为准。</p>
    <p>将发送书名、作者和下方画面描述，不上传音频或图片。生成后需主动采用并核对使用权，不会覆盖当前分镜。</p>
    <p class="scene-image-prompt">
      {{ description }}
    </p>
  </ConfirmAction>
</template>

<style scoped>
.scene-image-editor { margin-block: .75rem; min-width: 0; }
.scene-image-content { display: grid; grid-template-columns: minmax(0, 1fr); overflow-wrap: anywhere; gap: .85rem; padding-block: .75rem; min-width: 0; }
.scene-image-content > button { justify-self: start; }
.video-field { display: grid; align-content: start; gap: 8px; min-width: 0; }
.video-field :deep([role='combobox']) { width: 100%; min-width: 0; }
.scene-image-editor :deep(button) { max-width: 100%; min-height: 36px; height: auto; white-space: normal; }
.scene-image-editor :deep([data-slot='select-value']) { min-width: 0; text-align: left; }
.scene-image-crop { aspect-ratio: 16 / 9; width: min(100%, 600px); min-width: 0; max-width: 100%; overflow: hidden; background: var(--secondary); border: 1px solid var(--border); border-radius: .75rem; }
.scene-image-crop img { width: 100%; height: 100%; display: block; }
.scene-image-candidate img { max-width: 100%; max-height: 320px; object-fit: contain; border-radius: .75rem; }
.scene-image-prompt { white-space: pre-wrap; overflow-wrap: anywhere; max-height: 12rem; overflow: auto; }
</style>
