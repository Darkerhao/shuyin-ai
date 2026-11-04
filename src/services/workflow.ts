import { invoke } from '@tauri-apps/api/core'
export interface ResumeTask { projectId: string; title: string; kind: string; taskId: string; status: string; stage: string; updatedAt: string; running: boolean }
export interface WorkflowSnapshot { resume: ResumeTask[]; running: ResumeTask[] }
export const workflowSnapshot = () => invoke<WorkflowSnapshot>('workflow_snapshot')
export interface ProjectStage { path: string; status: string; tone: 'neutral' | 'ready' | 'working' | 'attention' }
export const projectProgress = (projectId: string) => invoke<ProjectStage[]>('project_progress', { projectId })
export function resumeRoute(task: ResumeTask) {
  const path = `/projects/${task.projectId}`
  if (task.kind === 'audio') return { path: `${path}/writing`, query: { tab: 'audio', audioJob: task.taskId } }
  if (task.kind === 'video') return { path: `${path}/video`, query: task.stage.startsWith('video_') ? {} : { job: task.taskId } }
  if (task.kind === 'publishing') return { path: `${path}/publishing`, query: { task: task.taskId } }
  return { path: `${path}/${task.kind === 'covers' ? 'covers' : 'writing'}` }
}
export function resumeLabel(task: ResumeTask) {
  if (task.kind === 'video') {
    if (task.running) return ({ proofreading: '正在校对字幕', storyboard: '正在生成分镜', previewing: '正在制作试片', rendering: '正在渲染视频', validating: '正在校验成片' } as Record<string, string>)[task.stage] ?? '视频任务进行中'
    return task.status === 'succeeded' ? '查看成片并准备发布' : task.status === 'awaiting_confirmation' ? '继续确认分镜' : '继续视频制作'
  }
  if (task.kind === 'audio') return task.running ? '音频制作进行中' : task.status === 'succeeded' ? '继续使用音频成品' : '继续音频任务'
  if (task.kind === 'publishing') return task.status === 'published' ? '查看发布记录' : task.status === 'ready' ? '继续待发布任务' : '补齐发布准备'
  return task.kind === 'covers' ? '继续封面制作' : '继续内容创作'
}
