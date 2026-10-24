<script setup lang="ts">
import { RouterLink } from 'vue-router'
import AppIcon from './AppIcon.vue'
defineProps<{ id: string }>()
const steps = [
  { path: 'writing', label: '内容创作', icon: 'pen' },
  { path: 'covers', label: '封面工坊', icon: 'image' },
  { path: 'publishing', label: '发布准备', icon: 'upload' },
] as const
</script>

<template>
  <nav
    class="project-navigation"
    aria-label="书籍创作流程"
  >
    <RouterLink
      v-for="(step, index) in steps"
      :key="step.path"
      :to="`/projects/${id}/${step.path}`"
    >
      <span class="step-number">0{{ index + 1 }}</span>
      <AppIcon :name="step.icon" />
      <span>{{ step.label }}</span>
      <AppIcon
        v-if="index < steps.length - 1"
        name="chevron"
        class="step-chevron"
      />
    </RouterLink>
  </nav>
</template>
