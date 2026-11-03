import type { VideoCue } from '../../types/video'

export interface SubtitleRow extends VideoCue { index: number; textStart: number; textEnd: number }
export interface VideoEditorPosition { section: 'subtitles' | 'scene' | 'srt'; index: number; onlyChanges: boolean; cursor: number }
export interface VideoEditorDraft { subtitles: string; scenes: string; position: VideoEditorPosition }
export const initialPosition = (): VideoEditorPosition => ({ section: 'subtitles', index: 0, onlyChanges: false, cursor: 0 })

// Offsets let sentence edits replace only the text, preserving the user's SRT formatting/timecodes.
export function subtitleRows(srt: string): SubtitleRow[] {
  const rows: SubtitleRow[] = []
  const blocks = /(?:^\uFEFF?|\r?\n[\t ]*\r?\n)(\d+)\r?\n(\d{2}:\d{2}:\d{2},\d{3}) --> (\d{2}:\d{2}:\d{2},\d{3})\r?\n([\s\S]*?)(?=\r?\n[\t ]*\r?\n|$)/g
  for (const match of srt.matchAll(blocks)) {
    const text = match[4]!.replace(/\r?\n$/, '')
    const textStart = match.index + match[0].length - match[4]!.length
    const time = (value: string) => {
      const [h, m, s, ms] = value.split(/[:,]/).map(Number)
      return m! < 60 && s! < 60 ? h! * 3600 + m! * 60 + s! + ms! / 1000 : NaN
    }
    const start = time(match[2]!); const end = time(match[3]!)
    if (!Number.isFinite(start) || !Number.isFinite(end) || end <= start || Number(match[1]) !== rows.length + 1) return []
    rows.push({ index: rows.length, start, end, text, textStart, textEnd: textStart + text.length })
  }
  // Do not offer row operations on a partially parsed file.
  const stripped = srt.replace(/^\uFEFF/, '').replace(blocks, '').trim()
  return stripped ? [] : rows
}

export function replaceSubtitleText(srt: string, row: SubtitleRow, text: string) {
  return srt.slice(0, row.textStart) + text + srt.slice(row.textEnd)
}

export const cueKey = (row: VideoCue) => `${Math.round(row.start * 1000)}:${Math.round(row.end * 1000)}`

export const subtitleClock = (seconds: number) => new Date(seconds * 1000).toISOString().slice(11, 23)
