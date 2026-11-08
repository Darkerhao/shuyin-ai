import { beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { invoke } from '@tauri-apps/api/core'
import AutomationSetup from '../src/components/AutomationSetup.vue'
import { automationRuns, automationConsent, automationAccount, renderAutomationCover, type AutomationRun } from '../src/services/automation'
import { readBackground } from '../src/services/covers'
import { coverPng, decodeBackground } from '../src/services/coverRenderer'
import { newCover } from '../src/types/cover'
import { listProviders } from '../src/services/ai'

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }))
vi.mock('../src/services/projects', () => ({ isDesktop: () => true }))
vi.mock('../src/services/ai', () => ({ listProviders: vi.fn() }))
vi.mock('../src/services/covers', () => ({ readBackground: vi.fn() }))
vi.mock('../src/services/coverRenderer', () => ({ coverPng: vi.fn(), decodeBackground: vi.fn() }))

const run = (overrides: Partial<AutomationRun> = {}): AutomationRun => ({
  id: 'run1', projectId: 'book1', input: { workflowVersion: 2, durationReason: '已保存目标', title: '原书', author: '作者', targetDurationSeconds: 600, selection: { projectId: 'book1', sourceAssetId: 'source1', textProviderId: 'text1', imageProviderId: 'image1', format: 'deep_dive', requestKey: 'request1' } },
  status: 'running', stage: 'landscape_cover', runId: 2, error: '', audioJobId: 'audio1', videoJobId: 'video1', writingTaskId: 'writing1', writingVersionId: 'version1', finalWritingVersionId: null, actualDurationSeconds: null, widescreenTaskId: null, widescreenCoverId: null, durationTaskId: null, contentCheckTaskId: null, landscapeTaskId: 'background1', portraitTaskId: null, landscapeCoverId: null, portraitCoverId: null, publishTaskId: null, createdAt: '', updatedAt: '', ...overrides,
})
beforeEach(() => {
  vi.resetAllMocks(); automationRuns.value = []; automationConsent.value = false; automationAccount.value = null
  vi.mocked(readBackground).mockResolvedValue(new ArrayBuffer(4))
  vi.mocked(decodeBackground).mockResolvedValue({} as globalThis.HTMLImageElement)
  vi.mocked(coverPng).mockResolvedValue([1, 2, 3])
  vi.mocked(listProviders).mockResolvedValue([
    { id: 'text1', name: '文本', modelKind: 'text', baseUrl: 'http://localhost', modelName: 'text', reasoningEffort: null, isDefault: true, isEnabled: true, credentialStatus: 'available' },
    { id: 'image1', name: '图片', modelKind: 'image', baseUrl: 'http://localhost', modelName: 'image', reasoningEffort: null, isDefault: true, isEnabled: true, credentialStatus: 'available' },
  ])
})

describe('automatic cover execution outside the cover page', () => {
  it('exports the widescreen cover with the existing renderer and skips an already saved output', async () => {
    vi.mocked(invoke).mockResolvedValue({ ...newCover('宽屏'), width: 1920, height: 1080, backgroundAssetId: 'wide-background' })
    await renderAutomationCover(run({ stage: 'widescreen_cover', widescreenTaskId: 'wide-task' }))
    expect(readBackground).toHaveBeenCalledWith('book1', 'wide-background')
    expect(coverPng).toHaveBeenCalledOnce()
    expect(invoke).toHaveBeenCalledWith('complete_automation_cover', { id: 'run1', runId: 2, stage: 'widescreen_cover', png: [1, 2, 3] })
    await renderAutomationCover(run({ stage: 'widescreen_cover', widescreenCoverId: 'saved' }))
    expect(coverPng).toHaveBeenCalledOnce()
  })
  it('renders the frozen background once and includes run generation and stage', async () => {
    let release!: (value: number[]) => void
    vi.mocked(coverPng).mockImplementation(() => new Promise(resolve => { release = resolve }))
    vi.mocked(invoke).mockResolvedValue({ ...newCover('本次文案'), backgroundAssetId: 'background1' })
    const pending = renderAutomationCover(run()); await flushPromises()
    await renderAutomationCover(run())
    expect(coverPng).toHaveBeenCalledTimes(1)
    release([1, 2, 3]); await pending
    expect(readBackground).toHaveBeenCalledWith('book1', 'background1')
    expect(invoke).toHaveBeenCalledWith('complete_automation_cover', { id: 'run1', runId: 2, stage: 'landscape_cover', png: [1, 2, 3] })
  })
  it('does not render completed, paused or non-cover work', async () => {
    for (const overrides of [{ status: 'paused' as const }, { landscapeCoverId: 'saved' }, { stage: 'render' }]) await renderAutomationCover(run(overrides))
    expect(invoke).not.toHaveBeenCalled(); expect(coverPng).not.toHaveBeenCalled()
  })
  it('records decoding failures without generating another paid image', async () => {
    vi.mocked(invoke).mockResolvedValue({ ...newCover('本次文案'), backgroundAssetId: 'background1' })
    vi.mocked(decodeBackground).mockRejectedValue(new Error('bad image'))
    await renderAutomationCover(run())
    expect(invoke).toHaveBeenCalledWith('fail_automation_cover', { id: 'run1', runId: 2, stage: 'landscape_cover' })
    expect(coverPng).not.toHaveBeenCalled()
    expect(vi.mocked(invoke).mock.calls.map(call => call[0])).toEqual(['automation_cover', 'fail_automation_cover'])
  })
})

describe('automatic workflow setup', () => {
  it('requires explicit consent and the confirmed visible account before starting', async () => {
    const account = { id: null, name: '本次账号', key: 'identity' }
    vi.mocked(invoke).mockImplementation(async command => command === 'prepare_automation_account' ? account : command === 'automation_snapshot' ? [run()] : run())
    const saveSource = vi.fn().mockResolvedValue({ projectId: 'book1', sourceAssetId: 'source1' })
    const wrapper = mount(AutomationSetup, { props: { saveSource } })
    await flushPromises()
    const button = (label: string) => wrapper.findAll('button').find(button => button.text() === label)!
    expect(button('自动制作并准备发布').attributes('disabled')).toBeDefined()
    await button('打开抖音并读取账号').trigger('click'); await flushPromises()
    expect(wrapper.text()).toContain('本次账号')
    await button('确认使用此账号').trigger('click'); await flushPromises()
    expect(button('自动制作并准备发布').attributes('disabled')).toBeDefined()
    await wrapper.get('[role="checkbox"]').trigger('click'); await flushPromises()
    expect(button('自动制作并准备发布').attributes('disabled')).toBeUndefined()
    await button('自动制作并准备发布').trigger('click'); await flushPromises()
    expect(saveSource).toHaveBeenCalledTimes(1)
    expect(invoke).toHaveBeenCalledWith('start_automation', { input: expect.objectContaining({ projectId: 'book1', sourceAssetId: 'source1', textProviderId: 'text1', imageProviderId: 'image1', format: 'deep_dive' }) })
    expect(wrapper.emitted('busy')?.at(-1)).toEqual([false])
    wrapper.unmount()
  })
  it('keeps the request identity on failed start and exposes the error without starting from partial input', async () => {
    automationConsent.value = true; automationAccount.value = { id: null, name: '账号', key: 'one' }
    vi.mocked(invoke).mockRejectedValue('运行环境缺失')
    const saveSource = vi.fn().mockResolvedValue({ projectId: 'book1', sourceAssetId: 'source1' })
    const wrapper = mount(AutomationSetup, { props: { saveSource } }); await flushPromises()
    const start = () => wrapper.findAll('button').find(button => button.text() === '自动制作并准备发布')!
    await start().trigger('click'); await flushPromises()
    expect(wrapper.get('[role="alert"]').text()).toContain('运行环境缺失')
    await start().trigger('click'); await flushPromises()
    const inputs = vi.mocked(invoke).mock.calls.filter(call => call[0] === 'start_automation').map(call => call[1])
    expect(inputs).toHaveLength(2); expect(inputs[0]).toEqual(inputs[1])
    wrapper.unmount()
  })
})
