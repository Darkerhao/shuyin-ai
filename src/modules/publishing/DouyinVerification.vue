<script setup lang="ts">
import { displayPath } from '@/lib/displayPath'
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { Button } from '@/components/ui/button'
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '@/components/ui/collapsible'
import { Checkbox } from '@/components/ui/checkbox'
import { Input } from '@/components/ui/input'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { continueDouyinTask, getDouyinSession, onDouyinProgress, openDouyinWindow, startDouyinTask, stopDouyinTask, type DouyinDecision, type DouyinSession } from '../../services/douyin'
import type { PublishTask } from '../../types/publishing'
import OperationFeedback from '../../components/OperationFeedback.vue'
import AppIcon from '../../components/AppIcon.vue'

const props = defineProps<{ desktop: boolean; task?: PublishTask | null; disabled?: boolean; hideStart?: boolean; prepareTask?: () => Promise<PublishTask> }>()
const emit = defineEmits<{ working: [active: boolean] }>()
const session = ref<DouyinSession | null>(null)
const pending = ref(false)
const stopping = ref(false)
const error = ref('')
const accountName = ref('')
const collection = ref<string | null>(null)
const manualCollection = ref('')
const remember = ref(false)
const active = computed(() => session.value?.status === 'running' || session.value?.status === 'waiting')
const locked = computed(() => pending.value || stopping.value || active.value)
watch(locked, value => emit('working', value), { immediate: true })
const steps = [
  { key: 'connect', label: '连接抖音', fields: ['connect'] },
  { key: 'video', label: '上传视频', fields: ['video'] },
  { key: 'text', label: '填写内容', fields: ['text'] },
  { key: 'covers', label: '封面与合集', fields: ['covers', 'collection'] },
  { key: 'settings', label: '发布设置', fields: ['settings'] },
  { key: 'review', label: '官方页面检查', fields: [] },
]
function stepState(step: typeof steps[number]) {
  if (step.fields.length && step.fields.every(field => session.value?.completed.includes(field))) return 'done'
  if (session.value?.step === step.key || (step.key === 'covers' && session.value?.step === 'collection')) return 'current'
  return 'pending'
}
const sessionLabel = computed(() => pending.value ? '正在准备' : session.value?.status === 'review' ? '准备完成，待人工发布' : session.value?.status === 'waiting' ? '等待你确认' : session.value?.status === 'running' ? '正在辅助发布' : session.value?.status === 'failed' ? '未完成' : session.value?.status === 'stopped' ? '已停止接管' : '等待开始')
let dispose: (() => void) | undefined
let unmounted = false
function accept(value: DouyinSession | null) {
  if (unmounted || !value || value.taskId !== props.task?.id || (session.value && value.runId < session.value.runId)) return
  if (session.value?.runId === value.runId && session.value.status === 'stopped' && value.status !== 'stopped') return
  if (value.account?.key !== session.value?.account?.key) { accountName.value = ''; manualCollection.value = ''; remember.value = false }
  session.value = value
  if (value.waitingFor === 'account' && !accountName.value) accountName.value = value.account?.name ?? ''
  collection.value = value.collection?.name ?? null
}
watch(() => props.task?.id, async () => {
  session.value = null; accountName.value = ''; manualCollection.value = ''; remember.value = false; error.value = ''
  if (props.desktop) {
    try { accept(await getDouyinSession()) } catch (cause) { error.value = String(cause) }
  }
})
onMounted(async () => {
  if (!props.desktop) return
  try {
    const unlisten = await onDouyinProgress(accept)
    if (unmounted) { unlisten(); return }
    dispose = unlisten
    accept(await getDouyinSession())
  } catch (cause) { error.value = String(cause) }
})
onBeforeUnmount(() => { unmounted = true; dispose?.(); emit('working', false) })
async function start() {
  if (locked.value || props.disabled || !props.desktop) return
  pending.value = true; error.value = ''
  try {
    const task = props.prepareTask ? await props.prepareTask() : props.task
    if (!task || task.status !== 'ready') throw new Error('请先补齐发布任务的文案、成片、音频、双封面和人工确认项。')
    accept(await startDouyinTask(task.id))
  } catch (cause) { error.value = String(cause) }
  finally { pending.value = false }
}
async function decide(decision: DouyinDecision) {
  if (!session.value || pending.value || stopping.value) return
  const runId = session.value.runId
  pending.value = true; error.value = ''
  try { accept(await continueDouyinTask(runId, decision)) }
  catch (cause) { error.value = String(cause) }
  finally { pending.value = false }
}
async function stop() {
  if (!session.value || stopping.value) return
  stopping.value = true; error.value = ''
  try { accept(await stopDouyinTask(session.value.runId)) }
  catch (cause) { error.value = String(cause) }
  finally { stopping.value = false }
}
async function show() {
  try { await openDouyinWindow() } catch (cause) { error.value = String(cause) }
}
const selectedNames = computed(() => [...new Set(session.value?.options.map(o => o.name) ?? [])])
defineExpose({ start })
</script>

<template>
  <section
    v-show="!hideStart || session || pending || error"
    class="panel douyin-workbench"
    :data-status="session?.status"
    aria-labelledby="douyin-verification-heading"
  >
    <div class="douyin-heading">
      <h2 id="douyin-verification-heading">
        <AppIcon name="upload" />抖音辅助发布
      </h2>
      <span
        class="douyin-session-status"
        role="status"
      >{{ sessionLabel }}</span>
    </div>
    <p class="muted">
      一次开始，自动准备当前任务。请在官方页面检查预览、声明和发布设置，亲自发布后回来记录实际结果。
    </p>
    <p v-if="task">
      <strong>{{ task.title || '待填写标题' }}</strong>
    </p>
    <p
      v-if="task"
      class="muted"
    >
      保存权限：{{ task.allowDownload ? '允许' : '不允许' }} · {{ task.scheduledAt ? `定时发布：${task.scheduledAt.slice(0, 16).replace('T', ' ')}（北京时间）` : '立即发布' }}
    </p>
    <slot name="summary" />
    <p v-if="session?.account">
      账号：{{ session.account.name || '请在官方窗口核对' }} · 目标合集：{{ session.collection ? session.collection.name ?? '不加入合集' : '待选择' }}
    </p>
    <ol
      class="douyin-steps"
      aria-label="辅助发布进度"
    >
      <li
        v-for="(step, index) in steps"
        :key="step.key"
        :data-state="stepState(step)"
        :aria-current="session?.step === step.key || (step.key === 'covers' && session?.step === 'collection') ? 'step' : undefined"
      >
        <span class="douyin-step-marker"><AppIcon
          v-if="stepState(step) === 'done'"
          name="check"
        /><template v-else>{{ index + 1 }}</template></span>
        <strong>{{ step.label }}</strong>
        <span v-if="stepState(step) === 'done'">
          {{ step.fields.some(field => session?.manual.includes(field)) ? '人工确认' : '已核对' }}
        </span>
        <span v-else-if="step.key === 'review' && session?.status === 'review'">等待你检查并发布</span>
        <span v-else-if="stepState(step) === 'current'">{{ session?.status === 'running' ? '进行中' : session?.status === 'waiting' ? '待确认' : session?.status === 'failed' ? '未完成' : '已停止' }}</span>
        <span v-else>待进行</span>
      </li>
    </ol>
    <p
      v-if="session"
      role="status"
      aria-live="polite"
    >
      {{ displayPath(session.message) }}
    </p>
    <div
      v-if="session?.status === 'waiting'"
      class="douyin-decision"
    >
      <template v-if="session.waitingFor === 'account'">
        <label for="douyin-account-name">官方窗口显示的账号昵称</label>
        <Input
          id="douyin-account-name"
          v-model="accountName"
          :disabled="pending || stopping"
          maxlength="200"
        />
        <Button
          type="button"
          :disabled="pending || stopping || !accountName.trim()"
          @click="decide({ kind: 'account', name: accountName })"
        >
          使用这个账号继续
        </Button>
        <p class="muted">
          上次确认的昵称会自动带入，可按官方窗口修改。当前窗口只需确认一次，后续步骤自动继续；退出登录、切换账号或重开窗口后再核对。
        </p>
      </template>
      <Button
        v-else-if="session.waitingFor === 'video'"
        type="button"
        :disabled="pending || stopping"
        @click="decide({ kind: 'existingVideo' })"
      >
        已核对视频，继续当前任务
      </Button>
      <Button
        v-else-if="session.waitingFor === 'processing'"
        type="button"
        :disabled="pending || stopping"
        @click="decide({ kind: 'processing' })"
      >
        已在官方页确认上传和处理完成
      </Button>
      <Button
        v-else-if="session.waitingFor === 'text'"
        type="button"
        :disabled="pending || stopping"
        @click="decide({ kind: 'text' })"
      >
        采用当前网页文案
      </Button>
      <Button
        v-else-if="session.waitingFor === 'covers'"
        type="button"
        :disabled="pending || stopping"
        @click="decide({ kind: 'covers' })"
      >
        已核对双封面，采用网页结果
      </Button>
      <template v-else-if="session.waitingFor === 'collection'">
        <label for="douyin-collection">本次合集</label>
        <Select
          v-model="collection"
          :disabled="pending || stopping"
        >
          <SelectTrigger id="douyin-collection">
            <SelectValue placeholder="请选择合集或明确不加入" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem :value="null">
              不加入合集
            </SelectItem>
            <SelectItem
              v-for="name in selectedNames"
              :key="name"
              :value="name"
              :disabled="session.options.filter(o => o.name === name).length !== 1"
            >
              {{ name }}
            </SelectItem>
          </SelectContent>
        </Select>
        <label
          v-if="session.account?.id"
          class="check-label"
          for="douyin-remember"
        >
          <Checkbox
            id="douyin-remember"
            v-model="remember"
            :disabled="pending || stopping"
          />记为这个账号的默认选择
        </label>
        <div class="actions wrap">
          <Button
            type="button"
            :disabled="pending || stopping"
            @click="decide({ kind: 'collection', name: collection, remember, manual: false })"
          >
            确认本次合集
          </Button>
          <Button
            type="button"
            variant="outline"
            :disabled="pending || stopping"
            @click="decide({ kind: 'page' })"
          >
            重新读取合集
          </Button>
        </div>
        <Collapsible :unmount-on-hide="false">
          <CollapsibleTrigger class="disclosure-trigger">
            在官方页人工设置合集
          </CollapsibleTrigger>
          <CollapsibleContent>
            <label for="douyin-manual-collection">已在官方页设置的合集名称（留空表示不加入）</label>
            <Input
              id="douyin-manual-collection"
              v-model="manualCollection"
              :disabled="pending || stopping"
              maxlength="200"
            />
            <Button
              type="button"
              variant="outline"
              :disabled="pending || stopping"
              @click="decide({ kind: 'collection', name: manualCollection.trim() || null, remember: false, manual: true })"
            >
              已核对官方页合集，人工确认
            </Button>
          </CollapsibleContent>
        </Collapsible>
      </template>
      <template v-else-if="session.waitingFor === 'settings'">
        <div class="actions wrap">
          <Button
            type="button"
            :disabled="pending || stopping"
            @click="decide({ kind: 'settings' })"
          >
            已按本次任务核对保存权限和发布时间
          </Button>
          <Button
            type="button"
            variant="outline"
            :disabled="pending || stopping"
            @click="decide({ kind: 'page' })"
          >
            重新填写发布设置
          </Button>
        </div>
        <p class="muted">
          如需更改任务中的时间或保存权限，请先停止并人工接管，再修改并保存任务。
        </p>
      </template>
      <Button
        v-else
        type="button"
        :disabled="pending || stopping"
        @click="decide({ kind: 'page' })"
      >
        已处理，继续核对
      </Button>
    </div>
    <div class="actions wrap">
      <Button
        v-if="(!hideStart || session?.status === 'failed') && !active && session?.status !== 'review'"
        type="button"
        :disabled="!desktop || disabled || pending || stopping || task?.status === 'published'"
        @click="start"
      >
        {{ session?.status === 'failed' ? '重新核对当前网页' : '开始抖音辅助发布' }}
      </Button>
      <Button
        v-if="session || pending"
        type="button"
        :variant="session?.status === 'review' ? 'default' : 'outline'"
        :disabled="!desktop"
        @click="show"
      >
        <AppIcon name="eye" />{{ session?.status === 'review' ? '去官方页面检查' : '显示抖音窗口' }}
      </Button>
      <Button
        v-if="active || pending || session?.status === 'review'"
        type="button"
        variant="outline"
        :disabled="!session || stopping"
        @click="stop"
      >
        {{ stopping ? '正在停止…' : '停止并人工接管' }}
      </Button>
    </div>
    <OperationFeedback
      :error="error"
    />
    <OperationFeedback
      :error="session?.status === 'failed' ? session.message : ''"
      :message="session?.status === 'waiting' || session?.status === 'review' ? session.message : ''"
      variant="warning"
    />
  </section>
</template>

<style scoped>
.douyin-heading { display: flex; flex-wrap: wrap; align-items: center; justify-content: space-between; gap: 12px; }
.douyin-heading h2 { display: flex; align-items: center; gap: 8px; margin: 0; }
.douyin-heading .app-icon { color: var(--primary); }
.douyin-session-status { padding: 5px 10px; border: 1px solid var(--input-border); border-radius: 8px; color: var(--info); font-size: 12px; }
.douyin-workbench[data-status='review'] .douyin-session-status { border-color: var(--active-border); background: var(--accent); color: var(--primary); }
.douyin-workbench[data-status='waiting'] .douyin-session-status, .douyin-workbench[data-status='stopped'] .douyin-session-status { color: var(--warning); }
.douyin-workbench[data-status='failed'] .douyin-session-status { color: var(--destructive); }
.douyin-steps { display: grid; grid-template-columns: repeat(6, minmax(0, 1fr)); gap: 8px; padding: 0; margin: 20px 0; list-style: none; }
.douyin-steps li { display: grid; align-content: start; justify-items: start; gap: 8px; min-width: 0; padding: 12px; border: 1px solid var(--border); border-radius: 10px; background: var(--card); font-size: 12px; line-height: 1.6; }
.douyin-steps li > span:last-child { color: var(--muted-foreground); }
.douyin-step-marker { display: grid; place-items: center; width: 26px; height: 26px; border: 1px solid var(--input-border); border-radius: 50%; color: var(--muted-foreground); }
.douyin-step-marker .app-icon { width: 15px; height: 15px; }
.douyin-steps li[data-state='done'] .douyin-step-marker { background: var(--accent); color: var(--primary); border-color: var(--active-border); }
.douyin-steps li[data-state='current'] { border-color: color-mix(in srgb, var(--info) 45%, var(--border)); background: color-mix(in srgb, var(--info) 5%, var(--card)); }
.douyin-steps li[data-state='current'] .douyin-step-marker { color: var(--info); border-color: var(--info); }
.douyin-workbench[data-status='review'] { animation: preparation-ready 220ms ease-out; }
@keyframes preparation-ready { from { opacity: .6; transform: translateY(4px); } to { opacity: 1; transform: translateY(0); } }
@media (max-width: 1200px) { .douyin-steps { grid-template-columns: repeat(3, minmax(0, 1fr)); } }
@media (max-width: 560px) { .douyin-steps { grid-template-columns: repeat(2, minmax(0, 1fr)); } }
@media (prefers-reduced-motion: reduce) { .douyin-workbench[data-status='review'] { animation: none; } }
</style>
