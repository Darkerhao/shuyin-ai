<script setup lang="ts">
import OperationFeedback from '../../components/OperationFeedback.vue'
import { computed, onMounted, ref } from 'vue'
import { RouterLink, useRoute, useRouter } from 'vue-router'
import { useQueryState } from '../../composables/useQueryState'
import { isDesktop, listProjects } from '../../services/projects'
import type { Project } from '../../types/project'
import { useOperation } from '../../composables/useOperation'
import AppIcon from '../../components/AppIcon.vue'
import { useWorkflow } from '../../composables/useWorkflow'
import { resumeLabel, resumeRoute } from '../../services/workflow'
import BookCard from '../../components/BookCard.vue'
import { Button, buttonVariants } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
const desktop = isDesktop()
const projects = ref<Project[]>([])
const route = useRoute()
const router = useRouter()
const search = useQueryState('q', '')
const filter = useQueryState('status', 'all', ['all', 'active', 'published'])
const sort = useQueryState('sort', 'updated', ['updated', 'title'])
const view = useQueryState('view', 'grid', ['grid', 'list'])
const loaded = ref(false)
const { snapshot: workflow, refresh: refreshWorkflow } = useWorkflow()
const continuation = (id: string) => workflow.value.resume.find(task => task.projectId === id)
const continueTo = (id: string) => { const task = continuation(id); return task ? resumeRoute(task) : `/projects/${id}/writing` }
const { busy, error, run } = useOperation()
const load = () => run(async () => { projects.value = await listProjects(); loaded.value = true; await refreshWorkflow() })
const filtered = computed(() => {
  const query = search.value.trim().toLocaleLowerCase()
  return projects.value.filter(project => {
    const matches = [project.title, project.author, project.isbn].some(value => value.toLocaleLowerCase().includes(query))
    return matches && (filter.value === 'all' || (filter.value === 'published' ? project.status === 'published' : project.status !== 'published' && project.status !== 'archived'))
  }).sort((a, b) => sort.value === 'title' ? a.title.localeCompare(b.title, 'zh-CN') : b.updatedAt.localeCompare(a.updatedAt))
})
const filters = computed(() => [
  { value: 'all', label: '全部书籍', count: projects.value.length },
  { value: 'active', label: '创作中', count: projects.value.filter(p => p.status !== 'published' && p.status !== 'archived').length },
  { value: 'published', label: '已发布', count: projects.value.filter(p => p.status === 'published').length },
] as const)
const recent = computed(() => projects.value.filter(p => p.status !== 'archived' && (p.status !== 'published' || continuation(p.id)?.running)).sort((a, b) => {
  const left = continuation(a.id), right = continuation(b.id)
  return Number(right?.running ?? false) - Number(left?.running ?? false) || (right?.updatedAt ?? b.updatedAt).localeCompare(left?.updatedAt ?? a.updatedAt)
})[0])
function clearFilters() { void router.replace({ query: { ...route.query, q: undefined, status: undefined } }) }
const date = (value: string) => new Date(value).toLocaleDateString('zh-CN', { month: 'long', day: 'numeric' })
onMounted(() => { if (desktop) void load() })

</script>

<template>
  <header class="library-heading">
    <div>
      <h1>
        我的书籍<span
          v-if="loaded"
          class="library-count"
          aria-hidden="true"
        >共 {{ projects.length }} 本</span>
      </h1>
      <p>
        把读过的好书，变成值得分享的内容。
      </p>
    </div>
    <div
      v-if="desktop"
      class="flex flex-wrap gap-3"
    >
      <RouterLink
        to="/publishing"
        :class="buttonVariants({ variant: 'outline' })"
      >
        <AppIcon name="upload" />发布任务
      </RouterLink>
      <RouterLink
        to="/projects/new"
        :class="[buttonVariants(), 'bg-gradient-to-r from-[#34d399] to-[#14b8a6]']"
      >
        <AppIcon name="plus" />添加一本书
      </RouterLink>
    </div>
    <Button
      v-else
      disabled
    >
      <AppIcon name="plus" />添加一本书
    </Button>
  </header>
  <p
    v-if="!desktop"
    class="mb-6 flex items-start gap-2 rounded-lg border bg-muted/50 px-4 py-3 text-sm text-muted-foreground"
    role="status"
  >
    <AppIcon
      name="info"
      class="mt-0.5"
    />当前为浏览器预览。请在桌面应用中管理书籍资料。
  </p>
  <section
    v-if="loaded && recent && !search && filter === 'all'"
    class="resume-project"
    aria-label="最近创作"
  >
    <span class="resume-icon"><small>ACTIVE</small><AppIcon name="pen" /><span>{{ recent.author || '作者待补充' }}</span></span>
    <div class="min-w-0 flex-1">
      <p class="resume-label">
        <i class="status-dot" />接着上次的灵感继续创作
      </p>
      <h2>{{ recent.title }}</h2>
      <p class="resume-meta">
        {{ recent.author || '作者待补充' }} · {{ date(recent.updatedAt) }}更新
      </p>
    </div>
    <RouterLink
      :to="continueTo(recent.id)"
      :class="buttonVariants()"
    >
      {{ continuation(recent.id) ? resumeLabel(continuation(recent.id)!) : '继续创作' }}<AppIcon name="arrow" />
    </RouterLink>
  </section>
  <div class="library-toolbar">
    <div
      class="library-filters"
      aria-label="书籍筛选"
    >
      <Button
        v-for="item in filters"
        :key="item.value"
        type="button"
        variant="ghost"
        size="sm"
        :class="filter === item.value ? 'bg-accent text-primary hover:bg-accent' : 'text-muted-foreground'"
        :aria-pressed="filter === item.value"
        :disabled="!desktop"
        @click="filter = item.value"
      >
        {{ item.label }} ({{ item.count }})
      </Button>
    </div>
    <form
      class="library-search"
      role="search"
      @submit.prevent
    >
      <div class="relative">
        <label
          class="sr-only"
          for="search"
        >搜索书名、作者或 ISBN</label>
        <AppIcon
          name="search"
          class="pointer-events-none absolute left-3 top-2.5 size-4 text-muted-foreground"
        />
        <Input
          id="search"
          v-model="search"
          name="q"
          type="search"
          autocomplete="off"
          class="pl-9 pr-9"
          placeholder="书名、作者或 ISBN"
          :disabled="!desktop"
          @keydown.esc="search = ''"
        />
        <Button
          v-if="search"
          type="button"
          variant="ghost"
          size="icon-sm"
          class="absolute right-0.5 top-0.5 text-muted-foreground"
          aria-label="清空搜索"
          :disabled="!desktop"
          @click="search = ''"
        >
          <AppIcon name="close" />
        </Button>
      </div>
      <Select
        v-model="sort"
        :disabled="!desktop"
      >
        <SelectTrigger
          id="book-sort"
          aria-label="书籍排序"
        >
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          <SelectItem value="updated">
            最近更新
          </SelectItem>
          <SelectItem value="title">
            书名排序
          </SelectItem>
        </SelectContent>
      </Select>
      <Button
        type="button"
        variant="outline"
        size="icon"
        class="size-[34px]"
        :disabled="!desktop || busy"
        aria-label="刷新书架"
        title="刷新书架"
        @click="load"
      >
        <AppIcon
          name="refresh"
          :class="{ spin: busy }"
        />
      </Button>
      <div
        class="library-views"
        role="group"
        aria-label="书架显示方式"
      >
        <Button
          v-for="mode in (['grid', 'list'] as const)"
          :key="mode"
          variant="ghost"
          size="icon-sm"
          class="h-6 w-7"
          :aria-label="mode === 'grid' ? '网格视图' : '列表视图'"
          :title="mode === 'grid' ? '网格视图' : '列表视图'"
          :aria-pressed="view === mode"
          :disabled="!desktop || !loaded"
          @click="view = mode"
        >
          <AppIcon :name="mode" />
        </Button>
      </div>
    </form>
  </div>
  <OperationFeedback :error="error ? `${error} · 点击刷新书架重试，已有内容保留。` : ''" />
  <p
    v-if="busy && !loaded"
    class="loading-state"
    role="status"
  >
    <AppIcon
      name="refresh"
      class="spin"
    />正在读取你的书架…
  </p>
  <template v-if="loaded">
    <p
      class="sr-only"
      role="status"
      aria-live="polite"
    >
      {{ search ? '搜索“' + search + '” · ' : '' }}共 {{ filtered.length }} 本书
    </p>
    <section
      v-if="filtered.length"
      class="library-grid"
      :class="{ 'is-list': view === 'list' }"
      aria-label="项目列表"
      :aria-busy="busy"
    >
      <BookCard
        v-for="project in filtered"
        :key="project.id"
        :project="project"
        :resume="continuation(project.id)"
        @deleted="load"
      />
      <RouterLink
        v-if="!search && filter === 'all'"
        class="add-project-card flex min-h-72 flex-col items-center justify-center gap-4 rounded-xl border border-dashed bg-muted/20 p-8 text-center text-muted-foreground transition-colors hover:border-primary/40 hover:bg-accent/50 hover:text-primary"
        to="/projects/new"
      >
        <span class="flex size-12 items-center justify-center rounded-xl border bg-card"><AppIcon name="plus" /></span>
        <div>
          <p class="m-0 text-sm font-medium text-foreground">
            开启下一本书
          </p><p class="mb-0 mt-2 text-xs">
            新的故事，从这里开始
          </p>
        </div>
      </RouterLink>
    </section>
    <section
      v-else
      class="flex flex-col items-center rounded-xl border border-dashed bg-card px-6 py-20 text-center"
    >
      <span class="mb-5 flex size-14 items-center justify-center rounded-2xl bg-muted text-primary"><AppIcon :name="search || filter !== 'all' ? 'search' : 'book'" /></span>
      <h2 class="mb-2 text-lg font-semibold">
        {{ search || filter !== 'all' ? '没有匹配的项目' : '你的书架，等待第一本好书' }}
      </h2>
      <p class="mb-6 mt-0 text-sm text-muted-foreground">
        {{ search || filter !== 'all' ? '换个关键词，或查看全部书籍。' : '只需一个书名，就可以开始准备下一期内容。' }}
      </p>
      <Button
        v-if="search || filter !== 'all'"
        variant="outline"
        @click="clearFilters"
      >
        查看全部书籍
      </Button>
      <RouterLink
        v-else
        :class="buttonVariants()"
        to="/projects/new"
      >
        <AppIcon name="plus" />添加第一本书
      </RouterLink>
    </section>
  </template>
</template>
