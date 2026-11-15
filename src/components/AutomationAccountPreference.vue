<script setup lang="ts">
import { onMounted, ref, useId, watch } from 'vue'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { getAutomationAccountReuse, setAutomationAccountReuse } from '../services/automation'

defineProps<{ disabled?: boolean }>()
const emit = defineEmits<{ busy: [value: boolean] }>()
const id = useId()
const enabled = ref(false)
const loaded = ref(false)
const pending = ref(true)
const error = ref('')
watch(pending, value => emit('busy', value), { immediate: true, flush: 'sync' })
onMounted(load)
async function load() {
  pending.value = true; error.value = ''
  try { enabled.value = await getAutomationAccountReuse(); loaded.value = true }
  catch (cause) { error.value = `账号确认设置读取失败：${String(cause)}` }
  finally { pending.value = false }
}
async function save(value: boolean | 'indeterminate') {
  if (pending.value || !loaded.value || typeof value !== 'boolean') return
  pending.value = true; error.value = ''
  try { await setAutomationAccountReuse(value); enabled.value = value }
  catch (cause) { error.value = `账号确认设置保存失败：${String(cause)}` }
  finally { pending.value = false }
}
</script>

<template>
  <div class="grid gap-2 text-sm">
    <label
      :for="id"
      class="flex items-start gap-3"
    >
      <Checkbox
        :id="id"
        :model-value="enabled"
        :disabled="disabled || pending || !loaded"
        :aria-describedby="`${id}-hint`"
        @update:model-value="save"
      />
      <span>自动沿用启动时确认的账号，不重复确认</span>
    </label>
    <p
      :id="`${id}-hint`"
      class="text-muted-foreground"
    >
      {{ pending ? '正在读取或保存设置…' : '设置会记住，适用于当前和后续自动任务。重开窗口或重新登录后，账号身份一致就继续上传和填写；账号变化时暂停。最后由你在抖音页面发布。' }}
    </p>
    <p
      v-if="error"
      role="alert"
      class="text-destructive"
    >
      {{ error }}
    </p>
    <Button
      v-if="!loaded && !pending"
      variant="outline"
      class="justify-self-start"
      :disabled="disabled"
      @click="load"
    >
      重新读取账号确认设置
    </Button>
  </div>
</template>
