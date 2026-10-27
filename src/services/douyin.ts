import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type { DouyinCollection } from '../types/publishing'

export interface DouyinSession {
  runId: number
  taskId: string
  status: 'running' | 'waiting' | 'review' | 'stopped' | 'failed'
  step: 'connect' | 'video' | 'text' | 'covers' | 'collection' | 'review'
  message: string
  waitingFor: 'account' | 'video' | 'processing' | 'page' | 'text' | 'covers' | 'collection' | null
  account: { id: string | null; name: string; key: string } | null
  completed: string[]
  manual: string[]
  collection: DouyinCollection | null
  options: { id: string | null; name: string }[]
}
export type DouyinDecision = { kind: 'account'; name: string } | { kind: 'existingVideo' | 'processing' | 'page' | 'text' | 'covers' } | { kind: 'collection'; name: string | null; remember: boolean; manual: boolean }
export const openDouyinWindow = () => invoke<void>('open_douyin_window')
export const getDouyinSession = () => invoke<DouyinSession | null>('douyin_session')
export const startDouyinTask = (id: string) => invoke<DouyinSession>('start_douyin_task', { id })
export const continueDouyinTask = (runId: number, decision: DouyinDecision) => invoke<DouyinSession>('continue_douyin_task', { runId, decision })
export const stopDouyinTask = (runId: number) => invoke<DouyinSession>('stop_douyin_task', { runId })
export const onDouyinProgress = (callback: (session: DouyinSession) => void) => listen<DouyinSession>('douyin-progress', event => callback(event.payload))
