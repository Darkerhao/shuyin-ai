import { defineComponent, h } from 'vue'
import AppNotifications from '../src/components/AppNotifications.vue'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import VerificationPage from '../src/modules/verification/VerificationPage.vue'
import * as service from '../src/services/verification'

vi.mock('../src/services/verification', async (importOriginal) => ({
  ...await importOriginal<typeof service>(),
  isDesktop: vi.fn(), readProbe: vi.fn(), saveProbe: vi.fn(), selectVideo: vi.fn(),
}))

const saved = { databasePath: 'D:\\test\\shuyin.sqlite3', record: { note: '已保存内容', updatedAt: '2026-10-23T00:00:00Z' } }
const wrappers: ReturnType<typeof mount>[] = []
function render() {
  const wrapper = mount(defineComponent({ setup: () => () => [h(AppNotifications), h(VerificationPage)] }))
  wrappers.push(wrapper)
  return wrapper
}

beforeEach(() => {
  vi.resetAllMocks()
  vi.mocked(service.isDesktop).mockReturnValue(true)
  vi.mocked(service.readProbe).mockResolvedValue(saved)
})
afterEach(() => { wrappers.splice(0).forEach(wrapper => wrapper.unmount()) })

describe('M0 verification behavior', () => {
  it('browser preview disables native actions and does not claim verification', async () => {
    vi.mocked(service.isDesktop).mockReturnValue(false)
    const wrapper = render()
    await flushPromises()
    expect(service.readProbe).not.toHaveBeenCalled()
    expect(wrapper.text()).toContain('当前为浏览器预览')
    expect(wrapper.findAll('button').every(button => button.element.disabled)).toBe(true)
  })

  it('loads saved data and preserves the draft and saved result after a failed write', async () => {
    const wrapper = render()
    await flushPromises()
    expect(wrapper.get('textarea').element.value).toBe('已保存内容')
    await wrapper.get('textarea').setValue('用户尚未保存的修改')
    vi.mocked(service.saveProbe).mockRejectedValueOnce('database is locked')
    await wrapper.get('form').trigger('submit')
    await flushPromises()
    expect(wrapper.get('textarea').element.value).toBe('用户尚未保存的修改')
    expect(wrapper.get('[data-sonner-toast][data-type="error"]:not([data-removed="true"])').text()).toContain('database is locked')
    expect(wrapper.get('dl').text()).toContain('已保存内容')
    expect(wrapper.text()).not.toContain('写入并回读成功')
  })

  it('updates displayed persisted data only after successful write', async () => {
    const wrapper = render()
    await flushPromises()
    await wrapper.get('textarea').setValue('新内容')
    vi.mocked(service.saveProbe).mockResolvedValueOnce({ ...saved, record: { ...saved.record, note: '新内容' } })
    await wrapper.get('form').trigger('submit')
    await flushPromises()
    expect(service.saveProbe).toHaveBeenCalledWith('新内容')
    expect(wrapper.get('dl').text()).toContain('新内容')
    expect(wrapper.text()).toContain('写入并回读成功')
  })

  it('preserves selected path after cancellation and picker failure', async () => {
    const wrapper = render()
    await flushPromises()
    const button = wrapper.findAll('button').find(item => item.text() === '选择本地视频')!
    vi.mocked(service.selectVideo).mockResolvedValueOnce('D:\\视频\\验证.mp4')
    await button.trigger('click')
    await flushPromises()
    vi.mocked(service.selectVideo).mockResolvedValueOnce(null)
    await button.trigger('click')
    await flushPromises()
    expect(wrapper.get('.file-path').text()).toBe('D:\\视频\\验证.mp4')
    expect(wrapper.text()).toContain('已取消选择')
    vi.mocked(service.selectVideo).mockRejectedValueOnce(new Error('对话框不可用'))
    await button.trigger('click')
    await flushPromises()
    expect(wrapper.get('.file-path').text()).toBe('D:\\视频\\验证.mp4')
    expect(wrapper.get('[data-sonner-toast][data-type="error"]:not([data-removed="true"])').text()).toContain('对话框不可用')
  })

  it('shows initialization failure and allows an explicit read retry', async () => {
    vi.mocked(service.readProbe).mockRejectedValueOnce('数据库无法打开')
    const wrapper = render()
    await flushPromises()
    expect(wrapper.get('[data-sonner-toast][data-type="error"]:not([data-removed="true"])').text()).toContain('数据库无法打开')
    const retry = wrapper.findAll('button').find(button => button.text() === '重新读取')!
    await retry.trigger('click')
    await flushPromises()
    await vi.waitFor(() => expect(wrapper.find('[data-sonner-toast][data-type="error"]:not([data-removed="true"])').exists()).toBe(false))
    expect(wrapper.get('textarea').element.value).toBe('已保存内容')
  })
})
