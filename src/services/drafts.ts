import { invoke } from '@tauri-apps/api/core'
export interface EditorDraft<T> { basedOn: string | null; content: T; updatedAt?: string }
export type DraftKind = 'writing' | 'cover' | 'video' | 'project' | 'publishing'
// Video drafts belong to a job; the other editors belong to a project.
export const readDraft = <T>(ownerId: string, kind: DraftKind) => kind === 'video' ? invoke<EditorDraft<T> | null>('read_video_draft', { id: ownerId }) : invoke<EditorDraft<T> | null>('read_editor_draft', { projectId: ownerId, kind })
export const saveDraft = <T>(ownerId: string, kind: DraftKind, draft: EditorDraft<T> | null) => kind === 'video' ? invoke<void>('save_video_draft', { id: ownerId, draft }) : invoke<void>('save_editor_draft', { projectId: ownerId, kind, draft })
