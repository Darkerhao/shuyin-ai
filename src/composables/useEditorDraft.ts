import { computed, onBeforeUnmount, ref, watch, type Ref } from 'vue'
import { useWindowCloseGuard } from './useWindowCloseGuard'
import { saveDraft, type DraftKind } from '../services/drafts'
import { errorMessage } from '../services/verification'

// One queue per mounted editor prevents slow writes from overtaking newer edits or explicit saves.
export function useEditorDraft<T>(projectId: () => string, kind: DraftKind, content: Ref<T>, basedOn: () => string | null, dirty: Ref<boolean>, ready: Ref<boolean>, canClose: () => boolean | Promise<boolean>, retainSavedDraft = false) {
  const status = ref('')
  const error = ref('')
  let timer: ReturnType<typeof globalThis.setTimeout> | undefined
  let queue = Promise.resolve()
  let editRevision = 0
  function flush() {
    globalThis.clearTimeout(timer)
    if (!ready.value) return queue
    const id = projectId()
    const revision = editRevision
    const draft = dirty.value || retainSavedDraft ? { basedOn: basedOn(), content: JSON.parse(JSON.stringify(content.value)) as T } : null
    status.value = '正在暂存…'
    queue = queue.catch(() => {}).then(() => saveDraft(id, kind, draft))
    return queue.then(() => { if (revision === editRevision) { status.value = draft ? '草稿已自动暂存' : ''; error.value = '' } }, cause => { if (revision === editRevision) { error.value = `自动暂存失败：${errorMessage(cause)}`; status.value = '' } throw cause })
  }
  function reset() {
    globalThis.clearTimeout(timer)
    queue = queue.catch(() => {})
    status.value = ''; error.value = ''
  }
  watch([content, dirty, ready], (_, [, , wasReady]) => {
    // Vue batches loading and readiness changes; unchanged loaded data is not an edit.
    if (!ready.value || (!wasReady && !dirty.value)) { globalThis.clearTimeout(timer); return }
    editRevision++
    status.value = dirty.value ? '等待暂存…' : ''
    globalThis.clearTimeout(timer)
    timer = globalThis.setTimeout(() => { void flush().catch(() => {}) }, 400)
  }, { deep: true })
  const closeGuard = useWindowCloseGuard(() => ready.value && dirty.value, async () => {
    if (!await canClose()) return false
    try { if (ready.value) await flush(); return true } catch { return false }
  })
  onBeforeUnmount(() => globalThis.clearTimeout(timer))
  return { status, error: computed(() => error.value || closeGuard.error.value), flush, reset }
}
