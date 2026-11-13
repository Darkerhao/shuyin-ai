export interface ChapterInput {
  title: string
  content: string
  targetDurationSeconds: number | null
}
export interface ChapterSummary {
  id: string
  projectId: string
  title: string
  targetDurationSeconds: number | null
  audioDurationSource: 'ai' | 'manual' | null
  createdAt: string
  updatedAt: string
}
export interface Chapter extends ChapterSummary { content: string }
export interface ChapterSource {
  chapterId: string
  bookTitle: string
  author: string
  chapterTitle: string
  content: string
  sha256: string
  targetDurationSeconds?: number | null
}
export const emptyChapter = (): ChapterInput => ({ title: '', content: '', targetDurationSeconds: null })
export const chapterOwner = (projectId: string, chapterId: string | null) => chapterId ? `${projectId}:chapter:${chapterId}` : projectId
export const chapterQuery = (chapterId: string | null) => chapterId ? { chapter: chapterId } : {}
export const chapterIdFromQuery = (value: unknown) => typeof value === 'string' && value && value !== 'new' ? value : null
export const belongsToChapter = (item: { chapterId?: string | null }, chapterId: string | null) => (item.chapterId ?? null) === chapterId
