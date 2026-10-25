import { coverSizes, type CoverConfig } from '../types/cover'

type Context = globalThis.CanvasRenderingContext2D
export const coverFonts = {
  sans: '"Microsoft YaHei", "PingFang SC", "Noto Sans CJK SC", sans-serif',
  serif: '"SimSun", "Songti SC", "Noto Serif CJK SC", serif',
}
const segmenter = new Intl.Segmenter('zh', { granularity: 'grapheme' })

export function wrapText(text: string, width: number, measure: (line: string) => number): string[] {
  const lines: string[] = []
  for (const paragraph of text.replace(/\r\n?/g, '\n').split('\n')) {
    let line = ''
    for (const { segment } of segmenter.segment(paragraph)) {
      if (measure(segment) > width) throw new Error('单个字符超出排版区域，请减小字号。')
      if (line && measure(line + segment) > width) { lines.push(line); line = '' }
      line += segment
    }
    lines.push(line)
  }
  return lines
}

function fitText(ctx: Context, text: string, family: string, requestedSize: number, width: number, height: number, weight: number) {
  if (!text.trim()) return { lines: [], size: requestedSize, lineHeight: 0, height: 0 }
  for (let size = requestedSize; size >= 20; size--) {
    ctx.font = `${weight} ${size}px ${family}`
    const lines = wrapText(text, width, line => {
      const metrics = ctx.measureText(line)
      return Math.max(metrics.width, metrics.actualBoundingBoxLeft + metrics.actualBoundingBoxRight)
    })
    const lineHeight = size * 1.45
    if (lines.length * lineHeight <= height) return { lines, size, lineHeight, height: lines.length * lineHeight }
  }
  throw new Error('文字过多，已超出安全区域。请减少文字或手动调整换行。')
}

export async function decodeBackground(bytes: ArrayBuffer): Promise<globalThis.HTMLImageElement> {
  const source = await new Promise<string>((resolve, reject) => {
    const reader = new globalThis.FileReader()
    reader.onload = () => resolve(String(reader.result))
    reader.onerror = () => reject(new Error('背景图读取失败。'))
    reader.readAsDataURL(new Blob([bytes]))
  })
  const image = new globalThis.Image()
  image.src = source
  try { await image.decode() } catch { throw new Error('背景图无法解码，请选择有效的 PNG、JPEG 或 WebP 图片。') }
  if (image.naturalWidth * image.naturalHeight > 40_000_000) throw new Error('背景图超过 4000 万像素，请缩小后再使用。')
  return image
}

// Preview and export call this same renderer at the full output resolution.
export function backgroundRect(config: CoverConfig, image: { naturalWidth: number; naturalHeight: number }) {
  const zoom = config.backgroundZoom ?? 1
  const x = config.backgroundX ?? 0.5; const y = config.backgroundY ?? 0.5
  if (![zoom,x,y].every(Number.isFinite) || zoom < 1 || zoom > 3 || x < 0 || x > 1 || y < 0 || y > 1) throw new Error('背景缩放或位置无效。')
  const scale = Math.max(config.width / image.naturalWidth, config.height / image.naturalHeight) * zoom
  const width = image.naturalWidth * scale; const height = image.naturalHeight * scale
  return { x: (config.width - width) * x, y: (config.height - height) * y, width, height }
}
export function renderCover(canvas: globalThis.HTMLCanvasElement, config: CoverConfig, background: globalThis.HTMLImageElement | null) {
  if (!coverSizes.some(size => size.width === config.width && size.height === config.height)) throw new Error('封面尺寸无效。')
  if (!Number.isInteger(config.titleSize) || config.titleSize < 40 || config.titleSize > 160 || !Number.isInteger(config.subtitleSize) || config.subtitleSize < 24 || config.subtitleSize > 80) throw new Error('标题字号须为 40–160，副标题字号须为 24–80 的整数。')
  canvas.width = config.width; canvas.height = config.height
  const ctx = canvas.getContext('2d')
  if (!ctx) throw new Error('当前环境无法创建封面画布。')
  const { width, height } = config
  ctx.fillStyle = config.backgroundColor; ctx.fillRect(0, 0, width, height)
  if (config.backgroundAssetId) {
    if (!background) throw new Error('背景图不可用，请重新选择图片或使用纯色背景。')
    const rect = backgroundRect(config, background)
    ctx.drawImage(background, rect.x, rect.y, rect.width, rect.height)
    ctx.globalAlpha = config.overlayOpacity
    ctx.fillStyle = config.backgroundColor; ctx.fillRect(0, 0, width, height)
    ctx.globalAlpha = 1
  }
  if (!config.title.trim()) throw new Error('请填写封面主标题。')
  const family = coverFonts[config.fontFamily]
  const padding = width * 0.09; const textWidth = width - padding * 2
  const title = fitText(ctx, config.title, family, config.titleSize, textWidth, height * 0.42, 700)
  const subtitle = fitText(ctx, config.subtitle, family, config.subtitleSize, textWidth, height * 0.18, 400)
  const brand = fitText(ctx, config.brand, family, 32, textWidth, height * 0.09, 400)
  const gap = subtitle.lines.length ? 32 : 0
  const total = title.height + subtitle.height + gap
  const y = config.position === 'top' ? height * 0.23 : config.position === 'bottom' ? height * 0.9 - total : (height - total) / 2
  const top = Math.max(height * 0.23, y)
  if (top + total > height * 0.92) throw new Error('文字超出安全区域，请减少文字。')
  const x = config.align === 'center' ? width / 2 : padding
  ctx.textAlign = config.align; ctx.textBaseline = 'middle'
  const draw = (block: ReturnType<typeof fitText>, start: number, color: string, weight: number) => {
    ctx.font = `${weight} ${block.size}px ${family}`; ctx.fillStyle = color
    block.lines.forEach((line, i) => ctx.fillText(line, x, start + block.lineHeight * (i + 0.5)))
  }
  draw(brand, height * 0.06, config.accentColor, 400)
  ctx.fillStyle = config.accentColor
  ctx.fillRect(config.align === 'center' ? width / 2 - 32 : padding, top - 28, 64, 6)
  draw(title, top, config.textColor, 700)
  draw(subtitle, top + title.height + gap, config.textColor, 400)
  return { titleSize: title.size, subtitleSize: subtitle.size }
}

export async function coverPng(config: CoverConfig, background: globalThis.HTMLImageElement | null): Promise<number[]> {
  await globalThis.document.fonts.ready
  const canvas = globalThis.document.createElement('canvas')
  renderCover(canvas, config, background)
  const blob = await new Promise<Blob>((resolve, reject) => canvas.toBlob(result => result ? resolve(result) : reject(new Error('PNG 导出失败，编辑内容已保留。')), 'image/png'))
  return Array.from(new Uint8Array(await blob.arrayBuffer()))
}
