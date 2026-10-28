<script setup lang="ts">
import { Toaster } from 'vue-sonner'
import 'vue-sonner/style.css'
</script>

<template>
  <Toaster
    position="top-right"
    theme="light"
    rich-colors
    close-button
    :expand="true"
    :visible-toasts="3"
    :offset="24"
    :mobile-offset="16"
    :toast-options="{ closeButtonAriaLabel: '关闭提示', closeButtonPosition: 'top-right' }"
    container-aria-label="操作提示"
  />
</template>

<style>
[data-sonner-toaster] { --width: 400px; font-family: inherit; }
[data-sonner-toast] { font-size: 14px; }
[data-sonner-toast] [data-title] { white-space: pre-wrap; overflow-wrap: anywhere; max-height: 45vh; overflow-y: auto; padding-right: 12px; line-height: 1.6; }
@media (prefers-reduced-motion: reduce) {
  [data-sonner-toast] { transition: none !important; animation: none !important; }
}
</style>
