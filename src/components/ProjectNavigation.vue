<script setup lang="ts">
import { RouterLink, useRouter } from 'vue-router'
import { TooltipContent, TooltipPortal, TooltipProvider, TooltipRoot, TooltipTrigger } from 'reka-ui'
import AppIcon from './AppIcon.vue'
import { isTauri } from '@tauri-apps/api/core'
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useRoute } from 'vue-router'
import { projectProgress, type ProjectStage } from '../services/workflow'
import { listChapters } from '../services/projects'
import { chapterIdFromQuery, chapterQuery, type ChapterSummary } from '../types/chapter'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
const props = defineProps<{ id: string; editing?: boolean; busy?: boolean }>()
const route = useRoute()
const router = useRouter()
const chapterId = computed(() => chapterIdFromQuery(route.query.chapter))
const creatingChapter = computed(() => route.query.chapter === 'new')
const chapters = ref<ChapterSummary[]>([])
const sourceError = ref('')
const scopeSelection = computed({
  get: () => typeof route.query.chapter === 'string' ? route.query.chapter : 'book',
  set: (value: string) => {
    const query = { ...route.query }
    for (const key of ['writingVersion', 'audioJob', 'job', 'task', 'audio', 'videoJob', 'coverVersion', 'assist']) delete query[key]
    if (value === 'book') delete query.chapter
    else query.chapter = value
    void router.push({ path: value === 'new' ? `/projects/${props.id}` : route.path, query })
  },
})
const progress = ref<ProjectStage[]>([])
let disposed = false
let request = 0
async function refresh() {
  if (!isTauri()) return
  const current = ++request
  try {
    const [result, sources] = await Promise.all([projectProgress(props.id, chapterId.value), listChapters(props.id)])
    if (!disposed && current === request) { progress.value = result; chapters.value = sources; sourceError.value = '' }
  } catch { if (!disposed && current === request) { progress.value = []; sourceError.value = '章节与流程暂时读取失败，请刷新页面。' } }
}
const currentPath = computed(() => route.path.slice(`/projects/${props.id}`.length).replace(/^\//, ''))
function state(path: string): ProjectStage {
  if (path === currentPath.value && props.editing) return { path, status: '编辑中', tone: 'attention' }
  return progress.value.find(value => value.path === path) ?? { path, status: '状态待同步', tone: 'neutral' }
}
onMounted(refresh)
watch([() => props.busy, chapterId], refresh)
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
  <div class="mb-3 flex flex-wrap items-center gap-3">
    <label
      for="creation-scope"
      class="text-sm text-muted-foreground"
    >本次创作范围</label>
    <Select v-model="scopeSelection">
      <SelectTrigger
        id="creation-scope"
        class="w-full max-w-sm"
      >
        <SelectValue placeholder="整本书" />
      </SelectTrigger>
      <SelectContent>
        <SelectItem value="book">
          整本书
        </SelectItem>
        <SelectItem
          v-for="chapter in chapters"
          :key="chapter.id"
          :value="chapter.id"
        >
          {{ chapter.title }}
        </SelectItem>
        <SelectItem value="new">
          新增章节内容…
        </SelectItem>
      </SelectContent>
    </Select>
    <p
      v-if="sourceError"
      role="alert"
      class="text-sm text-destructive"
    >
      {{ sourceError }}
    </p>
  </div>
  <TooltipProvider :delay-duration="200">
    <nav
      class="project-navigation"
      :data-stage="currentPath"
      aria-label="书籍创作流程"
    >
      <TooltipRoot
        v-for="(step, index) in steps"
        :key="step.path"
      >
        <TooltipTrigger as-child>
          <RouterLink
            :to="{ path: `/projects/${id}${step.path && !creatingChapter ? '/' + step.path : ''}`, query: creatingChapter ? { chapter: 'new' } : chapterQuery(chapterId) }"
            :exact-active-class="creatingChapter && step.path ? '' : 'is-current'"
            :aria-current-value="creatingChapter && step.path ? false : 'step'"
            :data-step="step.path"
            :aria-label="`${step.label} · ${creatingChapter && step.path ? '请先保存章节' : state(step.path).status}`"
            :aria-disabled="creatingChapter && !!step.path"
          >
            <AppIcon :name="step.icon" />
            <span class="step-copy">{{ index + 1 }}. {{ step.label }}</span>
            <span
              class="step-state animate-ping"
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
