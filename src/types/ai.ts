export interface ProviderInput { modelKind: 'text' | 'image'; name: string; baseUrl: string; modelName: string; isDefault: boolean; isEnabled: boolean }
export interface ModelProvider extends ProviderInput { id: string; credentialStatus: 'available' | 'missing' | 'unavailable' }
export const emptyProvider = (): ProviderInput => ({ modelKind: 'text', name: '', baseUrl: 'https://api.openai.com/v1', modelName: '', isDefault: false, isEnabled: true })
export type WritingKind = 'publishing'
export interface PublishingContent {
  format: 'publishing'
  title: string
  description: string
  audioPrompt: string
  coverTitle: string
  coverSubtitle: string
  coverPromptLandscape: string
  coverPromptPortrait: string
}
export const emptyPublishing = (): PublishingContent => ({ format: 'publishing', title: '', description: '', audioPrompt: '', coverTitle: '', coverSubtitle: '', coverPromptLandscape: '', coverPromptPortrait: '' })
export type WritingContent = PublishingContent
export interface WritingVersion { id: string; projectId: string; kind: WritingKind; versionNo: number; content: WritingContent; sourceType: 'ai' | 'manual' | 'mixed'; modelName: string; promptVersion: string; isConfirmed: boolean; createdAt: string }
export interface SaveWritingInput { projectId: string; kind: WritingKind; content: WritingContent; basedOn: string | null; isConfirmed: boolean }
export type PublishingField = keyof Omit<PublishingContent, 'format'>
export interface RevisionInput { basedOn: string; fields: PublishingField[]; instruction: string }
export interface GenerationInput { projectId: string; providerId: string; revision?: RevisionInput }
export interface GenerationPreview { taskId: string; fields: PublishingField[]; before: PublishingContent; content: PublishingContent }
export type GenerationResult = WritingVersion | GenerationPreview
