import { afterEach, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import AudioWaveform from '../src/components/AudioWaveform.vue'
import { audioPeaks, loadWaveform } from '../src/services/audioWaveform'
import { mediaClock } from '../src/lib/mediaTime'
import { mockResizeObserver } from './ui'

afterEach(() => { vi.restoreAllMocks(); vi.unstubAllGlobals() })
it('uses actual channel amplitudes and preserves silence instead of fabricating a decorative waveform', () => {
  const channel = Float32Array.from([0, 0, 0, 0, .5, .5, .5, .5, 1, 1, 1, 1, 0, 0, 0, 0])
  expect(audioPeaks({ length: 16, numberOfChannels: 1, getChannelData: () => channel }, 4)).toEqual([0, .5, 1, 0])
  expect(audioPeaks({ length: 16, numberOfChannels: 2, getChannelData: index => index ? channel : new Float32Array(16) }, 4)).toEqual([0, .5, 1, 0])
  expect(audioPeaks({ length: 16, numberOfChannels: 1, getChannelData: () => new Float32Array(16) }, 4)).toEqual([0, 0, 0, 0])
  expect(mediaClock(663.9)).toBe('11:03')
  expect(mediaClock(3605)).toBe('1:00:05')
})
it('shares one decode per source and decodes files sequentially at a low sample rate', async () => {
  let finish!: (buffer: AudioBuffer) => void
  const buffer = { length: 256, numberOfChannels: 1, getChannelData: () => new Float32Array(256).fill(.5) } as unknown as AudioBuffer
  const decode = vi.fn().mockReturnValueOnce(new Promise(resolve => { finish = resolve })).mockResolvedValue(buffer)
  const context = vi.fn(function () { return { decodeAudioData: decode } })
  vi.stubGlobal('OfflineAudioContext', context)
  const fetch = vi.fn().mockResolvedValue({ ok: true, arrayBuffer: async () => new ArrayBuffer(4) })
  vi.stubGlobal('fetch', fetch)
  const first = loadWaveform('http://asset.localhost/test-one.wav')
  expect(loadWaveform('http://asset.localhost/test-one.wav')).toBe(first)
  const second = loadWaveform('http://asset.localhost/test-two.wav')
  await flushPromises()
  expect(fetch).toHaveBeenCalledOnce()
  expect(context).toHaveBeenCalledWith(1, 1, 8000)
  finish(buffer)
  expect(await first).toHaveLength(256)
  expect(await second).toHaveLength(256)
  expect(fetch).toHaveBeenCalledTimes(2)
})
it('keeps media seeking usable when waveform decoding fails, and hover previews never seek by themselves', async () => {
  mockResizeObserver()
  vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ ok: false }))
  const wrapper = mount(AudioWaveform, { props: { source: 'http://asset.localhost/unreadable.wav', currentTime: 0, duration: 66, playing: false, disabled: false } })
  try {
    await flushPromises()
    expect(wrapper.attributes('data-waveform')).toBe('unavailable')
    expect(wrapper.text()).toContain('仍可拖动定位')
    const stage = wrapper.get('.waveform-stage')
    vi.spyOn(stage.element, 'getBoundingClientRect').mockReturnValue({ left: 0, width: 200 } as DOMRect)
    stage.element.dispatchEvent(new MouseEvent('pointermove', { clientX: 100, bubbles: true })); await flushPromises()
    expect(wrapper.get('.waveform-tooltip').text()).toBe('0:33')
    expect(wrapper.emitted('seek')).toBeUndefined()
    await wrapper.get('[role="slider"]').trigger('keydown', { key: 'End' })
    expect(wrapper.emitted('seek')!.at(-1)).toEqual([66])
    await stage.trigger('pointerleave')
    expect(wrapper.find('.waveform-tooltip').exists()).toBe(false)
  } finally { wrapper.unmount() }
})
