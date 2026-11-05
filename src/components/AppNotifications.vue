<script setup lang="ts">
import { Toaster } from 'vue-sonner'
import 'vue-sonner/style.css'
import { useTheme } from '../composables/useTheme'
const { theme } = useTheme()
</script>

<template>
  <Toaster
    position="top-right"
    :theme="theme"
    rich-colors
    close-button
    :expand="true"
    :visible-toasts="3"
    :offset="{ top: 80, right: 24 }"
    :mobile-offset="{ top: 80, right: 16, left: 16 }"
    :toast-options="{ closeButtonAriaLabel: '关闭提示', closeButtonPosition: 'top-right' }"
    container-aria-label="操作提示"
  />
</template>

<style>
[data-sonner-toaster] { --width: 400px; font-family: inherit; }
[data-sonner-toaster][data-sonner-theme] { --normal-bg: var(--card); --normal-bg-hover: var(--secondary); --normal-border: var(--input-border); --normal-border-hover: var(--muted-foreground); --normal-text: var(--foreground); --success-bg: color-mix(in srgb, var(--primary) 8%, var(--card)); --success-border: var(--active-border); --success-text: var(--primary); --info-bg: var(--card); --info-border: var(--input-border); --info-text: var(--info); --warning-bg: var(--warning-surface); --warning-border: var(--warning); --warning-text: var(--warning); --error-bg: var(--error-surface); --error-border: var(--destructive); --error-text: var(--destructive); }
[data-sonner-toast] { font-size: 14px; }
[data-sonner-toast] [data-title] { white-space: pre-wrap; overflow-wrap: anywhere; max-height: 45vh; overflow-y: auto; padding-right: 12px; line-height: 1.6; }
@media (prefers-reduced-motion: reduce) {
  [data-sonner-toast] { transition: none !important; animation: none !important; }
}
</style>
