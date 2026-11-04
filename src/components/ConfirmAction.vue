<script setup lang="ts">
import { Button } from '@/components/ui/button'
import { AlertDialog, AlertDialogCancel, AlertDialogContent, AlertDialogTitle } from '@/components/ui/alert-dialog'

defineOptions({ inheritAttrs: false })

const props = withDefaults(defineProps<{
  title: string
  confirmLabel?: string
  cancelLabel?: string
  busy?: boolean
  disabled?: boolean
  destructive?: boolean
  form?: string
  returnFocus?: globalThis.HTMLElement | null
}>(), { confirmLabel: '确认删除', cancelLabel: '取消', destructive: true, form: undefined, returnFocus: null })
const emit = defineEmits<{ confirm: []; cancel: [] }>()
const opener = globalThis.document.activeElement instanceof globalThis.HTMLElement ? globalThis.document.activeElement : null
function restoreFocus(event: globalThis.Event) {
  event.preventDefault()
  const target = props.returnFocus ?? opener
  if (target?.isConnected) target.focus({ preventScroll: true })
}
</script>

<template>
  <AlertDialog
    :open="true"
    @update:open="!$event && !busy && emit('cancel')"
  >
    <AlertDialogContent
      v-bind="$attrs"
      :aria-describedby="undefined"
      :aria-busy="busy"
      @escape-key-down="busy && $event.preventDefault()"
      @close-auto-focus="restoreFocus"
    >
      <AlertDialogTitle class="text-lg font-semibold">
        {{ title }}
      </AlertDialogTitle>
      <slot />
      <div class="actions wrap">
        <slot name="actions" />
        <Button
          :type="form ? 'submit' : 'button'"
          :form="form"
          :variant="destructive ? 'destructive' : 'default'"
          :disabled="busy || disabled"
          @click="!form && emit('confirm')"
        >
          {{ busy ? '处理中…' : confirmLabel }}
        </Button>
        <AlertDialogCancel as-child>
          <Button
            data-cancel
            type="button"
            variant="outline"
            :disabled="busy"
          >
            {{ cancelLabel }}
          </Button>
        </AlertDialogCancel>
      </div>
    </AlertDialogContent>
  </AlertDialog>
</template>
