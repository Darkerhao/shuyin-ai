import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { flushPromises, mount, type VueWrapper } from '@vue/test-utils'
import VideoImageEditor from '../src/modules/video/VideoImageEditor.vue'
import ConfirmAction from '../src/components/ConfirmAction.vue'
import OperationFeedback from '../src/components/OperationFeedback.vue'
import * as api from '../src/services/video'
import { cancelGeneration } from '../src/services/ai'
import type { ModelProvider } from '../src/types/ai'
import type { VideoSceneImage } from '../src/types/video'
vi.mock('../src/services/video', () => ({ chooseImage: vi.fn(), importImage: vi.fn(), readImage: vi.fn(), startIllustration: vi.fn(), executeIllustration: vi.fn() }))
vi.mock('../src/services/ai', () => ({ cancelGeneration: vi.fn() }))
const model: ModelProvider = { id: 'image-model', name: '插画模型', modelName: 'image-1', modelKind: 'image', baseUrl: 'http://localhost', reasoningEffort: null, isDefault: true, isEnabled: true, credentialStatus: 'available' }
const image: VideoSceneImage = { assetId: 'old', source: '本人拍摄', rights: '自有照片', fit: 'contain', x: .5, y: .5 }
const wrappers: VueWrapper[] = []
beforeEach(() => {
  vi.resetAllMocks()
  vi.stubGlobal('URL', class extends URL { static createObjectURL = vi.fn(() => 'blob:video-image'); static revokeObjectURL = vi.fn() })
  vi.mocked(api.readImage).mockResolvedValue(new ArrayBuffer(4))
  vi.mocked(api.startIllustration).mockResolvedValue('task-1')
  vi.mocked(api.executeIllustration).mockResolvedValue({ id: 'generated', projectId: 'book', assetType: 'image', filePath: 'D:/generated.png', originalName: '视频插画.png', fileSize: 4, modifiedAt: 1, existsStatus: 'available', createdAt: '' })
})
afterEach(() => { wrappers.splice(0).forEach(w => w.unmount()); document.body.innerHTML = ''; vi.unstubAllGlobals() })
async function render() {
  const wrapper = mount(VideoImageEditor, { attachTo: document.body, props: { projectId: 'book', sceneIndex: 0, image: { ...image }, assets: [], providers: [model], disabled: false } })
  wrappers.push(wrapper); await flushPromises(); return wrapper
}
async function click(wrapper: VueWrapper, text: string) {
  const button = wrapper.findAll('button').find(b => b.text() === text)
  expect(button, text).toBeDefined(); await button!.trigger('click'); await flushPromises()
}
async function openGeneration(wrapper: VueWrapper) {
  await click(wrapper, '主动生成一张关键插画')
  await wrapper.get('#video-image-0-description').setValue('窗边一本打开的书，暖白纸底，不要文字')
  await click(wrapper, '生成前确认')
}
it('requires a model and single-image cost confirmation, then keeps the result separate until adoption', async () => {
  const wrapper = await render()
  await openGeneration(wrapper)
  expect(document.body.textContent).toContain('插画模型 / image-1')
  expect(document.body.textContent).toContain('不上传音频或图片')
  expect(api.startIllustration).not.toHaveBeenCalled()
  wrapper.getComponent(ConfirmAction).vm.$emit('confirm'); await flushPromises()
  expect(api.startIllustration).toHaveBeenCalledExactlyOnceWith('book', 'image-model', '窗边一本打开的书，暖白纸底，不要文字')
  expect(api.executeIllustration).toHaveBeenCalledExactlyOnceWith('task-1')
  expect(wrapper.emitted('update:image')).toBeUndefined()
  expect(wrapper.text()).toContain('尚未用于本镜')
  await click(wrapper, '采用此插画')
  expect(wrapper.emitted('update:image')?.at(-1)?.[0]).toEqual(expect.objectContaining({ assetId: 'generated', source: 'AI插画 · 插画模型 / image-1', rights: '' }))
  expect(image.assetId).toBe('old')
})
it('sends no generation request when the confirmation is cancelled', async () => {
  const wrapper = await render(); await openGeneration(wrapper)
  wrapper.getComponent(ConfirmAction).vm.$emit('cancel'); await flushPromises()
  expect(api.startIllustration).not.toHaveBeenCalled()
  expect(wrapper.emitted('update:image')).toBeUndefined()
})
it('keeps the existing image and usage metadata when generation fails', async () => {
  vi.mocked(api.executeIllustration).mockRejectedValue(new Error('图片服务失败'))
  const wrapper = await render(); await openGeneration(wrapper)
  wrapper.getComponent(ConfirmAction).vm.$emit('confirm'); await flushPromises()
  expect(wrapper.getComponent(OperationFeedback).props('error')).toContain('图片服务失败')
  expect(wrapper.emitted('update:image')).toBeUndefined()
  expect(api.startIllustration).toHaveBeenCalledTimes(1)
})
it('uses the existing cancellation task and never adopts a late cancelled result', async () => {
  let reject: (reason: Error) => void = () => {}
  vi.mocked(api.executeIllustration).mockImplementation(() => new Promise((_resolve, fail) => { reject = fail }))
  vi.mocked(cancelGeneration).mockImplementation(async () => { reject(new Error('生成已取消')) })
  const wrapper = await render(); await openGeneration(wrapper)
  wrapper.getComponent(ConfirmAction).vm.$emit('confirm'); await flushPromises()
  await click(wrapper, '取消插画生成')
  expect(cancelGeneration).toHaveBeenCalledExactlyOnceWith('task-1')
  expect(wrapper.emitted('update:image')).toBeUndefined()
  expect(wrapper.getComponent(OperationFeedback).props('error')).toContain('生成已取消')
})
it('imports a validated book image and requires fresh usage information without touching the old image', async () => {
  vi.mocked(api.chooseImage).mockResolvedValue('D:/中文图片.png')
  vi.mocked(api.importImage).mockResolvedValue('imported')
  const wrapper = await render(); await click(wrapper, '导入本书图片')
  expect(api.importImage).toHaveBeenCalledExactlyOnceWith('book', 'D:/中文图片.png')
  expect(wrapper.emitted('update:image')?.at(-1)?.[0]).toEqual(expect.objectContaining({ assetId: 'imported', source: '', rights: '', fit: 'contain' }))
})
