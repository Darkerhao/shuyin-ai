import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { ref } from 'vue'
import { isTauri } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { useWindowCloseGuard } from '../src/composables/useWindowCloseGuard'

vi.mock('@tauri-apps/api/core', () => ({ isTauri: vi.fn() }))
vi.mock('@tauri-apps/api/window', () => ({ getCurrentWindow: vi.fn() }))
const wrappers: ReturnType<typeof mount>[] = []
beforeEach(() => vi.resetAllMocks())
afterEach(() => wrappers.splice(0).forEach(wrapper => wrapper.unmount()))

describe('window exit protection', () => {
  it('protects browser unload only while content or operations need protection', () => {
    vi.mocked(isTauri).mockReturnValue(false)
    const protectedState = ref(true)
    const wrapper = mount({ setup() { useWindowCloseGuard(() => protectedState.value, () => true) }, template: '<p>资料</p>' })
    wrappers.push(wrapper)
    const unload = new Event('beforeunload', { cancelable: true })
    globalThis.dispatchEvent(unload)
    expect(unload.defaultPrevented).toBe(true)
    protectedState.value = false
    const clean = new Event('beforeunload', { cancelable: true })
    globalThis.dispatchEvent(clean)
    expect(clean.defaultPrevented).toBe(false)
    wrapper.unmount(); wrappers.pop()
    protectedState.value = true
    const removed = new Event('beforeunload', { cancelable: true })
    globalThis.dispatchEvent(removed)
    expect(removed.defaultPrevented).toBe(false)
  })
  it('waits for the desktop close decision and removes the listener on unmount', async () => {
    vi.mocked(isTauri).mockReturnValue(true)
    let close!: (event: { preventDefault: () => void }) => Promise<void>
    const stop = vi.fn()
    vi.mocked(getCurrentWindow).mockReturnValue({ onCloseRequested: vi.fn(async callback => { close = callback as typeof close; return stop }) } as unknown as ReturnType<typeof getCurrentWindow>)
    let decide!: (allow: boolean) => void
    const canClose = vi.fn(() => new Promise<boolean>(resolve => { decide = resolve }))
    const wrapper = mount({ setup() { useWindowCloseGuard(() => true, canClose) }, template: '<p>关闭确认</p>' })
    wrappers.push(wrapper); await flushPromises()
    const denied = { preventDefault: vi.fn() }
    const waiting = close(denied)
    decide(false); await waiting
    expect(denied.preventDefault).toHaveBeenCalledOnce()
    const allowed = { preventDefault: vi.fn() }
    const accepted = close(allowed)
    decide(true); await accepted
    expect(allowed.preventDefault).not.toHaveBeenCalled()
    wrapper.unmount(); wrappers.pop()
    expect(stop).toHaveBeenCalledOnce()
  })
  it('disposes a desktop listener that finishes registering after navigation', async () => {
    vi.mocked(isTauri).mockReturnValue(true)
    let register!: (stop: () => void) => void
    vi.mocked(getCurrentWindow).mockReturnValue({ onCloseRequested: vi.fn(() => new Promise(resolve => { register = resolve })) } as unknown as ReturnType<typeof getCurrentWindow>)
    const wrapper = mount({ setup() { useWindowCloseGuard(() => false, () => true) }, template: '<p>资料</p>' })
    wrapper.unmount()
    const stop = vi.fn(); register(stop); await flushPromises()
    expect(stop).toHaveBeenCalledOnce()
  })
})
