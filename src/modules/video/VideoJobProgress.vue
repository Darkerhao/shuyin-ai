<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue'
import AppIcon from '../../components/AppIcon.vue'
import { videoStages, videoStatuses, type VideoJob } from '../../types/video'

const props = defineProps<{ job: VideoJob; syncError?: string }>()
const now = ref(Date.now())
let clock: ReturnType<typeof globalThis.setInterval> | undefined
const active = computed(() => props.job.status === 'running')
const progress = computed(() => active.value && props.job.progress?.stage === props.job.stage ? props.job.progress : null)
const steps = ['原声字幕', '动画分镜', '确认分镜', '画面渲染', '成片归档']
const step = computed(() => {
  if (props.job.status === 'succeeded') return steps.length
  if (props.job.status === 'awaiting_confirmation') return 2
  return { preparing: 0, transcribing: 0, storyboard: 1, rendering: 3, validating: 4, complete: 4 }[props.job.stage] ?? 0
})
const covered = computed(() => (props.job.plan.scenes.at(-1)?.last ?? -1) + 1)
const percent = computed(() => {
  const p = progress.value
  if (p && p.total > 0 && (props.job.stage === 'transcribing' || (props.job.stage === 'rendering' && p.phase === 'rendering_frames'))) return Math.min(100, Math.max(0, Math.floor(p.completed / p.total * 100)))
  if (props.job.stage === 'storyboard' && props.job.plan.cues.length) return Math.min(100, Math.floor(covered.value / props.job.plan.cues.length * 100))
  return null
})
const phaseCopy: Record<string, [string, string]> = {
  loading_model: ['正在唤醒本地语音模型', '模型在你的电脑上加载。准备好后，识别出的字幕会逐段出现在下方。'],
  decoding_audio: ['正在读取原声音频', '正在解码音频并定位人声，为逐句识别做准备。'],
  recognizing: ['正在听懂你的原声', '按原声时间轴逐段识别，字幕预览会随识别进展更新。'],
  finalizing_subtitles: ['正在整理字幕时间轴', '原声识别已结束，正在核对字幕，随后开始编排动画分镜。'],
  checking_layout: ['正在检查每一镜的排版', '逐镜检查文字与图形，检查通过后开始绘制视频画面。'],
  rendering_frames: ['正在让分镜动起来', '按已完成的视频片段更新真实帧数，复用的缓存也会计入。画面就绪后，再合成字幕与原声。'],
  rendering_subtitles: ['正在把字幕放入画面', '画面已绘制完成，正在按原声时间轴合成字幕。这一步仍需编码，请保持应用开启。'],
  muxing_audio: ['正在把原声合入画面', '画面已绘制完成，正在合入原声音轨并写入视频文件。'],
  checking_video: ['正在核对成片', '检查视频尺寸、时长与音轨，准备保存成片。'],
}
const copy = computed<[string, string]>(() => {
  const job = props.job
  if (job.status === 'awaiting_confirmation') return ['分镜已就绪，等你过目', '字幕与动画分镜已保存。向下校对，确认后再开始制作视频。']
  if (job.status === 'succeeded') return ['你的原声，已有了画面', '成片和字幕已保存，可在下方播放，或选用到发布准备。']
  if (job.status === 'failed') return ['这一步没有完成', `停在「${videoStages[job.stage] || job.stage}」。已保存的字幕与分镜保留，可检查原因后继续。`]
  if (job.status === 'cancelled') return ['制作已停止', '已保存的字幕与分镜保留，需要时可继续准备或重新确认制作。']
  if (job.status === 'draft') return ['准备好，再开始', '选好音频和字幕来源，即可开始制作。']
  if (job.stage === 'transcribing' && job.sourceSubtitleId) return ['正在读取已有字幕', '校验字幕内容与时间码后，将直接进入动画分镜编排。']
  const phase = progress.value?.phase
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
const cueCount = computed(() => progress.value?.cueCount || props.job.plan.cues.length)
const previewCues = computed(() => progress.value?.preview.length ? progress.value.preview : props.job.plan.cues.slice(-3))
const previewScenes = computed(() => props.job.plan.scenes.slice(-2))
const measure = computed(() => {
  if (props.job.stage === 'transcribing' && progress.value?.total) return `已识别 ${time(progress.value.completed)} / ${time(progress.value.total)}`
  if (props.job.stage === 'rendering' && progress.value?.phase === 'rendering_frames' && progress.value.total) return `已绘制 ${progress.value.completed.toLocaleString('zh-CN')} / ${progress.value.total.toLocaleString('zh-CN')} 帧`
  if (props.job.stage === 'storyboard' && props.job.plan.cues.length) return `已编排 ${covered.value} / ${props.job.plan.cues.length} 条字幕`
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
    :data-status="job.status"
  >
    <div class="progress-hero">
      <div
        class="progress-emblem"
        :class="{ 'is-active': active, 'is-indeterminate': percent === null }"
        aria-hidden="true"
      >
        <svg
          class="emblem-ring"
          viewBox="0 0 88 88"
        >
          <circle
            class="emblem-track"
            cx="44"
            cy="44"
            r="39"
          />
          <circle
            v-if="active"
            class="emblem-arc"
            cx="44"
            cy="44"
            r="39"
            pathLength="100"
            :stroke-dasharray="percent === null ? '22 78' : `${percent} 100`"
          />
        </svg>
        <strong
          v-if="active && percent !== null"
          class="emblem-value"
        >{{ percent }}<small>%</small></strong>
        <AppIcon
          v-else
          :name="job.status === 'succeeded' ? 'check' : job.status === 'failed' ? 'info' : job.status === 'awaiting_confirmation' ? 'pen' : job.stage === 'rendering' ? 'video' : 'headphones'"
        />
      </div>
      <div class="progress-intro">
        <span class="progress-eyebrow"><span class="progress-dot" />{{ videoStatuses[job.status] }}</span>
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
        <span>{{ label }}</span><small>{{ index < step ? '已完成' : index === step ? (job.status === 'awaiting_confirmation' ? '待你确认' : active ? '进行中' : videoStatuses[job.status]) : '待进行' }}</small>
      </li>
    </ol>

    <div
      v-if="active"
      class="stage-progress"
    >
      <div class="stage-progress-label">
        <span>{{ measure }}</span><strong>{{ percent === null ? '正在进行' : `本阶段 ${percent}%` }}</strong>
      </div>
      <div
        class="stage-progress-track"
        :class="{ 'is-indeterminate': percent === null }"
        role="progressbar"
        :aria-label="videoStages[job.stage] || '视频制作'"
        :aria-valuenow="percent === null ? undefined : percent"
        :aria-valuetext="percent === null ? '正在进行，暂无可量化进度' : measure"
        aria-valuemin="0"
        aria-valuemax="100"
      >
        <span :style="percent === null ? undefined : { width: `${percent}%` }" />
      </div>
    </div>

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
      <div class="output-preview">
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
            <AppIcon name="headphones" /><span>{{ progress?.preview.length ? '实时字幕预览' : '已保存字幕预览' }}</span><small>待校对</small>
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
          <AppIcon name="headphones" /><strong>让每一句原声，慢慢成为画面</strong><p>首段字幕识别出来后，将在这里实时呈现。</p>
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
  </div>
</template>

<style scoped>
.creation-progress { --progress-tone: var(--primary); overflow: hidden; border: 1px solid var(--border); border-radius: 14px; background: radial-gradient(ellipse at 0 0, var(--accent), transparent 68%), var(--card); }
.creation-progress[data-status='failed'] { --progress-tone: var(--color-destructive); }
.creation-progress[data-status='cancelled'], .creation-progress[data-status='draft'] { --progress-tone: var(--muted-foreground); }
.progress-hero { display: flex; align-items: center; gap: 20px; padding: 28px 28px 24px; }
.progress-emblem { position: relative; display: grid; place-items: center; width: 80px; height: 80px; color: var(--progress-tone); flex-shrink: 0; }
.progress-emblem > .emblem-ring { position: absolute; inset: 0; width: 100%; height: 100%; fill: none; stroke-width: 2.5; transform: rotate(-90deg); }
.emblem-track { stroke: color-mix(in srgb, var(--progress-tone) 12%, transparent); }
.emblem-arc { stroke: var(--progress-tone); stroke-linecap: round; transition: stroke-dasharray .5s ease; }
.is-active.is-indeterminate > .emblem-ring { animation: orbit 3s linear infinite; }
.progress-emblem > .app-icon { width: 28px; height: 28px; }
.emblem-value { font-size: 25px; font-weight: 600; font-variant-numeric: tabular-nums; letter-spacing: -.04em; }
.emblem-value small { margin-left: 2px; font-size: 12px; font-weight: 500; letter-spacing: 0; }
.progress-intro { flex: 1; min-width: 0; }
.progress-eyebrow { display: inline-flex; align-items: center; gap: 7px; color: var(--progress-tone); font-size: 12px; font-weight: 600; letter-spacing: .08em; }
.progress-dot { width: 6px; height: 6px; background: currentColor; border-radius: 50%; }
.progress-intro h3 { margin: 5px 0 7px; font-family: 'Songti SC', 'SimSun', serif; font-size: clamp(21px, 2.2vw, 27px); font-weight: 700; line-height: 1.45; letter-spacing: .02em; }
.progress-intro p { margin: 0; max-width: 600px; color: var(--muted-foreground); font-size: 13px; line-height: 1.8; }
.elapsed-time { display: inline-flex; align-items: center; align-self: flex-start; gap: 7px; white-space: nowrap; color: var(--muted-foreground); font-size: 12px; padding: 7px 10px; background: var(--card); border: 1px solid var(--border); border-radius: 7px; }
.elapsed-time .app-icon { width: 14px; height: 14px; }
.elapsed-time strong { color: var(--foreground); font-size: 13px; font-weight: 600; font-variant-numeric: tabular-nums; }
.creation-steps { display: grid; grid-template-columns: repeat(5, minmax(0, 1fr)); margin: 0 28px; padding: 0 0 22px; list-style: none; border-bottom: 1px solid var(--border); }
.creation-steps li { position: relative; display: grid; grid-template-columns: 26px 1fr; gap: 1px 8px; align-items: center; color: var(--muted-foreground); font-size: 12px; }
.step-marker { grid-row: span 2; display: grid; place-items: center; width: 26px; height: 26px; border: 1px solid var(--border); background: var(--card); border-radius: 50%; font-size: 10px; font-variant-numeric: tabular-nums; }
.step-marker .app-icon { width: 14px; height: 14px; }
.creation-steps li > small { font-size: 11px; }
.creation-steps .is-current > span:not(.step-marker) { color: var(--progress-tone); font-weight: 700; }
.is-current .step-marker { background: var(--progress-tone); border-color: var(--progress-tone); color: var(--card); box-shadow: 0 0 0 4px color-mix(in srgb, var(--progress-tone) 8%, transparent); }
.is-done .step-marker { background: var(--accent); color: var(--primary); border-color: var(--accent); }
.stage-progress { padding: 18px 28px 0; }
.stage-progress-label { display: flex; justify-content: space-between; gap: 12px; margin-bottom: 9px; font-size: 12px; color: var(--muted-foreground); font-variant-numeric: tabular-nums; }
.stage-progress-label strong { color: var(--progress-tone); font-weight: 600; white-space: nowrap; }
.stage-progress-track { height: 4px; overflow: hidden; border-radius: 4px; background: var(--secondary); }
.stage-progress-track > span { display: block; height: 100%; background: var(--progress-tone); border-radius: inherit; transition: width .5s ease; }
.stage-progress-track.is-indeterminate > span { width: 30%; animation: processing 2.4s ease-in-out infinite; }
.creation-output { display: grid; grid-template-columns: minmax(150px, .7fr) minmax(0, 2fr); gap: 24px; margin: 22px 28px 24px; }
.output-summary { display: grid; grid-template-columns: 1fr; align-content: start; gap: 12px; }
.output-label { font-size: 11px; color: var(--muted-foreground); letter-spacing: .1em; }
.output-metric { display: flex; align-items: baseline; flex-wrap: wrap; gap: 10px; }
.output-metric > strong { font-size: 18px; font-weight: 600; font-variant-numeric: tabular-nums; line-height: 1.5; }
.output-metric strong small { font-size: 11px; font-weight: 400; }
.output-metric > span { color: var(--muted-foreground); font-size: 11px; }
.output-preview { min-width: 0; padding: 17px 20px; border: 1px solid var(--border); border-radius: 10px; background: color-mix(in srgb, var(--card) 85%, transparent); }
.preview-heading { display: flex; gap: 7px; align-items: center; font-size: 12px; font-weight: 600; margin-bottom: 14px; }
.preview-heading .app-icon { width: 15px; height: 15px; color: var(--primary); }
.preview-heading small { margin-left: auto; font-size: 10px; color: var(--muted-foreground); font-weight: 400; }
.subtitle-preview { display: grid; gap: 9px; padding: 0; margin: 0; list-style: none; }
.subtitle-preview li { display: flex; gap: 13px; font-size: 13px; line-height: 1.7; }
.subtitle-preview time { flex-shrink: 0; color: var(--muted-foreground); font-size: 11px; padding-top: 2px; font-variant-numeric: tabular-nums; }
.subtitle-preview li > span { overflow-wrap: anywhere; }
.scene-preview-list { display: grid; gap: 13px; }
.scene-preview { display: flex; gap: 12px; align-items: flex-start; }
.scene-number { display: grid; place-items: center; flex-shrink: 0; width: 30px; height: 36px; background: var(--accent); border-radius: 3px 6px 6px 3px; color: var(--primary); font-size: 11px; border-left: 2px solid color-mix(in srgb, var(--primary) 20%, transparent); }
.scene-preview strong { font-size: 13px; font-weight: 500; overflow-wrap: anywhere; }
.scene-preview p { margin: 3px 0 0; color: var(--muted-foreground); font-size: 11px; overflow-wrap: anywhere; }
.preview-placeholder { display: flex; min-height: 125px; align-items: center; justify-content: center; flex-direction: column; text-align: center; gap: 10px; color: var(--muted-foreground); }
.preview-placeholder .app-icon { width: 23px; height: 23px; color: var(--primary); opacity: .65; }
.preview-placeholder strong { font-size: 13px; font-weight: 500; }
.preview-placeholder p { margin: 0; font-size: 11px; }
.progress-note { display: flex; gap: 7px; align-items: flex-start; margin: 0; padding: 12px 28px; border-top: 1px solid var(--border); color: var(--muted-foreground); font-size: 11px; line-height: 1.8; background: color-mix(in srgb, var(--accent) 30%, var(--card)); }
.progress-note .app-icon { flex-shrink: 0; width: 14px; height: 14px; margin-top: 3px; }
.sync-warning { color: var(--color-destructive); }
@keyframes orbit { to { transform: rotate(270deg); } }
@keyframes processing { 0% { transform: translateX(-100%); } 100% { transform: translateX(440%); } }
@media (max-width: 1100px) { .progress-hero { flex-wrap: wrap; gap: 14px; } .elapsed-time { margin-left: 94px; margin-top: -6px; } .creation-steps li { grid-template-columns: 1fr; gap: 5px; justify-items: center; } .step-marker { grid-row: auto; } .creation-output { grid-template-columns: minmax(130px, .65fr) minmax(0, 2fr); gap: 16px; } }
@media (max-width: 640px) {
  .progress-hero { display: grid; grid-template-columns: 52px minmax(0, 1fr); gap: 12px 14px; padding: 20px; }
  .progress-emblem { width: 52px; height: 52px; grid-column: 1; grid-row: 1; }
  .progress-emblem > .app-icon { width: 21px; height: 21px; }
  .emblem-value { font-size: 18px; }
  .emblem-value small { font-size: 10px; }
  .progress-intro { grid-column: 1 / -1; grid-row: 2; }
  .progress-intro h3 { font-size: 20px; }
  .progress-intro p { font-size: 12px; }
  .elapsed-time { grid-column: 2; grid-row: 1; justify-self: end; align-self: center; margin: 0; }
  .creation-steps { gap: 4px; margin-inline: 20px; }
  .creation-steps li { font-size: 10px; }
  .creation-steps li > span:not(.step-marker) { max-width: 2em; text-align: center; line-height: 1.5; }
  .creation-steps li > small { display: none; }
  .stage-progress { padding-inline: 20px; }
  .stage-progress-label { flex-wrap: wrap; gap: 4px 12px; }
  .stage-progress-label strong { margin-left: auto; }
  .creation-output { grid-template-columns: minmax(0, 1fr); margin: 20px; }
  .output-summary { grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 10px; }
  .output-label { grid-column: 1 / -1; }
  .output-metric { flex-direction: column; gap: 2px; }
  .output-metric > strong { font-size: 16px; }
  .output-preview { padding: 15px; }
  .progress-note { padding-inline: 20px; }
}
@media (prefers-reduced-motion: reduce) { .progress-emblem > .emblem-ring, .stage-progress-track > span { animation: none !important; transition: none; } }
</style>
