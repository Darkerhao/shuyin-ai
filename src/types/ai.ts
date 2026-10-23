export interface ProviderInput { name: string; baseUrl: string; modelName: string; isDefault: boolean; isEnabled: boolean }
export interface ModelProvider extends ProviderInput { id: string; credentialStatus: 'available' | 'missing' | 'unavailable' }
export const emptyProvider = (): ProviderInput => ({ name: '', baseUrl: 'https://api.openai.com/v1', modelName: '', isDefault: false, isEnabled: true })
export type WritingKind = 'publishing'
export interface PublishingContent { format: 'publishing'; title: string; description: string; audioPrompt: string }
export const emptyPublishing = (): PublishingContent => ({ format: 'publishing', title: '', description: '', audioPrompt: '' })
export type WritingContent = PublishingContent
export interface WritingVersion { id: string; projectId: string; kind: WritingKind; versionNo: number; content: WritingContent; sourceType: 'ai' | 'manual' | 'mixed'; modelName: string; promptVersion: string; isConfirmed: boolean; createdAt: string }
export interface SaveWritingInput { projectId: string; kind: WritingKind; content: WritingContent; basedOn: string | null; isConfirmed: boolean }
export interface GenerationInput { projectId: string; providerId: string }
