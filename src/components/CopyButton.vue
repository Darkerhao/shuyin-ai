<script setup lang="ts">
import { Button } from '@/components/ui/button'
import AppIcon from './AppIcon.vue'
defineProps<{ label: string; copied: boolean; disabled?: boolean; compact?: boolean }>()
const emit = defineEmits<{ click: [] }>()
</script>

<template>
  <Button
    type="button"
    variant="outline"
    :size="compact ? 'sm' : 'default'"
    :disabled="disabled"
    :aria-label="label"
    :class="{ 'bg-accent text-primary': copied }"
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
.copy-label { display: inline-grid; }
.copy-label > span { grid-area: 1 / 1; }
</style>
