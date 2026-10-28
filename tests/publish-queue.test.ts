import { defineComponent, h } from 'vue'
import AppNotifications from '../src/components/AppNotifications.vue'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { RouterView, createMemoryHistory, createRouter } from 'vue-router'
import PublishQueuePage from '../src/modules/publishing/PublishQueuePage.vue'
import { listPublishQueue } from '../src/services/publishing'
import { isDesktop } from '../src/services/projects'
import type { PublishQueueItem } from '../src/types/publishing'

vi.mock('../src/services/publishing', () => ({ listPublishQueue: vi.fn() }))
vi.mock('../src/services/projects', () => ({ isDesktop: vi.fn() }))
const ready: PublishQueueItem = { id: 'older', projectId: 'book-one', projectTitle: '第一本书', writingVersionId: 'writing-one', title: '第一篇标题', description: '发布内容', tags: '', videoAssetId: 'video-one', audioAssetId: 'audio-one', landscapeCoverId: 'cover-one', portraitCoverId: 'cover-two', titleConfirmed: true, coversConfirmed: true, status: 'ready', taskDirectory: 'D:/任务包', createdAt: '2026-10-24T00:00:00Z', updatedAt: '2026-10-24T00:00:00Z', checklist: [], record: null }
const tasks: PublishQueueItem[] = [ready, { ...ready, id: 'second', projectId: 'book-two', projectTitle: '第二本书', title: '第二篇标题' }, { ...ready, id: 'draft', status: 'draft', title: '待完善', checklist: [{ key: 'video', label: '视频已关联且文件可用', passed: false }] }, { ...ready, id: 'published', status: 'published', title: '已完成', record: { publishedAt: '2026-10-25T00:00:00Z', postUrl: 'https://www.douyin.com/video/123', notes: '人工确认的记录' } }]
const wrappers: ReturnType<typeof mount>[] = []
async function render() {
  const router = createRouter({ history: createMemoryHistory(), routes: [{ path: '/publishing', component: PublishQueuePage }, { path: '/projects', component: { template: '<p>书籍</p>' } }, { path: '/projects/:id/publishing', component: { template: '<p>处理任务</p>' } }] })
  await router.push('/publishing'); await router.isReady()
  const wrapper = mount(defineComponent({ setup: () => () => [h(AppNotifications), h(RouterView)] }), { global: { plugins: [router] } }); wrappers.push(wrapper)
  await flushPromises()
  return { wrapper, router }
}
const button = (wrapper: ReturnType<typeof mount>, text: string) => wrapper.findAll('button').find(button => button.text().startsWith(text))!
beforeEach(() => { vi.resetAllMocks(); vi.mocked(isDesktop).mockReturnValue(true); vi.mocked(listPublishQueue).mockResolvedValue(tasks) })
afterEach(() => { wrappers.splice(0).forEach(wrapper => wrapper.unmount()); vi.unstubAllGlobals() })

describe('daily publication queue', () => {
  it('opens the exact saved task across books and never begins uploading from the list', async () => {
    const { wrapper, router } = await render()
    expect(wrapper.findAll('article').map(article => article.get('h2').text())).toEqual(['第一篇标题', '第二篇标题'])
    const links = wrapper.findAll('a').filter(link => link.text() === '处理这篇')
    expect(links[0]!.attributes('href')).toBe('/projects/book-one/publishing?task=older')
    expect(links[1]!.attributes('href')).toBe('/projects/book-two/publishing?task=second')
    await links[1]!.trigger('click'); await flushPromises()
    expect(router.currentRoute.value.query.task).toBe('second')
    expect(router.currentRoute.value.params.id).toBe('book-two')
    expect(listPublishQueue).toHaveBeenCalledOnce()
  })
  it('shows missing requirements and archived links without treating a task as published', async () => {
    const writeText = vi.fn().mockResolvedValue(undefined)
    vi.stubGlobal('navigator', { clipboard: { writeText } })
    const { wrapper } = await render()
    await button(wrapper, '草稿').trigger('click'); await flushPromises()
    expect(wrapper.text()).toContain('待完成：视频已关联且文件可用')
    await button(wrapper, '已发布').trigger('click'); await flushPromises()
    expect(wrapper.findAll('article')).toHaveLength(1)
    expect(wrapper.text()).toContain('人工确认的记录')
    await button(wrapper, '复制作品链接').trigger('click'); await flushPromises()
    expect(writeText).toHaveBeenCalledWith('https://www.douyin.com/video/123')
    expect(wrapper.text()).toContain('作品链接已复制')
    await wrapper.get('#publish-search').setValue('不存在'); await flushPromises()
    expect(wrapper.text()).toContain('没有匹配的任务')
  })
  it('retains the current list after refresh failure and accepts a successful retry', async () => {
    const { wrapper } = await render()
    vi.mocked(listPublishQueue).mockRejectedValueOnce('读取失败')
    await button(wrapper, '刷新任务').trigger('click'); await flushPromises()
    expect(wrapper.text()).toContain('读取失败')
    expect(wrapper.findAll('article')).toHaveLength(2)
    vi.mocked(listPublishQueue).mockResolvedValue([])
    await button(wrapper, '刷新任务').trigger('click'); await flushPromises()
    expect(wrapper.text()).toContain('作品就绪后，在这里开始发布')
    expect(wrapper.findAll('article')).toHaveLength(0)
    await button(wrapper, '查看发布草稿').trigger('click'); await flushPromises()
    expect(wrapper.text()).toContain('还没有发布草稿')
    expect(wrapper.get('.empty-state a').attributes('href')).toBe('/projects')
  })
  it('clears a failed search with the clear control or Escape without reloading tasks', async () => {
    const { wrapper } = await render()
    await wrapper.get('#publish-search').setValue('不存在'); await flushPromises()
    expect(wrapper.findAll('article')).toHaveLength(0)
    await wrapper.get('[aria-label="清空任务搜索"]').trigger('click'); await flushPromises()
    expect(wrapper.findAll('article')).toHaveLength(2)
    await wrapper.get('#publish-search').setValue('第二本'); await flushPromises()
    expect(wrapper.findAll('article')).toHaveLength(1)
    await wrapper.get('#publish-search').trigger('keydown', { key: 'Escape' }); await flushPromises()
    expect(wrapper.findAll('article')).toHaveLength(2)
    expect(listPublishQueue).toHaveBeenCalledOnce()
  })
  it('does not access local tasks from browser preview', async () => {
    vi.mocked(isDesktop).mockReturnValue(false)
    const { wrapper } = await render()
    expect(listPublishQueue).not.toHaveBeenCalled()
    expect(wrapper.text()).toContain('请在桌面应用中查看发布任务')
    expect(button(wrapper, '刷新任务').attributes()).toHaveProperty('disabled')
  })
  it('restores search and status after opening a task and returning', async () => {
    const { wrapper, router } = await render()
    await button(wrapper, '草稿').trigger('click'); await flushPromises()
    await wrapper.get('#publish-search').setValue('待完善'); await flushPromises()
    expect(router.currentRoute.value.query).toEqual({ status: 'draft', q: '待完善' })
    await wrapper.get('article a').trigger('click'); await flushPromises()
    expect(wrapper.find('#publish-search').exists()).toBe(false)
    router.back(); await flushPromises()
    expect((wrapper.get('#publish-search').element as HTMLInputElement).value).toBe('待完善')
    expect(wrapper.findAll('article')).toHaveLength(1)
    expect(wrapper.get('article h2').text()).toBe('待完善')
  })
})
