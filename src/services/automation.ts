import { invoke } from '@tauri-apps/api/core'
import { ref } from 'vue'
import type { CoverConfig } from '../types/cover'
import { coverPng, decodeBackground } from './coverRenderer'
import { readBackground } from './covers'

export interface AutomationInput {
  projectId: string
  sourceAssetId: string
  textProviderId: string
  imageProviderId: string
  format: 'deep_dive' | 'brief'
  requestKey: string
}
export interface AutomationRun {
  id: string
  projectId: string
  input: { workflowVersion: number; title: string; author: string; targetDurationSeconds: number | null; durationReason: string; selection: AutomationInput }
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
  landscapeTaskId: string | null
  portraitTaskId: string | null
  landscapeCoverId: string | null
  portraitCoverId: string | null
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
  duration: '推荐目标时长', writing: '生成完整创作方案', audio: '按本次方案制作音频', video_prepare: '字幕校对与分镜', content_check: '依据实际字幕准备发布文案',
  landscape_image: '生成横版背景', landscape_cover: '导出横版封面', portrait_image: '生成竖版背景',
  portrait_cover: '导出竖版封面', render: '渲染成片', publishing: '上传并填写抖音',
}
export const startAutomation = (input: AutomationInput) => invoke<AutomationRun>('start_automation', { input })
export const controlAutomation = (id: string, action: 'pause' | 'resume' | 'cancel' | 'retry') => invoke<AutomationRun>('control_automation', { id, action })
export const prepareAccount = () => invoke<AutomationAccount>('prepare_automation_account')
export async function confirmAccount(expected: AutomationAccount) {
  await invoke<void>('confirm_automation_account', { expected })
  automationAccount.value = expected
}

let refreshing: Promise<void> | null = null
export function refreshAutomation(): Promise<void> {
  if (refreshing) return refreshing
  refreshing = invoke<AutomationRun[]>('automation_snapshot').then(runs => { automationRuns.value = runs; automationError.value = '' })
    .catch(() => { automationError.value = '自动流程状态读取失败，请稍后刷新。' })
    .finally(() => { refreshing = null })
  return refreshing
}

const rendering = new Set<string>()
export async function renderAutomationCover(run: AutomationRun): Promise<void> {
  if (run.status !== 'running' || !['landscape_cover', 'portrait_cover'].includes(run.stage)
    || (run.stage === 'landscape_cover' ? run.landscapeCoverId : run.portraitCoverId)) return
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
