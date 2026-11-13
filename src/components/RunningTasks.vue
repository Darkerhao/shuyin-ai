<script setup lang="ts">
import { computed, onMounted, onBeforeUnmount } from 'vue'
import { RouterLink } from 'vue-router'
import { isDesktop } from '../services/projects'
import { useWorkflow } from '../composables/useWorkflow'
import { resumeLabel, resumeRoute } from '../services/workflow'
import { generationJobs, isGenerating } from '../services/generation'
import { chapterQuery } from '../types/chapter'
import { useWindowCloseGuard } from '../composables/useWindowCloseGuard'
import { requestConfirmation } from '../composables/useConfirmation'
import AppIcon from './AppIcon.vue'
const { snapshot, error, refresh } = useWorkflow()
const jobs = computed(() => [...generationJobs.values()].filter(job => job.status !== 'cancelled'))
const running = computed(() => snapshot.value.running.filter(task => !jobs.value.some(job => job.id === task.taskId)))
const generating = computed(() => jobs.value.some(isGenerating))
const closeGuard = useWindowCloseGuard(() => generating.value, () => !generating.value || requestConfirmation('仍有 AI 生成任务。关闭应用会中断本地请求，已保存内容保留。确定关闭？'))
let timer: ReturnType<typeof globalThis.setInterval> | undefined
onMounted(() => { if (isDesktop()) { void refresh(); timer = globalThis.setInterval(() => { void refresh() }, 3000) } })
onBeforeUnmount(() => { globalThis.clearInterval(timer) })
</script>
<template>
  <aside
    v-if="running.length || jobs.length || error || closeGuard.error.value"
    class="running-tasks"
    aria-label="正在运行的任务"
  >
    <p
      v-if="error || closeGuard.error.value"
      role="status"
    >
      {{ error || closeGuard.error.value }}
    </p>
    <div class="running-tasks-heading">
      <AppIcon name="bolt" /><strong>创作任务</strong><span>随时回来查看结果</span>
    </div>
    <TransitionGroup
      v-if="running.length || jobs.length"
      name="running-task"
      tag="ul"
      class="running-tasks-list"
    >
      <li
        v-for="task in running"
        :key="`${task.kind}:${task.taskId}`"
      >
        <RouterLink
          :to="resumeRoute(task)"
          class="running-task-link"
        >
          <AppIcon name="clock" /><span><strong>{{ task.title }}</strong><small>{{ resumeLabel(task) }}</small></span><AppIcon name="chevron" />
        </RouterLink>
      </li>
      <li
        v-for="job in jobs"
        :key="`${job.projectId}:${job.chapterId ?? 'book'}:${job.kind}`"
      >
        <RouterLink
          :to="{ path: `/projects/${job.projectId}/${job.kind === 'cover' ? 'covers' : 'writing'}`, query: chapterQuery(job.chapterId ?? null) }"
          class="running-task-link"
          :data-status="isGenerating(job) ? 'running' : job.status === 'failed' ? 'failed' : 'done'"
        >
          <AppIcon :name="isGenerating(job) ? 'clock' : job.status === 'failed' ? 'info' : 'check'" />
          <span><strong>{{ job.title }} · {{ job.kind === 'cover' ? '封面背景' : job.mode === 'revision' ? '按项调整' : job.mode === 'duration' ? '时长推荐' : '文案生成' }}</strong><small>{{ isGenerating(job) ? '生成中，可继续其他创作' : job.status === 'failed' ? '未完成，查看原因' : '已完成，查看结果' }}</small></span><AppIcon name="chevron" />
        </RouterLink>
      </li>
    </TransitionGroup>
  </aside>
</template>

<style scoped>
.running-tasks { margin-bottom: 20px; padding: 16px; border: 1px solid var(--border); border-radius: 14px; background: var(--panel-surface); }
.running-tasks-heading { display: flex; align-items: center; flex-wrap: wrap; gap: 8px; margin-bottom: 12px; font-size: 12px; }
.running-tasks-heading > .app-icon { width: 15px; color: var(--info); }
.running-tasks-heading > span { margin-left: auto; color: var(--muted-foreground); }
.running-tasks-list { display: grid; grid-template-columns: repeat(auto-fit, minmax(min(100%, 240px), 1fr)); gap: 8px; padding: 0; margin: 0; list-style: none; }
.running-task-link { display: flex; align-items: center; gap: 10px; height: 100%; padding: 12px; border: 1px solid var(--border); border-radius: 10px; background: var(--input-surface); text-decoration: none; transition: border-color 180ms, background-color 180ms; }
.running-task-link > .app-icon { flex-shrink: 0; width: 16px; height: 16px; color: var(--info); }
.running-task-link[data-status='done'] > .app-icon:first-child { color: var(--primary); }
.running-task-link[data-status='failed'] > .app-icon:first-child { color: var(--warning); }
.running-task-link > span { flex: 1; min-width: 0; overflow-wrap: anywhere; }
.running-task-link strong { font-size: 12px; font-weight: 600; color: var(--foreground); }
.running-task-link small { display: block; margin-top: 4px; font-size: 12px; color: var(--muted-foreground); }
.running-task-link > .app-icon:last-child { width: 12px; color: var(--muted-foreground); }
.running-task-link:hover { border-color: var(--info); background: var(--card); }
.running-task-move, .running-task-enter-active { transition: transform 200ms, opacity 200ms; }
.running-task-enter-from { transform: translateY(5px); opacity: 0; }
@media (prefers-reduced-motion: reduce) { .running-task-enter-from { transform: none; opacity: 1; } }
</style>
