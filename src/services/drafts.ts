import { invoke } from '@tauri-apps/api/core'
export interface EditorDraft<T> { basedOn: string | null; content: T; updatedAt?: string }
export type DraftKind = 'writing' | 'cover'
export const readDraft = <T>(projectId: string, kind: DraftKind) => invoke<EditorDraft<T> | null>('read_editor_draft', { projectId, kind })
export const saveDraft = <T>(projectId: string, kind: DraftKind, draft: EditorDraft<T> | null) => invoke<void>('save_editor_draft', { projectId, kind, draft })
