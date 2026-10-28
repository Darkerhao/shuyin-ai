<script setup lang="ts">
import { nextTick, onBeforeUnmount, onMounted, ref, useId } from 'vue'
import { Button } from '@/components/ui/button'

withDefaults(defineProps<{
  title: string
  confirmLabel?: string
  cancelLabel?: string
  busy?: boolean
  disabled?: boolean
  destructive?: boolean
}>(), { confirmLabel: '确认删除', cancelLabel: '取消', destructive: true })
const emit = defineEmits<{ confirm: []; cancel: [] }>()
const titleId = useId()
const panel = ref<globalThis.HTMLElement | null>(null)
const opener = globalThis.document.activeElement instanceof globalThis.HTMLElement ? globalThis.document.activeElement : null
onMounted(async () => {
  await nextTick()
  panel.value?.querySelector<globalThis.HTMLButtonElement>('[data-cancel]')?.focus()
})
onBeforeUnmount(() => { if (opener?.isConnected) opener.focus({ preventScroll: true }) })
</script>

<template>
  <section
    ref="panel"
    class="panel confirmation"
    role="alertdialog"
    aria-modal="false"
    :aria-labelledby="titleId"
    :aria-busy="busy"
    @keydown.esc.stop.prevent="!busy && emit('cancel')"
  >
    <h2 :id="titleId">
      {{ title }}
    </h2>
    <slot />
    <div class="actions wrap">
      <slot name="actions" />
      <Button
        type="button"
        :variant="destructive ? 'destructive' : 'default'"
        :disabled="busy || disabled"
        @click="emit('confirm')"
      >
        {{ busy ? '处理中…' : confirmLabel }}
      </Button>
      <Button
        data-cancel
        type="button"
        variant="outline"
        :disabled="busy"
        @click="emit('cancel')"
      >
        {{ cancelLabel }}
      </Button>
    </div>
  </section>
</template>
