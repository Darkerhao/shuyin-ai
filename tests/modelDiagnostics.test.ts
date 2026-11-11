import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import ModelDiagnostics from '../src/modules/settings/ModelDiagnostics.vue'
import * as api from '../src/services/maintenance'
import { button, chooseSelect, dialog, mockResizeObserver } from './ui'

vi.mock('../src/services/maintenance', () => ({ modelCalls: vi.fn(), clearModelCalls: vi.fn(), setModelCallRetention: vi.fn() }))

let wrapper: ReturnType<typeof mount> | undefined
function pageResult(page = 1, pageSize = 20, total = 245, runningCount = 0, retentionDays = 0): api.ModelCallPage {
  const offset = (page - 1) * pageSize
  const rows = Array.from({ length: Math.min(pageSize, total - offset) }, (_, index): api.ModelCall => ({
    id: `call-${offset + index + 1}`, projectId: 'book', title: `第${offset + index + 1}条记录`, taskId: null,
    model: 'test-model', stage: 'storyboard', status: 'succeeded', startedAt: '2026-11-11T00:00:00Z',
    elapsedMs: 2500, firstResponseMs: 100, inputTokens: 20, outputTokens: null, totalTokens: null, errorCode: null,
  }))
  return { rows, total, page, pageSize, runningCount, retentionDays }
}

async function render() {
  wrapper = mount(ModelDiagnostics, { attachTo: document.body })
  await flushPromises()
  return wrapper
}

beforeEach(() => {
  vi.resetAllMocks()
  mockResizeObserver()
  vi.mocked(api.modelCalls).mockImplementation(async (page, size) => pageResult(page, size))
})
afterEach(() => {
  wrapper?.unmount()
  wrapper = undefined
  document.body.innerHTML = ''
  vi.useRealTimers()
  vi.unstubAllGlobals()
})

it('requests only 20 rows initially and navigates through history beyond the old 200-record limit', async () => {
  const view = await render()
  expect(api.modelCalls).toHaveBeenCalledExactlyOnceWith(1, 20)
  expect(view.findAll('tbody tr')).toHaveLength(20)
  expect(view.text()).toContain('共 245 条')
  expect(view.text()).toContain('第 1 / 13 页')
  expect(button(view, '上一页').attributes('disabled')).toBeDefined()
  await button(view, '末页').trigger('click'); await flushPromises()
  expect(api.modelCalls).toHaveBeenLastCalledWith(13, 20)
  expect(view.findAll('tbody tr')).toHaveLength(5)
  expect(view.text()).toContain('第245条记录')
  expect(view.text()).toContain('第 241–245 条')
  expect(button(view, '下一页').attributes('disabled')).toBeDefined()
  await button(view, '上一页').trigger('click'); await flushPromises()
  expect(api.modelCalls).toHaveBeenLastCalledWith(12, 20)
  await button(view, '首页').trigger('click'); await flushPromises()
  expect(api.modelCalls).toHaveBeenLastCalledWith(1, 20)
})

it('changes page size from the first page and resets the table scroll position', async () => {
  const view = await render()
  await button(view, '下一页').trigger('click'); await flushPromises()
  const table = view.get('[aria-label="模型调用记录表"]').element as HTMLElement
  table.scrollTop = 400
  await chooseSelect(view, '#model-call-page-size', '50 条')
  expect(api.modelCalls).toHaveBeenLastCalledWith(1, 50)
  expect(view.findAll('tbody tr')).toHaveLength(50)
  expect(view.text()).toContain('第 1 / 5 页')
  expect(table.scrollTop).toBe(0)
})

it('preserves the current page on query failure and retries without reporting an empty history', async () => {
  const view = await render()
  vi.mocked(api.modelCalls).mockRejectedValueOnce('读取失败，请重试')
  await button(view, '下一页').trigger('click'); await flushPromises()
  expect(view.get('[role="alert"]').text()).toContain('读取失败')
  expect(view.text()).toContain('第 1 / 13 页')
  expect(view.findAll('tbody tr')).toHaveLength(20)
  await button(view, '下一页').trigger('click'); await flushPromises()
  expect(view.text()).toContain('第 2 / 13 页')
  expect(view.find('[role="alert"]').exists()).toBe(false)
})

it('distinguishes an initial loading failure from a genuinely empty history', async () => {
  let reject!: (reason: string) => void
  vi.mocked(api.modelCalls).mockReturnValueOnce(new Promise((_resolve, fail) => { reject = fail }))
  const view = await render()
  expect(view.text()).toContain('正在读取调用记录')
  expect(view.attributes('aria-busy')).toBe('true')
  reject('数据库不可用'); await flushPromises()
  expect(view.text()).toContain('数据库不可用')
  expect(view.text()).not.toContain('还没有调用记录')
  vi.mocked(api.modelCalls).mockResolvedValueOnce(pageResult(1, 20, 0))
  await button(view, '刷新调用记录').trigger('click'); await flushPromises()
  expect(view.text()).toContain('还没有调用记录')
  expect(view.text()).toContain('共 0 条')
  expect(button(view, '清除记录').attributes('disabled')).toBeDefined()
})

it('clears only after confirmation and refreshes from page one while retaining live requests', async () => {
  vi.mocked(api.modelCalls).mockImplementation(async (page, size) => pageResult(page, size, 245, 1))
  const view = await render()
  await button(view, '末页').trigger('click'); await flushPromises()
  await button(view, '清除记录').trigger('click'); await flushPromises()
  expect(dialog().text()).toContain('当前 244 条')
  expect(api.clearModelCalls).not.toHaveBeenCalled()
  await button(view, '取消').trigger('click'); await flushPromises()
  expect(api.clearModelCalls).not.toHaveBeenCalled()
  await button(view, '清除记录').trigger('click'); await flushPromises()
  vi.mocked(api.clearModelCalls).mockResolvedValue(244)
  const remaining = pageResult(1, 20, 1, 1)
  remaining.rows[0]!.status = 'running'
  vi.mocked(api.modelCalls).mockResolvedValueOnce(remaining)
  await button(view, '确认清除').trigger('click'); await flushPromises()
  expect(api.clearModelCalls).toHaveBeenCalledTimes(1)
  expect(api.modelCalls).toHaveBeenLastCalledWith(1, 20)
  expect(view.text()).toContain('已清除 244 条')
  expect(view.text()).toContain('1 条请求进行中')
  expect(view.text()).toContain('第 1 / 1 页')
  expect(button(view, '清除记录').attributes('disabled')).toBeDefined()
  expect(view.emitted('working')).toEqual([[true], [false]])
})

it('shows cleanup failures inside the confirmation and preserves the history for retry', async () => {
  const view = await render()
  vi.mocked(api.clearModelCalls).mockRejectedValueOnce('清除失败')
  await button(view, '清除记录').trigger('click'); await flushPromises()
  await button(view, '确认清除').trigger('click'); await flushPromises()
  expect(dialog().get('[role="alert"]').text()).toBe('清除失败')
  expect(view.findAll('tbody tr')).toHaveLength(20)
  expect(view.emitted('working')).toEqual([[true], [false]])
  vi.mocked(api.clearModelCalls).mockResolvedValue(245)
  vi.mocked(api.modelCalls).mockResolvedValueOnce(pageResult(1, 20, 0))
  await button(view, '确认清除').trigger('click'); await flushPromises()
  expect(document.querySelector('[role="alertdialog"]')).toBeNull()
  expect(view.text()).toContain('已清除 245 条')
})

it('requires confirmation to enable retention, loads the saved preference, and can disable cleanup', async () => {
  const view = await render()
  expect(view.get('#model-call-retention').text()).toBe('全部保留')
  await chooseSelect(view, '#model-call-retention', '保留最近 30 天')
  await button(view, '保存设置').trigger('click'); await flushPromises()
  expect(dialog().text()).toContain('保留最近 30 天')
  expect(api.setModelCallRetention).not.toHaveBeenCalled()
  await button(view, '取消').trigger('click'); await flushPromises()
  expect(api.setModelCallRetention).not.toHaveBeenCalled()
  await button(view, '保存设置').trigger('click'); await flushPromises()
  vi.mocked(api.setModelCallRetention).mockResolvedValue(225)
  vi.mocked(api.modelCalls).mockResolvedValue(pageResult(1, 20, 20, 0, 30))
  await button(view, '保存并清理').trigger('click'); await flushPromises()
  expect(api.setModelCallRetention).toHaveBeenCalledExactlyOnceWith(30)
  expect(view.text()).toContain('清理了 225 条过期记录')
  expect(button(view, '保存设置').attributes('disabled')).toBeDefined()
  view.unmount(); wrapper = undefined
  const reopened = await render()
  expect(reopened.get('#model-call-retention').text()).toBe('保留最近 30 天')
  await chooseSelect(reopened, '#model-call-retention', '全部保留')
  vi.mocked(api.setModelCallRetention).mockResolvedValue(0)
  vi.mocked(api.modelCalls).mockResolvedValue(pageResult(1, 20, 20))
  await button(reopened, '保存设置').trigger('click'); await flushPromises()
  expect(api.setModelCallRetention).toHaveBeenLastCalledWith(0)
  expect(reopened.text()).toContain('已关闭自动清理')
  expect(document.querySelector('[role="alertdialog"]')).toBeNull()
})

it('polls active requests only on page one without overwriting an unsaved retention choice', async () => {
  vi.useFakeTimers({ toFake: ['setInterval', 'clearInterval'] })
  vi.mocked(api.modelCalls).mockImplementation(async (page, size) => pageResult(page, size, 245, 1))
  const view = await render()
  await chooseSelect(view, '#model-call-retention', '保留最近 7 天')
  await vi.advanceTimersByTimeAsync(3000); await flushPromises()
  expect(api.modelCalls).toHaveBeenCalledTimes(2)
  expect(view.get('#model-call-retention').text()).toBe('保留最近 7 天')
  await button(view, '下一页').trigger('click'); await flushPromises()
  expect(api.modelCalls).toHaveBeenCalledTimes(3)
  await vi.advanceTimersByTimeAsync(6000); await flushPromises()
  expect(api.modelCalls).toHaveBeenCalledTimes(3)
  expect(view.text()).toContain('第 2 / 13 页')
  await button(view, '首页').trigger('click'); await flushPromises()
  await button(view, '清除记录').trigger('click'); await flushPromises()
  await vi.advanceTimersByTimeAsync(3000); await flushPromises()
  expect(api.modelCalls).toHaveBeenCalledTimes(4)
  view.unmount(); wrapper = undefined
  await vi.advanceTimersByTimeAsync(3000)
  expect(api.modelCalls).toHaveBeenCalledTimes(4)
})

it('waits for other workspace operations and refreshes after they finish', async () => {
  wrapper = mount(ModelDiagnostics, { props: { disabled: true }, attachTo: document.body })
  await flushPromises()
  expect(api.modelCalls).not.toHaveBeenCalled()
  await wrapper.setProps({ disabled: false }); await flushPromises()
  expect(api.modelCalls).toHaveBeenCalledExactlyOnceWith(1, 20)
  await wrapper.setProps({ disabled: true })
  expect(button(wrapper, '清除记录').attributes('disabled')).toBeDefined()
  vi.mocked(api.modelCalls).mockResolvedValueOnce(pageResult(1, 20, 5, 0, 90))
  await wrapper.setProps({ disabled: false }); await flushPromises()
  expect(wrapper.text()).toContain('共 5 条')
  expect(wrapper.get('#model-call-retention').text()).toBe('保留最近 90 天')
})

it('can disable a successfully saved retention policy even when the following list refresh fails', async () => {
  const view = await render()
  await chooseSelect(view, '#model-call-retention', '保留最近 30 天')
  await button(view, '保存设置').trigger('click'); await flushPromises()
  vi.mocked(api.setModelCallRetention).mockResolvedValueOnce(225)
  vi.mocked(api.modelCalls).mockRejectedValueOnce('调用列表读取失败')
  await button(view, '保存并清理').trigger('click'); await flushPromises()
  expect(api.setModelCallRetention).toHaveBeenCalledExactlyOnceWith(30)
  expect(view.get('[role="alert"]').text()).toContain('调用列表读取失败')
  expect(view.text()).toContain('已保存保留最近 30 天')
  await chooseSelect(view, '#model-call-retention', '全部保留')
  expect(button(view, '保存设置').attributes('disabled')).toBeUndefined()
  vi.mocked(api.setModelCallRetention).mockResolvedValueOnce(0)
  vi.mocked(api.modelCalls).mockResolvedValueOnce(pageResult(1, 20, 20))
  await button(view, '保存设置').trigger('click'); await flushPromises()
  expect(api.setModelCallRetention).toHaveBeenLastCalledWith(0)
  expect(view.text()).toContain('已关闭自动清理')
  expect(button(view, '保存设置').attributes('disabled')).toBeDefined()
})

it.each(['success', 'failure'])('refreshes restored data after an older in-flight read settles with %s', async outcome => {
  let resolveOld!: (value: api.ModelCallPage) => void
  let rejectOld!: (reason: string) => void
  let resolveRestored!: (value: api.ModelCallPage) => void
  vi.mocked(api.modelCalls)
    .mockReturnValueOnce(new Promise((resolve, reject) => { resolveOld = resolve; rejectOld = reject }))
    .mockReturnValueOnce(new Promise(resolve => { resolveRestored = resolve }))
  const view = await render()
  await view.setProps({ disabled: true })
  await view.setProps({ disabled: false })
  if (outcome === 'success') resolveOld(pageResult(1, 20, 245, 0, 0))
  else rejectOld('恢复前的请求失败')
  await flushPromises()
  expect(api.modelCalls).toHaveBeenCalledTimes(2)
  expect(view.text()).not.toContain('共 245 条')
  expect(view.find('[role="alert"]').exists()).toBe(false)
  resolveRestored(pageResult(1, 20, 5, 0, 90)); await flushPromises()
  expect(view.text()).toContain('共 5 条')
  expect(view.get('#model-call-retention').text()).toBe('保留最近 90 天')
  expect(view.attributes('aria-busy')).toBe('false')
})

it('discards a read that completes during maintenance and loads only after maintenance finishes', async () => {
  let resolveOld!: (value: api.ModelCallPage) => void
  vi.mocked(api.modelCalls).mockReturnValueOnce(new Promise(resolve => { resolveOld = resolve }))
  const view = await render()
  await view.setProps({ disabled: true })
  resolveOld(pageResult(1, 20, 245)); await flushPromises()
  expect(view.text()).not.toContain('共 245 条')
  expect(api.modelCalls).toHaveBeenCalledTimes(1)
  vi.mocked(api.modelCalls).mockResolvedValueOnce(pageResult(1, 20, 5, 0, 7))
  await view.setProps({ disabled: false }); await flushPromises()
  expect(view.text()).toContain('共 5 条')
  expect(view.get('#model-call-retention').text()).toBe('保留最近 7 天')
})

it('does not restart an obsolete read after the diagnostics view is unmounted', async () => {
  let resolveOld!: (value: api.ModelCallPage) => void
  vi.mocked(api.modelCalls).mockReturnValueOnce(new Promise(resolve => { resolveOld = resolve }))
  const view = await render()
  await view.setProps({ disabled: true })
  await view.setProps({ disabled: false })
  view.unmount(); wrapper = undefined
  resolveOld(pageResult()); await flushPromises()
  expect(api.modelCalls).toHaveBeenCalledTimes(1)
})

it('keeps a failed retention save retryable and never reports that it was saved', async () => {
  const view = await render()
  await chooseSelect(view, '#model-call-retention', '保留最近 7 天')
  await button(view, '保存设置').trigger('click'); await flushPromises()
  vi.mocked(api.setModelCallRetention).mockRejectedValueOnce('设置保存失败')
  await button(view, '保存并清理').trigger('click'); await flushPromises()
  expect(dialog().get('[role="alert"]').text()).toBe('设置保存失败')
  expect(view.text()).not.toContain('已保存保留最近')
  expect(api.modelCalls).toHaveBeenCalledTimes(1)
  expect(button(view, '保存并清理').attributes('disabled')).toBeUndefined()
  vi.mocked(api.setModelCallRetention).mockResolvedValueOnce(230)
  vi.mocked(api.modelCalls).mockResolvedValueOnce(pageResult(1, 20, 15, 0, 7))
  await button(view, '保存并清理').trigger('click'); await flushPromises()
  expect(api.setModelCallRetention).toHaveBeenCalledTimes(2)
  expect(view.text()).toContain('共 15 条')
  expect(view.text()).toContain('清理了 230 条过期记录')
})

it('retries only the list after a confirmed cleanup succeeds but its refresh fails', async () => {
  const view = await render()
  await button(view, '清除记录').trigger('click'); await flushPromises()
  vi.mocked(api.clearModelCalls).mockResolvedValueOnce(245)
  vi.mocked(api.modelCalls).mockRejectedValueOnce('列表暂不可用')
  await button(view, '确认清除').trigger('click'); await flushPromises()
  expect(document.querySelector('[role="alertdialog"]')).toBeNull()
  expect(view.text()).toContain('已清除 245 条')
  expect(view.get('[role="alert"]').text()).toBe('列表暂不可用')
  vi.mocked(api.modelCalls).mockResolvedValueOnce(pageResult(1, 20, 0))
  await button(view, '刷新调用记录').trigger('click'); await flushPromises()
  expect(api.clearModelCalls).toHaveBeenCalledTimes(1)
  expect(view.text()).toContain('共 0 条')
  expect(view.find('[role="alert"]').exists()).toBe(false)
})
