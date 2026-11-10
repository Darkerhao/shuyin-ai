<script setup lang="ts">
import { onBeforeUnmount, ref, watch } from 'vue'
import { readCoverExport } from '../services/covers'
import { decodeBackground } from '../services/coverRenderer'
import type { CoverVersion } from '../types/cover'
import { Button } from './ui/button'
import AppIcon from './AppIcon.vue'
import { DialogRoot, DialogTrigger, DialogPortal, DialogOverlay, DialogContent, DialogTitle, DialogDescription, DialogClose } from 'reka-ui'
const props = defineProps<{ cover: CoverVersion; expandable?: boolean }>()
const url = ref('')
const error = ref('')
const preview = ref(false)
let request = 0
watch(() => props.cover, async cover => {
  const current = ++request
  preview.value = false
  url.value = ''; error.value = ''
  try {
    const image = await decodeBackground(await readCoverExport(cover.projectId, cover.id))
    if (current === request) url.value = image.src
  } catch { if (current === request) error.value = '封面文件不可用，请重新导出或选择。' }
}, { immediate: true })
onBeforeUnmount(() => { request++ })
</script>
<template>
  <figure class="mt-3 space-y-2 rounded-lg border bg-muted/30 p-3">
    <DialogRoot
      v-if="url && expandable"
      v-model:open="preview"
    >
      <DialogTrigger as-child>
        <button
          type="button"
          class="cover-thumbnail-open"
          :aria-label="`放大查看封面 v${cover.versionNo}`"
        >
          <img
            :src="url"
            :alt="`封面 v${cover.versionNo}：${cover.config.title}`"
            @error="error = '封面图片无法显示，请重新导出。'"
          >
          <span><AppIcon name="eye" />放大查看</span>
        </button>
      </DialogTrigger>
      <DialogPortal>
        <DialogOverlay class="cover-lightbox-overlay" />
        <DialogContent class="cover-lightbox">
          <DialogTitle class="sr-only">
            封面 v{{ cover.versionNo }} 放大预览
          </DialogTitle>
          <div class="cover-lightbox-heading">
            <span>v{{ cover.versionNo }} · {{ cover.config.width }} × {{ cover.config.height }}</span>
            <DialogClose as-child>
              <Button
                type="button"
                variant="ghost"
                size="icon"
                aria-label="关闭封面预览"
              >
                <AppIcon name="close" />
              </Button>
            </DialogClose>
          </div>
          <img
            :src="url"
            :alt="`封面 v${cover.versionNo}：${cover.config.title}`"
          >
          <DialogDescription>仅查看已导出封面，当前编辑保持不变。</DialogDescription>
        </DialogContent>
      </DialogPortal>
    </DialogRoot>
    <img
      v-else-if="url"
      :src="url"
      :alt="`封面 v${cover.versionNo}：${cover.config.title}`"
      class="mx-auto max-h-44 max-w-full rounded object-contain"
      @error="error = '封面图片无法显示，请重新导出。'"
    >
    <p
      v-if="error"
      class="error"
      role="alert"
    >
      {{ error }}
    </p>
    <figcaption class="text-sm">
      v{{ cover.versionNo }} · {{ cover.config.title }}<span class="block text-xs text-muted-foreground">{{ cover.config.width }} × {{ cover.config.height }}</span>
    </figcaption>
  </figure>
</template>

<style scoped>
.cover-thumbnail-open { display: grid; place-items: center; position: relative; width: 100%; height: 184px; padding: 12px; border: 0; border-radius: 8px; background: var(--terminal); cursor: zoom-in; }
.cover-thumbnail-open img { max-height: 160px; max-width: 100%; object-fit: contain; border-radius: 3px; box-shadow: 0 8px 18px #00000026; transition: transform 200ms; }
.cover-thumbnail-open > span { display: flex; align-items: center; gap: 5px; position: absolute; bottom: 8px; padding: 4px 8px; border-radius: 6px; background: var(--popover); color: var(--foreground); font-size: 11px; }
.cover-thumbnail-open .app-icon { width: 13px; height: 13px; }
.cover-thumbnail-open:focus-visible { outline: 2px solid var(--info); outline-offset: 3px; }
@media (hover: hover) and (prefers-reduced-motion: no-preference) { .cover-thumbnail-open:hover img { transform: translateY(-3px) scale(1.03); } }
.cover-lightbox { position: fixed; z-index: 50; left: 50%; top: 50%; transform: translate(-50%, -50%); width: min(880px, calc(100vw - 32px)); max-height: calc(100dvh - 32px); overflow-y: auto; padding: 16px; border: 1px solid var(--input-border); border-radius: 16px; background: var(--card); color: var(--foreground); box-shadow: var(--overlay-shadow); }
.cover-lightbox-overlay { position: fixed; inset: 0; z-index: 50; background: #020617b3; }
.cover-lightbox-heading { display: flex; justify-content: space-between; align-items: center; gap: 12px; margin-bottom: 12px; font-size: 13px; }
.cover-lightbox > img { display: block; max-width: 100%; max-height: calc(100dvh - 180px); margin: auto; object-fit: contain; }
.cover-lightbox > p { margin: 12px 0 0; font-size: 12px; color: var(--muted-foreground); }
</style>
