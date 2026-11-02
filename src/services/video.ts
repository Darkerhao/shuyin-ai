import type { Asset } from '../types/project'
import { convertFileSrc, invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import type { VideoJob, VideoScene, VideoSettings } from '../types/video'

export const settings = () => invoke<VideoSettings>('video_settings')
export const saveSettings = (settings: VideoSettings) => invoke<void>('save_video_settings', { settings })
export const listJobs = (projectId: string) => invoke<VideoJob[]>('list_video_jobs', { projectId })
export const startJob = (projectId: string, audioAssetId: string, providerId: string, subtitleAssetId: string | null) => invoke<VideoJob>('start_video_job', { input: { projectId, audioAssetId, providerId, subtitleAssetId } })
export const continueJob = (id: string, render: boolean) => invoke<VideoJob>('continue_video_job', { id, render })
export const previewJob = (id: string, firstScene: number, seconds: number) => invoke<VideoJob>('preview_video_job', { id, request: { firstScene, seconds } })
export const previewSource = async (projectId: string, id: string, runId: number, scene?: number) => convertFileSrc(await invoke<string>('video_preview_media', { projectId, id, runId, scene: scene ?? null }))
export const cancelJob = (id: string) => invoke<void>('cancel_video_job', { id })
export const savePlan = (id: string, subtitles: string, scenes: VideoScene[]) => invoke<void>('save_video_plan', { id, subtitles, scenes })
export const subtitles = (id: string) => invoke<string>('video_subtitles', { id })
export const exportSubtitles = (id: string) => invoke<string>('export_video_subtitles', { id })
export const mediaSource = async (projectId: string, assetId: string) => convertFileSrc(await invoke<string>('media_asset_path', { projectId, assetId }))
export const chooseRuntimePath = (directory: boolean) => open({ title: directory ? '选择独立引擎或本地模型目录' : '选择 Node / Python 可执行文件', directory, multiple: false, ...(directory ? {} : { filters: [{ name: 'Windows 程序', extensions: ['exe'] }] }) })

export const chooseImage = () => open({ title: '导入本书视频图片', multiple: false, filters: [{ name: '静态图片', extensions: ['png', 'jpg', 'jpeg', 'webp'] }] })
export const importImage = (projectId: string, path: string) => invoke<string>('import_video_image', { projectId, path })
export const readImage = (projectId: string, assetId: string) => invoke<ArrayBuffer>('read_video_image', { projectId, assetId })
export const startIllustration = (projectId: string, providerId: string, description: string) => invoke<string>('start_video_illustration', { input: { projectId, providerId, description, width: 1920, height: 1080 } })
export const executeIllustration = (id: string) => invoke<Asset>('execute_video_illustration', { id })
