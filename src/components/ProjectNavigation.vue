<script setup lang="ts">
import { RouterLink } from 'vue-router'
import { TooltipContent, TooltipPortal, TooltipProvider, TooltipRoot, TooltipTrigger } from 'reka-ui'
import AppIcon from './AppIcon.vue'
import { isTauri } from '@tauri-apps/api/core'
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useRoute } from 'vue-router'
import { projectProgress, type ProjectStage } from '../services/workflow'
const props = defineProps<{ id: string; editing?: boolean; busy?: boolean }>()
const route = useRoute()
const progress = ref<ProjectStage[]>([])
let disposed = false
let request = 0
async function refresh() {
  if (!isTauri()) return
  const current = ++request
  try {
    const result = await projectProgress(props.id)
    if (!disposed && current === request) progress.value = result
  } catch { if (!disposed && current === request) progress.value = [] }
}
const currentPath = computed(() => route.path.slice(`/projects/${props.id}`.length).replace(/^\//, ''))
function state(path: string): ProjectStage {
  if (path === currentPath.value && props.editing) return { path, status: '编辑中', tone: 'attention' }
  return progress.value.find(value => value.path === path) ?? { path, status: '状态待同步', tone: 'neutral' }
}
onMounted(refresh)
watch(() => props.busy, refresh)
onBeforeUnmount(() => { disposed = true })
const steps = [
  { path: '', label: '书籍资料', icon: 'book', description: '整理阅读素材' },
  { path: 'writing', label: '内容创作', icon: 'pen', description: '文案与音频制作' },
  { path: 'covers', label: '封面工坊', icon: 'image', description: '排版与比例导出' },
  { path: 'video', label: '视频制作', icon: 'video', description: '原声字幕与动画' },
  { path: 'publishing', label: '发布准备', icon: 'upload', description: '检查素材与发布' },
] as const
</script>

<template>
  <TooltipProvider :delay-duration="200">
    <nav
      class="project-navigation"
      aria-label="书籍创作流程"
    >
      <TooltipRoot
        v-for="(step, index) in steps"
        :key="step.path"
      >
        <TooltipTrigger as-child>
          <RouterLink
            :to="`/projects/${id}${step.path ? '/' + step.path : ''}`"
            exact-active-class="is-current"
            aria-current-value="step"
            :aria-label="`${step.label} · ${state(step.path).status}`"
          >
            <AppIcon :name="step.icon" />
            <span class="step-copy">{{ step.label }}</span>
            <span
              class="step-state"
              :data-tone="state(step.path).tone"
              aria-hidden="true"
            />
          </RouterLink>
        </TooltipTrigger>
        <TooltipPortal>
          <TooltipContent
            class="app-tooltip"
            :side-offset="8"
          >
            {{ state(step.path).status }} · {{ step.description }} · Ctrl / ⌘ {{ index + 1 }}
          </TooltipContent>
        </TooltipPortal>
      </TooltipRoot>
    </nav>
  </TooltipProvider>
</template>
