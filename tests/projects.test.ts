import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createMemoryHistory, createRouter } from 'vue-router'
import App from '../src/App.vue'
import ProjectsPage from '../src/modules/projects/ProjectsPage.vue'
import ProjectPage from '../src/modules/projects/ProjectPage.vue'
import ProjectAssets from '../src/modules/projects/ProjectAssets.vue'
import SettingsPage from '../src/modules/settings/SettingsPage.vue'
import * as api from '../src/services/projects'
import { emptyProject, type Asset, type Project } from '../src/types/project'
vi.mock('../src/services/projects', async (original) => ({
  ...await original<typeof api>(),
  isDesktop: vi.fn(), listProjects: vi.fn(), getProject: vi.fn(), saveProject: vi.fn(), deleteProject: vi.fn(),
  listAssets: vi.fn(), linkAsset: vi.fn(), unlinkAsset: vi.fn(), chooseAsset: vi.fn(),
  workspaceSettings: vi.fn(), chooseDirectory: vi.fn(), setWorkspaceRoot: vi.fn(), chooseBackup: vi.fn(), createBackup: vi.fn(), restoreBackup: vi.fn(),
}))
const saved: Project = { ...emptyProject(), id: 'p1', title: '活着', author: '余华', workspacePath: 'D:/books/p1', createdAt: '2026-10-23T00:00:00Z', updatedAt: '2026-10-23T00:00:00Z', archivedAt: null }
const asset: Asset = { id: 'a1', projectId: 'p1', assetType: 'video', filePath: 'D:/视频.mp4', originalName: '视频.mp4', fileSize: 1000, modifiedAt: 1, existsStatus: 'missing', createdAt: saved.createdAt }
const wrappers: ReturnType<typeof mount>[] = []
async function render(path: string) {
  const router = createRouter({ history: createMemoryHistory(), routes: [
    { path: '/projects', component: ProjectsPage }, { path: '/projects/new', component: ProjectPage },
    { path: '/projects/:id', component: ProjectPage, props: true }, { path: '/projects/:id/writing', component: { template: '<p>发布文案与音频提示词</p>' } }, { path: '/assets', component: ProjectAssets, props: { projectId: 'p1' } }, { path: '/settings', component: SettingsPage },
  ] })
  await router.push(path); await router.isReady()
  const wrapper = mount(App, { global: { plugins: [router] } })
  wrappers.push(wrapper); await flushPromises()
  return { wrapper, router }
}
function button(wrapper: ReturnType<typeof mount>, text: string) { return wrapper.findAll('button').find(b => b.text() === text)! }
beforeEach(() => {
  vi.resetAllMocks()
  vi.mocked(api.isDesktop).mockReturnValue(true)
  vi.mocked(api.listProjects).mockResolvedValue([saved])
  vi.mocked(api.getProject).mockResolvedValue(saved)
  vi.mocked(api.listAssets).mockResolvedValue([])
  vi.mocked(api.workspaceSettings).mockResolvedValue({ root: 'D:/books', databasePath: 'D:/data.sqlite3' })
  vi.stubGlobal('confirm', vi.fn(() => false))
})
afterEach(() => { wrappers.splice(0).forEach(w => w.unmount()); vi.unstubAllGlobals() })
describe('M1 projects and materials', () => {
  it('browser preview never invokes local database commands', async () => {
    vi.mocked(api.isDesktop).mockReturnValue(false)
    const { wrapper } = await render('/projects')
    expect(api.listProjects).not.toHaveBeenCalled()
    expect(wrapper.text()).toContain('浏览器预览')
    expect(wrapper.findAll('button').every(b => b.element.disabled)).toBe(true)
  })
  it('searches and filters with explicit query and recovers from a read failure', async () => {
    const { wrapper } = await render('/projects')
    await wrapper.get('#search').setValue('余华')
    vi.mocked(api.listProjects).mockRejectedValueOnce('读取失败')
    await wrapper.get('form').trigger('submit'); await flushPromises()
    expect(wrapper.get('[role="alert"]').text()).toContain('读取失败')
    expect(api.listProjects).toHaveBeenLastCalledWith('余华', '')
    vi.mocked(api.listProjects).mockResolvedValueOnce([])
    await wrapper.get('form').trigger('submit'); await flushPromises()
    expect(wrapper.text()).toContain('没有匹配的项目')
    expect(wrapper.find('[role="alert"]').exists()).toBe(false)
  })
  it('creates a project and navigates to its persisted detail', async () => {
    const { wrapper, router } = await render('/projects/new')
    await wrapper.get('#title').setValue('活着')
    vi.mocked(api.saveProject).mockResolvedValue(saved)
    await wrapper.get('form').trigger('submit'); await flushPromises()
    expect(api.saveProject).toHaveBeenCalledWith(null, expect.objectContaining({ title: '活着' }))
    expect(router.currentRoute.value.path).toBe('/projects/p1/writing')
    expect(wrapper.text()).toContain('发布文案与音频提示词')
  })
  it('preserves edited input after save failure and prevents accidental route loss', async () => {
    const { wrapper, router } = await render('/projects/p1')
    await wrapper.get('#notes').setValue('未保存的读书笔记')
    vi.mocked(api.saveProject).mockRejectedValueOnce('database is locked')
    await wrapper.get('form').trigger('submit'); await flushPromises()
    expect((wrapper.get('#notes').element as HTMLTextAreaElement).value).toBe('未保存的读书笔记')
    expect(wrapper.get('[role="alert"]').text()).toContain('database is locked')
    await router.push('/projects'); expect(router.currentRoute.value.path).toBe('/projects/p1')
    await router.push('/projects/p2'); expect(router.currentRoute.value.path).toBe('/projects/p1')
  })
  it('keeps existing metadata while the book form only asks for title, author and optional notes', async () => {
    const { wrapper } = await render('/projects/p1')
    expect(wrapper.findAll('input')).toHaveLength(2)
    expect(wrapper.findAll('textarea')).toHaveLength(1)
    expect(wrapper.findComponent(ProjectAssets).exists()).toBe(false)
    await wrapper.get('#notes').setValue('这期聚焦人物关系')
    vi.mocked(api.saveProject).mockResolvedValue(saved)
    await wrapper.get('form').trigger('submit'); await flushPromises()
    expect(api.saveProject).toHaveBeenCalledWith('p1', expect.objectContaining({ status: saved.status, description: saved.description, sourceNotes: '这期聚焦人物关系' }))
  })
  it('requires deletion confirmation, cancellation is harmless, success returns to list', async () => {
    const { wrapper, router } = await render('/projects/p1')
    await button(wrapper, '删除项目').trigger('click')
    expect(api.deleteProject).not.toHaveBeenCalled()
    expect(wrapper.get('[role="alertdialog"]').text()).toContain('原始视频、素材文件和工作目录都会保留')
    await button(wrapper, '取消').trigger('click'); expect(api.deleteProject).not.toHaveBeenCalled()
    await button(wrapper, '删除项目').trigger('click'); await button(wrapper, '确认删除').trigger('click'); await flushPromises()
    expect(api.deleteProject).toHaveBeenCalledWith('p1')
    expect(router.currentRoute.value.path).toBe('/projects')
  })
  it('shows missing assets, preserves reference on cancellation and failed relocation', async () => {
    vi.mocked(api.listAssets).mockResolvedValue([asset])
    const { wrapper } = await render('/assets')
    expect(wrapper.text()).toContain('文件已丢失')
    expect(button(wrapper, '打开文件').element.disabled).toBe(true)
    vi.mocked(api.chooseAsset).mockResolvedValueOnce(null)
    await button(wrapper, '重新定位').trigger('click'); await flushPromises()
    expect(api.linkAsset).not.toHaveBeenCalled()
    vi.mocked(api.chooseAsset).mockResolvedValueOnce('D:/新视频.mp4')
    vi.mocked(api.linkAsset).mockRejectedValueOnce('素材文件不可用')
    await button(wrapper, '重新定位').trigger('click'); await flushPromises()
    expect(wrapper.text()).toContain('D:/视频.mp4'); expect(wrapper.text()).toContain('素材文件不可用')
  })
  it('refreshes the same asset after successful relocation', async () => {
    vi.mocked(api.listAssets).mockResolvedValueOnce([asset]).mockResolvedValueOnce([{ ...asset, existsStatus: 'available', filePath: 'D:/新视频.mp4' }])
    const { wrapper } = await render('/assets')
    vi.mocked(api.chooseAsset).mockResolvedValueOnce('D:/新视频.mp4')
    await button(wrapper, '重新定位').trigger('click'); await flushPromises()
    expect(api.linkAsset).toHaveBeenCalledWith('p1', 'D:/新视频.mp4', 'a1')
    expect(wrapper.text()).toContain('文件可用'); expect(button(wrapper, '打开文件').element.disabled).toBe(false)
  })
  it('does not restore before confirmation, preserves selection on failure, shows previous backup on success', async () => {
    const { wrapper } = await render('/settings')
    vi.mocked(api.chooseBackup).mockResolvedValueOnce('D:/backup.sqlite3')
    await button(wrapper, '选择备份恢复').trigger('click'); await flushPromises()
    expect(api.restoreBackup).not.toHaveBeenCalled()
    expect(wrapper.get('[role="alertdialog"]').text()).toContain('D:/backup.sqlite3')
    vi.mocked(api.restoreBackup).mockRejectedValueOnce('备份数据库结构不匹配')
    await button(wrapper, '确认恢复').trigger('click'); await flushPromises()
    expect(wrapper.find('[role="alertdialog"]').exists()).toBe(true)
    expect(wrapper.get('[role="alert"]').text()).toContain('结构不匹配')
    vi.mocked(api.restoreBackup).mockResolvedValueOnce('D:/previous.sqlite3')
    await button(wrapper, '确认恢复').trigger('click'); await flushPromises()
    expect(wrapper.find('[role="alertdialog"]').exists()).toBe(false)
    expect(wrapper.get('[role="status"]').text()).toContain('D:/previous.sqlite3')
  })
  it('does not create a backup when the directory picker is cancelled', async () => {
    const { wrapper } = await render('/settings')
    vi.mocked(api.chooseDirectory).mockResolvedValueOnce(null)
    await button(wrapper, '创建备份').trigger('click'); await flushPromises()
    expect(api.createBackup).not.toHaveBeenCalled()
  })
})
