<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { RouterLink, onBeforeRouteLeave, onBeforeRouteUpdate, useRouter } from 'vue-router'
import * as api from '../../services/projects'
import { emptyProject, projectInput, type Project } from '../../types/project'
import { useOperation } from '../../composables/useOperation'
const props = defineProps<{ id?: string }>()
const router = useRouter()
const desktop = api.isDesktop()
const project = ref<Project | null>(null)
const form = ref(emptyProject())
const original = ref(JSON.stringify(form.value))
const dirty = computed(() => JSON.stringify(form.value) !== original.value)
const ready = ref(!props.id)
const deleting = ref(false)
let leaving = false
const { busy, error, message, run } = useOperation()
function accept(saved: Project) { project.value = saved; form.value = projectInput(saved); original.value = JSON.stringify(form.value); ready.value = true }
const load = () => run(async () => { if (props.id) accept(await api.getProject(props.id)) })
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
  <header>
    <RouterLink to="/projects">
      ← 我的书籍
    </RouterLink>
    <h1>{{ project ? project.title : '添加一本书' }}</h1>
    <p class="muted">
      填书名即可开始。有简介或想重点讲的内容，也可以一起补充。
    </p>
    <RouterLink
      v-if="project"
      :to="`/projects/${project.id}/writing`"
    >
      查看发布文案与音频提示词
    </RouterLink>
    <RouterLink
      v-if="project"
      class="cover-entry"
      :to="`/projects/${project.id}/covers`"
    >
      制作封面
    </RouterLink>
  </header>
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
  </p>
  <p
    v-if="message"
    role="status"
  >
    {{ message }}
  </p>
  <button
    v-if="!ready && !busy"
    :disabled="!desktop"
    @click="load"
  >
    重新读取项目
  </button>
  <p
    v-if="!ready && busy"
    role="status"
  >
    正在读取书籍…
  </p>
  <form
    v-if="ready"
    class="panel"
    @submit.prevent="save"
  >
    <fieldset :disabled="!desktop || busy">
      <label for="title">书名 *</label><input
        id="title"
        v-model="form.title"
        required
        maxlength="200"
        placeholder="例如：明朝那些事儿"
      >
      <label for="author">作者（选填）</label><input
        id="author"
        v-model="form.author"
        placeholder="同名书建议填写作者"
      >
      <label for="notes">书籍内容或创作要求（选填）</label>
      <textarea
        id="notes"
        v-model="form.sourceNotes"
        rows="7"
        placeholder="可以粘贴书籍简介、读书笔记，或说明这一期想重点讲什么。不需要先准备大纲和讲稿。"
      />
      <button
        type="submit"
        class="save-book"
      >
        保存并继续
      </button>
    </fieldset>
  </form>
  <details
    v-if="project"
    class="history-panel"
  >
    <summary>管理这本书</summary>
    <button
      class="secondary danger"
      :disabled="busy"
      @click="deleting = true"
    >
      删除项目
    </button>
  </details>
  <section
    v-if="deleting"
    class="panel confirmation"
    role="alertdialog"
    aria-labelledby="delete-title"
  >
    <h2 id="delete-title">
      确认删除这本书？
    </h2>
    <p>此项目及生成历史将被删除。原始视频、素材文件和工作目录都会保留。</p>
    <div class="actions">
      <button
        class="danger"
        :disabled="busy"
        @click="remove"
      >
        确认删除
      </button><button
        class="secondary"
        :disabled="busy"
        @click="deleting = false"
      >
        取消
      </button>
    </div>
  </section>
</template>
