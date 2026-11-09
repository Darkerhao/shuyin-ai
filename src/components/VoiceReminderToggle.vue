<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue'
import { isTauri } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { Button } from '@/components/ui/button'
import AppIcon from './AppIcon.vue'
import OperationFeedback from './OperationFeedback.vue'
import { activateVoiceReminders, initializeVoiceReminders, previewVoiceReminder, receiveWorkflowCompletion, setVoiceReminders, stopVoiceReminders, voiceReminders, type WorkflowCompletion } from '../services/voiceReminders'

const { enabled, error } = voiceReminders
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
      class="theme-toggle"
      role="switch"
      aria-label="语音提醒"
      :aria-checked="enabled"
      :title="enabled ? '语音提醒已开启，点击关闭' : '语音提醒已关闭，点击开启'"
      @click="setVoiceReminders(!enabled)"
    >
      <AppIcon :name="enabled ? 'volume' : 'volumeOff'" />
      <span>语音提醒{{ enabled ? '开' : '关' }}</span>
    </Button>
    <Button
      v-if="enabled"
      variant="ghost"
      size="icon-sm"
      aria-label="试听语音提醒"
      title="试听语音提醒"
      @click="previewVoiceReminder"
    >
      <AppIcon name="play" />
    </Button>
    <OperationFeedback :error="listenerError || error" />
  </div>
</template>
