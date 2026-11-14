import { reactive, shallowReactive } from 'vue'
import * as ai from './ai'
import * as covers from './covers'
import { errorMessage } from './verification'
import type { GenerationInput, GenerationResult, GenerationTextPreview, RevisionInput } from '../types/ai'
import type { Asset } from '../types/project'

export interface GenerationJob {
  projectId: string
  chapterId?: string | null
  title: string
  kind: 'writing' | 'cover'
  mode: 'writing' | 'revision' | 'cover'
  id: string
  startedAt: number
  status: 'starting' | 'running' | 'succeeded' | 'failed' | 'cancelled'
  cancelling: boolean
  preview: GenerationTextPreview
  result?: GenerationResult | Asset
  error: string
  size?: { width: number; height: number }
  revision?: RevisionInput
}

// Own the invoke and its channel for the application session, independent of mounted editors.
// Validated results and terminal task status remain in the existing SQLite task/version tables.
export const generationJobs = shallowReactive(new Map<string, GenerationJob>())
export const generationKey = (projectId: string, kind: GenerationJob['kind'], chapterId: string | null = null) => `${projectId}${chapterId ? `:chapter:${chapterId}` : ''}:${kind}`
export const isGenerating = (job?: GenerationJob) => !!job && (job.status === 'starting' || job.status === 'running')

async function start(job: GenerationJob, create: () => Promise<string>, execute: (id: string) => Promise<GenerationResult | Asset>) {
  const key = generationKey(job.projectId, job.kind, job.chapterId ?? null)
  if (isGenerating(generationJobs.get(key))) throw new Error('此书的生成任务仍在进行，请等待完成或取消。')
  generationJobs.set(key, job)
  try {
    job.id = await create()
    job.status = 'running'
    void execute(job.id).then(result => {
      job.result = result
      job.status = 'succeeded'
    }).catch(cause => {
      job.error = errorMessage(cause)
      job.status = typeof cause === 'object' && cause !== null && 'code' in cause && cause.code === 'cancelled' ? 'cancelled' : 'failed'
    }).finally(() => { job.preview = {}; job.cancelling = false })
  } catch (cause) {
    job.error = errorMessage(cause)
    job.status = 'failed'
    throw cause
  }
}

function newJob(projectId: string, title: string, kind: GenerationJob['kind'], mode: GenerationJob['mode']): GenerationJob {
  return reactive({ projectId, title, kind, mode, id: '', startedAt: Date.now(), status: 'starting', cancelling: false, preview: {}, error: '' })
}

export function startWritingGeneration(input: GenerationInput, title: string) {
  const job = newJob(input.projectId, title, 'writing', input.revision ? 'revision' : 'writing')
  job.chapterId = input.chapterId ?? null
  job.revision = input.revision
  return start(job, () => ai.startGeneration(input), id => ai.executeGeneration(id, content => {
    if (job.status === 'running' && !job.cancelling) job.preview = content
  }))
}

export function startBackgroundGeneration(projectId: string, title: string, providerId: string, description: string, width: number, height: number, chapterId: string | null = null) {
  const job = newJob(projectId, title, 'cover', 'cover')
  job.chapterId = chapterId
  job.size = { width, height }
  return start(job, () => covers.startCoverGeneration(projectId, providerId, description, width, height, chapterId), covers.executeCoverGeneration)
}

export async function cancelGenerationJob(job: GenerationJob) {
  if (job.status !== 'running' || job.cancelling) return
  job.cancelling = true
  try {
    await ai.cancelGeneration(job.id)
    job.preview = {}
    // The execute response is authoritative: completion may have won before cancellation.
    // Keep fragments suppressed while the backend settles the request.
    if (!isGenerating(job)) job.cancelling = false
  } catch (cause) { job.cancelling = false; throw cause }
}

export function dismissGeneration(job: GenerationJob) {
  const key = generationKey(job.projectId, job.kind, job.chapterId ?? null)
  if (!isGenerating(job) && generationJobs.get(key) === job) generationJobs.delete(key)
}
