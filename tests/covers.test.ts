import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createMemoryHistory, createRouter } from 'vue-router'
import App from '../src/App.vue'
import CoversPage from '../src/modules/covers/CoversPage.vue'
import * as api from '../src/services/covers'
import * as ai from '../src/services/ai'
import { emptyProvider } from '../src/types/ai'
import type { Asset } from '../src/types/project'
import * as projects from '../src/services/projects'
import * as renderer from '../src/services/coverRenderer'
import { newCover, type CoverVersion } from '../src/types/cover'
import { emptyProject } from '../src/types/project'

vi.mock('../src/services/covers', () => ({ listCovers: vi.fn(), saveCover: vi.fn(), readBackground: vi.fn(), chooseBackground: vi.fn(), startCoverGeneration: vi.fn(), executeCoverGeneration: vi.fn() }))
vi.mock('../src/services/ai', () => ({ listProviders: vi.fn(), cancelGeneration: vi.fn() }))
vi.mock('../src/services/projects', () => ({ isDesktop: vi.fn(), getProject: vi.fn(), listAssets: vi.fn(), linkAsset: vi.fn(), openProjectDirectory: vi.fn() }))
vi.mock('../src/services/coverRenderer', () => ({ renderCover: vi.fn(), coverPng: vi.fn(), decodeBackground: vi.fn() }))
const initial: CoverVersion = { id: 'v1', projectId: 'p1', versionNo: 1, config: newCover('最初的封面'), exportedPath: null, createdAt: '2026-10-23T00:00:00Z' }
const aiAsset: Asset = { id: 'a1', projectId: 'p1', assetType: 'image', filePath: 'D:/ai.png', originalName: 'ai.png', fileSize: 100, modifiedAt: 0, existsStatus: 'available', createdAt: '' }
const wrappers: ReturnType<typeof mount>[] = []
async function render() {
  const router = createRouter({ history: createMemoryHistory(), routes: [
    { path: '/projects/:id/covers', component: CoversPage, props: true },
    { path: '/projects/:id/writing', component: { template: '<p>writing</p>' } },
    { path: '/projects', component: { template: '<p>projects</p>' } },
    { path: '/settings/models', component: { template: '<p>models</p>' } },
  ] })
  await router.push('/projects/p1/covers'); await router.isReady()
  const wrapper = mount(App, { global: { plugins: [router] } }); wrappers.push(wrapper)
  await flushPromises(); return { wrapper, router }
}
function button(wrapper: ReturnType<typeof mount>, name: string) { return wrapper.findAll('button').find(b => b.text() === name)! }
beforeEach(() => {
  vi.resetAllMocks(); vi.stubGlobal('confirm', vi.fn(() => false))
  Object.defineProperty(document, 'fonts', { value: { ready: Promise.resolve() }, configurable: true })
  vi.mocked(projects.isDesktop).mockReturnValue(true)
  vi.mocked(projects.getProject).mockResolvedValue({ ...emptyProject(), title: '测试书', id: 'p1', workspacePath: 'D:/books/p1', createdAt: '', updatedAt: '', archivedAt: null })
  vi.mocked(projects.listAssets).mockResolvedValue([])
  vi.mocked(ai.listProviders).mockResolvedValue([])
  vi.mocked(api.listCovers).mockResolvedValue([initial])
  vi.mocked(renderer.renderCover).mockReturnValue({ titleSize: 112, subtitleSize: 44 })
  vi.mocked(renderer.coverPng).mockResolvedValue([137, 80, 78, 71])
  let versionNo = 1
  vi.mocked(api.saveCover).mockImplementation(async (projectId, config, png) => {
    versionNo++
    return { ...initial, id: `v${versionNo}`, projectId, config, versionNo, exportedPath: png ? `D:/cover-${config.width}x${config.height}-v${versionNo}.png` : null }
  })
})
afterEach(() => { wrappers.splice(0).forEach(w => w.unmount()); vi.unstubAllGlobals() })
describe('M3 cover workflow', () => {
  async function aiCover() {
    vi.mocked(ai.listProviders).mockResolvedValue([
      { ...emptyProvider(), id: 'text', name: '文本模型', credentialStatus: 'missing' },
      { ...emptyProvider(), id: 'image', name: '图片模型', modelKind: 'image', credentialStatus: 'missing' },
      { ...emptyProvider(), id: 'disabled', name: '停用图片', modelKind: 'image', isEnabled: false, credentialStatus: 'missing' },
    ])
    vi.mocked(api.startCoverGeneration).mockResolvedValue('image-task')
    vi.mocked(api.executeCoverGeneration).mockResolvedValue(aiAsset)
    vi.mocked(api.readBackground).mockResolvedValue(new ArrayBuffer(1))
    vi.mocked(renderer.decodeBackground).mockResolvedValue(document.createElement('img'))
    const result = await render()
    await button(result.wrapper, '使用 AI 生成背景（可选）').trigger('click'); await flushPromises()
    return result
  }
  it('uses only enabled image models, preserves text and exports the generated background in both ratios', async () => {
    const { wrapper } = await aiCover()
    expect(wrapper.findAll('#cover-model option').map(o => o.text())).toEqual(['图片模型'])
    await wrapper.get('#cover-title').setValue('手动修改标题')
    await wrapper.get('#cover-description').setValue('水墨山林，安静留白')
    await wrapper.get('section[aria-label="可选 AI 背景"] form').trigger('submit'); await flushPromises()
    expect(api.startCoverGeneration).toHaveBeenCalledExactlyOnceWith('p1', 'image', '水墨山林，安静留白')
    expect((wrapper.get('#cover-title').element as HTMLTextAreaElement).value).toBe('手动修改标题')
    expect((wrapper.get('#cover-background').element as HTMLSelectElement).value).toBe('a1')
    expect(api.saveCover).not.toHaveBeenCalled()
    await button(wrapper, '导出两张封面').trigger('click'); await flushPromises()
    expect(api.saveCover).toHaveBeenNthCalledWith(1, 'p1', expect.objectContaining({ backgroundAssetId: 'a1', width: 1440, height: 1080 }), expect.any(Array))
    expect(api.saveCover).toHaveBeenNthCalledWith(2, 'p1', expect.objectContaining({ backgroundAssetId: 'a1', width: 1080, height: 1440 }), expect.any(Array))
  })
  it.each(['network', 'decode'])('keeps current background and edits when AI fails at %s', async failure => {
    vi.mocked(api.listCovers).mockResolvedValue([{ ...initial, config: { ...initial.config, backgroundAssetId: 'old' } }])
    const { wrapper } = await aiCover()
    await wrapper.get('#cover-title').setValue('保留草稿')
    if (failure === 'network') vi.mocked(api.executeCoverGeneration).mockRejectedValue({ message: '服务失败' })
    else vi.mocked(renderer.decodeBackground).mockRejectedValueOnce(new Error('图片解码失败'))
    await wrapper.get('section[aria-label="可选 AI 背景"] form').trigger('submit'); await flushPromises()
    expect((wrapper.get('#cover-background').element as HTMLSelectElement).value).toBe('old')
    expect((wrapper.get('#cover-title').element as HTMLTextAreaElement).value).toBe('保留草稿')
    expect(api.saveCover).not.toHaveBeenCalled()
    expect(wrapper.get('[role="alert"]').text()).toContain('失败')
  })
  it('cancels image generation without replacing the current background or history', async () => {
    const { wrapper, router } = await aiCover()
    let reject!: (error: unknown) => void
    vi.mocked(api.executeCoverGeneration).mockReturnValue(new Promise((_resolve, fail) => { reject = fail }))
    vi.mocked(ai.cancelGeneration).mockImplementation(async () => { reject({ message: '生成已取消' }) })
    await wrapper.get('section[aria-label="可选 AI 背景"] form').trigger('submit'); await flushPromises()
    await router.push('/projects'); expect(router.currentRoute.value.path).toContain('/covers')
    await button(wrapper, '取消生成').trigger('click'); await flushPromises()
    expect(ai.cancelGeneration).toHaveBeenCalledExactlyOnceWith('image-task')
    expect((wrapper.get('#cover-title').element as HTMLTextAreaElement).value).toBe(initial.config.title)
    expect(api.readBackground).not.toHaveBeenCalled()
    expect(api.saveCover).not.toHaveBeenCalled()
    await button(wrapper, '导出两张封面').trigger('click'); await flushPromises()
    expect(api.saveCover).toHaveBeenCalledTimes(2)
  })
  it('allows local export when reading image model settings fails', async () => {
    vi.mocked(ai.listProviders).mockRejectedValue('模型列表读取失败')
    const { wrapper } = await render()
    await button(wrapper, '使用 AI 生成背景（可选）').trigger('click'); await flushPromises()
    await button(wrapper, '导出两张封面').trigger('click'); await flushPromises()
    expect(api.saveCover).toHaveBeenCalledTimes(2)
  })
  it('keeps local export available without querying or configuring image models', async () => {
    const { wrapper } = await render()
    expect(ai.listProviders).not.toHaveBeenCalled()
    await button(wrapper, '使用 AI 生成背景（可选）').trigger('click'); await flushPromises()
    expect(wrapper.text()).toContain('尚无已启用的图片模型')
    await button(wrapper, '导出两张封面').trigger('click'); await flushPromises()
    expect(api.saveCover).toHaveBeenCalledTimes(2)
    expect(api.startCoverGeneration).not.toHaveBeenCalled()
  })
  it('does not call desktop services in a browser preview', async () => {
    vi.mocked(projects.isDesktop).mockReturnValue(false)
    const { wrapper } = await render()
    expect(api.listCovers).not.toHaveBeenCalled(); expect(wrapper.text()).toContain('请在桌面应用中制作封面')
  })
  it('restores the latest configuration and exports both upload covers while retaining the preview ratio', async () => {
    const { wrapper } = await render()
    expect(wrapper.findAll('#cover-size option').map(o => o.text())).toEqual(['横版 4:3 · 1440 × 1080', '竖版 3:4 · 1080 × 1440'])
    expect((wrapper.get('#cover-title').element as HTMLTextAreaElement).value).toBe(initial.config.title)
    await wrapper.get('#cover-title').setValue('新的中文封面')
    await wrapper.get('#cover-size').setValue('1')
    await button(wrapper, '导出两张封面').trigger('click'); await flushPromises()
    expect(api.saveCover).toHaveBeenCalledTimes(2)
    expect(api.saveCover).toHaveBeenNthCalledWith(1, 'p1', expect.objectContaining({ title: '新的中文封面', width: 1440, height: 1080 }), [137, 80, 78, 71])
    expect(api.saveCover).toHaveBeenNthCalledWith(2, 'p1', expect.objectContaining({ title: '新的中文封面', width: 1080, height: 1440 }), [137, 80, 78, 71])
    expect((wrapper.get('#cover-size').element as HTMLSelectElement).value).toBe('1')
    expect(wrapper.get('[role="status"]').text()).toContain('已导出两张发布封面')
    expect(wrapper.get('[role="status"]').text()).toContain('D:/cover-1440x1080-v2.png')
    expect(wrapper.get('[role="status"]').text()).toContain('D:/cover-1080x1440-v3.png')
    expect(initial.config.title).toBe('最初的封面')
    await wrapper.get('#cover-history').setValue('v1'); await flushPromises()
    expect((wrapper.get('#cover-title').element as HTMLTextAreaElement).value).toBe('最初的封面')
  })
  it('preserves edits and blocks accidental history or route changes after export failure', async () => {
    const { wrapper, router } = await render()
    await wrapper.get('#cover-title').setValue('失败仍保留')
    vi.mocked(api.saveCover).mockRejectedValue('封面目录不可写')
    await button(wrapper, '导出两张封面').trigger('click'); await flushPromises()
    expect(wrapper.get('[role="alert"]').text()).toContain('不可写')
    await wrapper.get('#cover-history').setValue('v1'); await router.push('/projects')
    expect(router.currentRoute.value.path).toBe('/projects/p1/covers')
    expect((wrapper.get('#cover-title').element as HTMLTextAreaElement).value).toBe('失败仍保留')
  })
  it('keeps draft on render failure and saves config without requiring an image', async () => {
    const { wrapper } = await render()
    await wrapper.get('#cover-title').setValue('配置草稿')
    vi.mocked(renderer.coverPng).mockRejectedValue(new Error('无法导出 PNG'))
    await button(wrapper, '导出两张封面').trigger('click'); await flushPromises()
    expect(api.saveCover).not.toHaveBeenCalled()
    vi.mocked(api.saveCover).mockImplementation(async (projectId, config) => ({ ...initial, projectId, config, id: 'v2', versionNo: 2 }))
    await wrapper.get('form').trigger('submit'); await flushPromises()
    expect(api.saveCover).toHaveBeenCalledWith('p1', expect.objectContaining({ title: '配置草稿' }), null)
  })
  it('writes neither cover when the second ratio cannot be rendered', async () => {
    const { wrapper } = await render()
    await wrapper.get('#cover-title').setValue('两个比例都要完整排版')
    vi.mocked(renderer.coverPng).mockResolvedValueOnce([137, 80, 78, 71]).mockRejectedValueOnce(new Error('竖版文字超出安全区域'))
    await button(wrapper, '导出两张封面').trigger('click'); await flushPromises()
    expect(api.saveCover).not.toHaveBeenCalled()
    expect(wrapper.get('[role="alert"]').text()).toContain('竖版文字超出安全区域')
    expect((wrapper.get('#cover-title').element as HTMLTextAreaElement).value).toBe('两个比例都要完整排版')
  })
  it('retains the first export and the unsaved draft when writing the second cover fails', async () => {
    const { wrapper, router } = await render()
    await wrapper.get('#cover-title').setValue('第二张失败仍保留')
    vi.mocked(api.saveCover).mockResolvedValueOnce({ ...initial, id: 'v2', versionNo: 2, exportedPath: 'D:/cover-1440x1080-v2.png' }).mockRejectedValueOnce('竖版封面目录不可写')
    await button(wrapper, '导出两张封面').trigger('click'); await flushPromises()
    expect(wrapper.get('[role="status"]').text()).toContain('已导出 1/2 张发布封面：D:/cover-1440x1080-v2.png')
    expect(wrapper.get('[role="alert"]').text()).toContain('不可写')
    expect(wrapper.findAll('#cover-history option')).toHaveLength(3)
    await router.push('/projects')
    expect(router.currentRoute.value.path).toBe('/projects/p1/covers')
    expect((wrapper.get('#cover-title').element as HTMLTextAreaElement).value).toBe('第二张失败仍保留')
  })
  it.each([[1080, 1920, '1'], [1920, 1080, '0'], [1080, 1080, '1']])('adapts historical %i × %i configurations without modifying their saved data', async (width, height, index) => {
    const legacy = { ...initial, config: { ...initial.config, width, height } }
    vi.mocked(api.listCovers).mockResolvedValue([legacy])
    const { wrapper } = await render()
    expect((wrapper.get('#cover-size').element as HTMLSelectElement).value).toBe(index)
    expect(wrapper.get('[role="status"]').text()).toContain('原版本和文件保留')
    expect(legacy.config.width).toBe(width); expect(legacy.config.height).toBe(height)
    expect(api.saveCover).not.toHaveBeenCalled()
  })
  it('cancelling background selection keeps the current config', async () => {
    const { wrapper } = await render(); vi.mocked(api.chooseBackground).mockResolvedValue(null)
    await button(wrapper, '选择本地图片').trigger('click'); await flushPromises()
    expect(projects.linkAsset).not.toHaveBeenCalled(); expect(api.readBackground).not.toHaveBeenCalled()
    expect((wrapper.get('#cover-title').element as HTMLTextAreaElement).value).toBe(initial.config.title)
  })
  it('missing background is explicit, blocks export, and allows recovery to a plain background', async () => {
    vi.mocked(api.listCovers).mockResolvedValue([{ ...initial, config: { ...initial.config, backgroundAssetId: 'missing' } }])
    vi.mocked(api.readBackground).mockRejectedValue('背景图不存在')
    const { wrapper } = await render()
    expect(wrapper.text()).toContain('背景图不存在'); expect(button(wrapper, '导出两张封面').element.disabled).toBe(true)
    await wrapper.get('#cover-background').setValue(''); await flushPromises()
    expect(button(wrapper, '导出两张封面').element.disabled).toBe(false)
  })
  it('disables editing while a version is being saved', async () => {
    const { wrapper } = await render()
    let finish!: (value: CoverVersion) => void
    vi.mocked(api.saveCover).mockReturnValue(new Promise(resolve => { finish = resolve }))
    await wrapper.get('form').trigger('submit'); await flushPromises()
    expect(wrapper.get('fieldset').element.disabled).toBe(true)
    finish(initial); await flushPromises(); expect(wrapper.get('fieldset').element.disabled).toBe(false)
  })
})
