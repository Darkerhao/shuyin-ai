<script setup lang="ts">
import { onBeforeUnmount, ref, watch } from 'vue'
import { readCoverExport } from '../services/covers'
import { decodeBackground } from '../services/coverRenderer'
import type { CoverVersion } from '../types/cover'
const props = defineProps<{ cover: CoverVersion }>()
const url = ref('')
const error = ref('')
let request = 0
watch(() => props.cover, async cover => {
  const current = ++request
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
    <img
      v-if="url"
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
