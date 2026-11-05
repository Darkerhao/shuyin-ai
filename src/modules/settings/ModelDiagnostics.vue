<script setup lang="ts">
import { computed, ref, onMounted, onBeforeUnmount } from 'vue'
import { Button } from '@/components/ui/button'
import { modelCalls, type ModelCall } from '../../services/maintenance'
import { errorMessage } from '../../services/verification'
import AppIcon from '../../components/AppIcon.vue'
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
    class="panel model-diagnostics"
    aria-label="模型调用诊断"
  >
    <div class="diagnostics-heading">
      <div>
        <h2>
          <AppIcon name="clock" />模型调用诊断<span
            v-if="running"
            class="diagnostics-running"
          >请求进行中</span>
        </h2><p class="muted">
          最近 200 次请求的响应时间与实际 Token 用量。
        </p>
      </div><Button
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
      <table>
        <thead>
          <tr>
            <th scope="col">
              时间 / 书籍
            </th><th scope="col">
              模型 / 阶段
            </th><th scope="col">
              结果
            </th><th scope="col">
              首响应 / 总耗时
            </th><th scope="col">
              输入 / 输出 / 总 Token
            </th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="row in rows"
            :key="row.id"
            class="border-t align-top"
          >
            <td class="p-3">
              {{ new Date(row.startedAt).toLocaleString() }}<span class="diagnostic-secondary">{{ row.title || '模型连接检查' }}</span>
            </td><td class="p-3 break-all">
              <strong class="diagnostic-model">{{ row.model }}</strong><span class="diagnostic-secondary">{{ stages[row.stage] || row.stage }}</span>
            </td><td class="p-3">
              <span
                class="diagnostic-status"
                :data-status="row.status"
              >{{ statuses[row.status] || row.status }}</span><span class="diagnostic-secondary">{{ row.errorCode || '' }}</span>
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

<style scoped>
.diagnostics-heading { display: flex; justify-content: space-between; align-items: center; flex-wrap: wrap; gap: 12px; }
.diagnostics-heading h2 { display: flex; align-items: center; flex-wrap: wrap; gap: 8px; margin: 0; font-size: 16px; }
.diagnostics-heading h2 > .app-icon { color: var(--primary); }
.diagnostics-heading p { margin: 4px 0 0; }
.diagnostics-running { padding: 2px 8px; border: 1px solid var(--active-border); border-radius: 6px; color: var(--primary); background: var(--accent); font: 400 10px/15px var(--font-mono); }
.model-diagnostics > .muted { font: 400 11px/18px var(--font-mono); margin-block: 16px; }
table { width: 100%; min-width: 740px; text-align: left; font: 400 12px/18px var(--font-mono); }
th { padding: 12px; color: var(--muted-foreground); border-bottom: 1px solid var(--border); font-weight: 500; white-space: nowrap; }
td { color: #e2e8f0; }
td:first-child { white-space: nowrap; }
th:last-child, td:last-child { text-align: right; white-space: nowrap; }
tr:hover td { background: #1e293b4d; }
.diagnostic-secondary { display: block; color: var(--muted-foreground); font-size: 11px; margin-top: 2px; }
.diagnostic-model { color: var(--info); font-weight: 700; }
.diagnostic-status { display: inline-block; white-space: nowrap; padding: 2px 8px; border-radius: 6px; color: var(--muted-foreground); background: var(--secondary); font-size: 10px; }
.diagnostic-status[data-status='succeeded'], .diagnostic-status[data-status='running'] { color: var(--primary); background: var(--accent); }
.diagnostic-status[data-status='failed'] { color: var(--destructive); background: var(--error-surface); }
.diagnostic-status[data-status='interrupted'], .diagnostic-status[data-status='cancelled'] { color: var(--warning); background: var(--warning-surface); }
</style>
