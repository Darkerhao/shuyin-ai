<script setup lang="ts">
import { onMounted, onBeforeUnmount } from 'vue'
import { RouterLink } from 'vue-router'
import { isDesktop } from '../services/projects'
import { useWorkflow } from '../composables/useWorkflow'
import { resumeLabel, resumeRoute } from '../services/workflow'
const { snapshot, error, refresh } = useWorkflow()
let timer: ReturnType<typeof globalThis.setInterval> | undefined
onMounted(() => { if (isDesktop()) { void refresh(); timer = globalThis.setInterval(() => { void refresh() }, 3000) } })
onBeforeUnmount(() => { globalThis.clearInterval(timer) })
</script>
<template>
  <aside
    v-if="snapshot.running.length || error"
    class="mb-5 rounded-xl border bg-accent p-4"
    aria-label="正在运行的任务"
  >
    <p
      v-if="error"
      role="status"
    >
      {{ error }}
    </p>
    <ul
      v-if="snapshot.running.length"
      class="grid gap-2"
    >
      <li
        v-for="task in snapshot.running"
        :key="`${task.kind}:${task.taskId}`"
      >
        <RouterLink
          :to="resumeRoute(task)"
          class="underline"
        >
          {{ task.title }} · {{ resumeLabel(task) }} →
        </RouterLink>
      </li>
    </ul>
  </aside>
</template>
