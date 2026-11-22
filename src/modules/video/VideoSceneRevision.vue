<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue'
import { AlertDialog, AlertDialogCancel, AlertDialogContent, AlertDialogTitle } from '@/components/ui/alert-dialog'
import { Button } from '@/components/ui/button'
import { Textarea } from '@/components/ui/textarea'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { cancelGeneration } from '../../services/ai'
import { errorMessage } from '../../services/verification'
import * as api from '../../services/video'
import { visualCatalog, type VideoScene, type VideoSceneRevisionInput } from '../../types/video'
import type { ModelProvider } from '../../types/ai'
import { subtitleRows } from './subtitles'

const props = defineProps<{ input: VideoSceneRevisionInput; providers: ModelProvider[] }>()
const emit = defineEmits<{ adopt: [scene: VideoScene]; close: [] }>()
const providerId = ref(props.input.providerId)
const instruction = ref('')
const busy = ref(false)
const stopping = ref(false)
const taskId = ref('')
const error = ref('')
const candidate = ref<VideoScene>()
const original = props.input.scenes[props.input.sceneIndex]!
const cues = subtitleRows(props.input.subtitles)
const comparisons = computed(() => [{ label: '当前分镜', scene: original }, ...(candidate.value ? [{ label: '候选分镜', scene: candidate.value }] : [])])
let disposed = false
let stopped = false
const opener = globalThis.document.activeElement instanceof globalThis.HTMLElement ? globalThis.document.activeElement : null
function restoreFocus(event: globalThis.Event) { event.preventDefault(); if (opener?.isConnected) opener.focus({ preventScroll: true }) }
function adopt() { if (candidate.value && !busy.value) emit('adopt', candidate.value) }
async function generate() {
  if (busy.value || !instruction.value.trim() || !providerId.value) return
  busy.value = true; stopped = false; error.value = ''; candidate.value = undefined
  try {
    const id = await api.startSceneRevision({ ...props.input, instruction: instruction.value.trim(), providerId: providerId.value })
    taskId.value = id
    if (disposed) { await cancelGeneration(id); return }
    const scene = await api.executeSceneRevision(id)
    if (!disposed && !stopped) candidate.value = scene
  } catch (cause) { if (!disposed) error.value = stopped ? '单镜生成已停止，原字幕和分镜保留。' : errorMessage(cause) }
  finally { taskId.value = ''; busy.value = false }
}
async function cancel() {
  if (!taskId.value || stopping.value) return
  stopped = true; stopping.value = true
  try { await cancelGeneration(taskId.value); candidate.value = undefined; error.value = '单镜生成已停止，原字幕和分镜保留。' }
  catch (cause) { error.value = `停止失败：${errorMessage(cause)}` }
  finally { stopping.value = false }
}
watch([instruction, providerId], () => { candidate.value = undefined; error.value = '' })
onBeforeUnmount(() => { disposed = true; if (taskId.value) void cancelGeneration(taskId.value).catch(() => {}) })
</script>

<template>
  <AlertDialog
    :open="true"
    @update:open="!$event && !busy && emit('close')"
  >
    <AlertDialogContent
      class="max-w-4xl flex flex-col overflow-hidden"
      :aria-describedby="undefined"
      :aria-busy="busy"
      @escape-key-down="busy && $event.preventDefault()"
      @close-auto-focus="restoreFocus"
    >
      <AlertDialogTitle class="text-lg font-semibold">
        重新生成第 {{ input.sceneIndex + 1 }} 镜
      </AlertDialogTitle>
      <div class="scene-revision-body">
        <p class="muted">
          固定字幕 {{ original.first + 1 }}—{{ original.last + 1 }}。先对照候选再采用；采用后自动暂存，正式保存和试片仍需主动操作。
        </p>
        <p
          v-if="original.image"
          class="notice"
        >
          这一镜使用关键图片。图片、裁切和来源原样保留，本次只调整标题、构图、场景与景别。
        </p>
        <div class="video-field">
          <label for="scene-revision-provider">本次文本模型</label>
          <Select
            v-model="providerId"
            :disabled="busy"
          >
            <SelectTrigger
              id="scene-revision-provider"
              class="w-full"
            >
              <SelectValue placeholder="选择文本模型" />
            </SelectTrigger><SelectContent>
              <SelectItem
                v-for="provider in providers"
                :key="provider.id"
                :value="provider.id"
              >
                {{ provider.name }} · {{ provider.modelName }}
              </SelectItem>
            </SelectContent>
          </Select>
        </div>
        <div class="video-field">
          <label for="scene-revision-instruction">这一镜的修改要求</label>
          <Textarea
            id="scene-revision-instruction"
            v-model="instruction"
            :disabled="busy"
            rows="3"
            maxlength="2000"
            placeholder="例如：突出这一句的转折，把结论动作放在对应字幕出现时。"
          />
        </div>
        <p class="muted">
          将发送本镜与相邻字幕、分镜和修改要求，调用一次所选文本模型；不发送音频或图片。单次最多等待5分钟。
        </p>
        <p
          v-if="error"
          role="alert"
          class="notice"
        >
          {{ error }}
        </p>
        <p
          v-if="busy"
          role="status"
          aria-live="polite"
        >
          {{ stopping ? '正在停止…' : '正在生成这一镜，原分镜保留…' }}
        </p>
        <p
          v-else-if="candidate"
          role="status"
        >
          候选已就绪，请核对画面和字幕锚点，再采用。
        </p>
        <div
          class="scene-comparison"
          :class="{ 'has-candidate': candidate }"
        >
          <section
            v-for="entry in comparisons"
            :key="entry.label"
            :aria-label="entry.label"
            class="scene-candidate"
          >
            <h3>{{ entry.label }}</h3><strong>{{ entry.scene.title }}</strong>
            <p class="muted">
              {{ visualCatalog.kinds[entry.scene.kind as keyof typeof visualCatalog.kinds] }} · {{ visualCatalog.environments[entry.scene.environment ?? 'plain'] }} · {{ visualCatalog.framings[entry.scene.framing ?? 'reference'] }}
            </p>
            <p v-if="entry.scene.image">
              保留当前关键图片及裁切
            </p>
            <ol
              v-else
              class="scene-candidate-beats"
            >
              <li
                v-for="(beat, index) in entry.scene.beats"
                :key="index"
              >
                <strong>{{ visualCatalog.objects[beat.object] }} · {{ visualCatalog.actions[beat.action] }} · {{ beat.label }}</strong>
                <p class="muted">
                  字幕 {{ beat.cue + 1 }}：{{ cues[beat.cue]?.text }}
                </p>
              </li>
            </ol>
          </section>
        </div>
      </div>
      <div class="actions wrap scene-revision-actions">
        <Button
          v-if="candidate"
          :disabled="busy"
          @click="adopt"
        >
          采用这一镜
        </Button>
        <Button
          :variant="candidate ? 'outline' : 'default'"
          :disabled="busy || !providerId || !instruction.trim()"
          @click="generate"
        >
          {{ candidate ? '重新生成候选' : '生成候选分镜' }}
        </Button>
        <Button
          v-if="busy"
          variant="outline"
          :disabled="!taskId || stopping"
          @click="cancel"
        >
          停止单镜生成
        </Button>
        <AlertDialogCancel as-child>
          <Button
            variant="outline"
            :disabled="busy"
          >
            {{ candidate ? '放弃候选' : '关闭' }}
          </Button>
        </AlertDialogCancel>
      </div>
    </AlertDialogContent>
  </AlertDialog>
</template>

<style scoped>
.scene-revision-body { display: grid; gap: 1rem; min-height: 0; overflow-y: auto; }
.scene-revision-actions { flex-shrink: 0; border-top: 1px solid var(--border); padding-top: 1rem; }
.scene-comparison { display: grid; gap: 1rem; min-width: 0; }
.scene-comparison.has-candidate { grid-template-columns: repeat(2, minmax(0, 1fr)); }
.scene-candidate { display: grid; align-content: start; gap: .6rem; padding: 1rem; border: 1px solid var(--border); border-radius: .75rem; overflow-wrap: anywhere; min-width: 0; }
.scene-candidate-beats { display: grid; gap: .75rem; margin: 0; padding-left: 1.2rem; }
@media (max-width: 700px) { .scene-comparison.has-candidate { grid-template-columns: 1fr; } }
</style>
