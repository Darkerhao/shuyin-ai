import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import type { CoverConfig, CoverVersion, CoverPreset } from '../types/cover'
import type { Asset } from '../types/project'

export const startCoverGeneration = (projectId: string, providerId: string, description: string) => invoke<string>('start_cover_generation', { input: { projectId, providerId, description } })
export const executeCoverGeneration = (id: string) => invoke<Asset>('execute_cover_generation', { id })

export const listCovers = (projectId: string) => invoke<CoverVersion[]>('list_cover_versions', { projectId })
export const saveCover = (projectId: string, config: CoverConfig, png: number[] | null = null) => invoke<CoverVersion>('save_cover_version', { projectId, config, png })
export const readBackground = (projectId: string, assetId: string) => invoke<ArrayBuffer>('read_cover_background', { projectId, assetId })
export const readCoverExport = (projectId: string, id: string) => invoke<ArrayBuffer>('read_cover_export', { projectId, id })
export const listPresets = () => invoke<CoverPreset[]>('list_cover_presets')
export const savePreset = (name: string, config: CoverConfig) => invoke<void>('save_cover_preset', { name, config })
export const deletePreset = (id: string) => invoke<void>('delete_cover_preset', { id })
export const chooseBackground = () => open({ title: '选择封面背景图', multiple: false, filters: [{ name: '图片', extensions: ['png', 'jpg', 'jpeg', 'webp'] }] })
