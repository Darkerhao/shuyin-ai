<script setup lang="ts">
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
      <button
        type="button"
        class="icon-button ghost"
        aria-label="关闭提示"
        @click="$emit('dismiss')"
      >
        <AppIcon name="close" />
      </button>
    </div>
  </Transition>
</template>
