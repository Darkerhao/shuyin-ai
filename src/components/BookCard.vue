<script setup lang="ts">
import { computed, ref, shallowRef, watch } from 'vue'
import { RouterLink, useRouter } from 'vue-router'
import { DropdownMenuContent, DropdownMenuItem, DropdownMenuLabel, DropdownMenuPortal, DropdownMenuRoot, DropdownMenuSeparator, DropdownMenuTrigger, type DropdownMenuContentProps } from 'reka-ui'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import AppIcon from './AppIcon.vue'
import OperationFeedback from './OperationFeedback.vue'
import { projectStatuses, type Asset, type Project } from '../types/project'
import { resumeLabel, resumeRoute, type ResumeTask } from '../services/workflow'
import { deleteProject, exportVideoAsset, listAssets, openProjectDirectory } from '../services/projects'
import { requestConfirmation } from '../composables/useConfirmation'
import { useOperation } from '../composables/useOperation'
const props = defineProps<{ project: Project; resume?: ResumeTask }>()
const emit = defineEmits<{ deleted: [] }>()
const router = useRouter()
const menuOpen = ref(false)
const menuReference = shallowRef<DropdownMenuContentProps['reference']>()
const card = ref<globalThis.HTMLElement>()
const videos = ref<Asset[]>([])
const loaded = ref(false)
const { busy, error, message, run } = useOperation()
const to = computed(() => props.resume ? resumeRoute(props.resume) : `/projects/${props.project.id}/writing`)
const action = computed(() => props.resume ? resumeLabel(props.resume) : '继续创作')
const bookTones = [
  ['#48614f', '#eff2ed'], // 松柏绿
  ['#876648', '#f4eee7'], // 栗棕
  ['#4b667b', '#eaf0f3'], // 雾蓝
  ['#793f50', '#f3e9ed'], // 酒红
  ['#646b43', '#f1f2e8'], // 橄榄绿
  ['#555b7c', '#ededf4'], // 靛蓝
  ['#8b6338', '#f6efe3'], // 赭石
  ['#3e6a67', '#e9f1ef'], // 深青绿
  ['#985b49', '#f6ece6'], // 陶土红
  ['#354e6b', '#e8edf3'], // 藏青
  ['#706078', '#f0ebf2'], // 灰紫
  ['#51575b', '#eceff0'], // 石墨灰
] as const
const tone = computed(() => {
  // FNV-1a: keep each book's colors stable without depending on its title or position.
  const hash = [...props.project.id].reduce((value, char) => Math.imul(value ^ char.charCodeAt(0), 0x01000193) >>> 0, 0x811c9dc5)
  const [cover, backdrop] = bookTones[hash % bookTones.length]!
  return { '--book-cover': cover, '--book-backdrop': backdrop }
})
let returnFocus: globalThis.HTMLElement | null = null
function prepareMore(event: globalThis.Event) {
  returnFocus = event.currentTarget as globalThis.HTMLElement
  menuReference.value = undefined
}
function openContextMenu(event: globalThis.MouseEvent) {
  const { clientX, clientY } = event
  returnFocus = (event.target as globalThis.Element).closest<globalThis.HTMLElement>('a, button') ?? card.value?.querySelector<globalThis.HTMLElement>('a') ?? null
  menuReference.value = { getBoundingClientRect: () => new globalThis.DOMRect(clientX, clientY, 0, 0) }
  menuOpen.value = true
}
function menuShortcut(event: globalThis.KeyboardEvent) {
  if ((event.key === 'F10' && event.shiftKey) || event.key === 'ContextMenu') {
    event.preventDefault(); returnFocus = event.target as globalThis.HTMLElement; menuReference.value = undefined; menuOpen.value = true
  }
}
function restoreFocus(event: globalThis.Event) {
  event.preventDefault()
  const active = globalThis.document.activeElement as globalThis.HTMLElement | null
  if (active && active !== globalThis.document.body && !active.closest('[role="menu"]') && active !== returnFocus && !card.value?.contains(active)) { returnFocus = null; return }
  const target = returnFocus?.isConnected ? returnFocus : card.value?.querySelector<globalThis.HTMLElement>('a')
  target?.focus(); returnFocus = null
}
watch(menuOpen, async open => {
  if (!open) return
  error.value = ''
  loaded.value = false
  try {
    videos.value = (await listAssets(props.project.id)).filter(asset => asset.assetType === 'video' && asset.existsStatus === 'available' && /\.(mp4|mov|m4v)$/i.test(asset.filePath))
    loaded.value = true
  } catch { videos.value = []; error.value = '成片列表读取失败，请重新打开菜单重试。' }
})
const exportVideo = (asset: Asset) => run(async () => { if (await exportVideoAsset(asset)) message.value = '成片已另存，原文件保留。' })
const remove = async () => {
  if (!await requestConfirmation(`删除《${props.project.title}》及其关联记录？原始文件和工作目录将保留。`, `project-more-${props.project.id}`)) return
  await run(async () => { await deleteProject(props.project.id); emit('deleted') })
}
</script>

<template>
  <OperationFeedback
    :error="error"
    :message="message"
  />
  <DropdownMenuRoot
    v-model:open="menuOpen"
    :modal="false"
  >
    <article
      ref="card"
      class="project-card group relative overflow-hidden rounded-xl bg-card"
      @contextmenu.prevent="openContextMenu"
      @keydown="menuShortcut"
    >
      <RouterLink
        :to="to"
        class="project-card-link block min-w-0 text-card-foreground no-underline"
      >
        <div
          class="book-art relative flex h-48 items-center justify-center"
          :style="tone"
        >
          <Badge
            variant="secondary"
            class="absolute left-3 top-3 text-xs font-normal"
          >
            {{ resume?.running ? action : projectStatuses[project.status] }}
          </Badge>
          <div
            class="book-jacket"
            aria-hidden="true"
          >
            <strong>{{ project.title }}</strong><small v-if="project.author">{{ project.author }}</small>
          </div>
        </div>
        <div class="project-card-copy p-5">
          <h2
            class="project-card-title"
            :title="project.title"
          >
            {{ project.title }}
          </h2>
          <p class="mb-3 mt-1 text-xs text-muted-foreground">
            {{ project.author || '作者待补充' }}
          </p>
          <div class="project-card-footer mt-4 flex items-center justify-between gap-2 text-xs">
            <span class="text-muted-foreground">{{ new Date(project.updatedAt).toLocaleDateString('zh-CN', { month: 'long', day: 'numeric' }) }} 更新</span>
            <span class="project-continue flex items-center gap-1 font-medium text-primary">{{ action }}<AppIcon
              name="arrow"
              class="size-3.5"
            /></span>
          </div>
        </div>
      </RouterLink>
      <DropdownMenuTrigger as-child>
        <Button
          :id="`project-more-${project.id}`"
          variant="ghost"
          size="icon-sm"
          class="book-more absolute right-2 top-2 bg-white/90"
          :aria-label="`更多操作：${project.title}`"
          @click="prepareMore"
          @keydown="prepareMore"
        >
          <AppIcon name="more" />
        </Button>
      </DropdownMenuTrigger>
    </article>
    <DropdownMenuPortal>
      <DropdownMenuContent
        as-child
        :aria-labelledby="`project-more-${project.id}`"
        :reference="menuReference"
        :side="menuReference ? 'right' : 'bottom'"
        :align="menuReference ? 'start' : 'end'"
        :side-offset="menuReference ? 2 : 6"
        :collision-padding="12"
        @close-auto-focus="restoreFocus"
      >
        <div class="book-context-menu">
          <DropdownMenuLabel class="book-menu-label">
            {{ project.title }}
          </DropdownMenuLabel>
          <DropdownMenuItem
            class="book-menu-item"
            :disabled="busy"
            @select="router.push({ path: `/projects/${project.id}`, query: { focus: 'title' } })"
          >
            <AppIcon name="pen" />重命名
          </DropdownMenuItem>
          <DropdownMenuItem
            class="book-menu-item"
            :disabled="busy"
            @select="run(() => openProjectDirectory(project.id))"
          >
            <AppIcon name="folder" />在文件管理器中显示目录
          </DropdownMenuItem>
          <DropdownMenuSeparator class="book-menu-separator" />
          <DropdownMenuItem
            v-if="!videos.length"
            class="book-menu-item"
            disabled
          >
            {{ loaded ? '暂无可导出的成片' : error ? '成片列表读取失败' : '正在读取成片…' }}
          </DropdownMenuItem>
          <DropdownMenuItem
            v-for="asset in videos"
            :key="asset.id"
            class="book-menu-item"
            :disabled="busy"
            @select="exportVideo(asset)"
          >
            <AppIcon name="download" />{{ videos.length === 1 ? '导出成片' : `导出：${asset.originalName}` }}
          </DropdownMenuItem>
          <DropdownMenuSeparator class="book-menu-separator" />
          <DropdownMenuItem
            class="book-menu-item text-destructive"
            :disabled="busy || resume?.running"
            @select="remove"
          >
            <AppIcon name="trash" />删除书籍{{ resume?.running ? '（请先停止任务）' : '' }}
          </DropdownMenuItem>
        </div>
      </DropdownMenuContent>
    </DropdownMenuPortal>
  </DropdownMenuRoot>
</template>

<style scoped>
.book-art { background: var(--book-backdrop); }
.book-jacket { position: relative; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 10px; width: 112px; height: 142px; padding: 16px 14px; border-radius: 2px 5px 5px 2px; background: var(--book-cover); color: #fffaf0; box-shadow: 2px 1px 0 #f9f6ec, 0 8px 24px -4px #0000001f, 0 2px 6px -1px #00000014; text-align: center; }
.book-jacket::before { content: ''; position: absolute; inset: 0 auto 0 0; width: 13px; border-radius: inherit; background: linear-gradient(90deg, #00000024, #ffffff17 24%, #00000012 44%, transparent); pointer-events: none; }
.book-jacket::after { content: ''; position: absolute; inset: 2px -2px 2px auto; width: 2px; background: linear-gradient(#f9f6ec, #d9d6cf); border-radius: 0 2px 2px 0; pointer-events: none; }
.book-jacket strong { display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; font-family: var(--font-serif); font-size: 19px; line-height: 1.6; letter-spacing: 1px; }
.book-jacket small { max-width: 100%; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 11px; color: inherit; }
.book-context-menu { z-index: 50; width: 256px; max-width: calc(100vw - 24px); max-height: var(--reka-dropdown-menu-content-available-height); overflow-y: auto; padding: 6px; border: 1px solid var(--border); border-radius: 10px; background: var(--card); color: var(--foreground); box-shadow: 0 12px 32px -8px #202d2830, 0 2px 6px #202d2810; }
.book-more[data-state="open"] { background: var(--accent); color: var(--primary); }
.book-menu-label { padding: 8px 10px; font-size: 12px; line-height: 1.5; color: var(--muted-foreground); overflow-wrap: anywhere; }
.book-menu-item { display: flex; align-items: center; gap: 9px; min-height: 38px; padding: 8px 10px; border-radius: 6px; font-size: 13px; line-height: 1.5; cursor: pointer; outline: none; overflow-wrap: anywhere; }
.book-menu-item .app-icon { width: 16px; height: 16px; }
.book-menu-item[data-highlighted] { background: var(--accent); }
.book-menu-item.text-destructive[data-highlighted] { background: color-mix(in srgb, var(--color-destructive) 8%, var(--card)); }
.book-menu-item[data-disabled] { color: var(--muted-foreground); cursor: default; }
.book-menu-separator { height: 1px; margin: 5px 4px; background: var(--border); }
</style>
