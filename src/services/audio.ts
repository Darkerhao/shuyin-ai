import { invoke } from '@tauri-apps/api/core'
import type { AudioJobAction, AudioJobView, StartAudioJobInput } from '../types/audio'

export const getAudioJobView = (projectId: string, id?: string, chapterId: string | null = null) => invoke<AudioJobView>('audio_job_snapshot', { projectId, ...(id ? { id } : {}), ...(chapterId ? { chapterId } : {}) })
export const startAudioJob = (input: StartAudioJobInput) => invoke<AudioJobView>('start_audio_job', { input })
export const controlAudioJob = (id: string, action: AudioJobAction, notebookUrl?: string) => invoke<AudioJobView>('control_audio_job', { id, action, ...(notebookUrl ? { notebookUrl } : {}) })
