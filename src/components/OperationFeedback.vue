<script setup lang="ts">
import { onBeforeUnmount, watch } from 'vue'
import { toast } from 'vue-sonner'
import { displayPath } from '@/lib/displayPath'

const props = withDefaults(defineProps<{
  error?: string
  message?: string
  variant?: 'success' | 'info' | 'warning'
}>(), { error: '', message: '', variant: 'success' })
const emit = defineEmits<{ dismiss: [] }>()
let errorId: string | number | undefined
let messageId: string | number | undefined

watch(() => props.error, value => {
  if (errorId !== undefined) toast.dismiss(errorId)
  errorId = value ? toast.error(displayPath(value), { duration: Infinity }) : undefined
}, { immediate: true })

watch(() => props.message, value => {
  if (messageId !== undefined) toast.dismiss(messageId)
  messageId = value ? toast[props.variant](displayPath(value), {
    duration: props.variant === 'warning' ? 8000 : 5000,
    onDismiss: () => { if (props.message === value) emit('dismiss') },
    onAutoClose: () => { if (props.message === value) emit('dismiss') },
  }) : undefined
}, { immediate: true })

onBeforeUnmount(() => {
  if (errorId !== undefined) toast.dismiss(errorId)
})
</script>

<template>
  <slot />
</template>
