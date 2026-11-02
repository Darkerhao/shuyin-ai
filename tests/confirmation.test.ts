import { afterEach, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import AppConfirmation from '../src/components/AppConfirmation.vue'
import ConfirmAction from '../src/components/ConfirmAction.vue'
import { requestConfirmation, settleConfirmation } from '../src/composables/useConfirmation'
import { answerConfirmation, dialog } from './ui'

const wrappers: ReturnType<typeof mount>[] = []
afterEach(async () => {
  wrappers.splice(0).forEach(wrapper => wrapper.unmount())
  settleConfirmation(false)
  await flushPromises()
})

it('waits for an explicit answer and keeps concurrent requests independent', async () => {
  wrappers.push(mount(AppConfirmation, { attachTo: document.body }))
  const pending = requestConfirmation('替换当前封面？')
  const answered = vi.fn()
  void pending.then(answered)
  await flushPromises()
  expect(answered).not.toHaveBeenCalled()
  expect(dialog().text()).toContain('替换当前封面？')
  expect(document.getElementById(dialog().attributes('aria-labelledby')!)?.textContent).toContain('请确认操作')
  expect(await requestConfirmation('放弃其他修改？')).toBe(false)
  expect(dialog().text()).toContain('替换当前封面？')
  await answerConfirmation(true)
  expect(await pending).toBe(true)
  expect(document.querySelector('[role="alertdialog"]')).toBeNull()
})

it('focuses cancel, cancels on Escape and restores focus to the opener', async () => {
  const opener = document.createElement('button')
  document.body.append(opener)
  try {
    wrappers.push(mount(AppConfirmation, { attachTo: document.body }))
    opener.focus()
    const pending = requestConfirmation('放弃未保存的修改？')
    await flushPromises()
    await vi.waitFor(() => expect(document.activeElement).toBe(dialog().get('[data-cancel]').element))
    await dialog().get('[data-cancel]').trigger('keydown', { key: 'Escape' })
    expect(await pending).toBe(false)
    await vi.waitFor(() => expect(document.activeElement).toBe(opener))
  } finally { opener.remove() }
})

it('prevents Escape and repeated actions while a confirmed operation is running', async () => {
  const wrapper = mount(ConfirmAction, { attachTo: document.body, props: { title: '确认恢复？', busy: true } })
  wrappers.push(wrapper)
  await flushPromises()
  expect(dialog().attributes('aria-busy')).toBe('true')
  expect(dialog().findAll('button').every(button => button.attributes('disabled') !== undefined)).toBe(true)
  await dialog().trigger('keydown', { key: 'Escape' })
  expect(wrapper.emitted('cancel')).toBeUndefined()
  await wrapper.setProps({ busy: false })
  await dialog().trigger('keydown', { key: 'Escape' })
  expect(wrapper.emitted('cancel')).toHaveLength(1)
})

it('settles a pending request as cancelled when the application unmounts', async () => {
  const wrapper = mount(AppConfirmation, { attachTo: document.body })
  const pending = requestConfirmation('离开当前页面？')
  await flushPromises()
  wrapper.unmount()
  expect(await pending).toBe(false)
})
