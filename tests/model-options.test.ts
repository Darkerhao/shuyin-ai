import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount, DOMWrapper } from '@vue/test-utils'
import ModelOptionInput from '../src/modules/settings/ModelOptionInput.vue'
import { mockResizeObserver } from './ui'

const wrappers: ReturnType<typeof mount>[] = []
function popup() { return new DOMWrapper(document.querySelector<HTMLElement>('[data-model-options="test-model"]')!) }
async function render(options = ['model-a', 'model-b']) {
  const saveOptions = vi.fn(async (values: string[]) => [...new Set(values.map(value => value.trim()))])
  const wrapper = mount(ModelOptionInput, { attachTo: document.body, props: {
    id: 'test-model', label: '模型名称', modelValue: 'model-a', options, disabled: false, placeholder: '输入模型', saveOptions,
    'onUpdate:modelValue': (value: string) => { void wrapper.setProps({ modelValue: value }) },
  } })
  saveOptions.mockImplementation(async values => {
    const saved = [...new Set(values.map(value => value.trim()))]
    await wrapper.setProps({ options: saved })
    return saved
  })
  wrappers.push(wrapper)
  await flushPromises()
  return { wrapper, saveOptions, input: wrapper.get('input') }
}
beforeEach(mockResizeObserver)
afterEach(() => { wrappers.splice(0).forEach(wrapper => wrapper.unmount()); vi.restoreAllMocks() })

describe('saved model field options', () => {
  it('shows all saved choices, filters typed text and chooses without saving the model', async () => {
    const { wrapper, saveOptions, input } = await render()
    await wrapper.get('button').trigger('click'); await flushPromises()
    expect(popup().findAll('[data-option-select]')).toHaveLength(2)
    expect(input.attributes('aria-controls')).toBe('test-model-options')
    await input.setValue('model-b'); await flushPromises()
    expect(popup().findAll('[data-option-select]')).toHaveLength(1)
    await popup().get('[data-option-select]').trigger('click'); await flushPromises()
    expect(input.element).toHaveProperty('value', 'model-b')
    expect(input.attributes('aria-expanded')).toBe('false')
    expect(saveOptions).not.toHaveBeenCalled()
  })
  it('saves trimmed input on Enter, ignores IME confirmation, and does not duplicate existing or blank values', async () => {
    const { input, saveOptions, wrapper } = await render()
    await input.setValue('  model-new  ')
    await input.trigger('keydown', { key: 'Enter', isComposing: true }); await flushPromises()
    expect(saveOptions).not.toHaveBeenCalled()
    await input.trigger('keydown', { key: 'Enter' }); await flushPromises()
    expect(saveOptions).toHaveBeenCalledWith(['model-new', 'model-a', 'model-b'])
    expect(input.element).toHaveProperty('value', 'model-new')
    expect(wrapper.text()).toContain('选项已保存')
    await input.trigger('keydown', { key: 'Enter' }); await input.setValue('   ')
    await input.trigger('keydown', { key: 'Enter' }); await flushPromises()
    expect(saveOptions).toHaveBeenCalledOnce()
  })
  it('edits selected entries, merges duplicates and deletes without clearing the current input', async () => {
    const { wrapper, input, saveOptions } = await render()
    await wrapper.get('button').trigger('click'); await flushPromises()
    await popup().get('[aria-label="编辑选项 model-a"]').trigger('click'); await flushPromises()
    await popup().get('input').setValue(' model-b ')
    await popup().get('input').trigger('keydown', { key: 'Enter' }); await flushPromises()
    expect(saveOptions).toHaveBeenLastCalledWith([' model-b ', 'model-b'])
    expect(input.element).toHaveProperty('value', 'model-b')
    expect(popup().findAll('[data-option-select]')).toHaveLength(1)
    await popup().get('[aria-label="删除选项 model-b"]').trigger('click'); await flushPromises()
    expect(saveOptions).toHaveBeenLastCalledWith([])
    expect(input.element).toHaveProperty('value', 'model-b')
    expect(popup().text()).toContain('暂无选项')
  })
  it('preserves typed and edited values on storage failure and permits retry', async () => {
    const { wrapper, input, saveOptions } = await render()
    saveOptions.mockRejectedValueOnce('保存失败')
    await input.setValue('model-new'); await input.trigger('keydown', { key: 'Enter' }); await flushPromises()
    expect(input.element).toHaveProperty('value', 'model-new')
    expect(wrapper.get('[role="alert"]').text()).toBe('保存失败')
    expect(wrapper.props('options')).toEqual(['model-a', 'model-b'])
    await input.trigger('keydown', { key: 'Enter' }); await flushPromises()
    await wrapper.get('button').trigger('click'); await flushPromises()
    await popup().get('[aria-label="编辑选项 model-a"]').trigger('click'); await flushPromises()
    await popup().get('input').setValue('renamed')
    saveOptions.mockRejectedValueOnce('数据库锁定')
    await popup().get('input').trigger('keydown', { key: 'Enter' }); await flushPromises()
    expect(popup().get('input').element).toHaveProperty('value', 'renamed')
    expect(wrapper.props('options')).toContain('model-a')
    await popup().get('[aria-label="取消选项修改"]').trigger('click'); await flushPromises()
    await wrapper.get('button').trigger('click'); await flushPromises()
    saveOptions.mockRejectedValueOnce('删除失败')
    await popup().get('[aria-label="删除选项 model-a"]').trigger('click'); await flushPromises()
    expect(wrapper.props('options')).toContain('model-a')
  })
  it('supports keyboard entry, Arrow navigation, Escape and focused selection', async () => {
    const { wrapper, input } = await render()
    await input.trigger('keydown', { key: 'ArrowDown' }); await flushPromises()
    const choices = popup().findAll('[data-option-select]')
    expect(document.activeElement).toBe(choices[0]!.element)
    await choices[0]!.trigger('keydown', { key: 'ArrowDown' }); await flushPromises()
    expect(document.activeElement).toBe(choices[1]!.element)
    await choices[1]!.trigger('keydown', { key: 'Escape' }); await flushPromises()
    expect(input.attributes('aria-expanded')).toBe('false')
    expect(document.activeElement).toBe(input.element)
    await input.setValue('model-b'); await flushPromises()
    await input.trigger('keydown', { key: 'ArrowDown' }); await flushPromises()
    const filtered = popup().findAll('[data-option-select]')
    expect(filtered).toHaveLength(1)
    expect(filtered[0]!.attributes('aria-label')).toBe('选择 model-b')
    expect(document.activeElement).toBe(filtered[0]!.element)
    await filtered[0]!.trigger('keydown', { key: 'Escape' }); await flushPromises()
    await wrapper.get('button').trigger('click'); await flushPromises()
    await popup().get('[aria-label="编辑选项 model-a"]').trigger('click'); await flushPromises()
    await popup().get('input').setValue('not-saved')
    await popup().get('input').trigger('keydown', { key: 'Escape' }); await flushPromises()
    expect(input.attributes('aria-expanded')).toBe('false')
    expect(document.activeElement).toBe(input.element)
    expect(wrapper.props('options')).toEqual(['model-a', 'model-b'])
  })
  it('blocks repeat saves while pending and disables controls in read-only preview', async () => {
    const { wrapper, input, saveOptions } = await render()
    let finish!: (values: string[]) => void
    saveOptions.mockImplementationOnce(() => new Promise(resolve => { finish = resolve }))
    await input.setValue('model-new'); await input.trigger('keydown', { key: 'Enter' }); await flushPromises()
    await input.trigger('keydown', { key: 'Enter' })
    expect(saveOptions).toHaveBeenCalledOnce()
    expect(input.attributes('disabled')).toBeDefined()
    finish(['model-new']); await flushPromises()
    await wrapper.setProps({ disabled: true })
    await wrapper.get('button').trigger('click'); await input.trigger('keydown', { key: 'Enter' })
    expect(saveOptions).toHaveBeenCalledOnce()
    expect(input.attributes('aria-expanded')).toBe('false')
  })
})
