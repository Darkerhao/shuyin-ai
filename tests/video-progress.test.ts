import { afterEach, expect, it, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import VideoJobProgress from '../src/modules/video/VideoJobProgress.vue'
import type { VideoJob, VideoProgress } from '../src/types/video'

const startedAt = Date.parse('2026-11-01T08:00:00Z')
const progress: VideoProgress = { startedAt, stage: 'transcribing', phase: 'loading_model', completed: 0, total: 0, cueCount: 0, preview: [] }
const job: VideoJob = {
  id: 'j1', projectId: 'p1', audioAssetId: 'a1', sourceSubtitleId: null, providerId: 'm1',
  status: 'running', stage: 'transcribing', runId: 1, error: '', subtitleAssetId: null, videoAssetId: null,
  createdAt: '2026-10-30T08:00:00Z', updatedAt: '2026-11-01T08:00:00Z', progress,
  plan: { title: '原书', audioSha256: 'hash', duration: 100, cues: [], scenes: [] },
}
const mounted: ReturnType<typeof mount>[] = []
function render(value = job) {
  const wrapper = mount(VideoJobProgress, { props: { job: structuredClone(value) } })
  mounted.push(wrapper)
  return wrapper
}
afterEach(() => { mounted.splice(0).forEach(w => w.unmount()); vi.useRealTimers() })

it('reports preview frames without marking final rendering or archiving as completed', async () => {
  const wrapper = render({ ...job, stage: 'previewing', progress: { ...progress, stage: 'previewing', phase: 'rendering_frames', completed: 360, total: 720 } })
  expect(wrapper.text()).toContain('试片')
  expect(wrapper.get('[role="progressbar"]').attributes('aria-valuenow')).toBe('50')
  expect(wrapper.findAll('.is-done')).toHaveLength(2)
  await wrapper.setProps({ job: { ...job, stage: 'previewing', status: 'awaiting_confirmation', preview: { start: 0, end: 30, signature: 'snapshot', scenes: [0] } } })
  expect(wrapper.text()).toContain('完整视频尚未生成')
  expect(wrapper.findAll('.is-done')).toHaveLength(2)
})

it('explains model loading without claiming a percentage and ticks only the current run', async () => {
  vi.useFakeTimers()
  vi.setSystemTime(startedAt + 5000)
  const wrapper = render()
  expect(wrapper.text()).toContain('正在唤醒本地语音模型')
  expect(wrapper.get('[role="progressbar"]').attributes('aria-valuenow')).toBeUndefined()
  expect(wrapper.get('[aria-label="本次执行已用时间"]').text()).toContain('00:05')
  await vi.advanceTimersByTimeAsync(2000)
  expect(wrapper.get('[aria-label="本次执行已用时间"]').text()).toContain('00:07')
  expect(wrapper.findAll('[aria-current="step"]')).toHaveLength(1)
})

it('uses actual audio position and shows real intermediate subtitles', async () => {
  const wrapper = render()
  const preview = [{ start: 38, end: 41.5, text: '刚刚识别的真实原声' }]
  await wrapper.setProps({ job: { ...job, progress: { ...progress, phase: 'recognizing', completed: 41.5, total: 100, cueCount: 8, preview } } })
  expect(wrapper.get('[role="progressbar"]').attributes('aria-valuenow')).toBe('41')
  expect(wrapper.text()).toContain('已识别 00:41 / 01:40')
  expect(wrapper.text()).toContain('刚刚识别的真实原声')
  expect(wrapper.text()).toContain('实时字幕预览')
  expect(wrapper.text()).toContain('本阶段 41%')
})

it('shows progress from saved storyboard coverage, including the saved scene preview', () => {
  const wrapper = render({ ...job, stage: 'storyboard', plan: { ...job.plan,
    cues: Array.from({ length: 4 }, (_, i) => ({ start: i * 10, end: i * 10 + 5, text: `字幕${i}` })),
    scenes: [{ first: 0, last: 0, kind: 'book', title: '已保存的第一镜', labels: ['故事开始'] }],
  } })
  expect(wrapper.get('[role="progressbar"]').attributes('aria-valuenow')).toBe('25')
  expect(wrapper.text()).toContain('已编排 1 / 4 条字幕')
  expect(wrapper.text()).toContain('已保存的第一镜')
  expect(wrapper.get('[aria-current="step"]').text()).toContain('动画分镜')
})

it('keeps subtitle proofreading in the first step and reports only saved reviewed cues', async () => {
  const plan = { ...job.plan,
    cues: Array.from({ length: 120 }, (_, i) => ({ start: i, end: i + 0.9, text: i < 60 ? `已校对${i}` : `尚未校对${i}` })),
    subtitleReview: { promptVersion: 'subtitle-proofread-v1', completed: 60, reference: { book: '原书', author: '', opening: '我是甲子', ending: '下期见' } },
  }
  const wrapper = render({ ...job, stage: 'proofreading', plan })
  expect(wrapper.get('[role="progressbar"]').attributes('aria-valuenow')).toBe('50')
  expect(wrapper.text()).toContain('已校对 60 / 120 条字幕')
  expect(wrapper.text()).toContain('参考音频提示词的开场与结尾')
  expect(wrapper.text()).toContain('最近校对的字幕')
  expect(wrapper.text()).toContain('已校对59')
  expect(wrapper.text()).not.toContain('尚未校对')
  expect(wrapper.get('[aria-current="step"]').text()).toContain('原声字幕')
  expect(wrapper.findAll('.is-done')).toHaveLength(0)
  await wrapper.setProps({ job: { ...job, stage: 'proofreading', status: 'failed', plan } })
  expect(wrapper.text()).toContain('停在「AI 校对原声字幕」')
  expect(wrapper.text()).toContain('已校对59')
  expect(wrapper.find('[role="progressbar"]').exists()).toBe(false)
})

it('does not show unreviewed text as a recent proofreading result before the first batch', () => {
  const wrapper = render({ ...job, stage: 'proofreading', plan: { ...job.plan,
    cues: [{ start: 0, end: 1, text: '尚未校对的原文' }],
    subtitleReview: { promptVersion: 'subtitle-proofread-v1', completed: 0, reference: { book: '原书', author: '', opening: '', ending: '' } },
  } })
  expect(wrapper.text()).toContain('已校对 0 / 1 条字幕')
  expect(wrapper.text()).toContain('首批校对完成并保存后')
  expect(wrapper.text()).not.toContain('尚未校对的原文')
})

it('ends running feedback at confirmation, failure, cancellation and completion', async () => {
  vi.useFakeTimers()
  const wrapper = render()
  for (const status of ['awaiting_confirmation', 'failed', 'cancelled', 'succeeded'] as const) {
    await wrapper.setProps({ job: { ...job, status } })
    expect(wrapper.find('[role="progressbar"]').exists()).toBe(false)
    expect(wrapper.find('[aria-label="本次执行已用时间"]').exists()).toBe(false)
    expect(vi.getTimerCount()).toBe(0)
    if (status === 'awaiting_confirmation') expect(wrapper.get('[aria-current="step"]').text()).toContain('确认分镜')
    if (status === 'succeeded') expect(wrapper.findAll('.is-done')).toHaveLength(5)
  }
})

it('does not reuse the previous run or stage progress when resuming', async () => {
  vi.useFakeTimers()
  vi.setSystemTime(startedAt + 120000)
  const wrapper = render()
  await wrapper.setProps({ job: { ...job, runId: 2, progress: { ...progress, startedAt: startedAt + 120000 } } })
  expect(wrapper.get('[aria-label="本次执行已用时间"]').text()).toContain('00:00')
  await wrapper.setProps({ job: { ...job, stage: 'rendering', progress: { ...progress, completed: 80, total: 100 } } })
  expect(wrapper.get('[role="progressbar"]').attributes('aria-valuenow')).toBeUndefined()
  expect(wrapper.text()).not.toContain('80%')
  expect(wrapper.text()).toContain('正在准备视频画面')
})

it('explains real rendering phases and a temporarily stale snapshot', async () => {
  const wrapper = render({ ...job, stage: 'rendering', progress: { ...progress, stage: 'rendering', phase: 'muxing_audio' } })
  expect(wrapper.text()).toContain('正在把原声合入画面')
  await wrapper.setProps({ syncError: 'IPC disconnected' })
  expect(wrapper.text()).toContain('当前展示上次读取的进度')
  expect(wrapper.get('[role="progressbar"]').attributes('aria-valuenow')).toBeUndefined()
})

it('shows real rendered frame progress without treating it as overall completion', async () => {
  const rendering: VideoJob = { ...job, stage: 'rendering', progress: { ...progress, stage: 'rendering', phase: 'rendering_frames', completed: 480, total: 960 } }
  const wrapper = render(rendering)
  expect(wrapper.get('[role="progressbar"]').attributes('aria-valuenow')).toBe('50')
  expect(wrapper.text()).toContain('已绘制 480 / 960 帧')
  expect(wrapper.get('.emblem-value').text()).toBe('50%')
  expect(wrapper.get('[aria-current="step"]').text()).toContain('画面渲染')
  await wrapper.setProps({ job: { ...rendering, progress: { ...rendering.progress!, completed: 960 } } })
  expect(wrapper.get('[role="progressbar"]').attributes('aria-valuenow')).toBe('100')
  expect(wrapper.text()).not.toContain('成片和字幕已保存')
  expect(wrapper.get('[aria-current="step"]').text()).toContain('进行中')
})

it('does not carry frame percentages into subtitle compositing or audio muxing', async () => {
  const wrapper = render({ ...job, stage: 'rendering' })
  for (const [phase, title] of [['rendering_subtitles', '正在把字幕放入画面'], ['muxing_audio', '正在把原声合入画面']] as const) {
    await wrapper.setProps({ job: { ...job, stage: 'rendering', progress: { ...progress, stage: 'rendering', phase, completed: 960, total: 960 } } })
    expect(wrapper.text()).toContain(title)
    expect(wrapper.get('[role="progressbar"]').attributes('aria-valuenow')).toBeUndefined()
    expect(wrapper.find('.emblem-value').exists()).toBe(false)
  }
})

it('shows actual subtitle encoding time, percentage, speed and feedback age', () => {
  vi.useFakeTimers()
  vi.setSystemTime(startedAt + 62000)
  const wrapper = render({ ...job, stage: 'rendering', progress: { ...progress, stage: 'rendering', phase: 'rendering_subtitles', unit: 'seconds', completed: 41.5, total: 100, speed: 1.25, updatedAt: startedAt + 60000 } })
  expect(wrapper.get('[role="progressbar"]').attributes('aria-valuenow')).toBe('41')
  expect(wrapper.text()).toContain('已合成 00:41 / 01:40')
  expect(wrapper.text()).toContain('1.25×')
  expect(wrapper.text()).toContain('2 秒前')
  expect(wrapper.get('[aria-label="渲染步骤"] [aria-current="step"]').text()).toContain('合成字幕')
  expect(wrapper.text()).not.toContain('成片和字幕已保存')
})

it('reports subtitle preparation and audio muxing in their own units and resets phase numbers', async () => {
  const wrapper = render({ ...job, stage: 'rendering', progress: { ...progress, stage: 'rendering', phase: 'preparing_subtitles', unit: 'cues', completed: 25, total: 100 } })
  expect(wrapper.get('[role="progressbar"]').attributes('aria-valuenow')).toBe('25')
  expect(wrapper.text()).toContain('已准备 25 / 100 条字幕')
  await wrapper.setProps({ job: { ...job, stage: 'rendering', progress: { ...progress, stage: 'rendering', phase: 'muxing_audio', unit: 'seconds', completed: 50, total: 100, speed: 2 } } })
  expect(wrapper.get('[role="progressbar"]').attributes('aria-valuenow')).toBe('50')
  expect(wrapper.text()).toContain('已合入 00:50 / 01:40')
  expect(wrapper.get('[aria-label="渲染步骤"] [aria-current="step"]').text()).toContain('合入原声')
  await wrapper.setProps({ job: { ...job, stage: 'validating' } })
  expect(wrapper.get('[role="progressbar"]').attributes('aria-valuenow')).toBeUndefined()
  expect(wrapper.find('.render-speed').exists()).toBe(false)
})

it('shows full-source loudness processing for both previews and complete videos', async () => {
  const wrapper = render({ ...job, stage: 'rendering' })
  for (const stage of ['rendering', 'previewing']) {
    for (const [phase, verb, title] of [['measuring_audio', '分析', '正在分析整段音量'], ['normalizing_audio', '调整', '正在调整人声音量'], ['checking_audio', '核对', '正在核对输出音量']] as const) {
      await wrapper.setProps({ job: { ...job, stage, progress: { ...progress, stage, phase, completed: 50, total: 100, unit: 'seconds' } } })
      expect(wrapper.text()).toContain(title)
      expect(wrapper.text()).toContain(`已${verb} 00:50 / 01:40`)
      expect(wrapper.get('[role="progressbar"]').attributes('aria-valuenow')).toBe('50')
      expect(wrapper.get('[aria-label="渲染步骤"] [aria-current="step"]').text()).toContain('合入原声')
      expect(wrapper.text()).not.toContain('成片和字幕已保存')
    }
  }
})

it('keeps a stalled snapshot honest and suspends decorative motion on sync errors', async () => {
  vi.useFakeTimers()
  vi.setSystemTime(startedAt + 60000)
  const wrapper = render({ ...job, stage: 'rendering', progress: { ...progress, stage: 'rendering', phase: 'rendering_subtitles', unit: 'seconds', completed: 20, total: 100, updatedAt: startedAt + 59000 } })
  await vi.advanceTimersByTimeAsync(20000)
  expect(wrapper.get('[role="progressbar"]').attributes('aria-valuenow')).toBe('20')
  expect(wrapper.text()).toContain('21 秒前')
  expect(wrapper.text()).toContain('等待新的进度反馈')
  await wrapper.setProps({ syncError: 'disconnected' })
  expect(wrapper.get('.creation-progress').classes()).not.toContain('is-animating')
  expect(wrapper.text()).toContain('当前展示上次读取的进度')
})

it('clears the elapsed timer when the panel unmounts', () => {
  vi.useFakeTimers()
  const wrapper = render()
  expect(vi.getTimerCount()).toBe(1)
  wrapper.unmount()
  expect(vi.getTimerCount()).toBe(0)
  mounted.splice(mounted.indexOf(wrapper), 1)
})
