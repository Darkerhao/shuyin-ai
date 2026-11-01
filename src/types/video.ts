export interface VideoSettings { engineRoot: string; nodePath: string; pythonPath: string; modelPath: string }
export interface VideoCue { start: number; end: number; text: string }
export interface VideoScene { first: number; last: number; kind: 'book' | 'quote' | 'compare' | 'flow' | 'timeline'; title: string; labels: string[] }
export interface VideoPlan { title: string; audioSha256: string; duration: number; cues: VideoCue[]; scenes: VideoScene[] }
export interface VideoProgress {
  startedAt: number; stage: string; phase: string; completed: number; total: number
  cueCount: number; preview: VideoCue[]
}
export interface VideoJob {
  id: string; projectId: string; audioAssetId: string | null; sourceSubtitleId: string | null; providerId: string | null
  status: 'draft' | 'running' | 'awaiting_confirmation' | 'succeeded' | 'failed' | 'cancelled'
  stage: string; runId: number; plan: VideoPlan; error: string
  subtitleAssetId: string | null; videoAssetId: string | null; createdAt: string; updatedAt: string
  progress?: VideoProgress | null
}
export const videoStatuses: Record<VideoJob['status'], string> = { draft: '待开始', running: '制作中', awaiting_confirmation: '等待确认分镜', succeeded: '成片已保存', failed: '处理失败', cancelled: '已停止' }
export const videoStages: Record<string, string> = { preparing: '检查音频与运行环境', transcribing: '识别原声字幕', storyboard: '生成动画分镜', rendering: '绘制画面与合入原声', validating: '校验成片并归档', complete: '制作完成' }
