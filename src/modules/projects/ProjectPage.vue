<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { RouterLink, onBeforeRouteLeave, onBeforeRouteUpdate, useRouter } from 'vue-router'
import * as api from '../../services/projects'
import { emptyProject, projectInput, projectStatuses, type Project } from '../../types/project'
import { useOperation } from '../../composables/useOperation'
import ProjectAssets from './ProjectAssets.vue'
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
const openDirectory = () => run(() => api.openProjectDirectory(project.value!.id))
const { busy, error, message, run } = useOperation()
function accept(saved: Project) { project.value = saved; form.value = projectInput(saved); original.value = JSON.stringify(form.value); ready.value = true }
const load = () => run(async () => { if (props.id) accept(await api.getProject(props.id)) })
const save = () => run(async () => {
  const saved = await api.saveProject(props.id ?? null, { ...form.value })
  accept(saved)
  message.value = '项目已保存。'
  if (!props.id) { leaving = true; await router.replace(`/projects/${saved.id}`) }
})
const remove = () => run(async () => {
  if (!project.value) return
  await api.deleteProject(project.value.id)
  original.value = JSON.stringify(form.value)
  leaving = true
  await router.push('/projects')
})
const changeDirectory = () => run(async () => {
  if (!project.value) return
  const path = await api.chooseDirectory()
  if (path) { const updated = await api.relocateWorkspace(project.value.id, path); project.value = updated; message.value = '工作目录引用已更新，原目录和素材保持原位。' }
})
const canLeave = () => leaving || (!busy.value && (!dirty.value || globalThis.confirm('项目资料尚未保存，确定离开并放弃修改？')))
onBeforeRouteLeave(canLeave)
onBeforeRouteUpdate(canLeave)
onMounted(() => { if (desktop && props.id) void load() })
</script>
<template>
  <header>
    <RouterLink to="/projects">
      ← 书籍项目
    </RouterLink><h1>{{ project ? project.title : '新建书籍项目' }}</h1><p class="muted">
      {{ dirty ? '有未保存的修改' : '书籍资料与创作方向' }}
    </p>
    <RouterLink
      v-if="project"
      class="button"
      :to="`/projects/${project.id}/writing`"
    >
      进入 AI 创作
    </RouterLink>
  </header>
  <p
    v-if="!desktop"
    class="notice"
  >
    请在桌面应用中编辑项目。
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
    正在读取项目…
  </p>
  <form
    v-if="ready"
    class="panel"
    @submit.prevent="save"
  >
    <fieldset :disabled="!desktop || busy">
      <div class="form-grid">
        <div>
          <label for="title">书名 *</label><input
            id="title"
            v-model="form.title"
            required
            maxlength="200"
          >
        </div>
        <div>
          <label for="author">作者</label><input
            id="author"
            v-model="form.author"
          >
        </div>
        <div>
          <label for="publisher">出版社</label><input
            id="publisher"
            v-model="form.publisher"
          >
        </div>
        <div>
          <label for="isbn">ISBN</label><input
            id="isbn"
            v-model="form.isbn"
          >
        </div>
        <div>
          <label for="theme">主题</label><input
            id="theme"
            v-model="form.theme"
          >
        </div>
        <div>
          <label for="audience">目标受众</label><input
            id="audience"
            v-model="form.targetAudience"
          >
        </div>
        <div>
          <label for="style">讲解风格</label><input
            id="style"
            v-model="form.contentStyle"
            placeholder="例如：温暖、通俗、结合生活"
          >
        </div>
        <div>
          <label for="duration">目标时长（秒）</label><input
            id="duration"
            v-model.number="form.targetDurationSeconds"
            type="number"
            min="1"
            max="86400"
            step="1"
            required
          >
        </div>
      </div>
      <label for="description">书籍简介</label><textarea
        id="description"
        v-model="form.description"
        rows="3"
      />
      <label for="notes">用户资料与读书笔记</label><textarea
        id="notes"
        v-model="form.sourceNotes"
        rows="6"
        placeholder="整理参考资料，后续可用于 AI 创作。"
      />
      <label for="project-status">项目状态（选择已归档后保存即可归档）</label>
      <select
        id="project-status"
        v-model="form.status"
      >
        <option
          v-for="(label, value) in projectStatuses"
          :key="value"
          :value="value"
        >
          {{ label }}
        </option>
      </select>
      <div class="actions">
        <button type="submit">
          {{ busy ? '保存中…' : '保存项目' }}
        </button><button
          v-if="project"
          type="button"
          class="danger secondary"
          @click="deleting = true"
        >
          删除项目
        </button>
      </div>
    </fieldset>
  </form>
  <section
    v-if="deleting && project"
    class="panel confirmation"
    role="alertdialog"
    aria-modal="false"
    aria-labelledby="delete-title"
  >
    <h2 id="delete-title">
      确认删除「{{ project.title }}」？
    </h2><p>将删除项目资料、创作版本、生成任务及全部素材关联，无法撤销。原始视频、素材文件和工作目录都会保留。</p>
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
  <section
    v-if="project"
    class="panel"
  >
    <h2>项目工作目录</h2><p class="file-path">
      {{ project.workspacePath }}
    </p><p class="muted">
      重新指定只更新目录引用，不搬迁现有文件或修改素材路径。
    </p>
    <div class="actions">
      <button
        class="secondary"
        :disabled="busy"
        @click="openDirectory"
      >
        打开工作目录
      </button><button
        class="secondary"
        :disabled="busy"
        @click="changeDirectory"
      >
        重新指定目录
      </button>
    </div>
  </section>
  <ProjectAssets
    v-if="project"
    :project-id="project.id"
  />
</template>
