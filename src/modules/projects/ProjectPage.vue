<script setup lang="ts">
import { Badge } from '@/components/ui/badge'
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '@/components/ui/collapsible'
import { computed, onMounted, ref } from 'vue'
import { RouterLink, onBeforeRouteLeave, onBeforeRouteUpdate, useRouter } from 'vue-router'
import * as api from '../../services/projects'
import { appendNote, chooseNote, importNote, type ImportedNote } from '../../services/notes'
import { emptyProject, projectInput, type Project } from '../../types/project'
import { useOperation } from '../../composables/useOperation'
import AppIcon from '../../components/AppIcon.vue'
import ProjectNavigation from '../../components/ProjectNavigation.vue'
import { Button, buttonVariants } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Textarea } from '@/components/ui/textarea'
const props = defineProps<{ id?: string }>()
const router = useRouter()
const desktop = api.isDesktop()
const project = ref<Project | null>(null)
const form = ref(emptyProject())
const original = ref(JSON.stringify(form.value))
const dirty = computed(() => JSON.stringify(form.value) !== original.value)
const ready = ref(!props.id)
const deleting = ref(false)
const imported = ref<ImportedNote | null>(null)
let leaving = false
const { busy, error, message, run } = useOperation()
function accept(saved: Project) { project.value = saved; form.value = projectInput(saved); original.value = JSON.stringify(form.value); ready.value = true }
const load = () => run(async () => { if (props.id) accept(await api.getProject(props.id)) })
const chooseImport = () => run(async () => { const path = await chooseNote(); if (path) imported.value = await importNote(path) })
const applyImport = () => run(async () => {
  if (!imported.value) return
  form.value.sourceNotes = appendNote(form.value.sourceNotes, imported.value)
  imported.value = null; message.value = '资料已追加，保留来源说明。点击保存并继续写入书籍资料。'
})
const save = () => run(async () => {
  const saved = await api.saveProject(props.id ?? null, { ...form.value })
  accept(saved)
  message.value = '项目已保存。'
  leaving = true; await router.replace(`/projects/${saved.id}/writing`)
})
const remove = () => run(async () => {
  if (!project.value) return
  await api.deleteProject(project.value.id)
  original.value = JSON.stringify(form.value)
  leaving = true
  await router.push('/projects')
})
const canLeave = () => leaving || (!busy.value && (!dirty.value || globalThis.confirm('项目资料尚未保存，确定离开并放弃修改？')))
onBeforeRouteLeave(canLeave)
onBeforeRouteUpdate(canLeave)
onMounted(() => { if (desktop && props.id) void load() })
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
      <h1>{{ project ? '书籍资料' : '添加一本书' }}</h1><p class="muted">
        {{ project ? '补充你的阅读理解，让创作更有自己的味道。' : '从一个书名开始，把读过的好故事分享出去。' }}
      </p>
    </div>
    <Badge variant="secondary">
      <AppIcon name="book" />{{ project ? project.title : '一段新创作的开始' }}
    </Badge>
  </header>
  <ProjectNavigation
    v-if="project"
    :id="project.id"
  />
  <p
    v-if="!desktop"
    class="notice"
  >
    请在桌面应用中编辑书籍。
  </p>
  <p
    v-if="error"
    class="error"
    role="alert"
  >
    {{ error }}
  </p><p
    v-if="message"
    role="status"
  >
    {{ message }}
  </p>
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
    v-if="ready"
    class="book-form-layout"
  >
    <form
      class="panel"
      @submit.prevent="save"
    >
      <fieldset :disabled="!desktop || busy">
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
            <label for="author">作者 <span class="optional">· 选填</span></label><Input
              id="author"
              v-model="form.author"
              placeholder="同名书建议填写作者"
            />
          </div>
        </div>
        <label for="notes">这本书，你想聊什么？ <span class="optional">· 选填</span></label>
        <Textarea
          id="notes"
          v-model="form.sourceNotes"
          rows="9"
          class="min-h-52 resize-y leading-7"
          placeholder="粘贴书籍简介、读书笔记，或写下这一期想重点讲的内容。&#10;&#10;例如：想从人物的命运聊聊普通人的韧性，语言通俗一些，像朋友聊天。"
        />
        <p class="muted">
          不需要先准备大纲或讲稿，留空也可以继续。
        </p>
        <Button
          type="button"
          variant="outline"
          @click="chooseImport"
        >
          导入 TXT / Markdown 笔记
        </Button>
        <section
          v-if="imported"
          class="my-4 space-y-3 rounded-lg border p-4"
          aria-label="读书资料导入预览"
        >
          <p class="file-path">
            来源：{{ imported.sourceName }} · {{ imported.sourcePath }}
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
            </Button>
            <Button
              type="button"
              variant="outline"
              @click="imported = null"
            >
              取消导入
            </Button>
          </div>
        </section>
        <div class="actions wrap">
          <Button type="submit">
            <AppIcon name="check" />{{ busy ? '保存中…' : '保存并继续' }}
          </Button><RouterLink
            :to="project ? `/projects/${project.id}/writing` : '/projects'"
            :class="buttonVariants({ variant: 'outline' })"
          >
            取消
          </RouterLink><span
            v-if="dirty"
            class="save-status is-dirty"
          >有未保存的修改</span>
        </div>
      </fieldset>
    </form>
    <aside class="book-form-aside">
      <AppIcon name="sparkle" /><h2>好内容，始于你的视角</h2><p>一个打动你的片段、一个想讨论的问题，都能让生成的内容更贴近你的表达。</p><p>接下来，你可以一键准备发布文案、音频提示词与封面素材。</p>
    </aside>
  </div>
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
  <section
    v-if="deleting"
    class="panel confirmation"
    role="alertdialog"
    aria-labelledby="delete-title"
  >
    <h2 id="delete-title">
      确认删除这本书？
    </h2><p>此项目及生成历史将被删除。原始视频、素材文件和工作目录都会保留。</p>
    <div class="actions">
      <Button
        variant="destructive"
        :disabled="busy"
        @click="remove"
      >
        确认删除
      </Button><Button
        variant="outline"
        :disabled="busy"
        @click="deleting = false"
      >
        取消
      </Button>
    </div>
  </section>
</template>
