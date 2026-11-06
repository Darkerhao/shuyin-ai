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
  allowDownload: boolean
  scheduledAt: string | null
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
  return { projectId, title, description, tags, videoAssetId, audioAssetId, landscapeCoverId, portraitCoverId, titleConfirmed, coversConfirmed, allowDownload: task.allowDownload, scheduledAt: task.scheduledAt, writingVersionId: task.writingVersionId ?? null }
}

export function publishScheduleError(scheduledAt: string | null, now = Date.now()): string {
  if (scheduledAt === null) return ''
  const time = Date.parse(scheduledAt)
  if (!/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:00\+08:00$/.test(scheduledAt) || !Number.isFinite(time)
    || new Date(time + 8 * 3600000).toISOString().slice(0, 16) !== scheduledAt.slice(0, 16)) return '请选择有效的定时发布时间。'
  return time <= now ? '定时发布时间已过，请重新选择未来时间。' : ''
}
