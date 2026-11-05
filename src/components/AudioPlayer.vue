<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, useId, watch } from 'vue'
import { Button } from '@/components/ui/button'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { Slider } from '@/components/ui/slider'
import { mediaSource } from '../services/projects'
import AppIcon from './AppIcon.vue'
import AudioWaveform from './AudioWaveform.vue'

const props = withDefaults(defineProps<{
  projectId: string
  assetId: string
  title: string
  subtitle?: string
  format?: string
  ariaLabel?: string
  disabled?: boolean
}>(), { subtitle: '本地原声', format: '', ariaLabel: '原声音频播放器', disabled: false })
const emit = defineEmits<{ play: []; pause: []; ended: []; timeupdate: []; seek: []; error: [] }>()
const controlId = useId()
const media = ref<globalThis.HTMLAudioElement>()
const source = ref('')
const loading = ref(false)
const playing = ref(false)
const waiting = ref(false)
const playPending = ref(false)
const currentTime = ref(0)
const duration = ref(0)
const volume = ref(75)
const muted = ref(false)
const rate = ref('1')
const theme = ref<'emerald' | 'oled'>('emerald')
const error = ref('')
let sourceRevision = 0
let playRevision = 0
const available = computed(() => !!source.value && !loading.value && !props.disabled)
const silent = computed(() => muted.value || volume.value === 0)
const status = computed(() => error.value ? '暂不可播放' : loading.value || playPending.value || waiting.value ? '正在载入' : '原声试听')
function pause() {
  playRevision++
  if (playing.value || playPending.value || (media.value && !media.value.paused)) media.value?.pause()
  playing.value = false; waiting.value = false; playPending.value = false
}
async function loadSource() {
  const revision = ++sourceRevision
  pause(); source.value = ''; currentTime.value = 0; duration.value = 0; error.value = ''
  if (!props.assetId) { loading.value = false; return }
  loading.value = true
  try {
    const url = await mediaSource(props.projectId, props.assetId)
    if (revision === sourceRevision) source.value = url
  } catch {
    if (revision === sourceRevision) fail('音频暂时无法读取，请在素材管理中核对文件或重新定位。')
  } finally { if (revision === sourceRevision) loading.value = false }
}
function fail(message: string) {
  pause(); error.value = message; emit('error')
}
async function play(start?: number) {
  const element = media.value
  if (!available.value || !element) return false
  const revision = ++playRevision
  error.value = ''; playPending.value = true
  try {
    if (start !== undefined) element.currentTime = start
    else if (element.ended) element.currentTime = 0
    await element.play()
    if (revision !== playRevision) return false
    playing.value = true
    return true
  } catch {
    if (revision === playRevision) fail('音频无法播放，请核对本地文件；格式不受支持时可重新导入其他格式。')
    return false
  } finally { if (revision === playRevision) playPending.value = false }
}
function onPlay() {
  playing.value = true; waiting.value = false
  emit('play')
}
function exclusivePlayback(event: globalThis.Event) {
  const other = event.target
  if (other instanceof globalThis.HTMLMediaElement && other !== media.value && !other.paused) pause()
}
function onPause() {
  // Ignore a queued pause event after a newer playback has already resumed.
  if (!media.value?.paused) return
  playing.value = false; waiting.value = false; playPending.value = false
  emit('pause')
}
function metadata() {
  const element = media.value
  if (!element) return
  duration.value = Number.isFinite(element.duration) ? element.duration : 0
  element.volume = volume.value / 100; element.muted = muted.value; element.playbackRate = Number(rate.value)
}
function updateTime() {
  currentTime.value = media.value?.currentTime ?? 0
  emit('timeupdate')
}
function seek(values: number[] | undefined) {
  if (!available.value || !media.value || !duration.value || values?.[0] === undefined) return
  media.value.currentTime = Math.max(0, Math.min(duration.value, values[0]))
  currentTime.value = media.value.currentTime
  emit('seek'); emit('timeupdate')
}
function setVolume(values: number[] | undefined) {
  if (!media.value || values?.[0] === undefined) return
  volume.value = values[0]; muted.value = false
  media.value.volume = volume.value / 100; media.value.muted = false
}
function toggleMute() {
  if (!media.value) return
  if (!volume.value) setVolume([75])
  else { muted.value = !muted.value; media.value.muted = muted.value }
}
watch(rate, value => { if (media.value) media.value.playbackRate = Number(value) })
watch(() => [props.projectId, props.assetId], loadSource, { immediate: true })
watch(() => props.disabled, disabled => { if (disabled) pause() })
onMounted(() => globalThis.document.addEventListener('play', exclusivePlayback, true))
onBeforeUnmount(() => { sourceRevision++; pause(); globalThis.document.removeEventListener('play', exclusivePlayback, true) })
defineExpose({ media, play, pause })
</script>

<template>
  <div
    class="audio-player"
    :class="{ 'is-playing': playing && !waiting, 'has-error': error }"
    :data-theme="theme"
    role="group"
    :aria-label="ariaLabel"
    :aria-busy="loading"
  >
    <audio
      v-if="source"
      :key="source"
      ref="media"
      :src="source"
      :aria-label="ariaLabel"
      hidden
      preload="metadata"
      @loadedmetadata="metadata"
      @durationchange="metadata"
      @timeupdate="updateTime"
      @play="onPlay"
      @playing="waiting = false"
      @waiting="waiting = true"
      @canplay="waiting = false"
      @pause="onPause"
      @ended="playing = false; waiting = false; emit('ended')"
      @error="fail('音频读取失败，请核对文件，或重新导入受支持的音频格式。')"
    />
    <div class="player-heading">
      <span
        class="player-art"
        aria-hidden="true"
      ><AppIcon name="microphone" /></span>
      <div class="player-copy">
        <div class="player-title-row">
          <p
            class="player-title"
            :title="title"
          >
            {{ title }}
          </p>
          <span class="player-format">{{ format || '原声' }}</span>
        </div>
        <p class="player-subtitle">
          {{ subtitle }}
        </p>
      </div>
      <div
        class="player-state"
        role="status"
        aria-live="polite"
      >
        <span
          class="player-signal"
          aria-hidden="true"
        ><i /><i /><i /></span>
        <span>{{ status }}</span>
      </div>
    </div>
    <AudioWaveform
      :source="source"
      :audio="media"
      :current-time="currentTime"
      :duration="duration"
      :playing="playing"
      :disabled="!available"
      @seek="seek([$event])"
    />
    <div class="player-tools">
      <div class="player-skip">
        <Button
          type="button"
          variant="ghost"
          size="icon"
          class="player-control player-jump"
          :disabled="!available || !duration"
          aria-label="后退15秒"
          title="后退15秒"
          @click="seek([currentTime - 15])"
        >
          <AppIcon name="rewind" /><span aria-hidden="true">15</span>
        </Button>
        <Button
          type="button"
          size="icon"
          class="player-play"
          :disabled="!available"
          :aria-label="playing || playPending ? '暂停音频' : '播放音频'"
          :aria-pressed="playing"
          :aria-busy="playPending"
          @click="playing || playPending ? pause() : play()"
        >
          <AppIcon :name="playing || playPending ? 'pause' : 'play'" />
        </Button>
        <Button
          type="button"
          variant="ghost"
          size="icon"
          class="player-control player-jump"
          :disabled="!available || !duration"
          aria-label="前进15秒"
          title="前进15秒"
          @click="seek([currentTime + 15])"
        >
          <AppIcon name="forward" /><span aria-hidden="true">15</span>
        </Button>
      </div>
      <div class="player-preferences">
        <Button
          type="button"
          variant="ghost"
          size="icon"
          class="player-control player-theme"
          aria-label="切换播放器配色"
          :aria-pressed="theme === 'oled'"
          :title="theme === 'emerald' ? '科技蓝黑 · 切换 OLED 暗黑' : 'OLED 暗黑 · 切换科技蓝黑'"
          @click="theme = theme === 'emerald' ? 'oled' : 'emerald'"
        >
          <AppIcon name="palette" />
        </Button>
        <Select
          v-model="rate"
          :disabled="!available"
        >
          <SelectTrigger
            :id="`audio-speed-${controlId}`"
            class="player-speed"
            aria-label="播放倍速"
          >
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem
              v-for="speed in ['0.75', '1', '1.25', '1.5', '1.75', '2']"
              :key="speed"
              :value="speed"
            >
              {{ speed }}×
            </SelectItem>
          </SelectContent>
        </Select>
        <div class="player-volume">
          <Button
            type="button"
            variant="ghost"
            size="icon"
            class="player-control"
            :disabled="!available"
            :aria-label="silent ? '取消静音' : '静音'"
            :aria-pressed="silent"
            :title="silent ? '取消静音' : '静音'"
            @click="toggleMute"
          >
            <AppIcon :name="silent ? 'volumeOff' : 'volume'" />
          </Button>
          <Slider
            class="player-volume-slider"
            :model-value="[silent ? 0 : volume]"
            :max="100"
            :step="1"
            :disabled="!available"
            aria-label="音量"
            :aria-value-text="`${silent ? 0 : volume}%`"
            @update:model-value="setVolume"
          />
        </div>
      </div>
    </div>
    <div
      v-if="error"
      class="player-error"
      role="alert"
    >
      <p>{{ error }}</p>
      <Button
        type="button"
        variant="ghost"
        class="player-retry"
        :disabled="loading || disabled"
        @click="loadSource"
      >
        <AppIcon name="refresh" />重新加载
      </Button>
    </div>
  </div>
</template>

<style scoped>
.audio-player { --player-surface: #020617; --player-text: #ffffff; --player-muted: #94a3b8; --player-accent: #00ff9d; --player-accent-light: #00e5ff; --player-wave: #334155; --player-line: #1e293b; --player-glow: #00ff9d59; position: relative; isolation: isolate; container: audio-player / inline-size; min-width: 0; padding: 20px; border: 1px solid var(--player-line); border-radius: 16px; color: var(--player-text); background: linear-gradient(135deg, #020617, #0f172a, #020617); }
.audio-player[data-theme='oled'] { --player-surface: #000000; --player-accent: #00e5ff; --player-accent-light: #00e5ff; --player-glow: #00e5ff59; background: var(--player-surface); }
.player-heading { display: flex; align-items: center; gap: 12px; margin-bottom: 16px; min-width: 0; }
.player-art { display: grid; place-items: center; flex: 0 0 40px; height: 40px; border: 1px solid #00ff9d66; border-radius: 12px; color: var(--player-accent); background: #00ff9d26; }
.player-art .app-icon { width: 20px; height: 20px; stroke-width: 2; }
.player-copy { flex: 1; min-width: 0; }
.player-title-row { display: flex; align-items: center; gap: 8px; min-width: 0; }
.player-title { margin: 0; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: var(--player-text); font-size: 14px; font-weight: 700; line-height: 20px; }
.player-format { flex-shrink: 0; padding: 0 6px; color: var(--player-muted); background: #1e293b; border-radius: 4px; font: 400 10px/15px var(--font-mono); }
.player-subtitle { margin: 0; color: var(--player-muted); font: 400 11px/16.5px var(--font-mono); }
.player-state { display: flex; align-items: center; gap: 6px; flex-shrink: 0; color: var(--player-accent); font: 400 12px/16px var(--font-mono); }
.player-signal { width: 8px; height: 8px; border-radius: 50%; background: var(--player-accent); }
.player-signal i { display: none; }
.player-tools { display: flex; flex-wrap: wrap; align-items: center; justify-content: space-between; gap: 16px; padding-top: 12px; border-top: 1px solid var(--player-line); }
.player-skip, .player-preferences, .player-volume { display: flex; align-items: center; gap: 12px; min-width: 0; }
.player-play { width: 40px; height: 40px; flex-shrink: 0; padding: 0; border: 0; border-radius: 50%; background: var(--player-accent); color: #020617; box-shadow: 0 0 20px -3px var(--player-glow); transition: background-color 150ms, transform 150ms; }
.player-play:hover { background: color-mix(in srgb, var(--player-accent) 85%, white); }
.player-play:active { transform: scale(.95); }
.player-play .app-icon { width: 20px; height: 20px; fill: currentColor; stroke: none; }
.player-control { width: 32px; height: 32px; padding: 8px; flex-shrink: 0; border-radius: 8px; background: transparent; color: var(--player-muted); }
.player-control:hover, .player-retry:hover { background: #1e293b; color: var(--player-text); }
.player-jump .app-icon { width: 16px; height: 16px; }
.player-jump span { display: none; }
.player-theme { width: 24px; height: 24px; padding: 4px; }
.player-speed { width: 64px; height: 24px; min-height: 24px; padding: 4px 8px; border: 1px solid #334155cc; border-radius: 8px; background: #0f172a; color: var(--player-text); font: 400 11px/16px var(--font-mono); box-shadow: none; }
.player-speed:hover { background: #1e293b; }
.player-speed :deep(svg) { width: 10px; height: 10px; color: var(--player-accent); }
.player-volume { gap: 8px; padding: 0; height: 32px; }
.player-volume .player-control { width: 24px; height: 24px; padding: 4px; }
.player-volume-slider { width: 80px; height: 32px; }
.player-volume-slider :deep([data-slot='slider-track']) { height: 4px; background: #334155; }
.player-volume-slider :deep([data-slot='slider-range']) { background: var(--player-accent); }
.player-volume-slider :deep([data-slot='slider-thumb']) { width: 10px; height: 10px; border: 0; background: var(--player-text); box-shadow: none; }
.audio-player :deep(:focus-visible) { outline: 2px solid var(--player-accent-light); outline-offset: 3px; box-shadow: none; }
.player-error { display: flex; align-items: center; flex-wrap: wrap; gap: 8px 16px; margin-top: 16px; padding-top: 12px; border-top: 1px solid var(--player-line); }
.player-error p { flex: 1 1 240px; margin: 0; color: var(--player-text); font-size: 12px; }
.player-retry { color: var(--player-accent-light); padding: 0 10px; font-size: 12px; }
@container audio-player (max-width: 430px) { .player-heading { flex-wrap: wrap; gap: 8px 12px; } .player-state { margin-left: 52px; } .player-preferences { flex: 1 1 100%; justify-content: flex-end; } .player-volume-slider { width: 64px; } }
@container audio-player (max-width: 280px) { .player-title { font-size: 12px; } .player-preferences { gap: 8px; } .player-theme { width: 20px; } .player-volume-slider { width: 40px; } }
@media (max-width: 520px) { .audio-player { padding: 16px; } }
@media (pointer: coarse) { .player-control, .player-speed, .player-theme { min-width: 44px; min-height: 44px; } }
@media (prefers-reduced-motion: reduce) { .audio-player *, .audio-player :deep(*) { animation: none !important; transition: none !important; } }
</style>
