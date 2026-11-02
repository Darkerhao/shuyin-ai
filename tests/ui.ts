import { DOMWrapper, flushPromises, type VueWrapper } from '@vue/test-utils'
import { expect } from 'vitest'

export async function openSelect(wrapper: VueWrapper, selector: string) {
  const trigger = wrapper.find(selector).exists() ? wrapper.get(selector) : dialog().get(selector)
  ;(trigger.element as HTMLElement).focus()
  await trigger.trigger('keydown', { key: 'ArrowDown' })
  await flushPromises()
  expect(trigger.attributes('aria-expanded')).toBe('true')
  return new DOMWrapper(document.querySelector('[role="listbox"]')!)
}

export async function chooseSelect(wrapper: VueWrapper, selector: string, label: string | RegExp) {
  const listbox = await openSelect(wrapper, selector)
  const option = listbox.findAll('[role="option"]').find(item => typeof label === 'string' ? item.text() === label : label.test(item.text()))
  expect(option, `Missing option: ${label}`).toBeDefined()
  await option!.trigger('keydown', { key: 'Enter' })
  await flushPromises()
  const trigger = wrapper.find(selector).exists() ? wrapper.get(selector) : dialog().get(selector)
  expect(trigger.attributes('aria-expanded')).toBe('false')
}

export function dialog() {
  return new DOMWrapper(document.querySelector('[role="alertdialog"]')!)
}

export function button(wrapper: VueWrapper, label: string) {
  const modal = document.querySelector('[role="alertdialog"]')
  const buttons = modal ? new DOMWrapper(modal).findAll('button') : []
  return [...buttons, ...wrapper.findAll('button')].find(item => item.text() === label)!
}

export async function answerConfirmation(accepted: boolean) {
  await flushPromises()
  const label = accepted ? '确认继续' : '取消'
  await dialog().findAll('button').find(item => item.text() === label)!.trigger('click')
  await flushPromises()
}

export function tab(wrapper: VueWrapper, label: string) {
  return wrapper.findAll('[role="tab"]').find(item => item.text() === label)!
}

export function tabPanel(wrapper: VueWrapper, label: string) {
  return wrapper.get(`[id="${tab(wrapper, label).attributes('aria-controls')}"]`)
}
