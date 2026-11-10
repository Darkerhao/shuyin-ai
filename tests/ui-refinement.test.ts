import { requestConfirmation } from '../src/composables/useConfirmation'
import * as drafts from '../src/services/drafts'
vi.mock('../src/services/drafts', () => ({ readDraft: vi.fn(), saveDraft: vi.fn() }))
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { PhysicalPosition } from '@tauri-apps/api/dpi'
import type { DragDropEvent } from '@tauri-apps/api/webview'
import { createMemoryHistory, createRouter } from 'vue-router'
import App from '../src/App.vue'
import ProjectsPage from '../src/modules/projects/ProjectsPage.vue'
import ProjectPage from '../src/modules/projects/ProjectPage.vue'
import ProjectNavigation from '../src/components/ProjectNavigation.vue'
import { emptyProject, type Asset, type Project } from '../src/types/project'
import * as projects from '../src/services/projects'
import * as workflow from '../src/services/workflow'
import * as preferences from '../src/services/interface'
import { useInterfacePreferences } from '../src/composables/useInterfacePreferences'
import { initializeTheme } from '../src/composables/useTheme'
import { answerConfirmation } from './ui'
const native = vi.hoisted(() => ({ onDragDropEvent: vi.fn(), onCloseRequested: vi.fn(), destroy: vi.fn() }))
vi.mock('@tauri-apps/api/core', async original => ({ ...await original<typeof import('@tauri-apps/api/core')>(), isTauri: () => true }))
vi.mock('@tauri-apps/api/window', () => ({ getCurrentWindow: () => native }))
vi.mock('../src/services/interface', () => ({ interfacePreferences: vi.fn(), setSidebarPrompt: vi.fn() }))
vi.mock('../src/services/projects', async original => ({ ...await original<typeof projects>(), isDesktop: () => true, listProjects: vi.fn(), getProject: vi.fn(), saveProject: vi.fn(), listAssets: vi.fn(), workspaceSettings: vi.fn(), openProjectDirectory: vi.fn(), exportVideoAsset: vi.fn(), deleteProject: vi.fn() }))
vi.mock('../src/services/workflow', async original => ({ ...await original<typeof workflow>(), workflowSnapshot: vi.fn(), projectProgress: vi.fn() }))
vi.mock('../src/composables/useAppUpdates', async () => {
  const { ref } = await import('vue')
  return { useAppUpdates: () => ({ initialize: async () => {}, status: ref('idle'), update: ref(null), error: ref(''), settingsError: ref('') }) }
})
const saved: Project = { ...emptyProject(), id: 'p1', title: '文城', author: '余华', workspacePath: 'D:/书籍/文城', createdAt: '2026-11-03T00:00:00Z', updatedAt: '2026-11-03T00:00:00Z', archivedAt: null }
const mounted: ReturnType<typeof mount>[] = []
const Page = { components: { ProjectNavigation }, props: ['id'], template: '<ProjectNavigation :id="id" /><p>创作页面</p>' }
beforeEach(() => {
  vi.clearAllMocks()
  vi.mocked(drafts.readDraft).mockResolvedValue(null)
  vi.mocked(drafts.saveDraft).mockResolvedValue(undefined)
  localStorage.clear()
  initializeTheme()
  native.onDragDropEvent.mockResolvedValue(vi.fn())
  native.onCloseRequested.mockResolvedValue(vi.fn())
  vi.mocked(preferences.interfacePreferences).mockResolvedValue({ sidebarPrompt: true })
  vi.mocked(preferences.setSidebarPrompt).mockResolvedValue(undefined)
  vi.mocked(projects.listProjects).mockResolvedValue([saved])
  vi.mocked(projects.getProject).mockResolvedValue(saved)
  vi.mocked(projects.listAssets).mockResolvedValue([])
  vi.mocked(projects.workspaceSettings).mockResolvedValue({ root: 'D:/书籍', databasePath: 'D:/测试.sqlite3' })
  vi.mocked(workflow.workflowSnapshot).mockResolvedValue({ resume: [], running: [] })
  vi.mocked(workflow.projectProgress).mockResolvedValue(['', 'writing', 'covers', 'video', 'publishing'].map(path => ({ path, status: path === 'covers' ? '已导出 2/2' : '未开始', tone: path === 'covers' ? 'ready' : 'neutral' })))
})
afterEach(() => { mounted.splice(0).forEach(wrapper => wrapper.unmount()); document.body.innerHTML = ''; vi.restoreAllMocks(); vi.unstubAllGlobals() })
async function render(path = '/projects') {
  const router = createRouter({ history: createMemoryHistory(), routes: [
    { path: '/projects', component: ProjectsPage }, { path: '/projects/new', component: ProjectPage },
    { path: '/projects/:id', component: ProjectPage, props: true },
    ...['writing', 'covers', 'video', 'publishing'].map(step => ({ path: `/projects/:id/${step}`, component: Page, props: true })),
    ...['/publishing', '/settings/models', '/settings', '/about'].map(path => ({ path, component: { template: '<p>设置</p>' } })),
  ] })
  await router.push(path); await router.isReady()
  const wrapper = mount(App, { attachTo: document.body, global: { plugins: [router] } }); mounted.push(wrapper)
  await flushPromises(); return { wrapper, router }
}
it('switches the whole interface and notifications without remounting or saving the current editor', async () => {
  const { wrapper, router } = await render('/projects/p1')
  await wrapper.get('#title').setValue('尚未保存的书名')
  const editor = wrapper.get('#title').element
  await wrapper.get('button[aria-label="切换为浅色风格"]').trigger('click')
  await flushPromises()
  expect(document.documentElement.classList.contains('dark')).toBe(false)
  expect(localStorage.getItem('shuyin.theme')).toBe('light')
  expect(wrapper.findComponent({ name: 'Toaster' }).props('theme')).toBe('light')
  expect(wrapper.get('#title').element).toBe(editor)
  expect(editor).toHaveProperty('value', '尚未保存的书名')
  expect(projects.saveProject).not.toHaveBeenCalled()
  expect(router.currentRoute.value.path).toBe('/projects/p1')
  await wrapper.get('button[aria-label="切换为深色风格"]').trigger('click')
  expect(document.documentElement.classList.contains('dark')).toBe(true)
  expect(wrapper.get('#title').element).toHaveProperty('value', '尚未保存的书名')
})
it('navigates to each of the five real routes using Ctrl/Cmd while keeping exactly one current step', async () => {
  const { wrapper, router } = await render('/projects/p1/covers')
  for (const [index, suffix] of ['', '/writing', '/covers', '/video', '/publishing'].entries()) {
    globalThis.dispatchEvent(new KeyboardEvent('keydown', { key: String(index + 1), ctrlKey: index % 2 === 0, metaKey: index % 2 === 1 }))
    await flushPromises()
    expect(router.currentRoute.value.path).toBe(`/projects/p1${suffix}`)
    expect(wrapper.findAll('.project-navigation a[aria-current="step"]')).toHaveLength(1)
  }
})
it('does not navigate during composition, key repeat or a pending unsaved-content confirmation', async () => {
  const { wrapper, router } = await render('/projects/p1')
  for (const options of [{ isComposing: true }, { repeat: true }]) {
    globalThis.dispatchEvent(new KeyboardEvent('keydown', { key: '3', ctrlKey: true, ...options })); await flushPromises()
    expect(router.currentRoute.value.path).toBe('/projects/p1')
  }
  await wrapper.get('#title').setValue('未保存书名')
  expect(wrapper.get('.project-navigation a[aria-current="step"]').attributes('aria-label')).toContain('编辑中')
  const confirmation = requestConfirmation('确认本次操作？')
  await flushPromises()
  globalThis.dispatchEvent(new KeyboardEvent('keydown', { key: '4', ctrlKey: true })); await flushPromises()
  expect(router.currentRoute.value.path).toBe('/projects/p1')
  await answerConfirmation(false); await confirmation
  expect(wrapper.get('#title').element).toHaveProperty('value', '未保存书名')
  await router.push('/projects')
  expect(drafts.saveDraft).toHaveBeenCalledWith('p1', 'project', expect.objectContaining({ content: expect.objectContaining({ form: expect.objectContaining({ title: '未保存书名' }) }) }))
})
it('uses returned completion facts, overrides the current unsaved editor and never marks a failed query complete', async () => {
  const router = createRouter({ history: createMemoryHistory(), routes: [{ path: '/projects/:id/covers', component: { template: '<p>预览</p>' } }] })
  await router.push('/projects/p1/covers'); await router.isReady()
  const wrapper = mount(ProjectNavigation, { props: { id: 'p1' }, global: { plugins: [router] } }); mounted.push(wrapper)
  await flushPromises()
  expect(wrapper.get('.project-navigation a[aria-current="step"]').attributes('aria-label')).toContain('已导出 2/2')
  await wrapper.setProps({ editing: true })
  expect(wrapper.get('.project-navigation a[aria-current="step"]').attributes('aria-label')).toContain('编辑中')
  vi.mocked(workflow.projectProgress).mockRejectedValue(new Error('read failed'))
  await wrapper.setProps({ editing: false, busy: true })
  await wrapper.setProps({ busy: false }); await flushPromises()
  expect(wrapper.findAll('.project-navigation a').every(link => link.attributes('aria-label')?.includes('状态待同步'))).toBe(true)
})
it('stages a native dropped file in the original new-book form and preserves it when save fails', async () => {
  const { wrapper, router } = await render()
  const handler = native.onDragDropEvent.mock.calls[0]![0] as (event: { payload: { type: string; paths?: string[] } }) => void
  handler({ payload: { type: 'enter', paths: ['D:/书籍/新书.epub'] } }); await flushPromises()
  expect(wrapper.find('.book-drop-overlay').exists()).toBe(true)
  handler({ payload: { type: 'drop', paths: ['D:/书籍/新书.epub'] } }); await flushPromises()
  expect(router.currentRoute.value.path).toBe('/projects/new')
  expect(wrapper.get('#title').element).toHaveProperty('value', '新书')
  expect(wrapper.get('[aria-label="本地书籍文件"]').text()).toContain('D:/书籍/新书.epub')
  expect(projects.saveProject).not.toHaveBeenCalled()
  vi.mocked(projects.saveProject).mockRejectedValue(new Error('文件不可读'))
  await wrapper.get('form').trigger('submit'); await flushPromises()
  expect(router.currentRoute.value.path).toBe('/projects/new')
  expect(projects.saveProject).toHaveBeenCalledWith(null, expect.objectContaining({ title: '新书', author: '' }), ['D:/书籍/新书.epub'])
  expect(wrapper.get('[aria-label="本地书籍文件"]').text()).toContain('新书.epub')
})
it('rejects multiple and unsupported dropped files without creating books, and removes its listener on unmount', async () => {
  const unlisten = vi.fn(); native.onDragDropEvent.mockResolvedValue(unlisten)
  const { wrapper, router } = await render()
  const handler = native.onDragDropEvent.mock.calls[0]![0] as (event: { payload: { type: string; paths: string[] } }) => void
  for (const paths of [['D:/a.pdf', 'D:/b.pdf'], ['D:/movie.mp4']]) { handler({ payload: { type: 'drop', paths } }); await flushPromises(); expect(router.currentRoute.value.path).toBe('/projects') }
  expect(projects.saveProject).not.toHaveBeenCalled()
  wrapper.unmount(); expect(unlisten).toHaveBeenCalledOnce()
})
function sourceDrag(type: DragDropEvent['type'], paths = ['D:/书籍/新书.PDF'], position = new PhysicalPosition(300, 300)) {
  const payload = type === 'leave' ? { type } : { type, paths, position }
  for (const [handler] of native.onDragDropEvent.mock.calls) handler({ payload })
}
function sourceBounds(wrapper: ReturnType<typeof mount>) {
  vi.spyOn(wrapper.get('.book-source-dropzone').element, 'getBoundingClientRect').mockReturnValue(new DOMRect(100, 100, 500, 200))
  vi.stubGlobal('devicePixelRatio', 2)
}
it.each(['/projects/new', '/projects/p1'])('accepts a file only inside the book drop zone at scaled coordinates on %s', async path => {
  const { wrapper, router } = await render(path)
  sourceBounds(wrapper)
  await wrapper.get('#title').setValue('我的书名')
  await wrapper.get('#notes').setValue('保留我的读书笔记')
  sourceDrag('enter'); await flushPromises()
  expect(wrapper.get('.book-source-dropzone').classes()).toContain('is-drag-over')
  expect(wrapper.get('.book-source-dropzone').text()).toContain('松开鼠标')
  sourceDrag('over', [], new PhysicalPosition(20, 20)); await flushPromises()
  expect(wrapper.get('.book-source-dropzone').classes()).not.toContain('is-drag-over')
  sourceDrag('drop', ['D:/书籍/外部.pdf'], new PhysicalPosition(20, 20)); await flushPromises()
  expect(wrapper.find('[aria-label="本地书籍文件"]').exists()).toBe(false)
  sourceDrag('enter'); sourceDrag('leave'); await flushPromises()
  expect(wrapper.get('.book-source-dropzone').classes()).not.toContain('is-drag-over')
  sourceDrag('drop'); sourceDrag('drop'); await flushPromises()
  expect(wrapper.get('[aria-label="本地书籍文件"]').findAll('li')).toHaveLength(1)
  expect(wrapper.get('[aria-label="本地书籍文件"]').text()).toContain('新书.PDF')
  expect(wrapper.get('#title').element).toHaveProperty('value', '我的书名')
  expect(wrapper.get('#notes').element).toHaveProperty('value', '保留我的读书笔记')
  expect(projects.saveProject).not.toHaveBeenCalled()
  expect(router.currentRoute.value.path).toBe(path)
  await wrapper.get('[aria-label="取消绑定 新书.PDF"]').trigger('click')
  expect(wrapper.find('[aria-label="本地书籍文件"]').exists()).toBe(false)
})
it('rejects unsupported or multiple drops inline and stages supported files without duplicating saved documents', async () => {
  vi.mocked(projects.listAssets).mockResolvedValue([{ id: 'doc', projectId: 'p1', assetType: 'document', filePath: 'D:/原书.pdf', originalName: '原书.pdf', fileSize: 10, modifiedAt: 1, existsStatus: 'available', createdAt: saved.createdAt }])
  const { wrapper } = await render('/projects/p1')
  sourceBounds(wrapper)
  sourceDrag('drop', ['D:/原书.pdf']); await flushPromises()
  expect(wrapper.get('[aria-label="本地书籍文件"]').findAll('li')).toHaveLength(1)
  for (const paths of [['D:/视频.mp4'], ['D:/a.pdf', 'D:/b.epub']]) {
    sourceDrag('drop', paths); await flushPromises()
    expect(wrapper.get('.book-source-section [role="alert"]').text()).toMatch(/暂不支持|每次添加一个/)
    expect(wrapper.get('[aria-label="本地书籍文件"]').findAll('li')).toHaveLength(1)
  }
  for (const extension of ['epub', 'txt', 'md', 'docx']) sourceDrag('drop', [`D:/补充.${extension}`])
  await flushPromises()
  expect(wrapper.find('.book-source-section [role="alert"]').exists()).toBe(false)
  expect(wrapper.get('[aria-label="本地书籍文件"]').findAll('li')).toHaveLength(5)
  expect(projects.saveProject).not.toHaveBeenCalled()
})
it('fills an empty title, keeps a dropped source on save failure and ignores drops while saving', async () => {
  const { wrapper } = await render('/projects/new')
  sourceBounds(wrapper)
  sourceDrag('drop'); await flushPromises()
  expect(wrapper.get('#title').element).toHaveProperty('value', '新书')
  let failSave!: (reason: Error) => void
  vi.mocked(projects.saveProject).mockReturnValueOnce(new Promise((_, reject) => { failSave = reject }))
  await wrapper.get('form').trigger('submit'); await flushPromises()
  sourceDrag('drop', ['D:/保存期间.pdf']); await flushPromises()
  expect(wrapper.get('[aria-label="本地书籍文件"]').findAll('li')).toHaveLength(1)
  failSave(new Error('文件不可读')); await flushPromises()
  expect(wrapper.get('[aria-label="本地书籍文件"]').text()).toContain('新书.PDF')
  expect(projects.saveProject).toHaveBeenCalledWith(null, expect.objectContaining({ title: '新书' }), ['D:/书籍/新书.PDF'])
})
it('removes the form drop listener on navigation and ignores its stale callback', async () => {
  const unlisteners: ReturnType<typeof vi.fn>[] = []
  native.onDragDropEvent.mockImplementation(() => { const unlisten = vi.fn(); unlisteners.push(unlisten); return Promise.resolve(unlisten) })
  const { wrapper, router } = await render('/projects/new')
  sourceBounds(wrapper)
  await router.push('/settings/models'); await flushPromises()
  expect(unlisteners.filter(unlisten => unlisten.mock.calls.length)).toHaveLength(1)
  sourceDrag('drop'); await flushPromises()
  expect(router.currentRoute.value.path).toBe('/settings/models')
  expect(projects.saveProject).not.toHaveBeenCalled()
})
it('ignores file drops during automatic production and resumes when it finishes', async () => {
  const { wrapper } = await render('/projects/new')
  sourceBounds(wrapper)
  const setup = wrapper.findComponent({ name: 'AutomationSetup' })
  setup.vm.$emit('busy', true); await flushPromises()
  sourceDrag('enter'); sourceDrag('drop'); await flushPromises()
  expect(wrapper.get('.book-source-dropzone').classes()).not.toContain('is-drag-over')
  expect(wrapper.find('[aria-label="本地书籍文件"]').exists()).toBe(false)
  setup.vm.$emit('busy', false); await flushPromises()
  sourceDrag('drop'); await flushPromises()
  expect(wrapper.get('[aria-label="本地书籍文件"]').text()).toContain('新书.PDF')
})
it('cleans up a listener whose registration finishes after the form unmounts', async () => {
  const { router } = await render()
  let finishRegistration!: (unlisten: () => void) => void
  native.onDragDropEvent.mockReturnValueOnce(new Promise(resolve => { finishRegistration = resolve }))
  await router.push('/projects/new'); await flushPromises()
  await router.push('/settings/models'); await flushPromises()
  const unlisten = vi.fn()
  finishRegistration(unlisten); await flushPromises()
  expect(unlisten).toHaveBeenCalledOnce()
})
it('opens the same context menu from More and Shift+F10, restores focus on Escape and cancels deletion', async () => {
  const { wrapper } = await render()
  const more = wrapper.get('[aria-label="更多操作：文城"]')
  await more.trigger('click'); await flushPromises()
  let menu = document.querySelector('[role="menu"]')!
  expect(menu.textContent).toContain('暂无可导出的成片')
  menu.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true })); await flushPromises()
  expect(document.activeElement).toBe(more.element)
  await wrapper.get('.project-card-link').trigger('keydown', { key: 'F10', shiftKey: true }); await flushPromises()
  menu = document.querySelector('[role="menu"]')!
  const remove = [...menu.querySelectorAll('[role="menuitem"]')].find(item => item.textContent?.includes('删除书籍'))!
  remove.dispatchEvent(new MouseEvent('click', { bubbles: true })); await flushPromises()
  await answerConfirmation(false)
  expect(projects.deleteProject).not.toHaveBeenCalled()
  expect(wrapper.find('.project-card').exists()).toBe(true)
})
it('exports the chosen registered video through the existing asset service without rebuilding it', async () => {
  const video: Asset = { id: 'video-1', projectId: 'p1', assetType: 'video', filePath: 'D:/成片.mp4', originalName: '成片.mp4', fileSize: 10, modifiedAt: 1, existsStatus: 'available', createdAt: saved.createdAt }
  vi.mocked(projects.listAssets).mockResolvedValue([video])
  vi.mocked(projects.exportVideoAsset).mockResolvedValue(true)
  const { wrapper } = await render()
  await wrapper.get('[aria-label="更多操作：文城"]').trigger('click'); await flushPromises()
  const item = [...document.querySelectorAll('[role="menuitem"]')].find(item => item.textContent?.includes('导出成片'))!
  item.dispatchEvent(new MouseEvent('click', { bubbles: true })); await flushPromises()
  expect(projects.exportVideoAsset).toHaveBeenCalledExactlyOnceWith(video)
  expect(projects.saveProject).not.toHaveBeenCalled()
})
it('preserves the visible sidebar prompt on save failure and supports restoring it after a successful collapse', async () => {
  const { wrapper } = await render()
  vi.mocked(preferences.setSidebarPrompt).mockRejectedValueOnce(new Error('write failed'))
  await wrapper.get('[aria-label="收起创作提示"]').trigger('click'); await flushPromises()
  expect(wrapper.find('.sidebar-prompt').exists()).toBe(true)
  await wrapper.get('[aria-label="收起创作提示"]').trigger('click'); await flushPromises()
  expect(wrapper.find('.sidebar-prompt').exists()).toBe(false)
  await useInterfacePreferences().setPrompt(true); await flushPromises()
  expect(wrapper.find('.sidebar-prompt').exists()).toBe(true)
})
