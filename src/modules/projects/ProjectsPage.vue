<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { RouterLink } from 'vue-router'
import { isDesktop, listProjects } from '../../services/projects'
import { projectStatuses, type Project } from '../../types/project'
import { useOperation } from '../../composables/useOperation'
const desktop = isDesktop()
const projects = ref<Project[]>([])
const search = ref('')
const status = ref('')
const loaded = ref(false)
const { busy, error, run } = useOperation()
const load = () => run(async () => { projects.value = await listProjects(search.value, status.value); loaded.value = true })
onMounted(() => { if (desktop) void load() })
</script>
<template>
  <header>
    <p class="eyebrow">
      你的读书创作空间
    </p><h1>书籍项目</h1>
    <p class="muted">
      从一本书开始，把资料、创作方向和本地素材整理在一起。
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
    当前为浏览器预览。请在桌面应用中管理本地项目与素材。
  </p>
  <form
    class="panel filter-bar"
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
    <div>
      <label for="status">项目状态</label><select
        id="status"
        v-model="status"
        :disabled="!desktop || busy"
      >
        <option value="">
          全部状态
        </option><option
          v-for="(label, value) in projectStatuses"
          :key="value"
          :value="value"
        >
          {{ label }}
        </option>
      </select>
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
        :to="`/projects/${project.id}`"
      >
        <span class="badge">{{ projectStatuses[project.status] }}</span>
        <h2>{{ project.title }}</h2><p class="muted">
          {{ project.author || '尚未填写作者' }}
        </p>
        <p class="card-description">
          {{ project.description || '添加书籍简介与参考资料，开始整理创作方向。' }}
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
      <h2>{{ search || status ? '没有匹配的项目' : '还没有书籍项目' }}</h2><p class="muted">
        {{ search || status ? '调整关键词或状态后重新查询。' : '点击新建项目，保存第一本书的资料。' }}
      </p>
    </section>
  </template>
</template>
