import type { VideoPlan } from '../../types/video'

export interface StoryboardNote {
  sceneIndex: number
  code: 'repetition' | 'crowding' | 'late-subject' | 'long-hold'
  message: string
}

export function inspectStoryboard(plan: VideoPlan): StoryboardNote[] {
  const notes: StoryboardNote[] = []
  const signatures = plan.scenes.map(s => JSON.stringify([s.kind, s.environment ?? 'plain', s.framing ?? 'reference', s.beats?.map(b => [b.object, b.action, b.label])]))
  plan.scenes.forEach((scene, sceneIndex) => {
    if (!scene.beats?.length) return
    const add = (code: StoryboardNote['code'], message: string) => notes.push({ sceneIndex, code, message })
    const times = scene.beats.map(b => plan.cues[b.cue]?.start).filter((t): t is number => t !== undefined)
    if (sceneIndex >= 2 && signatures[sceneIndex] === signatures[sceneIndex - 1] && signatures[sceneIndex] === signatures[sceneIndex - 2] && signatures[sceneIndex] !== signatures[sceneIndex - 3]) {
      add('repetition', '连续三镜使用相同场景、景别、构图、物体与动作。请检查是否有叙事进展；有意回扣可以保留。')
    }
    if (times.some((time, i) => i > 0 && time > times[i - 1]! && time - times[i - 1]! < 1.2)) {
      add('crowding', '两次物体出场相隔不足1.2秒，可能来不及看清。并列对照可使用同一字幕锚点。')
    }
    if (times.length && times[0]! - (plan.cues[scene.first]?.start ?? times[0]!) > 4) {
      add('late-subject', '本镜开始讲述4秒后才出现主体。请核对是否需要把首个物体放到较早的字幕。')
    }
    if (times.length && (plan.cues[scene.last]?.end ?? times.at(-1)!) - times.at(-1)! >= 12) {
      add('long-hold', '最后一次物体出场后仍有至少12秒讲述。请试听检查；复杂解释需要停留时可以保留。')
    }
  })
  return notes
}
