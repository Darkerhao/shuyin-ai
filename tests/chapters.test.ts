import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { flushPromises, mount, type VueWrapper } from '@vue/test-utils'
import { createMemoryHistory, createRouter } from 'vue-router'
import ProjectChapters from '../src/modules/projects/ProjectChapters.vue'
import ProjectNavigation from '../src/components/ProjectNavigation.vue'
import * as projects from '../src/services/projects'
import * as drafts from '../src/services/drafts'
import * as workflow from '../src/services/workflow'
import { emptyChapter, type Chapter } from '../src/types/chapter'

vi.mock('@tauri-apps/api/core', () => ({ isTauri: () => true, invoke: vi.fn() }))
vi.mock('../src/services/projects', () => ({ isDesktop: () => true, getChapter: vi.fn(), listChapters: vi.fn(), saveChapter: vi.fn() }))
vi.mock('../src/services/drafts', () => ({ readDraft: vi.fn(), saveDraft: vi.fn() }))
vi.mock('../src/services/workflow', async original => ({ ...await original<typeof workflow>(), projectProgress: vi.fn() }))
vi.mock('../src/composables/useWindowCloseGuard', () => ({ useWindowCloseGuard: () => ({ error: { value: '' } }) }))

const chapter = (id: string): Chapter => ({ ...emptyChapter(), id, projectId: 'book', title: `${id}章`, content: `${id}章原文`, targetDurationSeconds: 300, audioDurationSource: 'manual', createdAt: '2026-11-13T01:00:00Z', updatedAt: '2026-11-13T02:00:00Z' })
const wrappers: VueWrapper[] = []
beforeEach(() => {
  vi.resetAllMocks()
  vi.mocked(projects.getChapter).mockImplementation(async (_project, id) => chapter(id))
  vi.mocked(projects.listChapters).mockResolvedValue([chapter('one'), chapter('two')])
  vi.mocked(projects.saveChapter).mockImplementation(async (_project, id, input) => ({ ...chapter(id ?? 'new-id'), ...input }))
  vi.mocked(drafts.readDraft).mockResolvedValue(null)
  vi.mocked(drafts.saveDraft).mockResolvedValue(undefined)
  vi.mocked(workflow.projectProgress).mockResolvedValue([])
})
afterEach(() => { wrappers.splice(0).forEach(wrapper => wrapper.unmount()); document.body.innerHTML = '' })

async function editor(path = '/projects/book?chapter=one') {
  const router = createRouter({ history: createMemoryHistory(), routes: [
    { path: '/projects/:id', component: ProjectChapters, props: route => ({ projectId: route.params.id, chapterId: route.query.chapter }) },
    { path: '/projects/:id/writing', component: { template: '<p>内容创作</p>' } },
  ] })
  await router.push(path); await router.isReady()
  const wrapper = mount({ template: '<RouterView v-slot="{ Component, route }"><component :is="Component" :key="route.query.chapter" /></RouterView>' }, { global: { plugins: [router] } })
  wrappers.push(wrapper); await flushPromises()
  return { wrapper, router }
}

it('saves a chapter without rewriting its book and keeps the complete workflow visible', async () => {
  const { wrapper } = await editor('/projects/book?chapter=new')
  await wrapper.get('#chapter-title').setValue('第三章：论证的细节')
  await wrapper.get('#chapter-content').setValue('本章完整正文\n需要逐条解释的论证。')
  expect(wrapper.find('#chapter-duration').exists()).toBe(false)
  await wrapper.get('form').trigger('submit'); await flushPromises()
  expect(projects.saveChapter).toHaveBeenCalledExactlyOnceWith('book', null, { title: '第三章：论证的细节', content: '本章完整正文\n需要逐条解释的论证。' })
  expect(drafts.saveDraft).toHaveBeenCalledWith('book:chapter:new', 'project', expect.objectContaining({ content: expect.objectContaining({ title: '第三章：论证的细节' }) }))
  expect(wrapper.text()).toContain('每章独立制作文案、音频、三尺寸封面和视频，并继续发布准备')
})

it('blocks switching chapters when saving the current draft fails', async () => {
  const { wrapper, router } = await editor()
  await wrapper.get('#chapter-content').setValue('第一章未保存的精讲资料')
  vi.mocked(drafts.saveDraft).mockRejectedValueOnce(new Error('磁盘暂不可写'))
  await router.push('/projects/book?chapter=two'); await flushPromises()
  expect(router.currentRoute.value.query.chapter).toBe('one')
  expect(wrapper.get('#chapter-content').element).toHaveProperty('value', '第一章未保存的精讲资料')
  expect(wrapper.text()).toContain('自动暂存失败')
  await router.push('/projects/book?chapter=two'); await flushPromises()
  expect(router.currentRoute.value.query.chapter).toBe('two')
  expect(wrapper.get('#chapter-content').element).toHaveProperty('value', 'two章原文')
  expect(drafts.saveDraft).toHaveBeenCalledWith('book:chapter:one', 'project', expect.objectContaining({ content: expect.objectContaining({ content: '第一章未保存的精讲资料' }) }))
})

it('restores chapter text from legacy drafts without submitting a retired duration', async () => {
  vi.mocked(drafts.readDraft).mockResolvedValue({ basedOn: 'older-version', content: { title: '恢复的第一章', content: '恢复正文', targetDurationSeconds: 1200 } })
  const { wrapper } = await editor()
  expect(wrapper.find('#chapter-duration').exists()).toBe(false)
  expect(wrapper.get('#chapter-content').element).toHaveProperty('value', '恢复正文')
  await wrapper.get('form').trigger('submit'); await flushPromises()
  expect(projects.saveChapter).toHaveBeenCalledWith('book', 'one', { title: '恢复的第一章', content: '恢复正文' })
})

it('keeps the selected chapter across all five steps and clears old output ids when changing scope', async () => {
  const router = createRouter({ history: createMemoryHistory(), routes: [{ path: '/projects/:id/:step?', component: { template: '<p>工作台</p>' } }] })
  await router.push('/projects/book/covers?chapter=one&coverVersion=old-cover&writingVersion=old-writing&videoJob=old-video&assist=1'); await router.isReady()
  const wrapper = mount(ProjectNavigation, { props: { id: 'book' }, global: { plugins: [router] } })
  wrappers.push(wrapper); await flushPromises()
  expect(workflow.projectProgress).toHaveBeenCalledWith('book', 'one')
  const links = wrapper.findAll('.project-navigation a')
  expect(links).toHaveLength(5)
  for (const link of links) expect(link.attributes('href')).toContain('chapter=one')
  wrapper.getComponent({ name: 'UiSelect' }).vm.$emit('update:modelValue', 'two'); await flushPromises()
  expect(router.currentRoute.value.path).toBe('/projects/book/covers')
  expect(router.currentRoute.value.query).toEqual({ chapter: 'two' })
  wrapper.getComponent({ name: 'UiSelect' }).vm.$emit('update:modelValue', 'book'); await flushPromises()
  expect(router.currentRoute.value.query).toEqual({})
})

it('routes resumed chapter tasks to their own workspace and exact output', () => {
  const task = { projectId: 'book', chapterId: 'one', title: '第一章', kind: 'audio', taskId: 'audio-one', status: 'paused', stage: 'generating', updatedAt: '', running: false }
  expect(workflow.resumeRoute(task)).toEqual({ path: '/projects/book/writing', query: { chapter: 'one', tab: 'audio', audioJob: 'audio-one' } })
  expect(workflow.resumeRoute({ ...task, kind: 'publishing', taskId: 'publish-one' })).toEqual({ path: '/projects/book/publishing', query: { chapter: 'one', task: 'publish-one' } })
  expect(workflow.resumeRoute({ ...task, kind: 'video', taskId: 'video-one' })).toEqual({ path: '/projects/book/video', query: { chapter: 'one', job: 'video-one' } })
  expect(workflow.resumeRoute({ ...task, kind: 'video', taskId: 'scene-task', stage: 'video_scene_revision' })).toEqual({ path: '/projects/book/video', query: { chapter: 'one' } })
})
