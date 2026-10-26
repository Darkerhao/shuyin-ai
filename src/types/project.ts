export const projectStatuses = {
  draft: '待创作', researching: '资料整理', scripting: '讲稿创作',
  material_preparing: '准备素材', ready_to_publish: '待发布', published: '已发布', archived: '已归档',
} as const
export type ProjectStatus = keyof typeof projectStatuses
export interface ProjectInput {
  title: string; author: string; publisher: string; isbn: string; description: string
  sourceNotes: string; theme: string; targetAudience: string; contentStyle: string
  targetDurationSeconds: number; status: ProjectStatus
}
export interface Project extends ProjectInput {
  id: string; workspacePath: string; createdAt: string; updatedAt: string; archivedAt: string | null
}
export interface Asset {
  id: string; projectId: string; assetType: 'video' | 'audio' | 'image' | 'cover' | 'subtitle' | 'document'
  filePath: string; originalName: string; fileSize: number; modifiedAt: number
  existsStatus: 'available' | 'missing' | 'unknown'; createdAt: string
}
export interface WorkspaceSettings { root: string; databasePath: string }
export const emptyProject = (): ProjectInput => ({
  title: '', author: '', publisher: '', isbn: '', description: '', sourceNotes: '', theme: '',
  targetAudience: '', contentStyle: '', targetDurationSeconds: 600, status: 'draft',
})
export function projectInput(project: Project): ProjectInput {
  const { title, author, publisher, isbn, description, sourceNotes, theme, targetAudience, contentStyle, targetDurationSeconds, status } = project
  return { title, author, publisher, isbn, description, sourceNotes, theme, targetAudience, contentStyle, targetDurationSeconds, status }
}
