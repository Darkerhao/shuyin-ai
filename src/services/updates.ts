import { getVersion } from '@tauri-apps/api/app'
import { invoke, isTauri } from '@tauri-apps/api/core'
import { check } from '@tauri-apps/plugin-updater'
import { relaunch } from '@tauri-apps/plugin-process'

export const isDesktop = () => isTauri()
export const appVersion = () => getVersion()
export const autoUpdateEnabled = () => invoke<boolean>('update_settings')
export const setAutoUpdate = (enabled: boolean) => invoke<void>('set_auto_update', { enabled })
export const checkUpdate = () => check({ timeout: 20_000 })
export const restartApp = () => relaunch()
export const currentReleaseNotes = import.meta.env.VITE_RELEASE_NOTES as string | undefined
