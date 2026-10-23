import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import type { CoverConfig, CoverVersion } from '../types/cover'
import type { Asset } from '../types/project'

export const startCoverGeneration = (projectId: string, providerId: string, description: string) => invoke<string>('start_cover_generation', { input: { projectId, providerId, description } })
export const executeCoverGeneration = (id: string) => invoke<Asset>('execute_cover_generation', { id })

export const listCovers = (projectId: string) => invoke<CoverVersion[]>('list_cover_versions', { projectId })
export const saveCover = (projectId: string, config: CoverConfig, png: number[] | null = null) => invoke<CoverVersion>('save_cover_version', { projectId, config, png })
export const readBackground = (projectId: string, assetId: string) => invoke<ArrayBuffer>('read_cover_background', { projectId, assetId })
export const chooseBackground = () => open({ title: '选择封面背景图', multiple: false, filters: [{ name: '图片', extensions: ['png', 'jpg', 'jpeg', 'webp'] }] })
