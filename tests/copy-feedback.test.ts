import { afterEach, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { defineComponent } from 'vue'
import { useCopyFeedback } from '../src/composables/useCopyFeedback'
import CopyButton from '../src/components/CopyButton.vue'
const mounted: ReturnType<typeof mount>[] = []
afterEach(() => { mounted.splice(0).forEach(wrapper => wrapper.unmount()); vi.useRealTimers(); vi.unstubAllGlobals() })
function render() {
  const wrapper = mount(defineComponent({
    components: { CopyButton },
    setup: () => ({ clipboard: useCopyFeedback() }),
    template: '<CopyButton label="复制测试内容" :copied="clipboard.copied.value === \'text\'" @click="clipboard.copy(\'第一行\\n第二行\', \'text\')" /><p role="status">{{ clipboard.error.value }}</p>',
  }))
  mounted.push(wrapper); return wrapper
}
it('copies exact text, keeps the action name and resets successful feedback after 800ms', async () => {
  vi.useFakeTimers()
  const writeText = vi.fn().mockResolvedValue(undefined)
  vi.stubGlobal('navigator', { clipboard: { writeText } })
  const wrapper = render()
  await wrapper.get('[aria-label="复制测试内容"]').trigger('click'); await flushPromises()
  expect(writeText).toHaveBeenCalledWith('第一行\n第二行')
  expect(wrapper.getComponent(CopyButton).props('copied')).toBe(true)
  await vi.advanceTimersByTimeAsync(799)
  expect(wrapper.getComponent(CopyButton).props('copied')).toBe(true)
  await vi.advanceTimersByTimeAsync(1)
  expect(wrapper.getComponent(CopyButton).props('copied')).toBe(false)
})
it('reports rejection and clears old success without changing text or claiming it was copied', async () => {
  const writeText = vi.fn().mockRejectedValue(new Error('denied'))
  vi.stubGlobal('navigator', { clipboard: { writeText } })
  const wrapper = render()
  await wrapper.get('button').trigger('click'); await flushPromises()
  expect(wrapper.get('[role="status"]').text()).toContain('复制失败')
  expect(wrapper.getComponent(CopyButton).props('copied')).toBe(false)
})
it('restarts the feedback timer on another copy and cleans it up on unmount', async () => {
  vi.useFakeTimers()
  vi.stubGlobal('navigator', { clipboard: { writeText: vi.fn().mockResolvedValue(undefined) } })
  const wrapper = render()
  await wrapper.get('button').trigger('click'); await flushPromises()
  await vi.advanceTimersByTimeAsync(700)
  await wrapper.get('button').trigger('click'); await flushPromises()
  await vi.advanceTimersByTimeAsync(700)
  expect(wrapper.getComponent(CopyButton).props('copied')).toBe(true)
  wrapper.unmount()
  expect(vi.getTimerCount()).toBe(0)
})
