<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { RouterLink, onBeforeRouteLeave, onBeforeRouteUpdate } from 'vue-router'
import * as api from '../../services/publishing'
import { getProject, isDesktop, listAssets } from '../../services/projects'
import { listVersions } from '../../services/ai'
import { listCovers } from '../../services/covers'
import { publishInput, publishStatuses, type PublishInput, type PublishTask } from '../../types/publishing'
import type { Asset } from '../../types/project'
import type { CoverVersion } from '../../types/cover'
import type { WritingVersion } from '../../types/ai'
import { useOperation } from '../../composables/useOperation'
import ProjectAssets from '../projects/ProjectAssets.vue'

const props = defineProps<{ id: string }>()
const desktop = isDesktop()
const title = ref('')
const loaded = ref(false)
const tasks = ref<PublishTask[]>([])
const selected = ref<PublishTask | null>(null)
const assets = ref<Asset[]>([])
const covers = ref<CoverVersion[]>([])
const writing = ref<WritingVersion[]>([])
const draft = ref<PublishInput>({ projectId: props.id, title: '', description: '', tags: '', videoAssetId: null, landscapeCoverId: null, portraitCoverId: null, titleConfirmed: false, coversConfirmed: false })
const original = ref(JSON.stringify(draft.value))
const dirty = computed(() => JSON.stringify(draft.value) !== original.value)
const published = computed(() => selected.value?.status === 'published')
const videos = computed(() => assets.value.filter(a => a.assetType === 'video'))
const landscape = computed(() => covers.value.filter(c => c.exportedPath && c.config.width === 1440 && c.config.height === 1080))
const portrait = computed(() => covers.value.filter(c => c.exportedPath && c.config.width === 1080 && c.config.height === 1440))
const video = computed(() => videos.value.find(a => a.id === draft.value.videoAssetId))
const recordOpen = ref(false)
const record = ref({ publishedAt: '', postUrl: '', notes: '' })
const { busy, error, message, run } = useOperation()

function defaults(): PublishInput {
  const content = writing.value[0]?.content
  return { projectId: props.id, title: content?.format === 'publishing' ? content.title : '', description: content?.format === 'publishing' ? content.description : '', tags: '', videoAssetId: videos.value.find(v => v.existsStatus === 'available')?.id ?? null, landscapeCoverId: landscape.value[0]?.id ?? null, portraitCoverId: portrait.value[0]?.id ?? null, titleConfirmed: false, coversConfirmed: false }
}
function accept(task: PublishTask | null) {
  selected.value = task
  draft.value = task ? publishInput(task) : defaults()
  original.value = JSON.stringify(draft.value)
}
function update(task: PublishTask) {
  tasks.value = [task, ...tasks.value.filter(t => t.id !== task.id)]
  accept(task)
}
function canDiscard() { return (!dirty.value && !recordOpen.value) || globalThis.confirm('发布准备尚未保存，确定放弃修改？') }
const canLeave = () => !busy.value && canDiscard()
onBeforeRouteLeave(canLeave)
onBeforeRouteUpdate(canLeave)
function beforeUnload(event: globalThis.BeforeUnloadEvent) { if (dirty.value || busy.value || recordOpen.value) event.preventDefault() }
globalThis.addEventListener('beforeunload', beforeUnload)
onBeforeUnmount(() => globalThis.removeEventListener('beforeunload', beforeUnload))

const load = () => run(async () => {
  const [project, saved, files, exported, versions] = await Promise.all([getProject(props.id), api.listPublishTasks(props.id), listAssets(props.id), listCovers(props.id), listVersions(props.id, 'publishing')])
  title.value = project.title; tasks.value = saved; assets.value = files; covers.value = exported; writing.value = versions
  accept(saved[0] ?? null); loaded.value = true
})
const refresh = () => run(async () => {
  const [files, exported, saved] = await Promise.all([listAssets(props.id), listCovers(props.id), api.listPublishTasks(props.id)])
  assets.value = files; covers.value = exported; tasks.value = saved
  if (selected.value) {
    const current = saved.find(t => t.id === selected.value?.id)
    if (!current) { error.value = '此任务已不存在，请重新打开这本书。'; return }
    if (dirty.value) selected.value = current
    else accept(current)
  }
  message.value = '素材与任务状态已刷新。'
})
function newTask() { if (canDiscard()) { accept(null); recordOpen.value = false; error.value = ''; message.value = '' } }
function switchTask(event: globalThis.Event) {
  const control = event.target as globalThis.HTMLSelectElement
  if (canDiscard()) { accept(tasks.value.find(t => t.id === control.value) ?? null); recordOpen.value = false }
  else control.value = selected.value?.id ?? ''
}
async function persist() {
  if (!selected.value || dirty.value) update(await api.savePublishTask(selected.value?.id ?? null, { ...draft.value }))
  return selected.value!.id
}
const save = () => run(async () => { await persist(); message.value = '发布草稿已保存。' })
const prepare = () => run(async () => {
  const id = await persist()
  update(await api.preparePublishTask(id))
  message.value = '任务包已生成。请检查平台预览并手动发布，成功后回来记录结果。'
})
const open = (item: Parameters<typeof api.openPublishItem>[1]) => run(async () => {
  if (selected.value) await api.openPublishItem(selected.value.id, item)
})
async function copy(field: 'title' | 'description' | 'tags' | 'all') {
  const text = field === 'all' ? [draft.value.title, draft.value.description, draft.value.tags].filter(Boolean).join('\n\n') : draft.value[field]
  try { await globalThis.navigator.clipboard.writeText(text); error.value = ''; message.value = '已复制发布文字。' }
  catch { error.value = '复制失败，请选中文字后手动复制。' }
}
function confirmPublish() {
  const now = new Date()
  record.value = { publishedAt: new Date(now.getTime() - now.getTimezoneOffset() * 60000).toISOString().slice(0, 16), postUrl: '', notes: '' }
  recordOpen.value = true
}
const recordSuccess = () => run(async () => {
  if (!selected.value) return
  const saved = await api.recordPublishSuccess(selected.value.id, { ...record.value, publishedAt: new Date(record.value.publishedAt).toISOString(), confirmed: true })
  update(saved); recordOpen.value = false; message.value = '已记录发布成功。'
})
onMounted(() => { if (desktop) void load() })
</script>

<template>
  <header>
    <RouterLink to="/projects">
      ← 我的书籍
    </RouterLink>
    <h1>{{ title || '书籍' }} · 发布准备</h1>
    <p class="muted">
      选好视频、文案与两张封面，生成任务包后到抖音手动发布。
    </p>
    <RouterLink :to="`/projects/${id}/writing`">
      编辑发布文案
    </RouterLink>
    <RouterLink
      class="cover-entry"
      :to="`/projects/${id}/covers`"
    >
      制作封面
    </RouterLink>
  </header>
  <p
    v-if="!desktop"
    class="notice"
  >
    请在桌面应用中准备发布。
  </p>
  <p
    v-if="error"
    role="alert"
    class="error"
  >
    {{ error }}
  </p>
  <p
    v-if="message"
    role="status"
    class="notice"
  >
    {{ message }}
  </p>
  <button
    v-if="!loaded"
    :disabled="busy || !desktop"
    @click="load"
  >
    {{ busy ? '读取中…' : '重新读取' }}
  </button>
  <template v-if="loaded">
    <section class="panel">
      <div class="result-heading">
        <h2>本书发布任务</h2><span class="badge">{{ selected ? publishStatuses[selected.status] : '新任务' }}</span>
      </div>
      <label
        v-if="tasks.length"
        for="publish-task"
      >选择任务</label>
      <select
        v-if="tasks.length"
        id="publish-task"
        :value="selected?.id ?? ''"
        :disabled="busy || recordOpen"
        @change="switchTask"
      >
        <option value="">
          新任务
        </option>
        <option
          v-for="task in tasks"
          :key="task.id"
          :value="task.id"
        >
          {{ task.title || '未填写标题' }} · {{ publishStatuses[task.status] }} · {{ new Date(task.createdAt).toLocaleString() }}
        </option>
      </select>
      <div class="actions">
        <button
          class="secondary"
          :disabled="busy || recordOpen"
          @click="newTask"
        >
          新建发布任务
        </button><button
          class="secondary"
          :disabled="busy || recordOpen"
          @click="refresh"
        >
          刷新素材与状态
        </button>
      </div>
    </section>
    <details
      v-if="!published"
      class="history-panel"
    >
      <summary>关联视频 / 重新定位素材</summary>
      <fieldset :disabled="busy || recordOpen">
        <ProjectAssets
          :project-id="id"
          @changed="refresh"
        />
      </fieldset>
    </details>
    <form @submit.prevent="save">
      <fieldset :disabled="busy || published || recordOpen">
        <section class="panel">
          <h2>发布文字</h2>
          <label for="publish-title">发布标题</label>
          <input
            id="publish-title"
            v-model="draft.title"
            maxlength="30"
            @input="draft.titleConfirmed = false"
          >
          <label class="check-label"><input
            id="title-confirmed"
            v-model="draft.titleConfirmed"
            type="checkbox"
          >我已确认发布标题</label>
          <label for="publish-description">发布内容</label>
          <textarea
            id="publish-description"
            v-model="draft.description"
            rows="6"
            maxlength="1000"
          />
          <label for="publish-tags">标签（选填，按平台格式填写）</label>
          <input
            id="publish-tags"
            v-model="draft.tags"
            maxlength="500"
            placeholder="#读书 #十分钟读懂一本书"
          >
        </section>
        <section class="panel">
          <h2>视频与封面</h2>
          <label for="publish-video">本地视频</label>
          <select
            id="publish-video"
            v-model="draft.videoAssetId"
          >
            <option :value="null">
              请选择视频
            </option>
            <option
              v-for="file in videos"
              :key="file.id"
              :value="file.id"
            >
              {{ file.originalName }}{{ file.existsStatus !== 'available' ? '（文件不可用）' : '' }}
            </option>
          </select>
          <p
            v-if="video"
            class="file-path"
          >
            {{ video.filePath }}
          </p>
          <p
            v-if="video && video.existsStatus !== 'available'"
            class="error"
          >
            视频文件不可用，请展开上方素材管理重新定位。
          </p>
          <p
            v-if="!videos.length"
            class="muted"
          >
            先展开上方“关联视频 / 重新定位素材”，关联本地视频。
          </p>
          <div class="form-grid">
            <div>
              <label for="publish-landscape">横版封面 · 4:3</label><select
                id="publish-landscape"
                v-model="draft.landscapeCoverId"
                @change="draft.coversConfirmed = false"
              >
                <option :value="null">
                  请选择已导出封面
                </option><option
                  v-for="cover in landscape"
                  :key="cover.id"
                  :value="cover.id"
                >
                  v{{ cover.versionNo }} · {{ cover.config.title }}
                </option>
              </select>
            </div>
            <div>
              <label for="publish-portrait">竖版封面 · 3:4</label><select
                id="publish-portrait"
                v-model="draft.portraitCoverId"
                @change="draft.coversConfirmed = false"
              >
                <option :value="null">
                  请选择已导出封面
                </option><option
                  v-for="cover in portrait"
                  :key="cover.id"
                  :value="cover.id"
                >
                  v{{ cover.versionNo }} · {{ cover.config.title }}
                </option>
              </select>
            </div>
          </div>
          <p
            v-if="!landscape.length || !portrait.length"
            class="muted"
          >
            请先在封面工坊一次导出横版和竖版两张封面。
          </p>
          <label class="check-label"><input
            id="covers-confirmed"
            v-model="draft.coversConfirmed"
            type="checkbox"
          >我已确认所选两张封面</label>
        </section>
        <div
          v-if="!published"
          class="actions wrap"
        >
          <button
            type="submit"
            class="secondary"
            :disabled="!!selected && !dirty"
          >
            保存发布草稿
          </button>
          <button
            type="button"
            @click="prepare"
          >
            {{ busy ? '处理中…' : '生成完整任务包' }}
          </button>
          <span class="muted">{{ dirty ? '有未保存修改，生成时会先保存' : '每次生成都保留已有任务包' }}</span>
        </div>
      </fieldset>
    </form>
    <section class="panel publish-actions">
      <h2>复制发布文字</h2>
      <div class="actions wrap">
        <button
          class="secondary"
          :disabled="!draft.title"
          @click="copy('title')"
        >
          复制标题
        </button><button
          class="secondary"
          :disabled="!draft.description"
          @click="copy('description')"
        >
          复制内容
        </button><button
          class="secondary"
          :disabled="!draft.tags"
          @click="copy('tags')"
        >
          复制标签
        </button><button
          class="secondary"
          :disabled="!draft.title || !draft.description"
          @click="copy('all')"
        >
          一键复制全部
        </button>
      </div>
    </section>
    <section
      v-if="selected"
      class="panel"
    >
      <h2>已保存任务检查</h2>
      <p
        v-if="dirty"
        class="notice"
      >
        当前修改尚未保存，下列检查对应已保存的任务。
      </p>
      <ul class="publish-checklist">
        <li
          v-for="item in selected.checklist"
          :key="item.key"
          :class="{ 'check-pending': !item.passed }"
        >
          {{ item.passed ? '✓' : '○' }} {{ item.label }}
        </li>
      </ul>
      <p
        v-if="selected.taskDirectory"
        class="file-path"
      >
        {{ selected.taskDirectory }}
      </p>
      <div class="actions wrap">
        <button
          v-if="selected.taskDirectory"
          class="secondary"
          :disabled="busy"
          @click="open('directory')"
        >
          打开任务包
        </button>
        <button
          v-if="video"
          class="secondary"
          :disabled="busy || video.existsStatus !== 'available' || dirty"
          @click="open('video')"
        >
          打开视频
        </button>
        <button
          v-if="selected.taskDirectory"
          class="secondary"
          :disabled="busy"
          @click="open('landscape')"
        >
          查看横版封面
        </button>
        <button
          v-if="selected.taskDirectory"
          class="secondary"
          :disabled="busy"
          @click="open('portrait')"
        >
          查看竖版封面
        </button>
        <button
          class="secondary"
          :disabled="busy"
          @click="open('portal')"
        >
          打开抖音创作者中心
        </button>
        <button
          v-if="!published"
          :disabled="busy || dirty || recordOpen || selected.status !== 'ready'"
          @click="confirmPublish"
        >
          记录发布成功
        </button>
      </div>
      <p class="muted">
        任务包保留视频原路径，上传和最终发布由你在平台完成。打开入口或生成任务包不会标记已发布。
      </p>
      <div
        v-if="selected.record"
        class="notice"
      >
        <strong>已记录发布成功</strong><p>{{ new Date(selected.record.publishedAt).toLocaleString() }}</p><p
          v-if="selected.record.postUrl"
          class="file-path"
        >
          {{ selected.record.postUrl }}
        </p><p v-if="selected.record.notes">
          {{ selected.record.notes }}
        </p>
      </div>
    </section>
    <form
      v-if="recordOpen"
      class="panel confirmation"
      role="alertdialog"
      aria-labelledby="publish-confirm-title"
      @submit.prevent="recordSuccess"
    >
      <h2 id="publish-confirm-title">
        确认作品已在抖音发布成功？
      </h2>
      <p>请在平台看到作品发布成功后再确认。这里仅记录结果。</p>
      <fieldset :disabled="busy">
        <label for="published-at">实际发布时间</label><input
          id="published-at"
          v-model="record.publishedAt"
          type="datetime-local"
          required
        >
        <label for="post-url">作品链接（选填）</label><input
          id="post-url"
          v-model="record.postUrl"
          type="url"
          maxlength="2048"
          placeholder="https://…"
        >
        <label for="publish-notes">备注（选填）</label><textarea
          id="publish-notes"
          v-model="record.notes"
          rows="3"
          maxlength="2000"
        />
        <div class="actions">
          <button type="submit">
            确认已发布并保存
          </button><button
            type="button"
            class="secondary"
            @click="recordOpen = false"
          >
            取消
          </button>
        </div>
      </fieldset>
    </form>
  </template>
</template>

<style scoped>
.publish-actions { margin-top: 24px; }
.publish-checklist { list-style: none; padding: 0; line-height: 2; font-size: 14px; }
.check-pending { color: #8c5d23; }
</style>
