import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createMemoryHistory, createRouter, RouterLink } from 'vue-router'
import AutomationTasks from '../src/components/AutomationTasks.vue'
import { automationRuns, automationError, type AutomationRun } from '../src/services/automation'

vi.mock('../src/services/projects', () => ({ isDesktop: () => false }))
vi.mock('../src/composables/useWindowCloseGuard', () => ({ useWindowCloseGuard: vi.fn() }))

const fixture = (overrides: Partial<AutomationRun> = {}): AutomationRun => ({
  id: 'run1', projectId: 'book1', input: { workflowVersion: 2, title: '原书', author: '', selection: { projectId: 'book1', source: { kind: 'asset', assetId: 'source1' }, textProviderId: 'text1', imageProviderId: 'image1', format: 'deep_dive', requestKey: 'key1' } },
  status: 'running', stage: 'audio', runId: 1, error: '', audioJobId: 'audio1', videoJobId: null, writingTaskId: 'writing1', writingVersionId: 'version1', finalWritingVersionId: null, actualDurationSeconds: null,
  durationTaskId: null, contentCheckTaskId: null, landscapeTaskId: null, portraitTaskId: null, widescreenTaskId: null, landscapeCoverId: null, portraitCoverId: null, widescreenCoverId: null, publishTaskId: null, createdAt: '', updatedAt: '', ...overrides,
})
const mounted: ReturnType<typeof mount>[] = []
beforeEach(() => { automationRuns.value = []; automationError.value = '' })
afterEach(() => { mounted.splice(0).forEach(wrapper => wrapper.unmount()) })
async function render(run: AutomationRun) {
  automationRuns.value = [run]
  const router = createRouter({ history: createMemoryHistory(), routes: [{ path: '/:pathMatch(.*)*', component: { template: '<div />' } }] })
  await router.push('/projects'); await router.isReady()
  const wrapper = mount(AutomationTasks, { global: { plugins: [router] } })
  mounted.push(wrapper)
  return wrapper
}
it('shows completed preceding stages without treating a created audio task as a completed audio result', async () => {
  const wrapper = await render(fixture())
  expect(wrapper.findAll('[data-state="done"]')).toHaveLength(1)
  expect(wrapper.get('[aria-current="step"]').text()).toBe('原声音频进行中')
  expect(wrapper.text()).not.toContain('实际音频')
  expect(wrapper.findAll('a').find(link => link.text() === '音频任务')?.attributes('href')).toBe('/projects/book1/writing?tab=audio&audioJob=audio1')
  expect(wrapper.findAll('a[href*="coverVersion="]')).toHaveLength(0)
})
it('announces automatic retries and displays the actual model after fallback', async () => {
  const run = fixture({ stage: 'storyboard', error: '已自动切换到备用模型，继续当前未完成步骤。' })
  run.input.textProvider = { name: '备用文本', modelName: 'fallback-text' }
  run.input.imageProvider = { name: '原图片', modelName: 'image-model' }
  const wrapper = await render(run)
  expect(wrapper.get('.automation-note[role="status"]').text()).toContain('已自动切换')
  expect(wrapper.get('.automation-summary').text()).toContain('备用文本 · fallback-text')
  expect(wrapper.get('.automation-summary').text()).toContain('原图片 · image-model')
})
it('preserves paused and failed phase feedback and only exposes saved cover versions', async () => {
  const wrapper = await render(fixture({ status: 'paused', stage: 'portrait_image', error: '图片生成失败，已有结果保留', actualDurationSeconds: 625, landscapeCoverId: 'cover1' }))
  expect(wrapper.get('[aria-current="step"]').text()).toBe('三尺寸封面已暂停')
  expect(wrapper.text()).toContain('实际音频 10 分 25 秒')
  expect(wrapper.text()).not.toContain('目标')
  expect(wrapper.findAll('a[href*="coverVersion="]')).toHaveLength(1)
  expect(wrapper.get('a[href="/projects/book1/covers?coverVersion=cover1"]').text()).toBe('4:3 成品封面')
  automationRuns.value = [fixture({ status: 'waiting_user', stage: 'portrait_image', error: '请检查模型设置' })]
  await flushPromises()
  expect(wrapper.get('[aria-current="step"]').text()).toBe('三尺寸封面待确认')
  expect(wrapper.text()).toContain('请检查模型设置')
})
it('keeps final publishing pending manual confirmation even when a publish task exists', async () => {
  const wrapper = await render(fixture({ status: 'awaiting_publish', stage: 'publishing', publishTaskId: 'publish1', videoJobId: 'video1' }))
  expect(wrapper.findAll('[data-state="done"]')).toHaveLength(5)
  expect(wrapper.get('[aria-current="step"]').text()).toBe('发布准备待确认')
  expect(wrapper.text()).toContain('等待最终发布')
  expect(wrapper.text()).not.toContain('已发布')
  expect(wrapper.get('a[href="/projects/book1/publishing?task=publish1"]').text()).toBe('发布详情与结果记录')
})
it('keeps separate stable output cards when checked copy reuses the original version', async () => {
  const wrapper = await render(fixture({ stage: 'portrait_image', finalWritingVersionId: 'version1' }))
  const links = wrapper.findAll('.automation-output')
  const original = links.find(link => link.text() === '本次创作方案')!
  const checked = links.find(link => link.text() === '已校核发布材料')!
  expect(original.attributes('href')).toBe(checked.attributes('href'))
  const keys = wrapper.findAllComponents(RouterLink).filter(link => link.classes().includes('automation-output')).map(link => link.vm.$.vnode.key)
  expect(new Set(keys).size).toBe(keys.length)
  automationRuns.value = [fixture({ stage: 'portrait_image', finalWritingVersionId: 'version2' })]
  await flushPromises()
  const updated = wrapper.findAll('.automation-output').find(link => link.text() === '已校核发布材料')!
  expect(updated.element).toBe(checked.element)
  expect(updated.attributes('href')).toBe('/projects/book1/writing?writingVersion=version2')
  expect(original.attributes('href')).toBe('/projects/book1/writing?writingVersion=version1')
})
it('does not project the new workflow sequence onto legacy or unknown stages', async () => {
  const original = fixture()
  const wrapper = await render({ ...original, input: { ...original.input, workflowVersion: 1 }, stage: 'video_prepare', status: 'paused' })
  expect(wrapper.find('[aria-label="自动制作阶段"]').exists()).toBe(false)
  expect(wrapper.text()).toContain('准备字幕与分镜（旧流程）')
  automationRuns.value = [fixture({ stage: 'unknown_stage' })]
  await flushPromises()
  expect(wrapper.find('[aria-label="自动制作阶段"]').exists()).toBe(false)
  expect(wrapper.text()).toContain('unknown_stage')
})

it('opens every saved output in the chapter that produced it', async () => {
  const run = fixture({ chapterId: 'chapter-one', videoJobId: 'video-one', landscapeCoverId: 'cover-one', portraitCoverId: 'cover-two', widescreenCoverId: 'cover-three', publishTaskId: 'publish-one' })
  run.input.chapterSource = { chapterTitle: '第一章：精讲' }
  const wrapper = await render(run)
  expect(wrapper.get('.automation-title').text()).toContain('第一章：精讲')
  for (const link of wrapper.findAll('a')) expect(link.attributes('href')).toContain('chapter=chapter-one')
  expect(wrapper.findAll('a[href*="coverVersion="]')).toHaveLength(3)
  expect(wrapper.get('a[href*="task=publish-one"]').text()).toContain('发布详情')
})
