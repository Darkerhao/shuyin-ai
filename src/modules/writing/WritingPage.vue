<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { RouterLink, onBeforeRouteLeave, onBeforeRouteUpdate } from 'vue-router'
import * as api from '../../services/ai'
import { getProject, isDesktop } from '../../services/projects'
import { writingKinds, taskStatuses, type AiTask, type ModelProvider, type WritingContent, type WritingKind, type WritingVersion } from '../../types/ai'
import { useOperation } from '../../composables/useOperation'
const props = defineProps<{ id: string }>()
const desktop = isDesktop()
const title = ref('')
const ready = ref(false)
const kind = ref<WritingKind>('outline')
const providers = ref<ModelProvider[]>([])
const providerId = ref('')
const versions = ref<WritingVersion[]>([])
const references = ref<WritingVersion[]>([])
const referenceId = ref('')
const selected = ref<WritingVersion | null>(null)
const draft = ref('')
const confirmed = ref(false)
const original = ref('')
const originalConfirmed = ref(false)
const dirty = computed(() => draft.value !== original.value || confirmed.value !== originalConfirmed.value)
const count = ref(5)
const tasks = ref<AiTask[]>([])
const taskId = ref('')
const cancelling = ref(false)
const { busy, error, message, run } = useOperation()
const isList = computed(() => !['outline', 'script'].includes(kind.value))
const content = (): WritingContent => isList.value ? { format: 'list', items: draft.value.split('\n').map(s => s.trim()).filter(Boolean) } : { format: 'markdown', text: draft.value }
function accept(version: WritingVersion | null) {
  selected.value = version
  draft.value = version ? (version.content.format === 'markdown' ? version.content.text : version.content.items.join('\n')) : ''
  confirmed.value = version?.isConfirmed ?? false
  original.value = draft.value; originalConfirmed.value = confirmed.value
}
function canDiscard() { return !dirty.value || globalThis.confirm('当前内容尚未保存，确定放弃修改？') }
const canLeave = () => !busy.value && canDiscard()
onBeforeRouteLeave(canLeave)
onBeforeRouteUpdate(canLeave)
function beforeUnload(event: globalThis.BeforeUnloadEvent) { if (dirty.value || busy.value) event.preventDefault() }
globalThis.addEventListener('beforeunload', beforeUnload)
onBeforeUnmount(() => globalThis.removeEventListener('beforeunload', beforeUnload))
async function refresh() {
  const [saved, recent, outlines, scripts] = await Promise.all([api.listVersions(props.id, kind.value), api.listTasks(props.id), api.listVersions(props.id, 'outline'), api.listVersions(props.id, 'script')])
  versions.value = saved; tasks.value = recent; references.value = [...outlines, ...scripts]
}
const load = () => run(async () => {
  const [project, models] = await Promise.all([getProject(props.id), api.listProviders()])
  title.value = project.title; providers.value = models.filter(p => p.isEnabled)
  providerId.value = providers.value.find(p => p.isDefault)?.id ?? providers.value[0]?.id ?? ''
  await refresh(); accept(versions.value[0] ?? null); ready.value = true
})
function switchVersion(event: globalThis.Event) {
  const select = event.target as globalThis.HTMLSelectElement
  if (canDiscard()) accept(versions.value.find(v => v.id === select.value) ?? null)
  else select.value = selected.value?.id ?? ''
}
async function switchKind(event: globalThis.Event) {
  const select = event.target as globalThis.HTMLSelectElement
  const next = select.value as WritingKind
  select.value = kind.value
  if (!canDiscard()) return
  await run(async () => {
    // Only replace the editor after the next history has loaded successfully.
    const saved = await api.listVersions(props.id, next)
    kind.value = next; versions.value = saved; count.value = next === 'descriptions' ? 3 : next === 'hashtags' ? 8 : 5
    accept(saved[0] ?? null)
  })
}
const save = () => run(async () => {
  const saved = await api.saveWriting({ projectId: props.id, kind: kind.value, content: content(), basedOn: selected.value?.id ?? null, isConfirmed: confirmed.value })
  accept(saved); versions.value = [saved, ...versions.value]; message.value = `已保存为版本 ${saved.versionNo}。`
  await refresh()
})
const generate = () => run(async () => {
  try {
    taskId.value = await api.startGeneration({ projectId: props.id, providerId: providerId.value, kind: kind.value, count: count.value, referenceVersionId: referenceId.value || null })
    const saved = await api.executeGeneration(taskId.value)
    versions.value = [saved, ...versions.value]
    if (!dirty.value) accept(saved)
    message.value = `生成结果已保存为版本 ${saved.versionNo}。${dirty.value ? '当前编辑草稿已保留，可保存后切换查看。' : '请核实书籍事实后确认。'}`
  } finally {
    taskId.value = ''
    // A history refresh failure must not hide the original generation error.
    try { await refresh() } catch { message.value = '历史列表刷新失败，当前编辑内容保留；可重新进入页面读取已保存版本。' }
  }
})
async function cancel(id: string) {
  if (cancelling.value) return
  cancelling.value = true
  try { await api.cancelGeneration(id); tasks.value = await api.listTasks(props.id) }
  catch { error.value = '取消任务失败，请稍后重试。' }
  finally { cancelling.value = false }
}
onMounted(() => { if (desktop) void load() })
</script>
<template>
  <header>
    <RouterLink :to="`/projects/${id}`">
      ← 项目资料
    </RouterLink><h1>{{ title || '书籍' }} · AI 创作</h1><p class="muted">
      {{ dirty ? '有未保存的修改' : '生成、编辑与版本记录' }}。生成使用已保存的项目资料，请先核实内容。
    </p>
  </header>
  <p
    v-if="!desktop"
    class="notice"
  >
    请在桌面应用中使用 AI 创作。
  </p>
  <p
    v-if="error"
    class="error"
    role="alert"
  >
    {{ error }}
  </p><p
    v-if="message"
    class="notice"
    role="status"
  >
    {{ message }}
  </p>
  <button
    v-if="!ready"
    :disabled="!desktop || busy"
    @click="load"
  >
    {{ busy ? '读取中…' : '重新读取' }}
  </button>
  <template v-if="ready">
    <section class="panel">
      <h2>创作内容</h2>
      <div class="form-grid">
        <div>
          <label for="writing-kind">内容类型</label><select
            id="writing-kind"
            :value="kind"
            :disabled="busy"
            @change="switchKind"
          >
            <option
              v-for="(label, value) in writingKinds"
              :key="value"
              :value="value"
            >
              {{ label }}
            </option>
          </select>
        </div>
        <div>
          <label for="writing-provider">文本模型</label><select
            id="writing-provider"
            v-model="providerId"
            :disabled="busy"
          >
            <option
              value=""
              disabled
            >
              请配置模型
            </option><option
              v-for="provider in providers"
              :key="provider.id"
              :value="provider.id"
            >
              {{ provider.name }} · {{ provider.modelName }}
            </option>
          </select>
        </div>
      </div>
      <p
        v-if="!providers.length"
        class="notice"
      >
        尚无启用的文本模型。<RouterLink to="/settings/models">
          前往模型设置
        </RouterLink>
      </p>
      <label for="writing-reference">参考已保存的大纲或讲稿（可选）</label><select
        id="writing-reference"
        v-model="referenceId"
        :disabled="busy"
      >
        <option value="">
          仅使用项目资料
        </option><option
          v-for="version in references"
          :key="version.id"
          :value="version.id"
        >
          {{ writingKinds[version.kind] }} · v{{ version.versionNo }}{{ version.isConfirmed ? ' · 已确认' : '' }}
        </option>
      </select>
      <template v-if="isList">
        <label for="writing-count">生成数量（1–20）</label><input
          id="writing-count"
          v-model.number="count"
          type="number"
          min="1"
          max="20"
          step="1"
          :disabled="busy"
        >
      </template>
      <div class="actions">
        <button
          :disabled="busy || !providerId || !Number.isInteger(count) || count < 1 || count > 20"
          @click="generate"
        >
          {{ busy && taskId ? '生成中…' : '生成新版本' }}
        </button><button
          v-if="taskId"
          class="secondary"
          :disabled="cancelling"
          @click="cancel(taskId)"
        >
          取消生成
        </button>
      </div>
      <p
        v-if="taskId"
        role="status"
        class="muted"
      >
        正在请求模型，最多等待 120 秒。已有内容保留。
      </p>
    </section>
    <form
      class="panel"
      @submit.prevent="save"
    >
      <h2>{{ writingKinds[kind] }}编辑</h2>
      <label for="writing-version">历史版本</label><select
        id="writing-version"
        :value="selected?.id ?? ''"
        :disabled="busy"
        @change="switchVersion"
      >
        <option value="">
          新建手写内容
        </option><option
          v-for="version in versions"
          :key="version.id"
          :value="version.id"
        >
          v{{ version.versionNo }} · {{ version.sourceType === 'ai' ? 'AI 生成' : version.sourceType === 'mixed' ? '人工修订' : '手写' }}{{ version.isConfirmed ? ' · 已确认' : '' }} · {{ new Date(version.createdAt).toLocaleString() }}
        </option>
      </select>
      <p
        v-if="selected"
        class="muted"
      >
        {{ selected.modelName || '人工创作' }}{{ selected.promptVersion ? ` · ${selected.promptVersion}` : '' }}。修改后保存为新版本，历史内容保留。
      </p>
      <label for="writing-draft">{{ isList ? '每行一项；可修改、增删候选内容' : '正文（支持 Markdown）' }}</label><textarea
        id="writing-draft"
        v-model="draft"
        :disabled="busy && !taskId"
        rows="16"
        maxlength="100000"
        :placeholder="isList ? '每行输入一个候选项' : '可以直接手写，也可以生成后编辑'"
      />
      <label class="check-label"><input
        v-model="confirmed"
        :disabled="busy && !taskId"
        type="checkbox"
      >已人工审核并确认此版本</label>
      <button
        :disabled="busy || !draft.trim()"
        type="submit"
      >
        保存为新版本
      </button>
    </form>
    <section class="panel">
      <h2>最近生成任务</h2><p
        v-if="!tasks.length"
        class="muted"
      >
        暂无生成记录。
      </p><div
        v-for="task in tasks"
        :key="task.id"
        class="task-row"
      >
        <span>{{ writingKinds[task.kind] }} · {{ task.modelName }} · {{ taskStatuses[task.status] }}</span><p
          v-if="task.errorMessage"
          class="muted"
        >
          {{ task.errorMessage }}
        </p><button
          v-if="['pending','running'].includes(task.status) && task.id !== taskId"
          class="secondary"
          :disabled="cancelling"
          @click="cancel(task.id)"
        >
          取消未完成任务
        </button>
      </div>
    </section>
  </template>
</template>
