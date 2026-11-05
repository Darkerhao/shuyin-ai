<script setup lang="ts">
import type { CoverDesign } from '../types/cover'
defineProps<{ design: CoverDesign; compact?: boolean }>()
</script>

<template>
  <div
    class="space-y-3"
    :class="{ 'design-compact': compact }"
  >
    <div
      class="rounded-lg border border-input p-6"
      :style="{ backgroundColor: design.backgroundColor }"
      aria-label="AI 推荐配色示意"
    >
      <span
        class="mb-3 block h-1.5 w-12 rounded-full"
        :style="{ backgroundColor: design.accentColor }"
      />
      <p
        class="!m-0 text-3xl font-extrabold tracking-wide"
        :style="{ color: design.textColor }"
      >
        让好故事被看见
      </p>
      <p
        class="!mb-0 text-sm font-medium"
        :class="compact ? '!mt-0' : '!mt-1'"
        :style="{ color: design.subtitleColor }"
      >
        书名与看点，清晰分层
      </p>
    </div>
    <div class="grid grid-cols-2 gap-3 text-xs font-mono sm:grid-cols-4">
      <span
        v-for="color in [{ label: '主标题', value: design.textColor }, { label: '副标题', value: design.subtitleColor }, { label: '点缀', value: design.accentColor }, { label: '底色', value: design.backgroundColor }]"
        :key="color.label"
        class="flex min-w-0 items-center gap-2.5 rounded-lg border bg-input-surface p-2.5"
      >
        <i
          class="h-5 w-5 shrink-0 rounded-md border border-white/20"
          :style="{ backgroundColor: color.value }"
        /><span class="min-w-0"><span class="block text-[10px] text-muted-foreground">{{ color.label }}{{ color.label === '主标题' ? ` ${design.titleSize}px` : color.label === '副标题' ? ` ${design.subtitleSize}px` : '' }}</span><span class="block truncate">{{ color.value }}</span></span>
      </span>
    </div>
    <slot name="hint">
      <p class="muted !m-0 text-xs">
        建议底色蒙层 {{ Math.round(design.overlayOpacity * 100) }}%，突出文字；长文字仍会按各比例自动缩小。
      </p>
    </slot>
  </div>
</template>

<style scoped>
.design-compact > [aria-label='AI 推荐配色示意'] { padding: 12px; }
.design-compact > [aria-label='AI 推荐配色示意'] > span { width: 32px; height: 4px; margin-bottom: 6px; }
.design-compact > [aria-label='AI 推荐配色示意'] > p:first-of-type { font-size: 14px; line-height: 20px; }
.design-compact > [aria-label='AI 推荐配色示意'] > p:last-of-type { font-size: 10px; line-height: 15px; }
.design-compact > .grid { grid-template-columns: repeat(2,minmax(0,1fr)); gap: 8px; margin-bottom: 0; font-size: 10px; line-height: 15px; }
.design-compact > .grid > span { display: block; padding: 0; border: 0; border-radius: 0; background: transparent; }
.design-compact > .grid > span > i { display: none; }
.design-compact > .grid > span > span > span { display: inline; font-size: 10px; line-height: 15px; }
.design-compact > .grid > span > span > span + span { margin-left: 4px; font-weight: 700; }
</style>
