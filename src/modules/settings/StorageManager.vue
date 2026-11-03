<script setup lang="ts">
import { ref } from 'vue'
import { Button } from '@/components/ui/button'
import ConfirmAction from '../../components/ConfirmAction.vue'
import * as api from '../../services/maintenance'
import { bytesLabel } from '../../services/maintenance'
import { errorMessage } from '../../services/verification'
const props = defineProps<{ disabled: boolean }>()
const emit = defineEmits<{ working: [value: boolean] }>()
const rows = ref<api.StorageUsage[]>([])
const preview = ref<api.CachePreview>()
const busy = ref(false)
const error = ref('')
const message = ref('')
async function run(action: () => Promise<void>) {
  if (busy.value || props.disabled) return
  busy.value = true; emit('working', true); error.value = ''; message.value = ''
  try { await action() } catch (cause) { error.value = errorMessage(cause) }
  finally { busy.value = false; emit('working', false) }
}
const scan = () => run(async () => { rows.value = await api.storageUsage() })
const inspect = (id: string) => run(async () => { preview.value = await api.previewCleanup(id) })
const clean = () => run(async () => {
  const current = preview.value; if (!current) return
  preview.value = undefined
  const bytes = await api.cleanCache(current)
  message.value = `已释放 ${bytesLabel(bytes)}。需要试片时可重新生成。`
  rows.value = await api.storageUsage()
})
</script>
<template>
  <section
    class="panel"
    aria-label="缓存管理"
  >
    <h2>视频缓存与书籍占用</h2>
    <p class="muted">
      统计书籍工作目录内文件，不计外部关联素材。可清理项仅包含可重建的渲染缓存和试片，原始素材、已登记文件及运行目录保留。目录链接不跟随。
    </p>
    <Button
      variant="outline"
      :disabled="disabled || busy"
      @click="scan"
    >
      {{ busy ? '处理中…' : '扫描占用' }}
    </Button>
    <p
      v-if="error"
      role="alert"
      class="notice"
    >
      {{ error }}
    </p><p
      v-if="message"
      role="status"
    >
      {{ message }}
    </p>
    <ul class="mt-4 grid gap-4">
      <li
        v-for="row in rows"
        :key="row.projectId"
        class="rounded-xl border p-4"
      >
        <strong>{{ row.title }}</strong><p>目录占用 {{ bytesLabel(row.bytes) }} · 可释放 {{ bytesLabel(row.reclaimableBytes) }}（{{ row.files }}个文件）</p>
        <p
          v-if="row.error"
          class="notice"
        >
          {{ row.error }}
        </p>
        <Button
          variant="outline"
          :disabled="disabled || busy || row.blocked || !row.files"
          @click="inspect(row.projectId)"
        >
          预览清理
        </Button>
      </li>
    </ul>
    <ConfirmAction
      v-if="preview"
      title="确认清理这些视频缓存？"
      confirm-label="清理列出的缓存"
      :busy="busy"
      @confirm="clean"
      @cancel="preview = undefined"
    >
      <p>将删除 {{ preview.files.length }} 个可重建文件，释放 {{ bytesLabel(preview.bytes) }}。试片和渲染缓存需要时重新生成。</p>
      <ul class="max-h-60 overflow-auto text-sm">
        <li
          v-for="file in preview.files"
          :key="file"
          class="break-all"
        >
          {{ file }}
        </li>
      </ul>
    </ConfirmAction>
  </section>
</template>
