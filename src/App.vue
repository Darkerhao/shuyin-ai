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
        <RouterLink to="/verification">
          工程验证
        </RouterLink>
      </nav>
      <p class="scope-note">
        M0 · 工程初始化<br>当前仅开放技术验证
      </p>
    </aside>
    <main><RouterView /></main>
  </div>
</template>
