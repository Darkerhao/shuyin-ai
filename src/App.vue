<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { RouterLink, RouterView, useRoute } from 'vue-router'
import AppIcon from './components/AppIcon.vue'
import AppNotifications from './components/AppNotifications.vue'
import OperationFeedback from './components/OperationFeedback.vue'
import appLogo from '../assets/app-icon.svg'
import { buttonVariants } from '@/components/ui/button'
import { cn } from '@/lib/utils'
import { useAppUpdates } from './composables/useAppUpdates'

const { initialize, status: updateStatus, update, error: updateError, settingsError } = useAppUpdates()
const updateMessage = computed(() => updateStatus.value === 'current' ? '当前已是最新版本。' : update.value
  ? `书引 AI v${update.value.version} ${updateStatus.value === 'ready' ? '已下载，请在关于与更新中确认安装。' : updateStatus.value === 'restart' ? '已安装，请重启应用。' : updateStatus.value === 'available' ? '可用，可在关于与更新中查看。' : ''}` : '')
onMounted(() => { void initialize() })

const route = useRoute()
const mainContent = ref<globalThis.HTMLElement | null>(null)
const inProjects = computed(() => route.path.startsWith('/projects') || route.path === '/publishing')
const navigation = computed(() => [
  { to: '/projects', label: '我的书籍', icon: 'shelf', active: inProjects.value },
  { to: '/settings/models', label: '模型设置', icon: 'settings', active: route.path === '/settings/models' },
] as const)
const workspaceTitle = computed(() => route.path === '/publishing' ? '发布任务' : inProjects.value ? '我的书籍' : route.path === '/settings/models' ? '模型设置' : route.path === '/about' ? '关于书引 AI' : '数据与备份')
</script>

<template>
  <AppNotifications />
  <OperationFeedback
    :error="updateError || settingsError"
    :message="['current', 'available', 'ready', 'restart'].includes(updateStatus) ? updateMessage : ''"
    variant="info"
  />
  <div class="app-shell grid min-h-screen grid-cols-[64px_minmax(0,1fr)] md:grid-cols-[208px_minmax(0,1fr)] xl:grid-cols-[224px_minmax(0,1fr)]">
    <a
      class="skip-link"
      href="#main-content"
      @click.prevent="mainContent?.focus()"
    >跳到主要内容</a>
    <aside class="app-sidebar sticky top-0 flex h-screen flex-col border-r border-sidebar-border bg-sidebar px-2 py-6 md:px-4">
      <RouterLink
        to="/projects"
        aria-label="书引 AI 首页"
        class="mx-auto flex items-center gap-3 rounded-lg md:mx-2"
      >
        <img
          :src="appLogo"
          alt=""
          width="40"
          height="40"
          class="size-10 shrink-0"
        >
        <span class="brand-copy hidden min-w-0 md:block">
          <span class="brand-name flex items-center gap-2 text-[22px] font-semibold leading-tight tracking-[0.06em]">书引 <span class="rounded px-1.5 py-0.5 text-[10px] font-bold leading-none tracking-normal">AI</span></span>
          <span class="mt-1 block whitespace-nowrap text-[11px]">让阅读，成为创作。</span>
        </span>
      </RouterLink>
      <p class="sidebar-caption mb-2 mt-10 hidden px-3 text-xs font-medium md:block">
        工作空间
      </p>
      <nav
        class="mt-8 grid gap-1 md:mt-0"
        aria-label="主导航"
      >
        <RouterLink
          v-for="item in navigation"
          :key="item.to"
          :to="item.to"
          :title="item.label"
          :aria-label="item.label"
          :aria-current="item.active ? 'page' : undefined"
          :class="cn(buttonVariants({ variant: 'ghost' }), 'h-11 justify-center px-3 md:justify-start')"
        >
          <AppIcon :name="item.icon" />
          <span class="hidden md:inline">{{ item.label }}</span>
          <span
            v-if="item.active"
            class="ml-auto hidden size-1.5 rounded-full bg-primary md:block"
          />
        </RouterLink>
      </nav>
      <div class="sidebar-footer mt-auto space-y-4 pt-8">
        <div class="sidebar-prompt mx-2 hidden rounded-xl border border-sidebar-border p-4 md:block">
          <AppIcon
            name="sparkle"
            class="mb-3 text-[#D5F267]"
          />
          <p class="m-0 text-sm font-medium leading-6">
            让阅读，成为创作。
          </p>
          <p class="mb-0 mt-1 text-xs leading-5 sidebar-caption">
            从一个好故事开始，分享你的独特视角。
          </p>
        </div>
        <div class="border-t border-sidebar-border pt-3">
          <RouterLink
            to="/settings"
            title="数据与备份"
            aria-label="数据与备份"
            :aria-current="route.path === '/settings' ? 'page' : undefined"
            :class="cn(buttonVariants({ variant: 'ghost' }), 'h-11 w-full justify-center px-3 md:justify-start')"
          >
            <AppIcon name="folder" /><span class="hidden md:inline">数据与备份</span>
          </RouterLink>
          <RouterLink
            to="/about"
            title="关于与更新"
            aria-label="关于与更新"
            :aria-current="route.path === '/about' ? 'page' : undefined"
            :class="cn(buttonVariants({ variant: 'ghost' }), 'h-11 w-full justify-center px-3 md:justify-start')"
          >
            <AppIcon name="info" /><span class="hidden md:inline">关于与更新</span>
            <span
              v-if="update"
              class="size-1.5 rounded-full bg-primary md:ml-auto"
            />
          </RouterLink>
          <p class="sidebar-caption mb-0 mt-4 hidden items-center gap-2 px-3 text-xs md:flex">
            <span class="size-1.5 rounded-full bg-primary" />本地保存，安心创作
          </p>
        </div>
      </div>
    </aside>
    <div class="app-workspace min-w-0">
      <div class="app-topbar flex h-16 items-center justify-between gap-4 border-b bg-white/80 px-5 md:px-8">
        <div class="flex items-center gap-3 text-sm">
          <span class="text-muted-foreground">工作台</span><span class="text-border">/</span><span class="font-medium">{{ workspaceTitle }}</span>
        </div>
        <span class="hidden items-center gap-1.5 text-xs text-muted-foreground sm:flex"><AppIcon name="shield" />资料保存在本机</span>
      </div>
      <main
        id="main-content"
        ref="mainContent"
        tabindex="-1"
        class="app-main mx-auto w-full max-w-[1440px] px-5 py-7 md:px-8 md:py-9 xl:px-10"
      >
        <RouterLink
          v-if="update && route.path !== '/about'"
          to="/about"
          class="mb-6 flex flex-wrap items-center justify-between gap-2 rounded-xl border bg-accent px-4 py-3 text-sm text-primary"
          aria-live="polite"
        >
          <span>书引 AI v{{ update.version }} {{ updateStatus === 'ready' ? '已下载，可以安装' : updateStatus === 'downloading' ? '正在后台下载' : updateStatus === 'restart' ? '已安装，请重启' : '可用' }}</span>
          <span>查看更新内容 →</span>
        </RouterLink>
        <RouterView v-slot="{ Component, route: currentRoute }">
          <div
            :key="currentRoute.path"
            class="min-w-0"
          >
            <component :is="Component" />
          </div>
        </RouterView>
      </main>
    </div>
  </div>
</template>
