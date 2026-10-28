<script setup lang="ts">
import { RouterLink } from 'vue-router'
import AppIcon from './AppIcon.vue'
defineProps<{ id: string }>()
const steps = [
  { path: '', label: '书籍资料', icon: 'book', description: '整理阅读素材' },
  { path: 'writing', label: '内容创作', icon: 'pen', description: '文案与音频制作' },
  { path: 'covers', label: '封面工坊', icon: 'image', description: '排版与比例导出' },
  { path: 'publishing', label: '发布准备', icon: 'upload', description: '检查素材与发布' },
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
      :to="`/projects/${id}${step.path ? '/' + step.path : ''}`"
      exact-active-class="is-current"
      aria-current-value="step"
    >
      <span class="step-number">0{{ index + 1 }}</span>
      <AppIcon :name="step.icon" />
      <span class="step-copy"><strong>{{ step.label }}</strong><small>{{ step.description }}</small></span>
      <AppIcon
        v-if="index < steps.length - 1"
        name="chevron"
        class="step-chevron"
      />
    </RouterLink>
  </nav>
</template>
