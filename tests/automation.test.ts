import { beforeEach, describe, expect, it, vi } from 'vitest'
import { DOMWrapper, flushPromises, mount } from '@vue/test-utils'
import { invoke } from '@tauri-apps/api/core'
import AutomationSetup from '../src/components/AutomationSetup.vue'
import { automationRuns, automationConsent, automationAccount, renderAutomationCover, type AutomationRun } from '../src/services/automation'
import * as automation from '../src/services/automation'
import { readBackground } from '../src/services/covers'
import { coverPng, decodeBackground } from '../src/services/coverRenderer'
import { newCover } from '../src/types/cover'
import { listProviders } from '../src/services/ai'
import { douyinCollectionOptions, saveDouyinCollectionOptions } from '../src/services/publishing'
import { chooseSelect, mockResizeObserver } from './ui'

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }))
vi.mock('../src/services/projects', () => ({ isDesktop: () => true }))
vi.mock('../src/services/ai', () => ({ listProviders: vi.fn() }))
vi.mock('../src/services/publishing', () => ({ douyinCollectionOptions: vi.fn(), saveDouyinCollectionOptions: vi.fn() }))
vi.mock('../src/services/covers', () => ({ readBackground: vi.fn() }))
vi.mock('../src/services/coverRenderer', () => ({ coverPng: vi.fn(), decodeBackground: vi.fn() }))

const run = (overrides: Partial<AutomationRun> = {}): AutomationRun => ({
  id: 'run1', projectId: 'book1', input: { workflowVersion: 2, title: '原书', author: '作者', selection: { projectId: 'book1', source: { kind: 'asset', assetId: 'source1' }, textProviderId: 'text1', imageProviderId: 'image1', format: 'deep_dive', requestKey: 'request1' } },
  status: 'running', stage: 'landscape_cover', runId: 2, error: '', audioJobId: 'audio1', videoJobId: 'video1', writingTaskId: 'writing1', writingVersionId: 'version1', finalWritingVersionId: null, actualDurationSeconds: null, widescreenTaskId: null, widescreenCoverId: null, durationTaskId: null, contentCheckWarnings: [], contentCheckTaskId: null, landscapeTaskId: 'background1', portraitTaskId: null, landscapeCoverId: null, portraitCoverId: null, publishTaskId: null, createdAt: '', updatedAt: '', ...overrides,
})
beforeEach(() => {
  mockResizeObserver()
  vi.resetAllMocks(); automationRuns.value = []; automationConsent.value = false; automationAccount.value = null
  vi.spyOn(automation, 'getAutomationAccountReuse').mockResolvedValue(true)
  vi.mocked(douyinCollectionOptions).mockResolvedValue([])
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
  it('waits for account reuse settings to finish saving before starting a new automatic task', async () => {
    automationConsent.value = true
    automationAccount.value = { id: null, name: '已确认账号', key: 'identity' }
    let finish!: () => void
    vi.spyOn(automation, 'setAutomationAccountReuse').mockImplementation(() => new Promise(resolve => { finish = resolve }))
    const saveSource = vi.fn()
    const wrapper = mount(AutomationSetup, { props: { saveSource } }); await flushPromises()
    const preference = wrapper.get('label[for] [role="checkbox"]')
    await preference.trigger('click'); await flushPromises()
    const start = () => wrapper.findAll('button').find(button => button.text() === '自动制作并准备发布')!
    expect(start().attributes('disabled')).toBeDefined()
    expect(wrapper.emitted('busy')?.at(-1)).toEqual([true])
    await start().trigger('click'); await flushPromises()
    expect(saveSource).not.toHaveBeenCalled()
    finish(); await flushPromises()
    expect(start().attributes('disabled')).toBeUndefined()
    expect(wrapper.emitted('busy')?.at(-1)).toEqual([false])
    wrapper.unmount()
  })

  it('saves collections with Enter and supports choosing, renaming, deleting and reopening without starting work', async () => {
    let saved = ['甲子读书 | 读懂一本书', '三国风云录']
    vi.mocked(douyinCollectionOptions).mockImplementation(async () => [...saved])
    vi.mocked(saveDouyinCollectionOptions).mockImplementation(async values => {
      saved = [...new Set(values.map(value => value.trim()))]
      return [...saved]
    })
    const saveSource = vi.fn()
    const wrapper = mount(AutomationSetup, { attachTo: document.body, props: { saveSource } })
    const popup = () => new DOMWrapper(document.querySelector<HTMLElement>('[data-saved-options="automation-collection-name"]')!)
    await flushPromises()
    wrapper.findAllComponents({ name: 'UiSelect' }).at(-1)!.vm.$emit('update:modelValue', 'named'); await flushPromises()
    const input = wrapper.get('#automation-collection-name')
    expect(input.attributes('autocomplete')).toBe('off')
    await wrapper.get('[aria-label="展开合集选项"]').trigger('click'); await flushPromises()
    expect(popup().findAll('[data-option-select]')).toHaveLength(2)
    await popup().get('[aria-label="选择 三国风云录"]').trigger('click'); await flushPromises()
    expect(input.element).toHaveProperty('value', '三国风云录')
    expect(saveDouyinCollectionOptions).not.toHaveBeenCalled()
    await input.setValue('  历史精讲  ')
    await input.trigger('keydown', { key: 'Enter', isComposing: true }); await flushPromises()
    expect(saveDouyinCollectionOptions).not.toHaveBeenCalled()
    await input.trigger('keydown', { key: 'Enter' }); await flushPromises()
    expect(input.element).toHaveProperty('value', '历史精讲')
    expect(saveDouyinCollectionOptions).toHaveBeenCalledWith(['历史精讲', '甲子读书 | 读懂一本书', '三国风云录'])
    await input.trigger('keydown', { key: 'Enter' }); await flushPromises()
    expect(saveDouyinCollectionOptions).toHaveBeenCalledOnce()
    await wrapper.get('[aria-label="展开合集选项"]').trigger('click'); await flushPromises()
    await popup().get('[aria-label="编辑选项 历史精讲"]').trigger('click'); await flushPromises()
    await popup().get('input').setValue(' 历史人物精讲 ')
    await popup().get('input').trigger('keydown', { key: 'Enter' }); await flushPromises()
    expect(input.element).toHaveProperty('value', '历史人物精讲')
    await popup().get('[aria-label="删除选项 历史人物精讲"]').trigger('click'); await flushPromises()
    expect(saved).toEqual(['甲子读书 | 读懂一本书', '三国风云录'])
    expect(input.element).toHaveProperty('value', '历史人物精讲')
    expect(popup().text()).toContain('不会创建、重命名或删除抖音上的合集')
    wrapper.unmount()
    const reopened = mount(AutomationSetup, { attachTo: document.body, props: { saveSource } }); await flushPromises()
    reopened.findAllComponents({ name: 'UiSelect' }).at(-1)!.vm.$emit('update:modelValue', 'named'); await flushPromises()
    await reopened.get('[aria-label="展开合集选项"]').trigger('click'); await flushPromises()
    expect(popup().findAll('[data-option-select]').map(option => option.text())).toEqual(saved)
    expect(saveSource).not.toHaveBeenCalled()
    expect(invoke).not.toHaveBeenCalled()
    reopened.unmount()
  })

  it('retries collection loading and keeps the input and saved options when a write fails', async () => {
    vi.mocked(douyinCollectionOptions).mockRejectedValueOnce('数据库读取失败').mockResolvedValue(['读书'])
    const wrapper = mount(AutomationSetup, { attachTo: document.body, props: { saveSource: vi.fn() } })
    await flushPromises()
    wrapper.findAllComponents({ name: 'UiSelect' }).at(-1)!.vm.$emit('update:modelValue', 'named'); await flushPromises()
    const input = wrapper.get('#automation-collection-name')
    expect(input.attributes('disabled')).toBeDefined()
    expect(wrapper.text()).toContain('合集选项读取失败')
    await wrapper.findAll('button').find(button => button.text() === '重新读取合集选项')!.trigger('click'); await flushPromises()
    expect(input.attributes('disabled')).toBeUndefined()
    vi.mocked(saveDouyinCollectionOptions).mockRejectedValueOnce('保存失败').mockResolvedValue(['新合集', '读书'])
    await input.setValue('新合集'); await input.trigger('keydown', { key: 'Enter' }); await flushPromises()
    expect(input.element).toHaveProperty('value', '新合集')
    expect(wrapper.get('[role="alert"]').text()).toBe('保存失败')
    await wrapper.get('[aria-label="展开合集选项"]').trigger('click'); await flushPromises()
    const popup = new DOMWrapper(document.querySelector<HTMLElement>('[data-saved-options="automation-collection-name"]')!)
    expect(popup.findAll('[data-option-select]')).toHaveLength(1)
    expect(popup.text()).toContain('读书')
    await input.trigger('keydown', { key: 'Enter' }); await flushPromises()
    expect(wrapper.text()).toContain('选项已保存')
    expect(saveDouyinCollectionOptions).toHaveBeenCalledTimes(2)
    wrapper.unmount()
  })

  it('blocks starting or repeating a collection save while persistence is pending', async () => {
    automationConsent.value = true
    automationAccount.value = { id: null, name: '本次账号', key: 'identity' }
    let finish!: (values: string[]) => void
    vi.mocked(saveDouyinCollectionOptions).mockImplementation(() => new Promise(resolve => { finish = resolve }))
    const saveSource = vi.fn()
    const wrapper = mount(AutomationSetup, { attachTo: document.body, props: { saveSource } }); await flushPromises()
    wrapper.findAllComponents({ name: 'UiSelect' }).at(-1)!.vm.$emit('update:modelValue', 'named'); await flushPromises()
    const input = wrapper.get('#automation-collection-name')
    await input.setValue('读书合集'); await input.trigger('keydown', { key: 'Enter' }); await flushPromises()
    expect(input.attributes('disabled')).toBeDefined()
    const start = wrapper.findAll('button').find(button => button.text().includes('正在准备'))!
    expect(start.attributes('disabled')).toBeDefined()
    await start.trigger('click'); await input.trigger('keydown', { key: 'Enter' }); await flushPromises()
    expect(saveSource).not.toHaveBeenCalled()
    expect(saveDouyinCollectionOptions).toHaveBeenCalledOnce()
    finish(['读书合集']); await flushPromises()
    expect(input.attributes('disabled')).toBeUndefined()
    expect(wrapper.findAll('button').find(button => button.text() === '自动制作并准备发布')!.attributes('disabled')).toBeUndefined()
    wrapper.unmount()
  })

  it('starts a chapter with the same image, account and complete publishing setup as a whole book', async () => {
    automationConsent.value = true
    automationAccount.value = { id: null, name: '精讲账号', key: 'chapter-account' }
    vi.mocked(invoke).mockImplementation(async command => command === 'automation_snapshot' ? [] : run())
    const source = { projectId: 'book1', source: { kind: 'chapter' as const, chapterId: 'chapter-one' } }
    const wrapper = mount(AutomationSetup, { props: { saveSource: vi.fn().mockResolvedValue(source) } })
    await flushPromises()
    expect(wrapper.get('#automation-image').text()).toContain('图片')
    expect(wrapper.text()).toContain('三尺寸封面和视频')
    expect(wrapper.text()).toContain('本次发布账号与合集')
    const start = wrapper.findAll('button').find(button => button.text() === '自动制作并准备发布')!
    expect(start.attributes('disabled')).toBeUndefined()
    await start.trigger('click'); await flushPromises()
    expect(invoke).toHaveBeenCalledWith('start_automation', { input: expect.objectContaining({ ...source, imageProviderId: 'image1', textProviderId: 'text1', theme: 'ink', detailedInk: true, collectionName: null }) })
    wrapper.unmount()
  })

  it('passes the selected classic video theme to automatic creation', async () => {
    automationConsent.value = true
    automationAccount.value = { id: null, name: '精讲账号', key: 'classic-account' }
    vi.mocked(invoke).mockImplementation(async command => command === 'automation_snapshot' ? [] : run())
    const source = { projectId: 'book1', source: { kind: 'asset' as const, assetId: 'source1' } }
    const wrapper = mount(AutomationSetup, { props: { saveSource: vi.fn().mockResolvedValue(source) } })
    await flushPromises()
    await chooseSelect(wrapper, '#automation-theme', '书卷手绘')
    const start = wrapper.findAll('button').find(button => button.text() === '自动制作并准备发布')!
    await start.trigger('click'); await flushPromises()
    expect(invoke).toHaveBeenCalledWith('start_automation', { input: expect.objectContaining({ ...source, theme: 'classic', detailedInk: false }) })
  })

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
    const saveSource = vi.fn().mockResolvedValue({ projectId: 'book1', source: { kind: 'asset', assetId: 'source1' } })
    const wrapper = mount(AutomationSetup, { props: { saveSource } })
    await flushPromises()
    const button = (label: string) => wrapper.findAll('button').find(button => button.text() === label)!
    expect(button('自动制作并准备发布').attributes('disabled')).toBeDefined()
    await button('打开抖音并读取账号').trigger('click'); await flushPromises()
    expect(wrapper.get('#automation-account-name').element).toHaveProperty('value', '本次账号')
    await button('确认使用此账号').trigger('click'); await flushPromises()
    expect(button('自动制作并准备发布').attributes('disabled')).toBeDefined()
    await wrapper.get('[aria-label="允许自动制作和上传"]').trigger('click'); await flushPromises()
    expect(button('自动制作并准备发布').attributes('disabled')).toBeUndefined()
    await button('自动制作并准备发布').trigger('click'); await flushPromises()
    expect(saveSource).toHaveBeenCalledTimes(1)
    expect(invoke).toHaveBeenCalledWith('start_automation', { input: expect.objectContaining({ projectId: 'book1', source: { kind: 'asset', assetId: 'source1' }, textProviderId: 'text1', imageProviderId: 'image1', format: 'deep_dive', collectionName: null }) })
    expect(wrapper.emitted('busy')?.at(-1)).toEqual([false])
    wrapper.unmount()
  })
  it('saves the chosen collection at startup and does not ask for the same account after remounting', async () => {
    const account = { id: null, name: '本次账号', key: 'identity' }
    automationConsent.value = true
    vi.mocked(invoke).mockImplementation(async command => ['prepare_automation_account', 'confirm_automation_account'].includes(command) ? account : command === 'automation_snapshot' ? [] : run())
    const saveSource = vi.fn().mockResolvedValue({ projectId: 'book1', source: { kind: 'asset', assetId: 'source1' } })
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
    const saveSource = vi.fn().mockResolvedValue({ projectId: 'book1', source: { kind: 'asset', assetId: 'source1' } })
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
    const saveSource = vi.fn().mockResolvedValue({ projectId: 'book1', source: { kind: 'asset', assetId: 'source1' } })
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
