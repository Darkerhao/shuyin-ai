import { afterEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { computed, defineComponent, ref } from 'vue'
import { useEditorDraft } from '../src/composables/useEditorDraft'
import { saveDraft } from '../src/services/drafts'
vi.mock('../src/services/drafts', () => ({ saveDraft: vi.fn() }))
afterEach(() => { vi.useRealTimers(); vi.resetAllMocks() })
describe('serialized automatic drafts', () => {
  it('does not mark newer edits saved when a slow earlier write completes', async () => {
    vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] })
    let finish!: () => void
    vi.mocked(saveDraft).mockReturnValueOnce(new Promise(resolve => { finish = resolve })).mockResolvedValue(undefined)
    const harness = defineComponent({
      setup() {
        const content = ref({ text: '' })
        const autosave = useEditorDraft(() => 'book', 'writing', content, () => null, computed(() => !!content.value.text), ref(true), () => true)
        return { content, status: autosave.status }
      },
      template: '<textarea v-model="content.text" /><p>{{ status }}</p>',
    })
    const wrapper = mount(harness)
    try {
      await wrapper.get('textarea').setValue('第一段')
      await vi.advanceTimersByTimeAsync(400)
      expect(saveDraft).toHaveBeenCalledWith('book', 'writing', { basedOn: null, content: { text: '第一段' } })
      await wrapper.get('textarea').setValue('新输入仍待暂存')
      finish(); await flushPromises()
      expect(wrapper.get('p').text()).toBe('等待暂存…')
      await vi.advanceTimersByTimeAsync(400); await flushPromises()
      expect(saveDraft).toHaveBeenLastCalledWith('book', 'writing', { basedOn: null, content: { text: '新输入仍待暂存' } })
      expect(wrapper.get('p').text()).toBe('草稿已自动暂存')
    } finally { wrapper.unmount() }
  })
})
