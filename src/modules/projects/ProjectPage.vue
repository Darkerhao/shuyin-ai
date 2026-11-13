<script setup lang="ts">
import { displayPath } from '@/lib/displayPath'
import OperationFeedback from '../../components/OperationFeedback.vue'
import ConfirmAction from '../../components/ConfirmAction.vue'
import { useEditorDraft } from '../../composables/useEditorDraft'
import { readDraft } from '../../services/drafts'
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '@/components/ui/collapsible'
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { RouterLink, onBeforeRouteLeave, onBeforeRouteUpdate, useRouter, useRoute } from 'vue-router'
import * as api from '../../services/projects'
import { appendNote, chooseNote, importNote, type ImportedNote } from '../../services/notes'
import { emptyProject, projectInput, type Asset, type Project, type ProjectInput } from '../../types/project'
import { useOperation } from '../../composables/useOperation'
import AppIcon from '../../components/AppIcon.vue'
import ProjectNavigation from '../../components/ProjectNavigation.vue'
import AutomationSetup from '../../components/AutomationSetup.vue'
import ProjectChapters from './ProjectChapters.vue'
import { chapterIdFromQuery, chapterQuery, type Chapter } from '../../types/chapter'
import { Button, buttonVariants } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Textarea } from '@/components/ui/textarea'
const props = defineProps<{ id?: string }>()
const router = useRouter()
const route = useRoute()
const desktop = api.isDesktop()
const project = ref<Project | null>(null)
const form = ref(emptyProject())
const original = ref(JSON.stringify(form.value))
const sourcePaths = ref<string[]>([])
const sourceDropZone = ref<globalThis.HTMLElement | null>(null)
const sourceDragOver = ref(false)
const sourceError = ref('')
let unlistenSourceDrop: (() => void) | undefined
let disposed = false
if (!props.id && typeof route.query.source === 'string' && /\.(txt|md|epub|pdf|docx)$/i.test(route.query.source)) {
  sourcePaths.value = [route.query.source]
  form.value.title = route.query.source.split(/[\\/]/).pop()!.replace(/\.[^.]+$/, '')
}
const documents = ref<Asset[]>([])
const dirty = computed(() => JSON.stringify(form.value) !== original.value || sourcePaths.value.length > 0)
const ready = ref(false)
const automationBusy = ref(false)
const chapterEditor = ref<InstanceType<typeof ProjectChapters>>()
const pendingChapter = ref('')
const chapterId = computed(() => chapterIdFromQuery(route.query.chapter))
function chapterSaved(chapter: Chapter) {
  if (automationBusy.value) { pendingChapter.value = chapter.id; return }
  void router.replace({ path: `/projects/${chapter.projectId}`, query: chapterQuery(chapter.id) })
}
watch(automationBusy, value => {
  if (value || !pendingChapter.value || !project.value) return
  const id = pendingChapter.value; pendingChapter.value = ''
  void router.replace({ path: `/projects/${project.value.id}`, query: chapterQuery(id) })
})
const deleting = ref(false)
const imported = ref<ImportedNote | null>(null)
let leaving = false
const { busy, error, message, operation, run } = useOperation()
const draftContent = computed(() => ({ form: form.value, sourcePaths: sourcePaths.value }))
const autosave = useEditorDraft(() => project.value?.id ?? props.id ?? 'new', 'project', draftContent, () => project.value?.updatedAt ?? null, dirty, ready, () => !busy.value)
function accept(saved: Project) { project.value = saved; form.value = projectInput(saved); original.value = JSON.stringify(form.value); ready.value = true }
const load = () => run(async () => {
  ready.value = false
  if (props.id) {
    const [saved, assets] = await Promise.all([api.getProject(props.id), api.listAssets(props.id)])
    documents.value = assets.filter(asset => asset.assetType === 'document')
    accept(saved)
    ready.value = false
  }
  const recovery = await readDraft<{ form: ProjectInput; sourcePaths: string[] }>(props.id ?? 'new', 'project')
  if (recovery) {
    const dropped = sourcePaths.value
    const { title, author, sourceNotes } = recovery.content.form
    form.value = { ...form.value, title, author, sourceNotes }
    sourcePaths.value = [...new Set([...recovery.content.sourcePaths, ...dropped])]
    message.value = '已恢复书籍资料暂存。保存并继续后，资料与所选文件才正式生效。'
  }
  ready.value = true
}, '正在读取书籍资料…')
const sourceName = (path: string) => path.split(/[\\/]/).pop() ?? path
function stageSource(paths: string[]) {
  sourceError.value = ''
  if (paths.length !== 1) { sourceError.value = '请每次添加一个书籍文件。'; return }
  const path = paths[0]!
  if (!/\.(txt|md|epub|pdf|docx)$/i.test(path)) {
    sourceError.value = '暂不支持此格式，请选择 EPUB、PDF、TXT、Markdown 或 DOCX 文件。'
    return
  }
  if (sourcePaths.value.includes(path) || documents.value.some(asset => asset.filePath === path)) {
    message.value = '这份文件已在本书资料中。'
    return
  }
  sourcePaths.value.push(path)
  if (!form.value.title.trim()) form.value.title = sourceName(path).replace(/\.[^.]+$/, '')
  message.value = '已选择本地文件，保存并继续后可在音频制作中直接使用。'
}
const chooseSource = () => run(async () => {
  const path = await api.chooseSourceDocument()
  if (path) stageSource([path])
}, '正在选择书籍文件…')
const chooseImport = () => run(async () => { const path = await chooseNote(); if (path) imported.value = await importNote(path) }, '正在读取导入资料…')
const applyImport = () => run(async () => {
  if (!imported.value) return
  form.value.sourceNotes = appendNote(form.value.sourceNotes, imported.value)
  imported.value = null; message.value = '资料已追加，保留来源说明。点击保存并继续写入书籍资料。'
}, '正在追加资料…')
const save = () => run(async () => {
  await autosave.flush()
  const saved = await api.saveProject(project.value?.id ?? props.id ?? null, { ...form.value }, [...sourcePaths.value])
  autosave.reset()
  sourcePaths.value = []
  accept(saved)
  message.value = '项目已保存。'
  leaving = true; await router.replace(`/projects/${saved.id}/writing`)
}, '正在保存书籍资料…')
async function saveAutomaticSource() {
  if (project.value && route.query.chapter) {
    if (!chapterEditor.value) throw new Error('请先输入章节内容。')
    const chapter = await chapterEditor.value.save()
    return { projectId: project.value.id, source: { kind: 'chapter' as const, chapterId: chapter.id } }
  }
  if (!form.value.title.trim() || !form.value.author.trim()) throw new Error('自动制作请填写书名和作者。')
  if (documents.value.length + sourcePaths.value.length !== 1) throw new Error('自动制作每次需要且仅使用一个原书文件。请保留一份PDF、EPUB、TXT或Markdown。')
  await autosave.flush()
  const saved = await api.saveProject(project.value?.id ?? props.id ?? null, { ...form.value }, [...sourcePaths.value])
  autosave.reset(); sourcePaths.value = []; accept(saved)
  documents.value = (await api.listAssets(saved.id)).filter(asset => asset.assetType === 'document')
  const source = documents.value[0]
  if (!source || source.existsStatus !== 'available') throw new Error('原书文件不可读取。')
  return { projectId: saved.id, source: { kind: 'asset' as const, assetId: source.id } }
}
const remove = () => run(async () => {
  if (!project.value) return
  await autosave.flush()
  ready.value = false
  try { await api.deleteProject(project.value.id) }
  catch (cause) { ready.value = true; throw cause }
  original.value = JSON.stringify(form.value)
  leaving = true
  await router.push('/projects')
}, '正在删除项目…')
const canLeave = async () => {
  if (leaving) return true
  if (busy.value || automationBusy.value) return false
  try { await autosave.flush(); return true } catch { return false }
}
onBeforeRouteLeave(canLeave)
onBeforeRouteUpdate(canLeave)
onMounted(async () => {
  if (desktop) await load()
  if (disposed) return
  if (route.query.focus === 'title') globalThis.document.getElementById('title')?.focus()
  if (!desktop) return
  try {
    const unlisten = await getCurrentWindow().onDragDropEvent(({ payload }) => {
      sourceDragOver.value = false
      if (disposed || !ready.value || busy.value || automationBusy.value || payload.type === 'leave') return
      const bounds = sourceDropZone.value?.getBoundingClientRect()
      if (!bounds) return
      const position = payload.position.toLogical(globalThis.devicePixelRatio)
      const inside = position.x >= bounds.left && position.x <= bounds.right && position.y >= bounds.top && position.y <= bounds.bottom
      if (!inside) return
      if (payload.type === 'drop') stageSource(payload.paths)
      else sourceDragOver.value = true
    })
    if (disposed) unlisten()
    else unlistenSourceDrop = unlisten
  } catch { sourceError.value = '拖拽暂不可用，请点击「选择本地文件」添加。' }
})
onBeforeUnmount(() => { disposed = true; unlistenSourceDrop?.() })
</script>
<template>
  <div class="page-return-row">
    <RouterLink
      class="back-link"
      to="/projects"
    >
      <AppIcon name="back" />返回我的书籍
    </RouterLink>
    <span class="text-xs font-mono text-muted-foreground"><i class="status-dot mr-2" />当前目标：{{ project?.title || '一本新书' }}</span>
  </div>
  <header class="page-heading">
    <div>
      <h1>{{ project ? route.query.chapter ? '章节资料' : '书籍资料' : '添加一本书' }}</h1>
      <p class="muted">
        {{ project ? '补充你的阅读理解，让 AI 创作成片更有专属的思考味道。' : '从一个书名开始，把读过的好故事分享出去。' }}
      </p>
    </div>
  </header>
  <ProjectNavigation
    v-if="project"
    :id="project.id"
    :editing="dirty"
    :busy="busy || automationBusy"
  />
  <Button
    v-if="project && !route.query.chapter"
    variant="outline"
    class="mb-3"
    :disabled="busy || automationBusy"
    @click="router.push({ path: `/projects/${project!.id}`, query: { chapter: 'new' } })"
  >
    输入一个章节的内容
  </Button>
  <ProjectChapters
    v-if="project && route.query.chapter"
    ref="chapterEditor"
    :project-id="project.id"
    :chapter-id="chapterId"
    :disabled="automationBusy"
    @saved="chapterSaved"
  />
  <p
    v-if="!desktop"
    class="notice"
  >
    请在桌面应用中编辑书籍。
  </p>
  <OperationFeedback :error="autosave.error.value" />
  <OperationFeedback
    :error="error"
    :message="message"
  />
  <Button
    v-if="!ready && !busy"
    :disabled="!desktop"
    @click="load"
  >
    重新读取项目
  </Button>
  <p
    v-if="!ready && busy"
    class="loading-state"
    role="status"
  >
    正在读取书籍…
  </p>
  <div
    v-if="ready && !route.query.chapter"
    class="book-form-layout"
  >
    <form
      class="panel"
      @submit.prevent="save"
    >
      <fieldset :disabled="!desktop || busy || automationBusy">
        <div class="form-grid">
          <div>
            <label for="title">书名 *</label><Input
              id="title"
              v-model="form.title"
              required
              maxlength="200"
              placeholder="例如：生死疲劳"
            />
          </div>
          <div>
            <label for="author">作者 <span class="optional">(选填)</span></label><Input
              id="author"
              v-model="form.author"
              placeholder="同名书建议填写作者"
            />
          </div>
        </div>
        <section
          class="book-source-section"
          aria-labelledby="source-documents-title"
          aria-describedby="source-documents-help"
        >
          <div class="book-source-heading">
            <h2 id="source-documents-title">
              本地书籍文件
            </h2>
          </div>
          <div
            ref="sourceDropZone"
            class="book-source-dropzone"
            :class="{ 'is-drag-over': sourceDragOver && !busy && !automationBusy }"
          >
            <AppIcon
              name="upload"
              class="book-source-drop-icon"
            />
            <div class="book-source-drop-copy">
              <p
                class="book-source-drop-title"
                role="status"
              >
                {{ sourceDragOver && !busy && !automationBusy ? '松开鼠标，添加这本书' : '将书籍文件拖到这里' }}
              </p>
              <p>也可以点击按钮，从电脑中选择原书</p>
            </div>
            <Button
              type="button"
              class="book-source-select text-sm"
              aria-describedby="source-documents-help"
              @click="chooseSource"
            >
              <AppIcon name="folder" />选择本地文件
            </Button>
          </div>
          <p
            id="source-documents-help"
            class="book-source-help"
          >
            支持 EPUB、PDF、TXT、Markdown、DOCX，每次一个文件。仅关联本地路径，保存后可用于音频制作。
          </p>
          <p
            v-if="sourceError"
            class="error"
            role="alert"
          >
            {{ sourceError }}
          </p>
          <ul
            v-if="documents.length || sourcePaths.length"
            class="m-0 grid list-none gap-2 p-0"
            aria-label="本地书籍文件"
          >
            <li
              v-for="document in documents"
              :key="document.id"
              class="book-source-file"
            >
              <div class="flex items-center justify-between gap-2">
                <p class="book-source-name">
                  {{ document.originalName }}
                </p>
                <span class="book-source-status">{{ document.existsStatus === 'available' ? '已绑定生效' : '文件不可用' }}</span>
              </div>
              <p
                class="file-path"
                :title="displayPath(document.filePath)"
              >
                {{ displayPath(document.filePath) }}
              </p>
              <p
                v-if="document.existsStatus !== 'available'"
                class="error"
              >
                文件不可用，请到发布准备的素材管理中重新定位。
              </p>
            </li>
            <li
              v-for="(path, index) in sourcePaths"
              :key="path"
              class="book-source-file"
            >
              <div class="flex items-center justify-between gap-2">
                <p class="book-source-name">
                  {{ sourceName(path) }}
                </p>
                <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  class="h-6 px-2"
                  :aria-label="`取消绑定 ${sourceName(path)}`"
                  @click="sourcePaths.splice(index, 1)"
                >
                  取消绑定
                </Button>
              </div>
              <p class="file-path">
                {{ displayPath(path) }}
              </p><p class="book-source-status">
                待保存
              </p>
            </li>
          </ul>
          <RouterLink
            v-if="documents.length && project"
            :to="`/projects/${project.id}/publishing`"
            class="mt-2 inline-block text-[10px] font-mono text-muted-foreground"
          >
            管理已绑定文件
          </RouterLink>
        </section>
        <label for="notes">这本书，你想聊什么？<span class="optional">(选填灵感提示)</span></label>
        <Textarea
          id="notes"
          v-model="form.sourceNotes"
          rows="4"
          placeholder="粘贴书籍简介、读书笔记，或写下这一期想重点讲的内容。"
        />
        <section
          v-if="imported"
          class="my-4 space-y-3 rounded-lg border p-4"
          aria-label="读书资料导入预览"
        >
          <p class="file-path">
            来源：{{ imported.sourceName }} · {{ displayPath(imported.sourcePath) }}
          </p>
          <Textarea
            :model-value="imported.text"
            rows="8"
            readonly
            aria-label="导入资料预览"
          />
          <div class="actions wrap">
            <Button
              type="button"
              @click="applyImport"
            >
              采用并追加到资料
            </Button><Button
              type="button"
              variant="outline"
              @click="imported = null"
            >
              取消导入
            </Button>
          </div>
        </section>
        <div class="book-form-footer">
          <Button
            type="button"
            variant="outline"
            size="sm"
            class="font-mono"
            @click="chooseImport"
          >
            <AppIcon name="upload" />导入 TXT / Markdown 笔记
          </Button>
          <div class="actions wrap">
            <span
              v-if="dirty"
              class="save-status is-dirty"
              role="status"
            >{{ autosave.error.value || autosave.status.value || '等待暂存…' }}</span>
            <Button
              v-if="autosave.error.value"
              type="button"
              variant="outline"
              @click="run(() => autosave.flush())"
            >
              重试暂存
            </Button>
            <RouterLink
              :to="project ? `/projects/${project.id}/writing` : '/projects'"
              :class="buttonVariants({ variant: 'ghost', size: 'sm' })"
            >
              返回
            </RouterLink>
            <Button
              type="submit"
              size="sm"
            >
              {{ busy ? operation : '保存并继续' }}<AppIcon name="arrow" />
            </Button>
          </div>
        </div>
      </fieldset>
    </form>
    <aside class="book-form-aside">
      <h2><AppIcon name="bolt" />好内容，始于你的视角</h2>
      <p>一个打动你的片段、一个想讨论的问题，都能让 AI 生成的内容更贴近你的表达风格与人设立场。</p>
      <div class="book-form-guide">
        <strong>下一步创作建议：</strong><p>1. 准备发布文案</p><p>2. 制作读书音频</p><p>3. 准备多比例封面素材</p>
      </div>
      <RouterLink
        v-if="project"
        :to="`/projects/${project.id}/writing`"
        :class="[buttonVariants({ variant: 'outline' }), 'w-full font-mono text-info']"
      >
        快速进入内容制作
      </RouterLink>
    </aside>
  </div>
  <AutomationSetup
    :save-source="saveAutomaticSource"
    :disabled="busy"
    @busy="automationBusy = $event"
  />
  <Collapsible
    v-if="project"
    class="history-panel"
    :unmount-on-hide="false"
  >
    <CollapsibleTrigger class="disclosure-trigger">
      管理这本书
    </CollapsibleTrigger>
    <CollapsibleContent>
      <Button
        variant="outline"
        class="mt-4 text-destructive hover:text-destructive"
        :disabled="busy"
        @click="deleting = true"
      >
        删除项目
      </Button>
    </CollapsibleContent>
  </Collapsible>
  <ConfirmAction
    v-if="deleting"
    :title="`确认删除「${project?.title}」？`"
    :busy="busy"
    @confirm="remove"
    @cancel="deleting = false"
  >
    <p>此项目及生成历史将被删除。原始视频、素材文件和工作目录都会保留。</p>
  </ConfirmAction>
</template>
