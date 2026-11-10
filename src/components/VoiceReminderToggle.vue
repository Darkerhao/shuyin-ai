<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue'
import { isTauri } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { Button } from '@/components/ui/button'
import AppIcon from './AppIcon.vue'
import OperationFeedback from './OperationFeedback.vue'
import { activateVoiceReminders, initializeVoiceReminders, previewVoiceReminder, receiveWorkflowCompletion, setVoiceReminders, stopVoiceReminders, voiceReminders, type WorkflowCompletion } from '../services/voiceReminders'

const { enabled, playing, error } = voiceReminders
const listenerError = ref('')
let unlisten: UnlistenFn | undefined
let disposed = false
initializeVoiceReminders()
onMounted(async () => {
  globalThis.addEventListener('pointerdown', activateVoiceReminders)
  globalThis.addEventListener('keydown', activateVoiceReminders)
  if (!isTauri()) return
  try {
    const stop = await listen<WorkflowCompletion>('workflow-completed', event => receiveWorkflowCompletion(event.payload))
    if (disposed) stop()
    else unlisten = stop
  } catch { listenerError.value = '语音提醒连接失败，请重新打开应用；任务结果仍可在原页面查看。' }
})
onBeforeUnmount(() => {
  disposed = true
  unlisten?.()
  globalThis.removeEventListener('pointerdown', activateVoiceReminders)
  globalThis.removeEventListener('keydown', activateVoiceReminders)
  stopVoiceReminders()
})
</script>

<template>
  <div class="flex shrink-0 items-center gap-1">
    <Button
      variant="outline"
      class="theme-toggle voice-toggle"
      :class="{ 'is-playing': playing }"
      role="switch"
      aria-label="语音提醒"
      :aria-checked="enabled"
      :title="playing ? '正在播放语音提醒，点击关闭并停止' : enabled ? '语音提醒已开启，点击关闭' : '语音提醒已关闭，点击开启'"
      @click="setVoiceReminders(!enabled)"
    >
      <span
        :key="playing ? 'playing' : enabled ? 'enabled' : 'disabled'"
        class="voice-icon"
        aria-hidden="true"
      >
        <span
          v-if="playing"
          class="voice-wave"
        ><i /><i /><i /><i /></span>
        <AppIcon
          v-else
          :name="enabled ? 'volume' : 'volumeOff'"
        />
      </span>
      <span>语音提醒{{ enabled ? '开' : '关' }}</span>
    </Button>
    <Button
      v-if="enabled"
      variant="ghost"
      size="icon-sm"
      class="voice-preview"
      :class="{ 'is-playing': playing }"
      :aria-label="playing ? '停止语音播放' : '试听语音提醒'"
      :title="playing ? '停止本轮语音，保留提醒开关' : '试听语音提醒'"
      @click="playing ? stopVoiceReminders() : previewVoiceReminder()"
    >
      <span
        :key="String(playing)"
        class="voice-icon"
        aria-hidden="true"
      ><AppIcon :name="playing ? 'stop' : 'play'" /></span>
    </Button>
    <span
      class="sr-only"
      role="status"
    >{{ playing ? '正在播放语音提醒' : '' }}</span>
    <OperationFeedback :error="listenerError || error" />
  </div>
</template>

<style scoped>
.voice-toggle, .voice-preview { transition: color 180ms ease-out, background-color 180ms ease-out, border-color 180ms ease-out, box-shadow 180ms ease-out, transform 180ms ease-out; }
.voice-toggle[aria-checked='true'], .voice-preview.is-playing { color: var(--primary); border-color: var(--active-border); background: var(--accent); }
.voice-toggle.is-playing { box-shadow: 0 0 0 3px color-mix(in srgb, var(--primary) 10%, transparent); }
.voice-toggle:focus-visible, .voice-preview:focus-visible { outline: 2px solid var(--info); outline-offset: 3px; }
.voice-toggle:active, .voice-preview:active { transform: scale(.96); transition-duration: 90ms; }
.voice-icon { display: inline-grid; place-items: center; width: 16px; height: 16px; flex-shrink: 0; animation: voice-icon-in 180ms ease-out; }
.voice-wave { display: flex; align-items: center; justify-content: center; gap: 2px; width: 16px; height: 16px; }
.voice-wave i { width: 2px; height: 12px; border-radius: 2px; background: currentColor; animation: voice-wave 800ms ease-in-out infinite alternate; }
.voice-wave i:nth-child(1) { height: 7px; animation-delay: -300ms; }
.voice-wave i:nth-child(2) { animation-delay: -600ms; }
.voice-wave i:nth-child(3) { height: 15px; animation-delay: -100ms; }
.voice-wave i:nth-child(4) { height: 9px; animation-delay: -450ms; }
@keyframes voice-icon-in { from { opacity: 0; transform: scale(.8); } to { opacity: 1; transform: scale(1); } }
@keyframes voice-wave { from { transform: scaleY(.4); } to { transform: scaleY(1); } }
@media (hover: hover) {
  .voice-toggle:hover, .voice-preview:hover { transform: translateY(-1px); }
  .voice-toggle:active, .voice-preview:active { transform: scale(.96); }
}
@media (prefers-reduced-motion: reduce) {
  .voice-toggle, .voice-preview { transition: none; }
  .voice-toggle:is(:hover, :active), .voice-preview:is(:hover, :active) { transform: none; }
  .voice-icon, .voice-wave i { animation: none; }
}
</style>
