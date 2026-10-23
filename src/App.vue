<script setup lang="ts">
import { RouterLink, RouterView } from 'vue-router'
</script>

<template>
  <div class="app-shell">
    <aside class="sidebar">
      <div class="brand">
        书引 <span>AI</span>
      </div>
      <p class="muted">
        读书发布助手
      </p>
      <nav aria-label="主导航">
        <RouterLink to="/projects">
          我的书籍
        </RouterLink>
        <RouterLink to="/settings/models">
          模型设置
        </RouterLink>
      </nav>
      <p class="scope-note">
        本地优先 · 从一本书开始<br>发布文案与音频提示词
      </p>
    </aside>
    <main>
      <RouterView v-slot="{ Component, route }">
        <component
          :is="Component"
          :key="route.path"
        />
      </RouterView>
    </main>
  </div>
</template>
