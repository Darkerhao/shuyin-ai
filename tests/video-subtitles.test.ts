import { afterEach, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { defineComponent, ref } from 'vue'
import VideoSubtitleEditor from '../src/modules/video/VideoSubtitleEditor.vue'
import { initialPosition, replaceSubtitleText, subtitleRows } from '../src/modules/video/subtitles'

const srt = '1\r\n00:00:01,200 --> 00:00:02,800\r\n孔字\r\n\r\n2\r\n00:00:03,000 --> 00:00:05,000\r\n原声\r\n'
const originals = [{ start: 1.2, end: 2.8, text: '孔子' }, { start: 3, end: 5, text: '原声' }]
const mounted: ReturnType<typeof mount>[] = []
afterEach(() => { mounted.splice(0).forEach(wrapper => wrapper.unmount()); vi.restoreAllMocks(); vi.useRealTimers() })
function render() {
  const wrapper = mount(defineComponent({
    components: { VideoSubtitleEditor },
    setup: () => ({ text: ref(srt), position: ref(initialPosition()), originals }),
    template: '<VideoSubtitleEditor v-model="text" v-model:position="position" :originals="originals" original-error="" audio-src="http://asset.localhost/task-audio.m4a" :disabled="false" />',
  }))
  mounted.push(wrapper); return wrapper
}
it('replaces only the chosen sentence and preserves BOM, CRLF and every timecode', () => {
  const input = `\uFEFF${srt}`
  const rows = subtitleRows(input)
  expect(rows).toHaveLength(2)
  expect(rows[0]).toMatchObject({ start: 1.2, end: 2.8, text: '孔字' })
  expect(replaceSubtitleText(input, rows[0]!, '孔子')).toBe(input.replace('孔字', '孔子'))
  expect(subtitleRows(`${srt}\r\n3\r\n尚未写完`)).toEqual([])
  expect(subtitleRows(srt.replace('00:00:03,000', '00:60:03,000'))).toEqual([])
})
it('filters real differences and restores one sentence without changing another sentence or timestamps', async () => {
  const wrapper = render()
  await wrapper.findAll('button').find(button => button.text().startsWith('只看修改项'))!.trigger('click')
  expect(wrapper.findAll('.subtitle-row')).toHaveLength(1)
  await wrapper.get('[aria-label="恢复第1句原文"]').trigger('click')
  expect(wrapper.findAll('.subtitle-row')).toHaveLength(0)
  expect(wrapper.getComponent(VideoSubtitleEditor).props('modelValue')).toBe(srt.replace('孔字', '孔子'))
})
it('seeks the task audio to the cue, tolerates buffering, stops at the end and pauses on unmount', async () => {
  vi.useFakeTimers()
  const play = vi.spyOn(HTMLMediaElement.prototype, 'play').mockResolvedValue()
  const pause = vi.spyOn(HTMLMediaElement.prototype, 'pause').mockImplementation(() => {})
  const wrapper = render()
  await wrapper.get('[aria-label="试听第1句"]').trigger('click'); await flushPromises()
  const audio = wrapper.get('audio').element as HTMLAudioElement
  expect(audio.currentTime).toBe(1.2)
  expect(play).toHaveBeenCalledOnce()
  await vi.advanceTimersByTimeAsync(2500) // Buffering has not advanced media time.
  expect(pause).not.toHaveBeenCalled()
  audio.currentTime = 2.8
  await wrapper.get('audio').trigger('timeupdate')
  expect(pause).toHaveBeenCalledOnce()
  await wrapper.get('[aria-label="试听第2句"]').trigger('click'); await flushPromises()
  expect(audio.currentTime).toBe(3)
  wrapper.unmount()
  expect(pause).toHaveBeenCalledTimes(2)
})
it('keeps the new sentence boundary when an earlier playback emits a delayed pause event', async () => {
  const wrapper = render()
  const audio = wrapper.get('audio').element as HTMLAudioElement
  let paused = true
  vi.spyOn(audio, 'paused', 'get').mockImplementation(() => paused)
  const pause = vi.spyOn(audio, 'pause').mockImplementation(() => {
    paused = true
    queueMicrotask(() => audio.dispatchEvent(new Event('pause')))
  })
  vi.spyOn(audio, 'play').mockImplementation(async () => {
    paused = false
    queueMicrotask(() => audio.dispatchEvent(new Event('play')))
  })
  await wrapper.get('[aria-label="试听第1句"]').trigger('click'); await flushPromises()
  await wrapper.get('[aria-label="试听第2句"]').trigger('click'); await flushPromises()
  expect(pause).toHaveBeenCalledOnce()
  expect(audio.currentTime).toBe(3)
  audio.currentTime = 5.1
  await wrapper.get('audio').trigger('timeupdate'); await flushPromises()
  expect(pause).toHaveBeenCalledTimes(2)
  expect(wrapper.getComponent(VideoSubtitleEditor).emitted('playback')!.at(-1)).toEqual([-1, false])
  expect(wrapper.getComponent(VideoSubtitleEditor).props('modelValue')).toBe(srt)
})
it('reports a playback failure without changing text or starting another playback', async () => {
  vi.spyOn(HTMLMediaElement.prototype, 'play').mockRejectedValue(new Error('missing'))
  vi.spyOn(HTMLMediaElement.prototype, 'pause').mockImplementation(() => {})
  const wrapper = render()
  await wrapper.get('[aria-label="试听第1句"]').trigger('click'); await flushPromises()
  expect(wrapper.get('[role="alert"]').text()).toContain('原声无法播放')
  expect(wrapper.getComponent(VideoSubtitleEditor).props('modelValue')).toBe(srt)
})
it('plays continuously across cue boundaries with one audio element and preserves the saved editing position', async () => {
  vi.spyOn(HTMLMediaElement.prototype, 'play').mockResolvedValue()
  vi.spyOn(HTMLMediaElement.prototype, 'pause').mockImplementation(() => {})
  const wrapper = render()
  const editor = wrapper.getComponent(VideoSubtitleEditor)
  const before = { ...editor.props('position') }
  await wrapper.findAll('button').find(item => item.text() === '连续播放原声')!.trigger('click'); await flushPromises()
  const audio = wrapper.get('audio').element as HTMLAudioElement
  audio.currentTime = 3.5; await wrapper.get('audio').trigger('timeupdate')
  expect(wrapper.findAll('audio')).toHaveLength(1)
  expect(wrapper.get('#subtitle-row-1').classes()).toContain('subtitle-row-playing')
  expect(editor.emitted('playback')!.at(-1)).toEqual([1, false])
  expect(editor.props('position')).toEqual(before)
  expect(editor.props('modelValue')).toBe(srt)
})
it('restores the current cue indication after pausing and resuming the native player', async () => {
  vi.spyOn(HTMLMediaElement.prototype, 'play').mockResolvedValue()
  vi.spyOn(HTMLMediaElement.prototype, 'pause').mockImplementation(() => {})
  const wrapper = render()
  const editor = wrapper.getComponent(VideoSubtitleEditor)
  const audio = wrapper.get('audio').element as HTMLAudioElement
  await wrapper.get('audio').trigger('play')
  audio.currentTime = 3.5; await wrapper.get('audio').trigger('timeupdate')
  await wrapper.get('audio').trigger('pause')
  expect(editor.emitted('playback')!.at(-1)).toEqual([-1, false])
  await wrapper.get('audio').trigger('play'); await wrapper.get('audio').trigger('timeupdate')
  expect(editor.emitted('playback')!.at(-1)).toEqual([1, false])
  expect(wrapper.get('#subtitle-row-1').classes()).toContain('subtitle-row-playing')
})
it('follows playback across pages without replacing an actively edited subtitle or saving a new editing position', async () => {
  vi.spyOn(HTMLMediaElement.prototype, 'play').mockResolvedValue()
  vi.spyOn(HTMLMediaElement.prototype, 'pause').mockImplementation(() => {})
  const wrapper = render()
  const editor = wrapper.getComponent(VideoSubtitleEditor)
  const longText = Array.from({ length: 30 }, (_, index) => `${index + 1}\n00:00:${String(index).padStart(2, '0')},000 --> 00:00:${String(index + 1).padStart(2, '0')},000\n字幕${index + 1}\n`).join('\n')
  editor.vm.$emit('update:modelValue', longText); await flushPromises()
  await wrapper.findAll('button').find(item => item.text() === '跟随播放')!.trigger('click')
  const input = wrapper.get('#subtitle-text-0').element as HTMLTextAreaElement
  document.body.appendChild(wrapper.element)
  input.focus()
  const before = { ...editor.props('position') }
  const audio = wrapper.get('audio').element as HTMLAudioElement
  audio.currentTime = 25.2
  await wrapper.get('audio').trigger('play'); await wrapper.get('audio').trigger('timeupdate')
  expect(document.activeElement).toBe(input)
  expect(wrapper.find('#subtitle-text-0').exists()).toBe(true)
  expect(wrapper.find('#subtitle-text-25').exists()).toBe(false)
  expect(editor.emitted('playback')!.at(-1)).toEqual([25, false])
  input.blur()
  audio.currentTime = 26.2
  await wrapper.get('audio').trigger('timeupdate')
  expect(wrapper.find('#subtitle-text-26').exists()).toBe(true)
  expect(editor.emitted('playback')!.at(-1)).toEqual([26, true])
  expect(editor.props('position')).toEqual(before)
  expect(editor.props('modelValue')).toBe(longText)
})
