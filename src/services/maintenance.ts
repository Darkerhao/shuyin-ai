import { invoke } from '@tauri-apps/api/core'
export interface BackupRecord { kind: string; path: string; completedAt: string }
export interface StorageUsage { projectId: string; title: string; bytes: number; reclaimableBytes: number; files: number; blocked: boolean; error: string }
export interface CachePreview { projectId: string; token: string; bytes: number; files: string[] }
export interface ModelCall { id: string; projectId: string | null; title: string | null; taskId: string | null; model: string; stage: string; status: string; startedAt: string; elapsedMs: number | null; firstResponseMs: number | null; inputTokens: number | null; outputTokens: number | null; totalTokens: number | null; errorCode: string | null }
export const lastBackup = () => invoke<BackupRecord | null>('last_backup')
export const storageUsage = () => invoke<StorageUsage[]>('storage_usage')
export const previewCleanup = (projectId: string) => invoke<CachePreview>('preview_cache_cleanup', { projectId })
export const cleanCache = (preview: CachePreview) => invoke<number>('clean_video_cache', { projectId: preview.projectId, token: preview.token, confirmed: true })
export const modelCalls = () => invoke<ModelCall[]>('list_model_calls')
export const bytesLabel = (bytes: number) => bytes < 1024 * 1024 ? `${(bytes / 1024).toFixed(1)} KB` : `${(bytes / 1024 / 1024).toFixed(1)} MB`
export const backupDue = (record: BackupRecord | null, now = Date.now()) => !record || now - Date.parse(record.completedAt) >= 7 * 86400000
