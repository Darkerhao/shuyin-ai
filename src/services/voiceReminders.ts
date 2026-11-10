import { readonly, ref } from 'vue'
import phrases from '../assets/voice/phrases.json'

export type ReminderKind = keyof typeof phrases
export interface WorkflowCompletion { id: string; kind: ReminderKind }
const clips = import.meta.glob<string>('../assets/voice/*.wav', { eager: true, query: '?url', import: 'default' })
const storageKey = 'shuyin.voice-reminders'
const enabled = ref(false)
const playing = ref(false)
const error = ref('')
const received = new Set<string>()
const sources = new Set<globalThis.AudioBufferSourceNode>()
let context: globalThis.AudioContext | undefined
let pending = Promise.resolve()
let activation = Promise.resolve()
let nextStart = 0
let epoch = 0

export function initializeVoiceReminders() {
  try {
    enabled.value = globalThis.localStorage.getItem(storageKey) === 'true'
    error.value = ''
  } catch {
    enabled.value = false
    error.value = '语音提醒偏好读取失败，请重新开启。'
  }
}

export function activateVoiceReminders() {
  if (!enabled.value) return
  try {
    context ??= new globalThis.AudioContext()
    if (context.state !== 'running') {
      activation = context.resume().catch(() => { error.value = '语音提醒暂不可用，请点击试听重试。' })
    }
  } catch {
    error.value = '语音提醒暂不可用，请检查音频设备后重试。'
  }
}

export function stopVoiceReminders() {
  epoch += 1
  for (const source of sources) { source.stop(); source.disconnect() }
  sources.clear()
  playing.value = false
  nextStart = 0
  pending = Promise.resolve()
}

export function playVoiceReminder(kind: ReminderKind) {
  if (!enabled.value || !Object.hasOwn(phrases, kind)) return
  const requestedEpoch = epoch
  pending = pending.then(async () => {
    if (!enabled.value || requestedEpoch !== epoch) return
    await activation
    const audio = context
    if (!audio || audio.state !== 'running') throw new Error('audio_unavailable')
    const response = await globalThis.fetch(clips[`../assets/voice/${kind}.wav`]!)
    if (!response.ok) throw new Error('clip_unavailable')
    const buffer = await audio.decodeAudioData(await response.arrayBuffer())
    if (!enabled.value || requestedEpoch !== epoch) return
    const source = audio.createBufferSource()
    source.buffer = buffer
    source.connect(audio.destination)
    source.onended = () => { sources.delete(source); source.disconnect(); playing.value = sources.size > 0 }
    const start = Math.max(audio.currentTime, nextStart)
    source.start(start)
    sources.add(source)
    playing.value = true
    nextStart = start + buffer.duration + 0.15
    error.value = ''
  }).catch(() => {
    if (enabled.value && requestedEpoch === epoch) error.value = '语音提醒未能播放，请点击试听并检查系统音量；任务成果已保留。'
  })
}

export function receiveWorkflowCompletion(completion: WorkflowCompletion) {
  if (!completion || !completion.id || !Object.hasOwn(phrases, completion.kind)) return
  const key = `${completion.kind}:${completion.id}`
  if (received.has(key)) return
  received.add(key)
  playVoiceReminder(completion.kind)
}

export function setVoiceReminders(value: boolean) {
  try {
    globalThis.localStorage.setItem(storageKey, String(value))
    enabled.value = value
    error.value = ''
    if (value) {
      activateVoiceReminders()
      playVoiceReminder('enabled')
    } else stopVoiceReminders()
  } catch {
    error.value = '语音提醒设置未保存，请重试。'
  }
}

export function previewVoiceReminder() {
  activateVoiceReminders()
  playVoiceReminder('enabled')
}

export const voiceReminders = { enabled: readonly(enabled), playing: readonly(playing), error: readonly(error) }
