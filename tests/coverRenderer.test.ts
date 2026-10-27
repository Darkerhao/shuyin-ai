import { describe, expect, it, vi } from 'vitest'
import { backgroundRect, renderCover, wrapText } from '../src/services/coverRenderer'
import { coverSizes, newCover } from '../src/types/cover'

function canvas() {
  const ctx = { font: '', fillStyle: '', textAlign: '', textBaseline: '', fillRect: vi.fn(), drawImage: vi.fn(), fillText: vi.fn(), measureText(text: string) {
    const size = Number(this.font.match(/(\d+)px/)?.[1] ?? 20)
    return { width: Array.from(text).length * size, actualBoundingBoxLeft: 0, actualBoundingBoxRight: Array.from(text).length * size }
  } }
  return { ctx, canvas: { width: 0, height: 0, getContext: () => ctx } as unknown as HTMLCanvasElement }
}
describe('cover text layout', () => {
  it.each(coverSizes)('draws title, subtitle and accents with independent colors at $width × $height', size => {
    const target = canvas()
    const colors: Record<string, string> = {}
    target.ctx.fillText.mockImplementation((text: string) => { colors[text] = target.ctx.fillStyle })
    const config = { ...newCover('主标题'), ...size, subtitle: '副标题', brand: '品牌', textColor: '#ffe45c', subtitleColor: '#a8e5ff', accentColor: '#ff7547' }
    renderCover(target.canvas, config, null)
    expect(colors).toEqual({ 主标题: config.textColor, 副标题: config.subtitleColor, 品牌: config.accentColor })
    const legacy = { ...config }
    Reflect.deleteProperty(legacy, 'subtitleColor')
    renderCover(target.canvas, legacy, null)
    expect(colors['副标题']).toBe(config.textColor)
  })
  it.each(coverSizes)('preserves the chosen focal point and covers all edges at $width × $height', size => {
    const image = { naturalWidth: 2000, naturalHeight: 1000 } as HTMLImageElement
    const config = { ...newCover('主体'), width: size.width, height: size.height, backgroundAssetId: 'image', backgroundX: 0.2, backgroundY: 0.9, backgroundZoom: 1.5 }
    const rect = backgroundRect(config, image)
    expect(rect.x).toBeCloseTo((config.width - rect.width) * 0.2)
    expect(rect.y).toBeCloseTo((config.height - rect.height) * 0.9)
    expect(rect.x).toBeLessThanOrEqual(0); expect(rect.y).toBeLessThanOrEqual(0)
    expect(rect.x + rect.width).toBeGreaterThanOrEqual(config.width)
    expect(rect.y + rect.height).toBeGreaterThanOrEqual(config.height)
    const target = canvas()
    renderCover(target.canvas, config, image)
    expect(target.ctx.drawImage).toHaveBeenCalledWith(image, rect.x, rect.y, rect.width, rect.height)
    expect(() => backgroundRect({ ...config, backgroundZoom: NaN }, image)).toThrow('背景')
  })
  it('wraps Chinese, long words, explicit newlines and whole graphemes', () => {
    const measure = (line: string) => [...new Intl.Segmenter('zh', { granularity: 'grapheme' }).segment(line)].length * 10
    expect(wrapText('中文封面\r\nABCDEF\n👨‍👩‍👧‍👦你好', 20, measure)).toEqual(['中文', '封面', 'AB', 'CD', 'EF', '👨‍👩‍👧‍👦你', '好'])
  })
  it.each(coverSizes)('fits long Chinese text inside $width × $height', size => {
    const target = canvas(); const config = { ...newCover('中文长标题'.repeat(35)), ...size, subtitle: '副标题'.repeat(70), brand: '甲子', titleSize: 160, subtitleSize: 80 }
    const result = renderCover(target.canvas, config, null)
    expect(target.canvas.width).toBe(size.width); expect(target.canvas.height).toBe(size.height)
    expect(result.titleSize).toBeLessThan(160)
    for (const [, x, y] of target.ctx.fillText.mock.calls as [string, number, number][]) {
      expect(x).toBeGreaterThanOrEqual(size.width * 0.09)
      expect(y).toBeGreaterThan(0); expect(y).toBeLessThan(size.height * 0.92)
    }
  })
  it('rejects unsafe sizes, empty title, unavailable background and overflowing line breaks', () => {
    const target = canvas(); const config = newCover('测试')
    for (const [width, height] of [[1080, 1920], [1280, 720], [1080, 1080], [1440, 1440]]) {
      expect(() => renderCover(target.canvas, { ...config, width: width!, height: height! }, null)).toThrow('尺寸无效')
    }
    expect(() => renderCover(target.canvas, { ...config, titleSize: Infinity }, null)).toThrow('字号')
    expect(() => renderCover(target.canvas, { ...config, title: '' }, null)).toThrow('主标题')
    expect(() => renderCover(target.canvas, { ...config, backgroundAssetId: 'a1' }, null)).toThrow('背景图')
    expect(() => renderCover(target.canvas, { ...config, title: '字\n'.repeat(99) }, null)).toThrow('安全区域')
  })
})
