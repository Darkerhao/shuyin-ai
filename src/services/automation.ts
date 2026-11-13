import { invoke } from '@tauri-apps/api/core'
import { ref } from 'vue'
import type { CoverConfig } from '../types/cover'
import type { AudioSourceInput } from '../types/audio'
import { coverPng, decodeBackground } from './coverRenderer'
import { readBackground } from './covers'

export interface AutomationInput {
  projectId: string
  source: AudioSourceInput
  textProviderId: string
  imageProviderId: string
  format: 'deep_dive' | 'brief'
  requestKey: string
  collectionName?: string | null
}
export interface AutomationRun {
  id: string
  chapterId?: string | null
  projectId: string
  input: { workflowVersion: number; title: string; author: string; chapterSource?: { chapterTitle: string }; targetDurationSeconds: number | null; durationReason: string; selection: AutomationInput; account?: AutomationAccount; textProvider?: { name: string; modelName: string }; imageProvider?: { name: string; modelName: string } }
  status: 'running' | 'paused' | 'waiting_user' | 'awaiting_publish' | 'completed' | 'cancelled'
  stage: string
  runId: number
  error: string
  audioJobId: string | null
  videoJobId: string | null
  writingTaskId: string | null
  durationTaskId: string | null
  contentCheckTaskId: string | null
  writingVersionId: string | null
  finalWritingVersionId: string | null
  actualDurationSeconds: number | null
  landscapeTaskId: string | null
  portraitTaskId: string | null
  widescreenTaskId: string | null
  landscapeCoverId: string | null
  portraitCoverId: string | null
  widescreenCoverId: string | null
  publishTaskId: string | null
  createdAt: string
  updatedAt: string
}
export interface AutomationAccount { id: string | null; name: string; key: string }
export const automationRuns = ref<AutomationRun[]>([])
export const automationError = ref('')
export const automationConsent = ref(false)
export const automationAccount = ref<AutomationAccount | null>(null)
export const stages: Record<string, string> = {
  video_prepare: '准备字幕与分镜（旧流程）',
  duration: '推荐目标时长', writing: '生成完整创作方案', audio: '按本次方案制作音频', transcript: '识别并校对实际字幕', content_check: '核对文案与实际音频',
  landscape_image: '生成横版背景', landscape_cover: '导出横版封面', portrait_image: '生成竖版背景',
  portrait_cover: '导出竖版封面', widescreen_image: '生成16:9宽屏背景', widescreen_cover: '导出16:9宽屏封面', storyboard: '根据实际字幕生成分镜', render: '渲染成片', publishing: '上传并填写抖音',
}
export const startAutomation = (input: AutomationInput) => invoke<AutomationRun>('start_automation', { input })
export const controlAutomation = (id: string, action: 'pause' | 'resume' | 'cancel' | 'retry') => invoke<AutomationRun>('control_automation', { id, action })
export function prepareAccount() {
  automationAccount.value = null
  return invoke<AutomationAccount>('prepare_automation_account')
}
export async function confirmAccount(expected: AutomationAccount) {
  automationAccount.value = null
  const confirmed = await invoke<AutomationAccount>('confirm_automation_account', { expected })
  automationAccount.value = confirmed
  return confirmed
}

let refreshing: Promise<void> | null = null
export function refreshAutomation(): Promise<void> {
  if (refreshing) return refreshing
  refreshing = invoke<AutomationRun[]>('automation_snapshot').then(runs => { automationRuns.value = runs; automationError.value = '' })
    .catch(() => { automationError.value = '自动流程状态读取失败，请稍后刷新。' })
    .finally(() => { refreshing = null })
  return refreshing
}

const coverSlots = { landscape_cover: 'landscapeCoverId', portrait_cover: 'portraitCoverId', widescreen_cover: 'widescreenCoverId' } as const
const rendering = new Set<string>()
export async function renderAutomationCover(run: AutomationRun): Promise<void> {
  const slot = coverSlots[run.stage as keyof typeof coverSlots]
  if (run.status !== 'running' || !slot || run[slot]) return
  const key = `${run.id}:${run.runId}:${run.stage}`
  if (rendering.has(key)) return
  rendering.add(key)
  try {
    const config = await invoke<CoverConfig>('automation_cover', { id: run.id })
    if (!config.backgroundAssetId) throw new Error('背景尚未就绪。')
    const background = await decodeBackground(await readBackground(run.projectId, config.backgroundAssetId))
    const png = await coverPng(config, background)
    await invoke<void>('complete_automation_cover', { id: run.id, runId: run.runId, stage: run.stage, png })
  } catch {
    await invoke<void>('fail_automation_cover', { id: run.id, runId: run.runId, stage: run.stage }).catch(() => { automationError.value = '封面导出状态保存失败，请暂停并检查原流程。' })
  } finally { rendering.delete(key) }
}
