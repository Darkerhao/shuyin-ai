<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { RouterLink } from 'vue-router'
import { isDesktop, listProjects } from '../../services/projects'
import { type Project } from '../../types/project'
import { useOperation } from '../../composables/useOperation'
const desktop = isDesktop()
const projects = ref<Project[]>([])
const search = ref('')
const loaded = ref(false)
const { busy, error, run } = useOperation()
const load = () => run(async () => { projects.value = await listProjects(search.value, ''); loaded.value = true })
onMounted(() => { if (desktop) void load() })
</script>
<template>
  <header>
    <p class="eyebrow">
      你的读书创作空间
    </p><h1>我的书籍</h1>
    <p class="muted">
      选一本书，准备发布标题、发布内容和音频生成提示词。
    </p>
    <RouterLink
      v-if="desktop"
      class="button header-action"
      to="/projects/new"
    >
      新建项目
    </RouterLink>
  </header>
  <p
    v-if="!desktop"
    class="notice"
    role="status"
  >
    当前为浏览器预览。请在桌面应用中管理书籍资料。
  </p>
  <form
    class="panel book-search"
    @submit.prevent="load"
  >
    <div>
      <label for="search">搜索项目</label><input
        id="search"
        v-model="search"
        placeholder="书名、作者或 ISBN"
        :disabled="!desktop || busy"
      >
    </div>
    <button :disabled="!desktop || busy">
      {{ busy ? '查询中…' : '查询' }}
    </button>
  </form>
  <p
    v-if="error"
    class="error"
    role="alert"
  >
    {{ error }} · 可重新查询
  </p>
  <p
    v-if="busy"
    role="status"
  >
    正在读取项目…
  </p>
  <template v-else-if="loaded">
    <p class="muted">
      共 {{ projects.length }} 个项目
    </p>
    <section
      v-if="projects.length"
      class="project-grid"
      aria-label="项目列表"
    >
      <RouterLink
        v-for="project in projects"
        :key="project.id"
        class="panel project-card"
        :to="`/projects/${project.id}/writing`"
      >
        <h2>{{ project.title }}</h2><p class="muted">
          {{ project.author || '尚未填写作者' }}
        </p>
        <p class="card-description">
          {{ project.description || '生成这一期的发布文案和音频提示词。' }}
        </p>
        <p class="muted">
          更新于 {{ new Date(project.updatedAt).toLocaleString() }}
        </p>
      </RouterLink>
    </section>
    <section
      v-else
      class="panel empty-state"
    >
      <h2>{{ search ? '没有匹配的项目' : '还没有书籍项目' }}</h2><p class="muted">
        {{ search ? '调整关键词后重新查询。' : '点击新建项目，保存第一本书的资料。' }}
      </p>
    </section>
  </template>
</template>
