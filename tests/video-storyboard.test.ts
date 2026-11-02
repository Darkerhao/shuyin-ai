import { expect, it } from 'vitest'
import { inspectStoryboard } from '../src/modules/video/storyboard'
import type { VideoPlan } from '../src/types/video'

it('points to repeated scenes without rewriting the storyboard or flagging a deliberate changed action', () => {
  const plan: VideoPlan = {
    title: '原书', audioSha256: 'source', duration: 24,
    cues: [0, 8, 16].map(start => ({ start, end: start + 7, text: '原声' })),
    scenes: [0, 1, 2].map(i => ({ first: i, last: i, kind: 'focus', title: '阅读', beats: [{ cue: i, object: 'book', action: 'draw', label: '书籍' }] })),
  }
  const original = structuredClone(plan)
  expect(inspectStoryboard(plan).map(n => [n.sceneIndex, n.code])).toEqual([[2, 'repetition']])
  expect(plan).toEqual(original)
  plan.scenes[2]!.environment = 'study'
  expect(inspectStoryboard(plan)).toEqual([])
  plan.scenes[2]!.environment = 'plain'
  expect(inspectStoryboard(plan).map(n => n.code)).toEqual(['repetition'])
  plan.scenes[2]!.framing = 'detail'
  expect(inspectStoryboard(plan)).toEqual([])
  plan.scenes[2]!.framing = 'reference'
  plan.scenes[2]!.beats![0]!.action = 'check'
  expect(inspectStoryboard(plan)).toEqual([])
})

it('warns about crowded separate cues while allowing simultaneous comparison and natural silence', () => {
  const plan: VideoPlan = {
    title: '原书', audioSha256: 'source', duration: 60,
    cues: [{ start: 20, end: 20.5, text: '前' }, { start: 20.7, end: 24, text: '后' }],
    scenes: [{ first: 0, last: 1, kind: 'compare', title: '对照', beats: [{ cue: 0, object: 'book', action: 'draw', label: '前' }, { cue: 1, object: 'pen', action: 'draw', label: '后' }] }],
  }
  expect(inspectStoryboard(plan).map(n => n.code)).toEqual(['crowding'])
  plan.scenes[0]!.beats![1]!.cue = 0
  expect(inspectStoryboard(plan)).toEqual([])
  plan.cues[1]!.end = 40
  expect(inspectStoryboard(plan).map(n => n.code)).toEqual(['long-hold'])
})
