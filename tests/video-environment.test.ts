import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import VideoEnvironmentCheck from '../src/modules/video/VideoEnvironmentCheck.vue'
import { checkEnvironment } from '../src/services/video'
import { chooseSelect } from './ui'
import type { VideoEnvironmentReport } from '../src/types/video'
vi.mock('../src/services/video', () => ({ checkEnvironment: vi.fn() }))
const settings = { engineRoot: 'D:/explicit', nodePath: '', pythonPath: '', modelPath: '' }
const result: VideoEnvironmentReport = { settings: { ...settings, nodePath: 'D:/node/node.exe' }, items: [
  { id: 'node', status: 'error', version: '24.11.0', path: 'D:/node/node.exe', detail: '中文路径版本缺陷' },
  { id: 'python', status: 'skipped', version: '', path: '', detail: '已有SRT不需要' },
] }
let wrapper: ReturnType<typeof mount>
beforeEach(() => { vi.resetAllMocks(); vi.mocked(checkEnvironment).mockResolvedValue(structuredClone(result)) })
afterEach(() => wrapper?.unmount())
function render(hasSubtitles = true) { wrapper = mount(VideoEnvironmentCheck, { props: { settings, hasSubtitles, disabled: false } }); return wrapper }
it('checks the selected usage, reports missing items and only applies detected paths explicitly', async () => {
  const page = render()
  await page.findAll('button').find(b => b.text() === '一键检查环境')!.trigger('click'); await flushPromises()
  expect(checkEnvironment).toHaveBeenCalledWith(settings, false)
  expect(page.text()).toContain('有 1 项需要处理')
  expect(page.text()).toContain('不需要')
  expect(page.emitted('apply')).toBeUndefined()
  await page.findAll('button').find(b => b.text() === '填入识别到的路径')!.trigger('click')
  expect(page.emitted('apply')![0]).toEqual([{ ...settings, nodePath: 'D:/node/node.exe' }])
  await page.findAll('button').find(b => b.text() === '重新选择Node 程序')!.trigger('click')
  expect(page.emitted('choose')![0]).toEqual(['nodePath'])
  await page.setProps({ settings: { ...settings, nodePath: 'D:/fixed.exe' } })
  expect(page.find('.environment-results').exists()).toBe(false)
})
it('requires Python checks for new transcription and invalidates the report after changing usage', async () => {
  const page = render(false)
  await page.findAll('button').find(b => b.text() === '一键检查环境')!.trigger('click'); await flushPromises()
  expect(checkEnvironment).toHaveBeenCalledWith(settings, true)
  await chooseSelect(page, '#environment-mode', '使用已有 SRT / 已保存字幕')
  expect(page.find('.environment-results').exists()).toBe(false)
})
it('keeps settings unchanged when the check fails and releases the busy state for retry', async () => {
  vi.mocked(checkEnvironment).mockRejectedValue(new Error('无法启动 Node'))
  const page = render()
  await page.findAll('button').find(b => b.text() === '一键检查环境')!.trigger('click'); await flushPromises()
  expect(page.get('[role="alert"]').text()).toContain('无法启动 Node')
  expect(page.emitted('working')).toEqual([[true], [false]])
  expect(page.emitted('apply')).toBeUndefined()
})
