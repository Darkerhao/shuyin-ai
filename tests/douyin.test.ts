import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import DouyinVerification from '../src/modules/publishing/DouyinVerification.vue'
import * as api from '../src/services/douyin'
import type { PublishTask } from '../src/types/publishing'

vi.mock('../src/services/douyin', () => ({ openDouyinWindow: vi.fn(), inspectDouyinWindow: vi.fn(), uploadDouyinVideo: vi.fn(), completeDouyinTask: vi.fn(), stopDouyinTask: vi.fn(), onDouyinProgress: vi.fn() }))
const task = { id: 'selected-task', status: 'ready' } as PublishTask
const wrappers: ReturnType<typeof mount>[] = []
function render(disabled = false) {
  const wrapper = mount(DouyinVerification, { props: { desktop: true, task, disabled } })
  wrappers.push(wrapper)
  return wrapper
}
function button(wrapper: ReturnType<typeof mount>, text: string) { return wrapper.findAll('button').find(b => b.text() === text)! }
beforeEach(() => {
  vi.resetAllMocks()
  vi.mocked(api.onDouyinProgress).mockResolvedValue(vi.fn())
  vi.mocked(api.inspectDouyinWindow).mockResolvedValue({ supported: true, open: true, path: '/creator-micro/content/upload', loginVisible: false, online: true })
})
afterEach(() => wrappers.splice(0).forEach(w => w.unmount()))

describe('Douyin task verification', () => {
  it('requires page inspection and explicit account confirmation before sending the selected task ID', async () => {
    const w = render()
    const upload = button(w, '上传并自动填写当前任务')
    expect(upload.attributes()).toHaveProperty('disabled')
    await button(w, '检查抖音页面').trigger('click'); await flushPromises()
    expect(upload.attributes()).toHaveProperty('disabled')
    await w.get('#douyin-account-confirmed').trigger('click')
    await upload.trigger('click'); await flushPromises()
    expect(api.uploadDouyinVideo).toHaveBeenCalledWith('selected-task', true)
    expect(w.text()).toContain('这不代表已发布')
  })
  it.each([{ open: false, loginVisible: false, online: true }, { open: true, loginVisible: true, online: true }, { open: true, loginVisible: false, online: false }])('does not upload when the page is unavailable: %j', async state => {
    vi.mocked(api.inspectDouyinWindow).mockResolvedValue({ supported: true, path: '/', ...state })
    const w = render()
    await button(w, '检查抖音页面').trigger('click'); await flushPromises()
    await w.get('#douyin-account-confirmed').trigger('click')
    expect(button(w, '上传并自动填写当前任务').attributes()).toHaveProperty('disabled')
    expect(api.uploadDouyinVideo).not.toHaveBeenCalled()
  })
  it('blocks unsaved task changes and preserves errors for manual recovery', async () => {
    const w = render(true)
    expect(w.get('#douyin-account-confirmed').attributes('disabled')).toBeDefined()
    await w.setProps({ disabled: false })
    await button(w, '检查抖音页面').trigger('click'); await flushPromises()
    await w.get('#douyin-account-confirmed').trigger('click')
    vi.mocked(api.uploadDouyinVideo).mockRejectedValue('已有草稿，请人工处理')
    await button(w, '上传并自动填写当前任务').trigger('click'); await flushPromises()
    expect(w.text()).toContain('已有草稿，请人工处理')
    expect(w.text()).not.toContain('视频、文案和双封面已交给官方页面')
  })
  it('requires separate confirmation for an existing editor and keeps saved covers for manual takeover', async () => {
    const editor = { title: '', description: '', videoPreview: true, coverDialog: false, covers: [{ ratio: 'landscape' as const, selected: false, preview: '' }] }
    vi.mocked(api.inspectDouyinWindow).mockResolvedValue({ supported: true, open: true, path: '/creator-micro/content/post/video', loginVisible: false, online: true, editor })
    vi.mocked(api.completeDouyinTask).mockResolvedValue(editor)
    const w = render()
    await button(w, '检查抖音页面').trigger('click'); await flushPromises()
    expect(button(w, '继续填写文案和双封面').attributes()).toHaveProperty('disabled')
    expect(button(w, '上传并自动填写当前任务').attributes()).toHaveProperty('disabled')
    await w.get('#douyin-video-confirmed').trigger('click')
    await button(w, '继续填写文案和双封面').trigger('click'); await flushPromises()
    expect(api.completeDouyinTask).toHaveBeenCalledWith('selected-task', true)
    expect(w.text()).toContain('亲自点击发布')
    vi.mocked(api.inspectDouyinWindow).mockResolvedValue({ supported: true, open: true, path: '/creator-micro/content/post/video', loginVisible: false, online: true, editor: { ...editor, covers: [{ ratio: 'landscape', selected: true, preview: '' }] } })
    await button(w, '检查抖音页面').trigger('click'); await flushPromises()
    await w.get('#douyin-video-confirmed').trigger('click')
    expect(button(w, '继续填写文案和双封面').attributes()).toHaveProperty('disabled')
  })
  it('exposes stop while the upload runs and emits the working state until it settles', async () => {
    let finish!: () => void
    vi.mocked(api.uploadDouyinVideo).mockImplementation(() => new Promise<void>(resolve => { finish = resolve }))
    const w = render()
    await button(w, '检查抖音页面').trigger('click'); await flushPromises()
    await w.get('#douyin-account-confirmed').trigger('click')
    await button(w, '上传并自动填写当前任务').trigger('click'); await flushPromises()
    expect(w.emitted('working')?.at(-1)).toEqual([true])
    await button(w, '停止自动填写 / 人工接管').trigger('click'); await flushPromises()
    expect(api.stopDouyinTask).toHaveBeenCalledOnce()
    expect(w.text()).toContain('已停止后续填写')
    finish(); await flushPromises()
    expect(w.emitted('working')?.at(-1)).toEqual([false])
  })
  it('resets confirmation when the task changes and hides native upload on unsupported platforms', async () => {
    const w = render()
    await button(w, '检查抖音页面').trigger('click'); await flushPromises()
    await w.get('#douyin-account-confirmed').trigger('click')
    await w.setProps({ task: { ...task, id: 'another-task' } })
    expect(button(w, '上传并自动填写当前任务').attributes()).toHaveProperty('disabled')
    vi.mocked(api.inspectDouyinWindow).mockResolvedValue({ supported: false, open: false, path: '', loginVisible: false, online: true })
    await button(w, '检查抖音页面').trigger('click'); await flushPromises()
    expect(w.find('#douyin-account-confirmed').exists()).toBe(false)
    expect(api.uploadDouyinVideo).not.toHaveBeenCalled()
  })
})
