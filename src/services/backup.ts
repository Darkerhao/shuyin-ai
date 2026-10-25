import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
export interface BackupSummary { manifestPath: string; fileCount: number; totalBytes: number; omittedVideos: number }
export interface RestoreResult { previousDatabase: string; workspacePath: string }
export const createFullBackup = (directory: string, includeVideos: boolean) => invoke<BackupSummary>('create_full_backup', { directory, includeVideos })
export const inspectFullBackup = (path: string) => invoke<BackupSummary>('inspect_full_backup', { path })
export const restoreFullBackup = (path: string, directory: string) => invoke<RestoreResult>('restore_full_backup', { path, directory })
export const chooseFullBackup = () => open({ title: '选择完整备份目录中的 manifest.json', multiple: false, filters: [{ name: '书引 AI 完整备份清单', extensions: ['json'] }] })
