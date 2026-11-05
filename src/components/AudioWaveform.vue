<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, useId, watch } from 'vue'
import { Slider } from '@/components/ui/slider'
import { mediaClock } from '../lib/mediaTime'
import { loadWaveform } from '../services/audioWaveform'

const props = defineProps<{
  source: string; audio?: globalThis.HTMLAudioElement; currentTime: number; duration: number
  playing: boolean; disabled: boolean
}>()
const emit = defineEmits<{ seek: [time: number] }>()
const id = useId()
const root = ref<globalThis.HTMLElement>()
const playedClip = ref<globalThis.SVGRectElement>()
const playhead = ref<globalThis.HTMLElement>()
const peaks = ref<number[]>([])
const visible = ref(false)
const loading = ref(false)
const failed = ref(false)
const hover = ref<number>()
const count = ref(96)
const wavePath = computed(() => Array.from({ length: count.value }, (_, bar) => {
  const start = Math.floor(bar * peaks.value.length / count.value)
  const end = Math.max(start + 1, Math.floor((bar + 1) * peaks.value.length / count.value))
  const peak = Math.max(0, ...peaks.value.slice(start, end))
  const height = Math.max(3, peak * 52)
  const x = (bar + .5) * 1000 / count.value
  return `M${x.toFixed(2)},${(32 - height / 2).toFixed(2)}v${height.toFixed(2)}`
}).join(''))
const hint = computed(() => loading.value ? '正在读取原声波形…' : failed.value ? '波形暂不可用，仍可拖动定位' : '点击或拖动波形定位')
let frame = 0
let resize: globalThis.ResizeObserver | undefined
let visibility: globalThis.IntersectionObserver | undefined
function drawPosition() {
  const time = props.audio?.currentTime ?? props.currentTime
  const progress = props.duration ? Math.max(0, Math.min(1, time / props.duration)) : 0
  playedClip.value?.setAttribute('width', String(progress * 1000))
  if (playhead.value) playhead.value.style.left = `${progress * 100}%`
}
function animate() {
  drawPosition()
  if (props.playing && !props.disabled && visible.value) frame = globalThis.requestAnimationFrame(animate)
}
function move(event: globalThis.PointerEvent) {
  if (props.disabled || !props.duration) return
  const box = (event.currentTarget as globalThis.HTMLElement).getBoundingClientRect()
  if (box.width) hover.value = Math.max(0, Math.min(1, (event.clientX - box.left) / box.width))
}
watch(() => [props.source, visible.value, props.disabled], async (_next, _old, onCleanup) => {
  let current = true
  onCleanup(() => { current = false })
  if (!props.source || !visible.value || props.disabled) return
  peaks.value = []; failed.value = false; loading.value = true
  try { const result = await loadWaveform(props.source); if (current) peaks.value = result }
  catch { if (current) failed.value = true }
  finally { if (current) loading.value = false }
}, { immediate: true })
watch(() => [props.currentTime, props.duration], drawPosition, { flush: 'post' })
watch(() => [props.playing, props.disabled, visible.value], () => {
  globalThis.cancelAnimationFrame(frame)
  drawPosition()
  if (props.playing && !props.disabled && visible.value) frame = globalThis.requestAnimationFrame(animate)
})
onMounted(() => {
  if (!root.value) return
  resize = new globalThis.ResizeObserver(entries => { count.value = Math.max(24, Math.min(128, Math.floor(entries[0]!.contentRect.width / 9))) })
  resize.observe(root.value)
  visibility = new globalThis.IntersectionObserver(entries => { visible.value = entries[0]!.isIntersecting }, { rootMargin: '100px' })
  visibility.observe(root.value)
  drawPosition()
})
onBeforeUnmount(() => { resize?.disconnect(); visibility?.disconnect(); globalThis.cancelAnimationFrame(frame) })
</script>

<template>
  <div
    ref="root"
    class="audio-waveform"
    :data-waveform="loading ? 'loading' : failed ? 'unavailable' : peaks.length ? 'ready' : 'pending'"
  >
    <div class="waveform-surface">
      <div
        class="waveform-stage"
        @pointermove="move"
        @pointerleave="hover = undefined"
      >
        <svg
          class="waveform-image"
          viewBox="0 0 1000 64"
          preserveAspectRatio="none"
          aria-hidden="true"
        >
          <defs>
            <linearGradient
              :id="`wave-color-${id}`"
              x1="0"
              y1="0"
              x2="0"
              y2="1"
            >
              <stop
                offset="0"
                stop-color="var(--player-accent-light)"
              />
              <stop
                offset="1"
                stop-color="var(--player-accent)"
              />
            </linearGradient>
            <clipPath :id="`wave-played-${id}`"><rect
              ref="playedClip"
              x="0"
              y="0"
              width="0"
              height="64"
            /></clipPath>
            <clipPath :id="`wave-hover-${id}`"><rect
              x="0"
              y="0"
              :width="(hover ?? 0) * 1000"
              height="64"
            /></clipPath>
          </defs>
          <path
            :d="wavePath"
            class="waveform-base"
          />
          <path
            v-if="hover !== undefined"
            :d="wavePath"
            class="waveform-hover"
            :clip-path="`url(#wave-hover-${id})`"
          />
          <path
            :d="wavePath"
            :stroke="`url(#wave-color-${id})`"
            :clip-path="`url(#wave-played-${id})`"
          />
        </svg>
        <span
          ref="playhead"
          class="waveform-playhead"
          aria-hidden="true"
        />
        <Slider
          class="waveform-seek player-progress"
          :model-value="[currentTime]"
          :max="duration || 1"
          :step="0.1"
          :disabled="disabled || !duration"
          aria-label="播放进度"
          :aria-value-text="`${mediaClock(currentTime)} / ${duration ? mediaClock(duration) : '时长待载入'}`"
          @update:model-value="$event?.[0] !== undefined && emit('seek', $event[0])"
        />
      </div>
    </div>
    <span
      v-if="hover !== undefined"
      class="waveform-tooltip"
      :style="{ left: `clamp(30px, ${hover * 100}%, calc(100% - 30px))` }"
      aria-hidden="true"
    >{{ mediaClock(hover * duration) }}</span>
    <div class="player-times">
      <span>{{ mediaClock(currentTime) }}</span><span class="waveform-hint">{{ hint }}</span><span>{{ duration ? mediaClock(duration) : '--:--' }}</span>
    </div>
  </div>
</template>

<style scoped>
.audio-waveform { min-width: 0; position: relative; }
.waveform-surface { position: relative; height: 64px; padding: 0; overflow: hidden; border: 0; border-radius: 0; background: transparent; transition: border-color 160ms; }
.waveform-surface:hover, .waveform-surface:focus-within { border-color: var(--player-accent); }
.waveform-stage { position: relative; width: 100%; height: 100%; }
.waveform-image { position: absolute; width: 100%; height: 64px; top: 0; left: 0; pointer-events: none; }
.waveform-image path { fill: none; stroke-width: 5; stroke-linecap: round; vector-effect: non-scaling-stroke; }
.waveform-base { stroke: var(--player-wave); }
.waveform-hover { stroke: var(--player-accent); opacity: .35; }
.waveform-playhead { position: absolute; top: 0; bottom: 0; left: 0; width: 2px; pointer-events: none; background: var(--player-accent); box-shadow: 0 0 10px var(--player-accent); transform: translateX(-1px); }
.waveform-playhead::before { position: absolute; top: -2px; left: -4px; width: 10px; height: 10px; border: 2px solid var(--player-accent); border-radius: 50%; background: var(--player-text); box-shadow: 0 0 10px var(--player-accent); content: ''; }
.waveform-seek { position: absolute; inset: 0; height: 100%; cursor: pointer; }
.waveform-seek :deep([data-slot='slider-track']) { height: 100%; background: transparent !important; }
.waveform-seek :deep([data-slot='slider-range']) { background: transparent !important; }
.waveform-seek :deep([data-slot='slider-thumb']) { height: 100%; width: 24px; border: 0; opacity: 0; box-shadow: none !important; }
.waveform-tooltip { position: absolute; top: -31px; transform: translateX(-50%); z-index: 2; padding: 3px 8px; color: var(--player-accent-light); background: var(--player-surface); border: 1px solid var(--player-line); border-radius: 6px; font-size: 11px; font-variant-numeric: tabular-nums; pointer-events: none; box-shadow: 0 3px 10px #00000026; }
.player-times { display: flex; justify-content: space-between; align-items: center; gap: 12px; margin-top: 4px; margin-bottom: 12px; color: var(--player-muted); font: 400 11px/16.5px var(--font-mono); font-variant-numeric: tabular-nums; }
.player-times > span:first-child { color: var(--player-accent-light); font-weight: 550; }
.waveform-hint { color: var(--player-muted); font-size: 11px; text-align: center; }
@container audio-player (max-width: 400px) { .waveform-surface { height: 72px; } .waveform-image { height: 56px; } .waveform-hint { display: none; } }
@media (prefers-reduced-motion: reduce) { .waveform-surface { transition: none; } }
</style>
