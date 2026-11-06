<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { RouterLink } from 'vue-router'
import { isDesktop } from '../services/projects'
import { automationRuns, automationError, refreshAutomation, renderAutomationCover, controlAutomation, stages, type AutomationRun } from '../services/automation'
import { useWindowCloseGuard } from '../composables/useWindowCloseGuard'
import { requestConfirmation } from '../composables/useConfirmation'
import { openDouyinWindow } from '../services/douyin'
import { Button } from '@/components/ui/button'

const active = computed(() => automationRuns.value.filter(run => !['completed', 'cancelled'].includes(run.status)))
const working = computed(() => active.value.some(run => run.status === 'running'))
const busy = ref(false)
const error = ref('')
useWindowCloseGuard(() => working.value, () => !working.value || requestConfirmation('自动制作仍在进行。退出会中断本地执行，重开后可核对并继续。确定退出？'))
let disposed = false
let timer: ReturnType<typeof globalThis.setTimeout> | undefined
async function poll() {
  if (disposed) return
  await refreshAutomation()
  if (disposed) return
  for (const run of active.value) void renderAutomationCover(run)
  timer = globalThis.setTimeout(() => { void poll() }, 1500)
}
async function act(run: AutomationRun, action: 'pause' | 'resume' | 'cancel' | 'retry') {
  if (action === 'retry' && !await requestConfirmation('再次执行当前失败阶段可能产生模型费用。已有成功结果保留，确认重试？')) return
  if (action === 'cancel' && !await requestConfirmation('停止流程后保留全部成果。已开始的音频、生成或上传可能仍会完成，停止不等于撤稿。确定停止？')) return
  busy.value = true; error.value = ''
  try { await controlAutomation(run.id, action); await refreshAutomation() }
  catch (cause) { error.value = String(cause) }
  finally { busy.value = false }
}
async function showPublishing() {
  try { await openDouyinWindow() } catch (cause) { error.value = String(cause) }
}
onMounted(() => { if (isDesktop()) void poll() })
onBeforeUnmount(() => { disposed = true; globalThis.clearTimeout(timer) })
</script>

<template>
  <aside
    v-if="active.length || automationError || error"
    class="mb-5 grid gap-3 rounded-xl border border-primary/30 bg-card p-4"
    aria-label="自动制作与发布"
  >
    <p
      v-if="automationError || error"
      role="alert"
    >
      {{ automationError || error }}
    </p>
    <article
      v-for="run in active"
      :key="run.id"
      class="grid gap-3"
    >
      <div class="flex flex-wrap items-center justify-between gap-2">
        <RouterLink
          :to="`/projects/${run.projectId}`"
          class="font-semibold underline"
        >
          {{ run.input.title }} · 自动制作
        </RouterLink>
        <span
          role="status"
          class="text-sm"
        >{{ run.status === 'awaiting_publish' ? '等待最终发布' : run.status === 'running' ? stages[run.stage] : run.status === 'paused' ? '已暂停' : '需要处理' }}</span>
      </div>
      <p class="text-sm text-muted-foreground">
        {{ run.error || '保持应用运行，可切换页面。AI背景、双封面和视频将自动制作；最后在抖音窗口点击发布。' }}
      </p>
      <div class="flex flex-wrap gap-2">
        <Button
          v-if="run.status === 'running'"
          variant="outline"
          :disabled="busy"
          @click="act(run, 'pause')"
        >
          暂停后续步骤
        </Button>
        <Button
          v-else-if="run.status !== 'awaiting_publish'"
          :disabled="busy"
          @click="act(run, 'resume')"
        >
          继续流程
        </Button>
        <Button
          v-if="['paused', 'waiting_user'].includes(run.status) && ['writing', 'landscape_image', 'portrait_image', 'video_prepare', 'render'].includes(run.stage)"
          variant="outline"
          :disabled="busy"
          @click="act(run, 'retry')"
        >
          重试当前阶段
        </Button>
        <Button
          v-if="run.publishTaskId"
          variant="outline"
          @click="showPublishing"
        >
          显示抖音窗口
        </Button>
        <RouterLink
          v-if="run.publishTaskId"
          :to="`/projects/${run.projectId}/publishing?task=${run.publishTaskId}`"
          class="self-center text-sm underline"
        >
          发布详情与结果记录
        </RouterLink>
        <RouterLink
          v-else-if="run.videoJobId"
          :to="`/projects/${run.projectId}/video?job=${run.videoJobId}`"
          class="self-center text-sm underline"
        >
          查看视频任务
        </RouterLink>
        <RouterLink
          v-else-if="run.audioJobId"
          :to="`/projects/${run.projectId}/writing?tab=audio&audioJob=${run.audioJobId}`"
          class="self-center text-sm underline"
        >
          查看音频任务
        </RouterLink>
        <Button
          variant="ghost"
          :disabled="busy"
          @click="act(run, 'cancel')"
        >
          停止流程
        </Button>
      </div>
    </article>
  </aside>
</template>
