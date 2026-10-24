<script setup lang="ts">
import { computed } from 'vue'
import { RouterLink, RouterView, useRoute } from 'vue-router'
import AppIcon from './components/AppIcon.vue'
const route = useRoute()
const inProjects = computed(() => route.path.startsWith('/projects'))
</script>

<template>
  <div class="app-shell">
    <aside class="sidebar">
      <RouterLink
        class="brand"
        to="/projects"
        aria-label="书引 AI 首页"
      >
        <span class="brand-mark"><AppIcon name="book" /></span>
        <span>书引 <small>AI</small><span class="brand-subtitle">让阅读，成为创作</span></span>
      </RouterLink>
      <div class="nav-caption">
        工作空间
      </div>
      <nav
        class="main-navigation"
        aria-label="主导航"
      >
        <RouterLink
          to="/projects"
          :class="{ 'is-active': inProjects }"
        >
          <AppIcon name="shelf" />我的书籍<AppIcon
            class="nav-arrow"
            name="chevron"
          />
        </RouterLink>
        <RouterLink
          to="/settings/models"
          :class="{ 'is-active': route.path.startsWith('/settings') }"
        >
          <AppIcon name="settings" />模型设置
        </RouterLink>
      </nav>
      <div class="sidebar-note">
        <AppIcon name="sparkle" /><p>从读完一本书<br>到分享一个好故事。</p><span>专注创作，其余交给书引。</span>
      </div>
      <div class="sidebar-footer">
        <RouterLink to="/settings">
          <AppIcon name="folder" />数据与备份
        </RouterLink>
        <span><i class="status-dot" />本地优先 · 自在创作</span>
      </div>
    </aside>
    <div class="workspace">
      <div class="workspace-bar">
        <span><AppIcon :name="inProjects ? 'book' : 'settings'" />{{ inProjects ? '我的创作空间' : '工作台设置' }}</span>
        <span class="workspace-hint"><AppIcon name="shield" />资料保存在本机</span>
      </div>
      <main id="main-content">
        <RouterView v-slot="{ Component, route: currentRoute }">
          <div
            :key="currentRoute.path"
            class="page-content"
          >
            <component :is="Component" />
          </div>
        </RouterView>
      </main>
    </div>
  </div>
</template>
