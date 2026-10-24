<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { RouterLink } from 'vue-router'
import { isDesktop, listProjects } from '../../services/projects'
import { projectStatuses, type Project } from '../../types/project'
import { useOperation } from '../../composables/useOperation'
import AppIcon from '../../components/AppIcon.vue'
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
  <header class="library-hero">
    <div>
      <span class="eyebrow"><AppIcon name="sparkle" /> 灵感，从翻开一本书开始</span>
      <h1>让每一本好书，都被听见。</h1>
      <p>从书籍到文案、音频提示词与封面，陪你完成每一次分享。</p>
      <RouterLink
        v-if="desktop"
        class="button"
        to="/projects/new"
      >
        <AppIcon name="plus" />添加一本书
      </RouterLink>
    </div>
    <div
      class="hero-art"
      aria-hidden="true"
    >
      <div class="hero-book back" /><div class="hero-book front">
        <span>书引</span><small>A STORY TO SHARE</small>
      </div>
    </div>
  </header>
  <p
    v-if="!desktop"
    class="notice"
    role="status"
  >
    当前为浏览器预览。请在桌面应用中管理书籍资料。
  </p>
  <div class="section-heading">
    <h2>我的书籍</h2><span class="muted">你的私人创作书架</span>
  </div>
  <div class="library-toolbar">
    <div
      class="library-filters"
      aria-label="书籍筛选"
    >
      <button
        v-for="item in filters"
        :key="item.value"
        type="button"
        :aria-pressed="filter === item.value"
        :disabled="!desktop"
        @click="filter = item.value"
      >
        {{ item.label }}<span class="filter-count">{{ item.count }}</span>
      </button>
    </div>
    <form
      class="book-search"
      role="search"
      @submit.prevent="load"
    >
      <div class="search-input">
        <label
          class="sr-only"
          for="search"
        >搜索书名、作者或 ISBN</label><AppIcon name="search" />
        <input
          id="search"
          v-model="search"
          placeholder="搜索书名、作者或 ISBN"
          :disabled="!desktop"
          @keydown.esc="search = ''"
        >
        <button
          v-if="search"
          type="button"
          class="icon-button ghost"
          aria-label="清空搜索"
          :disabled="!desktop"
          @click="search = ''"
        >
          <AppIcon name="close" />
        </button>
      </div>
      <label
        class="sr-only"
        for="book-sort"
      >书籍排序</label>
      <select
        id="book-sort"
        v-model="sort"
        class="sort-select"
        :disabled="!desktop"
      >
        <option value="updated">
          最近更新
        </option><option value="title">
          书名排序
        </option>
      </select>
      <button
        type="submit"
        class="icon-button secondary"
        :disabled="!desktop || busy"
        aria-label="刷新书架"
        title="刷新书架"
      >
        <AppIcon
          name="refresh"
          :class="{ spin: busy }"
        />
      </button>
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
      class="library-count"
      role="status"
    >
      {{ search ? `搜索“${search}” · ` : '' }}共 {{ filtered.length }} 本书
    </p>
    <section
      v-if="filtered.length"
      class="project-grid"
      aria-label="项目列表"
    >
      <RouterLink
        v-for="project in filtered"
        :key="project.id"
        class="project-card"
        :to="`/projects/${project.id}/writing`"
      >
        <div
          class="book-art"
          :class="`tone-${coverTone(project.id)}`"
          aria-hidden="true"
        >
          <span class="badge">{{ projectStatuses[project.status] }}</span>
          <div class="book-jacket">
            <AppIcon name="book" /><strong>{{ project.title }}</strong><small>{{ project.author || '我的读书计划' }}</small>
          </div>
        </div>
        <div class="book-card-body">
          <h2 :title="project.title">
            {{ project.title }}
          </h2><p class="book-author">
            {{ project.author || '作者待补充' }}
          </p>
          <p class="card-description">
            {{ project.description || project.sourceNotes || '关于这本书，你想分享什么？从一个想法开始创作。' }}
          </p>
          <div class="book-card-footer">
            <span>更新于 {{ date(project.updatedAt) }}</span><span>继续创作<AppIcon name="arrow" /></span>
          </div>
        </div>
      </RouterLink>
      <RouterLink
        v-if="!search && filter === 'all'"
        class="new-book-card"
        to="/projects/new"
      >
        <AppIcon name="plus" /><strong>开启下一本书</strong><small>新的故事，正在等你分享</small>
      </RouterLink>
    </section>
    <section
      v-else
      class="panel empty-state"
    >
      <AppIcon :name="search || filter !== 'all' ? 'search' : 'book'" />
      <h2>{{ search || filter !== 'all' ? '没有匹配的项目' : '你的书架，等待第一本好书' }}</h2>
      <p class="muted">
        {{ search || filter !== 'all' ? '换个关键词，或查看全部书籍。' : '只需一个书名，就可以开始准备下一期内容。' }}
      </p>
      <button
        v-if="search || filter !== 'all'"
        class="secondary"
        @click="clearFilters"
      >
        查看全部书籍
      </button>
      <RouterLink
        v-else
        class="button"
        to="/projects/new"
      >
        <AppIcon name="plus" />添加第一本书
      </RouterLink>
    </section>
    <p class="library-footnote">
      一本书，一个视角。把你的阅读，变成值得分享的故事。
    </p>
  </template>
</template>
