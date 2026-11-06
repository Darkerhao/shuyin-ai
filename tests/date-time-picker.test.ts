import { afterEach, describe, expect, it, vi } from 'vitest'
import { defineComponent, ref } from 'vue'
import { DOMWrapper, flushPromises, mount } from '@vue/test-utils'
import { DateTimePicker } from '../src/components/ui/date-time-picker'

const wrappers: ReturnType<typeof mount>[] = []
function render(initial = '2099-01-05T18:30', disabled = false) {
  const submit = vi.fn()
  const wrapper = mount(defineComponent({
    components: { DateTimePicker },
    setup: () => ({ value: ref(initial), disabled, submit }),
    template: '<form @submit.prevent="submit"><label for="test-date">发布时间</label><DateTimePicker id="test-date" v-model="value" :disabled="disabled" /><output>{{ value }}</output></form>',
  }), { attachTo: document.body })
  wrappers.push(wrapper)
  return { wrapper, submit }
}
const panel = () => new DOMWrapper(document.querySelector<HTMLElement>('.date-time-panel')!)
async function selectTime(part: string, text: string) {
  await panel().get(`#test-date-${part}`).trigger('keydown', { key: 'ArrowDown' }); await flushPromises()
  const option = [...document.querySelectorAll('[role="option"]')].find(node => node.textContent?.trim() === text)!
  await new DOMWrapper(option).trigger('keydown', { key: 'Enter' }); await flushPromises()
}
afterEach(() => wrappers.splice(0).forEach(wrapper => wrapper.unmount()))

describe('shared UI date and time picker', () => {
  it('uses the calendar and hour/minute components, preserves time on day changes and never submits the parent form', async () => {
    const { wrapper, submit } = render()
    expect(wrapper.find('input[type="datetime-local"]').exists()).toBe(false)
    await wrapper.get('#test-date').trigger('click'); await flushPromises()
    await panel().get('[data-value^="2099-01-06"]:not([data-outside-view])').trigger('click'); await flushPromises()
    expect(wrapper.get('output').text()).toBe('2099-01-06T18:30')
    await selectTime('hour', '20')
    await selectTime('minute', '45')
    expect(wrapper.get('output').text()).toBe('2099-01-06T20:45')
    await panel().findAll('button').find(button => button.text() === '确定')!.trigger('click'); await flushPromises()
    expect(document.querySelector('.date-time-panel')).toBeNull()
    expect(wrapper.get('#test-date').text()).toContain('2099-01-06 20:45')
    expect(submit).not.toHaveBeenCalled()
  })
  it('clears the value through the popup without invoking a native date input', async () => {
    const { wrapper } = render()
    await wrapper.get('#test-date').trigger('click'); await flushPromises()
    await panel().findAll('button').find(button => button.text() === '清空')!.trigger('click'); await flushPromises()
    expect(wrapper.get('output').text()).toBe('')
    expect(wrapper.get('#test-date').text()).toContain('请选择日期和时间')
    expect(document.querySelector('.date-time-panel')).toBeNull()
  })
  it('keeps a disabled saved value unchanged and the popup closed', async () => {
    const { wrapper } = render('2099-01-05T18:30', true)
    expect(wrapper.get('#test-date').element.matches(':disabled')).toBe(true)
    await wrapper.get('#test-date').trigger('click'); await flushPromises()
    expect(document.querySelector('.date-time-panel')).toBeNull()
    expect(wrapper.get('output').text()).toBe('2099-01-05T18:30')
  })
})
