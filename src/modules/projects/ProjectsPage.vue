<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { RouterLink } from 'vue-router'
import { isDesktop, listProjects } from '../../services/projects'
import { projectStatuses, type Project } from '../../types/project'
import { useOperation } from '../../composables/useOperation'
import AppIcon from '../../components/AppIcon.vue'
import { Badge } from '@/components/ui/badge'
import { Button, buttonVariants } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
const desktop = isDesktop()
const projects = ref<Project[]>([])
const search = ref('')
const filter = ref('all')
const sort = ref('updated')
const loaded = ref(false)
const { busy, error, run } = useOperation()
const load = () => run(async () => { projects.value = await listProjects(); loaded.value = true })
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
])
function clearFilters() { search.value = ''; filter.value = 'all' }
function coverTone(id: string) { return [...id].reduce((sum, char) => sum + char.charCodeAt(0), 0) % 4 }
const date = (value: string) => new Date(value).toLocaleDateString('zh-CN', { month: 'long', day: 'numeric' })
onMounted(() => { if (desktop) void load() })

</script>

<template>
  <header class="mb-8 flex flex-wrap items-center justify-between gap-5">
    <div>
      <h1 class="m-0 text-2xl font-semibold tracking-tight">
        我的书籍
      </h1>
      <p class="mb-0 mt-2 text-sm text-muted-foreground">
        把读过的好书，变成值得分享的内容。
      </p>
    </div>
    <RouterLink
      v-if="desktop"
      to="/projects/new"
      :class="buttonVariants()"
    >
      <AppIcon name="plus" />添加一本书
    </RouterLink>
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
  <div class="mb-6 flex flex-wrap items-center justify-between gap-4 border-b pb-5">
    <div
      class="library-filters flex gap-1 rounded-lg bg-muted p-1"
      aria-label="书籍筛选"
    >
      <Button
        v-for="item in filters"
        :key="item.value"
        type="button"
        variant="ghost"
        size="sm"
        :class="filter === item.value ? 'bg-white text-foreground shadow-xs hover:bg-white' : 'text-muted-foreground'"
        :aria-pressed="filter === item.value"
        :disabled="!desktop"
        @click="filter = item.value"
      >
        {{ item.label }}<span class="text-xs tabular-nums text-muted-foreground">{{ item.count }}</span>
      </Button>
    </div>
    <form
      class="flex flex-wrap items-center gap-2"
      role="search"
      @submit.prevent="load"
    >
      <div class="relative w-56">
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
          class="bg-white pl-9 pr-9"
          placeholder="搜索书名、作者或 ISBN"
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
          class="w-28 bg-white"
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
        type="submit"
        variant="outline"
        size="icon"
        class="bg-white"
        :disabled="!desktop || busy"
        aria-label="刷新书架"
        title="刷新书架"
      >
        <AppIcon
          name="refresh"
          :class="{ spin: busy }"
        />
      </Button>
    </form>
  </div>
  <p
    v-if="error"
    class="error"
    role="alert"
  >
    {{ error }} · 点击刷新书架重试，已有内容保留。
  </p>
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
      class="mb-4 text-xs text-muted-foreground"
      role="status"
    >
      {{ search ? '搜索“' + search + '” · ' : '' }}共 {{ filtered.length }} 本书
    </p>
    <section
      v-if="filtered.length"
      class="grid grid-cols-1 gap-5 min-[620px]:grid-cols-2 xl:grid-cols-3 min-[1600px]:grid-cols-4"
      aria-label="项目列表"
    >
      <RouterLink
        v-for="project in filtered"
        :key="project.id"
        class="project-card group block min-w-0 overflow-hidden rounded-xl border bg-card text-card-foreground shadow-xs transition-shadow hover:border-primary/30 hover:shadow-md"
        :to="'/projects/' + project.id + '/writing'"
      >
        <div
          class="book-art relative flex h-48 items-center justify-center border-b"
          :class="'tone-' + coverTone(project.id)"
        >
          <Badge
            variant="outline"
            class="absolute right-3 top-3 border-white/70 bg-white/90 text-xs font-normal"
          >
            {{ projectStatuses[project.status] }}
          </Badge>
          <div
            class="book-jacket"
            aria-hidden="true"
          >
            <AppIcon name="book" /><strong>{{ project.title }}</strong><small>{{ project.author || '我的读书计划' }}</small>
          </div>
        </div>
        <div class="p-5">
          <h2
            class="m-0 truncate text-base font-semibold"
            :title="project.title"
          >
            {{ project.title }}
          </h2>
          <p class="mb-3 mt-1 text-xs text-muted-foreground">
            {{ project.author || '作者待补充' }}
          </p>
          <p class="m-0 line-clamp-2 min-h-10 text-sm leading-5 text-muted-foreground">
            {{ project.description || project.sourceNotes || '记录一个想法，开始这本书的创作。' }}
          </p>
          <div class="mt-5 flex items-center justify-between gap-2 border-t pt-4 text-xs">
            <span class="text-muted-foreground">{{ date(project.updatedAt) }} 更新</span>
            <span class="flex items-center gap-1 font-medium text-primary">继续创作<AppIcon
              name="arrow"
              class="size-3.5"
            /></span>
          </div>
        </div>
      </RouterLink>
      <RouterLink
        v-if="!search && filter === 'all'"
        class="flex min-h-72 flex-col items-center justify-center gap-4 rounded-xl border border-dashed bg-muted/20 p-8 text-center text-muted-foreground transition-colors hover:border-primary/40 hover:bg-accent/50 hover:text-primary"
        to="/projects/new"
      >
        <span class="flex size-12 items-center justify-center rounded-xl border bg-white"><AppIcon name="plus" /></span>
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
      class="flex flex-col items-center rounded-xl border border-dashed bg-white px-6 py-20 text-center"
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

<style scoped>
.book-art { background: #eff2ed; }
.book-art.tone-1 { background: #f4eee7; }
.book-art.tone-2 { background: #eaf0f3; }
.book-art.tone-3 { background: #f2ebeb; }
.book-jacket { display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 10px; width: 112px; height: 138px; padding: 16px 10px; border-radius: 2px 5px 5px 2px; border-left: 5px solid #ffffff20; background: #48614f; color: #fffaf0; box-shadow: 3px 2px 0 #fffcf2, 5px 3px 0 #d4d6cf, 9px 10px 18px #24372b20; text-align: center; }
.tone-1 .book-jacket { background: #916f50; }
.tone-2 .book-jacket { background: #4b667b; }
.tone-3 .book-jacket { background: #855960; }
.book-jacket strong { display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; font-family: 'Songti SC', 'SimSun', serif; font-size: 19px; line-height: 1.6; letter-spacing: 1px; }
.book-jacket small { max-width: 100%; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 10px; color: #ffffffcc; }
.book-jacket .app-icon { width: 14px; height: 14px; }
</style>
