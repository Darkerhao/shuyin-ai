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
it('reports a playback failure without changing text or starting another playback', async () => {
  vi.spyOn(HTMLMediaElement.prototype, 'play').mockRejectedValue(new Error('missing'))
  vi.spyOn(HTMLMediaElement.prototype, 'pause').mockImplementation(() => {})
  const wrapper = render()
  await wrapper.get('[aria-label="试听第1句"]').trigger('click'); await flushPromises()
  expect(wrapper.get('[role="alert"]').text()).toContain('原声无法播放')
  expect(wrapper.getComponent(VideoSubtitleEditor).props('modelValue')).toBe(srt)
})
