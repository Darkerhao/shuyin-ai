import { invoke } from '@tauri-apps/api/core'
import type { AudioJobAction, AudioJobView, StartAudioJobInput } from '../types/audio'

export const getAudioJobView = (projectId: string) => invoke<AudioJobView>('audio_job_snapshot', { projectId })
export const startAudioJob = (input: StartAudioJobInput) => invoke<AudioJobView>('start_audio_job', { input })
export const controlAudioJob = (id: string, action: AudioJobAction) => invoke<AudioJobView>('control_audio_job', { id, action })
