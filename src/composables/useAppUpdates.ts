import { computed, readonly, ref, shallowRef } from 'vue'
import type { Update } from '@tauri-apps/plugin-updater'
import * as api from '../services/updates'
import { errorMessage } from '../services/verification'

// Shared by the app banner and About page; navigation never starts a second download.
const version = ref('')
const automatic = ref(true)
const initialized = ref(false)
const saving = ref(false)
const status = ref<'idle' | 'checking' | 'current' | 'available' | 'downloading' | 'ready' | 'installing' | 'restart'>('idle')
const update = shallowRef<Update | null>(null)
const error = ref('')
const settingsError = ref('')
const checkedAt = ref('')
const downloaded = ref(0)
const total = ref<number>()
const busy = computed(() => ['checking', 'downloading', 'installing'].includes(status.value))
const progress = computed(() => total.value ? Math.min(100, Math.round(downloaded.value / total.value * 100)) : undefined)
let initialization: Promise<void> | undefined

async function initialize() {
  if (!api.isDesktop()) return
  initialization ??= (async () => {
    settingsError.value = ''
    try {
      version.value = await api.appVersion()
      automatic.value = await api.autoUpdateEnabled()
      initialized.value = true
      if (automatic.value) await checkForUpdates(true)
    } catch (cause) {
      settingsError.value = `读取更新设置失败：${errorMessage(cause)}`
      initialization = undefined
    }
  })()
  return initialization
}

async function checkForUpdates(background = false) {
  if (!api.isDesktop() || busy.value || status.value === 'ready' || status.value === 'restart') return
  status.value = 'checking'
  error.value = ''
  try {
    await update.value?.close()
    update.value = null
    update.value = await api.checkUpdate()
    checkedAt.value = new Date().toLocaleString('zh-CN')
    status.value = update.value ? 'available' : 'current'
    // Respect a preference disabled while the request was in flight.
    if (background && automatic.value && update.value) await downloadUpdate()
  } catch (cause) {
    status.value = 'idle'
    error.value = `检查更新失败：${errorMessage(cause)}`
  }
}

async function downloadUpdate() {
  if (!update.value || status.value !== 'available') return
  status.value = 'downloading'
  error.value = ''
  downloaded.value = 0
  total.value = undefined
  try {
    await update.value.download(event => {
      if (event.event === 'Started') total.value = event.data.contentLength
      if (event.event === 'Progress') downloaded.value += event.data.chunkLength
    }, { timeout: 120_000 })
    status.value = 'ready'
  } catch (cause) {
    status.value = 'available'
    error.value = `下载或签名校验失败：${errorMessage(cause)}`
  }
}

async function installUpdate() {
  if (!update.value || (status.value !== 'ready' && status.value !== 'restart')) return
  const installed = status.value === 'restart'
  status.value = 'installing'
  error.value = ''
  try {
    if (!installed) await update.value.install()
  } catch (cause) {
    status.value = 'ready'
    error.value = `安装失败：${errorMessage(cause)}`
    return
  }
  // Windows exits inside install(); macOS/Linux need an explicit relaunch.
  status.value = 'restart'
  try { await api.restartApp() } catch (cause) { error.value = `更新已安装，重启失败：${errorMessage(cause)}。请关闭后重新打开应用。` }
}

async function setAutomatic(enabled: boolean) {
  if (saving.value || !initialized.value) return
  saving.value = true
  settingsError.value = ''
  try {
    await api.setAutoUpdate(enabled)
    automatic.value = enabled
  } catch (cause) {
    settingsError.value = `保存更新设置失败：${errorMessage(cause)}`
    return
  }
  finally { saving.value = false }
  if (enabled) {
    if (status.value === 'available') await downloadUpdate()
    else await checkForUpdates(true)
  }
}

export function useAppUpdates() {
  return {
    version: readonly(version), automatic: readonly(automatic), initialized: readonly(initialized), saving: readonly(saving),
    status: readonly(status), update: readonly(update), error: readonly(error), settingsError: readonly(settingsError),
    checkedAt: readonly(checkedAt), downloaded: readonly(downloaded), total: readonly(total), progress, busy,
    initialize, checkForUpdates, downloadUpdate, installUpdate, setAutomatic,
  }
}
