import { Channel, invoke } from '@tauri-apps/api/core'
import type { GenerationTextPreview, GenerationInput, GenerationResult, ModelProvider, ProviderInput, SaveWritingInput, WritingKind, WritingVersion } from '../types/ai'
export const listProviders = () => invoke<ModelProvider[]>('list_model_providers')
export type ModelOptionField = 'baseUrl' | 'modelName'
export const modelFieldOptions = (field: ModelOptionField) => invoke<string[]>('model_field_options', { field })
export const saveModelFieldOptions = (field: ModelOptionField, values: string[]) => invoke<string[]>('save_model_field_options', { field, values })
export const saveProvider = (id: string | null, input: ProviderInput, apiKey: string | null) => invoke<{ provider: ModelProvider; warning: string | null }>('save_model_provider', { id, input, apiKey })
export const deleteProvider = (id: string) => invoke<void>('delete_model_provider', { id })
export const testConnection = (id: string) => invoke<string>('test_model_connection', { id })
export const listVersions = (projectId: string, kind: WritingKind, chapterId: string | null = null) => invoke<WritingVersion[]>('list_writing_versions', { projectId, kind, ...(chapterId ? { chapterId } : {}) })
export const saveWriting = (input: SaveWritingInput) => invoke<WritingVersion>('save_writing_version', { input })
export const startGeneration = (input: GenerationInput) => invoke<string>('start_generation', { input })
export async function executeGeneration(id: string, onPreview?: (content: GenerationTextPreview) => void) {
  const channel = onPreview ? new Channel<GenerationTextPreview>(onPreview) : undefined
  try { return await invoke<GenerationResult>('execute_generation', { id, onPreview: channel ?? null }) }
  finally { if (channel) channel.onmessage = () => {} }
}
export const applyGeneration = (id: string) => invoke<WritingVersion>('apply_generation', { id })
export const cancelGeneration = (id: string) => invoke<void>('cancel_generation', { id })
