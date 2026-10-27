<script setup lang="ts">
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '@/components/ui/collapsible'
import { Input } from '@/components/ui/input'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { Textarea } from '@/components/ui/textarea'
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { RouterLink, onBeforeRouteLeave, onBeforeRouteUpdate, useRoute, useRouter } from 'vue-router'
import * as api from '../../services/publishing'
import { getProject, isDesktop, listAssets } from '../../services/projects'
import { listVersions } from '../../services/ai'
import { listCovers } from '../../services/covers'
import { publishInput, publishStatuses, type PublishInput, type PublishQueueItem, type PublishTask } from '../../types/publishing'
import type { Asset } from '../../types/project'
import type { CoverVersion } from '../../types/cover'
import type { WritingVersion } from '../../types/ai'
import { useOperation } from '../../composables/useOperation'
import ProjectAssets from '../projects/ProjectAssets.vue'
import AppIcon from '../../components/AppIcon.vue'
import ProjectNavigation from '../../components/ProjectNavigation.vue'
import OperationFeedback from '../../components/OperationFeedback.vue'
import CoverThumbnail from '../../components/CoverThumbnail.vue'
import DouyinVerification from './DouyinVerification.vue'

const props = defineProps<{ id: string; taskId?: string }>()
const router = useRouter()
const route = useRoute()
const assistant = ref<InstanceType<typeof DouyinVerification> | null>(null)
const nextTasks = ref<PublishQueueItem[]>([])
const nextTaskId = ref<string | null>(null)
const desktop = isDesktop()
const title = ref('')
const loaded = ref(false)
const tasks = ref<PublishTask[]>([])
const selected = ref<PublishTask | null>(null)
const assets = ref<Asset[]>([])
const covers = ref<CoverVersion[]>([])
const writing = ref<WritingVersion[]>([])
const draft = ref<PublishInput>({ projectId: props.id, writingVersionId: null, title: '', description: '', tags: '', videoAssetId: null, audioAssetId: null, landscapeCoverId: null, portraitCoverId: null, titleConfirmed: false, coversConfirmed: false })
const original = ref(JSON.stringify(draft.value))
const dirty = computed(() => JSON.stringify(draft.value) !== original.value)
const published = computed(() => selected.value?.status === 'published')
const videos = computed(() => assets.value.filter(a => a.assetType === 'video'))
const audios = computed(() => assets.value.filter(a => a.assetType === 'audio'))
const landscape = computed(() => covers.value.filter(c => c.exportedPath && c.config.width === 1440 && c.config.height === 1080))
const portrait = computed(() => covers.value.filter(c => c.exportedPath && c.config.width === 1080 && c.config.height === 1440))
const video = computed(() => videos.value.find(a => a.id === draft.value.videoAssetId))
const audio = computed(() => audios.value.find(a => a.id === draft.value.audioAssetId))
const selectedLandscape = computed(() => landscape.value.find(c => c.id === draft.value.landscapeCoverId))
const selectedPortrait = computed(() => portrait.value.find(c => c.id === draft.value.portraitCoverId))
const recordOpen = ref(false)
const record = ref({ publishedAt: '', postUrl: '', notes: '' })
const { busy: operationBusy, error, message, run } = useOperation()
const douyinBusy = ref(false)
const busy = computed(() => operationBusy.value || douyinBusy.value)

const taskLabel = (task: PublishTask) => `${task.title || '未填写标题'} · ${publishStatuses[task.status]} · ${new Date(task.createdAt).toLocaleString()}`
const assetLabel = (asset: Asset) => `${asset.originalName}${asset.existsStatus !== 'available' ? '（文件不可用）' : ''}`

function defaults(): PublishInput {
  return { projectId: props.id, writingVersionId: null, title: '', description: '', tags: '', videoAssetId: videos.value.find(v => v.existsStatus === 'available')?.id ?? null, audioAssetId: audios.value.find(v => v.existsStatus === 'available')?.id ?? null, landscapeCoverId: landscape.value[0]?.id ?? null, portraitCoverId: portrait.value[0]?.id ?? null, titleConfirmed: false, coversConfirmed: false }
}
function selectWriting(id: string | null) {
  if (id === draft.value.writingVersionId) return
  const version = writing.value.find(v => v.id === id)
  if (version && (draft.value.title || draft.value.description) && !globalThis.confirm('使用此版本将替换当前发布标题和内容，确定继续？')) return
  draft.value.writingVersionId = id
  if (version) { draft.value.title = version.content.title; draft.value.description = version.content.description }
  draft.value.titleConfirmed = false
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
  loaded.value = false
  const [project, saved, files, exported, versions] = await Promise.all([getProject(props.id), api.listPublishTasks(props.id), listAssets(props.id), listCovers(props.id), listVersions(props.id, 'publishing')])
  const target = props.taskId ? saved.find(task => task.id === props.taskId) : saved[0] ?? null
  if (props.taskId && !target) throw new Error('此任务不属于当前书籍或已不存在，请返回发布任务列表。')
  title.value = project.title; tasks.value = saved; assets.value = files; covers.value = exported; writing.value = versions
  accept(target ?? null); loaded.value = true
})
async function loadTask() {
  await load()
  if (loaded.value && route.query.assist === '1') {
    // Consume the explicit next-task action before running, so refresh cannot replay it.
    const query = { ...route.query }
    delete query.assist
    await router.replace({ path: route.path, query })
    await nextTick()
    await assistant.value?.start()
  }
}
watch(() => props.taskId, () => { if (desktop) void loadTask() })
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
function switchTask(id: string | null) {
  if (canDiscard()) { accept(tasks.value.find(t => t.id === id) ?? null); recordOpen.value = false }
}
async function persist() {
  if (!selected.value || dirty.value) update(await api.savePublishTask(selected.value?.id ?? null, { ...draft.value }))
  return selected.value!.id
}
async function prepareDouyinTask() {
  const id = await persist()
  if (selected.value?.status !== 'ready') update(await api.preparePublishTask(id))
  return selected.value!
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
  nextTaskId.value = null; nextTasks.value = []
  void api.listPublishQueue().then(tasks => { if (recordOpen.value) nextTasks.value = tasks.filter(task => task.status === 'ready' && task.id !== selected.value?.id) }).catch(cause => { error.value = String(cause) })
}
async function recordSuccess(startNext = false) {
  const target = startNext ? nextTasks.value.find(task => task.id === nextTaskId.value) : null
  if (startNext && !target) return
  let recorded = false
  await run(async () => {
    if (!selected.value) return
    const saved = await api.recordPublishSuccess(selected.value.id, { ...record.value, publishedAt: new Date(record.value.publishedAt).toISOString(), confirmed: true })
    update(saved); recordOpen.value = false; message.value = '已记录发布成功。'; recorded = true
  })
  if (recorded && target) await router.push({ path: `/projects/${target.projectId}/publishing`, query: { task: target.id, assist: '1' } })
}
onMounted(() => { if (desktop) void loadTask() })
</script>

<template>
  <header class="page-heading">
    <div>
      <RouterLink
        class="back-link"
        to="/projects"
      >
        <AppIcon name="back" />我的书籍
      </RouterLink>
      <h1>准备好，分享你的作品</h1>
      <p class="muted">
        {{ title || '书籍' }} · 选好素材与文案，开始辅助发布，检查后亲自发布。
      </p>
    </div>
    <RouterLink
      to="/publishing"
      class="back-link"
    >
      {{ published ? '继续处理下一篇' : '发布任务列表' }}
    </RouterLink>
    <Badge variant="secondary">
      <AppIcon name="upload" />发布前的最后一步
    </Badge>
  </header>
  <ProjectNavigation :id="id" />
  <p
    v-if="!desktop"
    class="notice"
  >
    请在桌面应用中准备发布。
  </p>
  <OperationFeedback
    :error="error"
    :message="message"
    @dismiss="message = ''"
  />
  <Button
    v-if="!loaded"
    :disabled="busy || !desktop"
    type="button"
    @click="load"
  >
    {{ busy ? '读取中…' : '重新读取' }}
  </Button>
  <template v-if="loaded">
    <DouyinVerification
      v-if="!published"
      ref="assistant"
      :desktop="desktop"
      :task="selected"
      :disabled="operationBusy || recordOpen || published"
      :prepare-task="prepareDouyinTask"
      @working="douyinBusy = $event"
    >
      <template #summary>
        <p class="muted">
          {{ title }} · 成片：{{ video?.originalName || '尚未选择' }}
        </p>
        <div class="actions wrap">
          <CoverThumbnail
            v-if="selectedLandscape"
            :cover="selectedLandscape"
          />
          <CoverThumbnail
            v-if="selectedPortrait"
            :cover="selectedPortrait"
          />
        </div>
      </template>
    </DouyinVerification>
    <section class="panel">
      <div class="result-heading">
        <h2>本书发布任务</h2><Badge variant="secondary">
          {{ selected ? publishStatuses[selected.status] : '新任务' }}
        </Badge>
      </div>
      <label
        v-if="tasks.length"
        for="publish-task"
      >选择任务</label>
      <Select
        v-if="tasks.length"
        :model-value="selected?.id ?? null"
        :disabled="busy || recordOpen"
        @update:model-value="switchTask"
      >
        <SelectTrigger
          id="publish-task"
          class="w-full min-w-0"
        >
          <SelectValue>{{ selected ? taskLabel(selected) : '新任务' }}</SelectValue>
        </SelectTrigger>
        <SelectContent>
          <SelectItem :value="null">
            新任务
          </SelectItem>
          <SelectItem
            v-for="task in tasks"
            :key="task.id"
            :value="task.id"
          >
            {{ taskLabel(task) }}
          </SelectItem>
        </SelectContent>
      </Select>
      <div class="actions">
        <Button
          :disabled="busy || recordOpen"
          variant="outline"
          type="button"
          @click="newTask"
        >
          新建发布任务
        </Button><Button
          :disabled="busy || recordOpen"
          variant="outline"
          type="button"
          @click="refresh"
        >
          刷新素材与状态
        </Button>
      </div>
    </section>
    <Collapsible
      v-if="!published"
      class="history-panel"
      :unmount-on-hide="false"
    >
      <CollapsibleTrigger class="disclosure-trigger">
        关联视频 / 重新定位素材
      </CollapsibleTrigger>
      <CollapsibleContent>
        <fieldset :disabled="busy || recordOpen">
          <ProjectAssets
            :project-id="id"
            @changed="refresh"
          />
        </fieldset>
      </CollapsibleContent>
    </Collapsible>
    <form @submit.prevent="save">
      <fieldset
        class="publish-form-grid"
        :disabled="busy || published || recordOpen"
      >
        <section class="panel">
          <h2>发布文字</h2>
          <label for="publish-writing-version">文案来源版本</label>
          <Select
            :model-value="draft.writingVersionId"
            :disabled="busy || published || recordOpen"
            @update:model-value="selectWriting"
          >
            <SelectTrigger
              id="publish-writing-version"
              class="w-full min-w-0"
            >
              <SelectValue placeholder="请选择历史文案，或手动填写" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem :value="null">
                手动填写 / 旧任务未记录来源
              </SelectItem>
              <SelectItem
                v-for="version in writing"
                :key="version.id"
                :value="version.id"
              >
                v{{ version.versionNo }} · {{ version.content.title }} · {{ new Date(version.createdAt).toLocaleString() }}
              </SelectItem>
            </SelectContent>
          </Select>
          <p class="muted">
            选择后带入标题和内容，仍可编辑；保存任务后固定文字快照与来源版本。
          </p>
          <label for="publish-title">发布标题</label>
          <Input
            id="publish-title"
            v-model="draft.title"
            maxlength="30"
            @input="draft.titleConfirmed = false"
          />
          <label class="check-label"><Checkbox
            id="title-confirmed"
            :model-value="draft.titleConfirmed"
            :disabled="busy || published || recordOpen"
            @update:model-value="draft.titleConfirmed = $event === true"
          />我已确认发布标题</label>
          <label for="publish-description">发布内容</label>
          <Textarea
            id="publish-description"
            v-model="draft.description"
            rows="6"
            maxlength="1000"
          />
          <label for="publish-tags">标签（选填，按平台格式填写）</label>
          <Input
            id="publish-tags"
            v-model="draft.tags"
            maxlength="500"
            placeholder="#读书 #十分钟读懂一本书"
          />
        </section>
        <section class="panel">
          <h2>音频、成片与封面</h2>
          <label for="publish-audio">音频成品</label>
          <Select
            v-model="draft.audioAssetId"
            :disabled="busy || published || recordOpen"
          >
            <SelectTrigger
              id="publish-audio"
              class="w-full min-w-0"
            >
              <SelectValue>{{ audio ? assetLabel(audio) : '请选择用于制作的音频' }}</SelectValue>
            </SelectTrigger>
            <SelectContent>
              <SelectItem :value="null">
                请选择用于制作的音频
              </SelectItem>
              <SelectItem
                v-for="file in audios"
                :key="file.id"
                :value="file.id"
              >
                {{ assetLabel(file) }}
              </SelectItem>
            </SelectContent>
          </Select>
          <p
            v-if="audio"
            class="file-path"
          >
            {{ audio.filePath }}
          </p>
          <p
            v-if="audio && audio.existsStatus !== 'available'"
            class="error"
          >
            音频文件不可用，请展开上方素材管理重新定位。
          </p>
          <p
            v-if="!audios.length"
            class="muted"
          >
            先在 <RouterLink :to="`/projects/${id}/writing`">
              音频制作
            </RouterLink> 导入音频成品；音频仅用于制作记录，不会上传抖音。
          </p>
          <label for="publish-video">剪映导出的最终成片</label>
          <Select
            v-model="draft.videoAssetId"
            :disabled="busy || published || recordOpen"
          >
            <SelectTrigger
              id="publish-video"
              class="w-full min-w-0"
            >
              <SelectValue>{{ video ? assetLabel(video) : '请选择最终成片' }}</SelectValue>
            </SelectTrigger>
            <SelectContent>
              <SelectItem :value="null">
                请选择最终成片
              </SelectItem>
              <SelectItem
                v-for="file in videos"
                :key="file.id"
                :value="file.id"
              >
                {{ assetLabel(file) }}
              </SelectItem>
            </SelectContent>
          </Select>
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
            先展开上方“关联素材”，关联剪映导出的最终成片。发布检查会验证 MP4/MOV 文件、可读取画面和音轨。
          </p>
          <div class="form-grid">
            <div>
              <label for="publish-landscape">横版封面 · 4:3</label><Select
                v-model="draft.landscapeCoverId"
                :disabled="busy || published || recordOpen"
                @update:model-value="draft.coversConfirmed = false"
              >
                <SelectTrigger
                  id="publish-landscape"
                  class="w-full min-w-0"
                >
                  <SelectValue placeholder="请选择" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem :value="null">
                    请选择已导出封面
                  </SelectItem><SelectItem
                    v-for="cover in landscape"
                    :key="cover.id"
                    :value="cover.id"
                  >
                    v{{ cover.versionNo }} · {{ cover.config.title }}
                  </SelectItem>
                </SelectContent>
              </Select>
              <CoverThumbnail
                v-if="published && selectedLandscape"
                :cover="selectedLandscape"
              />
            </div>
            <div>
              <label for="publish-portrait">竖版封面 · 3:4</label><Select
                v-model="draft.portraitCoverId"
                :disabled="busy || published || recordOpen"
                @update:model-value="draft.coversConfirmed = false"
              >
                <SelectTrigger
                  id="publish-portrait"
                  class="w-full min-w-0"
                >
                  <SelectValue placeholder="请选择" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem :value="null">
                    请选择已导出封面
                  </SelectItem><SelectItem
                    v-for="cover in portrait"
                    :key="cover.id"
                    :value="cover.id"
                  >
                    v{{ cover.versionNo }} · {{ cover.config.title }}
                  </SelectItem>
                </SelectContent>
              </Select>
              <CoverThumbnail
                v-if="published && selectedPortrait"
                :cover="selectedPortrait"
              />
            </div>
          </div>
          <p
            v-if="!landscape.length || !portrait.length"
            class="muted"
          >
            请先在封面工坊一次导出横版和竖版两张封面。
          </p>
          <label class="check-label"><Checkbox
            id="covers-confirmed"
            :model-value="draft.coversConfirmed"
            :disabled="busy || published || recordOpen"
            @update:model-value="draft.coversConfirmed = $event === true"
          />我已确认所选两张封面</label>
        </section>
        <div
          v-if="!published"
          class="sticky-actions publish-save"
        >
          <Button
            type="submit"
            :disabled="!!selected && !dirty"
            variant="outline"
          >
            保存发布草稿
          </Button>
          <Button
            type="button"
            variant="outline"
            @click="prepare"
          >
            {{ busy ? '处理中…' : '生成完整任务包' }}
          </Button>
          <span class="muted">{{ dirty ? '有未保存修改，生成时会先保存' : '每次生成都保留已有任务包' }}</span>
        </div>
      </fieldset>
    </form>
    <section class="panel publish-actions">
      <h2>复制发布文字</h2>
      <div class="actions wrap">
        <Button
          :disabled="!draft.title"
          variant="outline"
          type="button"
          @click="copy('title')"
        >
          复制标题
        </Button><Button
          :disabled="!draft.description"
          variant="outline"
          type="button"
          @click="copy('description')"
        >
          复制内容
        </Button><Button
          :disabled="!draft.tags"
          variant="outline"
          type="button"
          @click="copy('tags')"
        >
          复制标签
        </Button><Button
          :disabled="!draft.title || !draft.description"
          variant="outline"
          type="button"
          @click="copy('all')"
        >
          一键复制全部
        </Button>
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
        <Button
          v-if="selected.taskDirectory"
          :disabled="busy"
          variant="outline"
          type="button"
          @click="open('directory')"
        >
          打开任务包
        </Button>
        <Button
          v-if="video"
          :disabled="busy || video.existsStatus !== 'available' || dirty"
          variant="outline"
          type="button"
          @click="open('video')"
        >
          打开视频
        </Button>
        <Button
          v-if="selected.taskDirectory"
          :disabled="busy"
          variant="outline"
          type="button"
          @click="open('landscape')"
        >
          查看横版封面
        </Button>
        <Button
          v-if="selected.taskDirectory"
          :disabled="busy"
          variant="outline"
          type="button"
          @click="open('portrait')"
        >
          查看竖版封面
        </Button>
        <Button
          :disabled="busy"
          variant="outline"
          type="button"
          @click="open('portal')"
        >
          打开抖音创作者中心
        </Button>
        <Button
          v-if="!published"
          :disabled="busy || dirty || recordOpen || selected.status !== 'ready'"
          type="button"
          @click="confirmPublish"
        >
          记录发布成功
        </Button>
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
      @submit.prevent="recordSuccess()"
    >
      <h2 id="publish-confirm-title">
        确认作品已在抖音发布成功？
      </h2>
      <p>请在平台看到作品发布成功后再确认。这里仅记录结果。</p>
      <fieldset :disabled="busy">
        <label for="published-at">实际发布时间</label><Input
          id="published-at"
          v-model="record.publishedAt"
          type="datetime-local"
          required
        />
        <label for="post-url">作品链接（选填）</label><Input
          id="post-url"
          v-model="record.postUrl"
          type="url"
          maxlength="2048"
          placeholder="https://…"
        />
        <label for="publish-notes">备注（选填）</label><Textarea
          id="publish-notes"
          v-model="record.notes"
          rows="3"
          maxlength="2000"
        />
        <div class="actions">
          <Button type="submit">
            确认已发布并保存
          </Button><Button
            type="button"
            variant="outline"
            @click="recordOpen = false"
          >
            取消
          </Button>
        </div>
        <template v-if="nextTasks.length">
          <label for="publish-next-task">接下来处理的作品（选填）</label>
          <Select
            v-model="nextTaskId"
            :disabled="busy"
          >
            <SelectTrigger id="publish-next-task">
              <SelectValue placeholder="请选择下一篇，不会自动选择" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem
                v-for="task in nextTasks"
                :key="task.id"
                :value="task.id"
              >
                {{ task.projectTitle }} · {{ task.title }}
              </SelectItem>
            </SelectContent>
          </Select>
          <Button
            type="button"
            :disabled="busy || !nextTaskId"
            @click="recordSuccess(true)"
          >
            确认记录并准备下一篇
          </Button>
        </template>
      </fieldset>
    </form>
  </template>
</template>

<style scoped>
.publish-form-grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 0 20px; }
.publish-save { grid-column: 1 / -1; justify-content: flex-start; margin-top: 0; }
.publish-save .muted { margin-left: auto; font-size: 11px; }
.publish-actions { margin-top: 24px; }
.publish-checklist { list-style: none; padding: 0; line-height: 2; font-size: 12px; display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 8px; }
.publish-checklist li { padding: 9px 12px; background: #f1f6ec; border-radius: 7px; }
.check-pending { color: #8c5d23; }
@media (max-width: 1000px) { .publish-form-grid { grid-template-columns: 1fr; } }
</style>
