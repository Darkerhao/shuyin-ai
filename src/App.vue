<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { isTauri } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { RouterLink, RouterView, START_LOCATION, useRoute, useRouter } from 'vue-router'
import AppIcon from './components/AppIcon.vue'
import AppNotifications from './components/AppNotifications.vue'
import AppConfirmation from './components/AppConfirmation.vue'
import OperationFeedback from './components/OperationFeedback.vue'
import { useAppUpdates } from './composables/useAppUpdates'
import RunningTasks from './components/RunningTasks.vue'
import { useInterfacePreferences } from './composables/useInterfacePreferences'
import { workspaceSettings, openProjectDirectory } from './services/projects'
import { displayPath } from './lib/displayPath'
import { Button } from '@/components/ui/button'

const { initialize, version: appVersion, status: updateStatus, update, error: updateError, settingsError } = useAppUpdates()
const updateMessage = computed(() => updateStatus.value === 'current' ? '当前已是最新版本。' : update.value
  ? `书引 AI v${update.value.version} ${updateStatus.value === 'ready' ? '已下载，请在关于与更新中确认安装。' : updateStatus.value === 'restart' ? '已安装，请重启应用。' : updateStatus.value === 'available' ? '可用，可在关于与更新中查看。' : ''}` : '')
onMounted(() => { void initialize() })

const route = useRoute()
const router = useRouter()
const preferences = useInterfacePreferences()
const workspaceRoot = ref('')
const directoryError = ref('')
const dropHover = ref(false)
let unlistenDrop: (() => void) | undefined
let disposed = false
onMounted(async () => {
  if (!isTauri()) return
  try {
    const unlisten = await getCurrentWindow().onDragDropEvent(event => {
      if (route.path !== '/projects') { dropHover.value = false; return }
      const data = event.payload
      dropHover.value = data.type === 'enter' || data.type === 'over'
      if (data.type !== 'drop') return
      if (data.paths.length !== 1) { directoryError.value = '请每次拖入一个文件，逐本添加书籍。'; return }
      const path = data.paths[0]!
      if (!/\.(txt|md|epub|pdf|docx)$/i.test(path)) { directoryError.value = '支持 TXT、Markdown、EPUB、PDF 或 DOCX 书籍文件。'; return }
      directoryError.value = ''
      void router.push({ path: '/projects/new', query: { source: path } })
    })
    if (disposed) unlisten()
    else unlistenDrop = unlisten
  } catch { directoryError.value = '文件拖入暂不可用，仍可通过添加书籍选择本地文件。' }
})
onBeforeUnmount(() => { disposed = true; unlistenDrop?.() })
watch(() => route.path, () => { dropHover.value = false })
onMounted(async () => {
  await preferences.initialize()
  if (isTauri()) {
    try { workspaceRoot.value = (await workspaceSettings()).root } catch { workspaceRoot.value = '' }
  }
})
async function openDirectory() {
  if (typeof route.params.id !== 'string') return
  try { await openProjectDirectory(route.params.id); directoryError.value = '' }
  catch { directoryError.value = '工作目录暂不可用，请在书籍资料中检查路径。' }
}
function switchStep(event: globalThis.KeyboardEvent) {
  if (!(event.ctrlKey || event.metaKey) || event.altKey || event.shiftKey || event.repeat || event.isComposing || event.keyCode === 229 || !/^[1-5]$/.test(event.key) || typeof route.params.id !== 'string') return
  if (globalThis.document.querySelector('[role="alertdialog"], [role="dialog"], [role="menu"]')) return
  event.preventDefault()
  const suffix = ['', '/writing', '/covers', '/video', '/publishing'][Number(event.key) - 1]
  void router.push(`/projects/${route.params.id}${suffix}`)
}
onMounted(() => globalThis.addEventListener('keydown', switchStep))
onBeforeUnmount(() => globalThis.removeEventListener('keydown', switchStep))
const mainContent = ref<globalThis.HTMLElement | null>(null)
const inProjects = computed(() => route.path.startsWith('/projects'))
const navigation = computed(() => [
  { to: '/projects', label: '我的书籍', icon: 'shelf', active: route.path === '/projects', step: 0, disabled: false },
  { to: `/projects/${route.params.id ?? ''}`, label: '书籍资料', icon: 'book', active: !!route.params.id && route.path === `/projects/${route.params.id}`, step: 1, disabled: !route.params.id },
  { to: `/projects/${route.params.id ?? ''}/writing`, label: '内容创作', icon: 'pen', active: route.path.endsWith('/writing'), step: 2, disabled: !route.params.id },
  { to: '/publishing', label: '发布任务', icon: 'upload', active: route.path === '/publishing', step: 0, disabled: false },
  { to: '/settings/models', label: '模型设置', icon: 'settings', active: route.path === '/settings/models', step: 0, disabled: false },
] as const)
const workspaceTitle = computed(() => {
  if (route.path === '/projects/new') return '添加一本书'
  if (route.params.id) return route.path.endsWith('/writing') ? '内容创作' : route.path.endsWith('/covers') ? '封面工坊' : route.path.endsWith('/video') ? '视频制作' : route.path.endsWith('/publishing') ? '发布准备' : '书籍资料'
  return route.path === '/publishing' ? '发布任务' : inProjects.value ? '我的书籍' : route.path === '/settings/models' ? '模型设置' : route.path === '/about' ? '关于书引 AI' : route.path === '/verification' ? '技术验证' : '数据与备份'
})
watch(router.currentRoute, async (to, from) => {
  await nextTick()
  globalThis.document.title = `${workspaceTitle.value} · 书引 AI`
  if (from && from !== START_LOCATION && to.path !== from.path) {
    mainContent.value?.focus({ preventScroll: true })
  }
}, { immediate: true })
</script>

<template>
  <AppNotifications />
  <AppConfirmation />
  <div
    v-if="dropHover"
    class="book-drop-overlay"
    role="status"
  >
    <div><AppIcon name="book" /><strong>释放文件，快速添加书籍</strong><p>TXT · Markdown · EPUB · PDF · DOCX</p><span>保存前可以核对书名与来源文件。</span></div>
  </div>
  <OperationFeedback :error="preferences.error.value || directoryError" />
  <OperationFeedback
    :error="updateError || settingsError"
    :message="['current', 'available', 'ready', 'restart'].includes(updateStatus) ? updateMessage : ''"
    variant="info"
  />
  <div class="app-shell">
    <a
      class="skip-link"
      href="#main-content"
      @click.prevent="mainContent?.focus()"
    >跳到主要内容</a>
    <aside class="app-sidebar">
      <div class="sidebar-main">
        <RouterLink
          to="/projects"
          aria-label="书引 AI 首页"
          class="brand-link"
        >
          <span class="brand-symbol"><AppIcon name="book" /></span>
          <span class="brand-copy hidden min-w-0 md:block">
            <span class="brand-name">书引 <span>AI</span></span>
            <span class="block whitespace-nowrap text-[11px]">让阅读，成为创作。</span>
          </span>
        </RouterLink>
        <p class="sidebar-caption sidebar-group-label hidden md:block">
          工作空间 · WORKSPACE
        </p>
        <nav
          class="sidebar-navigation"
          aria-label="主导航"
        >
          <template
            v-for="item in navigation"
            :key="item.label"
          >
            <RouterLink
              v-if="!item.disabled"
              :to="item.to"
              :title="item.label"
              :aria-label="item.label"
              :aria-current="item.active ? 'page' : undefined"
              class="sidebar-nav-row"
            >
              <AppIcon :name="item.icon" />
              <span class="hidden md:inline">{{ item.label }}</span>
              <span
                v-if="item.step"
                class="sidebar-step hidden md:inline"
              >Step {{ item.step }}</span>
              <span
                v-if="item.active && !item.step"
                class="sidebar-current-dot hidden md:block"
              />
            </RouterLink>
            <span
              v-else
              class="sidebar-nav-row"
              role="link"
              aria-disabled="true"
              :aria-label="`${item.label}，请先选择一本书`"
            ><AppIcon :name="item.icon" /><span class="hidden md:inline">{{ item.label }}</span><span class="sidebar-step hidden md:inline">Step {{ item.step }}</span></span>
          </template>
        </nav>
      </div>
      <div class="sidebar-footer">
        <details
          v-if="preferences.sidebarPrompt.value"
          class="sidebar-guidance hidden md:block"
        >
          <summary>创作提示</summary>
          <div class="sidebar-prompt relative">
            <Button
              variant="ghost"
              size="icon-sm"
              class="absolute right-1 top-1"
              aria-label="收起创作提示"
              :disabled="!isTauri() || preferences.busy.value"
              @click="preferences.setPrompt(false)"
            >
              <AppIcon name="close" />
            </Button>
            <p class="m-0 text-sm font-medium leading-6">
              让阅读，成为创作。
            </p>
            <p class="mb-0 mt-1 text-xs leading-5 sidebar-caption">
              从一个好故事开始，分享你的独特视角。
            </p>
          </div>
        </details>
        <div
          v-if="workspaceRoot"
          class="sidebar-location hidden min-w-0 md:block"
        >
          <p class="m-0 text-xs text-muted-foreground">
            本地存储目录
          </p>
          <div class="sidebar-directory-row">
            <p
              class="sidebar-directory"
              :title="displayPath(workspaceRoot)"
            >
              {{ displayPath(workspaceRoot) }}
            </p>
            <Button
              v-if="route.params.id"
              variant="ghost"
              size="icon-sm"
              class="size-5 p-0 text-info"
              aria-label="打开本书目录"
              title="打开本书目录"
              @click="openDirectory"
            >
              <AppIcon name="folder" />
            </Button>
          </div>
        </div>
        <div>
          <RouterLink
            to="/settings"
            title="数据与备份"
            aria-label="数据与备份"
            :aria-current="route.path === '/settings' ? 'page' : undefined"
            class="sidebar-backup-link"
          >
            <AppIcon name="folder" /><span class="hidden md:inline">数据与备份</span>
          </RouterLink>
          <div class="sidebar-system hidden md:flex">
            <span><i class="status-dot" />本地保存，安心创作</span><RouterLink
              to="/about"
              title="关于与更新"
              aria-label="关于与更新"
              :aria-current="route.path === '/about' ? 'page' : undefined"
              class="sidebar-version-link"
            >
              {{ appVersion ? `v${appVersion}` : '关于与更新' }}<span
                v-if="update"
                class="status-dot"
              />
            </RouterLink>
          </div>
          <RouterLink
            to="/about"
            aria-label="关于与更新"
            class="sidebar-about-link md:hidden"
          >
            <AppIcon name="info" />
          </RouterLink>
        </div>
      </div>
    </aside>
    <div class="app-workspace min-w-0">
      <div class="app-topbar">
        <div class="flex items-center gap-3 text-sm">
          <RouterLink
            to="/projects"
            class="workspace-home"
          >
            工作台
          </RouterLink><span
            class="text-border"
            aria-hidden="true"
          >/</span><span
            class="font-medium"
            aria-current="page"
          >{{ workspaceTitle }}</span>
        </div>
        <div class="topbar-actions">
          <nav
            class="workspace-shortcuts"
            aria-label="页面快捷入口"
          >
            <span>页面导航：</span><RouterLink to="/projects">
              1. 我的书籍
            </RouterLink>
            <RouterLink
              v-if="route.params.id"
              :to="`/projects/${route.params.id}`"
            >
              2. 书籍资料
            </RouterLink>
            <RouterLink
              v-if="route.params.id"
              :to="`/projects/${route.params.id}/writing`"
            >
              3. 内容创作
            </RouterLink>
            <RouterLink to="/settings">
              4. 数据与诊断
            </RouterLink>
          </nav>
          <span class="local-storage-badge"><AppIcon name="shield" />资料保存在本地</span>
        </div>
      </div>
      <main
        id="main-content"
        ref="mainContent"
        tabindex="-1"
        class="app-main"
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
        <RunningTasks />
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
