import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import VideoSceneRevision from '../src/modules/video/VideoSceneRevision.vue'
import * as api from '../src/services/video'
import { cancelGeneration } from '../src/services/ai'
import type { VideoSceneRevisionInput, VideoScene } from '../src/types/video'
import { emptyProvider } from '../src/types/ai'
import { chooseSelect, dialog } from './ui'

vi.mock('../src/services/video', () => ({ startSceneRevision: vi.fn(), executeSceneRevision: vi.fn() }))
vi.mock('../src/services/ai', () => ({ cancelGeneration: vi.fn() }))
const input: VideoSceneRevisionInput = { jobId: 'job', runId: 2, providerId: 'm1', sceneIndex: 0, instruction: '', subtitles: '1\n00:00:00,000 --> 00:00:03,000\n本镜字幕\n', scenes: [{ first: 0, last: 0, kind: 'focus', title: '原观点', beats: [{ cue: 0, object: 'book', action: 'draw', label: '阅读' }] }] }
const candidate: VideoScene = { ...input.scenes[0]!, title: '新观点', beats: [{ cue: 0, object: 'pen', action: 'draw', label: '思考' }] }
let wrapper: ReturnType<typeof mount>
beforeEach(() => {
  vi.resetAllMocks()
  vi.mocked(api.startSceneRevision).mockResolvedValue('task')
  vi.mocked(api.executeSceneRevision).mockResolvedValue(structuredClone(candidate))
  vi.mocked(cancelGeneration).mockResolvedValue(undefined)
})
afterEach(() => { wrapper?.unmount(); document.body.innerHTML = '' })
async function render() {
  wrapper = mount(VideoSceneRevision, { attachTo: document.body, props: { input: structuredClone(input), providers: ['m1', 'm2'].map(id => ({ ...emptyProvider(), id, name: id, credentialStatus: 'available' as const })) } })
  await flushPromises(); return wrapper
}
async function generate() {
  await dialog().get('#scene-revision-instruction').setValue('改为钢笔表达思考')
  await dialog().findAll('button').find(b => b.text() === '生成候选分镜')!.trigger('click')
  await flushPromises()
}
it('sends one selected model request, previews before adoption and invalidates a candidate after changed instructions', async () => {
  await render()
  await chooseSelect(wrapper, '#scene-revision-provider', /^m2/)
  await generate()
  expect(api.startSceneRevision).toHaveBeenCalledExactlyOnceWith({ ...input, providerId: 'm2', instruction: '改为钢笔表达思考' })
  expect(dialog().get('[aria-label="当前分镜"]').text()).toContain('原观点')
  expect(dialog().get('[aria-label="候选分镜"]').text()).toContain('新观点')
  expect(wrapper.emitted('adopt')).toBeUndefined()
  await dialog().findAll('button').find(b => b.text() === '采用这一镜')!.trigger('click')
  expect(wrapper.emitted('adopt')![0]).toEqual([candidate])
  await dialog().get('#scene-revision-instruction').setValue('换个要求')
  expect(dialog().find('[aria-label="候选分镜"]').exists()).toBe(false)
})
it('discards candidates without changing input or emitting adoption', async () => {
  await render(); await generate()
  await dialog().findAll('button').find(b => b.text() === '放弃候选')!.trigger('click'); await flushPromises()
  expect(wrapper.emitted('close')).toHaveLength(1)
  expect(wrapper.emitted('adopt')).toBeUndefined()
  expect(wrapper.props()).toHaveProperty('input', input)
})
it('preserves original content after a model failure and allows an explicit retry', async () => {
  vi.mocked(api.executeSceneRevision).mockRejectedValueOnce({ message: '模型返回了两镜，已拒绝' })
  await render(); await generate()
  expect(dialog().get('[role="alert"]').text()).toContain('模型返回了两镜')
  expect(dialog().find('[aria-label="候选分镜"]').exists()).toBe(false)
  expect(wrapper.emitted('adopt')).toBeUndefined()
  await dialog().findAll('button').find(b => b.text() === '生成候选分镜')!.trigger('click'); await flushPromises()
  expect(api.startSceneRevision).toHaveBeenCalledTimes(2)
  expect(dialog().find('[aria-label="候选分镜"]').exists()).toBe(true)
})
it('cancels the shared AI task and ignores a successful response arriving after stop', async () => {
  let finish!: (scene: VideoScene) => void
  vi.mocked(api.executeSceneRevision).mockReturnValueOnce(new Promise(resolve => { finish = resolve }))
  await render(); await generate()
  expect(dialog().get('#scene-revision-instruction').attributes('disabled')).toBeDefined()
  await dialog().findAll('button').find(b => b.text() === '停止单镜生成')!.trigger('click'); await flushPromises()
  expect(cancelGeneration).toHaveBeenCalledExactlyOnceWith('task')
  finish(candidate); await flushPromises()
  expect(dialog().find('[aria-label="候选分镜"]').exists()).toBe(false)
  expect(wrapper.emitted('adopt')).toBeUndefined()
  expect(dialog().text()).toContain('单镜生成已停止')
})
