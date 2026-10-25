<script setup lang="ts">
import { Button } from '@/components/ui/button'
import AppIcon from './AppIcon.vue'
defineProps<{ error?: string; message?: string }>()
defineEmits<{ dismiss: [] }>()
</script>

<template>
  <p
    v-if="error"
    class="error"
    role="alert"
  >
    <AppIcon name="info" />{{ error }}
  </p>
  <Transition name="feedback">
    <div
      v-if="message"
      class="operation-feedback"
      role="status"
    >
      <AppIcon name="check" />
      <span>{{ message }}</span>
      <Button
        type="button"
        aria-label="关闭提示"
        variant="ghost"
        size="icon-sm"
        @click="$emit('dismiss')"
      >
        <AppIcon name="close" />
      </Button>
    </div>
  </Transition>
</template>
