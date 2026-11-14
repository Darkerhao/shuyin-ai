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
function tiltBook(event: globalThis.PointerEvent) {
  if (event.pointerType !== 'mouse' || !globalThis.matchMedia('(hover: hover) and (prefers-reduced-motion: no-preference)').matches) return
  const surface = event.currentTarget as globalThis.HTMLElement
  const bounds = surface.getBoundingClientRect()
  const horizontal = Math.max(0, Math.min(1, (event.clientX - bounds.left) / bounds.width))
  const vertical = Math.max(0, Math.min(1, (event.clientY - bounds.top) / bounds.height))
  surface.style.setProperty('--book-rotate-x', `${(0.5 - vertical) * 6}deg`)
  surface.style.setProperty('--book-rotate-y', `${(horizontal - 0.5) * 10}deg`)
  surface.style.setProperty('--book-light-x', `${horizontal * 100}%`)
  surface.style.setProperty('--book-light-y', `${vertical * 100}%`)
}
function resetBook(event: globalThis.PointerEvent) {
  const surface = event.currentTarget as globalThis.HTMLElement
  for (const property of ['--book-rotate-x', '--book-rotate-y', '--book-light-x', '--book-light-y']) surface.style.removeProperty(property)
}
const { busy, error, message, run } = useOperation()
const to = computed(() => props.resume ? resumeRoute(props.resume) : `/projects/${props.project.id}/writing`)
const action = computed(() => props.resume ? resumeLabel(props.resume) : '继续创作')
const tone = computed(() => {
  if (props.project.status !== 'published') return 'neutral'
  const hash = [...props.project.id].reduce((value, char) => Math.imul(value ^ char.charCodeAt(0), 0x01000193) >>> 0, 0x811c9dc5)
  return hash % 3 === 0 ? 'warm' : 'cyan'
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
      class="project-card group relative overflow-hidden"
      :data-tone="tone"
      @contextmenu.prevent="openContextMenu"
      @keydown="menuShortcut"
      @pointermove="tiltBook"
      @pointerleave="resetBook"
      @pointercancel="resetBook"
    >
      <RouterLink
        :to="to"
        class="project-card-link min-w-0 text-card-foreground no-underline"
      >
        <div class="project-card-header">
          <Badge
            variant="outline"
            class="book-status"
            :data-tone="project.status === 'published' ? tone === 'warm' ? 'success' : 'ready' : 'attention'"
          >
            {{ resume?.running ? action : projectStatuses[project.status] }}
          </Badge>
        </div>
        <div
          class="book-jacket"
        >
          <h2
            class="project-card-title"
            :title="project.title"
          >
            {{ project.title }}
          </h2>
          <p class="book-author">
            {{ project.author || '作者待补充' }}
          </p>
        </div>
        <div class="project-card-footer">
          <span>{{ new Date(project.updatedAt).toLocaleDateString('zh-CN', { month: 'long', day: 'numeric' }) }} 更新</span>
          <span class="project-continue">{{ action }}<AppIcon
            name="arrow"
            class="size-3.5"
          /></span>
        </div>
      </RouterLink>
      <DropdownMenuTrigger as-child>
        <Button
          :id="`project-more-${project.id}`"
          variant="ghost"
          size="icon-sm"
          class="book-more"
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
.project-card-link { padding: 20px; display: var(--book-link-display, block); }
.project-card-header { display: flex; align-items: center; justify-content: space-between; min-height: 22px; padding-right: 28px; margin-bottom: 16px; }
.book-status { padding: 2px 10px; border-radius: 999px; font: 400 10px/15px var(--font-mono); color: var(--warning); border-color: color-mix(in srgb, var(--warning) 30%, transparent); background: color-mix(in srgb, var(--warning) 15%, transparent); }
.book-status[data-tone='ready'] { color: var(--info); border-color: color-mix(in srgb, var(--info) 30%, transparent); background: color-mix(in srgb, var(--info) 15%, transparent); }
.book-status[data-tone='success'] { color: var(--primary); border-color: color-mix(in srgb, var(--primary) 30%, transparent); background: color-mix(in srgb, var(--primary) 15%, transparent); }
.book-jacket { position: relative; isolation: isolate; display: flex; flex-direction: column; align-items: center; justify-content: center; height: var(--book-preview-height, 192px); padding: 16px 22px; border: 1px solid #33415599; border-left: 5px solid #334155; border-radius: 5px 12px 12px 5px; background: linear-gradient(to bottom, #1e293b, #020617); box-shadow: 4px 4px 0 -1px #cbd5e133, 8px 8px 0 -3px #64748b33, 0 12px 22px -12px #00000080; text-align: center; transform: perspective(700px) rotateY(-6deg) rotateX(2deg); transition: transform 240ms cubic-bezier(.2,.8,.2,1), box-shadow 240ms; }
.book-jacket::after { content: ''; position: absolute; inset: 0; z-index: -1; border-radius: inherit; pointer-events: none; background: radial-gradient(ellipse at var(--book-light-x, 20%) var(--book-light-y, 0%), #ffffff1f, transparent 65%); opacity: .35; transition: opacity 240ms; }
.book-jacket::before { content: ''; position: absolute; inset: 0 0 auto; height: 4px; background: linear-gradient(to right, transparent, var(--info), transparent); }
.project-card[data-tone='neutral'] .book-jacket { background: linear-gradient(to bottom, #0f172a, #020617); }
.project-card[data-tone='warm'] .book-jacket { background: linear-gradient(to bottom, #451a0366, #0f172a, #020617); border-color: #78350f66; }
.project-card[data-tone='neutral'] .book-jacket::before, .project-card[data-tone='warm'] .book-jacket::before { display: none; }
.project-card[data-tone='neutral'] .book-author { color: #fcd34d; }
.project-card[data-tone='warm'] .project-card-title { color: #fde68a; }
.project-card[data-tone='warm'] .book-author { color: #fbbf24cc; }
.project-card-title { display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; margin: 0; color: #fff; font-size: 20px; font-weight: 900; line-height: 28px; letter-spacing: .5px; }
.book-author { margin: 8px 0 0; font: 400 12px/16px var(--font-mono); color: #00e5ff; }
.project-card-footer { display: flex; align-items: center; justify-content: space-between; gap: 8px; margin-top: 20px; padding-top: 12px; border-top: 1px solid var(--border); color: var(--muted-foreground); font: 400 12px/16px var(--font-mono); }
.book-more { position: absolute; top: 19px; right: 16px; width: 24px; height: 24px; padding: 4px; border-radius: 8px; color: var(--muted-foreground); background: transparent; }
.project-card:focus-within .book-jacket { box-shadow: 4px 4px 0 -1px #cbd5e14d, 0 16px 28px -12px color-mix(in srgb, var(--info) 35%, transparent); }
@media (hover: hover) and (pointer: fine) { .project-card:hover .book-jacket { transform: perspective(700px) rotateY(var(--book-rotate-y, -2deg)) rotateX(var(--book-rotate-x, 0deg)) translateY(-4px); box-shadow: 4px 4px 0 -1px #cbd5e14d, 0 18px 28px -12px color-mix(in srgb, var(--info) 35%, transparent); } .project-card:hover .book-jacket::after { opacity: 1; } }
@media (prefers-reduced-motion: reduce) { .book-jacket, .project-card:hover .book-jacket { transform: none; } .book-jacket::after { background: linear-gradient(135deg, #ffffff12, transparent); } }
.book-context-menu { z-index: 50; width: 256px; max-width: calc(100vw - 24px); max-height: var(--reka-dropdown-menu-content-available-height); overflow-y: auto; padding: 6px; border: 1px solid var(--input-border); border-radius: 10px; background: var(--card); color: var(--foreground); box-shadow: var(--overlay-shadow); }
.book-more[data-state="open"] { background: var(--accent); color: var(--primary); }
.book-menu-label { padding: 8px 10px; font-size: 12px; line-height: 1.5; color: var(--muted-foreground); overflow-wrap: anywhere; }
.book-menu-item { display: flex; align-items: center; gap: 9px; min-height: 38px; padding: 8px 10px; border-radius: 6px; font-size: 13px; line-height: 1.5; cursor: pointer; outline: none; overflow-wrap: anywhere; }
.book-menu-item .app-icon { width: 16px; height: 16px; }
.book-menu-item[data-highlighted] { background: var(--accent); }
.book-menu-item.text-destructive[data-highlighted] { background: color-mix(in srgb, var(--color-destructive) 8%, var(--card)); }
.book-menu-item[data-disabled] { color: var(--muted-foreground); cursor: default; }
.book-menu-separator { height: 1px; margin: 5px 4px; background: var(--border); }
</style>
