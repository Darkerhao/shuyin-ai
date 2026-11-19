<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, shallowRef, watch } from 'vue'
import { Button } from '@/components/ui/button'
import type { VideoBeat, VideoScene, VideoCue, PencilCharacter } from '../../types/video'
import '../../../src-tauri/video/pencil-drawing.js'
const props = defineProps<{ beats: VideoBeat[]; title: string; scene?: VideoScene; cues?: VideoCue[]; characters?: PencilCharacter[] }>()
const strokes = computed(() => props.beats.flatMap(beat => beat.drawing?.strokes ?? []))
const canvas = ref<globalThis.HTMLCanvasElement>()
const error = ref('')
const drawing = shallowRef<ReturnType<typeof globalThis.SHUYIN_PENCIL_DRAWING.compile>>()
const duration = ref(0)
const elapsed = ref(0)
const playing = ref(false)
const reducedMotion = ref(false)
let frame: number | undefined
let lastFrame = 0
let motionQuery: ReturnType<typeof globalThis.matchMedia> | undefined
let motionChange: ((event: { matches: boolean }) => void) | undefined

function stop() {
  playing.value = false
  if (frame !== undefined) globalThis.cancelAnimationFrame(frame)
  frame = undefined
}
function render() {
  if (canvas.value && drawing.value) globalThis.SHUYIN_PENCIL_DRAWING.render(canvas.value, drawing.value, elapsed.value, false)
}
function compile() {
  stop()
  drawing.value = undefined
  duration.value = 0
  elapsed.value = 0
  if (!props.scene?.artwork || !props.cues?.length) return
  try {
    const scene = props.scene
    const start = props.cues[scene.first]!.start
    const end = props.cues[scene.last + 1]?.start ?? props.cues[scene.last]!.end
    drawing.value = globalThis.SHUYIN_PENCIL_DRAWING.compile(scene.artwork!, props.characters ?? [], props.cues.map((c, index) => ({ ...c, index })), start, end - start, scene.first, scene.last)
    duration.value = end - start
    elapsed.value = duration.value
    render()
    error.value = ''
  } catch (cause) { error.value = cause instanceof Error ? cause.message : '画稿预览失败，请重新读取。' }
}
function tick(now: number) {
  if (!playing.value || reducedMotion.value) return
  elapsed.value = Math.min(duration.value, elapsed.value + Math.max(0, now - lastFrame) / 1000)
  lastFrame = now
  if (elapsed.value >= duration.value) { stop(); return }
  frame = globalThis.requestAnimationFrame(tick)
}
function toggle() {
  if (!drawing.value || reducedMotion.value || !duration.value) return
  if (playing.value) { stop(); return }
  if (elapsed.value >= duration.value) elapsed.value = 0
  playing.value = true
  lastFrame = globalThis.performance.now()
  frame = globalThis.requestAnimationFrame(tick)
}
function seek() { stop(); render() }
watch(() => [props.scene?.artwork, props.cues, props.characters], compile, { deep: true, immediate: true })
watch(elapsed, render)
watch(canvas, render)
onMounted(() => {
  reducedMotion.value = typeof globalThis.matchMedia === 'function' && globalThis.matchMedia('(prefers-reduced-motion: reduce)').matches
  if (typeof globalThis.matchMedia !== 'function') return
  motionQuery = globalThis.matchMedia('(prefers-reduced-motion: reduce)')
  motionChange = event => {
    reducedMotion.value = event.matches
    if (event.matches) { stop(); elapsed.value = duration.value; render() }
  }
  motionQuery.addEventListener('change', motionChange)
})
onBeforeUnmount(() => {
  stop()
  if (motionQuery && motionChange) motionQuery.removeEventListener('change', motionChange)
})
</script>

<template>
  <canvas
    v-if="scene?.artwork"
    ref="canvas"
    width="1600"
    height="900"
    class="drawing-preview"
    role="img"
    :aria-label="`${title}精绘场景预览`"
  />
  <div
    v-if="scene?.artwork && duration"
    class="drawing-preview-controls"
  >
    <Button
      variant="outline"
      size="sm"
      :disabled="reducedMotion"
      :aria-label="playing ? '暂停精绘预览' : '播放精绘预览'"
      @click="toggle"
    >
      {{ reducedMotion ? '减少动态效果' : playing ? '暂停绘制' : elapsed >= duration ? '重新播放' : '播放绘制' }}
    </Button>
    <label class="drawing-preview-seek">
      <span>绘制进度</span>
      <input
        v-model.number="elapsed"
        type="range"
        min="0"
        :max="duration"
        step="0.01"
        aria-label="精绘预览进度"
        @input="seek"
      >
      <output>{{ elapsed.toFixed(1) }} / {{ duration.toFixed(1) }} 秒</output>
    </label>
  </div>
  <p
    v-if="error"
    role="alert"
  >
    {{ error }}
  </p>
  <svg
    v-if="!scene?.artwork && strokes.length"
    viewBox="0 0 1000 300"
    class="drawing-preview"
    role="img"
    :aria-label="`${title}完整场景笔画预览`"
  >
    <polyline
      v-for="(stroke, index) in strokes"
      :key="index"
      :points="stroke.map(point => point.join(',')).join(' ')"
    />
  </svg>
</template>

<style scoped>
.drawing-preview { width: 100%; background: #fbfaf7; border: 1px solid var(--border); border-radius: 8px; }
.drawing-preview-controls { display: flex; align-items: center; gap: 12px; flex-wrap: wrap; }
.drawing-preview-seek { display: grid; grid-template-columns: auto minmax(120px, 1fr) auto; align-items: center; gap: 8px; min-width: min(100%, 340px); color: var(--muted-foreground); font-size: 12px; }
.drawing-preview-seek input { min-width: 120px; accent-color: var(--video-accent); }
.drawing-preview-seek output { white-space: nowrap; font-variant-numeric: tabular-nums; }
.drawing-preview polyline { fill: none; stroke: #414348; stroke-width: 1.2; stroke-linecap: round; stroke-linejoin: round; }
@media (max-width: 520px) { .drawing-preview-seek { grid-template-columns: 1fr auto; width: 100%; } .drawing-preview-seek input { grid-column: 1 / -1; width: 100%; } }
</style>
