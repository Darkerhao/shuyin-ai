<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { onBeforeRouteLeave, onBeforeRouteUpdate } from 'vue-router'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Textarea } from '@/components/ui/textarea'
import { useEditorDraft } from '../../composables/useEditorDraft'
import { readDraft } from '../../services/drafts'
import { chooseNote, importNote, type ImportedNote } from '../../services/notes'
import { getChapter, isDesktop, saveChapter } from '../../services/projects'
import { chapterOwner, emptyChapter, type Chapter, type ChapterInput } from '../../types/chapter'

const props = defineProps<{ projectId: string; chapterId?: string | null; disabled?: boolean }>()
const emit = defineEmits<{ saved: [chapter: Chapter] }>()
const desktop = isDesktop()
const saved = ref<Chapter | null>(null)
const form = ref(emptyChapter())
const original = ref(JSON.stringify(form.value))
const originalDuration = ref<number | null>(null)
const ready = ref(false)
const busy = ref(false)
const error = ref('')
const message = ref('')
const imported = ref<ImportedNote | null>(null)
const dirty = computed(() => original.value !== JSON.stringify(form.value))
const owner = () => chapterOwner(props.projectId, saved.value?.id ?? props.chapterId ?? 'new')
const autosave = useEditorDraft(owner, 'project', form, () => saved.value?.updatedAt ?? null, dirty, ready, () => !busy.value && !props.disabled)
const characters = computed(() => [...form.value.content].length)
const durationMinutes = computed({
  get: () => form.value.targetDurationSeconds === null ? '' : String(form.value.targetDurationSeconds / 60),
  set: (value: string | number) => { form.value.targetDurationSeconds = String(value).trim() ? Number(value) * 60 : null },
})
const valid = computed(() => !!form.value.title.trim() && [...form.value.title].length <= 200 && !!form.value.content.trim() && characters.value <= 100_000
  && (form.value.targetDurationSeconds === null || (Number.isInteger(form.value.targetDurationSeconds / 60) && form.value.targetDurationSeconds >= 60 && form.value.targetDurationSeconds <= 86400)))

function accept(chapter: Chapter) {
  saved.value = chapter
  form.value = { title: chapter.title, content: chapter.content, targetDurationSeconds: chapter.targetDurationSeconds }
  originalDuration.value = chapter.targetDurationSeconds
  original.value = JSON.stringify(form.value)
}
onMounted(async () => {
  if (!desktop) return
  try {
    if (props.chapterId && props.chapterId !== 'new') accept(await getChapter(props.projectId, props.chapterId))
    const recovery = await readDraft<ChapterInput>(owner(), 'project')
    if (recovery) {
      form.value = { ...form.value, title: recovery.content.title, content: recovery.content.content }
      if (!saved.value || recovery.basedOn === saved.value.updatedAt) form.value.targetDurationSeconds = recovery.content.targetDurationSeconds
      message.value = '已恢复章节暂存。保存后才用于生成提示词和音频。'
    }
    ready.value = true
  } catch (cause) { error.value = String(cause) }
})
async function save() {
  if (busy.value || !desktop || !ready.value) throw new Error('请等待章节资料读取或保存完成。')
  if (!valid.value) { error.value = '请填写章节名称和完整正文，目标时长须为1—1440分钟的整数。'; throw new Error(error.value) }
  busy.value = true; error.value = ''
  try {
    await autosave.flush()
    const input = { ...form.value, targetDurationSeconds: saved.value && form.value.targetDurationSeconds === originalDuration.value ? null : form.value.targetDurationSeconds }
    const chapter = await saveChapter(props.projectId, saved.value?.id ?? null, input)
    autosave.reset(); accept(chapter)
    message.value = '章节已保存。后续创作只使用这份章节内容。'
    emit('saved', chapter)
    return chapter
  } catch (cause) { error.value = String(cause); throw cause }
  finally { busy.value = false }
}
async function saveFromForm() { try { await save() } catch { /* The editor keeps the visible error and draft. */ } }
async function chooseImport() {
  busy.value = true; error.value = ''
  try { const path = await chooseNote(); if (path) imported.value = await importNote(path) }
  catch (cause) { error.value = String(cause) }
  finally { busy.value = false }
}
function applyImport() {
  if (!imported.value) return
  form.value.content = imported.value.text
  if (!form.value.title.trim()) form.value.title = imported.value.sourceName.replace(/\.[^.]+$/, '')
  imported.value = null
  message.value = '已带入章节正文。检查并保存后再制作。'
}
async function canLeave() {
  if (busy.value || props.disabled) return false
  try { await autosave.flush(); return true } catch { return false }
}
onBeforeRouteLeave(canLeave)
onBeforeRouteUpdate(canLeave)
defineExpose({ save })
</script>

<template>
  <section
    class="panel my-5"
    aria-labelledby="chapter-heading"
    :aria-busy="busy"
  >
    <div class="flex flex-wrap items-start justify-between gap-3">
      <div>
        <h2 id="chapter-heading">
          {{ saved ? '章节内容' : '新增章节内容' }}
        </h2>
        <p class="mt-2 text-sm text-muted-foreground">
          逐章细致解读这本书。每章独立制作文案、音频、三尺寸封面和视频，并继续发布准备。
        </p>
      </div>
      <Button
        variant="outline"
        :disabled="busy || disabled || !desktop"
        @click="chooseImport"
      >
        导入 TXT / Markdown
      </Button>
    </div>
    <p
      v-if="error || autosave.error.value"
      role="alert"
      class="mt-3 text-sm text-destructive"
    >
      {{ error || autosave.error.value }}
    </p>
    <p
      v-else-if="message || autosave.status.value"
      role="status"
      class="mt-3 text-sm text-muted-foreground"
    >
      {{ message || autosave.status.value }}
    </p>
    <form
      class="mt-5 grid gap-5"
      @submit.prevent="saveFromForm"
    >
      <div class="grid gap-2">
        <label for="chapter-title">章节名称 *</label>
        <Input
          id="chapter-title"
          v-model="form.title"
          maxlength="200"
          placeholder="例如：第三章：习惯是怎样形成的"
          :disabled="busy || disabled || !ready"
        />
      </div>
      <div class="grid gap-2">
        <label for="chapter-content">章节正文 *</label>
        <Textarea
          id="chapter-content"
          v-model="form.content"
          rows="16"
          class="min-h-72 text-sm leading-7"
          placeholder="在此粘贴这一章的完整内容"
          :disabled="busy || disabled || !ready"
          aria-describedby="chapter-content-help"
        />
        <p
          id="chapter-content-help"
          class="text-sm text-muted-foreground"
        >
          {{ characters }} / 100000 字符 · {{ saved ? '修改只影响新创作；已有任务仍使用原章节快照。' : '会自动暂存，正式保存后才能开始生成。' }}
        </p>
      </div>
      <div class="grid gap-2">
        <label for="chapter-duration">本章目标时长（分钟，选填）</label>
        <Input
          id="chapter-duration"
          v-model="durationMinutes"
          type="number"
          min="1"
          max="1440"
          step="1"
          class="max-w-48"
          placeholder="按本章内容推荐"
          :disabled="busy || disabled || !ready"
        />
        <p class="text-sm text-muted-foreground">
          留空沿用本章已设置的时长；未设置时由 AI 推荐。实际视频按完整音频制作。
        </p>
      </div>
      <div
        v-if="imported"
        class="rounded-lg border border-border p-4"
      >
        <p class="mb-3 text-sm">
          导入预览：{{ imported.sourceName }}。采用后替换当前编辑框正文。
        </p>
        <Textarea
          :model-value="imported.text"
          readonly
          rows="6"
          aria-label="章节导入预览"
        />
        <div class="mt-3 flex gap-3">
          <Button
            type="button"
            @click="applyImport"
          >
            采用这份正文
          </Button><Button
            type="button"
            variant="outline"
            @click="imported = null"
          >
            取消导入
          </Button>
        </div>
      </div>
      <Button
        type="submit"
        class="justify-self-start"
        :disabled="busy || disabled || !ready || !valid"
      >
        {{ busy ? '正在保存…' : '保存章节' }}
      </Button>
    </form>
  </section>
</template>
