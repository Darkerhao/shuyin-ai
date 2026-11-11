<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { RouterLink } from 'vue-router'
import { isDesktop } from '../services/projects'
import { automationRuns, automationError, refreshAutomation, renderAutomationCover, controlAutomation, stages, type AutomationRun } from '../services/automation'
import { useWindowCloseGuard } from '../composables/useWindowCloseGuard'
import { requestConfirmation } from '../composables/useConfirmation'
import { openDouyinWindow } from '../services/douyin'
import { Button } from '@/components/ui/button'
import AppIcon from './AppIcon.vue'

const phases = [
  { label: '创作方案', icon: 'pen', stages: ['duration', 'writing'] },
  { label: '原书音频', icon: 'headphones', stages: ['audio'] },
  { label: '字幕与校核', icon: 'shield', stages: ['transcript', 'content_check'] },
  { label: '三尺寸封面', icon: 'image', stages: ['landscape_image', 'landscape_cover', 'portrait_image', 'portrait_cover', 'widescreen_image', 'widescreen_cover'] },
  { label: '分镜与成片', icon: 'video', stages: ['storyboard', 'render'] },
  { label: '发布准备', icon: 'upload', stages: ['publishing'] },
] as const
const phaseIndex = (run: AutomationRun) => run.input.workflowVersion === 2 ? phases.findIndex(phase => (phase.stages as readonly string[]).includes(run.stage)) : -1
function outputs(run: AutomationRun) {
  const project = `/projects/${run.projectId}`
  const entries = [
    { key: 'writing', id: run.writingVersionId, label: '本次创作方案', icon: 'pen', to: `${project}/writing?writingVersion=${run.writingVersionId}` },
    { key: 'checked-writing', id: run.finalWritingVersionId, label: '已校核发布材料', icon: 'shield', to: `${project}/writing?writingVersion=${run.finalWritingVersionId}` },
    { key: 'audio', id: run.audioJobId, label: '音频任务', icon: 'headphones', to: `${project}/writing?tab=audio&audioJob=${run.audioJobId}` },
    { key: 'video', id: run.videoJobId, label: '视频任务', icon: 'video', to: `${project}/video?job=${run.videoJobId}` },
    ...[{ id: run.landscapeCoverId, label: '4:3' }, { id: run.portraitCoverId, label: '3:4' }, { id: run.widescreenCoverId, label: '16:9' }].map(cover => ({ key: `cover-${cover.label}`, id: cover.id, label: `${cover.label} 成品封面`, icon: 'image' as const, to: `${project}/covers?coverVersion=${cover.id}` })),
    { key: 'publishing', id: run.publishTaskId, label: '发布详情与结果记录', icon: 'upload', to: `${project}/publishing?task=${run.publishTaskId}` },
  ] as const
  return entries.filter(output => output.id)
}

const active = computed(() => automationRuns.value.filter(run => !['completed', 'cancelled'].includes(run.status)))
const working = computed(() => active.value.some(run => run.status === 'running'))
const busy = ref(false)
const error = ref('')
function audioDuration(run: AutomationRun) {
  const seconds = Math.round(run.actualDurationSeconds ?? 0)
  const target = run.input.targetDurationSeconds
  const difference = target === null ? '' : seconds === target ? ' · 与目标一致' : ` · 比目标${seconds > target ? '长' : '短'} ${Math.abs(seconds - target)} 秒`
  return `${Math.floor(seconds / 60)} 分 ${seconds % 60} 秒${difference}`
}
useWindowCloseGuard(() => working.value, () => !working.value || requestConfirmation('自动制作仍在进行。退出会中断本地执行，重开后可核对并继续。确定退出？'))
let disposed = false
let timer: ReturnType<typeof globalThis.setTimeout> | undefined
async function poll() {
  if (disposed) return
  await refreshAutomation()
  if (disposed) return
  for (const run of active.value) void renderAutomationCover(run)
  timer = globalThis.setTimeout(() => { void poll() }, 1500)
}
async function act(run: AutomationRun, action: 'pause' | 'resume' | 'cancel' | 'retry') {
  if (action === 'retry' && !await requestConfirmation('重新尝试当前失败阶段：每个模型失败后最多自动重试3次，再自动切换其他已启用的同类模型，可能产生额外费用。已有成功结果保留，确认重试？')) return
  if (action === 'cancel' && !await requestConfirmation('停止流程后保留全部成果。已开始的音频、生成或上传可能仍会完成，停止不等于撤稿。确定停止？')) return
  busy.value = true; error.value = ''
  try { await controlAutomation(run.id, action); await refreshAutomation() }
  catch (cause) { error.value = String(cause) }
  finally { busy.value = false }
}
async function showPublishing() {
  try { await openDouyinWindow() } catch (cause) { error.value = String(cause) }
}
onMounted(() => { if (isDesktop()) void poll() })
onBeforeUnmount(() => { disposed = true; globalThis.clearTimeout(timer) })
</script>

<template>
  <aside
    v-if="active.length || automationError || error"
    class="automation-studio"
    aria-label="自动制作与发布"
  >
    <p
      v-if="automationError || error"
      role="alert"
    >
      {{ automationError || error }}
    </p>
    <article
      v-for="run in active"
      :key="run.id"
      class="automation-run"
      :data-status="run.status"
    >
      <div class="automation-heading">
        <span class="automation-emblem"><AppIcon name="sparkle" /></span>
        <RouterLink
          :to="`/projects/${run.projectId}`"
          class="automation-title"
        >
          {{ run.input.title }} · 自动制作
        </RouterLink>
        <span
          role="status"
          class="automation-status"
        ><span
          class="automation-status-dot"
          aria-hidden="true"
        />{{ run.status === 'awaiting_publish' ? '等待最终发布' : `${run.status === 'running' ? '' : run.status === 'paused' ? '已暂停 · ' : '需要处理 · '}${stages[run.stage] ?? run.stage}` }}</span>
      </div>
      <ol
        v-if="phaseIndex(run) >= 0"
        class="automation-phases"
        aria-label="自动制作阶段"
      >
        <li
          v-for="(phase, index) in phases"
          :key="phase.label"
          :data-state="index < phaseIndex(run) ? 'done' : index === phaseIndex(run) ? 'current' : 'pending'"
          :aria-current="index === phaseIndex(run) ? 'step' : undefined"
        >
          <span class="phase-marker"><AppIcon :name="index < phaseIndex(run) ? 'check' : phase.icon" /></span>
          <span>{{ phase.label }}<small>{{ index < phaseIndex(run) ? '已完成' : index === phaseIndex(run) ? run.status === 'running' ? '进行中' : run.status === 'paused' ? '已暂停' : '待确认' : '待开始' }}</small></span>
        </li>
      </ol>
      <div class="automation-summary">
        <p v-if="run.input.account">
          发布账号：{{ run.input.account.name || '启动时已核对的账号' }}
          · 合集：{{ run.input.selection.collectionName ?? '不加入合集' }}
        </p>
        <p v-if="run.input.textProvider && run.input.imageProvider">
          当前文本模型：{{ run.input.textProvider.name }} · {{ run.input.textProvider.modelName }}
          · 当前图片模型：{{ run.input.imageProvider.name }} · {{ run.input.imageProvider.modelName }}
        </p>
        <p>
          {{ run.input.targetDurationSeconds ? `目标 ${run.input.targetDurationSeconds / 60} 分钟` : '目标时长待推荐' }}<span v-if="run.input.durationReason"> · {{ run.input.durationReason }}</span>
        </p>
        <p
          v-if="run.actualDurationSeconds"
        >
          实际音频 {{ audioDuration(run) }}
          · 封面已导出 {{ [run.landscapeCoverId, run.portraitCoverId, run.widescreenCoverId].filter(Boolean).length }} / {{ run.input.workflowVersion === 2 ? 3 : 2 }}
        </p>
      </div>
      <p
        class="automation-note"
        role="status"
        :class="{ 'needs-attention': run.status === 'waiting_user' || run.status === 'paused' }"
      >
        <AppIcon :name="run.status === 'running' ? 'info' : 'pause'" />{{ run.error || '保持应用运行，可切换页面。完成后请在抖音窗口检查并亲自发布。' }}
      </p>
      <TransitionGroup
        name="task-output"
        tag="div"
        class="automation-outputs"
        aria-label="本次任务与成果"
      >
        <RouterLink
          v-for="output in outputs(run)"
          :key="output.key"
          :to="output.to"
          class="automation-output"
        >
          <AppIcon :name="output.icon" />
          <span>{{ output.label }}</span><AppIcon name="chevron" />
        </RouterLink>
      </TransitionGroup>
      <div class="automation-actions">
        <Button
          v-if="run.status === 'running'"
          variant="outline"
          :disabled="busy"
          @click="act(run, 'pause')"
        >
          暂停后续步骤
        </Button>
        <Button
          v-else-if="run.status !== 'awaiting_publish' && run.input.workflowVersion === 2"
          :disabled="busy"
          @click="act(run, 'resume')"
        >
          继续流程
        </Button>
        <Button
          v-if="run.input.workflowVersion === 2 && ['paused', 'waiting_user'].includes(run.status) && ['duration', 'writing', 'content_check', 'landscape_image', 'portrait_image', 'widescreen_image', 'transcript', 'storyboard', 'render'].includes(run.stage)"
          variant="outline"
          :disabled="busy"
          @click="act(run, 'retry')"
        >
          重试当前阶段
        </Button>
        <Button
          v-if="run.publishTaskId"
          variant="outline"
          @click="showPublishing"
        >
          显示抖音窗口
        </Button>
        <Button
          variant="ghost"
          :disabled="busy"
          @click="act(run, 'cancel')"
        >
          停止流程
        </Button>
      </div>
    </article>
  </aside>
</template>

<style scoped>
.automation-studio { margin-bottom: 24px; border: 1px solid var(--active-border); border-radius: 16px; background: var(--panel-surface); box-shadow: var(--panel-shadow); overflow: hidden; }
.automation-studio > [role='alert'] { margin: 0; padding: 16px 20px; color: var(--destructive); }
.automation-run { --task-tone: var(--primary); min-width: 0; padding: 20px; }
.automation-run + .automation-run { border-top: 1px solid var(--border); }
.automation-run[data-status='paused'], .automation-run[data-status='waiting_user'] { --task-tone: var(--warning); }
.automation-run[data-status='awaiting_publish'] { --task-tone: var(--info); }
.automation-heading { display: flex; align-items: center; flex-wrap: wrap; gap: 12px; }
.automation-emblem { display: grid; place-items: center; width: 38px; height: 38px; border-radius: 12px; background: color-mix(in srgb, var(--task-tone) 10%, var(--card)); border: 1px solid color-mix(in srgb, var(--task-tone) 25%, var(--border)); color: var(--task-tone); }
.automation-emblem .app-icon { width: 20px; height: 20px; }
.automation-title { flex: 1; min-width: 130px; font-size: 15px; font-weight: 700; color: var(--heading); text-decoration: none; overflow-wrap: anywhere; }
.automation-title:hover { color: var(--task-tone); }
.automation-status { display: flex; align-items: center; gap: 8px; font-size: 12px; color: var(--task-tone); }
.automation-status-dot { width: 6px; height: 6px; flex-shrink: 0; border-radius: 50%; background: currentColor; }
.automation-run[data-status='running'] .automation-status-dot { animation: task-breathe 2s ease-in-out infinite; }
.automation-phases { display: grid; grid-template-columns: repeat(6, minmax(0, 1fr)); gap: 6px; margin: 20px 0 16px; padding: 0; list-style: none; }
.automation-phases li { display: flex; align-items: center; gap: 8px; min-width: 0; padding: 10px 8px; border-top: 2px solid var(--border); color: var(--muted-foreground); font-size: 12px; transition: border-color 200ms, background-color 200ms; }
.phase-marker { display: grid; place-items: center; flex-shrink: 0; width: 28px; height: 28px; border-radius: 50%; background: var(--input-surface); }
.phase-marker .app-icon { width: 14px; height: 14px; }
.automation-phases small { display: block; margin-top: 3px; font-size: 11px; color: var(--muted-foreground); }
.automation-phases [data-state='done'] { border-color: color-mix(in srgb, var(--primary) 45%, var(--border)); }
.automation-phases [data-state='done'] .phase-marker { color: var(--primary); background: var(--accent); }
.automation-phases [data-state='current'] { border-color: var(--task-tone); background: color-mix(in srgb, var(--task-tone) 6%, transparent); border-radius: 0 0 8px 8px; color: var(--foreground); }
.automation-phases [data-state='current'] .phase-marker { color: var(--task-tone); box-shadow: inset 0 0 0 1px currentColor; }
.automation-summary { display: flex; flex-wrap: wrap; gap: 6px 24px; margin-top: 14px; color: var(--text-secondary); font-size: 12px; }
.automation-summary p { margin: 0; }
.automation-note { display: flex; align-items: flex-start; gap: 8px; margin: 10px 0 0; font-size: 12px; line-height: 1.7; color: var(--muted-foreground); overflow-wrap: anywhere; }
.automation-note .app-icon { flex-shrink: 0; width: 15px; height: 15px; margin-top: 3px; }
.automation-note.needs-attention { color: var(--warning); }
.automation-outputs { display: grid; grid-template-columns: repeat(auto-fit, minmax(170px, 1fr)); gap: 8px; }
.automation-outputs:not(:empty) { margin-top: 16px; }
.automation-output { display: flex; align-items: center; gap: 9px; min-width: 0; padding: 12px; border: 1px solid var(--border); border-radius: 10px; background: var(--input-surface); color: var(--text-secondary); font-size: 12px; text-decoration: none; transition: border-color 180ms, background-color 180ms; }
.automation-output span { flex: 1; }
.automation-output .app-icon { width: 16px; height: 16px; flex-shrink: 0; color: var(--info); }
.automation-output .app-icon:last-child { width: 12px; color: var(--muted-foreground); }
.automation-output:hover { border-color: var(--info); background: var(--card); }
.automation-actions { display: flex; align-items: center; flex-wrap: wrap; gap: 8px; margin-top: 16px; padding-top: 14px; border-top: 1px solid var(--border); }
.task-output-enter-active { transition: opacity 200ms, transform 200ms; }
.task-output-enter-from { opacity: 0; transform: translateY(6px); }
@keyframes task-breathe { 50% { opacity: .45; } }
@media (max-width: 1100px) { .automation-phases { grid-template-columns: repeat(3, minmax(0, 1fr)); } }
@media (max-width: 560px) { .automation-run { padding: 16px; } .automation-phases { grid-template-columns: repeat(2, minmax(0, 1fr)); } .automation-status { flex-basis: 100%; } .automation-outputs { grid-template-columns: minmax(0, 1fr); } }
@media (prefers-reduced-motion: reduce) { .automation-status-dot { animation: none !important; } .task-output-enter-active { transition: none; } .task-output-enter-from { transform: none; opacity: 1; } }
</style>
