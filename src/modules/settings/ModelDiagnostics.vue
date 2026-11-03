<script setup lang="ts">
import { computed, ref, onMounted, onBeforeUnmount } from 'vue'
import { Button } from '@/components/ui/button'
import { modelCalls, type ModelCall } from '../../services/maintenance'
import { errorMessage } from '../../services/verification'
const rows = ref<ModelCall[]>([])
const busy = ref(false)
const error = ref('')
let timer: ReturnType<typeof globalThis.setInterval> | undefined
let disposed = false
const stages: Record<string, string> = { connection_test: '连接测试', writing: '内容生成', writing_revision: '文案调整', duration: '时长建议', cover: '封面生成', video_illustration: '视频插画', video_scene_revision: '单镜调整', proofreading: '字幕校对', storyboard: '分镜生成' }
const statuses: Record<string, string> = { running: '请求中', succeeded: '响应完成', failed: '请求失败', cancelled: '已取消', interrupted: '被中断' }
const running = computed(() => rows.value.some(row => row.status === 'running'))
const duration = (ms: number | null) => ms === null ? '—' : `${(ms / 1000).toFixed(2)}秒`
async function load() {
  if (busy.value) return
  busy.value = true
  try { const values = await modelCalls(); if (!disposed) { rows.value = values; error.value = '' } }
  catch (cause) { if (!disposed) error.value = errorMessage(cause) }
  finally { busy.value = false }
}
onMounted(() => { void load(); timer = globalThis.setInterval(() => { if (running.value) void load() }, 3000) })
onBeforeUnmount(() => { disposed = true; globalThis.clearInterval(timer) })
</script>
<template>
  <section
    class="panel"
    aria-label="模型调用诊断"
  >
    <div class="actions">
      <h2>模型调用诊断</h2><Button
        variant="outline"
        :disabled="busy"
        @click="load"
      >
        刷新调用记录
      </Button>
    </div>
    <p class="muted">
      最近200次请求。首响应是收到服务响应头的时间，总耗时包含接收完整响应。Token仅显示服务实际返回的值，“—”表示未返回。响应完成不代表内容已通过业务校验或已采用。
    </p>
    <p
      v-if="error"
      role="alert"
      class="notice"
    >
      {{ error }}
    </p><p v-if="!busy && !rows.length">
      还没有调用记录，新发起的请求将在这里显示。
    </p>
    <div
      v-if="rows.length"
      class="overflow-x-auto"
    >
      <table class="w-full text-left text-sm">
        <thead><tr><th>时间 / 书籍</th><th>模型 / 阶段</th><th>结果</th><th>首响应 / 总耗时</th><th>输入 / 输出 / 总 Token</th></tr></thead>
        <tbody>
          <tr
            v-for="row in rows"
            :key="row.id"
            class="border-t align-top"
          >
            <td class="p-3">
              {{ new Date(row.startedAt).toLocaleString() }}<br>{{ row.title || '模型连接检查' }}
            </td><td class="p-3 break-all">
              {{ row.model }}<br>{{ stages[row.stage] || row.stage }}
            </td><td class="p-3">
              {{ statuses[row.status] || row.status }}<br>{{ row.errorCode || '' }}
            </td><td class="p-3">
              {{ duration(row.firstResponseMs) }} / {{ duration(row.elapsedMs) }}
            </td><td class="p-3">
              {{ row.inputTokens ?? '—' }} / {{ row.outputTokens ?? '—' }} / {{ row.totalTokens ?? '—' }}
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  </section>
</template>
