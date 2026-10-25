import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
export interface ImportedNote { text: string; sourceName: string; sourcePath: string }
export const chooseNote = () => open({ title: '导入读书笔记（UTF-8）', multiple: false, filters: [{ name: 'TXT / Markdown', extensions: ['txt', 'md'] }] })
export const importNote = (path: string) => invoke<ImportedNote>('import_reading_notes', { path })
export function appendNote(existing: string, note: ImportedNote) {
  const result = [existing.trimEnd(), `来源：${note.sourceName}\n文件：${note.sourcePath}\n\n${note.text}`].filter(Boolean).join('\n\n---\n\n')
  if ([...result].length > 100_000) throw new Error('追加后的资料超过 10 万字，请精简后再导入。')
  return result
}
