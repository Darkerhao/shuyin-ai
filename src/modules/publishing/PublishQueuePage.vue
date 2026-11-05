<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { RouterLink } from 'vue-router'
import { Badge } from '@/components/ui/badge'
import { Button, buttonVariants } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import AppIcon from '../../components/AppIcon.vue'
import OperationFeedback from '../../components/OperationFeedback.vue'
import { useOperation } from '../../composables/useOperation'
import { useQueryState } from '../../composables/useQueryState'
import { isDesktop } from '../../services/projects'
import { listPublishQueue } from '../../services/publishing'
import { publishStatuses, type PublishQueueItem } from '../../types/publishing'

const desktop = isDesktop()
const tasks = ref<PublishQueueItem[]>([])
const status = useQueryState('status', 'ready', ['ready', 'draft', 'published'])
const search = useQueryState('q', '')
const loaded = ref(false)
const { busy, error, message, run } = useOperation()
const filters = ['ready', 'draft', 'published'] as const
const visible = computed(() => {
  const query = search.value.trim().toLocaleLowerCase()
  return tasks.value.filter(task => task.status === status.value && [task.projectTitle, task.title].some(text => text.toLocaleLowerCase().includes(query)))
})
const load = () => run(async () => { tasks.value = await listPublishQueue(); loaded.value = true })
async function copyLink(link: string) {
  error.value = ''; message.value = ''
  try { await globalThis.navigator.clipboard.writeText(link); error.value = ''; message.value = '作品链接已复制。' }
  catch { error.value = '复制失败，请选中作品链接后手动复制。' }
}
onMounted(() => { if (desktop) void load() })
</script>

<template>
  <div class="publish-queue-page">
    <div class="page-return-row">
      <RouterLink
        to="/projects"
        class="back-link"
      >
        <AppIcon name="back" />返回我的书籍
      </RouterLink>
      <span class="queue-reminder"><AppIcon name="shield" />检查后亲自发布</span>
    </div>
    <header class="page-heading">
      <div>
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
    <div class="queue-toolbar">
      <div
        class="queue-filters"
        aria-label="发布任务筛选"
      >
        <Button
          v-for="filter in filters"
          :key="filter"
          variant="ghost"
          size="sm"
          type="button"
          :aria-pressed="status === filter"
          :data-status="filter"
          :disabled="!desktop"
          @click="status = filter"
        >
          {{ publishStatuses[filter] }} <span class="text-xs tabular-nums">{{ tasks.filter(task => task.status === filter).length }}</span>
        </Button>
      </div>
      <div class="relative w-64 max-w-full">
        <label
          for="publish-search"
          class="sr-only"
        >搜索书名或发布标题</label>
        <Input
          id="publish-search"
          v-model="search"
          name="search"
          type="search"
          autocomplete="off"
          class="pl-9 pr-9"
          placeholder="搜索书名或发布标题"
          :disabled="!desktop"
          @keydown.esc="search = ''"
        />
        <AppIcon
          name="search"
          class="pointer-events-none absolute left-3 top-2.5 size-4 text-muted-foreground"
        />
        <Button
          v-if="search"
          type="button"
          variant="ghost"
          size="icon-sm"
          class="absolute right-0.5 top-0.5"
          aria-label="清空任务搜索"
          @click="search = ''"
        >
          <AppIcon name="close" />
        </Button>
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
      <section
        v-if="!visible.length"
        class="panel empty-state"
      >
        <AppIcon :name="search ? 'search' : 'upload'" />
        <h2>{{ search ? '没有匹配的任务' : status === 'ready' ? '作品就绪后，在这里开始发布' : status === 'draft' ? '还没有发布草稿' : '还没有发布记录' }}</h2>
        <p class="muted">
          {{ search ? '试试其他书名或发布标题。' : status === 'ready' ? '在草稿中补齐音频、成片和封面，生成任务包后继续。' : status === 'draft' ? '打开一本书，进入发布准备，选择文案与素材即可新建任务。' : '在官方页面完成发布后，回到发布准备页确认并记录结果。' }}
        </p>
        <div class="actions justify-center">
          <Button
            v-if="search"
            variant="outline"
            @click="search = ''"
          >
            清空搜索
          </Button>
          <Button
            v-else-if="status === 'ready'"
            variant="outline"
            @click="status = 'draft'"
          >
            查看发布草稿
          </Button>
          <RouterLink
            v-else
            to="/projects"
            :class="buttonVariants({ variant: 'outline' })"
          >
            前往我的书籍
          </RouterLink>
        </div>
      </section>
      <article
        v-for="task in visible"
        :key="task.id"
        class="panel queue-task"
        :data-status="task.status"
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
          class="queue-pending"
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
        <div class="actions queue-task-actions">
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
  </div>
</template>

<style scoped>
.queue-reminder { display: flex; align-items: center; gap: 6px; color: var(--info); font: 400 12px/16px var(--font-mono); }
.queue-toolbar { display: flex; align-items: center; justify-content: space-between; flex-wrap: wrap; gap: 16px; margin-bottom: 24px; padding: 12px; border: 1px solid var(--border); border-radius: 16px; background: var(--panel-surface); backdrop-filter: blur(16px); }
.queue-filters { display: flex; flex-wrap: wrap; gap: 4px; }
.queue-filters button { border: 1px solid transparent; border-radius: 12px; font-size: 12px; color: var(--muted-foreground); }
.queue-filters button[aria-pressed='true'] { color: var(--info); background: color-mix(in srgb, var(--info) 10%, transparent); border-color: color-mix(in srgb, var(--info) 30%, transparent); box-shadow: var(--info-glow); }
.queue-filters button[data-status='draft'][aria-pressed='true'] { color: var(--warning); background: var(--warning-surface); border-color: var(--cover-border); box-shadow: none; }
.queue-filters button[data-status='published'][aria-pressed='true'] { color: var(--primary); background: var(--accent); border-color: var(--active-border); box-shadow: var(--primary-glow); }
.queue-task { padding: 24px; margin-bottom: 16px; box-shadow: var(--panel-shadow); transition: border-color 180ms; }
.queue-task:hover { border-color: color-mix(in srgb, var(--info) 40%, transparent); }
.queue-task[data-status='draft']:hover { border-color: var(--cover-border); }
.queue-task[data-status='published']:hover { border-color: var(--active-border); }
.queue-task h2 { margin: 0; color: var(--heading); font-size: 20px; line-height: 28px; letter-spacing: .3px; }
.queue-task p { font-family: var(--font-mono); font-size: 12px; }
.queue-task > div:first-child > div > p:first-child { margin: 0 0 8px; }
.queue-task :deep([data-slot='badge']) { flex-shrink: 0; padding: 4px 12px; border: 1px solid color-mix(in srgb, var(--info) 40%, transparent); border-radius: 999px; color: var(--info); background: color-mix(in srgb, var(--info) 15%, transparent); }
.queue-task[data-status='draft'] :deep([data-slot='badge']) { color: var(--warning); background: var(--warning-surface); border-color: var(--cover-border); }
.queue-task[data-status='published'] :deep([data-slot='badge']) { color: var(--primary); background: var(--accent); border-color: var(--active-border); }
.queue-pending { padding: 12px; border-radius: 12px; border: 1px solid var(--cover-border); color: var(--warning); background: var(--warning-surface); }
.queue-task-actions { justify-content: flex-end; padding-top: 16px; border-top: 1px solid var(--border); margin-top: 16px; }
.queue-task-actions a { color: var(--info); border-color: color-mix(in srgb, var(--info) 40%, transparent); }
.queue-task[data-status='draft'] .queue-task-actions a { color: var(--warning); border-color: var(--cover-border); }
@media (prefers-reduced-transparency: reduce) { .queue-toolbar { backdrop-filter: none; background: var(--card); } }
@media (max-width: 600px) { .queue-reminder { font-size: 11px; } .queue-toolbar > div:last-child { width: 100%; } .queue-task { padding: 16px; } .queue-task h2 { font-size: 18px; line-height: 26px; } .queue-task-actions { justify-content: flex-start; } }
</style>
