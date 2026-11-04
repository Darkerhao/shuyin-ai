import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import AudioPlayer from '../src/components/AudioPlayer.vue'
import { mediaSource } from '../src/services/projects'
import { chooseSelect, mockResizeObserver } from './ui'

vi.mock('../src/services/projects', () => ({ mediaSource: vi.fn() }))
vi.mock('../src/services/audioWaveform', () => ({ loadWaveform: vi.fn().mockResolvedValue([.2, .6, .8, .1]) }))
const mounted: ReturnType<typeof mount>[] = []
beforeEach(() => { mockResizeObserver(); vi.mocked(mediaSource).mockReset().mockResolvedValue('http://asset.localhost/audio.m4a') })
afterEach(() => { mounted.splice(0).forEach(wrapper => wrapper.unmount()); vi.restoreAllMocks(); vi.unstubAllGlobals() })
async function render(assetId = 'a1') {
  const wrapper = mount(AudioPlayer, { props: { projectId: 'p1', assetId, title: '钢铁是怎样炼成的' }, attachTo: document.body })
  mounted.push(wrapper); await flushPromises()
  return wrapper
}
function playback(wrapper: Awaited<ReturnType<typeof render>>, seconds = 663) {
  const audio = wrapper.get('audio').element as HTMLAudioElement
  let paused = true
  vi.spyOn(audio, 'duration', 'get').mockReturnValue(seconds)
  vi.spyOn(audio, 'paused', 'get').mockImplementation(() => paused)
  const play = vi.spyOn(audio, 'play').mockImplementation(async () => { paused = false; audio.dispatchEvent(new Event('play')) })
  const pause = vi.spyOn(audio, 'pause').mockImplementation(() => { paused = true; audio.dispatchEvent(new Event('pause')) })
  return { audio, play, pause }
}

it('resolves only the registered local asset and waits for a user click to play', async () => {
  const wrapper = await render()
  const { play } = playback(wrapper)
  expect(mediaSource).toHaveBeenCalledExactlyOnceWith('p1', 'a1')
  expect(play).not.toHaveBeenCalled()
  expect(wrapper.get('audio').attributes('controls')).toBeUndefined()
  expect(wrapper.get('audio').attributes('hidden')).toBeDefined()
  await wrapper.get('audio').trigger('loadedmetadata')
  expect(wrapper.text()).toContain('11:03')
  await wrapper.get('[aria-label="播放音频"]').trigger('click'); await flushPromises()
  expect(play).toHaveBeenCalledOnce()
  expect(wrapper.get('[aria-label="暂停音频"]').attributes('aria-pressed')).toBe('true')
  await wrapper.get('[aria-label="暂停音频"]').trigger('click')
  expect(wrapper.find('[aria-label="播放音频"]').exists()).toBe(true)
})

it('seeks and skips using media time, clamps both ends and exposes a readable clock to keyboards and screen readers', async () => {
  const wrapper = await render()
  const { audio } = playback(wrapper)
  await wrapper.get('audio').trigger('loadedmetadata')
  const progress = wrapper.get('[role="slider"][aria-label="播放进度"]')
  await progress.trigger('keydown', { key: 'ArrowRight' })
  expect(audio.currentTime).toBe(.1)
  expect(progress.attributes('aria-valuetext')).toBe('0:00 / 11:03')
  await progress.trigger('keydown', { key: 'End' })
  expect(audio.currentTime).toBe(663)
  await wrapper.get('[aria-label="前进15秒"]').trigger('click')
  expect(audio.currentTime).toBe(663)
  await wrapper.get('[aria-label="后退15秒"]').trigger('click')
  expect(audio.currentTime).toBe(648)
  await progress.trigger('keydown', { key: 'Home' })
  await wrapper.get('[aria-label="后退15秒"]').trigger('click')
  expect(audio.currentTime).toBe(0)
})

it('changes actual playback speed, volume and mute state through the custom controls', async () => {
  const wrapper = await render()
  const { audio } = playback(wrapper)
  await wrapper.get('audio').trigger('loadedmetadata')
  await chooseSelect(wrapper, '[aria-label="播放倍速"]', '1.5×')
  expect(audio.playbackRate).toBe(1.5)
  const volume = wrapper.get('[role="slider"][aria-label="音量"]')
  await volume.trigger('keydown', { key: 'End' })
  expect(audio.volume).toBe(1)
  await wrapper.get('[aria-label="静音"]').trigger('click')
  expect(audio.muted).toBe(true)
  await wrapper.get('[aria-label="取消静音"]').trigger('click')
  expect(audio.muted).toBe(false)
  await volume.trigger('keydown', { key: 'Home' })
  expect(audio.volume).toBe(0)
  await wrapper.get('[aria-label="取消静音"]').trigger('click')
  expect(audio.volume).toBe(0.75)
})

it('pauses another player when starting and stops playback on source change, disable and unmount', async () => {
  const first = await render('a1'); const one = playback(first)
  const second = await render('a2'); const two = playback(second)
  await first.get('[aria-label="播放音频"]').trigger('click'); await flushPromises()
  await second.get('[aria-label="播放音频"]').trigger('click'); await flushPromises()
  expect(one.pause).toHaveBeenCalledOnce()
  expect(first.find('[aria-label="播放音频"]').exists()).toBe(true)
  await second.setProps({ disabled: true })
  expect(two.pause).toHaveBeenCalledOnce()
  await first.get('[aria-label="播放音频"]').trigger('click'); await flushPromises()
  await first.setProps({ assetId: 'a3' }); await flushPromises()
  expect(one.pause).toHaveBeenCalledTimes(2)
  const three = playback(first)
  await first.get('[aria-label="播放音频"]').trigger('click'); await flushPromises()
  first.unmount()
  expect(three.pause).toHaveBeenCalledOnce()
})

it('shows source failure and retries only on request without replacing another selected asset with a late response', async () => {
  vi.mocked(mediaSource).mockRejectedValueOnce(new Error('missing'))
  const wrapper = await render()
  expect(wrapper.get('[role="alert"]').text()).toContain('重新定位')
  expect(mediaSource).toHaveBeenCalledOnce()
  expect(wrapper.get('[aria-label="播放音频"]').attributes('disabled')).toBeDefined()
  await wrapper.get('.player-retry').trigger('click'); await flushPromises()
  expect(wrapper.find('audio').exists()).toBe(true)
  let finish!: (url: string) => void
  vi.mocked(mediaSource).mockReturnValueOnce(new Promise(resolve => { finish = resolve }))
  await wrapper.setProps({ assetId: 'late' })
  vi.mocked(mediaSource).mockResolvedValueOnce('http://asset.localhost/new.m4a')
  await wrapper.setProps({ assetId: 'new' }); await flushPromises()
  finish('http://asset.localhost/late.m4a'); await flushPromises()
  expect(wrapper.get('audio').attributes('src')).toBe('http://asset.localhost/new.m4a')
})

it('reports playback rejection, permits replay after ending and preserves a new playback across delayed pause events', async () => {
  const wrapper = await render()
  const { audio, play } = playback(wrapper)
  play.mockRejectedValueOnce(new Error('unsupported'))
  await wrapper.get('[aria-label="播放音频"]').trigger('click'); await flushPromises()
  expect(wrapper.get('[role="alert"]').text()).toContain('音频无法播放')
  expect(play).toHaveBeenCalledOnce()
  await wrapper.get('[aria-label="播放音频"]').trigger('click'); await flushPromises()
  await wrapper.get('audio').trigger('pause') // Old queued event; this audio is already playing.
  expect(wrapper.find('[aria-label="暂停音频"]').exists()).toBe(true)
  vi.spyOn(audio, 'ended', 'get').mockReturnValue(true)
  audio.currentTime = 663
  await wrapper.get('audio').trigger('ended')
  await wrapper.get('[aria-label="播放音频"]').trigger('click'); await flushPromises()
  expect(audio.currentTime).toBe(0)
})

it('pauses the original audio when a video preview starts and keeps playback unchanged when switching themes', async () => {
  const wrapper = await render()
  const { audio, pause } = playback(wrapper)
  await wrapper.get('[aria-label="播放音频"]').trigger('click'); await flushPromises()
  await wrapper.get('[aria-label="切换播放器配色"]').trigger('click')
  expect(wrapper.attributes('data-theme')).toBe('oled')
  expect(audio.paused).toBe(false)
  const video = document.createElement('video')
  document.body.appendChild(video)
  vi.spyOn(video, 'paused', 'get').mockReturnValue(false)
  video.dispatchEvent(new Event('play'))
  expect(pause).toHaveBeenCalledOnce()
  video.remove()
})
