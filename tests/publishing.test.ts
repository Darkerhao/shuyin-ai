import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createMemoryHistory, createRouter } from 'vue-router'
import App from '../src/App.vue'
import PublishingPage from '../src/modules/publishing/PublishingPage.vue'
import * as api from '../src/services/publishing'
import * as projects from '../src/services/projects'
import * as ai from '../src/services/ai'
import * as covers from '../src/services/covers'
import { emptyProject, type Asset } from '../src/types/project'
import { newCover } from '../src/types/cover'
import type { PublishTask } from '../src/types/publishing'
vi.mock('../src/services/publishing', () => ({ listPublishTasks: vi.fn(), savePublishTask: vi.fn(), preparePublishTask: vi.fn(), recordPublishSuccess: vi.fn(), openPublishItem: vi.fn() }))
vi.mock('../src/services/projects', () => ({ isDesktop: vi.fn(), getProject: vi.fn(), listAssets: vi.fn(), linkAsset: vi.fn(), chooseAsset: vi.fn(), unlinkAsset: vi.fn(), openAsset: vi.fn() }))
vi.mock('../src/services/ai', () => ({ listVersions: vi.fn() }))
vi.mock('../src/services/covers', () => ({ listCovers: vi.fn() }))
const task: PublishTask = { id: 't1', projectId: 'p1', title: '中文标题', description: '第一行\n第二行', tags: '#读书', videoAssetId: 'a1', landscapeCoverId: 'c1', portraitCoverId: 'c2', titleConfirmed: true, coversConfirmed: true, status: 'ready', taskDirectory: 'D:/任务包', createdAt: '2026-10-24T00:00:00Z', updatedAt: '2026-10-24T00:00:00Z', checklist: [{ key: 'video', label: '视频已关联且文件可用', passed: true }], record: null }
const asset: Asset = { id: 'a1', projectId: 'p1', assetType: 'video', originalName: '视频.mp4', filePath: 'D:/视频.mp4', fileSize: 100, modifiedAt: 0, existsStatus: 'available', createdAt: '' }
const wrappers: ReturnType<typeof mount>[] = []
async function render() {
  const router = createRouter({ history: createMemoryHistory(), routes: [
    { path: '/projects/:id/publishing', component: PublishingPage, props: true },
    ...['/projects', '/projects/:id/writing', '/projects/:id/covers', '/settings/models'].map(path => ({ path, component: { template: '<p>other page</p>' } })),
  ] })
  await router.push('/projects/p1/publishing'); await router.isReady()
  const wrapper = mount(App, { global: { plugins: [router] } }); wrappers.push(wrapper); await flushPromises()
  return { wrapper, router }
}
function button(w: ReturnType<typeof mount>, label: string) { return w.findAll('button').find(b => b.text() === label)! }
beforeEach(() => {
  vi.resetAllMocks(); vi.stubGlobal('confirm', vi.fn(() => false))
  vi.mocked(projects.isDesktop).mockReturnValue(true)
  vi.mocked(projects.getProject).mockResolvedValue({ ...emptyProject(), id: 'p1', title: '测试书', workspacePath: 'D:/书籍', createdAt: '', updatedAt: '', archivedAt: null })
  vi.mocked(projects.listAssets).mockResolvedValue([asset])
  vi.mocked(api.listPublishTasks).mockResolvedValue([{ ...task }])
  vi.mocked(ai.listVersions).mockResolvedValue([])
  vi.mocked(covers.listCovers).mockResolvedValue([
    { id: 'c1', projectId: 'p1', config: newCover('横版'), versionNo: 1, exportedPath: 'D:/横.png', createdAt: '' },
    { id: 'c2', projectId: 'p1', config: { ...newCover('竖版'), width: 1080, height: 1440 }, versionNo: 2, exportedPath: 'D:/竖.png', createdAt: '' },
  ])
  vi.mocked(api.savePublishTask).mockImplementation(async (_, input) => ({ ...task, ...input, status: 'draft', taskDirectory: null }))
  vi.mocked(api.preparePublishTask).mockResolvedValue({ ...task })
})
afterEach(() => { wrappers.splice(0).forEach(w => w.unmount()); vi.unstubAllGlobals() })

describe('M4 publish preparation', () => {
  it('does not access local data in browser preview', async () => {
    vi.mocked(projects.isDesktop).mockReturnValue(false)
    const { wrapper } = await render(); expect(api.listPublishTasks).not.toHaveBeenCalled(); expect(wrapper.text()).toContain('请在桌面应用中准备发布')
  })
  it('starts from saved writing and exported covers without requiring a model', async () => {
    vi.mocked(api.listPublishTasks).mockResolvedValue([])
    vi.mocked(ai.listVersions).mockResolvedValue([{ id: 'w1', projectId: 'p1', kind: 'publishing', content: { format: 'publishing', title: '已有发布标题', description: '已有内容', audioPrompt: '提示词' }, versionNo: 1, sourceType: 'manual', modelName: '', promptVersion: '', isConfirmed: false, createdAt: '' }])
    const { wrapper } = await render()
    expect((wrapper.get('#publish-title').element as HTMLInputElement).value).toBe('已有发布标题')
    expect((wrapper.get('#publish-landscape').element as HTMLSelectElement).value).toBe('c1')
    expect((wrapper.get('#title-confirmed').element as HTMLInputElement).checked).toBe(false)
    await wrapper.get('form').trigger('submit'); await flushPromises()
    expect(api.savePublishTask).toHaveBeenCalledWith(null, expect.objectContaining({ title: '已有发布标题', landscapeCoverId: 'c1', portraitCoverId: 'c2', titleConfirmed: false }))
  })
  it('invalidates confirmation after edits and preserves draft on save failure and navigation', async () => {
    const { wrapper, router } = await render()
    await wrapper.get('#publish-title').setValue('新标题')
    expect((wrapper.get('#title-confirmed').element as HTMLInputElement).checked).toBe(false)
    vi.mocked(api.savePublishTask).mockRejectedValueOnce('保存失败')
    await button(wrapper, '生成完整任务包').trigger('click'); await flushPromises()
    expect(api.preparePublishTask).not.toHaveBeenCalled(); expect(wrapper.text()).toContain('保存失败')
    expect((wrapper.get('#publish-title').element as HTMLInputElement).value).toBe('新标题')
    await router.push('/projects'); expect(router.currentRoute.value.path).toContain('/publishing')
  })
  it('saves before preparing and retains persisted draft after package failure', async () => {
    const { wrapper } = await render()
    await wrapper.get('#publish-description').setValue('新内容\n保留换行')
    vi.mocked(api.preparePublishTask).mockRejectedValueOnce('任务包生成失败：磁盘不可写')
    await button(wrapper, '生成完整任务包').trigger('click'); await flushPromises()
    expect(api.savePublishTask).toHaveBeenCalledOnce(); expect(api.preparePublishTask).toHaveBeenCalledWith('t1')
    expect(wrapper.text()).toContain('磁盘不可写'); expect((wrapper.get('#publish-description').element as HTMLTextAreaElement).value).toBe('新内容\n保留换行')
    expect(button(wrapper, '记录发布成功').element.disabled).toBe(true)
  })
  it('copies exact Chinese text and explains clipboard failure', async () => {
    const writeText = vi.fn().mockResolvedValue(undefined); vi.stubGlobal('navigator', { clipboard: { writeText } })
    const { wrapper } = await render(); await button(wrapper, '一键复制全部').trigger('click'); await flushPromises()
    expect(writeText).toHaveBeenCalledWith('中文标题\n\n第一行\n第二行\n\n#读书')
    writeText.mockRejectedValueOnce(new Error('denied')); await button(wrapper, '复制内容').trigger('click'); await flushPromises()
    expect(wrapper.text()).toContain('复制失败')
  })
  it('refreshes missing-video state without discarding current edits', async () => {
    const { wrapper } = await render(); await wrapper.get('#publish-tags').setValue('#新标签')
    vi.mocked(projects.listAssets).mockResolvedValue([{ ...asset, existsStatus: 'missing' }])
    vi.mocked(api.listPublishTasks).mockResolvedValue([{ ...task, status: 'draft' }])
    await button(wrapper, '刷新素材与状态').trigger('click'); await flushPromises()
    expect((wrapper.get('#publish-tags').element as HTMLInputElement).value).toBe('#新标签')
    expect(wrapper.text()).toContain('视频文件不可用'); expect(button(wrapper, '记录发布成功').element.disabled).toBe(true)
  })
  it('opening portal does not publish; confirmation cancellation and failed record preserve data', async () => {
    const { wrapper } = await render(); await button(wrapper, '打开抖音创作者中心').trigger('click'); await flushPromises()
    expect(api.openPublishItem).toHaveBeenCalledWith('t1', 'portal'); expect(api.recordPublishSuccess).not.toHaveBeenCalled()
    await button(wrapper, '记录发布成功').trigger('click'); await button(wrapper, '取消').trigger('click'); expect(api.recordPublishSuccess).not.toHaveBeenCalled()
    await button(wrapper, '记录发布成功').trigger('click'); await wrapper.get('#publish-notes').setValue('真实发布备注')
    vi.mocked(api.recordPublishSuccess).mockRejectedValueOnce('写入失败')
    await wrapper.get('[role="alertdialog"]').trigger('submit'); await flushPromises()
    expect(wrapper.find('[role="alertdialog"]').exists()).toBe(true); expect((wrapper.get('#publish-notes').element as HTMLTextAreaElement).value).toBe('真实发布备注')
    vi.mocked(api.recordPublishSuccess).mockResolvedValue({ ...task, status: 'published', record: { publishedAt: task.createdAt, postUrl: '', notes: '真实发布备注' } })
    await wrapper.get('[role="alertdialog"]').trigger('submit'); await flushPromises()
    expect(api.recordPublishSuccess).toHaveBeenLastCalledWith('t1', expect.objectContaining({ confirmed: true, notes: '真实发布备注' }))
    expect(wrapper.find('[role="alertdialog"]').exists()).toBe(false); expect(wrapper.text()).toContain('已记录发布成功')
    expect(wrapper.findAll('fieldset')[0]!.element.disabled).toBe(true)
  })
  it('locks task editing and repeated confirmation while entering the publication result', async () => {
    const { wrapper } = await render()
    await button(wrapper, '记录发布成功').trigger('click')
    await wrapper.get('#publish-notes').setValue('正在填写的备注')
    expect(wrapper.get('#publish-title').element.matches(':disabled')).toBe(true)
    expect(wrapper.get('#publish-video').element.matches(':disabled')).toBe(true)
    expect(button(wrapper, '关联素材').element.matches(':disabled')).toBe(true)
    expect(button(wrapper, '记录发布成功').element.disabled).toBe(true)
    expect(wrapper.get('#publish-notes').element.matches(':disabled')).toBe(false)
    await button(wrapper, '取消').trigger('click')
    expect(wrapper.get('#publish-title').element.matches(':disabled')).toBe(false)
    expect(api.recordPublishSuccess).not.toHaveBeenCalled()
  })
})
