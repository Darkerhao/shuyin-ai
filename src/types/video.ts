import visualCatalog from '../../src-tauri/video/visual-catalog.json'

export { visualCatalog }
export type VideoTheme = 'classic' | 'ink'
export const videoThemes: Record<VideoTheme, string> = { ink: '铅笔素描', classic: '书卷手绘' }
export interface VideoBeat {
  cue: number
  object: string
  action: keyof typeof visualCatalog.actions
  label: string
  drawing?: { strokes: [number, number][][] }
}
export type PencilPath = number[][]
export interface PencilStroke { p: PencilPath; w: number; a: number }
export interface PencilShade extends PencilStroke { angle: number; gap: number }
export interface PencilText { x: number; y: number; text: string; size: number; a: number }
export interface PencilHead { mask: PencilPath[]; strokes: PencilStroke[]; shades: PencilShade[] }
export interface PencilCharacter { id: string; name: string; description: string; heads: { front: PencilHead; side: PencilHead } }
export interface PencilLayer extends PencilHead {
  cue: number; name: string; texts: PencilText[]
  portrait?: { characterId: string; view: 'front' | 'side'; x: number; y: number; scale: number; flip: boolean }
}
export interface PencilArtwork { design: string; layers: PencilLayer[] }
export interface VideoSettings { engineRoot: string; nodePath: string; pythonPath: string; modelPath: string }
export interface VideoEnvironmentItem { id: 'node' | 'engine' | 'ffmpeg' | 'browser' | 'fonts' | 'python' | 'model'; status: 'ok' | 'error' | 'skipped'; version: string; path: string; detail: string }
export interface VideoEnvironmentReport { settings: VideoSettings; items: VideoEnvironmentItem[] }
export interface VideoCue { start: number; end: number; text: string }
export interface VideoSceneRevisionInput { jobId: string; runId: number; providerId: string; sceneIndex: number; subtitles: string; scene: VideoScene; previousScene?: VideoScene; nextScene?: VideoScene; instruction: string }
export interface VideoSceneImage {
  assetId: string; source: string; rights: string; creator?: string
  fit: 'contain' | 'cover'; x: number; y: number; sha256?: string
}
export interface VideoScene {
  first: number; last: number; kind: keyof typeof visualCatalog.kinds | 'book' | 'quote' | 'flow' | 'timeline'; title: string
  environment?: keyof typeof visualCatalog.environments
  framing?: keyof typeof visualCatalog.framings
  beats?: VideoBeat[]
  labels?: string[]
  image?: VideoSceneImage
  painting?: { description: string; motion: 'static' | 'push_in' | 'pull_out' | 'pan_left' | 'pan_right'; taskId?: string }
  artwork?: PencilArtwork
}
export interface VideoSubtitleReview {
  promptVersion: string; completed: number
  reference: { book: string; author: string; opening: string; ending: string }
}
export interface VideoPlan { title: string; audioSha256: string; duration: number; cues: VideoCue[]; scenes: VideoScene[]; theme?: VideoTheme; imageProviderId?: string; subtitleReview?: VideoSubtitleReview; pencil?: { characters: PencilCharacter[]; pending: { first: number; last: number; title: string; description: string; characters: string[] }[] } }
export interface VideoProgress {
  startedAt: number; stage: string; phase: string; completed: number; total: number
  cueCount: number; preview: VideoCue[]
  unit?: 'frames' | 'seconds' | 'cues' | null; speed?: number | null; updatedAt?: number
}
export interface VideoPreview { start: number; end: number; signature: string; scenes: number[] }
export interface VideoJob {
  chapterId?: string | null
  id: string; projectId: string; audioAssetId: string | null; sourceSubtitleId: string | null; providerId: string | null
  status: 'draft' | 'running' | 'awaiting_confirmation' | 'succeeded' | 'failed' | 'cancelled'
  stage: string; runId: number; plan: VideoPlan; error: string
  subtitleAssetId: string | null; videoAssetId: string | null; createdAt: string; updatedAt: string
  progress?: VideoProgress | null
  preview?: VideoPreview | null
  draftUpdatedAt?: string | null
}
export type VideoJobSummary = Omit<VideoJob, 'plan' | 'preview' | 'draftUpdatedAt'> & { progress?: VideoProgress | null }
export const videoStatuses: Record<VideoJob['status'], string> = { draft: '待开始', running: '制作中', awaiting_confirmation: '等待确认分镜', succeeded: '成片已保存', failed: '处理失败', cancelled: '已停止' }
export const videoStages: Record<string, string> = { preparing: '检查音频与运行环境', transcribing: '识别原声字幕', proofreading: 'AI 校对原声字幕', transcript_ready: '字幕已就绪，自动流程正在准备文案和封面', storyboard: '生成分镜', painting: '逐镜绘制精细水墨画作', previewing: '渲染原声试片', rendering: '绘制画面与合入原声', validating: '校验成片并归档', complete: '制作完成' }
