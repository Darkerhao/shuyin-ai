import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createRouter, createMemoryHistory } from 'vue-router'
import App from '../src/App.vue'
import WritingPage from '../src/modules/writing/WritingPage.vue'
import ModelsPage from '../src/modules/settings/ModelsPage.vue'
import * as api from '../src/services/ai'
import * as projects from '../src/services/projects'
import { emptyProject } from '../src/types/project'
import { emptyProvider, type ModelProvider, type WritingVersion } from '../src/types/ai'
vi.mock('../src/services/ai', () => ({ listProviders: vi.fn(), saveProvider: vi.fn(), deleteProvider: vi.fn(), testConnection: vi.fn(), listVersions: vi.fn(), saveWriting: vi.fn(), startGeneration: vi.fn(), executeGeneration: vi.fn(), cancelGeneration: vi.fn() }))
vi.mock('../src/services/projects', () => ({ isDesktop: vi.fn(), getProject: vi.fn() }))
const provider: ModelProvider = { ...emptyProvider(), id: 'm1', name: '本机模型', modelName: 'model', isDefault: true, credentialStatus: 'available' }
const initial: WritingVersion = { id: 'v1', projectId: 'p1', kind: 'publishing', versionNo: 1, content: { format: 'publishing', title: '原有标题', description: '原有内容', audioPrompt: '原有提示词' }, sourceType: 'ai', modelName: 'model', promptVersion: 'writing-v1', isConfirmed: true, createdAt: '2026-10-23T00:00:00Z' }
const generated: WritingVersion = { ...initial, id: 'v2', versionNo: 2, content: { format: 'publishing', title: '新标题', description: '新内容', audioPrompt: '新提示词' }, isConfirmed: false }
const wrappers: ReturnType<typeof mount>[] = []
async function render(path = '/projects/p1/writing') {
  const router = createRouter({ history: createMemoryHistory(), routes: [
    { path: '/projects/:id/writing', component: WritingPage, props: true },
    { path: '/settings/models', component: ModelsPage },
    { path: '/projects', component: { template: '<p>Projects</p>' } },
    { path: '/projects/:id', component: { template: '<p>Project</p>' } },
    { path: '/settings', component: { template: '<p>Settings</p>' } },
    { path: '/verification', component: { template: '<p>Verification</p>' } },
  ] })
  await router.push(path); await router.isReady()
  const wrapper = mount(App, { global: { plugins: [router] } }); wrappers.push(wrapper); await flushPromises()
  return { wrapper, router }
}
function button(w: ReturnType<typeof mount>, text: string) { return w.findAll('button').find(b => b.text() === text)! }
beforeEach(() => {
  vi.resetAllMocks(); vi.stubGlobal('confirm', vi.fn(() => false))
  vi.mocked(projects.isDesktop).mockReturnValue(true)
  vi.mocked(projects.getProject).mockResolvedValue({ ...emptyProject(), id: 'p1', title: '测试书', workspacePath: '', archivedAt: null, createdAt: '', updatedAt: '' })
  vi.mocked(api.listProviders).mockResolvedValue([provider])
  vi.mocked(api.listVersions).mockImplementation(async (_id, kind) => kind === 'publishing' ? [initial] : [])
  vi.mocked(api.startGeneration).mockResolvedValue('t1')
})
afterEach(() => { wrappers.splice(0).forEach(w => w.unmount()); vi.unstubAllGlobals() })
describe('publishing workflow', () => {
  it('shows only three outputs and blocks desktop IPC in browser preview', async () => {
    vi.mocked(projects.isDesktop).mockReturnValue(false)
    const { wrapper } = await render()
    expect(api.listVersions).not.toHaveBeenCalled()
    expect(wrapper.text()).toContain('请在桌面应用')
  })
  it('generates all three outputs in one task without outline/count/reference controls', async () => {
    const { wrapper } = await render()
    expect(wrapper.findAll('textarea')).toHaveLength(3)
    expect(wrapper.find('#writing-kind').exists()).toBe(false)
    expect(wrapper.find('#writing-count').exists()).toBe(false)
    expect(wrapper.find('#writing-reference').exists()).toBe(false)
    vi.mocked(api.executeGeneration).mockResolvedValue(generated)
    await button(wrapper, '一键生成').trigger('click'); await flushPromises()
    expect(api.startGeneration).toHaveBeenCalledExactlyOnceWith({ projectId: 'p1', providerId: 'm1' })
    expect((wrapper.get('#result-title').element as HTMLTextAreaElement).value).toBe('新标题')
    expect((wrapper.get('#result-description').element as HTMLTextAreaElement).value).toBe('新内容')
    expect((wrapper.get('#result-audioPrompt').element as HTMLTextAreaElement).value).toBe('新提示词')
  })
  it('preserves all edited fields when generation fails', async () => {
    const { wrapper } = await render()
    await wrapper.get('#result-title').setValue('未保存标题')
    await wrapper.get('#result-description').setValue('未保存内容')
    await wrapper.get('#result-audioPrompt').setValue('未保存提示词')
    vi.mocked(api.executeGeneration).mockRejectedValue({ code: 'timeout', message: '模型请求超时' })
    await button(wrapper, '一键生成').trigger('click'); await flushPromises()
    expect(wrapper.get('[role="alert"]').text()).toContain('模型请求超时')
    expect((wrapper.get('#result-title').element as HTMLTextAreaElement).value).toBe('未保存标题')
    expect((wrapper.get('#result-description').element as HTMLTextAreaElement).value).toBe('未保存内容')
    expect((wrapper.get('#result-audioPrompt').element as HTMLTextAreaElement).value).toBe('未保存提示词')
    expect(api.saveWriting).not.toHaveBeenCalled()
  })
  it('keeps edits made during generation and leaves generated output in history', async () => {
    let complete!: (v: WritingVersion) => void
    vi.mocked(api.executeGeneration).mockReturnValue(new Promise(resolve => { complete = resolve }))
    const { wrapper } = await render(); await button(wrapper, '一键生成').trigger('click'); await flushPromises()
    await wrapper.get('#result-description').setValue('生成中手动编辑')
    complete(generated); await flushPromises()
    expect((wrapper.get('#result-description').element as HTMLTextAreaElement).value).toBe('生成中手动编辑')
    expect(wrapper.get('#writing-version').text()).toContain('v2')
    expect(wrapper.text()).toContain('当前修改已保留')
  })
  it('saves edits, preserves history, and guards navigation and version switching', async () => {
    const { wrapper, router } = await render()
    await wrapper.get('#result-title').setValue('人工修订标题')
    await router.push('/projects'); expect(router.currentRoute.value.path).toContain('/writing')
    await router.push('/projects/p2/writing'); expect(router.currentRoute.value.path).toBe('/projects/p1/writing')
    vi.mocked(api.saveWriting).mockRejectedValueOnce('保存失败')
    await wrapper.get('form').trigger('submit'); await flushPromises()
    expect((wrapper.get('#result-title').element as HTMLTextAreaElement).value).toBe('人工修订标题')
    const revised: WritingVersion = { ...generated, content: { format: 'publishing', title: '人工修订标题', description: '原有内容', audioPrompt: '原有提示词' }, sourceType: 'mixed' }
    vi.mocked(api.saveWriting).mockResolvedValue(revised)
    await wrapper.get('form').trigger('submit'); await flushPromises()
    expect(api.saveWriting).toHaveBeenLastCalledWith(expect.objectContaining({ kind: 'publishing', basedOn: 'v1', content: revised.content }))
    await wrapper.get('#writing-version').setValue('v1')
    expect((wrapper.get('#result-title').element as HTMLTextAreaElement).value).toBe('原有标题')
    await router.push('/projects'); expect(router.currentRoute.value.path).toBe('/projects')
  })
  it('cancels an in-flight task and restores navigation without losing data', async () => {
    let reject!: (e: unknown) => void
    vi.mocked(api.executeGeneration).mockReturnValue(new Promise((_resolve, fail) => { reject = fail }))
    vi.mocked(api.cancelGeneration).mockImplementation(async () => { reject({ code: 'cancelled', message: '生成已取消' }) })
    const { wrapper, router } = await render(); await button(wrapper, '一键生成').trigger('click'); await flushPromises()
    await router.push('/projects'); expect(router.currentRoute.value.path).toContain('/writing')
    await button(wrapper, '取消生成').trigger('click'); await flushPromises()
    expect(api.cancelGeneration).toHaveBeenCalledWith('t1')
    expect((wrapper.get('#result-title').element as HTMLTextAreaElement).value).toBe('原有标题')
    await router.push('/projects'); expect(router.currentRoute.value.path).toBe('/projects')
  })
  it('copies the current edited field, reports failure, and preserves content', async () => {
    const writeText = vi.fn().mockResolvedValue(undefined)
    vi.stubGlobal('navigator', { clipboard: { writeText } })
    const { wrapper } = await render()
    await wrapper.get('#result-description').setValue('编辑后的多行内容\n中文保持换行')
    await button(wrapper, '复制发布内容').trigger('click'); await flushPromises()
    expect(writeText).toHaveBeenLastCalledWith('编辑后的多行内容\n中文保持换行')
    writeText.mockRejectedValueOnce(new Error('denied'))
    await button(wrapper, '复制音频生成提示词').trigger('click'); await flushPromises()
    expect(wrapper.get('[role="alert"]').text()).toContain('复制失败')
    expect((wrapper.get('#result-audioPrompt').element as HTMLTextAreaElement).value).toBe('原有提示词')
  })
})

describe('model configuration', () => {
  it('preserves key input on save failure, clears it after success and never reads it back', async () => {
    const { wrapper } = await render('/settings/models')
    await wrapper.get('#provider-name').setValue('本机模型'); await wrapper.get('#model-name').setValue('model'); await wrapper.get('#api-key').setValue('test-only-key')
    vi.mocked(api.saveProvider).mockRejectedValueOnce('凭据库锁定')
    await wrapper.get('form').trigger('submit'); await flushPromises()
    expect((wrapper.get('#api-key').element as HTMLInputElement).value).toBe('test-only-key')
    vi.mocked(api.saveProvider).mockResolvedValueOnce({ provider, warning: null })
    await wrapper.get('form').trigger('submit'); await flushPromises()
    expect((wrapper.get('#api-key').element as HTMLInputElement).value).toBe('')
    expect(api.saveProvider).toHaveBeenLastCalledWith(null, expect.objectContaining({ name: '本机模型' }), 'test-only-key')
    vi.mocked(api.testConnection).mockResolvedValueOnce('连接成功')
    await button(wrapper, '测试连接').trigger('click'); await flushPromises(); expect(api.testConnection).toHaveBeenCalledWith('m1')
  })
  it('retains existing credentials when key input is empty and confirms deletion', async () => {
    const { wrapper } = await render('/settings/models'); await button(wrapper, '本机模型 · 默认 · 密钥已保存').trigger('click')
    vi.mocked(api.saveProvider).mockResolvedValueOnce({ provider, warning: null })
    await wrapper.get('form').trigger('submit'); await flushPromises(); expect(api.saveProvider).toHaveBeenCalledWith('m1', expect.any(Object), null)
    await button(wrapper, '删除模型').trigger('click'); expect(api.deleteProvider).not.toHaveBeenCalled()
    await button(wrapper, '确认删除').trigger('click'); await flushPromises(); expect(api.deleteProvider).toHaveBeenCalledWith('m1')
  })
})
