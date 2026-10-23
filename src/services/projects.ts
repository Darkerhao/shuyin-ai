import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import type { Asset, Project, ProjectInput, WorkspaceSettings } from '../types/project'
export { isDesktop, errorMessage } from './verification'
export const listProjects = (search = '', status = '') => invoke<Project[]>('list_projects', { search, status })
export const getProject = (id: string) => invoke<Project>('get_project', { id })
export const saveProject = (id: string | null, input: ProjectInput) => invoke<Project>('save_project', { id, input })
export const deleteProject = (id: string) => invoke<void>('delete_project', { id })
export const listAssets = (projectId: string) => invoke<Asset[]>('list_assets', { projectId })
export const linkAsset = (projectId: string, path: string, assetId: string | null = null) => invoke<string>('link_asset', { projectId, path, assetId })
export const unlinkAsset = (id: string) => invoke<void>('unlink_asset', { id })
export const openAsset = (id: string, directory = false) => invoke<void>('open_asset', { id, directory })
export const workspaceSettings = () => invoke<WorkspaceSettings>('workspace_settings')
export const setWorkspaceRoot = (path: string) => invoke<WorkspaceSettings>('set_workspace_root', { path })
export const openProjectDirectory = (id: string) => invoke<void>('open_project_directory', { id })
export const relocateWorkspace = (id: string, path: string) => invoke<Project>('relocate_workspace', { id, path })
export const createBackup = (directory: string) => invoke<string>('create_backup', { directory })
export const restoreBackup = (path: string) => invoke<string>('restore_backup', { path })
export const chooseDirectory = () => open({ title: '选择目录', directory: true, multiple: false })
export const chooseBackup = () => open({ title: '选择书引 AI 数据库备份', multiple: false, filters: [{ name: 'SQLite 备份', extensions: ['sqlite3'] }] })
export const chooseAsset = () => open({ title: '关联本地素材', multiple: false, filters: [{ name: '视频、图片、字幕和文档', extensions: ['mp4', 'mov', 'mkv', 'avi', 'webm', 'm4v', 'png', 'jpg', 'jpeg', 'webp', 'srt', 'ass', 'vtt', 'txt', 'md', 'pdf', 'docx', 'epub'] }] })
