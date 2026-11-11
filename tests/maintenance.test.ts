import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { button, dialog } from './ui'
import StorageManager from '../src/modules/settings/StorageManager.vue'
import ModelDiagnostics from '../src/modules/settings/ModelDiagnostics.vue'
import * as api from '../src/services/maintenance'
import { resumeRoute, resumeLabel, type ResumeTask } from '../src/services/workflow'
vi.mock('../src/services/maintenance', async original => ({ ...await original<typeof api>(), storageUsage: vi.fn(), previewCleanup: vi.fn(), cleanCache: vi.fn(), modelCalls: vi.fn() }))
let wrapper: ReturnType<typeof mount>
beforeEach(() => vi.resetAllMocks())
afterEach(() => { wrapper?.unmount(); document.body.innerHTML = ''; vi.useRealTimers() })
it('routes directly to the exact video, audio and publish task with state-specific labels', () => {
  const task: ResumeTask = { projectId: 'p1', title: '原书', kind: 'video', taskId: 'v2', status: 'running', stage: 'rendering', updatedAt: '', running: true }
  expect(resumeRoute(task)).toEqual({ path: '/projects/p1/video', query: { job: 'v2' } })
  expect(resumeLabel(task)).toBe('正在渲染视频')
  expect(resumeRoute({ ...task, kind: 'audio', taskId: 'a9' })).toEqual({ path: '/projects/p1/writing', query: { tab: 'audio', audioJob: 'a9' } })
  expect(resumeRoute({ ...task, kind: 'publishing', taskId: 'publish7' })).toEqual({ path: '/projects/p1/publishing', query: { task: 'publish7' } })
})
it('only cleans the previewed cache after explicit confirmation and refreshes occupancy', async () => {
  vi.mocked(api.storageUsage).mockResolvedValue([{ projectId: 'p', title: '书', bytes: 1024, reclaimableBytes: 512, files: 1, blocked: false, error: '' }])
  vi.mocked(api.previewCleanup).mockResolvedValue({ projectId: 'p', token: 'snapshot', bytes: 512, files: ['video/job/cache/file.mp4'] })
  vi.mocked(api.cleanCache).mockResolvedValue(512)
  wrapper = mount(StorageManager, { props: { disabled: false }, attachTo: document.body })
  await button(wrapper, '扫描占用').trigger('click'); await flushPromises()
  await button(wrapper, '预览清理').trigger('click'); await flushPromises()
  expect(dialog().text()).toContain('video/job/cache/file.mp4')
  expect(api.cleanCache).not.toHaveBeenCalled()
  await dialog().findAll('button').find(b => b.text() === '取消')!.trigger('click'); await flushPromises(); expect(api.cleanCache).not.toHaveBeenCalled()
  await button(wrapper, '预览清理').trigger('click'); await flushPromises()
  await dialog().findAll('button').find(b => b.text() === '清理列出的缓存')!.trigger('click'); await flushPromises()
  expect(api.cleanCache).toHaveBeenCalledExactlyOnceWith({ projectId: 'p', token: 'snapshot', bytes: 512, files: ['video/job/cache/file.mp4'] })
  expect(wrapper.text()).toContain('已释放')
  expect(api.storageUsage).toHaveBeenCalledTimes(2)
})
it('shows cleanup refusal and allows a fresh preview without hiding the failure', async () => {
  vi.mocked(api.storageUsage).mockResolvedValue([{ projectId: 'p', title: '书', bytes: 1024, reclaimableBytes: 512, files: 1, blocked: false, error: '' }])
  vi.mocked(api.previewCleanup).mockResolvedValue({ projectId: 'p', token: 'old', bytes: 512, files: ['cache'] })
  vi.mocked(api.cleanCache).mockRejectedValue('缓存或素材引用已变化')
  wrapper = mount(StorageManager, { props: { disabled: false }, attachTo: document.body })
  await button(wrapper, '扫描占用').trigger('click'); await flushPromises()
  await button(wrapper, '预览清理').trigger('click'); await flushPromises(); await dialog().findAll('button').find(b => b.text() === '清理列出的缓存')!.trigger('click'); await flushPromises()
  expect(wrapper.get('[role="alert"]').text()).toContain('引用已变化')
  expect(document.querySelector('[role="alertdialog"]')).toBeNull()
})
it('shows only returned tokens and separates response timing from whole request timing', async () => {
  vi.mocked(api.modelCalls).mockResolvedValue({ rows: [{ id: 'c', projectId: 'p', title: '书', taskId: 'j', model: 'test-model', stage: 'proofreading', status: 'succeeded', startedAt: '2026-11-03T00:00:00Z', elapsedMs: 2500, firstResponseMs: 100, inputTokens: 20, outputTokens: null, totalTokens: null, errorCode: null }], total: 1, page: 1, pageSize: 20, runningCount: 0, retentionDays: 0 })
  wrapper = mount(ModelDiagnostics); await flushPromises()
  expect(wrapper.text()).toContain('字幕校对')
  expect(wrapper.text()).toContain('0.10秒 / 2.50秒')
  expect(wrapper.text()).toContain('20 / — / —')
})
it('reminds after seven days without assuming that an absent record is successful', () => {
  const now = Date.parse('2026-11-03T00:00:00Z')
  expect(api.backupDue(null, now)).toBe(true)
  expect(api.backupDue({ kind: 'full', path: 'D:/backup', completedAt: '2026-11-02T00:00:00Z' }, now)).toBe(false)
  expect(api.backupDue({ kind: 'database', path: 'D:/backup', completedAt: '2026-10-20T00:00:00Z' }, now)).toBe(true)
})
