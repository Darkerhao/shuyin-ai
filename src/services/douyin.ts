import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

export interface DouyinEditor {
  title: string
  description: string
  videoPreview: boolean
  coverDialog: boolean
  covers: { ratio: 'landscape' | 'portrait'; selected: boolean; preview: string }[]
}

export interface DouyinPage {
  supported: boolean
  open: boolean
  path: string
  loginVisible: boolean
  online: boolean
  editor?: DouyinEditor | null
}

export const openDouyinWindow = () => invoke<void>('open_douyin_window')
export const inspectDouyinWindow = () => invoke<DouyinPage>('inspect_douyin_window')
export const uploadDouyinVideo = (id: string, accountConfirmed: boolean) => invoke<void>('upload_douyin_video', { id, accountConfirmed })
export const completeDouyinTask = (id: string, existingVideoConfirmed: boolean) => invoke<DouyinEditor>('complete_douyin_task', { id, existingVideoConfirmed })
export const stopDouyinTask = () => invoke<void>('stop_douyin_task')
export const onDouyinProgress = (callback: (progress: { taskId: string; message: string }) => void) => listen<{ taskId: string; message: string }>('douyin-progress', event => callback(event.payload))
