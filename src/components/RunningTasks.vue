<script setup lang="ts">
import { computed, onMounted, onBeforeUnmount } from 'vue'
import { RouterLink } from 'vue-router'
import { isDesktop } from '../services/projects'
import { useWorkflow } from '../composables/useWorkflow'
import { resumeLabel, resumeRoute } from '../services/workflow'
import { generationJobs, isGenerating } from '../services/generation'
import { useWindowCloseGuard } from '../composables/useWindowCloseGuard'
import { requestConfirmation } from '../composables/useConfirmation'
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
    class="mb-5 rounded-xl border bg-accent p-4"
    aria-label="正在运行的任务"
  >
    <p
      v-if="error || closeGuard.error.value"
      role="status"
    >
      {{ error || closeGuard.error.value }}
    </p>
    <ul
      v-if="running.length || jobs.length"
      class="grid gap-2"
    >
      <li
        v-for="task in running"
        :key="`${task.kind}:${task.taskId}`"
      >
        <RouterLink
          :to="resumeRoute(task)"
          class="underline"
        >
          {{ task.title }} · {{ resumeLabel(task) }} →
        </RouterLink>
      </li>
      <li
        v-for="job in jobs"
        :key="`${job.projectId}:${job.kind}`"
      >
        <RouterLink
          :to="`/projects/${job.projectId}/${job.kind === 'cover' ? 'covers' : 'writing'}`"
          class="underline"
        >
          {{ job.title }} · {{ job.kind === 'cover' ? '封面背景' : job.mode === 'revision' ? '按项调整' : job.mode === 'duration' ? '时长推荐' : '文案生成' }} · {{ isGenerating(job) ? '生成中，可继续其他创作' : job.status === 'failed' ? '未完成，查看原因' : '已完成，查看结果' }} →
        </RouterLink>
      </li>
    </ul>
  </aside>
</template>
