<script setup lang="ts">
const props = defineProps<{ id: string; modelValue: string; label: string; tabs: readonly { value: string; label: string }[] }>()
const emit = defineEmits<{ 'update:modelValue': [value: string] }>()
function navigate(event: globalThis.KeyboardEvent, index: number) {
  if (!['ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(event.key)) return
  event.preventDefault()
  const next = event.key === 'Home' ? 0 : event.key === 'End' ? props.tabs.length - 1 : (index + (event.key === 'ArrowRight' ? 1 : -1) + props.tabs.length) % props.tabs.length
  emit('update:modelValue', props.tabs[next]!.value)
  const target = event.currentTarget as globalThis.HTMLButtonElement
  target.parentElement?.querySelectorAll('button')[next]?.focus()
}
</script>

<template>
  <div
    class="editor-tabs"
    role="tablist"
    :aria-label="label"
  >
    <button
      v-for="(tab, index) in tabs"
      :id="`${id}-tab-${tab.value}`"
      :key="tab.value"
      type="button"
      role="tab"
      :aria-selected="modelValue === tab.value"
      :aria-controls="`${id}-panel-${tab.value}`"
      :tabindex="modelValue === tab.value ? 0 : -1"
      @click="emit('update:modelValue', tab.value)"
      @keydown="navigate($event, index)"
    >
      {{ tab.label }}
    </button>
  </div>
</template>
