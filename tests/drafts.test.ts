import { afterEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { computed, defineComponent, ref } from 'vue'
import { useEditorDraft } from '../src/composables/useEditorDraft'
import { saveDraft } from '../src/services/drafts'
import { isTauri } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
vi.mock('../src/services/drafts', () => ({ saveDraft: vi.fn() }))
vi.mock('@tauri-apps/api/core', () => ({ isTauri: vi.fn() }))
vi.mock('@tauri-apps/api/window', () => ({ getCurrentWindow: vi.fn() }))
afterEach(() => { vi.useRealTimers(); vi.resetAllMocks() })
describe('serialized automatic drafts', () => {
  it.each([false, true])('skips unchanged initialization and still saves edits with retained workspace = %s', async retainSavedDraft => {
    vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] })
    vi.mocked(saveDraft).mockResolvedValue(undefined)
    const harness = {
      setup() {
        const content = ref({ text: '', activeSize: 0 })
        const saved = ref('')
        const ready = ref(false)
        useEditorDraft(() => 'book', 'cover', content, () => 'v1', computed(() => content.value.text !== saved.value), ready, () => true, retainSavedDraft)
        function load() {
          content.value = { text: '已保存', activeSize: 0 }
          saved.value = content.value.text
          ready.value = true
        }
        return { content, load }
      },
      template: '<button aria-label="载入" @click="load">载入</button><textarea v-model="content.text" /><button aria-label="切换尺寸" @click="content.activeSize = 1">切换尺寸</button>',
    }
    const wrapper = mount(harness)
    try {
      await wrapper.get('[aria-label="载入"]').trigger('click')
      await vi.advanceTimersByTimeAsync(400)
      expect(saveDraft).not.toHaveBeenCalled()
      await wrapper.get('textarea').setValue('未保存的修改')
      await vi.advanceTimersByTimeAsync(399)
      expect(saveDraft).not.toHaveBeenCalled()
      await vi.advanceTimersByTimeAsync(1)
      expect(saveDraft).toHaveBeenCalledExactlyOnceWith('book', 'cover', { basedOn: 'v1', content: { text: '未保存的修改', activeSize: 0 } })
      await wrapper.get('textarea').setValue('已保存')
      await vi.advanceTimersByTimeAsync(400)
      expect(saveDraft).toHaveBeenLastCalledWith('book', 'cover', retainSavedDraft ? { basedOn: 'v1', content: { text: '已保存', activeSize: 0 } } : null)
      if (retainSavedDraft) {
        await wrapper.get('[aria-label="切换尺寸"]').trigger('click')
        await vi.advanceTimersByTimeAsync(400)
        expect(saveDraft).toHaveBeenLastCalledWith('book', 'cover', { basedOn: 'v1', content: { text: '已保存', activeSize: 1 } })
      }
    } finally { wrapper.unmount() }
  })
  it('autosaves unsaved content when asynchronous initialization becomes ready', async () => {
    vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] })
    vi.mocked(saveDraft).mockResolvedValue(undefined)
    const harness = {
      setup() {
        const content = ref({ text: '' })
        const ready = ref(false)
        useEditorDraft(() => 'book', 'writing', content, () => null, computed(() => !!content.value.text), ready, () => true)
        return { content, ready }
      },
      template: '<textarea v-model="content.text" /><button @click="ready = true">载入完成</button>',
    }
    const wrapper = mount(harness)
    try {
      await wrapper.get('textarea').setValue('尚未保存的带入内容')
      await vi.advanceTimersByTimeAsync(400)
      expect(saveDraft).not.toHaveBeenCalled()
      await wrapper.get('button').trigger('click')
      await vi.advanceTimersByTimeAsync(400)
      expect(saveDraft).toHaveBeenCalledExactlyOnceWith('book', 'writing', { basedOn: null, content: { text: '尚未保存的带入内容' } })
    } finally { wrapper.unmount() }
  })
  it('waits for asynchronous close preparation and blocks closing if it fails', async () => {
    vi.mocked(isTauri).mockReturnValue(true)
    let close!: (event: { preventDefault: () => void }) => Promise<void>
    vi.mocked(getCurrentWindow).mockReturnValue({ onCloseRequested: vi.fn(async callback => { close = callback as typeof close; return () => {} }) } as unknown as ReturnType<typeof getCurrentWindow>)
    let finish!: (allowed: boolean) => void
    const prepare = vi.fn(() => new Promise<boolean>(resolve => { finish = resolve }))
    const harness = {
      setup() {
        useEditorDraft(() => 'book', 'writing', ref({ text: '内容' }), () => null, ref(false), ref(true), prepare)
      },
      template: '<p>关闭前保存时长</p>',
    }
    const wrapper = mount(harness)
    try {
      await flushPromises()
      const event = { preventDefault: vi.fn() }
      const pending = close(event)
      expect(saveDraft).not.toHaveBeenCalled()
      finish(false); await pending
      expect(event.preventDefault).toHaveBeenCalledOnce()
      expect(saveDraft).not.toHaveBeenCalled()
      const allowed = close(event)
      finish(true); await allowed
      expect(saveDraft).toHaveBeenCalledWith('book', 'writing', null)
    } finally { wrapper.unmount() }
  })
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
