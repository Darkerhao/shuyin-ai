<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { RouterLink } from 'vue-router'
import { Badge } from '@/components/ui/badge'
import { Button, buttonVariants } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import AppIcon from '../../components/AppIcon.vue'
import OperationFeedback from '../../components/OperationFeedback.vue'
import { useOperation } from '../../composables/useOperation'
import { isDesktop } from '../../services/projects'
import { listPublishQueue } from '../../services/publishing'
import { publishStatuses, type PublishQueueItem, type PublishTask } from '../../types/publishing'

const desktop = isDesktop()
const tasks = ref<PublishQueueItem[]>([])
const status = ref<PublishTask['status']>('ready')
const search = ref('')
const loaded = ref(false)
const { busy, error, message, run } = useOperation()
const filters = ['ready', 'draft', 'published'] as const
const visible = computed(() => {
  const query = search.value.trim().toLocaleLowerCase()
  return tasks.value.filter(task => task.status === status.value && [task.projectTitle, task.title].some(text => text.toLocaleLowerCase().includes(query)))
})
const load = () => run(async () => { tasks.value = await listPublishQueue(); loaded.value = true })
async function copyLink(link: string) {
  try { await globalThis.navigator.clipboard.writeText(link); error.value = ''; message.value = '作品链接已复制。' }
  catch { error.value = '复制失败，请选中作品链接后手动复制。' }
}
onMounted(() => { if (desktop) void load() })
</script>

<template>
  <header class="page-heading">
    <div>
      <RouterLink
        to="/projects"
        class="back-link"
      >
        <AppIcon name="back" />我的书籍
      </RouterLink>
      <h1>发布任务</h1>
      <p class="muted">
        每次处理一篇，检查后亲自发布，再记录结果与作品链接。
      </p>
    </div>
    <Button
      :disabled="!desktop || busy"
      variant="outline"
      type="button"
      @click="load"
    >
      <AppIcon
        name="refresh"
        :class="{ spin: busy }"
      />{{ busy ? '读取中…' : '刷新任务' }}
    </Button>
  </header>
  <p
    v-if="!desktop"
    class="notice"
    role="status"
  >
    请在桌面应用中查看发布任务。
  </p>
  <OperationFeedback
    :error="error"
    :message="message"
    @dismiss="message = ''"
  />
  <div class="mb-6 flex flex-wrap items-center justify-between gap-3 border-b pb-5">
    <div
      class="flex gap-1 rounded-lg bg-muted p-1"
      aria-label="发布任务筛选"
    >
      <Button
        v-for="filter in filters"
        :key="filter"
        variant="ghost"
        size="sm"
        type="button"
        :aria-pressed="status === filter"
        :class="status === filter ? 'bg-white shadow-xs hover:bg-white' : 'text-muted-foreground'"
        :disabled="!desktop"
        @click="status = filter"
      >
        {{ publishStatuses[filter] }} <span class="text-xs tabular-nums">{{ tasks.filter(task => task.status === filter).length }}</span>
      </Button>
    </div>
    <div class="w-64 max-w-full">
      <label
        for="publish-search"
        class="sr-only"
      >搜索书名或发布标题</label>
      <Input
        id="publish-search"
        v-model="search"
        placeholder="搜索书名或发布标题"
        :disabled="!desktop"
      />
    </div>
  </div>
  <p
    v-if="busy && !loaded"
    class="loading-state"
    role="status"
  >
    正在读取发布任务…
  </p>
  <template v-if="loaded">
    <p
      class="mb-4 text-xs text-muted-foreground"
      role="status"
    >
      共 {{ visible.length }} 项{{ publishStatuses[status] }}任务 · 按创建时间先后排列
    </p>
    <p
      v-if="!visible.length"
      class="notice"
    >
      {{ search ? '没有匹配的任务，请调整搜索。' : status === 'ready' ? '暂时没有待发布任务。可在草稿中补齐素材并生成任务包。' : status === 'draft' ? '暂时没有发布草稿。请从书籍的发布准备页新建任务。' : '暂时没有已确认的发布记录。' }}
    </p>
    <article
      v-for="task in visible"
      :key="task.id"
      class="panel"
    >
      <div class="flex flex-wrap items-start justify-between gap-4">
        <div class="min-w-0 flex-1">
          <p class="mb-2 text-sm text-muted-foreground">
            {{ task.projectTitle }}
          </p>
          <h2 class="break-words">
            {{ task.title || '未填写发布标题' }}
          </h2>
          <p class="muted">
            {{ task.record ? '发布于 ' + new Date(task.record.publishedAt).toLocaleString() : '创建于 ' + new Date(task.createdAt).toLocaleString() }}
          </p>
        </div>
        <Badge variant="secondary">
          {{ publishStatuses[task.status] }}
        </Badge>
      </div>
      <p
        v-if="task.status === 'draft'"
        class="muted"
      >
        待完成：{{ task.checklist.filter(item => !item.passed).map(item => item.label).join('；') || '重新生成有效任务包' }}
      </p>
      <template v-if="task.record">
        <p
          v-if="task.record.postUrl"
          class="file-path select-text"
        >
          {{ task.record.postUrl }}
        </p>
        <p
          v-else
          class="muted"
        >
          未填写作品链接。
        </p>
        <p
          v-if="task.record.notes"
          class="whitespace-pre-wrap text-sm"
        >
          {{ task.record.notes }}
        </p>
      </template>
      <div class="actions wrap">
        <RouterLink
          :to="{ path: '/projects/' + task.projectId + '/publishing', query: { task: task.id } }"
          :class="buttonVariants({ variant: 'outline' })"
        >
          {{ task.status === 'published' ? '查看发布记录' : task.status === 'ready' ? '处理这篇' : '完善发布准备' }}
        </RouterLink>
        <Button
          v-if="task.record?.postUrl"
          type="button"
          variant="outline"
          @click="copyLink(task.record.postUrl)"
        >
          复制作品链接
        </Button>
      </div>
    </article>
  </template>
</template>
