import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import DouyinVerification from '../src/modules/publishing/DouyinVerification.vue'
import * as api from '../src/services/douyin'
import type { DouyinSession } from '../src/services/douyin'
import type { PublishTask } from '../src/types/publishing'

vi.mock('../src/services/douyin', () => ({ openDouyinWindow: vi.fn(), getDouyinSession: vi.fn(), startDouyinTask: vi.fn(), continueDouyinTask: vi.fn(), stopDouyinTask: vi.fn(), onDouyinProgress: vi.fn() }))
const task = { id: 'selected-task', status: 'ready' } as PublishTask
const state = (changes: Partial<DouyinSession> = {}): DouyinSession => ({
  runId: 1, taskId: task.id, status: 'waiting', step: 'connect', message: '请核对账号', waitingFor: 'account',
  account: { id: null, name: '甲子', key: 'visible-avatar' }, completed: [], manual: [], collection: null, options: [], ...changes,
})
const wrappers: ReturnType<typeof mount>[] = []
let progress: (session: DouyinSession) => void
function render(extra = {}) {
  const wrapper = mount(DouyinVerification, { props: { desktop: true, task, ...extra } })
  wrappers.push(wrapper)
  return wrapper
}
function button(wrapper: ReturnType<typeof mount>, text: string) { return wrapper.findAll('button').find(b => b.text() === text)! }
beforeEach(() => {
  vi.resetAllMocks()
  vi.mocked(api.getDouyinSession).mockResolvedValue(null)
  vi.mocked(api.onDouyinProgress).mockImplementation(async callback => { progress = callback; return vi.fn<() => void>() })
  vi.mocked(api.startDouyinTask).mockResolvedValue(state())
  vi.mocked(api.continueDouyinTask).mockResolvedValue(state({ status: 'review', step: 'review', waitingFor: null, message: '请检查并亲自发布' }))
})
afterEach(() => wrappers.splice(0).forEach(w => w.unmount()))

describe('single host-owned Douyin workflow', () => {
  it('prepares local changes before starting and confirms the account through the current run', async () => {
    const prepareTask = vi.fn(async () => task)
    const w = render({ prepareTask })
    await flushPromises()
    await button(w, '开始抖音辅助发布').trigger('click'); await flushPromises()
    expect(prepareTask).toHaveBeenCalledOnce()
    expect(api.startDouyinTask).toHaveBeenCalledWith(task.id)
    expect(prepareTask.mock.invocationCallOrder[0]).toBeLessThan(vi.mocked(api.startDouyinTask).mock.invocationCallOrder[0]!)
    await button(w, '使用这个账号继续').trigger('click'); await flushPromises()
    expect(api.continueDouyinTask).toHaveBeenCalledWith(1, { kind: 'account', name: '甲子' })
    expect(w.text()).toContain('亲自发布')
    expect(w.text()).not.toContain('检查抖音页面')
  })
  it('keeps local changes and does not start when saving or packaging fails', async () => {
    const w = render({ prepareTask: vi.fn().mockRejectedValue('缺少最终成片') })
    await flushPromises()
    await button(w, '开始抖音辅助发布').trigger('click'); await flushPromises()
    expect(w.text()).toContain('缺少最终成片')
    expect(api.startDouyinTask).not.toHaveBeenCalled()
    expect(w.emitted('working')?.at(-1)).toEqual([false])
  })
  it('restores a waiting host session on mount and retains the editing lock', async () => {
    vi.mocked(api.getDouyinSession).mockResolvedValue(state({ step: 'collection', waitingFor: 'collection', completed: ['connect', 'video', 'text', 'covers'] }))
    const w = render(); await flushPromises()
    expect(w.text()).toContain('本次合集')
    expect(w.emitted('working')?.at(-1)).toEqual([true])
    expect(api.startDouyinTask).not.toHaveBeenCalled()
    expect(w.text()).not.toContain('记为这个账号的默认选择')
  })
  it('filters another task and stale runs from events and command responses', async () => {
    const w = render(); await flushPromises()
    progress(state({ runId: 4, message: '当前进度' })); await flushPromises()
    progress(state({ runId: 3, message: '旧运行' }))
    progress(state({ runId: 5, taskId: 'other', message: '其他作品' })); await flushPromises()
    expect(w.text()).toContain('当前进度')
    expect(w.text()).not.toContain('旧运行')
    expect(w.text()).not.toContain('其他作品')
  })
  it('waits for host stop acknowledgement before releasing the lock', async () => {
    vi.mocked(api.getDouyinSession).mockResolvedValue(state())
    let finish!: (s: DouyinSession) => void
    vi.mocked(api.stopDouyinTask).mockImplementation(() => new Promise(resolve => { finish = resolve }))
    const w = render(); await flushPromises()
    await button(w, '停止并人工接管').trigger('click'); await flushPromises()
    expect(api.stopDouyinTask).toHaveBeenCalledWith(1)
    expect(w.emitted('working')?.at(-1)).toEqual([true])
    finish(state({ status: 'stopped', waitingFor: null, message: '人工接管' })); await flushPromises()
    expect(w.emitted('working')?.at(-1)).toEqual([false])
  })
  it('keeps the lock and shows the reason if stopping fails', async () => {
    vi.mocked(api.getDouyinSession).mockResolvedValue(state())
    vi.mocked(api.stopDouyinTask).mockRejectedValue('停止失败')
    const w = render(); await flushPromises()
    await button(w, '停止并人工接管').trigger('click'); await flushPromises()
    expect(w.text()).toContain('停止失败')
    expect(w.emitted('working')?.at(-1)).toEqual([true])
  })
  it('persists an explicit no-collection decision and labels manual confirmation', async () => {
    vi.mocked(api.getDouyinSession).mockResolvedValue(state({ waitingFor: 'collection', step: 'collection' }))
    const w = render(); await flushPromises()
    await button(w, '确认本次合集').trigger('click'); await flushPromises()
    expect(api.continueDouyinTask).toHaveBeenCalledWith(1, { kind: 'collection', name: null, remember: false, manual: false })
    progress(state({ status: 'review', waitingFor: null, step: 'review', completed: ['covers', 'collection'], manual: ['collection'] }))
    await flushPromises()
    expect(w.text()).toContain('人工确认')
  })
  it('never starts again when receiving ordinary progress or showing the official window', async () => {
    const w = render(); await flushPromises()
    progress(state({ waitingFor: 'processing', step: 'video' })); await flushPromises()
    await button(w, '显示抖音窗口').trigger('click'); await flushPromises()
    expect(api.openDouyinWindow).toHaveBeenCalledOnce()
    expect(api.startDouyinTask).not.toHaveBeenCalled()
    expect(api.continueDouyinTask).not.toHaveBeenCalled()
  })
  it('clears an unconfirmed manual collection when the account changes', async () => {
    vi.mocked(api.getDouyinSession).mockResolvedValue(state({ waitingFor: 'collection', step: 'collection' }))
    const w = render(); await flushPromises()
    await w.get('#douyin-manual-collection').setValue('原账号合集')
    progress(state({ waitingFor: 'collection', step: 'collection', account: { id: null, name: '另一个账号', key: 'another-avatar' } }))
    await flushPromises()
    expect((w.get('#douyin-manual-collection').element as HTMLInputElement).value).toBe('')
    await button(w, '已核对官方页合集，人工确认').trigger('click'); await flushPromises()
    expect(api.continueDouyinTask).toHaveBeenCalledWith(1, { kind: 'collection', name: null, remember: false, manual: true })
  })
})
