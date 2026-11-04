import { ref } from 'vue'
import { isTauri } from '@tauri-apps/api/core'
import { interfacePreferences, setSidebarPrompt } from '../services/interface'
const sidebarPrompt = ref(true)
const error = ref('')
const busy = ref(false)
let loading: Promise<void> | undefined
async function initialize() {
  if (!isTauri()) return
  loading ??= interfacePreferences().then(value => { sidebarPrompt.value = value.sidebarPrompt }, () => { error.value = '界面偏好读取失败，可重新打开应用重试。' })
  await loading
}
async function setPrompt(visible: boolean) {
  if (busy.value) return
  busy.value = true
  try { await setSidebarPrompt(visible); sidebarPrompt.value = visible; error.value = '' }
  catch { error.value = '界面偏好未保存，请重试。' }
  finally { busy.value = false }
}
export const useInterfacePreferences = () => ({ sidebarPrompt, error, busy, initialize, setPrompt })
