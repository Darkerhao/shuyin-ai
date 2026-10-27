import { chooseSelect, openSelect } from './ui'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createMemoryHistory, createRouter } from 'vue-router'
import App from '../src/App.vue'
import CoversPage from '../src/modules/covers/CoversPage.vue'
import * as api from '../src/services/covers'
import * as ai from '../src/services/ai'
import * as drafts from '../src/services/drafts'
vi.mock('../src/services/drafts', () => ({ readDraft: vi.fn(), saveDraft: vi.fn() }))
import { emptyProvider, emptyPublishing, type WritingVersion } from '../src/types/ai'
import type { Asset } from '../src/types/project'
import * as projects from '../src/services/projects'
import * as renderer from '../src/services/coverRenderer'
import { coverTemplates, newCover, type CoverVersion } from '../src/types/cover'
import { emptyProject } from '../src/types/project'

vi.mock('../src/services/covers', () => ({ listCovers: vi.fn(), saveCover: vi.fn(), readBackground: vi.fn(), chooseBackground: vi.fn(), startCoverGeneration: vi.fn(), executeCoverGeneration: vi.fn(), listPresets: vi.fn(), savePreset: vi.fn(), deletePreset: vi.fn() }))
vi.mock('../src/services/ai', () => ({ listProviders: vi.fn(), listVersions: vi.fn(), cancelGeneration: vi.fn() }))
vi.mock('../src/services/projects', () => ({ isDesktop: vi.fn(), getProject: vi.fn(), listAssets: vi.fn(), linkAsset: vi.fn(), openProjectDirectory: vi.fn() }))
vi.mock('../src/services/coverRenderer', () => ({ coverFonts: { sans: 'sans-serif', serif: 'serif' }, renderCover: vi.fn(), coverPng: vi.fn(), decodeBackground: vi.fn() }))
const initial: CoverVersion = { id: 'v1', projectId: 'p1', versionNo: 1, config: newCover('最初的封面'), exportedPath: null, createdAt: '2026-10-23T00:00:00Z' }
const aiAsset: Asset = { id: 'a1', projectId: 'p1', assetType: 'image', filePath: 'D:/ai.png', originalName: 'ai.png', fileSize: 100, modifiedAt: 0, existsStatus: 'available', createdAt: '' }
const wrappers: ReturnType<typeof mount>[] = []
const writing: WritingVersion = { id: 'w1', projectId: 'p1', kind: 'publishing', versionNo: 1, content: { ...emptyPublishing(), coverTitle: '轮回几世，何时放下', coverSubtitle: '《生死疲劳》里的人间执念', coverPromptLandscape: '横版 4:3，田野向两侧延伸，无字背景。', coverPromptPortrait: '竖版 3:4，田垄引向远方，无字背景。' }, sourceType: 'ai', modelName: 'text', promptVersion: 'publishing-v2', isConfirmed: false, createdAt: '' }
async function render(path = '/projects/p1/covers') {
  const router = createRouter({ history: createMemoryHistory(), routes: [
    { path: '/projects/:id/covers', component: CoversPage, props: true },
    { path: '/projects/:id/writing', component: { template: '<p>writing</p>' } },
    { path: '/settings', component: { template: '<p>Settings</p>' } },
    { path: '/projects/:id/publishing', component: { template: '<p>publishing</p>' } },
    { path: '/projects', component: { template: '<p>projects</p>' } },
    { path: '/settings/models', component: { template: '<p>models</p>' } },
  ] })
  await router.push(path); await router.isReady()
  const wrapper = mount(App, { attachTo: document.body, global: { plugins: [router] } }); wrappers.push(wrapper)
  await flushPromises(); return { wrapper, router }
}
function button(wrapper: ReturnType<typeof mount>, name: string) { return wrapper.findAll('button').find(b => b.text() === name)! }
beforeEach(() => {
  vi.resetAllMocks(); vi.stubGlobal('confirm', vi.fn(() => false))
  vi.mocked(drafts.readDraft).mockResolvedValue(null)
  vi.mocked(drafts.saveDraft).mockResolvedValue(undefined)
  vi.mocked(api.listPresets).mockResolvedValue([])
  vi.stubGlobal('ResizeObserver', class { observe() {} unobserve() {} disconnect() {} })
  Object.defineProperty(document, 'fonts', { value: { ready: Promise.resolve() }, configurable: true })
  vi.mocked(projects.isDesktop).mockReturnValue(true)
  vi.mocked(projects.getProject).mockResolvedValue({ ...emptyProject(), title: '测试书', id: 'p1', workspacePath: 'D:/books/p1', createdAt: '', updatedAt: '', archivedAt: null })
  vi.mocked(projects.listAssets).mockResolvedValue([])
  vi.mocked(ai.listProviders).mockResolvedValue([])
  vi.mocked(ai.listVersions).mockResolvedValue([])
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
  const design = { titleSize: 148, subtitleSize: 54, textColor: '#ffe45c', subtitleColor: '#fff8e8', accentColor: '#ff7547', backgroundColor: '#141414', overlayOpacity: 0.75 }
  it('automatically carries AI design into new covers and exports the design in all ratios', async () => {
    vi.mocked(ai.listVersions).mockResolvedValue([{ ...writing, content: { ...writing.content, coverDesign: design } }])
    vi.mocked(api.listCovers).mockResolvedValue([])
    const { wrapper } = await render()
    await button(wrapper, '排版样式').trigger('click')
    expect(wrapper.get('[aria-label="AI 推荐排版"]').text()).toContain('148 px')
    expect((wrapper.get('#cover-subtitle-color').element as HTMLInputElement).value).toBe(design.subtitleColor)
    await button(wrapper, '导出 3 张封面').trigger('click'); await flushPromises()
    for (const [, config] of vi.mocked(api.saveCover).mock.calls) expect(config).toMatchObject(design)
    expect(api.saveCover).toHaveBeenCalledTimes(3)
  })
  it('applies AI design to an existing cover only on request and preserves copy and crop', async () => {
    vi.mocked(ai.listVersions).mockResolvedValue([{ ...writing, content: { ...writing.content, coverDesign: design } }])
    const config = { ...initial.config, backgroundX: 0.2, backgroundZoom: 1.6 }
    vi.mocked(api.listCovers).mockResolvedValue([{ ...initial, config }])
    const { wrapper } = await render()
    expect(renderer.renderCover).toHaveBeenCalledWith(expect.anything(), config, null)
    await button(wrapper, '排版样式').trigger('click')
    await button(wrapper, '应用 AI 推荐排版').trigger('click')
    await wrapper.get('#cover-subtitle-color').setValue('#aabbcc')
    await wrapper.get('#cover-edit-form').trigger('submit'); await flushPromises()
    expect(api.saveCover).toHaveBeenCalledWith('p1', { ...config, ...design, subtitleColor: '#aabbcc' }, null)
  })
  it.each(Object.entries(coverTemplates))('applies the complete %s preset while keeping this book content', async (key, template) => {
    const { wrapper } = await render()
    await button(wrapper, '排版样式').trigger('click')
    const card = wrapper.get(`button[aria-label="${template.label}"]`)
    await card.trigger('click')
    expect(card.attributes('aria-pressed')).toBe('true')
    await wrapper.get('#cover-edit-form').trigger('submit'); await flushPromises()
    expect(api.saveCover).toHaveBeenCalledWith('p1', { ...initial.config, ...template.style, templateId: key }, null)
  })
  it.each(['version', 'draft', 'preset'])('keeps the original text color for legacy %s data', async source => {
    const config = { ...initial.config, textColor: '#abcdef' }
    Reflect.deleteProperty(config, 'subtitleColor')
    if (source === 'version') vi.mocked(api.listCovers).mockResolvedValue([{ ...initial, config }])
    if (source === 'draft') vi.mocked(drafts.readDraft).mockResolvedValue({ basedOn: initial.id, content: config })
    if (source === 'preset') vi.mocked(api.listPresets).mockResolvedValue([{ id: 'legacy', name: '旧个人样式', style: config }])
    const { wrapper } = await render()
    await button(wrapper, '排版样式').trigger('click')
    if (source === 'preset') await chooseSelect(wrapper, '#cover-preset', '旧个人样式')
    expect((wrapper.get('#cover-subtitle-color').element as HTMLInputElement).value).toBe('#abcdef')
    await wrapper.get('#cover-edit-form').trigger('submit'); await flushPromises()
    expect(api.saveCover).toHaveBeenCalledWith('p1', expect.objectContaining({ textColor: '#abcdef', subtitleColor: '#abcdef' }), null)
  })
  it('applies template, typography and numeric controls without changing the cover copy', async () => {
    const { wrapper } = await render()
    await button(wrapper, '排版样式').trigger('click')
    await wrapper.get('button[aria-label="深林"]').trigger('click')
    expect(wrapper.get('#cover-font').text()).toBe('黑体 / 系统无衬线')
    await chooseSelect(wrapper, '#cover-align', '左对齐')
    await chooseSelect(wrapper, '#cover-position', '靠下')
    await wrapper.get('#cover-title-size').setValue('96')
    await wrapper.get('#cover-subtitle-size').setValue('36')
    await wrapper.get('#cover-bg-color').setValue('#112233')
    await wrapper.get('#cover-edit-form').trigger('submit'); await flushPromises()
    expect(api.saveCover).toHaveBeenCalledExactlyOnceWith('p1', expect.objectContaining({
      templateId: 'night', fontFamily: 'sans', align: 'left', position: 'bottom', titleSize: 96, subtitleSize: 36, backgroundColor: '#112233', title: initial.config.title,
    }), null)
  })

  it('keeps the selected history label and draft when a different version is declined', async () => {
    vi.mocked(api.listCovers).mockResolvedValue([{ ...initial, id: 'v2', versionNo: 2 }, initial])
    const { wrapper } = await render()
    await wrapper.get('#cover-title').setValue('不要丢失的草稿')
    await wrapper.get('.history-panel [data-slot=collapsible-trigger]').trigger('click')
    await chooseSelect(wrapper, '#cover-history', /^v1 ·/)
    expect(globalThis.confirm).toHaveBeenCalled()
    expect(wrapper.get('#cover-history').text()).toContain('v2')
    expect((wrapper.get('#cover-title').element as HTMLTextAreaElement).value).toBe('不要丢失的草稿')
    vi.mocked(globalThis.confirm).mockReturnValue(true)
    await chooseSelect(wrapper, '#cover-history', /^v1 ·/)
    expect(wrapper.get('#cover-history').text()).toContain('v1')
    expect((wrapper.get('#cover-title').element as HTMLTextAreaElement).value).toBe(initial.config.title)
  })

  it('updates overlay opacity by keyboard and disables custom controls while saving', async () => {
    vi.mocked(api.listCovers).mockResolvedValue([{ ...initial, config: { ...initial.config, backgroundAssetId: 'a1' } }])
    vi.mocked(projects.listAssets).mockResolvedValue([aiAsset])
    vi.mocked(api.readBackground).mockResolvedValue(new ArrayBuffer(1))
    vi.mocked(renderer.decodeBackground).mockResolvedValue(document.createElement('img'))
    const { wrapper } = await render()
    await button(wrapper, '背景图片').trigger('click')
    const slider = wrapper.get('[role=slider]')
    expect(slider.attributes('aria-label')).toBe('底色蒙层')
    await slider.trigger('keydown', { key: 'ArrowRight' })
    expect(slider.attributes('aria-valuenow')).toBe('0.8')
    let finish!: (value: CoverVersion) => void
    vi.mocked(api.saveCover).mockReturnValue(new Promise(resolve => { finish = resolve }))
    await wrapper.get('#cover-edit-form').trigger('submit'); await flushPromises()
    expect(api.saveCover).toHaveBeenCalledWith('p1', expect.objectContaining({ overlayOpacity: 0.8 }), null)
    expect(wrapper.get('#cover-background').element.matches(':disabled')).toBe(true)
    expect(wrapper.get('button[aria-label="深林"]').element.matches(':disabled')).toBe(true)
    expect(slider.attributes('data-disabled')).toBeDefined()
    finish(initial); await flushPromises()
  })
  it('prefills a new cover with the selected writing version and keeps it unsaved', async () => {
    vi.mocked(ai.listVersions).mockResolvedValue([{ ...writing, id: 'w2', content: { ...writing.content, coverTitle: '较新的标题' } }, writing])
    vi.mocked(api.listCovers).mockResolvedValue([])
    const { wrapper, router } = await render('/projects/p1/covers?writingVersion=w1')
    expect((wrapper.get('#cover-title').element as HTMLTextAreaElement).value).toBe(writing.content.coverTitle)
    expect((wrapper.get('#cover-subtitle').element as HTMLTextAreaElement).value).toBe(writing.content.coverSubtitle)
    expect(api.saveCover).not.toHaveBeenCalled()
    await router.push('/projects'); expect(router.currentRoute.value.path).toBe('/projects')
    expect(drafts.saveDraft).toHaveBeenLastCalledWith('p1', 'cover', expect.objectContaining({ basedOn: null, content: expect.objectContaining({ title: writing.content.coverTitle }) }))
  })
  it('preserves existing cover styles and text until titles are explicitly applied', async () => {
    vi.mocked(ai.listVersions).mockResolvedValue([writing])
    const { wrapper } = await render()
    expect((wrapper.get('#cover-title').element as HTMLTextAreaElement).value).toBe(initial.config.title)
    await button(wrapper, '使用这组主副标题').trigger('click'); await flushPromises()
    expect((wrapper.get('#cover-title').element as HTMLTextAreaElement).value).toBe(writing.content.coverTitle)
    await button(wrapper, '导出 3 张封面').trigger('click'); await flushPromises()
    expect(api.saveCover).toHaveBeenNthCalledWith(1, 'p1', { ...initial.config, title: writing.content.coverTitle, subtitle: writing.content.coverSubtitle }, expect.any(Array))
  })
  it('copies either ratio without an image model and applies a prompt only on request', async () => {
    const writeText = vi.fn().mockResolvedValue(undefined)
    vi.stubGlobal('navigator', { clipboard: { writeText } })
    vi.mocked(ai.listVersions).mockResolvedValue([writing])
    const { wrapper } = await render()
    await button(wrapper, '复制封面提示词').trigger('click'); await flushPromises()
    expect(writeText).toHaveBeenLastCalledWith(writing.content.coverPromptLandscape)
    await chooseSelect(wrapper, '#cover-prompt-ratio', '竖版 3:4 · 1080 × 1440')
    await button(wrapper, '复制封面提示词').trigger('click'); await flushPromises()
    expect(writeText).toHaveBeenLastCalledWith(writing.content.coverPromptPortrait)
    expect(ai.listProviders).not.toHaveBeenCalled()
    await button(wrapper, '用于 AI 背景').trigger('click'); await flushPromises()
    expect((wrapper.get('#cover-description').element as HTMLTextAreaElement).value).toBe(writing.content.coverPromptPortrait)
    await chooseSelect(wrapper, '#cover-prompt-ratio', '横版 4:3 · 1440 × 1080')
    expect((wrapper.get('#cover-description').element as HTMLTextAreaElement).value).toBe(writing.content.coverPromptPortrait)
    expect(api.startCoverGeneration).not.toHaveBeenCalled()
  })
  it('allows local editing after writing lookup fails and does not overwrite it on retry', async () => {
    vi.mocked(ai.listVersions).mockRejectedValueOnce('读取失败').mockResolvedValue([writing])
    const { wrapper } = await render()
    await wrapper.get('#cover-title').setValue('自己的标题')
    await button(wrapper, '重新读取').trigger('click'); await flushPromises()
    expect((wrapper.get('#cover-title').element as HTMLTextAreaElement).value).toBe('自己的标题')
    await button(wrapper, '导出 3 张封面').trigger('click'); await flushPromises()
    expect(api.saveCover).toHaveBeenCalledTimes(3)
  })
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
  it('uses only enabled image models, preserves text and exports the generated background in all three ratios', async () => {
    const { wrapper } = await aiCover()
    const models = await openSelect(wrapper, '#cover-model')
    expect(models.findAll('[role=option]').map(o => o.text())).toEqual(['图片模型'])
    await models.trigger('keydown', { key: 'Escape' }); await flushPromises()
    await wrapper.get('#cover-title').setValue('手动修改标题')
    await wrapper.get('#cover-description').setValue('水墨山林，安静留白')
    await wrapper.get('section[aria-label="可选 AI 背景"] form').trigger('submit'); await flushPromises()
    expect(api.startCoverGeneration).toHaveBeenCalledExactlyOnceWith('p1', 'image', '水墨山林，安静留白')
    expect((wrapper.get('#cover-title').element as HTMLTextAreaElement).value).toBe('手动修改标题')
    expect(wrapper.get('#cover-background').text()).toBe('ai.png')
    expect(api.saveCover).not.toHaveBeenCalled()
    await button(wrapper, '导出 3 张封面').trigger('click'); await flushPromises()
    expect(api.saveCover).toHaveBeenNthCalledWith(1, 'p1', expect.objectContaining({ backgroundAssetId: 'a1', width: 1440, height: 1080 }), expect.any(Array))
    expect(api.saveCover).toHaveBeenNthCalledWith(2, 'p1', expect.objectContaining({ backgroundAssetId: 'a1', width: 1080, height: 1440 }), expect.any(Array))
    expect(api.saveCover).toHaveBeenNthCalledWith(3, 'p1', expect.objectContaining({ backgroundAssetId: 'a1', width: 1920, height: 1080 }), expect.any(Array))
  })
  it.each(['network', 'decode'])('keeps current background and edits when AI fails at %s', async failure => {
    vi.mocked(api.listCovers).mockResolvedValue([{ ...initial, config: { ...initial.config, backgroundAssetId: 'old' } }])
    const { wrapper } = await aiCover()
    await wrapper.get('#cover-title').setValue('保留草稿')
    if (failure === 'network') vi.mocked(api.executeCoverGeneration).mockRejectedValue({ message: '服务失败' })
    else vi.mocked(renderer.decodeBackground).mockRejectedValueOnce(new Error('图片解码失败'))
    await wrapper.get('section[aria-label="可选 AI 背景"] form').trigger('submit'); await flushPromises()
    expect(wrapper.get('#cover-background').text()).toBe('背景关联已失效，请重新选择')
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
    await button(wrapper, '导出 3 张封面').trigger('click'); await flushPromises()
    expect(api.saveCover).toHaveBeenCalledTimes(3)
  })
  it('allows local export when reading image model settings fails', async () => {
    vi.mocked(ai.listProviders).mockRejectedValue('模型列表读取失败')
    const { wrapper } = await render()
    await button(wrapper, '使用 AI 生成背景（可选）').trigger('click'); await flushPromises()
    await button(wrapper, '导出 3 张封面').trigger('click'); await flushPromises()
    expect(api.saveCover).toHaveBeenCalledTimes(3)
  })
  it('keeps local export available without querying or configuring image models', async () => {
    const { wrapper } = await render()
    expect(ai.listProviders).not.toHaveBeenCalled()
    await button(wrapper, '使用 AI 生成背景（可选）').trigger('click'); await flushPromises()
    expect(wrapper.text()).toContain('尚无已启用的图片模型')
    await button(wrapper, '导出 3 张封面').trigger('click'); await flushPromises()
    expect(api.saveCover).toHaveBeenCalledTimes(3)
    expect(api.startCoverGeneration).not.toHaveBeenCalled()
  })
  it('does not call desktop services in a browser preview', async () => {
    vi.mocked(projects.isDesktop).mockReturnValue(false)
    const { wrapper } = await render()
    expect(api.listCovers).not.toHaveBeenCalled(); expect(wrapper.text()).toContain('请在桌面应用中制作封面')
  })
  it('restores the latest configuration and exports all three covers while retaining the preview ratio', async () => {
    const { wrapper } = await render()
    const sizes = await openSelect(wrapper, '#cover-size')
    expect(sizes.findAll('[role=option]').map(o => o.text())).toEqual(['横版 4:3 · 1440 × 1080', '竖版 3:4 · 1080 × 1440', '横版 16:9 · 1920 × 1080'])
    await sizes.trigger('keydown', { key: 'Escape' }); await flushPromises()
    expect((wrapper.get('#cover-title').element as HTMLTextAreaElement).value).toBe(initial.config.title)
    await wrapper.get('#cover-title').setValue('新的中文封面')
    await chooseSelect(wrapper, '#cover-size', '竖版 3:4 · 1080 × 1440')
    await button(wrapper, '导出 3 张封面').trigger('click'); await flushPromises()
    expect(api.saveCover).toHaveBeenCalledTimes(3)
    expect(api.saveCover).toHaveBeenNthCalledWith(1, 'p1', expect.objectContaining({ title: '新的中文封面', width: 1440, height: 1080 }), [137, 80, 78, 71])
    expect(api.saveCover).toHaveBeenNthCalledWith(2, 'p1', expect.objectContaining({ title: '新的中文封面', width: 1080, height: 1440 }), [137, 80, 78, 71])
    expect(api.saveCover).toHaveBeenNthCalledWith(3, 'p1', expect.objectContaining({ title: '新的中文封面', width: 1920, height: 1080 }), [137, 80, 78, 71])
    expect(wrapper.get('#cover-size').text()).toBe('竖版 3:4 · 1080 × 1440')
    expect(wrapper.get('[role="status"]').text()).toContain('已导出 3 张封面')
    expect(wrapper.get('[role="status"]').text()).toContain('D:/cover-1440x1080-v2.png')
    expect(wrapper.get('[role="status"]').text()).toContain('D:/cover-1080x1440-v3.png')
    expect(wrapper.get('[role="status"]').text()).toContain('D:/cover-1920x1080-v4.png')
    expect(initial.config.title).toBe('最初的封面')
    await wrapper.get('.history-panel [data-slot=collapsible-trigger]').trigger('click')
    await chooseSelect(wrapper, '#cover-history', /^v1 ·/); await flushPromises()
    expect((wrapper.get('#cover-title').element as HTMLTextAreaElement).value).toBe('最初的封面')
  })
  it('preserves edits and blocks accidental history or route changes after export failure', async () => {
    const { wrapper, router } = await render()
    await wrapper.get('#cover-title').setValue('失败仍保留')
    vi.mocked(api.saveCover).mockRejectedValue('封面目录不可写')
    await button(wrapper, '导出 3 张封面').trigger('click'); await flushPromises()
    expect(wrapper.get('[role="alert"]').text()).toContain('不可写')
    await wrapper.get('.history-panel [data-slot=collapsible-trigger]').trigger('click')
    await chooseSelect(wrapper, '#cover-history', /^v1 ·/)
    expect((wrapper.get('#cover-title').element as HTMLTextAreaElement).value).toBe('失败仍保留')
    await router.push('/projects')
    expect(router.currentRoute.value.path).toBe('/projects')
    expect(drafts.saveDraft).toHaveBeenLastCalledWith('p1', 'cover', expect.objectContaining({ content: expect.objectContaining({ title: '失败仍保留' }) }))
  })
  it('keeps draft on render failure and saves config without requiring an image', async () => {
    const { wrapper } = await render()
    await wrapper.get('#cover-title').setValue('配置草稿')
    vi.mocked(renderer.coverPng).mockRejectedValue(new Error('无法导出 PNG'))
    await button(wrapper, '导出 3 张封面').trigger('click'); await flushPromises()
    expect(api.saveCover).not.toHaveBeenCalled()
    vi.mocked(api.saveCover).mockImplementation(async (projectId, config) => ({ ...initial, projectId, config, id: 'v2', versionNo: 2 }))
    await wrapper.get('form').trigger('submit'); await flushPromises()
    expect(api.saveCover).toHaveBeenCalledWith('p1', expect.objectContaining({ title: '配置草稿' }), null)
  })
  it('saves and restores a 16:9 draft without converting its size or changing its content', async () => {
    const { wrapper } = await render()
    await chooseSelect(wrapper, '#cover-size', '横版 16:9 · 1920 × 1080')
    expect(renderer.renderCover).toHaveBeenLastCalledWith(expect.anything(), { ...initial.config, width: 1920, height: 1080 }, null)
    expect(wrapper.get('canvas').attributes('style')).toContain('1920 / 1080')
    await wrapper.get('#cover-edit-form').trigger('submit'); await flushPromises()
    expect(api.saveCover).toHaveBeenCalledExactlyOnceWith('p1', { ...initial.config, width: 1920, height: 1080 }, null)
    const saved = await vi.mocked(api.saveCover).mock.results[0]!.value as CoverVersion
    wrapper.unmount(); wrappers.splice(wrappers.indexOf(wrapper), 1)
    vi.mocked(api.listCovers).mockResolvedValue([saved, initial])
    const { wrapper: restored } = await render()
    expect(restored.get('#cover-size').text()).toBe('横版 16:9 · 1920 × 1080')
    expect((restored.get('#cover-title').element as HTMLTextAreaElement).value).toBe(initial.config.title)
    expect(restored.text()).not.toContain('当前按')
    await button(restored, '导出 3 张封面').trigger('click'); await flushPromises()
    expect(restored.get('#cover-size').text()).toBe('横版 16:9 · 1920 × 1080')
  })
  it.each([1, 2])('writes no covers when ratio index %i cannot be rendered', async index => {
    const { wrapper } = await render()
    await wrapper.get('#cover-title').setValue('所有比例都要完整排版')
    for (let i = 0; i < index; i++) vi.mocked(renderer.coverPng).mockResolvedValueOnce([137, 80, 78, 71])
    vi.mocked(renderer.coverPng).mockRejectedValueOnce(new Error('文字超出安全区域'))
    await button(wrapper, '导出 3 张封面').trigger('click'); await flushPromises()
    expect(api.saveCover).not.toHaveBeenCalled()
    expect(wrapper.get('[role="alert"]').text()).toContain('文字超出安全区域')
    expect((wrapper.get('#cover-title').element as HTMLTextAreaElement).value).toBe('所有比例都要完整排版')
  })
  it('retains the first export and the unsaved draft when writing the second cover fails', async () => {
    const { wrapper, router } = await render()
    await wrapper.get('#cover-title').setValue('第二张失败仍保留')
    vi.mocked(api.saveCover).mockResolvedValueOnce({ ...initial, id: 'v2', versionNo: 2, exportedPath: 'D:/cover-1440x1080-v2.png' }).mockRejectedValueOnce('竖版封面目录不可写')
    await button(wrapper, '导出 3 张封面').trigger('click'); await flushPromises()
    expect(wrapper.get('[role="status"]').text()).toContain('已导出 1/3 张封面：D:/cover-1440x1080-v2.png')
    expect(wrapper.get('[role="alert"]').text()).toContain('不可写')
    expect(wrapper.get('.history-panel [data-slot=collapsible-trigger]').text()).toContain('2 个版本')
    expect((wrapper.get('#cover-title').element as HTMLTextAreaElement).value).toBe('第二张失败仍保留')
    await router.push('/projects')
    expect(router.currentRoute.value.path).toBe('/projects')
    expect(drafts.saveDraft).toHaveBeenLastCalledWith('p1', 'cover', expect.objectContaining({ basedOn: 'v1', content: expect.objectContaining({ title: '第二张失败仍保留' }) }))
  })
  it('retains both completed exports and the draft when writing the 16:9 cover fails', async () => {
    const saveCover = vi.mocked(api.saveCover).getMockImplementation()!
    vi.mocked(api.saveCover).mockImplementation((projectId, config, png) => config.width === 1920
      ? Promise.reject('16:9 封面目录不可写') : saveCover(projectId, config, png))
    const { wrapper, router } = await render()
    await wrapper.get('#cover-title').setValue('第三张失败仍保留')
    await button(wrapper, '导出 3 张封面').trigger('click'); await flushPromises()
    expect(wrapper.get('[role="status"]').text()).toContain('已导出 2/3 张封面')
    expect(wrapper.get('[role="status"]').text()).toContain('D:/cover-1440x1080-v2.png')
    expect(wrapper.get('[role="status"]').text()).toContain('D:/cover-1080x1440-v3.png')
    expect(wrapper.get('[role="alert"]').text()).toContain('16:9 封面目录不可写')
    expect(wrapper.get('.history-panel [data-slot=collapsible-trigger]').text()).toContain('3 个版本')
    expect((wrapper.get('#cover-title').element as HTMLTextAreaElement).value).toBe('第三张失败仍保留')
    await router.push('/projects')
    expect(router.currentRoute.value.path).toBe('/projects')
    expect(drafts.saveDraft).toHaveBeenLastCalledWith('p1', 'cover', expect.objectContaining({ basedOn: 'v1', content: expect.objectContaining({ title: '第三张失败仍保留' }) }))
  })
  it('recovers cover crop and text without creating a formal cover version', async () => {
    const config = { ...initial.config, title: '恢复封面草稿', backgroundX: 0.2, backgroundY: 0.8, backgroundZoom: 1.5 }
    vi.mocked(drafts.readDraft).mockResolvedValue({ basedOn: initial.id, content: config })
    const { wrapper, router } = await render()
    expect((wrapper.get('#cover-title').element as HTMLTextAreaElement).value).toBe(config.title)
    expect(renderer.renderCover).toHaveBeenCalledWith(expect.anything(), config, null)
    expect(wrapper.get('[aria-label="三种比例并排预览"]').findAll('canvas')).toHaveLength(3)
    expect(api.saveCover).not.toHaveBeenCalled()
    await router.push('/projects')
    expect(drafts.saveDraft).toHaveBeenLastCalledWith('p1', 'cover', expect.objectContaining({ basedOn: initial.id, content: config }))
  })
  it('applies a personal style without replacing this book copy, background or crop', async () => {
    const style = { templateId: 'night' as const, brand: '我的读书频道', fontFamily: 'sans' as const, titleSize: 96, subtitleSize: 36, align: 'center' as const, position: 'bottom' as const, backgroundColor: '#112233', textColor: '#ffffff', subtitleColor: '#ffeeaa', accentColor: '#ddaa11', overlayOpacity: 0.6 }
    vi.mocked(api.listPresets).mockResolvedValue([{ id: 'preset1', name: '我的品牌', style }])
    const { wrapper } = await render()
    await button(wrapper, '排版样式').trigger('click')
    await chooseSelect(wrapper, '#cover-preset', '我的品牌')
    await wrapper.get('#cover-edit-form').trigger('submit'); await flushPromises()
    expect(api.saveCover).toHaveBeenCalledWith('p1', { ...initial.config, ...style }, null)
    await wrapper.get('[aria-label="新样式名称"]').setValue('新风格')
    await button(wrapper, '保存为我的样式').trigger('click'); await flushPromises()
    expect(api.savePreset).toHaveBeenCalledWith('新风格', { ...initial.config, ...style })
    await button(wrapper, '删除样式').trigger('click'); await flushPromises()
    expect(api.deletePreset).toHaveBeenCalledWith('preset1')
  })
  it.each([[1080, 1920, '1'], [1600, 900, '0'], [1080, 1080, '1']])('adapts historical %i × %i configurations without modifying their saved data', async (width, height, index) => {
    const legacy = { ...initial, config: { ...initial.config, width, height } }
    vi.mocked(api.listCovers).mockResolvedValue([legacy])
    const { wrapper } = await render()
    expect(wrapper.get('#cover-size').text()).toBe(index === '1' ? '竖版 3:4 · 1080 × 1440' : '横版 4:3 · 1440 × 1080')
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
    expect(wrapper.text()).toContain('背景图不存在'); expect(button(wrapper, '导出 3 张封面').element.disabled).toBe(true)
    await button(wrapper, '背景图片').trigger('click')
    await chooseSelect(wrapper, '#cover-background', '纯色背景'); await flushPromises()
    expect(button(wrapper, '导出 3 张封面').element.disabled).toBe(false)
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
