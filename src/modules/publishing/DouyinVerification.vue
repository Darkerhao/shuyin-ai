<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { completeDouyinTask, inspectDouyinWindow, onDouyinProgress, openDouyinWindow, stopDouyinTask, uploadDouyinVideo, type DouyinPage } from '../../services/douyin'
import type { PublishTask } from '../../types/publishing'
import { useOperation } from '../../composables/useOperation'
import OperationFeedback from '../../components/OperationFeedback.vue'

const props = defineProps<{ desktop: boolean; task?: PublishTask | null; disabled?: boolean }>()
const emit = defineEmits<{ working: [active: boolean] }>()
const accountConfirmed = ref(false)
const existingVideoConfirmed = ref(false)
const page = ref<DouyinPage | null>(null)
const { busy, error, message, run } = useOperation()
watch(busy, active => emit('working', active))
watch(() => props.task, () => { accountConfirmed.value = false; existingVideoConfirmed.value = false; page.value = null })
let dispose: (() => void) | undefined
let unmounted = false
onMounted(async () => {
  if (!props.desktop) return
  try {
    const unlisten = await onDouyinProgress(progress => { if (progress.taskId === props.task?.id) message.value = progress.message })
    if (unmounted) unlisten()
    else dispose = unlisten
  } catch { error.value = '无法读取填写进度，请重新打开页面。' }
})
onBeforeUnmount(() => { unmounted = true; dispose?.(); if (busy.value) void stopDouyinTask(); emit('working', false) })
async function open() {
  try {
    await openDouyinWindow()
    if (!busy.value) message.value = '请在抖音窗口扫码登录，然后返回检查页面。'
  } catch (cause) { error.value = String(cause) }
}
const inspect = () => run(async () => {
  accountConfirmed.value = false
  existingVideoConfirmed.value = false
  page.value = null
  page.value = await inspectDouyinWindow()
  message.value = !page.value.open ? '发布窗口已关闭。'
    : !page.value.online ? '当前网络不可用，请检查连接。'
      : page.value.loginVisible ? '等待你在抖音窗口完成登录。'
        : '当前未显示登录表单，请在官方页面核对账号；这不代表已通过登录或上传验收。'
})
const complete = () => run(async () => {
  if (!props.task || props.disabled || !existingVideoConfirmed.value) return
  const editor = await completeDouyinTask(props.task.id, existingVideoConfirmed.value)
  if (page.value) page.value.editor = editor
  message.value = '文案和双封面已回读核对。请在抖音网页检查声明、预览和发布设置，亲自点击发布；成功后回来记录结果。'
})
async function stop() {
  try { await stopDouyinTask(); message.value = '已停止后续填写。已发起的网页上传可能仍继续，请在官方页面检查并接管。' }
  catch { error.value = '停止失败，请关闭抖音窗口并检查页面。' }
}
const upload = () => run(async () => {
  if (!props.task || props.disabled || !accountConfirmed.value) return
  await uploadDouyinVideo(props.task.id, accountConfirmed.value)
  page.value = await inspectDouyinWindow()
  message.value = '视频、文案和双封面已交给官方页面并回读核对。这不代表已发布，请在抖音网页检查并亲自发布，成功后回来记录结果。'
})
</script>

<template>
  <section
    class="panel"
    aria-labelledby="douyin-verification-heading"
  >
    <h2 id="douyin-verification-heading">
      应用内抖音辅助发布
    </h2>
    <p class="muted">
      打开官方窗口并登录，核对账号后自动上传当前任务的视频、填写文案和双封面。请检查预览并亲自发布。
    </p>
    <div class="actions wrap">
      <Button
        :disabled="!desktop"
        type="button"
        @click="open"
      >
        打开 / 显示抖音窗口
      </Button>
      <Button
        :disabled="!desktop || busy"
        type="button"
        variant="outline"
        @click="inspect"
      >
        检查抖音页面
      </Button>
    </div>
    <template v-if="task && task.status === 'ready' && page?.supported !== false">
      <label
        class="check-label"
        for="douyin-account-confirmed"
      >
        <Checkbox
          id="douyin-account-confirmed"
          v-model="accountConfirmed"
          :disabled="disabled || busy"
        />
        已在抖音窗口核对账号，允许上传当前任务的素材
      </label>
      <Button
        type="button"
        :disabled="disabled || busy || !accountConfirmed || !page?.open || page.loginVisible || !page.online || page.path !== '/creator-micro/content/upload'"
        @click="upload"
      >
        上传并自动填写当前任务
      </Button>
      <template v-if="page?.editor">
        <label
          class="check-label"
          for="douyin-video-confirmed"
        >
          <Checkbox
            id="douyin-video-confirmed"
            v-model="existingVideoConfirmed"
            :disabled="disabled || busy"
          />
          已核对官方网页的账号和视频与当前任务一致，视频处理完成
        </label>
        <Button
          type="button"
          :disabled="disabled || busy || !existingVideoConfirmed || !page.online || page.loginVisible || page.editor.coverDialog || page.editor.covers.some(cover => cover.selected)"
          @click="complete"
        >
          继续填写文案和双封面
        </Button>
        <p class="muted">
          网页已有不同文案或封面时停止，保留现有内容供你处理。
        </p>
      </template>
    </template>
    <Button
      v-if="busy"
      type="button"
      variant="outline"
      @click="stop"
    >
      停止自动填写 / 人工接管
    </Button>
    <OperationFeedback
      :error="error"
      :message="message"
      @dismiss="message = ''"
    />
    <p
      v-if="page?.open"
      class="muted"
    >
      当前页面：{{ page.path }}
    </p>
  </section>
</template>
