export interface CoverConfig {
  schemaVersion: 1
  templateId: 'classic' | 'night' | 'spotlight' | 'ember' | 'ocean'
  width: number
  height: number
  title: string
  subtitle: string
  brand: string
  fontFamily: 'sans' | 'serif'
  titleSize: number
  subtitleSize: number
  align: 'left' | 'center'
  position: 'top' | 'center' | 'bottom'
  backgroundColor: string
  textColor: string
  subtitleColor: string
  accentColor: string
  backgroundAssetId: string | null
  overlayOpacity: number
  backgroundX: number
  backgroundY: number
  backgroundZoom: number
  backgroundPrompt: string
}
export type CoverDesign = Pick<CoverConfig, 'titleSize' | 'subtitleSize' | 'backgroundColor' | 'textColor' | 'subtitleColor' | 'accentColor' | 'overlayOpacity'>
export type CoverStyle = Pick<CoverConfig, 'templateId' | 'brand' | 'fontFamily' | 'align' | 'position'> & CoverDesign
export interface CoverPreset { id: string; name: string; style: CoverStyle }
export interface CoverVersion {
  id: string
  chapterId?: string | null
  projectId: string
  versionNo: number
  config: CoverConfig
  exportedPath: string | null
  createdAt: string
}
export const coverSizes = [
  { label: '横版 4:3 · 1440 × 1080', width: 1440, height: 1080, promptField: 'coverPromptLandscape' },
  { label: '竖版 3:4 · 1080 × 1440', width: 1080, height: 1440, promptField: 'coverPromptPortrait' },
  { label: '横版 16:9 · 1920 × 1080', width: 1920, height: 1080, promptField: 'coverPromptWidescreen' },
] as const
export interface CoverWorkspace {
  schemaVersion: 2
  activeSize: number
  covers: CoverConfig[]
}
export function coverSizeIndex(config: CoverConfig): number {
  const index = coverSizes.findIndex(s => s.width === config.width && s.height === config.height)
  return index < 0 ? (config.width > config.height ? 0 : 1) : index
}
export function normalizeCover(config: CoverConfig, index = coverSizeIndex(config)): CoverConfig {
  const { width, height } = coverSizes[index]!
  return { ...newCover(), ...config, width, height, subtitleColor: config.subtitleColor ?? config.textColor }
}
export function coverWorkspace(versions: CoverVersion[], fallback = newCover()): CoverWorkspace {
  const latest = versions[0]?.config ?? fallback
  return {
    schemaVersion: 2,
    activeSize: coverSizeIndex(latest),
    covers: coverSizes.map((size, index) => normalizeCover(versions.find(v => v.config.width === size.width && v.config.height === size.height)?.config ?? latest, index)),
  }
}
export function recoverCoverWorkspace(content: CoverWorkspace | CoverConfig): CoverWorkspace {
  if (content.schemaVersion === 1) return coverWorkspace([], content)
  return { ...content, covers: content.covers.map((config, index) => normalizeCover(config, index)) }
}
export const coverTemplates = {
  classic: { label: '纸页', description: '温暖人文 · 观点解读', style: { backgroundColor: '#f4efdf', textColor: '#24362f', subtitleColor: '#74432b', accentColor: '#9a5938', titleSize: 136, subtitleSize: 50, overlayOpacity: 0.72, fontFamily: 'serif', align: 'left', position: 'center' } },
  night: { label: '深林', description: '沉静有力 · 文学故事', style: { backgroundColor: '#183c32', textColor: '#fff8e8', subtitleColor: '#e6d29f', accentColor: '#d6bc7f', titleSize: 148, subtitleSize: 52, overlayOpacity: 0.65, fontFamily: 'sans', align: 'center', position: 'center' } },
  spotlight: { label: '黑金焦点', description: '强烈反差 · 悬念提问', style: { backgroundColor: '#141414', textColor: '#ffe45c', subtitleColor: '#ffffff', accentColor: '#ff7547', titleSize: 152, subtitleSize: 56, overlayOpacity: 0.75, fontFamily: 'sans', align: 'center', position: 'center' } },
  ember: { label: '赤焰醒目', description: '鲜明情绪 · 冲突故事', style: { backgroundColor: '#36151b', textColor: '#fff6ed', subtitleColor: '#ffd166', accentColor: '#ff715b', titleSize: 152, subtitleSize: 54, overlayOpacity: 0.72, fontFamily: 'sans', align: 'left', position: 'center' } },
  ocean: { label: '蓝调纪实', description: '清晰利落 · 历史新知', style: { backgroundColor: '#102c46', textColor: '#ffffff', subtitleColor: '#a8e5ff', accentColor: '#ffcf66', titleSize: 144, subtitleSize: 52, overlayOpacity: 0.7, fontFamily: 'sans', align: 'left', position: 'center' } },
} as const
export function newCover(title = ''): CoverConfig {
  return { schemaVersion: 1, templateId: 'spotlight', width: 1440, height: 1080, title, subtitle: '读懂一本书', brand: '甲子 · 读书', backgroundAssetId: null, backgroundX: 0.5, backgroundY: 0.5, backgroundZoom: 1, backgroundPrompt: '', ...coverTemplates.spotlight.style }
}
