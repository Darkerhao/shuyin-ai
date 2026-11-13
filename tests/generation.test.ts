import { beforeEach, expect, it, vi } from 'vitest'
import { flushPromises } from '@vue/test-utils'
import * as ai from '../src/services/ai'
import * as covers from '../src/services/covers'
import { generationJobs, generationKey, startWritingGeneration, startBackgroundGeneration, cancelGenerationJob, isGenerating } from '../src/services/generation'
import { emptyPublishing, type GenerationResult, type GenerationTextPreview, type WritingVersion } from '../src/types/ai'
vi.mock('../src/services/ai', () => ({ startGeneration: vi.fn(), executeGeneration: vi.fn(), cancelGeneration: vi.fn() }))
vi.mock('../src/services/covers', () => ({ startCoverGeneration: vi.fn(), executeCoverGeneration: vi.fn() }))
beforeEach(() => { generationJobs.clear(); vi.resetAllMocks() })
const version: WritingVersion = { id: 'v1', projectId: 'book', kind: 'publishing', versionNo: 1, content: emptyPublishing(), sourceType: 'ai', modelName: '', promptVersion: '', isConfirmed: false, createdAt: '' }

it('keeps per-book requests independent and refuses a duplicate active request', async () => {
  const finish: ((result: GenerationResult) => void)[] = []
  vi.mocked(ai.startGeneration).mockResolvedValueOnce('task-1').mockResolvedValueOnce('task-2')
  vi.mocked(ai.executeGeneration).mockImplementation(() => new Promise(resolve => finish.push(resolve)))
  await startWritingGeneration({ projectId: 'one', providerId: 'model' }, '第一本')
  await expect(startWritingGeneration({ projectId: 'one', providerId: 'model' }, '第一本')).rejects.toThrow('仍在进行')
  await startWritingGeneration({ projectId: 'two', providerId: 'model' }, '第二本')
  finish[0]!(version); await flushPromises()
  expect(generationJobs.get(generationKey('one', 'writing'))?.result).toEqual(version)
  expect(isGenerating(generationJobs.get(generationKey('two', 'writing')))).toBe(true)
  expect(ai.executeGeneration).toHaveBeenCalledTimes(2)
  finish[1]!(version); await flushPromises()
})

it('cancels after remount without retaining late fragments', async () => {
  let emit!: (value: GenerationTextPreview) => void
  let fail!: (cause: unknown) => void
  vi.mocked(ai.startGeneration).mockResolvedValue('task')
  vi.mocked(ai.executeGeneration).mockImplementation((_id, observer) => { emit = observer!; return new Promise((_resolve, reject) => { fail = reject }) })
  vi.mocked(ai.cancelGeneration).mockResolvedValue(undefined)
  await startWritingGeneration({ projectId: 'book', providerId: 'model' }, '书')
  const job = generationJobs.get(generationKey('book', 'writing'))!
  emit({ title: '正在接收' })
  expect(job.preview.title).toBe('正在接收')
  await cancelGenerationJob(job)
  emit({ title: '迟到片段' }); fail({ code: 'cancelled', message: '取消成功' }); await flushPromises()
  expect(job.status).toBe('cancelled')
  expect(job.preview).toEqual({})
  expect(job.result).toBeUndefined()
})

it('preserves a valid result when backend completion wins the cancellation race', async () => {
  let finish!: (value: GenerationResult) => void
  vi.mocked(ai.startGeneration).mockResolvedValue('task')
  vi.mocked(ai.executeGeneration).mockImplementation(() => new Promise(resolve => { finish = resolve }))
  vi.mocked(ai.cancelGeneration).mockResolvedValue(undefined)
  await startWritingGeneration({ projectId: 'book', providerId: 'model' }, '书')
  const job = generationJobs.get(generationKey('book', 'writing'))!
  await cancelGenerationJob(job)
  finish(version); await flushPromises()
  expect(job.status).toBe('succeeded')
  expect(job.result).toEqual(version)
})

it('retains an image candidate and the requested size without applying it to an editor', async () => {
  const asset = { id: 'image', assetType: 'image' } as Awaited<ReturnType<typeof covers.executeCoverGeneration>>
  vi.mocked(covers.startCoverGeneration).mockResolvedValue('image-task')
  vi.mocked(covers.executeCoverGeneration).mockResolvedValue(asset)
  await startBackgroundGeneration('book', '书', 'model', '山水', 1080, 1440); await flushPromises()
  const job = generationJobs.get(generationKey('book', 'cover'))!
  expect(job.size).toEqual({ width: 1080, height: 1440 })
  expect(job.result).toEqual(asset)
  expect(job.status).toBe('succeeded')
})

it('keeps simultaneous chapters of the same book separate when results arrive out of order', async () => {
  const finish = new Map<string, (result: GenerationResult) => void>()
  vi.mocked(ai.startGeneration).mockImplementation(async input => `task-${input.chapterId ?? 'book'}`)
  vi.mocked(ai.executeGeneration).mockImplementation(id => new Promise(resolve => finish.set(id, resolve)))
  for (const chapterId of [null, 'chapter-one', 'chapter-two']) {
    await startWritingGeneration({ projectId: 'book', providerId: 'model', chapterId }, chapterId ?? '整本书')
  }
  finish.get('task-chapter-two')!({ ...version, id: 'v2', chapterId: 'chapter-two' }); await flushPromises()
  expect(generationJobs.get(generationKey('book', 'writing', 'chapter-two'))?.result).toMatchObject({ id: 'v2', chapterId: 'chapter-two' })
  expect(isGenerating(generationJobs.get(generationKey('book', 'writing', 'chapter-one')))).toBe(true)
  expect(isGenerating(generationJobs.get(generationKey('book', 'writing')))).toBe(true)
  finish.get('task-chapter-one')!({ ...version, id: 'v1', chapterId: 'chapter-one' })
  finish.get('task-book')!(version); await flushPromises()
  expect(generationJobs.size).toBe(3)
  expect(generationJobs.get(generationKey('book', 'writing', 'chapter-one'))?.result).toMatchObject({ chapterId: 'chapter-one' })
})
