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
        读书创作工作台
      </p>
      <nav aria-label="主导航">
        <RouterLink to="/projects">
          书籍项目
        </RouterLink>
        <RouterLink to="/settings">
          目录与备份
        </RouterLink>
        <RouterLink to="/verification">
          工程验证
        </RouterLink>
      </nav>
      <p class="scope-note">
        本地优先 · 从一本书开始<br>资料与素材，井然有序
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
