<script setup lang="ts">
import type { CoverDesign } from '../types/cover'
defineProps<{ design: CoverDesign }>()
</script>

<template>
  <div class="space-y-3">
    <div
      class="rounded-lg p-4"
      :style="{ backgroundColor: design.backgroundColor }"
      aria-label="AI 推荐配色示意"
    >
      <span
        class="mb-2 block h-1 w-8 rounded"
        :style="{ backgroundColor: design.accentColor }"
      />
      <p
        class="!m-0 text-xl font-bold"
        :style="{ color: design.textColor }"
      >
        让好故事被看见
      </p>
      <p
        class="!mb-0 !mt-2 text-sm"
        :style="{ color: design.subtitleColor }"
      >
        书名与看点，清晰分层
      </p>
    </div>
    <div class="grid grid-cols-2 gap-2 text-xs">
      <span>主标题 {{ design.titleSize }} px</span>
      <span>副标题 {{ design.subtitleSize }} px</span>
      <span
        v-for="color in [{ label: '主标题', value: design.textColor }, { label: '副标题', value: design.subtitleColor }, { label: '点缀', value: design.accentColor }, { label: '底色', value: design.backgroundColor }]"
        :key="color.label"
        class="flex min-w-0 items-center gap-1.5"
      >
        <i
          class="h-3 w-3 shrink-0 rounded-sm border"
          :style="{ backgroundColor: color.value }"
        />{{ color.label }} {{ color.value }}
      </span>
    </div>
    <p class="muted !m-0 text-xs">
      建议底色蒙层 {{ Math.round(design.overlayOpacity * 100) }}%，突出文字；长文字仍会按各比例自动缩小。
    </p>
  </div>
</template>
