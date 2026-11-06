import { onBeforeUnmount, onMounted, ref } from 'vue'
import { isTauri } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { errorMessage } from '../services/verification'

const guards = new Set<() => boolean | Promise<boolean>>()
let registration: Promise<void> | undefined
let unlisten: (() => void) | undefined
let closing = false
function ensureListener() {
  if (registration || unlisten) return registration
  registration = getCurrentWindow().onCloseRequested(async event => {
    if (closing) { event.preventDefault(); return }
    closing = true
    try {
      for (const guard of [...guards]) {
        if (!await guard()) { event.preventDefault(); return }
      }
    } catch { event.preventDefault() }
    finally { closing = false }
  }).then(stop => { if (guards.size) unlisten = stop; else stop() }).finally(() => { registration = undefined })
  return registration
}

export function useWindowCloseGuard(protectedState: () => boolean, canClose: () => boolean | Promise<boolean>) {
  const error = ref('')
  let disposed = false
  async function guard() {
    try { return await canClose() }
    catch (cause) { error.value = errorMessage(cause); return false }
  }
  function beforeUnload(event: globalThis.BeforeUnloadEvent) {
    if (protectedState()) { event.preventDefault(); event.returnValue = '' }
  }
  globalThis.addEventListener('beforeunload', beforeUnload)
  onMounted(async () => {
    if (!isTauri()) return
    guards.add(guard)
    try {
      await ensureListener()
    } catch (cause) { error.value = `窗口退出保护未启用：${errorMessage(cause)}` }
    if (disposed) guards.delete(guard)
  })
  onBeforeUnmount(() => {
    disposed = true; guards.delete(guard)
    if (!guards.size) { unlisten?.(); unlisten = undefined }
    globalThis.removeEventListener('beforeunload', beforeUnload)
  })
  return { error }
}
