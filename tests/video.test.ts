import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createRouter, createMemoryHistory } from 'vue-router'
import VideoPage from '../src/modules/video/VideoPage.vue'
import * as api from '../src/services/video'
import * as projects from '../src/services/projects'
import * as ai from '../src/services/ai'
import { emptyProject } from '../src/types/project'
import type { VideoJob } from '../src/types/video'
vi.mock('../src/services/video', () => ({ settings: vi.fn(), saveSettings: vi.fn(), listJobs: vi.fn(), startJob: vi.fn(), continueJob: vi.fn(), cancelJob: vi.fn(), savePlan: vi.fn(), subtitles: vi.fn(), exportSubtitles: vi.fn(), mediaSource: vi.fn(), chooseRuntimePath: vi.fn() }))
vi.mock('../src/services/projects', () => ({ isDesktop: () => true, getProject: vi.fn(), listAssets: vi.fn(), openAsset: vi.fn(), openProjectDirectory: vi.fn() }))
vi.mock('../src/services/ai', () => ({ listProviders: vi.fn() }))
const job: VideoJob = { id: 'j1', projectId: 'p1', audioAssetId: 'a1', sourceSubtitleId: null, providerId: 'm1', status: 'awaiting_confirmation', stage: 'storyboard', runId: 1, plan: { title: '原书', audioSha256: 'hash', duration: 8, cues: [{ start: 0, end: 7, text: '原声字幕' }], scenes: [{ first: 0, last: 0, kind: 'opening', title: '观点', beats: [{ cue: 0, object: 'book', action: 'draw', label: '一本书' }] }] }, error: '', subtitleAssetId: 's1', videoAssetId: null, createdAt: '', updatedAt: '' }
const mounted: ReturnType<typeof mount>[] = []
beforeEach(() => {
  vi.resetAllMocks()
  vi.mocked(projects.getProject).mockResolvedValue({ ...emptyProject(), id: 'p1', title: '原书', workspacePath: 'D:/books', createdAt: '', updatedAt: '', archivedAt: null })
  vi.mocked(projects.listAssets).mockResolvedValue([{ id: 'a1', projectId: 'p1', assetType: 'audio', filePath: 'D:/原声.m4a', originalName: '原声.m4a', fileSize: 100, modifiedAt: 1, existsStatus: 'available', createdAt: '' }])
  vi.mocked(ai.listProviders).mockResolvedValue([{ id: 'm1', modelKind: 'text', name: '文本模型', modelName: 'text', baseUrl: 'http://localhost', isDefault: true, isEnabled: true, credentialStatus: 'available' }])
  vi.mocked(api.settings).mockResolvedValue({ engineRoot: 'D:/engine', nodePath: 'D:/node.exe', pythonPath: 'D:/python.exe', modelPath: 'D:/model' })
  vi.mocked(api.listJobs).mockResolvedValue([structuredClone(job)])
  vi.mocked(api.subtitles).mockResolvedValue('1\n00:00:00,000 --> 00:00:07,000\n原声字幕\n')
  vi.mocked(api.mediaSource).mockResolvedValue('http://asset.localhost/audio.m4a')
})
afterEach(() => { mounted.splice(0).forEach(w => w.unmount()); document.body.innerHTML = ''; vi.useRealTimers() })
async function render() {
  const router = createRouter({ history: createMemoryHistory(), routes: [{ path: '/projects/:id/video', component: VideoPage, props: true }, { path: '/:pathMatch(.*)*', component: { template: '<p>其他页面</p>' } }] })
  await router.push('/projects/p1/video?audio=a1'); await router.isReady()
  const wrapper = mount({ template: '<router-view />' }, { global: { plugins: [router] } }); mounted.push(wrapper)
  await flushPromises(); return wrapper
}
it('requires an explicit confirmation before rendering a prepared storyboard', async () => {
  const wrapper = await render()
  const scroll = vi.fn()
  wrapper.get('.video-job').element.scrollIntoView = scroll
  expect(wrapper.text()).toContain('等待确认分镜')
  expect(api.continueJob).not.toHaveBeenCalled()
  vi.mocked(api.continueJob).mockResolvedValue({ ...job, status: 'running', stage: 'rendering' })
  await wrapper.get('[data-test="render-video"]').trigger('click'); await flushPromises()
  expect(api.savePlan).toHaveBeenCalledWith('j1', expect.stringContaining('原声字幕'), job.plan.scenes)
  expect(api.continueJob).toHaveBeenCalledWith('j1', true)
  expect(wrapper.get('[aria-label="选择视频历史任务"]').text()).toContain('制作中')
  expect(scroll).toHaveBeenCalledWith({ block: 'start' })
})
it('a failed save keeps the edited subtitles and never starts a render', async () => {
  const wrapper = await render()
  await wrapper.get('#video-subtitles').setValue('未保存的校对')
  vi.mocked(api.savePlan).mockRejectedValue(new Error('保存失败'))
  await wrapper.get('[data-test="render-video"]').trigger('click'); await flushPromises()
  expect((wrapper.get('#video-subtitles').element as HTMLTextAreaElement).value).toBe('未保存的校对')
  expect(api.continueJob).not.toHaveBeenCalled()
})
it('retries a failed render with the saved subtitles and storyboard instead of starting over', async () => {
  vi.useFakeTimers()
  vi.mocked(api.listJobs).mockResolvedValue([{ ...structuredClone(job), status: 'failed', stage: 'rendering', error: 'Layout check failed: not deterministic' }])
  const wrapper = await render()
  wrapper.get('.video-job').element.scrollIntoView = vi.fn()
  const subtitles = (wrapper.get('#video-subtitles').element as HTMLTextAreaElement).value
  expect(api.continueJob).not.toHaveBeenCalled()
  vi.mocked(api.continueJob).mockResolvedValue({ ...job, status: 'running', stage: 'rendering', runId: 2 })
  await wrapper.get('[data-test="render-video"]').trigger('click'); await flushPromises()
  expect(api.savePlan).toHaveBeenCalledExactlyOnceWith('j1', subtitles, job.plan.scenes)
  expect(api.continueJob).toHaveBeenCalledExactlyOnceWith('j1', true)
  expect(api.startJob).not.toHaveBeenCalled()
  vi.mocked(api.listJobs).mockResolvedValue([{ ...job, status: 'cancelled', stage: 'rendering', runId: 2 }])
  await vi.advanceTimersByTimeAsync(1500)
  expect((wrapper.get('#video-subtitles').element as HTMLTextAreaElement).value).toBe(subtitles)
})
it('does not show another book after a late background polling response', async () => {
  vi.useFakeTimers()
  vi.mocked(api.listJobs).mockResolvedValue([{ ...job, status: 'running' }])
  const wrapper = await render()
  let finish: (jobs: VideoJob[]) => void = () => {}
  vi.mocked(api.listJobs).mockImplementationOnce(() => new Promise(resolve => { finish = resolve }))
  await vi.advanceTimersByTimeAsync(1500)
  vi.mocked(api.listJobs).mockResolvedValue([])
  await wrapper.vm.$router.push('/projects/p2/video')
  await flushPromises()
  finish([{ ...job, status: 'succeeded' }])
  await flushPromises()
  expect(wrapper.text()).not.toContain('任务与成片')
})
it('refreshes intermediate transcription feedback and clears the stale connection notice after recovery', async () => {
  vi.useFakeTimers()
  const current: VideoJob = { ...job, status: 'running', stage: 'transcribing', plan: { ...job.plan, cues: [], scenes: [] } }
  vi.mocked(api.listJobs).mockResolvedValue([current])
  const wrapper = await render()
  expect(wrapper.text()).toContain('正在准备原声识别')
  vi.mocked(api.listJobs).mockRejectedValueOnce(new Error('disconnected'))
  await vi.advanceTimersByTimeAsync(1500)
  expect(wrapper.text()).toContain('当前展示上次读取的进度')
  vi.mocked(api.listJobs).mockResolvedValue([{ ...current, progress: { startedAt: Date.now() - 10000, stage: 'transcribing', phase: 'recognizing', completed: 4, total: 8, cueCount: 1, preview: [{ start: 1, end: 4, text: '识别中的真实句子' }] } }])
  await vi.advanceTimersByTimeAsync(1500)
  expect(wrapper.text()).toContain('识别中的真实句子')
  expect(wrapper.get('[role="progressbar"]').attributes('aria-valuenow')).toBe('50')
  expect(wrapper.text()).not.toContain('当前展示上次读取的进度')
})

it('does not render legacy text cards and upgrades only the storyboard while preserving subtitles', async () => {
  vi.mocked(api.listJobs).mockResolvedValue([{ ...job, plan: { ...job.plan, scenes: [{ first: 0, last: 0, kind: 'book', title: '旧图解', labels: ['一本书'] }] } }])
  const wrapper = await render()
  expect(wrapper.text()).toContain('旧版文字图解分镜')
  expect(wrapper.get('[data-test="render-video"]').attributes('disabled')).toBeDefined()
  vi.mocked(api.continueJob).mockResolvedValue({ ...job, status: 'running', stage: 'storyboard' })
  const upgrade = wrapper.findAll('button').find(button => button.text().includes('保留字幕并升级手绘分镜'))!
  await upgrade.trigger('click'); await flushPromises()
  expect(api.savePlan).toHaveBeenCalledWith('j1', expect.stringContaining('原声字幕'), [])
  expect(api.continueJob).toHaveBeenCalledWith('j1', false)
  expect(api.startJob).not.toHaveBeenCalled()
})

it('shows the concrete object, action and subtitle trigger and saves label edits', async () => {
  const wrapper = await render()
  expect(wrapper.text()).toContain('字幕 1 · 书本 · 逐笔绘制')
  await wrapper.get('[aria-label="第1镜物体1标签"]').setValue('原书')
  vi.mocked(api.continueJob).mockResolvedValue({ ...job, status: 'running', stage: 'rendering' })
  await wrapper.get('[data-test="render-video"]').trigger('click'); await flushPromises()
  expect(api.savePlan).toHaveBeenCalledWith('j1', expect.any(String), [{ ...job.plan.scenes[0], beats: [{ cue: 0, object: 'book', action: 'draw', label: '原书' }] }])
  expect(job.plan.scenes[0]?.beats?.[0]?.label).toBe('一本书')
})

it('makes a separate hand-drawn version from the completed task audio and saved subtitles', async () => {
  vi.mocked(api.listJobs).mockResolvedValue([{ ...job, status: 'succeeded', videoAssetId: 'v1' }])
  vi.mocked(api.startJob).mockResolvedValue({ ...job, id: 'j2', status: 'running', plan: { ...job.plan, cues: [], scenes: [] } })
  const wrapper = await render()
  const remake = wrapper.findAll('button').find(button => button.text().includes('用原字幕制作新版手绘'))!
  await remake.trigger('click'); await flushPromises()
  expect(api.startJob).toHaveBeenCalledWith('p1', 'a1', 'm1', 's1')
  expect(api.savePlan).not.toHaveBeenCalled()
})
