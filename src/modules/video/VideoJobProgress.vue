<script setup lang="ts">
import { Progress } from '@/components/ui/progress'
import { computed, onBeforeUnmount, ref, watch } from 'vue'
import AppIcon from '../../components/AppIcon.vue'
import { videoStages, videoStatuses, type VideoJob } from '../../types/video'

const props = defineProps<{ job: VideoJob; syncError?: string }>()
const now = ref(Date.now())
let clock: ReturnType<typeof globalThis.setInterval> | undefined
const active = computed(() => props.job.status === 'running')
const stateLabel = computed(() => props.job.stage === 'transcript_ready' && props.job.status === 'draft' ? '字幕已就绪' : videoStatuses[props.job.status])
const progress = computed(() => active.value && props.job.progress?.stage === props.job.stage ? props.job.progress : null)
const rendering = computed(() => active.value && ['rendering', 'previewing'].includes(props.job.stage))
const renderSteps = [
  { phase: 'rendering_frames', label: '绘制画面', unit: 'frames' },
  { phase: 'preparing_subtitles', label: '准备字幕', unit: 'cues' },
  { phase: 'rendering_subtitles', label: '合成字幕', unit: 'seconds' },
  { phase: 'muxing_audio', label: '合入原声', unit: 'seconds' },
]
const audioPhases: Record<string, string> = { measuring_audio: '分析', normalizing_audio: '调整', checking_audio: '核对' }
const renderStep = computed(() => progress.value?.phase === 'checking_video' ? renderSteps.length : renderSteps.findIndex(item => item.phase === (audioPhases[progress.value?.phase ?? ''] ? 'muxing_audio' : progress.value?.phase)))
const steps = ['原声字幕', '动画分镜', '确认分镜', '画面渲染', '成片归档']
const step = computed(() => {
  if (props.job.status === 'succeeded') return steps.length
  if (props.job.status === 'awaiting_confirmation') return 2
  return { preparing: 0, transcribing: 0, proofreading: 0, transcript_ready: 1, storyboard: 1, previewing: 2, rendering: 3, validating: 4, complete: 4 }[props.job.stage] ?? 0
})
const covered = computed(() => (props.job.plan.scenes.at(-1)?.last ?? -1) + 1)
const reviewed = computed(() => props.job.plan.subtitleReview?.completed ?? 0)
const percent = computed(() => {
  const p = progress.value
  const measured = p && rendering.value && ((audioPhases[p.phase] && p.unit === 'seconds') || renderSteps.some(item => item.phase === p.phase && (p.phase === 'rendering_frames' || item.unit === p.unit)))
  if (p && p.total > 0 && (props.job.stage === 'transcribing' || measured)) return Math.min(100, Math.max(0, Math.floor(p.completed / p.total * 100)))
  if (props.job.stage === 'storyboard' && props.job.plan.cues.length) return Math.min(100, Math.floor(covered.value / props.job.plan.cues.length * 100))
  if (props.job.stage === 'proofreading' && props.job.plan.cues.length) return Math.min(100, Math.floor(reviewed.value / props.job.plan.cues.length * 100))
  return null
})
const phaseCopy: Record<string, [string, string]> = {
  loading_model: ['正在唤醒本地语音模型', '模型在你的电脑上加载。准备好后，识别出的字幕会逐段出现在下方。'],
  decoding_audio: ['正在读取原声音频', '正在解码音频并定位人声，为逐句识别做准备。'],
  recognizing: ['正在听懂你的原声', '按原声时间轴逐段识别，字幕预览会随识别进展更新。'],
  finalizing_subtitles: ['正在整理字幕时间轴', '原声识别已结束，保存字幕后将由文本模型逐段校对，再编排动画分镜。'],
  checking_layout: ['正在检查每一镜的排版', '逐镜检查文字与图形，检查通过后开始绘制视频画面。'],
  rendering_thumbnails: ['正在生成分镜缩略图', '从本次真实绘图中截取，不使用另一套预览画面。'],
  rendering_frames: ['正在让分镜动起来', '按已完成的视频片段更新真实帧数，复用的缓存也会计入。画面就绪后，再合成字幕与原声。'],
  preparing_subtitles: ['正在准备每一句字幕', '画面已绘制完成，正在按原声时间轴生成字幕图层。下方显示已准备好的字幕数量。'],
  rendering_subtitles: ['正在把字幕放入画面', '正在逐帧合成字幕，进度跟随实际处理的视频位置更新。长视频需要一些时间，可在下方查看处理速度。'],
  measuring_audio: ['正在分析整段音量', '测量完整原声，为试片与成片确定一致的音量。原音频文件保持不变。'],
  normalizing_audio: ['正在调整人声音量', '自动提高偏轻的人声并控制峰值，保留语速和字幕时间轴。'],
  checking_audio: ['正在核对输出音量', '检查编码后的音轨，确认峰值合适后再合入画面。'],
  muxing_audio: ['正在把原声合入画面', '字幕已合成，正在合入原声音轨并写入视频文件。完成后还会检查并归档成片。'],
  checking_video: ['正在核对成片', '检查视频尺寸、时长与音轨，准备保存成片。'],
}
const copy = computed<[string, string]>(() => {
  const job = props.job
  if (job.stage === 'transcript_ready' && job.status === 'draft') return ['字幕校对已完成', '自动流程将先校核发布材料、导出三尺寸封面，再继续本任务生成分镜，无需重复识别音频。']
  if (job.status === 'awaiting_confirmation' && job.preview) return ['试片已就绪，完整视频尚未生成', '请播放片段并检查分镜；修改后重新试片，最后再确认完整视频。']
  if (job.status === 'awaiting_confirmation') return ['分镜已就绪，等你过目', '字幕与动画分镜已保存。向下校对，确认后再开始制作视频。']
  if (job.status === 'succeeded') return ['你的原声，已有了画面', '成片和字幕已保存，可在下方播放，或选用到发布准备。']
  if (job.status === 'failed') return ['这一步没有完成', `停在「${videoStages[job.stage] || job.stage}」。已保存的字幕与分镜保留，可检查原因后继续。`]
  if (job.status === 'cancelled') return ['制作已停止', '已保存的字幕与分镜保留，需要时可继续准备或重新确认制作。']
  if (job.status === 'draft') return ['准备好，再开始', '选好音频和字幕来源，即可开始制作。']
  if (job.stage === 'transcribing' && job.sourceSubtitleId) return ['正在读取已有字幕', '校验字幕内容与时间码后，将由文本模型逐段校对，再编排动画分镜。']
  if (job.stage === 'proofreading') return ['正在逐段校对原声字幕', '参考音频提示词的开场与结尾，结合上下文检查正文错别字、人名和术语。保留原时间码，不确定的文字留待试听。']
  const phase = progress.value?.phase
  if (job.stage === 'previewing') return ['正在制作原声试片', `${phase === 'checking_video' ? '正在核对试片' : phaseCopy[phase ?? '']?.[0] || '正在准备绘制环境'}。仅预览所选片段，不重新识别字幕，不登记为正式成片。`]
  if (phase && phaseCopy[phase]) return phaseCopy[phase]
  if (job.stage === 'transcribing') return ['正在准备原声识别', '本地语音模型正在准备，首段字幕返回后会在这里实时显示。']
  if (job.stage === 'storyboard') return ['正在把文字编排成画面', '按批次生成动画分镜，完成的分镜会持续保存。全部就绪后交给你确认。']
  if (job.stage === 'rendering') return ['正在准备视频画面', '正在启动绘制环境，随后逐镜检查排版、绘制画面并合入原声。']
  if (job.stage === 'validating') return ['正在为成片收尾', '核对视频与字幕，登记到本书素材后即可播放。']
  return ['正在准备这次创作', '检查原声音频与运行环境，随后开始制作。']
})
function time(seconds: number) {
  const value = Math.max(0, Math.floor(seconds))
  return `${String(Math.floor(value / 60)).padStart(2, '0')}:${String(value % 60).padStart(2, '0')}`
}
const elapsed = computed(() => progress.value?.startedAt ? time((now.value - progress.value.startedAt) / 1000) : '')
const feedbackAge = computed(() => progress.value?.updatedAt ? Math.max(0, Math.floor((now.value - progress.value.updatedAt) / 1000)) : null)
const feedbackTime = computed(() => feedbackAge.value === null ? '' : feedbackAge.value < 2 ? '刚刚更新' : feedbackAge.value < 60 ? `${feedbackAge.value} 秒前` : `${Math.floor(feedbackAge.value / 60)} 分钟前`)
const fresh = computed(() => !props.syncError && (feedbackAge.value === null || feedbackAge.value < 15))
const speed = computed(() => {
  const p = progress.value
  return fresh.value && p?.unit === 'seconds' && p.speed && Number.isFinite(p.speed) && p.speed > 0 ? `${p.speed.toFixed(2)}×` : ''
})
const cueCount = computed(() => progress.value?.cueCount || props.job.plan.cues.length)
const previewCues = computed(() => props.job.stage === 'proofreading' ? props.job.plan.cues.slice(Math.max(0, reviewed.value - 3), reviewed.value) : progress.value?.preview.length ? progress.value.preview : props.job.plan.cues.slice(-3))
const previewScenes = computed(() => props.job.plan.scenes.slice(-2))
const measure = computed(() => {
  if (props.job.stage === 'transcribing' && progress.value?.total) return `已识别 ${time(progress.value.completed)} / ${time(progress.value.total)}`
  if (['rendering', 'previewing'].includes(props.job.stage) && progress.value?.phase === 'rendering_frames' && progress.value.total) return `已绘制 ${progress.value.completed.toLocaleString('zh-CN')} / ${progress.value.total.toLocaleString('zh-CN')} 帧`
  if (rendering.value && progress.value?.total) {
    const p = progress.value
    if (p.phase === 'preparing_subtitles' && p.unit === 'cues') return `已准备 ${p.completed} / ${p.total} 条字幕`
    if (audioPhases[p.phase] && p.unit === 'seconds') return `已${audioPhases[p.phase]} ${time(p.completed)} / ${time(p.total)}`
    if (p.unit === 'seconds' && ['rendering_subtitles', 'muxing_audio'].includes(p.phase)) return `已${p.phase === 'rendering_subtitles' ? '合成' : '合入'} ${time(p.completed)} / ${time(p.total)}`
  }
  if (props.job.stage === 'storyboard' && props.job.plan.cues.length) return `已编排 ${covered.value} / ${props.job.plan.cues.length} 条字幕`
  if (props.job.stage === 'proofreading' && props.job.plan.cues.length) return `已校对 ${reviewed.value} / ${props.job.plan.cues.length} 条字幕`
  return phaseCopy[progress.value?.phase ?? '']?.[0] ?? videoStages[props.job.stage] ?? props.job.stage
})
watch(active, value => {
  if (clock) globalThis.clearInterval(clock)
  now.value = Date.now()
  if (value) clock = globalThis.setInterval(() => { now.value = Date.now() }, 1000)
}, { immediate: true })
onBeforeUnmount(() => { if (clock) globalThis.clearInterval(clock) })
</script>

<template>
  <div
    class="creation-progress"
    :class="{ 'is-animating': active && fresh }"
    :data-status="job.status"
  >
    <div class="progress-hero">
      <div
        class="progress-emblem"
        aria-hidden="true"
      >
        <strong
          v-if="active && percent !== null"
          class="emblem-value"
        >{{ percent }}<small>%</small></strong>
        <AppIcon
          v-else
          :name="job.status === 'succeeded' ? 'check' : job.status === 'failed' ? 'info' : job.status === 'awaiting_confirmation' ? 'pen' : ['rendering', 'previewing'].includes(job.stage) ? 'video' : 'headphones'"
        />
      </div>
      <div class="progress-intro">
        <span class="progress-eyebrow"><span class="progress-dot" />{{ stateLabel }}</span>
        <div
          role="status"
          aria-live="polite"
          aria-atomic="true"
        >
          <h3>{{ copy[0] }}</h3>
          <p>{{ copy[1] }}</p>
        </div>
      </div>
      <span
        v-if="active && elapsed"
        class="elapsed-time"
        aria-label="本次执行已用时间"
      ><AppIcon name="clock" />已用 <strong>{{ elapsed }}</strong></span>
    </div>

    <ol
      class="creation-steps"
      aria-label="视频制作流程"
    >
      <li
        v-for="(label, index) in steps"
        :key="label"
        :class="{ 'is-done': index < step, 'is-current': index === step }"
        :aria-current="index === step ? 'step' : undefined"
      >
        <span class="step-marker"><AppIcon
          v-if="index < step"
          name="check"
        /><span v-else>{{ String(index + 1).padStart(2, '0') }}</span></span>
        <span>{{ label }}</span><small>{{ index < step ? '已完成' : index === step ? (job.status === 'awaiting_confirmation' ? '待你确认' : active ? '进行中' : stateLabel) : '待进行' }}</small>
      </li>
    </ol>

    <div
      v-if="active"
      class="stage-progress"
      :class="{ 'render-monitor': rendering }"
    >
      <div class="stage-progress-label">
        <span>{{ measure }}</span><strong>{{ percent === null ? '正在进行' : `本阶段 ${percent}%` }}</strong>
      </div>
      <Progress
        :key="progress?.phase || job.stage"
        :model-value="percent"
        class="stage-progress-track"
        :aria-label="videoStages[job.stage] || '视频制作'"
        :aria-valuetext="percent === null ? '正在进行，暂无可量化进度' : measure"
      />
      <template v-if="rendering">
        <div class="render-telemetry">
          <span
            v-if="speed"
            class="render-speed"
          ><AppIcon name="video" />处理速度 <strong>{{ speed }}</strong><small>倍速</small></span>
          <span v-if="feedbackTime"><span
            class="telemetry-dot"
            :class="{ 'is-fresh': fresh }"
          />进度更新 <strong>{{ feedbackTime }}</strong></span>
          <span
            v-if="feedbackAge !== null && feedbackAge >= 15 && !syncError"
            class="render-wait"
          >等待新的进度反馈</span>
          <span v-if="!speed && !feedbackTime">进度随实际处理更新</span>
        </div>
        <ol
          class="render-phases"
          aria-label="渲染步骤"
        >
          <li
            v-for="(item, index) in renderSteps"
            :key="item.phase"
            :data-state="index < renderStep ? 'done' : index === renderStep ? 'current' : 'pending'"
            :aria-current="index === renderStep ? 'step' : undefined"
          >
            <span class="render-phase-marker"><AppIcon
              v-if="index < renderStep"
              name="check"
            /><span v-else>{{ index + 1 }}</span></span>
            <span>{{ item.label }}<small>{{ index < renderStep ? '已完成' : index === renderStep ? '进行中' : '待进行' }}</small></span>
          </li>
        </ol>
      </template>
    </div>

    <slot />
    <div class="creation-output">
      <div class="output-summary">
        <span class="output-label">这次创作</span>
        <div class="output-metric">
          <strong>{{ job.plan.duration ? time(job.plan.duration) : '读取中' }}</strong><span>原声时长</span>
        </div>
        <div class="output-metric">
          <strong>{{ cueCount || '待识别' }}<small v-if="cueCount"> 条</small></strong><span>{{ progress?.cueCount ? '已识别字幕' : '已保存字幕' }}</span>
        </div>
        <div class="output-metric">
          <strong>{{ job.plan.scenes.length || '待编排' }}<small v-if="job.plan.scenes.length"> 镜</small></strong><span>已保存分镜</span>
        </div>
      </div>
      <div
        v-if="job.status !== 'succeeded'"
        class="output-preview"
      >
        <template v-if="previewScenes.length">
          <div class="preview-heading">
            <AppIcon name="video" /><span>最近完成的分镜</span><small>已保存</small>
          </div>
          <div class="scene-preview-list">
            <article
              v-for="(scene, index) in previewScenes"
              :key="scene.first"
              class="scene-preview"
            >
              <span class="scene-number">{{ String(job.plan.scenes.length - previewScenes.length + index + 1).padStart(2, '0') }}</span>
              <div><strong>{{ scene.title }}</strong><p>{{ scene.beats?.map(beat => beat.label).join(' · ') || scene.labels?.join(' · ') }}</p></div>
            </article>
          </div>
        </template>
        <template v-else-if="previewCues.length">
          <div class="preview-heading">
            <AppIcon name="headphones" /><span>{{ job.stage === 'proofreading' ? '最近校对的字幕' : progress?.preview.length ? '实时字幕预览' : '已保存字幕预览' }}</span><small>{{ job.plan.subtitleReview ? '仍需试听核对' : '待校对' }}</small>
          </div>
          <ol class="subtitle-preview">
            <li
              v-for="cue in previewCues"
              :key="`${cue.start}-${cue.end}`"
            >
              <time>{{ time(cue.start) }}</time><span>{{ cue.text }}</span>
            </li>
          </ol>
        </template>
        <div
          v-else
          class="preview-placeholder"
        >
          <AppIcon name="headphones" /><strong>让每一句原声，慢慢成为画面</strong><p>{{ job.stage === 'proofreading' ? '首批校对完成并保存后，将在这里展示。' : '首段字幕识别出来后，将在这里实时呈现。' }}</p>
        </div>
      </div>
    </div>
    <p
      v-if="syncError"
      class="progress-note sync-warning"
      role="status"
    >
      <AppIcon name="info" />进度暂未同步，正在重试连接。当前展示上次读取的进度。
    </p>
    <p
      v-else-if="active"
      class="progress-note"
    >
      <AppIcon name="shield" />可离开本页，任务会在后台继续。请保持应用开启；关闭后需手动重试。
    </p>
    <div
      v-if="job.error"
      class="progress-error"
      role="alert"
    >
      <strong><AppIcon name="info" />错误详情</strong>
      <p>{{ job.error }}</p>
    </div>
    <div
      v-if="$slots.actions"
      class="progress-actions"
    >
      <slot name="actions" />
    </div>
  </div>
</template>

<style scoped>
.creation-progress { --progress-tone: var(--video-accent); display: grid; grid-template-columns: minmax(0, 1fr); gap: 20px; min-width: 0; font-family: var(--font-sans); }
.creation-progress[data-status='succeeded'], .creation-progress[data-status='awaiting_confirmation'] { --progress-tone: var(--primary); }
.creation-progress[data-status='failed'] { --progress-tone: var(--destructive); }
.creation-progress[data-status='cancelled'], .creation-progress[data-status='draft'] { --progress-tone: var(--muted-foreground); }
.progress-hero { display: flex; align-items: center; gap: 16px; min-width: 0; }
.progress-emblem { display: grid; place-items: center; width: 56px; height: 56px; flex-shrink: 0; color: var(--progress-tone); border: 1px solid color-mix(in srgb, var(--progress-tone) 35%, transparent); border-radius: 16px; background: color-mix(in srgb, var(--progress-tone) 10%, transparent); }
.progress-emblem > .app-icon { width: 28px; height: 28px; }
.emblem-value { font-size: 16px; font-weight: 800; font-variant-numeric: tabular-nums; }
.emblem-value small { font-size: inherit; }
.progress-intro { flex: 1; min-width: 0; }
.progress-eyebrow { display: inline-flex; align-items: center; gap: 8px; color: var(--progress-tone); font-size: 12px; font-weight: 700; }
.progress-dot { width: 8px; height: 8px; background: currentColor; border-radius: 50%; }
.is-animating .progress-dot { animation: progress-pulse 2s ease-in-out infinite; }
.progress-intro h3 { margin: 4px 0 2px; color: var(--foreground); font: 700 20px/28px var(--font-sans); }
.progress-intro p { margin: 0; color: var(--muted-foreground); font-size: 12px; line-height: 1.65; }
.elapsed-time { display: inline-flex; align-items: center; gap: 7px; white-space: nowrap; color: var(--muted-foreground); font-size: 12px; padding: 8px 16px; background: var(--input-surface); border: 1px solid var(--border); border-radius: 12px; }
.elapsed-time .app-icon { width: 14px; height: 14px; }
.elapsed-time strong { color: var(--foreground); font-weight: 700; font-variant-numeric: tabular-nums; }
.creation-steps { display: grid; grid-template-columns: repeat(5, minmax(0, 1fr)); gap: 8px; margin: 0; padding: 8px 0 0; list-style: none; border-top: 1px solid var(--border); }
.creation-steps li { display: grid; grid-template-columns: auto minmax(0, 1fr); gap: 4px 8px; align-content: start; align-items: center; padding: 10px; min-width: 0; color: var(--muted-foreground); font-size: 12px; font-weight: 600; border-bottom: 2px solid var(--border); border-radius: 6px 6px 0 0; }
.step-marker { display: flex; align-items: center; font-size: 11px; font-variant-numeric: tabular-nums; }
.step-marker .app-icon { width: 14px; height: 14px; }
.creation-steps li > small { grid-column: 1 / -1; color: var(--muted-foreground); font-size: 12px; font-weight: 400; }
.creation-steps .is-current { color: var(--progress-tone); border-color: color-mix(in srgb, var(--progress-tone) 40%, transparent); background: color-mix(in srgb, var(--progress-tone) 10%, var(--card)); }
.creation-steps .is-current > small { color: var(--progress-tone); }
.creation-steps .is-done { color: var(--primary); border-color: color-mix(in srgb, var(--primary) 30%, transparent); background: var(--accent); }
.stage-progress-label { display: flex; justify-content: space-between; flex-wrap: wrap; gap: 6px 12px; margin-bottom: 8px; font-size: 12px; color: var(--text-secondary); font-variant-numeric: tabular-nums; }
.stage-progress-label strong { color: var(--progress-tone); font-weight: 700; }
.stage-progress-track { height: 8px; border: 1px solid var(--border); background: var(--terminal); }
.stage-progress-track :deep([data-slot='progress-indicator']) { background: linear-gradient(to right, var(--video-accent), #6366f1, var(--info)); }
.creation-progress:not(.is-animating) .stage-progress-track :deep([data-state='indeterminate']) { animation-play-state: paused; }
.render-monitor { padding: 16px; border: 1px solid var(--border); border-radius: 12px; background: color-mix(in srgb, var(--terminal) 70%, transparent); }
.render-telemetry { display: flex; flex-wrap: wrap; align-items: center; gap: 8px 24px; margin-top: 12px; color: var(--muted-foreground); font-size: 12px; }
.render-telemetry > span { display: inline-flex; align-items: center; gap: 7px; }
.render-telemetry .app-icon { width: 14px; height: 14px; }
.render-telemetry strong { color: var(--foreground); font-weight: 600; font-variant-numeric: tabular-nums; }
.render-telemetry small { font-size: inherit; }
.telemetry-dot { width: 5px; height: 5px; border-radius: 50%; background: var(--muted-foreground); }
.telemetry-dot.is-fresh { background: var(--primary); }
.render-wait { color: var(--warning); }
.render-phases { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 12px; margin: 16px 0 0; padding: 16px 0 0; border-top: 1px solid var(--border); list-style: none; }
.render-phases li { display: flex; align-items: center; gap: 8px; min-width: 0; color: var(--muted-foreground); font-size: 12px; }
.render-phase-marker { display: grid; place-items: center; flex-shrink: 0; width: 24px; height: 24px; border: 1px solid var(--input-border); border-radius: 8px; font-size: 10px; }
.render-phase-marker .app-icon { width: 13px; height: 13px; }
.render-phases small { display: block; margin-top: 2px; font-size: 12px; }
.render-phases [data-state='done'] { color: var(--primary); }
.render-phases [data-state='current'] { color: var(--progress-tone); }
.render-phases [data-state='current'] .render-phase-marker { border-color: var(--progress-tone); background: var(--video-surface); }
.creation-output { display: grid; grid-template-columns: minmax(0, 1fr); gap: 16px; }
.output-summary, .output-preview { min-width: 0; padding: 16px; border: 1px solid var(--border); border-radius: 10px; background: var(--input-surface); }
.output-summary { display: grid; grid-template-columns: auto repeat(3, minmax(0, 1fr)); align-items: center; gap: 16px; }
.output-label { font-size: 12px; font-weight: 700; color: var(--muted-foreground); }
.output-metric { display: flex; align-items: baseline; flex-wrap: wrap; gap: 4px 8px; }
.output-metric > strong { font-size: 16px; font-weight: 600; font-variant-numeric: tabular-nums; }
.output-metric:nth-last-child(2) > strong { color: var(--info); }
.output-metric:last-child > strong { color: var(--primary); }
.output-metric strong small { font-size: 12px; font-weight: 400; }
.output-metric > span { color: var(--muted-foreground); font-size: 12px; }
.preview-heading { display: flex; flex-wrap: wrap; gap: 6px; align-items: center; padding-bottom: 8px; margin-bottom: 10px; border-bottom: 1px solid var(--border); color: var(--info); font-size: 12px; font-weight: 700; }
.preview-heading .app-icon { width: 15px; height: 15px; }
.preview-heading small { margin-left: auto; font-size: 12px; color: var(--muted-foreground); font-weight: 400; }
.subtitle-preview { display: grid; gap: 8px; padding: 0; margin: 0; list-style: none; }
.subtitle-preview li { display: flex; gap: 10px; padding: 6px; border: 1px solid var(--border); border-radius: 6px; background: color-mix(in srgb, var(--input-surface) 60%, transparent); font-size: 12px; line-height: 1.65; }
.subtitle-preview time { flex-shrink: 0; color: var(--video-accent); font-weight: 700; font-variant-numeric: tabular-nums; }
.subtitle-preview li > span { overflow-wrap: anywhere; }
.scene-preview-list { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 8px; }
.scene-preview { display: flex; gap: 12px; align-items: flex-start; min-width: 0; padding: 12px; border: 1px solid var(--border); border-radius: 12px; background: var(--input-surface); }
.scene-preview > div { min-width: 0; }
.scene-number { flex-shrink: 0; padding: 2px 8px; background: var(--secondary); border-radius: 4px; color: var(--video-accent); font-size: 12px; font-weight: 700; }
.scene-preview strong { font-size: 12px; font-weight: 700; overflow-wrap: anywhere; }
.scene-preview p { margin: 4px 0 0; color: var(--muted-foreground); font-size: 12px; overflow-wrap: anywhere; }
.preview-placeholder { display: grid; min-height: 125px; align-content: center; justify-items: center; text-align: center; gap: 8px; color: var(--muted-foreground); }
.preview-placeholder .app-icon { width: 24px; height: 24px; }
.preview-placeholder strong { font-size: 12px; font-weight: 500; }
.preview-placeholder p { margin: 0; font-size: 12px; }
.progress-note { display: flex; gap: 8px; align-items: flex-start; margin: 0; color: var(--muted-foreground); font-size: 12px; line-height: 1.7; }
.progress-note .app-icon { width: 14px; height: 14px; margin-top: 2px; }
.sync-warning { color: var(--warning); }
.progress-error { padding: 16px; border: 1px solid color-mix(in srgb, var(--destructive) 30%, transparent); border-radius: 12px; background: var(--error-surface); color: var(--destructive); font-size: 12px; }
.progress-error strong { display: flex; align-items: center; gap: 8px; }
.progress-error p { margin: 10px 0 0; white-space: pre-wrap; overflow-wrap: anywhere; }
.progress-actions { padding-top: 16px; border-top: 1px solid var(--border); }
@keyframes progress-pulse { 50% { opacity: .4; } }
@media (max-width: 1100px) { .progress-hero { flex-wrap: wrap; } .elapsed-time { margin-left: 72px; } .creation-output { grid-template-columns: minmax(0, 1fr); } }
@media (max-width: 700px) { .creation-steps { grid-template-columns: repeat(2, minmax(0, 1fr)); } .render-phases { grid-template-columns: repeat(2, minmax(0, 1fr)); } .scene-preview-list { grid-template-columns: minmax(0, 1fr); } .output-summary { grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 12px; } .output-label { grid-column: 1 / -1; } }
@media (max-width: 520px) { .progress-hero { display: grid; grid-template-columns: 56px minmax(0, 1fr); gap: 12px; } .progress-emblem { grid-column: 1; grid-row: 1; } .progress-intro { grid-column: 1 / -1; grid-row: 2; } .elapsed-time { grid-column: 2; grid-row: 1; justify-self: end; margin: 0; padding: 8px; } .elapsed-time .app-icon { display: none; } .creation-steps li { padding: 8px; } .render-phases { grid-template-columns: minmax(0, 1fr); } .output-summary { grid-template-columns: minmax(0, 1fr); } }
@media (prefers-reduced-motion: reduce) { .progress-dot { animation: none; } }
</style>
