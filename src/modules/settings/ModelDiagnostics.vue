<script setup lang="ts">
import { computed, ref, onMounted, onBeforeUnmount, watch } from 'vue'
import { Button } from '@/components/ui/button'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { modelCalls, clearModelCalls, setModelCallRetention, type ModelCallPage } from '../../services/maintenance'
import { errorMessage } from '../../services/verification'
import AppIcon from '../../components/AppIcon.vue'
import ConfirmAction from '../../components/ConfirmAction.vue'

const props = withDefaults(defineProps<{ disabled?: boolean }>(), { disabled: false })
const emit = defineEmits<{ working: [value: boolean] }>()
const result = ref<ModelCallPage | null>(null)
const busy = ref(false)
const error = ref('')
const message = ref('')
const retention = ref('0')
const pending = ref<'clear' | 'retention' | null>(null)
const tableContainer = ref<globalThis.HTMLElement | null>(null)
let timer: ReturnType<typeof globalThis.setInterval> | undefined
let disposed = false
let refreshAfterMaintenance = false
const stages: Record<string, string> = { connection_test: '连接测试', writing: '内容生成', writing_revision: '文案调整', duration: '时长建议', cover: '封面生成', video_illustration: '视频插画', video_scene_revision: '单镜调整', proofreading: '字幕校对', storyboard: '分镜生成' }
const statuses: Record<string, string> = { running: '请求中', succeeded: '响应完成', failed: '请求失败', cancelled: '已取消', interrupted: '被中断' }
const retentionOptions = [{ value: '0', label: '全部保留' }, ...[7, 30, 90].map(days => ({ value: String(days), label: `保留最近 ${days} 天` }))]
const rows = computed(() => result.value?.rows ?? [])
const page = computed(() => result.value?.page ?? 1)
const pageSize = computed(() => result.value?.pageSize ?? 20)
const total = computed(() => result.value?.total ?? 0)
const pages = computed(() => Math.max(1, Math.ceil(total.value / pageSize.value)))
const running = computed(() => result.value?.runningCount ?? 0)
const locked = computed(() => props.disabled || busy.value || pending.value !== null)
const duration = (ms: number | null) => ms === null ? '—' : `${(ms / 1000).toFixed(2)}秒`

async function readPage(nextPage: number, nextSize: number) {
  const values = await modelCalls(nextPage, nextSize)
  if (disposed || props.disabled || refreshAfterMaintenance) return
  if (!result.value || retention.value === String(result.value.retentionDays)) retention.value = String(values.retentionDays)
  if (tableContainer.value && (values.page !== page.value || values.pageSize !== pageSize.value)) tableContainer.value.scrollTop = 0
  result.value = values
}
async function load(nextPage = page.value, nextSize = pageSize.value) {
  if (locked.value) return
  refreshAfterMaintenance = false
  busy.value = true
  error.value = ''
  try { await readPage(nextPage, nextSize) }
  catch (cause) { if (!disposed && !props.disabled && !refreshAfterMaintenance) error.value = errorMessage(cause) }
  finally {
    busy.value = false
    if (!disposed && refreshAfterMaintenance) void load()
  }
}
async function apply(action: 'clear' | 'retention') {
  if (busy.value || props.disabled) return
  const days = Number(retention.value)
  busy.value = true; emit('working', true); error.value = ''; message.value = ''
  try {
    const deleted = action === 'clear' ? await clearModelCalls() : await setModelCallRetention(days)
    if (disposed) return
    if (action === 'retention' && result.value) result.value.retentionDays = days
    message.value = action === 'clear'
      ? `已清除 ${deleted} 条已结束的调用记录，正在请求中的记录已保留。`
      : retention.value === '0' ? '已关闭自动清理，之后的调用记录将全部保留。' : `已保存保留最近 ${retention.value} 天，清理了 ${deleted} 条过期记录。`
    pending.value = null
    if (tableContainer.value) tableContainer.value.scrollTop = 0
    await readPage(1, pageSize.value)
  } catch (cause) { if (!disposed) error.value = errorMessage(cause) }
  finally { busy.value = false; emit('working', false) }
}
function saveRetention() {
  error.value = ''; message.value = ''
  if (retention.value === '0') void apply('retention')
  else pending.value = 'retention'
}
watch(() => props.disabled, (disabled, previous) => {
  if (previous && !disabled) { refreshAfterMaintenance = true; void load() }
})
onMounted(() => {
  void load()
  timer = globalThis.setInterval(() => { if (page.value === 1 && running.value && !error.value) void load() }, 3000)
})
onBeforeUnmount(() => { disposed = true; globalThis.clearInterval(timer) })
</script>
<template>
  <section
    class="panel model-diagnostics"
    aria-label="模型调用诊断"
    :aria-busy="busy"
  >
    <div class="diagnostics-heading">
      <div>
        <h2>
          <AppIcon name="clock" />模型调用诊断<span
            v-if="running"
            class="diagnostics-running"
          >{{ running }} 条请求进行中</span>
        </h2><p class="muted">
          调用记录保存在本机，按页查看响应时间与实际 Token 用量。
        </p>
      </div><Button
        variant="outline"
        :disabled="locked"
        @click="load()"
      >
        {{ busy ? '处理中…' : '刷新调用记录' }}
      </Button>
    </div>
    <div class="diagnostics-maintenance">
      <div class="diagnostics-retention">
        <label for="model-call-retention">自动清理</label>
        <Select
          v-model="retention"
          :disabled="locked || !result"
        >
          <SelectTrigger
            id="model-call-retention"
            aria-describedby="model-call-retention-hint"
          >
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem
              v-for="option in retentionOptions"
              :key="option.value"
              :value="option.value"
            >
              {{ option.label }}
            </SelectItem>
          </SelectContent>
        </Select>
        <Button
          variant="outline"
          :disabled="locked || !result || Number(retention) === result.retentionDays"
          @click="saveRetention"
        >
          保存设置
        </Button>
      </div>
      <Button
        variant="outline"
        :disabled="locked || total === running"
        @click="pending = 'clear'; error = ''; message = ''"
      >
        清除记录
      </Button>
      <p
        id="model-call-retention-hint"
        class="muted"
      >
        默认全部保留。启用后立即清理，之后在应用启动时及运行期间每小时检查；正在请求中的记录始终保留。
      </p>
    </div>
    <p class="muted">
      首响应是收到服务响应头的时间，总耗时包含接收完整响应。Token 仅显示服务实际返回的值，“—”表示未返回。响应完成不代表内容已通过业务校验或已采用。
    </p>
    <p
      v-if="error && !pending"
      role="alert"
      class="notice"
    >
      {{ error }}
    </p>
    <p
      v-if="message"
      role="status"
      class="diagnostics-message"
    >
      {{ message }}
    </p>
    <p
      v-if="busy && !result"
      role="status"
    >
      正在读取调用记录…
    </p>
    <p v-else-if="result && !rows.length && !error">
      还没有调用记录，新发起的请求将在这里显示。
    </p>
    <div
      v-if="rows.length"
      ref="tableContainer"
      class="diagnostics-table"
      role="region"
      aria-label="模型调用记录表"
      tabindex="0"
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
    <footer
      v-if="result"
      class="diagnostics-pagination"
    >
      <p
        class="diagnostics-count"
        role="status"
        aria-live="polite"
      >
        共 {{ total }} 条<span v-if="total"> · 第 {{ (page - 1) * pageSize + 1 }}–{{ Math.min(page * pageSize, total) }} 条</span>
      </p>
      <div class="diagnostics-page-size">
        <label for="model-call-page-size">每页</label>
        <Select
          :model-value="String(pageSize)"
          :disabled="locked"
          @update:model-value="load(1, Number($event))"
        >
          <SelectTrigger id="model-call-page-size">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem
              v-for="size in [10, 20, 50, 100]"
              :key="size"
              :value="String(size)"
            >
              {{ size }} 条
            </SelectItem>
          </SelectContent>
        </Select>
      </div>
      <nav
        class="diagnostics-pages"
        aria-label="调用记录分页"
      >
        <Button
          variant="outline"
          size="sm"
          :disabled="locked || page === 1"
          @click="load(1)"
        >
          首页
        </Button>
        <Button
          variant="outline"
          size="sm"
          :disabled="locked || page === 1"
          @click="load(page - 1)"
        >
          上一页
        </Button>
        <span>第 {{ page }} / {{ pages }} 页</span>
        <Button
          variant="outline"
          size="sm"
          :disabled="locked || page >= pages"
          @click="load(page + 1)"
        >
          下一页
        </Button>
        <Button
          variant="outline"
          size="sm"
          :disabled="locked || page >= pages"
          @click="load(pages)"
        >
          末页
        </Button>
      </nav>
    </footer>
    <ConfirmAction
      v-if="pending"
      :title="pending === 'clear' ? '清除全部已结束的调用记录？' : '启用调用记录自动清理？'"
      :confirm-label="pending === 'clear' ? '确认清除' : '保存并清理'"
      :busy="busy"
      :disabled="disabled"
      @confirm="pending && apply(pending)"
      @cancel="pending = null; error = ''"
    >
      <p v-if="pending === 'clear'">
        将清除全部已结束的调用记录（当前 {{ total - running }} 条），正在请求中的记录保留。
      </p>
      <p v-else>
        将保留最近 {{ retention }} 天的记录，并立即清除更早的已结束记录。此后在应用启动时及运行期间每小时自动检查。
      </p>
      <p>仅删除响应时间、Token 用量等诊断记录，不影响书籍、创作内容、素材或任务。清除后无法撤销。</p>
      <p
        v-if="error"
        role="alert"
        class="notice"
      >
        {{ error }}
      </p>
    </ConfirmAction>
  </section>
</template>

<style scoped>
.diagnostics-heading { display: flex; justify-content: space-between; align-items: center; flex-wrap: wrap; gap: 12px; }
.diagnostics-heading h2 { display: flex; align-items: center; flex-wrap: wrap; gap: 8px; margin: 0; font-size: 16px; }
.diagnostics-heading h2 > .app-icon { color: var(--primary); }
.diagnostics-heading p { margin: 4px 0 0; font-size: 12px; }
.diagnostics-running { padding: 2px 8px; border: 1px solid var(--active-border); border-radius: 6px; color: var(--primary); background: var(--accent); font: 400 10px/15px var(--font-mono); }
.diagnostics-maintenance { display: flex; align-items: center; justify-content: space-between; flex-wrap: wrap; gap: 12px; margin-top: 20px; padding: 16px; border: 1px solid var(--border); border-radius: 12px; background: color-mix(in srgb, var(--terminal) 50%, transparent); }
.diagnostics-retention, .diagnostics-page-size { display: flex; align-items: center; flex-wrap: wrap; gap: 8px; font-size: 12px; }
.diagnostics-maintenance > p { flex-basis: 100%; margin: 0; font-size: 12px; line-height: 20px; }
.model-diagnostics > .muted { font: 400 12px/20px var(--font-mono); margin-block: 16px; }
.diagnostics-message { color: var(--primary); font-size: 12px; }
.diagnostics-table { max-height: 520px; overflow: auto; border: 1px solid var(--border); border-radius: 12px; scrollbar-gutter: stable; }
.diagnostics-table:focus-visible { outline: 2px solid var(--ring); outline-offset: 2px; }
table { width: 100%; min-width: 740px; text-align: left; font: 400 12px/18px var(--font-mono); }
th { position: sticky; top: 0; z-index: 1; padding: 12px; color: var(--muted-foreground); background: var(--card); border-bottom: 1px solid var(--border); font-weight: 500; white-space: nowrap; }
td { color: var(--editor-foreground); }
td:first-child { white-space: nowrap; }
th:last-child, td:last-child { text-align: right; white-space: nowrap; }
tr:hover td { background: color-mix(in srgb, var(--secondary) 30%, transparent); }
.diagnostic-secondary { display: block; color: var(--muted-foreground); font-size: 12px; margin-top: 2px; }
.diagnostic-model { color: var(--info); font-weight: 700; }
.diagnostic-status { display: inline-block; white-space: nowrap; padding: 2px 8px; border-radius: 6px; color: var(--muted-foreground); background: var(--secondary); font-size: 10px; }
.diagnostic-status[data-status='succeeded'], .diagnostic-status[data-status='running'] { color: var(--primary); background: var(--accent); }
.diagnostic-status[data-status='failed'] { color: var(--destructive); background: var(--error-surface); }
.diagnostic-status[data-status='interrupted'], .diagnostic-status[data-status='cancelled'] { color: var(--warning); background: var(--warning-surface); }
.diagnostics-pagination { display: flex; align-items: center; flex-wrap: wrap; gap: 12px; padding-top: 16px; font: 400 12px/20px var(--font-mono); }
.diagnostics-count { margin: 0; margin-right: auto; color: var(--muted-foreground); }
.diagnostics-pages { display: flex; align-items: center; justify-content: flex-end; flex-wrap: wrap; gap: 8px; margin-left: auto; }
.diagnostics-pages > span { white-space: nowrap; color: var(--muted-foreground); }
@media (max-width: 700px) { .diagnostics-pages { flex-basis: 100%; justify-content: center; } .diagnostics-maintenance { padding: 12px; } }
</style>
