import { invoke, isTauri } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'

export interface ProbeRecord {
  note: string
  updatedAt: string
}

export interface ProbeSnapshot {
  databasePath: string
  record: ProbeRecord | null
}

export const isDesktop = () => isTauri()

export const readProbe = () => invoke<ProbeSnapshot>('read_probe')

export const saveProbe = (note: string) => invoke<ProbeSnapshot>('save_probe', { note })

export async function selectVideo(): Promise<string | null> {
  return open({
    title: '选择用于 M0 验证的本地视频',
    multiple: false,
    directory: false,
    filters: [{ name: '视频', extensions: ['mp4', 'mov', 'mkv', 'avi', 'webm', 'm4v'] }],
  })
}

export function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error)
}
