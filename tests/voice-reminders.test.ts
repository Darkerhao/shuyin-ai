import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import phrases from '../src/assets/voice/phrases.json'
import type { WorkflowCompletion } from '../src/services/voiceReminders'

const events = vi.hoisted(() => ({ listen: vi.fn(), unlisten: vi.fn() }))
vi.mock('@tauri-apps/api/core', () => ({ isTauri: () => true }))
vi.mock('@tauri-apps/api/event', () => ({ listen: events.listen }))
const sources: { start: ReturnType<typeof vi.fn>; stop: ReturnType<typeof vi.fn>; connect: ReturnType<typeof vi.fn>; disconnect: ReturnType<typeof vi.fn>; onended: (() => void) | null; buffer: unknown }[] = []
const audioContexts: FakeAudioContext[] = []
class FakeAudioContext {
  state = 'suspended'
  currentTime = 10
  destination = {}
  resume = vi.fn(async () => { this.state = 'running' })
  decodeAudioData = vi.fn(async () => ({ duration: 3 }))
  constructor() { audioContexts.push(this) }
  createBufferSource() {
    const source = { start: vi.fn(), stop: vi.fn(), connect: vi.fn(), disconnect: vi.fn(), onended: null, buffer: null }
    sources.push(source)
    return source
  }
}
let service: typeof import('../src/services/voiceReminders')
beforeEach(async () => {
  vi.resetModules()
  vi.clearAllMocks()
  localStorage.clear()
  sources.length = 0
  audioContexts.length = 0
  vi.stubGlobal('AudioContext', FakeAudioContext)
  vi.stubGlobal('fetch', vi.fn(async () => ({ ok: true, arrayBuffer: async () => new ArrayBuffer(8) })))
  events.listen.mockResolvedValue(events.unlisten)
  service = await import('../src/services/voiceReminders')
  service.initializeVoiceReminders()
})
afterEach(() => { service.stopVoiceReminders(); vi.unstubAllGlobals(); vi.restoreAllMocks() })

it('starts silent, persists the switch and never replays completions received while disabled', async () => {
  service.receiveWorkflowCompletion({ id: 'old', kind: 'writing' })
  await flushPromises()
  expect(fetch).not.toHaveBeenCalled()
  service.setVoiceReminders(true)
  await flushPromises()
  expect(localStorage.getItem('shuyin.voice-reminders')).toBe('true')
  expect(sources).toHaveLength(1)
  service.initializeVoiceReminders()
  expect(service.voiceReminders.enabled.value).toBe(true)
  service.receiveWorkflowCompletion({ id: 'old', kind: 'writing' })
  await flushPromises()
  expect(sources).toHaveLength(1)
  service.setVoiceReminders(false)
  service.initializeVoiceReminders()
  expect(service.voiceReminders.enabled.value).toBe(false)
})

it('queues different completions in order without overlapping and deduplicates repeated events', async () => {
  service.setVoiceReminders(true)
  service.receiveWorkflowCompletion({ id: 'book-task', kind: 'audio' })
  service.receiveWorkflowCompletion({ id: 'book-task', kind: 'audio' })
  service.receiveWorkflowCompletion({ id: 'book-task', kind: 'video' })
  service.receiveWorkflowCompletion({ id: 'another-book', kind: 'audio' })
  await flushPromises()
  expect(sources).toHaveLength(4)
  for (const [index, start] of [10, 13.15, 16.3, 19.45].entries()) {
    expect(sources[index]!.start.mock.calls[0]![0]).toBeCloseTo(start)
  }
  expect(vi.mocked(fetch).mock.calls.map(call => String(call[0]))).toEqual([
    expect.stringContaining('enabled.wav'), expect.stringContaining('audio.wav'), expect.stringContaining('video.wav'), expect.stringContaining('audio.wav'),
  ])
})

it('stops both playing and scheduled audio immediately when disabled', async () => {
  service.setVoiceReminders(true)
  service.receiveWorkflowCompletion({ id: 'render', kind: 'video' })
  await flushPromises()
  service.setVoiceReminders(false)
  expect(sources).toHaveLength(2)
  expect(sources.every(source => source.stop.mock.calls.length === 1)).toBe(true)
  service.receiveWorkflowCompletion({ id: 'late', kind: 'audio' })
  await flushPromises()
  expect(sources).toHaveLength(2)
})

it('announces each exported package while ignoring repeated delivery of the same export', async () => {
  service.receiveWorkflowCompletion({ id: 'book/publish/package-1', kind: 'publish-package' })
  service.setVoiceReminders(true)
  await flushPromises()
  service.receiveWorkflowCompletion({ id: 'book/publish/package-2', kind: 'publish-package' })
  service.receiveWorkflowCompletion({ id: 'book/publish/package-2', kind: 'publish-package' })
  service.receiveWorkflowCompletion({ id: 'book/publish/package-3', kind: 'publish-package' })
  await flushPromises()
  expect(sources).toHaveLength(3)
  expect(vi.mocked(fetch).mock.calls.map(call => String(call[0]))).toEqual([
    expect.stringContaining('enabled.wav'), expect.stringContaining('publish-package.wav'), expect.stringContaining('publish-package.wav'),
  ])
})

it('discards in-flight clip decoding after disabling, even if enabled again before decoding finishes', async () => {
  service.setVoiceReminders(true)
  await flushPromises()
  let finish!: (value: { duration: number }) => void
  audioContexts[0]!.decodeAudioData.mockImplementationOnce(() => new Promise(resolve => { finish = resolve }))
  service.receiveWorkflowCompletion({ id: 'slow-clip', kind: 'writing' })
  await flushPromises()
  service.setVoiceReminders(false)
  service.setVoiceReminders(true)
  await flushPromises()
  finish({ duration: 3 })
  await flushPromises()
  expect(sources).toHaveLength(2)
})

it('keeps the saved switch state on storage failure and reports the failure', () => {
  vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => { throw new Error('disk unavailable') })
  service.setVoiceReminders(true)
  expect(service.voiceReminders.enabled.value).toBe(false)
  expect(service.voiceReminders.error.value).toContain('未保存')
})

it('reports unavailable audio without rejecting the completion or blocking later reminders', async () => {
  service.setVoiceReminders(true)
  await flushPromises()
  vi.mocked(fetch).mockRejectedValueOnce(new Error('file unavailable'))
  expect(() => service.receiveWorkflowCompletion({ id: 'missing', kind: 'preview' })).not.toThrow()
  await flushPromises()
  expect(service.voiceReminders.error.value).toContain('任务成果已保留')
  service.receiveWorkflowCompletion({ id: 'next', kind: 'video' })
  await flushPromises()
  expect(sources).toHaveLength(2)
  expect(service.voiceReminders.error.value).toBe('')
})

it('uses the next user interaction to activate audio after restoring an enabled preference', async () => {
  localStorage.setItem('shuyin.voice-reminders', 'true')
  service.initializeVoiceReminders()
  service.activateVoiceReminders()
  service.receiveWorkflowCompletion({ id: 'restored-session', kind: 'storyboard' })
  await flushPromises()
  expect(audioContexts[0]!.resume).toHaveBeenCalledOnce()
  expect(sources).toHaveLength(1)
})

it('ignores unknown event types and does not speak arbitrary event text', async () => {
  service.setVoiceReminders(true)
  service.receiveWorkflowCompletion({ id: 'unknown', kind: 'failed' } as unknown as WorkflowCompletion)
  await flushPromises()
  expect(sources).toHaveLength(1)
})

it('keeps a single global event listener with a labelled switch and preview control', async () => {
  const Toggle = (await import('../src/components/VoiceReminderToggle.vue')).default
  const wrapper = mount(Toggle)
  await flushPromises()
  expect(events.listen).toHaveBeenCalledOnce()
  expect(events.listen.mock.calls[0]![0]).toBe('workflow-completed')
  expect(wrapper.get('[role="switch"]').attributes('aria-checked')).toBe('false')
  await wrapper.get('[role="switch"]').trigger('click')
  await flushPromises()
  expect(wrapper.get('[role="switch"]').attributes('aria-checked')).toBe('true')
  expect(wrapper.find('[aria-label="试听语音提醒"]').exists()).toBe(true)
  const callback = events.listen.mock.calls[0]![1] as (event: { payload: WorkflowCompletion }) => void
  callback({ payload: { id: 'background', kind: 'audio' } })
  await flushPromises()
  expect(sources).toHaveLength(2)
  wrapper.unmount()
  expect(events.unlisten).toHaveBeenCalledOnce()
  expect(sources.every(source => source.stop.mock.calls.length === 1)).toBe(true)
})

it('cleans up a listener whose registration finishes after the control unmounts', async () => {
  let finish!: (stop: () => void) => void
  events.listen.mockImplementationOnce(() => new Promise(resolve => { finish = resolve }))
  const Toggle = (await import('../src/components/VoiceReminderToggle.vue')).default
  const wrapper = mount(Toggle)
  wrapper.unmount()
  finish(events.unlisten)
  await flushPromises()
  expect(events.unlisten).toHaveBeenCalledOnce()
})

it('ships a playable, nonempty PCM recording for every phrase', () => {
  for (const kind of Object.keys(phrases)) {
    const clip = readFileSync(resolve('src/assets/voice', `${kind}.wav`))
    expect(clip.subarray(0, 4).toString()).toBe('RIFF')
    expect(clip.subarray(8, 12).toString()).toBe('WAVE')
    expect(clip.length).toBeGreaterThan(44000)
  }
})
