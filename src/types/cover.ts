export interface CoverConfig {
  schemaVersion: 1
  templateId: 'classic' | 'night'
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
  accentColor: string
  backgroundAssetId: string | null
  overlayOpacity: number
}
export interface CoverVersion {
  id: string
  projectId: string
  versionNo: number
  config: CoverConfig
  exportedPath: string | null
  createdAt: string
}
export const coverSizes = [
  { label: '横版 4:3 · 1440 × 1080', width: 1440, height: 1080 },
  { label: '竖版 3:4 · 1080 × 1440', width: 1080, height: 1440 },
] as const
export const coverTemplates = {
  classic: { label: '纸页', style: { backgroundColor: '#f4efdf', textColor: '#24362f', accentColor: '#9a5938', fontFamily: 'serif', align: 'left', position: 'center' } },
  night: { label: '深林', style: { backgroundColor: '#183c32', textColor: '#fff8e8', accentColor: '#d6bc7f', fontFamily: 'sans', align: 'center', position: 'center' } },
} as const
export function newCover(title = ''): CoverConfig {
  return { schemaVersion: 1, templateId: 'classic', width: 1440, height: 1080, title, subtitle: '十分钟读懂一本书', brand: '甲子 · 书引', titleSize: 112, subtitleSize: 44, backgroundAssetId: null, overlayOpacity: 0.45, ...coverTemplates.classic.style }
}
