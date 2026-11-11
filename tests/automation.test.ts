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
  it('shows the active task models after reopening and after an automatic switch', async () => {
    const providers = await listProviders()
    vi.mocked(listProviders).mockResolvedValue([...providers,
      { ...providers[0]!, id: 'chosen-text', name: '本次文本', isDefault: false },
      { ...providers[1]!, id: 'chosen-image', name: '本次图片', isDefault: false },
    ])
    const active = run()
    active.input.selection.textProviderId = 'chosen-text'
    active.input.selection.imageProviderId = 'chosen-image'
    automationRuns.value = [active]
    const wrapper = mount(AutomationSetup, { props: { saveSource: vi.fn() } })
    await flushPromises()
    expect(wrapper.get('#automation-text').text()).toContain('本次文本')
    expect(wrapper.get('#automation-image').text()).toContain('本次图片')
    automationRuns.value = [run()]
    await flushPromises()
    expect(wrapper.get('#automation-text').text()).toContain('文本')
    expect(wrapper.get('#automation-text').text()).not.toContain('本次文本')
    expect(invoke).not.toHaveBeenCalled()
    wrapper.unmount()
  })

  it('requires explicit consent and the confirmed visible account before starting', async () => {
    const account = { id: null, name: '本次账号', key: 'identity' }
    vi.mocked(invoke).mockImplementation(async command => ['prepare_automation_account', 'confirm_automation_account'].includes(command) ? account : command === 'automation_snapshot' ? [run()] : run())
    const saveSource = vi.fn().mockResolvedValue({ projectId: 'book1', sourceAssetId: 'source1' })
    const wrapper = mount(AutomationSetup, { props: { saveSource } })
    await flushPromises()
    const button = (label: string) => wrapper.findAll('button').find(button => button.text() === label)!
    expect(button('自动制作并准备发布').attributes('disabled')).toBeDefined()
    await button('打开抖音并读取账号').trigger('click'); await flushPromises()
    expect(wrapper.get('#automation-account-name').element).toHaveProperty('value', '本次账号')
    await button('确认使用此账号').trigger('click'); await flushPromises()
    expect(button('自动制作并准备发布').attributes('disabled')).toBeDefined()
    await wrapper.get('[role="checkbox"]').trigger('click'); await flushPromises()
    expect(button('自动制作并准备发布').attributes('disabled')).toBeUndefined()
    await button('自动制作并准备发布').trigger('click'); await flushPromises()
    expect(saveSource).toHaveBeenCalledTimes(1)
    expect(invoke).toHaveBeenCalledWith('start_automation', { input: expect.objectContaining({ projectId: 'book1', sourceAssetId: 'source1', textProviderId: 'text1', imageProviderId: 'image1', format: 'deep_dive', collectionName: null }) })
    expect(wrapper.emitted('busy')?.at(-1)).toEqual([false])
    wrapper.unmount()
  })
  it('saves the chosen collection at startup and does not ask for the same account after remounting', async () => {
    const account = { id: null, name: '本次账号', key: 'identity' }
    automationConsent.value = true
    vi.mocked(invoke).mockImplementation(async command => ['prepare_automation_account', 'confirm_automation_account'].includes(command) ? account : command === 'automation_snapshot' ? [] : run())
    const saveSource = vi.fn().mockResolvedValue({ projectId: 'book1', sourceAssetId: 'source1' })
    const wrapper = mount(AutomationSetup, { props: { saveSource } })
    await flushPromises()
    const button = (label: string) => wrapper.findAll('button').find(button => button.text() === label)!
    await button('打开抖音并读取账号').trigger('click'); await flushPromises()
    await button('确认使用此账号').trigger('click'); await flushPromises()
    wrapper.findAllComponents({ name: 'UiSelect' }).at(-1)!.vm.$emit('update:modelValue', 'named')
    await flushPromises()
    expect(button('自动制作并准备发布').attributes('disabled')).toBeDefined()
    await wrapper.get('#automation-collection-name').setValue('  甲子读书 | 读懂一本书  ')
    wrapper.findAllComponents({ name: 'UiSelect' }).at(-1)!.vm.$emit('update:modelValue', 'named'); await flushPromises()
    expect(wrapper.get('#automation-collection-name').element).toHaveProperty('value', '  甲子读书 | 读懂一本书  ')
    await button('自动制作并准备发布').trigger('click'); await flushPromises()
    expect(invoke).toHaveBeenCalledWith('start_automation', { input: expect.objectContaining({ collectionName: '甲子读书 | 读懂一本书' }) })
    expect(vi.mocked(invoke).mock.calls.filter(call => call[0] === 'confirm_automation_account')).toHaveLength(1)
    wrapper.unmount()
    const reopened = mount(AutomationSetup, { props: { saveSource } }); await flushPromises()
    expect(reopened.text()).toContain('账号已确认，后续步骤自动沿用')
    expect(reopened.findAll('button').find(button => button.text() === '确认使用此账号')).toBeUndefined()
    expect(vi.mocked(invoke).mock.calls.filter(call => call[0] === 'confirm_automation_account')).toHaveLength(1)
    reopened.unmount()
  })
  it('lets the user confirm the nickname at the start when the upload page cannot display it', async () => {
    const account = { id: null, name: '', key: 'identity' }
    vi.mocked(invoke).mockImplementation(async (command, args) => command === 'prepare_automation_account' ? account : command === 'confirm_automation_account' ? (args as { expected: typeof account }).expected : undefined)
    const wrapper = mount(AutomationSetup, { props: { saveSource: vi.fn() } }); await flushPromises()
    const button = (label: string) => wrapper.findAll('button').find(button => button.text() === label)!
    await button('打开抖音并读取账号').trigger('click'); await flushPromises()
    expect(button('确认使用此账号').attributes('disabled')).toBeDefined()
    await wrapper.get('#automation-account-name').setValue('已核对的昵称')
    await button('确认使用此账号').trigger('click'); await flushPromises()
    expect(automationAccount.value).toEqual({ ...account, name: '已核对的昵称' })
    expect(invoke).toHaveBeenCalledWith('confirm_automation_account', { expected: { ...account, name: '已核对的昵称' } })
    wrapper.unmount()
  })
  it('clears stale account confirmation on a failed reread without discarding the collection', async () => {
    automationConsent.value = true
    automationAccount.value = { id: null, name: '原账号', key: 'old' }
    const wrapper = mount(AutomationSetup, { props: { saveSource: vi.fn() } }); await flushPromises()
    wrapper.findAllComponents({ name: 'UiSelect' }).at(-1)!.vm.$emit('update:modelValue', 'named'); await flushPromises()
    await wrapper.get('#automation-collection-name').setValue('本次合集')
    vi.mocked(invoke).mockRejectedValue('请先登录')
    await wrapper.findAll('button').find(button => button.text() === '打开抖音并读取账号')!.trigger('click'); await flushPromises()
    expect(automationAccount.value).toBeNull()
    expect(wrapper.get('#automation-collection-name').element).toHaveProperty('value', '本次合集')
    expect(wrapper.findAll('button').find(button => button.text() === '自动制作并准备发布')!.attributes('disabled')).toBeDefined()
    wrapper.unmount()
  })
  it('recovers a failed account confirmation by rereading the account while retaining the chosen collection', async () => {
    automationConsent.value = true
    let observed = { id: null, name: '原账号', key: 'old' }
    let confirmationFails = true
    vi.mocked(invoke).mockImplementation(async (command, args) => {
      if (command === 'prepare_automation_account') return { ...observed }
      if (command === 'confirm_automation_account') {
        if (confirmationFails) throw '账号已变化，请重新读取并核对。'
        return (args as { expected: typeof observed }).expected
      }
      if (command === 'automation_snapshot') return []
      return run()
    })
    const saveSource = vi.fn().mockResolvedValue({ projectId: 'book1', sourceAssetId: 'source1' })
    const wrapper = mount(AutomationSetup, { props: { saveSource } }); await flushPromises()
    const button = (label: string) => wrapper.findAll('button').find(button => button.text() === label)!
    wrapper.findAllComponents({ name: 'UiSelect' }).at(-1)!.vm.$emit('update:modelValue', 'named'); await flushPromises()
    await wrapper.get('#automation-collection-name').setValue('本次合集')
    await button('打开抖音并读取账号').trigger('click'); await flushPromises()
    await button('确认使用此账号').trigger('click'); await flushPromises()
    expect(wrapper.get('[role="alert"]').text()).toContain('账号已变化')
    expect(automationAccount.value).toBeNull()
    expect(button('自动制作并准备发布').attributes('disabled')).toBeDefined()
    expect(saveSource).not.toHaveBeenCalled()
    observed = { id: null, name: '新账号', key: 'new' }; confirmationFails = false
    await button('打开抖音并读取账号').trigger('click'); await flushPromises()
    await button('确认使用此账号').trigger('click'); await flushPromises()
    expect(automationAccount.value).toEqual(observed)
    expect(wrapper.get('#automation-collection-name').element).toHaveProperty('value', '本次合集')
    await button('自动制作并准备发布').trigger('click'); await flushPromises()
    expect(invoke).toHaveBeenCalledWith('start_automation', { input: expect.objectContaining({ collectionName: '本次合集' }) })
    wrapper.unmount()
  })
  it('restores a paused task account and collection without treating the saved nickname as a new confirmation', async () => {
    const active = run({ status: 'paused' })
    active.input.account = { id: null, name: '已保存账号', key: 'saved' }
    active.input.selection.collectionName = '已保存合集'
    automationRuns.value = [active]
    const wrapper = mount(AutomationSetup, { props: { saveSource: vi.fn() } }); await flushPromises()
    expect(wrapper.get('#automation-account-name').element).toHaveProperty('value', '已保存账号')
    expect(wrapper.get('#automation-collection-name').element).toHaveProperty('value', '已保存合集')
    expect(wrapper.get('#automation-collection').attributes('disabled')).toBeDefined()
    expect(wrapper.text()).toContain('已保存本次任务的账号与合集')
    expect(wrapper.text()).not.toContain('账号已确认，后续步骤自动沿用')
    expect(automationAccount.value).toBeNull()
    expect(invoke).not.toHaveBeenCalled()
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
