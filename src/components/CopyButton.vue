<script setup lang="ts">
import { Button } from '@/components/ui/button'
import AppIcon from './AppIcon.vue'
defineProps<{ label: string; copied: boolean; disabled?: boolean; compact?: boolean }>()
const emit = defineEmits<{ click: [] }>()
</script>

<template>
  <Button
    type="button"
    :variant="compact ? 'ghost' : 'outline'"
    :size="compact ? 'sm' : 'default'"
    :disabled="disabled"
    :aria-label="label"
    :class="[{ 'bg-accent text-primary': copied }, compact ? 'copy-compact' : '']"
    @click="emit('click')"
  >
    <AppIcon :name="copied ? 'check' : 'copy'" />
    <span
      class="copy-label"
      aria-hidden="true"
    >
      <span :style="{ visibility: copied ? 'hidden' : 'visible' }">{{ label }}</span>
      <span v-if="copied">已复制</span>
    </span>
  </Button>
</template>

<style scoped>
.copy-compact { height: 16px; padding: 0; color: var(--muted-foreground); font: 400 12px/16px var(--font-mono); }
.copy-compact:hover { color: var(--info); background: transparent; }
.copy-label { display: inline-grid; }
.copy-label > span { grid-area: 1 / 1; }
</style>
