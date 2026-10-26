import { invoke } from '@tauri-apps/api/core'
import type { GenerationInput, GenerationResult, ModelProvider, ProviderInput, SaveWritingInput, WritingKind, WritingVersion } from '../types/ai'
export const listProviders = () => invoke<ModelProvider[]>('list_model_providers')
export const openAudioTool = () => invoke<void>('open_audio_tool')
export const saveProvider = (id: string | null, input: ProviderInput, apiKey: string | null) => invoke<{ provider: ModelProvider; warning: string | null }>('save_model_provider', { id, input, apiKey })
export const deleteProvider = (id: string) => invoke<void>('delete_model_provider', { id })
export const testConnection = (id: string) => invoke<string>('test_model_connection', { id })
export const listVersions = (projectId: string, kind: WritingKind) => invoke<WritingVersion[]>('list_writing_versions', { projectId, kind })
export const saveWriting = (input: SaveWritingInput) => invoke<WritingVersion>('save_writing_version', { input })
export const startGeneration = (input: GenerationInput) => invoke<string>('start_generation', { input })
export const executeGeneration = (id: string) => invoke<GenerationResult>('execute_generation', { id })
export const applyGeneration = (id: string) => invoke<WritingVersion>('apply_generation', { id })
export const cancelGeneration = (id: string) => invoke<void>('cancel_generation', { id })
