export interface ProviderInput { name: string; baseUrl: string; modelName: string; isDefault: boolean; isEnabled: boolean }
export interface ModelProvider extends ProviderInput { id: string; credentialStatus: 'available' | 'missing' | 'unavailable' }
export const emptyProvider = (): ProviderInput => ({ name: '', baseUrl: 'https://api.openai.com/v1', modelName: '', isDefault: false, isEnabled: true })
export const writingKinds = { outline: '讲解大纲', script: '讲稿', titles: '标题候选', descriptions: '发布描述', hashtags: '话题标签' } as const
export type WritingKind = keyof typeof writingKinds
export type WritingContent = { format: 'markdown'; text: string } | { format: 'list'; items: string[] }
export interface WritingVersion { id: string; projectId: string; kind: WritingKind; versionNo: number; content: WritingContent; sourceType: 'ai' | 'manual' | 'mixed'; modelName: string; promptVersion: string; isConfirmed: boolean; createdAt: string }
export interface SaveWritingInput { projectId: string; kind: WritingKind; content: WritingContent; basedOn: string | null; isConfirmed: boolean }
export interface GenerationInput { projectId: string; providerId: string; kind: WritingKind; count: number; referenceVersionId: string | null }
export interface AiTask { id: string; kind: WritingKind; status: 'pending' | 'running' | 'succeeded' | 'failed' | 'cancelled'; modelName: string; errorCode: string | null; errorMessage: string | null; versionId: string | null; createdAt: string }
export const taskStatuses: Record<AiTask['status'], string> = { pending: '等待执行', running: '生成中', succeeded: '已保存', failed: '失败', cancelled: '已取消' }
