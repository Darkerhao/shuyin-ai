import { onBeforeUnmount, onMounted, ref } from 'vue'
import { isTauri } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { errorMessage } from '../services/verification'

export function useWindowCloseGuard(protectedState: () => boolean, canClose: () => boolean | Promise<boolean>) {
  const error = ref('')
  let disposed = false
  let unlisten: (() => void) | undefined
  function beforeUnload(event: globalThis.BeforeUnloadEvent) {
    if (protectedState()) { event.preventDefault(); event.returnValue = '' }
  }
  globalThis.addEventListener('beforeunload', beforeUnload)
  onMounted(async () => {
    if (!isTauri()) return
    try {
      const stop = await getCurrentWindow().onCloseRequested(async event => {
        try { if (!await canClose()) event.preventDefault() }
        catch (cause) { event.preventDefault(); error.value = errorMessage(cause) }
      })
      if (disposed) stop(); else unlisten = stop
    } catch (cause) { error.value = `窗口退出保护未启用：${errorMessage(cause)}` }
  })
  onBeforeUnmount(() => { disposed = true; unlisten?.(); globalThis.removeEventListener('beforeunload', beforeUnload) })
  return { error }
}
