import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createRouter, createMemoryHistory } from 'vue-router'
import VideoPage from '../src/modules/video/VideoPage.vue'
import * as api from '../src/services/video'
import * as projects from '../src/services/projects'
import * as ai from '../src/services/ai'
import { loadWaveform } from '../src/services/audioWaveform'
import { emptyProject } from '../src/types/project'
import { emptyProvider } from '../src/types/ai'
import { chooseSelect, mockResizeObserver } from './ui'
import type { VideoJob } from '../src/types/video'
import VideoSceneRevision from '../src/modules/video/VideoSceneRevision.vue'
import VideoSubtitleEditor from '../src/modules/video/VideoSubtitleEditor.vue'
import * as drafts from '../src/services/drafts'
vi.mock('../src/services/drafts', () => ({ readDraft: vi.fn(), saveDraft: vi.fn() }))
vi.mock('../src/services/video', () => ({ originalCues: vi.fn(), settings: vi.fn(), saveSettings: vi.fn(), checkEnvironment: vi.fn(), listJobs: vi.fn(), startJob: vi.fn(), continueJob: vi.fn(), previewJob: vi.fn(), previewSource: vi.fn(), cancelJob: vi.fn(), savePlan: vi.fn(), subtitles: vi.fn(), exportSubtitles: vi.fn(), chooseRuntimePath: vi.fn(), chooseImage: vi.fn(), importImage: vi.fn(), readImage: vi.fn(), startIllustration: vi.fn(), executeIllustration: vi.fn() }))
vi.mock('../src/services/projects', () => ({ isDesktop: () => true, getProject: vi.fn(), listAssets: vi.fn(), mediaSource: vi.fn(), openAsset: vi.fn(), openProjectDirectory: vi.fn() }))
vi.mock('../src/services/ai', () => ({ listProviders: vi.fn() }))
vi.mock('../src/services/audioWaveform', () => ({ loadWaveform: vi.fn().mockResolvedValue([.2, .6, .8, .1]) }))
const job: VideoJob = { id: 'j1', projectId: 'p1', audioAssetId: 'a1', sourceSubtitleId: null, providerId: 'm1', status: 'awaiting_confirmation', stage: 'storyboard', runId: 1, plan: { title: '原书', audioSha256: 'hash', duration: 8, cues: [{ start: 0, end: 7, text: '原声字幕' }], scenes: [{ first: 0, last: 0, kind: 'opening', title: '观点', beats: [{ cue: 0, object: 'book', action: 'draw', label: '一本书' }] }] }, error: '', subtitleAssetId: 's1', videoAssetId: null, createdAt: '', updatedAt: '' }
const mounted: ReturnType<typeof mount>[] = []
beforeEach(() => {
  vi.resetAllMocks()
  vi.mocked(loadWaveform).mockResolvedValue([.2, .6, .8, .1])
  mockResizeObserver()
  vi.mocked(drafts.readDraft).mockResolvedValue(null)
  vi.mocked(drafts.saveDraft).mockResolvedValue(undefined)
  vi.mocked(api.originalCues).mockResolvedValue(job.plan.cues)
  vi.mocked(projects.getProject).mockResolvedValue({ ...emptyProject(), id: 'p1', title: '原书', workspacePath: 'D:/books', createdAt: '', updatedAt: '', archivedAt: null })
  vi.mocked(projects.listAssets).mockResolvedValue([{ id: 'a1', projectId: 'p1', assetType: 'audio', filePath: 'D:/原声.m4a', originalName: '原声.m4a', fileSize: 100, modifiedAt: 1, existsStatus: 'available', createdAt: '' }])
  vi.mocked(ai.listProviders).mockResolvedValue([{ id: 'm1', modelKind: 'text', name: '文本模型', modelName: 'text', reasoningEffort: null, baseUrl: 'http://localhost', isDefault: true, isEnabled: true, credentialStatus: 'available' }])
  vi.mocked(api.settings).mockResolvedValue({ engineRoot: 'D:/engine', nodePath: 'D:/node.exe', pythonPath: 'D:/python.exe', modelPath: 'D:/model' })
  vi.mocked(api.listJobs).mockResolvedValue([structuredClone(job)])
  vi.mocked(api.subtitles).mockResolvedValue('1\n00:00:00,000 --> 00:00:07,000\n原声字幕\n')
  vi.mocked(projects.mediaSource).mockResolvedValue('http://asset.localhost/audio.m4a')
})
afterEach(() => { mounted.splice(0).forEach(w => w.unmount()); document.body.innerHTML = ''; vi.useRealTimers(); vi.unstubAllGlobals() })
async function render(onStoryboardUpdate?: () => void, openSetup = true) {
  const router = createRouter({ history: createMemoryHistory(), routes: [{ path: '/projects/:id/video', component: VideoPage, props: true }, { path: '/:pathMatch(.*)*', component: { template: '<p>其他页面</p>' } }] })
  await router.push('/projects/p1/video?audio=a1'); await router.isReady()
  const wrapper = mount({ template: '<router-view />' }, { attachTo: document.body, global: { plugins: [router], mixins: [{ updated() { if (this.$options.name === 'UiSelect' && this.$el.parentElement?.closest('.video-scenes')) onStoryboardUpdate?.() } }] } }); mounted.push(wrapper)
  await flushPromises()
  if (openSetup && wrapper.get('.video-setup-trigger').attributes('aria-expanded') === 'false') {
    await wrapper.get('.video-setup-trigger').trigger('click')
    await flushPromises()
  }
  return wrapper
}
it('keeps existing tasks foremost and preserves setup choices and editor changes when collapsing settings', async () => {
  const wrapper = await render(undefined, false)
  expect(wrapper.get('.video-setup-trigger').attributes('aria-expanded')).toBe('false')
  expect(wrapper.get('.video-inputs').isVisible()).toBe(false)
  expect(wrapper.find('audio[aria-label="所选原声音频"]').exists()).toBe(false)
  await wrapper.get('.video-setup-trigger').trigger('click'); await flushPromises()
  const selector = wrapper.findAllComponents({ name: 'UiSelect' }).find(select => select.find('#video-audio').exists())!
  selector.vm.$emit('update:modelValue', 'another-audio'); await flushPromises()
  await wrapper.get('#video-subtitles').setValue('保留本次校对')
  await wrapper.get('.video-setup-trigger').trigger('click'); await flushPromises()
  expect(wrapper.find('audio[aria-label="所选原声音频"]').exists()).toBe(false)
  expect((wrapper.get('#video-subtitles').element as HTMLTextAreaElement).value).toBe('保留本次校对')
  await wrapper.get('.video-setup-trigger').trigger('click'); await flushPromises()
  expect(wrapper.findAllComponents({ name: 'UiSelect' }).find(select => select.find('#video-audio').exists())!.props('modelValue')).toBe('another-audio')
  expect(api.startJob).not.toHaveBeenCalled()
  expect(api.savePlan).not.toHaveBeenCalled()
})
it('opens creation settings for a book without video tasks', async () => {
  vi.mocked(api.listJobs).mockResolvedValue([])
  const wrapper = await render(undefined, false)
  expect(wrapper.get('.video-setup-trigger').attributes('aria-expanded')).toBe('true')
  expect(wrapper.find('#video-audio').exists()).toBe(true)
  expect(wrapper.text()).toContain('还没有视频任务')
  expect(wrapper.find('[aria-label="视频编辑分区"]').exists()).toBe(false)
})
it('retains the chosen environment check purpose and report when settings are collapsed', async () => {
  const wrapper = await render()
  await wrapper.get('.video-runtime-trigger').trigger('click'); await flushPromises()
  await chooseSelect(wrapper, '#environment-mode', '从原声新转写字幕')
  const settings = await api.settings()
  vi.mocked(api.checkEnvironment).mockResolvedValue({ settings, items: [{ id: 'python', status: 'error', version: '', path: settings.pythonPath, detail: '缺少语音识别依赖' }] })
  await wrapper.findAll('button').find(button => button.text() === '一键检查环境')!.trigger('click'); await flushPromises()
  expect(api.checkEnvironment).toHaveBeenLastCalledWith(settings, true)
  expect(wrapper.get('.environment-results').text()).toContain('缺少语音识别依赖')
  await wrapper.get('.video-setup-trigger').trigger('click'); await flushPromises()
  await wrapper.get('.video-setup-trigger').trigger('click'); await flushPromises()
  expect(wrapper.get('#environment-mode').text()).toContain('从原声新转写字幕')
  expect(wrapper.get('.environment-results').text()).toContain('缺少语音识别依赖')
  expect(api.checkEnvironment).toHaveBeenCalledOnce()
  expect(api.saveSettings).not.toHaveBeenCalled()
  await wrapper.findAll('button').find(button => button.text() === '一键检查环境')!.trigger('click'); await flushPromises()
  expect(api.checkEnvironment).toHaveBeenLastCalledWith(settings, true)
})
it('moves keyboard focus to each editor section without saving or changing unsaved content', async () => {
  const wrapper = await render(undefined, false)
  await wrapper.get('#video-subtitles').setValue('尚未保存的字幕')
  await wrapper.get('[aria-label="第1镜标题"]').setValue('尚未保存的分镜')
  const targets = ['subtitles', 'storyboard', 'preview']
  const buttons = wrapper.get('[aria-label="视频编辑分区"]').findAll('button')
  for (const [index, section] of targets.entries()) {
    const target = wrapper.get(`#video-workbench-${section}`).element as HTMLElement
    target.scrollIntoView = vi.fn()
    await buttons[index]!.trigger('click')
    expect(target.scrollIntoView).toHaveBeenCalledWith({ block: 'start' })
    expect(document.activeElement).toBe(target)
  }
  expect((wrapper.get('#video-subtitles').element as HTMLTextAreaElement).value).toBe('尚未保存的字幕')
  expect((wrapper.get('[aria-label="第1镜标题"]').element as HTMLInputElement).value).toBe('尚未保存的分镜')
  expect(api.savePlan).not.toHaveBeenCalled()
  expect(api.continueJob).not.toHaveBeenCalled()
})
it('adopts only the selected scene into its existing automatic draft without changing subtitles or other unsaved scenes', async () => {
  vi.useFakeTimers()
  const current = structuredClone(job)
  current.plan.duration = 12
  current.plan.cues = Array.from({ length: 3 }, (_, i) => ({ start: i * 4, end: i * 4 + 3, text: `字幕${i}` }))
  current.plan.scenes = current.plan.cues.map((_, i) => ({ first: i, last: i, kind: 'focus', title: `分镜${i}`, beats: [{ cue: i, object: 'book', action: 'draw', label: '原书' }] }))
  const srt = current.plan.cues.map((cue, i) => `${i+1}\n00:00:${String(cue.start).padStart(2, '0')},000 --> 00:00:${String(cue.end).padStart(2, '0')},000\n${cue.text}\n`).join('\n')
  vi.mocked(api.listJobs).mockResolvedValue([current]); vi.mocked(api.subtitles).mockResolvedValue(srt)
  const wrapper = await render()
  await wrapper.get('[aria-label="第1镜标题"]').setValue('其他镜头未保存的修改')
  await wrapper.get('[aria-label="重新生成第2镜"]').trigger('click'); await flushPromises()
  const dialog = wrapper.getComponent(VideoSceneRevision)
  expect(dialog.props('input').sceneIndex).toBe(1)
  expect(dialog.props('input').scenes[0]!.title).toBe('其他镜头未保存的修改')
  const revised = { ...current.plan.scenes[1]!, title: '只替换第二镜' }
  dialog.vm.$emit('adopt', revised); await flushPromises()
  await vi.advanceTimersByTimeAsync(400)
  const draft = vi.mocked(drafts.saveDraft).mock.lastCall![2]!.content as { subtitles: string; scenes: string }
  expect(draft.subtitles).toBe(srt)
  expect(JSON.parse(draft.scenes)).toEqual([{ ...current.plan.scenes[0], title: '其他镜头未保存的修改' }, revised, current.plan.scenes[2]])
  expect(api.savePlan).not.toHaveBeenCalled()
  expect(api.continueJob).not.toHaveBeenCalled()
  expect(api.startJob).not.toHaveBeenCalled()
})
it('rejects a candidate if its editor snapshot changed instead of overwriting newer work', async () => {
  const wrapper = await render()
  await wrapper.get('[aria-label="重新生成第1镜"]').trigger('click'); await flushPromises()
  const dialog = wrapper.getComponent(VideoSceneRevision)
  wrapper.getComponent(VideoSubtitleEditor).vm.$emit('update:modelValue', '更新后的字幕'); await flushPromises()
  dialog.vm.$emit('adopt', { ...job.plan.scenes[0], title: '过期候选' }); await flushPromises()
  expect((wrapper.get('[aria-label="第1镜标题"]').element as HTMLInputElement).value).toBe('观点')
  expect((wrapper.get('#video-subtitles').element as HTMLTextAreaElement).value).toBe('更新后的字幕')
  expect(wrapper.findComponent(VideoSceneRevision).exists()).toBe(false)
})
it('restores the last edited task, unsaved SRT, storyboard and cursor after remounting without saving a plan', async () => {
  const content = { subtitles: '还没写完的 SRT', scenes: JSON.stringify([{ ...job.plan.scenes[0], title: '暂存镜头' }]), position: { section: 'srt', index: 0, onlyChanges: true, cursor: 4 } }
  vi.mocked(drafts.readDraft).mockResolvedValue({ basedOn: '1', content })
  vi.mocked(api.listJobs).mockResolvedValue([{ ...structuredClone(job), id: 'newer' }, { ...structuredClone(job), draftUpdatedAt: '2026-11-03T10:00:00Z' }])
  const wrapper = await render()
  expect(drafts.readDraft).toHaveBeenCalledWith('j1', 'video')
  expect((wrapper.get('#video-subtitles').element as HTMLTextAreaElement).value).toBe(content.subtitles)
  expect((wrapper.get('#video-subtitles').element as HTMLTextAreaElement).selectionStart).toBe(4)
  expect((wrapper.get('[aria-label="第1镜标题"]').element as HTMLInputElement).value).toBe('暂存镜头')
  expect(api.savePlan).not.toHaveBeenCalled()
})
it('autosaves incomplete text and blocks navigation on failure until a successful retry', async () => {
  vi.useFakeTimers()
  const wrapper = await render()
  vi.mocked(drafts.saveDraft).mockRejectedValue(new Error('磁盘写入失败'))
  await wrapper.get('#video-subtitles').setValue('未完成的时间码')
  await vi.advanceTimersByTimeAsync(400)
  expect(wrapper.text()).toContain('自动暂存失败')
  await wrapper.vm.$router.push('/projects')
  expect(wrapper.vm.$router.currentRoute.value.path).toBe('/projects/p1/video')
  vi.mocked(drafts.saveDraft).mockResolvedValue(undefined)
  await wrapper.vm.$router.push('/projects')
  expect(wrapper.vm.$router.currentRoute.value.path).toBe('/projects')
  expect(drafts.saveDraft).toHaveBeenLastCalledWith('j1', 'video', expect.objectContaining({ content: expect.objectContaining({ subtitles: '未完成的时间码' }) }))
  expect(api.savePlan).not.toHaveBeenCalled()
})
it.each(['running', 'succeeded'] as const)('refreshes an externally advanced transcript task to %s and releases navigation after a stale draft rejection', async status => {
  vi.useFakeTimers()
  vi.mocked(api.listJobs).mockResolvedValue([{ ...structuredClone(job), status: 'draft', stage: 'transcript_ready' }])
  const wrapper = await render()
  vi.mocked(api.listJobs).mockResolvedValue([{ ...structuredClone(job), status, stage: status === 'running' ? 'rendering' : 'complete', runId: 2 }])
  vi.mocked(drafts.saveDraft).mockRejectedValue(new Error('执行中或已完成的视频不能暂存编辑。'))
  await wrapper.vm.$router.push('/projects/p1/publishing?task=published-task')
  expect(wrapper.vm.$router.currentRoute.value.path).toBe('/projects/p1/video')
  expect(wrapper.text()).toContain('自动暂存失败')
  const writes = vi.mocked(drafts.saveDraft).mock.calls.length
  await vi.advanceTimersByTimeAsync(1500)
  expect(wrapper.find('#video-subtitles').exists()).toBe(false)
  expect(wrapper.text()).not.toContain('自动暂存失败')
  await wrapper.vm.$router.push('/projects/p1/publishing?task=published-task')
  expect(wrapper.vm.$router.currentRoute.value.fullPath).toBe('/projects/p1/publishing?task=published-task')
  expect(drafts.saveDraft).toHaveBeenCalledTimes(writes)
  expect(api.startJob).not.toHaveBeenCalled()
  expect(api.continueJob).not.toHaveBeenCalled()
})
it('keeps unsaved manual edits when polling an unchanged waiting task', async () => {
  vi.useFakeTimers()
  const wrapper = await render()
  await wrapper.get('#video-subtitles').setValue('保留人工修改的字幕')
  await vi.advanceTimersByTimeAsync(1500)
  expect(api.listJobs).toHaveBeenCalledTimes(2)
  expect((wrapper.get('#video-subtitles').element as HTMLTextAreaElement).value).toBe('保留人工修改的字幕')
  expect(api.subtitles).toHaveBeenCalledOnce()
  expect(api.savePlan).not.toHaveBeenCalled()
})
it('flushes the old task before switching, then restores the other task without mixing drafts', async () => {
  vi.useFakeTimers()
  const other = { ...structuredClone(job), id: 'j2', runId: 2, createdAt: '2026-11-02T09:00:00Z' }
  vi.mocked(api.listJobs).mockResolvedValue([structuredClone(job), other])
  const wrapper = await render()
  await wrapper.get('#video-subtitles').setValue('任务一的修改')
  let finish!: () => void
  vi.mocked(drafts.saveDraft).mockReturnValueOnce(new Promise(resolve => { finish = resolve }))
  const selector = wrapper.findAllComponents({ name: 'UiSelect' }).find(select => select.find('[aria-label="选择视频历史任务"]').exists())!
  selector.vm.$emit('update:modelValue', 'j2'); await flushPromises()
  expect(drafts.readDraft).not.toHaveBeenCalledWith('j2', 'video')
  finish(); await flushPromises()
  expect(drafts.readDraft).toHaveBeenCalledWith('j2', 'video')
  expect((wrapper.get('#video-subtitles').element as HTMLTextAreaElement).value).toContain('原声字幕')
  await wrapper.get('#video-subtitles').setValue('任务二的修改')
  await vi.advanceTimersByTimeAsync(400)
  expect(drafts.saveDraft).toHaveBeenLastCalledWith('j2', 'video', expect.objectContaining({ basedOn: '2', content: expect.objectContaining({ subtitles: '任务二的修改' }) }))
})
it('keeps task audition bound to its source when the new-task audio selector changes', async () => {
  const wrapper = await render()
  const selector = wrapper.findAllComponents({ name: 'UiSelect' }).find(select => select.find('#video-audio').exists())!
  vi.mocked(projects.mediaSource).mockResolvedValue('http://asset.localhost/another-audio.m4a')
  selector.vm.$emit('update:modelValue', 'a2'); await flushPromises()
  expect(wrapper.get('audio[aria-label="所选原声音频"]').attributes('src')).toContain('another-audio')
  expect(wrapper.get('audio[aria-label="当前视频任务原声"]').attributes('src')).toBe('http://asset.localhost/audio.m4a')
})
it('never offers the previous editor content for saving when another task draft fails to load', async () => {
  vi.mocked(api.listJobs).mockResolvedValue([structuredClone(job), { ...structuredClone(job), id: 'j2' }])
  const wrapper = await render()
  vi.mocked(drafts.readDraft).mockRejectedValueOnce(new Error('读取失败'))
  const selector = wrapper.findAllComponents({ name: 'UiSelect' }).find(select => select.find('[aria-label="选择视频历史任务"]').exists())!
  selector.vm.$emit('update:modelValue', 'j2'); await flushPromises()
  expect(wrapper.find('#video-subtitles').exists()).toBe(false)
  expect(wrapper.find('[data-test="render-video"]').exists()).toBe(false)
  expect(wrapper.text()).toContain('重新读取字幕与草稿')
  expect(api.savePlan).not.toHaveBeenCalled()
})
it('keeps storyboard controls idle while typing a long SRT and saves the latest input immediately', async () => {
  const current = structuredClone(job)
  current.plan.duration = 1200
  current.plan.cues = Array.from({ length: 64 }, (_, i) => ({ start: i * 1.8, end: i * 1.8 + 1.5, text: `原声字幕${i + 1}` }))
  current.plan.scenes = Array.from({ length: 12 }, (_, i) => {
    const first = Math.floor(i * 64 / 12)
    return { first, last: Math.floor((i + 1) * 64 / 12) - 1, kind: 'focus', title: `观点${i + 1}`, beats: [{ cue: first, object: 'book', action: 'draw', label: '原书' }] }
  })
  const clock = (seconds: number) => new Date(seconds * 1000).toISOString().slice(11, 23).replace('.', ',')
  const srt = current.plan.cues.map((cue, i) => `${i + 1}\n${clock(cue.start)} --> ${clock(cue.end)}\n${cue.text}\n`).join('\n')
  vi.mocked(api.listJobs).mockResolvedValue([current])
  vi.mocked(api.subtitles).mockResolvedValue(srt)
  const updates = vi.fn()
  const wrapper = await render(updates)
  updates.mockClear()
  const stringify = vi.spyOn(JSON, 'stringify')
  await wrapper.get('#video-subtitles').setValue(`${srt}校`)
  await wrapper.get('#video-subtitles').setValue(`${srt}校对`)
  expect(stringify.mock.calls.filter(([value]) => Array.isArray(value) && Array.isArray(value[1]))).toHaveLength(0)
  expect(updates).not.toHaveBeenCalled()
  await wrapper.findAll('button').find(button => button.text() === '保存校对')!.trigger('click')
  await flushPromises()
  expect(api.savePlan).toHaveBeenCalledExactlyOnceWith('j1', `${srt}校对`, current.plan.scenes)
}, 30000)
it('requires an explicit confirmation before rendering a prepared storyboard', async () => {
  const wrapper = await render()
  const scroll = vi.fn()
  wrapper.get('.video-task-workspace').element.scrollIntoView = scroll
  expect(wrapper.text()).toContain('等待确认分镜')
  expect(api.continueJob).not.toHaveBeenCalled()
  vi.mocked(api.continueJob).mockResolvedValue({ ...job, status: 'running', stage: 'rendering' })
  await wrapper.get('[data-test="render-video"]').trigger('click'); await flushPromises()
  expect(api.savePlan).toHaveBeenCalledWith('j1', expect.stringContaining('原声字幕'), job.plan.scenes)
  expect(api.continueJob).toHaveBeenCalledWith('j1', true, null)
  expect(wrapper.get('[aria-label="选择视频历史任务"]').text()).toContain('制作中')
  expect(scroll).toHaveBeenCalledWith({ block: 'start' })
})
it('mounts beat selectors only when expanded and preserves edits after closing and reopening', async () => {
  const wrapper = await render()
  expect(wrapper.find('#scene-0-object-0').exists()).toBe(false)
  const toggle = wrapper.get('.video-beat-settings .disclosure-trigger')
  await toggle.trigger('click'); await flushPromises()
  await chooseSelect(wrapper, '#scene-0-object-0', '钢笔')
  await toggle.trigger('click'); await flushPromises()
  expect(wrapper.find('#scene-0-object-0').exists()).toBe(false)
  await toggle.trigger('click'); await flushPromises()
  expect(wrapper.get('#scene-0-object-0').text()).toContain('钢笔')
  await wrapper.findAll('button').find(button => button.text() === '保存校对')!.trigger('click')
  await flushPromises()
  expect(api.savePlan).toHaveBeenCalledWith('j1', expect.any(String), [expect.objectContaining({ beats: [expect.objectContaining({ object: 'pen' })] })])
})
it('a failed save keeps the edited subtitles and never starts a render', async () => {
  const wrapper = await render()
  await wrapper.get('#video-subtitles').setValue('未保存的校对')
  vi.mocked(api.savePlan).mockRejectedValue(new Error('保存失败'))
  await wrapper.get('[data-test="render-video"]').trigger('click'); await flushPromises()
  expect((wrapper.get('#video-subtitles').element as HTMLTextAreaElement).value).toBe('未保存的校对')
  expect(api.continueJob).not.toHaveBeenCalled()
})

it('saves the same storyboard before a preview and never starts final rendering', async () => {
  const wrapper = await render()
  wrapper.get('.video-job').element.scrollIntoView = vi.fn()
  await wrapper.get('[aria-label="第1镜物体1标签"]').setValue('校对后')
  vi.mocked(api.previewJob).mockResolvedValue({ ...job, status: 'running', stage: 'previewing', runId: 2 })
  await wrapper.get('[data-test="preview-video"]').trigger('click'); await flushPromises()
  expect(api.savePlan).toHaveBeenCalledWith('j1', expect.any(String), [expect.objectContaining({ beats: [expect.objectContaining({ label: '校对后' })] })])
  expect(api.previewJob).toHaveBeenCalledWith('j1', 0, 30)
  expect(api.continueJob).not.toHaveBeenCalled()
  expect(api.startJob).not.toHaveBeenCalled()
  expect(wrapper.text()).toContain('试片')
})

it('shows renderer thumbnails and a clearly marked preview, then hides them as soon as edits are made', async () => {
  vi.mocked(api.listJobs).mockResolvedValue([{ ...job, stage: 'previewing', runId: 2, preview: { start: 0, end: 8, signature: 'saved', scenes: [0] } }])
  vi.mocked(api.previewSource).mockImplementation(async (_project, _id, _run, scene) => scene === undefined ? 'http://asset.localhost/preview.mp4' : 'http://asset.localhost/scene-0.png')
  const wrapper = await render()
  expect(wrapper.get('video[aria-label="原声试片（非正式成片）"]').attributes('src')).toContain('preview.mp4')
  expect(wrapper.get('img[alt="第1镜渲染缩略图"]').attributes('src')).toContain('scene-0.png')
  expect(wrapper.text()).toContain('试片不是正式成片')
  expect(wrapper.find('video[aria-label="原声 MG 视频成片"]').exists()).toBe(false)
  await wrapper.get('[aria-label="第1镜标题"]').setValue('已改变')
  await flushPromises()
  expect(wrapper.find('video[aria-label="原声试片（非正式成片）"]').exists()).toBe(false)
  expect(api.previewJob).not.toHaveBeenCalled()
  expect(api.continueJob).not.toHaveBeenCalled()
})

it('does not revive the previous preview after saving edits when the next preview cannot start', async () => {
  vi.mocked(api.listJobs).mockResolvedValue([{ ...job, stage: 'previewing', runId: 2, preview: { start: 0, end: 8, signature: 'saved', scenes: [] } }])
  vi.mocked(api.previewSource).mockResolvedValue('http://asset.localhost/old-preview.mp4')
  vi.mocked(api.previewJob).mockRejectedValue(new Error('正在停止上次进程'))
  const wrapper = await render()
  expect(wrapper.find('video[aria-label="原声试片（非正式成片）"]').exists()).toBe(true)
  await wrapper.get('[aria-label="第1镜标题"]').setValue('修改后')
  await wrapper.get('[data-test="preview-video"]').trigger('click'); await flushPromises()
  expect(api.previewJob).toHaveBeenCalledWith('j1', 0, 30)
  expect(wrapper.find('video[aria-label="原声试片（非正式成片）"]').exists()).toBe(false)
  expect(api.continueJob).not.toHaveBeenCalled()
})

it('keeps the selected preview scene title in sync with the existing editor state', async () => {
  const wrapper = await render()
  await wrapper.get('[aria-label="第1镜标题"]').setValue('新的镜头主题')
  expect(wrapper.get('#preview-first-scene').text()).toContain('新的镜头主题')
})
it('edits environment and framing in the same plan, invalidates the preview and saves both before rendering', async () => {
  vi.mocked(api.listJobs).mockResolvedValue([{ ...structuredClone(job), stage: 'previewing', runId: 2, preview: { start: 0, end: 8, signature: 'saved', scenes: [0] } }])
  vi.mocked(api.previewSource).mockResolvedValue('http://asset.localhost/preview.mp4')
  const wrapper = await render()
  expect(wrapper.get('[aria-label="第1镜场景"]').text()).toContain('空纸')
  expect(wrapper.get('[aria-label="第1镜景别"]').text()).toContain('原布局')
  const environment = wrapper.findAllComponents({ name: 'UiSelect' }).find(select => select.find('[aria-label="第1镜场景"]').exists())!
  environment.vm.$emit('update:modelValue', 'study')
  await flushPromises()
  const framing = wrapper.findAllComponents({ name: 'UiSelect' }).find(select => select.find('[aria-label="第1镜景别"]').exists())!
  framing.vm.$emit('update:modelValue', 'detail')
  await flushPromises()
  expect(wrapper.get('[aria-label="第1镜场景"]').text()).toContain('书房案头')
  expect(wrapper.get('[aria-label="第1镜景别"]').text()).toContain('特写')
  expect(wrapper.find('video[aria-label="原声试片（非正式成片）"]').exists()).toBe(false)
  vi.mocked(api.continueJob).mockResolvedValue({ ...job, status: 'running', stage: 'rendering' })
  await wrapper.get('[data-test="render-video"]').trigger('click'); await flushPromises()
  expect(api.savePlan).toHaveBeenCalledWith('j1', expect.stringContaining('原声字幕'), [expect.objectContaining({ environment: 'study', framing: 'detail' })])
  expect(api.continueJob).toHaveBeenCalledWith('j1', true, null)
  expect(job.plan.scenes[0]?.environment).toBeUndefined()
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
  expect(api.continueJob).toHaveBeenCalledExactlyOnceWith('j1', true, null)
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
  expect(wrapper.text()).not.toContain('当前制作任务')
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
  expect(wrapper.find('[data-test="render-video"]').exists()).toBe(false)
  vi.mocked(api.continueJob).mockResolvedValue({ ...job, status: 'running', stage: 'storyboard' })
  const upgrade = wrapper.findAll('button').find(button => button.text().includes('保留字幕并升级手绘分镜'))!
  await upgrade.trigger('click'); await flushPromises()
  expect(api.savePlan).toHaveBeenCalledWith('j1', expect.stringContaining('原声字幕'), [])
  expect(api.continueJob).toHaveBeenCalledWith('j1', false, 'm1')
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

it('saves an image scene as a complete plan without treating it as a legacy text storyboard', async () => {
  const image = { assetId: 'img1', source: '本人拍摄', rights: '自有照片', fit: 'cover' as const, x: .25, y: .75, sha256: 'a'.repeat(64) }
  const current = structuredClone(job)
  current.plan.scenes[0]!.beats = []
  current.plan.scenes[0]!.image = image
  vi.mocked(api.listJobs).mockResolvedValue([current])
  vi.mocked(api.readImage).mockResolvedValue(new ArrayBuffer(0))
  const wrapper = await render()
  expect(wrapper.text()).not.toContain('当前是旧版文字图解')
  await wrapper.get('#video-image-0-rights').setValue('本人拍摄，允许本次视频使用')
  vi.mocked(api.continueJob).mockResolvedValue({ ...current, status: 'running', stage: 'rendering' })
  await wrapper.get('[data-test="render-video"]').trigger('click'); await flushPromises()
  expect(api.savePlan).toHaveBeenCalledWith('j1', expect.any(String), [expect.objectContaining({ beats: [], image: expect.objectContaining({ assetId: 'img1', rights: '本人拍摄，允许本次视频使用', x: .25, y: .75 }) })])
  expect(api.continueJob).toHaveBeenCalledWith('j1', true, null)
})


it('restores the task model instead of the global default when continuing a failed storyboard', async () => {
  const defaultModel = { ...emptyProvider(), id: 'default', name: '默认模型', isDefault: true, credentialStatus: 'available' as const }
  const selectedModel = { ...defaultModel, id: 'selected', name: '任务所选模型', isDefault: false }
  vi.mocked(ai.listProviders).mockResolvedValue([defaultModel, selectedModel])
  const failed = { ...structuredClone(job), status: 'failed' as const, providerId: 'selected' }
  vi.mocked(api.listJobs).mockResolvedValue([failed])
  vi.mocked(api.continueJob).mockResolvedValue({ ...failed, status: 'running' })
  const wrapper = await render()
  expect(wrapper.get('#video-provider').text()).toContain('任务所选模型')
  wrapper.get('.video-job').element.scrollIntoView = vi.fn()
  await wrapper.findAll('button').find(button => button.text() === '继续准备字幕与分镜')!.trigger('click')
  await flushPromises()
  expect(api.continueJob).toHaveBeenCalledExactlyOnceWith('j1', false, 'selected')
})

it('requires an explicit replacement when the saved task model is unavailable', async () => {
  vi.mocked(api.listJobs).mockResolvedValue([{ ...structuredClone(job), status: 'failed', providerId: 'removed-model' }])
  const wrapper = await render()
  expect(wrapper.findAll('button').find(button => button.text() === '继续准备字幕与分镜')!.attributes('disabled')).toBeDefined()
  expect(api.continueJob).not.toHaveBeenCalled()
})

it.each(['继续准备字幕与分镜', '保存字幕并重新生成分镜'])('uses the newly selected model for %s on an existing task', async action => {
  const oldModel = { ...emptyProvider(), id: 'm1', name: '旧文本模型', modelName: 'old-model', baseUrl: 'http://localhost:1111/v1', isDefault: true, credentialStatus: 'available' as const }
  const newModel = { ...oldModel, id: 'm2', name: '新文本模型', modelName: 'new-model', baseUrl: 'http://localhost:2222/v1', isDefault: false }
  vi.mocked(ai.listProviders).mockResolvedValue([oldModel, newModel])
  const failedJob = structuredClone(job)
  failedJob.status = 'failed'; failedJob.error = 'HTTP 524'
  failedJob.plan.duration = 12
  failedJob.plan.cues.push({ start: 8, end: 11, text: '尚未生成分镜的字幕' })
  vi.mocked(api.listJobs).mockResolvedValue([failedJob])
  const wrapper = await render()
  wrapper.get('.video-job').element.scrollIntoView = vi.fn()
  const subtitles = (wrapper.get('#video-subtitles').element as HTMLTextAreaElement).value
  await chooseSelect(wrapper, '#video-provider', '新文本模型')
  expect(wrapper.get('#video-provider').text()).toContain('新文本模型')
  vi.mocked(api.continueJob).mockResolvedValue({ ...failedJob, providerId: 'm2', status: 'running', error: '', runId: 2, plan: { ...failedJob.plan, scenes: action === '继续准备字幕与分镜' ? failedJob.plan.scenes : [] } })
  await wrapper.findAll('button').find(button => button.text() === action)!.trigger('click')
  await flushPromises()
  expect(api.continueJob).toHaveBeenCalledExactlyOnceWith('j1', false, 'm2')
  expect(api.startJob).not.toHaveBeenCalled()
  expect(wrapper.get('.output-summary').text()).toMatch(/2\s*条/)
  if (action === '继续准备字幕与分镜') {
    expect(api.savePlan).not.toHaveBeenCalled()
    expect(wrapper.get('.output-preview').text()).toContain('观点')
  } else {
    expect(api.savePlan).toHaveBeenCalledExactlyOnceWith('j1', subtitles, [])
  }
})


it('blocks both storyboard retry actions when no enabled text model is available', async () => {
  vi.mocked(ai.listProviders).mockResolvedValue([])
  vi.mocked(api.listJobs).mockResolvedValue([{ ...structuredClone(job), status: 'failed' }])
  const wrapper = await render()
  for (const label of ['继续准备字幕与分镜', '保存字幕并重新生成分镜']) {
    expect(wrapper.findAll('button').find(button => button.text() === label)!.attributes('disabled')).toBeDefined()
  }
  expect(api.continueJob).not.toHaveBeenCalled()
  expect(api.savePlan).not.toHaveBeenCalled()
})
