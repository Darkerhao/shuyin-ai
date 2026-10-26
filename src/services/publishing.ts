import { invoke } from '@tauri-apps/api/core'
import type { PublishInput, PublishQueueItem, PublishRecord, PublishTask } from '../types/publishing'

export const listPublishTasks = (projectId: string) => invoke<PublishTask[]>('list_publish_tasks', { projectId })
export const listPublishQueue = () => invoke<PublishQueueItem[]>('list_publish_queue')
export const savePublishTask = (id: string | null, input: PublishInput) => invoke<PublishTask>('save_publish_task', { id, input })
export const preparePublishTask = (id: string) => invoke<PublishTask>('prepare_publish_task', { id })
export const recordPublishSuccess = (id: string, input: PublishRecord & { confirmed: boolean }) => invoke<PublishTask>('record_publish_success', { id, input })
export const openPublishItem = (id: string, item: 'directory' | 'video' | 'landscape' | 'portrait' | 'portal') => invoke<void>('open_publish_item', { id, item })
