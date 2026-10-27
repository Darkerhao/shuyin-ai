import { onBeforeUnmount, onMounted, ref, watch, type Ref } from 'vue'
import { isTauri } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { saveDraft, type DraftKind } from '../services/drafts'
import { errorMessage } from '../services/verification'

// One queue per mounted editor prevents slow writes from overtaking newer edits or explicit saves.
export function useEditorDraft<T>(projectId: () => string, kind: DraftKind, content: Ref<T>, basedOn: () => string | null, dirty: Ref<boolean>, ready: Ref<boolean>, canClose: () => boolean | Promise<boolean>) {
  const status = ref('')
  const error = ref('')
  let timer: ReturnType<typeof globalThis.setTimeout> | undefined
  let queue = Promise.resolve()
  let disposed = false
  let unlisten: (() => void) | undefined
  let editRevision = 0
  function flush() {
    globalThis.clearTimeout(timer)
    const id = projectId()
    const revision = editRevision
    const draft = dirty.value ? { basedOn: basedOn(), content: JSON.parse(JSON.stringify(content.value)) as T } : null
    status.value = '正在暂存…'
    queue = queue.catch(() => {}).then(() => saveDraft(id, kind, draft))
    return queue.then(() => { if (revision === editRevision) { status.value = draft ? '草稿已自动暂存' : ''; error.value = '' } }, cause => { if (revision === editRevision) { error.value = `自动暂存失败：${errorMessage(cause)}`; status.value = '' } throw cause })
  }
  watch([content, dirty], () => {
    if (!ready.value) return
    editRevision++
    status.value = dirty.value ? '等待暂存…' : ''
    globalThis.clearTimeout(timer)
    timer = globalThis.setTimeout(() => { void flush().catch(() => {}) }, 400)
  }, { deep: true })
  onMounted(async () => {
    if (!isTauri()) return
    try {
      const stop = await getCurrentWindow().onCloseRequested(async event => {
        if (!await canClose()) { event.preventDefault(); return }
        try { if (ready.value) await flush() } catch { event.preventDefault() }
      })
      if (disposed) stop(); else unlisten = stop
    } catch (cause) { error.value = `退出暂存监听失败：${errorMessage(cause)}` }
  })
  onBeforeUnmount(() => { disposed = true; globalThis.clearTimeout(timer); unlisten?.() })
  return { status, error, flush }
}
