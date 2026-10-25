import { describe, expect, it } from 'vitest'
import { appendNote } from '../src/services/notes'
describe('reading notes provenance', () => {
  const note = { text: '# 笔记\n保留内容', sourceName: '原始笔记.md', sourcePath: 'D:/书籍/原始笔记.md' }
  it('retains existing notes, markdown and the source name and path', () => {
    expect(appendNote('已有资料\n', note)).toBe('已有资料\n\n---\n\n来源：原始笔记.md\n文件：D:/书籍/原始笔记.md\n\n# 笔记\n保留内容')
    expect(appendNote('', note)).toMatch(/^来源：/)
  })
  it('rejects oversized merged notes without changing the original input', () => {
    const existing = '字'.repeat(100_000)
    expect(() => appendNote(existing, note)).toThrow('10 万字')
    expect(existing.length).toBe(100_000)
  })
})
