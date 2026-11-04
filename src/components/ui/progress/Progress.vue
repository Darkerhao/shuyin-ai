<script setup lang="ts">
import type { HTMLAttributes } from 'vue'
import { ProgressIndicator, ProgressRoot } from 'reka-ui'
import { cn } from '@/lib/utils'

defineOptions({ name: 'UiProgress' })
const props = defineProps<{ modelValue?: number | null; class?: HTMLAttributes['class'] }>()
</script>

<template>
  <ProgressRoot
    data-slot="progress"
    :model-value="modelValue ?? null"
    :class="cn('relative h-2 w-full overflow-hidden rounded-full bg-primary/20', props.class)"
  >
    <ProgressIndicator
      data-slot="progress-indicator"
      class="progress-indicator h-full rounded-full bg-primary"
      :style="modelValue == null ? undefined : { width: `${modelValue}%` }"
    />
  </ProgressRoot>
</template>

<style scoped>
.progress-indicator { transition: width 180ms ease; }
.progress-indicator[data-state='indeterminate'] { width: 30%; animation: processing 2.4s ease-in-out infinite; }
@keyframes processing { from { transform: translateX(-100%); } to { transform: translateX(440%); } }
@media (prefers-reduced-motion: reduce) { .progress-indicator { animation: none !important; transition: none; } }
</style>
