import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import AutomationAccountPreference from '../src/components/AutomationAccountPreference.vue'
import { getAutomationAccountReuse, setAutomationAccountReuse } from '../src/services/automation'

vi.mock('../src/services/automation', () => ({ getAutomationAccountReuse: vi.fn(), setAutomationAccountReuse: vi.fn() }))
const wrappers: ReturnType<typeof mount>[] = []
function render() {
  const wrapper = mount(AutomationAccountPreference, { attachTo: document.body })
  wrappers.push(wrapper)
  return wrapper
}
beforeEach(() => { vi.resetAllMocks(); vi.mocked(getAutomationAccountReuse).mockResolvedValue(true) })
afterEach(() => wrappers.splice(0).forEach(wrapper => wrapper.unmount()))

describe('automatic publishing account preference', () => {
  it('loads the saved choice and remembers changes after reopening without starting publishing', async () => {
    let stored = true
    vi.mocked(getAutomationAccountReuse).mockImplementation(async () => stored)
    vi.mocked(setAutomationAccountReuse).mockImplementation(async enabled => { stored = enabled })
    const wrapper = render(); await flushPromises()
    expect(wrapper.get('[role="checkbox"]').attributes('aria-checked')).toBe('true')
    expect(wrapper.text()).toContain('重开窗口或重新登录后')
    expect(wrapper.text()).toContain('最后由你在抖音页面发布')
    await wrapper.get('[role="checkbox"]').trigger('click'); await flushPromises()
    expect(setAutomationAccountReuse).toHaveBeenCalledExactlyOnceWith(false)
    expect(wrapper.get('[role="checkbox"]').attributes('aria-checked')).toBe('false')
    wrapper.unmount()
    const reopened = render(); await flushPromises()
    expect(reopened.get('[role="checkbox"]').attributes('aria-checked')).toBe('false')
    await reopened.get('[role="checkbox"]').trigger('click'); await flushPromises()
    expect(setAutomationAccountReuse).toHaveBeenLastCalledWith(true)
  })

  it('keeps the saved choice and blocks repeat writes while saving, then allows retry after failure', async () => {
    let fail!: (error: Error) => void
    vi.mocked(setAutomationAccountReuse).mockImplementationOnce(() => new Promise((_, reject) => { fail = reject }))
    const wrapper = render(); await flushPromises()
    const checkbox = wrapper.get('[role="checkbox"]')
    await checkbox.trigger('click'); await flushPromises()
    expect(checkbox.attributes('disabled')).toBeDefined()
    expect(checkbox.attributes('aria-checked')).toBe('true')
    expect(wrapper.emitted('busy')?.at(-1)).toEqual([true])
    await checkbox.trigger('click'); await flushPromises()
    expect(setAutomationAccountReuse).toHaveBeenCalledOnce()
    fail(new Error('数据库写入失败')); await flushPromises()
    expect(checkbox.attributes('aria-checked')).toBe('true')
    expect(wrapper.get('[role="alert"]').text()).toContain('数据库写入失败')
    expect(wrapper.emitted('busy')?.at(-1)).toEqual([false])
    await checkbox.trigger('click'); await flushPromises()
    expect(setAutomationAccountReuse).toHaveBeenCalledTimes(2)
    expect(checkbox.attributes('aria-checked')).toBe('false')
  })

  it('provides an accessible retry after a failed read without offering an unverified choice', async () => {
    vi.mocked(getAutomationAccountReuse).mockRejectedValueOnce('数据库读取失败').mockResolvedValue(false)
    const wrapper = render(); await flushPromises()
    const checkbox = wrapper.get('[role="checkbox"]')
    expect(checkbox.attributes('disabled')).toBeDefined()
    expect(wrapper.get('[role="alert"]').text()).toContain('数据库读取失败')
    expect(wrapper.get('label').attributes('for')).toBe(checkbox.attributes('id'))
    expect(checkbox.attributes('aria-describedby')).toBe(wrapper.get('p').attributes('id'))
    await wrapper.findAll('button').find(button => button.text() === '重新读取账号确认设置')!.trigger('click'); await flushPromises()
    expect(checkbox.attributes('disabled')).toBeUndefined()
    expect(checkbox.attributes('aria-checked')).toBe('false')
    expect(setAutomationAccountReuse).not.toHaveBeenCalled()
  })
})
