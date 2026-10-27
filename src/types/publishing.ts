export interface PublishInput {
  writingVersionId: string | null
  projectId: string
  title: string
  description: string
  tags: string
  videoAssetId: string | null
  audioAssetId: string | null
  landscapeCoverId: string | null
  portraitCoverId: string | null
  titleConfirmed: boolean
  coversConfirmed: boolean
}
export interface PublishRecord { publishedAt: string; postUrl: string; notes: string }
export interface DouyinCollection { accountId: string | null; accountName: string; id: string | null; name: string | null }
export interface PublishTask extends PublishInput {
  id: string
  status: 'draft' | 'ready' | 'published'
  taskDirectory: string | null
  createdAt: string
  updatedAt: string
  checklist: { key: string; label: string; passed: boolean }[]
  record: PublishRecord | null
  douyinCollection?: DouyinCollection | null
}
export const publishStatuses = { draft: '草稿', ready: '待发布', published: '已发布' } as const
export interface PublishQueueItem extends PublishTask { projectTitle: string }
export function publishInput(task: PublishInput): PublishInput {
  const { projectId, title, description, tags, videoAssetId, audioAssetId, landscapeCoverId, portraitCoverId, titleConfirmed, coversConfirmed } = task
  return { projectId, title, description, tags, videoAssetId, audioAssetId, landscapeCoverId, portraitCoverId, titleConfirmed, coversConfirmed, writingVersionId: task.writingVersionId ?? null }
}
