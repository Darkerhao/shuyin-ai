<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue'
import { Button } from '@/components/ui/button'
import { Textarea } from '@/components/ui/textarea'
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '@/components/ui/collapsible'
import type { VideoCue } from '../../types/video'
import { cueKey, replaceSubtitleText, subtitleClock, subtitleRows, type VideoEditorPosition } from './subtitles'
import { revealInScroller } from '../../lib/scroll'

const props = defineProps<{ originals: VideoCue[]; originalError: string; audioSrc: string; disabled: boolean }>()
const emit = defineEmits<{ playback: [index: number, follow: boolean] }>()
const text = defineModel<string>({ required: true })
const position = defineModel<VideoEditorPosition>('position', { required: true })
const editorRoot = ref<globalThis.HTMLElement>()
const audio = ref<globalThis.HTMLAudioElement>()
const playing = ref<number | null>(null)
const isPlaying = ref(false)
const playbackCue = ref(-1)
const playbackTime = ref(0)
const followPlayback = ref(false)
const playbackError = ref('')
let stopTimer: ReturnType<typeof globalThis.setTimeout> | undefined
let playRevision = 0
const rows = computed(() => subtitleRows(text.value))
const originalsByTime = computed(() => new Map(props.originals.map(cue => [cueKey(cue), cue])))
const compared = computed(() => rows.value.map(row => ({ ...row, original: originalsByTime.value.get(cueKey(row)) })))
const changed = computed(() => compared.value.filter(row => row.original && row.original.text !== row.text))
const filtered = computed(() => position.value.onlyChanges ? changed.value : compared.value)
function isEditingText() {
  const active = globalThis.document.activeElement as globalThis.HTMLElement | null
  return active?.tagName === 'INPUT' || active?.tagName === 'TEXTAREA' || active?.isContentEditable
}
const page = computed(() => Math.max(0, Math.floor(filtered.value.findIndex(row => row.index === (followPlayback.value && isPlaying.value && playbackCue.value >= 0 && !isEditingText() ? playbackCue.value : position.value.index)) / 20)))
const visible = computed(() => filtered.value.slice(page.value * 20, (page.value + 1) * 20))
const rawOpen = ref(false)
watch(() => position.value.section, section => { if (section === 'srt') rawOpen.value = true }, { immediate: true })

function select(index: number) { position.value = { ...position.value, section: 'subtitles', index } }
function stop() {
  playRevision++
  globalThis.clearTimeout(stopTimer)
  if (playing.value !== null || isPlaying.value) audio.value?.pause()
  isPlaying.value = false
  playbackCue.value = -1
  emit('playback', -1, false)
  playing.value = null
}
function stopAtEnd() {
  const row = rows.value.find(row => row.index === playing.value)
  if (row && audio.value && audio.value.currentTime >= row.end) stop()
}
function updatePlayback() {
  playbackTime.value = audio.value?.currentTime ?? 0
  const index = rows.value.find(row => playbackTime.value >= row.start && playbackTime.value < row.end)?.index ?? -1
  const following = followPlayback.value && !isEditingText()
  if (index !== playbackCue.value) { playbackCue.value = index; emit('playback', index, !!following) }
  if (following && index >= 0) {
    const row = editorRoot.value?.querySelector<globalThis.HTMLElement>(`#subtitle-row-${index}`)
    revealInScroller(row)
  }
  stopAtEnd()
}
function pausePlayback() {
  // A queued pause event can arrive after the next play() has resumed this element.
  if (!audio.value?.paused) return
  isPlaying.value = false; playing.value = null; globalThis.clearTimeout(stopTimer)
  playbackCue.value = -1
  emit('playback', -1, false)
}
function toggleFollow() {
  followPlayback.value = !followPlayback.value
  emit('playback', playbackCue.value, followPlayback.value)
}
async function playAll() {
  stop(); playbackError.value = ''
  if (!audio.value) return
  const revision = playRevision
  isPlaying.value = true
  try { await audio.value.play(); if (revision === playRevision) isPlaying.value = true }
  catch { if (revision === playRevision) { stop(); playbackError.value = '原声无法播放，请核对音频文件后重试。' } }
}
function checkPlayback() {
  stopAtEnd()
  if (playing.value !== null) stopTimer = globalThis.setTimeout(checkPlayback, 50)
}
async function play(index: number) {
  const row = rows.value[index]
  stop(); select(index); playbackError.value = ''
  if (!audio.value || !row) return
  const revision = playRevision
  try {
    audio.value.currentTime = row.start
    playing.value = index
    isPlaying.value = true
    await audio.value.play()
    if (revision !== playRevision) return
    checkPlayback()
  } catch {
    if (revision !== playRevision) return
    stop(); playbackError.value = '原声无法播放，请核对音频文件后重试。'
  }
}
function edit(index: number, value: string) {
  const row = rows.value[index]
  if (row) { select(index); text.value = replaceSubtitleText(text.value, row, value) }
}
function filterChanges() {
  const onlyChanges = !position.value.onlyChanges
  position.value = { ...position.value, onlyChanges, section: 'subtitles', index: (onlyChanges ? changed.value[0]?.index : position.value.index) ?? 0 }
}
function rawPosition(event: globalThis.Event) {
  const target = event.target as globalThis.HTMLTextAreaElement
  position.value = { ...position.value, section: 'srt', cursor: target.selectionStart }
}
function restorePosition() {
  if (position.value.section === 'srt') {
    const target = editorRoot.value?.querySelector<globalThis.HTMLTextAreaElement>('#video-subtitles')
    target?.focus({ preventScroll: true }); target?.setSelectionRange(position.value.cursor, position.value.cursor)
    target?.scrollIntoView?.({ block: 'center' })
  } else {
    editorRoot.value?.querySelector(`#subtitle-row-${position.value.index}`)?.scrollIntoView?.({ block: 'center' })
  }
}
defineExpose({ restorePosition, play })
watch(() => props.audioSrc, stop)
watch(() => props.disabled, disabled => { if (disabled) stop() })
onBeforeUnmount(stop)
</script>

<template>
  <section
    ref="editorRoot"
    class="subtitle-review"
    aria-label="字幕逐句对照"
  >
    <div class="actions">
      <Button
        variant="outline"
        :disabled="disabled || !audioSrc"
        @click="playAll"
      >
        连续播放原声
      </Button>
      <Button
        variant="ghost"
        :aria-pressed="followPlayback"
        :disabled="disabled || !audioSrc"
        @click="toggleFollow"
      >
        跟随播放
      </Button>
      <Button
        variant="outline"
        :aria-pressed="position.onlyChanges"
        :disabled="disabled || !originals.length"
        @click="filterChanges"
      >
        只看修改项（{{ changed.length }}）
      </Button>
      <span class="muted">共 {{ rows.length }} 句 · {{ changed.length }} 句与原文不同</span>
      <Button
        v-if="playing !== null || isPlaying"
        variant="outline"
        @click="stop"
      >
        停止试听
      </Button>
    </div>
    <p
      v-if="originalError"
      class="notice"
      role="status"
    >
      {{ originalError }}
    </p>
    <p
      v-if="playbackError"
      class="notice"
      role="alert"
    >
      {{ playbackError }}
    </p>
    <audio
      v-if="audioSrc"
      ref="audio"
      :src="audioSrc"
      preload="metadata"
      controls
      aria-label="当前视频任务原声"
      @timeupdate="updatePlayback"
      @play="isPlaying = true"
      @pause="pausePlayback"
      @ended="stop"
      @error="stop(); playbackError = '原声读取失败，请在素材管理中核对音频。'"
    />
    <p
      v-if="audioSrc"
      class="muted playback-status"
      role="status"
    >
      原声 {{ subtitleClock(playbackTime) }}{{ isPlaying && playbackCue >= 0 ? ` · 第 ${playbackCue + 1} 句` : '' }}
    </p>
    <p
      v-if="!audioSrc"
      class="muted"
    >
      此任务的原声暂不可试听，请核对音频素材。
    </p>
    <p
      v-if="!rows.length"
      class="notice"
    >
      SRT 格式尚未完整，请在下方完整文本中继续编辑；未完成内容也会暂存。
    </p>
    <p
      v-else-if="!filtered.length"
      class="muted"
    >
      没有与原文不同的字幕。
    </p>
    <div class="subtitle-list">
      <article
        v-for="row in visible"
        :id="`subtitle-row-${row.index}`"
        :key="row.index"
        class="subtitle-row"
        :class="{ 'subtitle-row-active': position.section === 'subtitles' && position.index === row.index, 'subtitle-row-playing': isPlaying && playbackCue === row.index }"
      >
        <div class="actions">
          <Button
            variant="ghost"
            :disabled="disabled || !audioSrc"
            :aria-label="`试听第${row.index + 1}句`"
            @click="play(row.index)"
          >
            {{ playing === row.index ? '正在试听' : '试听' }} · 第 {{ row.index + 1 }} 句
          </Button>
          <span class="muted subtitle-time">{{ subtitleClock(row.start) }}—<wbr>{{ subtitleClock(row.end) }}</span>
          <span
            v-if="row.original && row.text !== row.original.text"
            class="subtitle-change"
          >已修改</span>
          <Button
            variant="ghost"
            :disabled="disabled || !row.original || row.text === row.original.text"
            :aria-label="`恢复第${row.index + 1}句原文`"
            @click="edit(row.index, row.original!.text)"
          >
            恢复原文
          </Button>
        </div>
        <div class="subtitle-columns">
          <div>
            <span class="muted">原文</span><p class="subtitle-original">
              {{ row.original?.text ?? '时间码已变化，无法对应原文' }}
            </p>
          </div>
          <div>
            <label :for="`subtitle-text-${row.index}`">校对文</label><Textarea
              :id="`subtitle-text-${row.index}`"
              :model-value="row.text"
              :disabled="disabled"
              rows="2"
              spellcheck="false"
              @focus="select(row.index)"
              @update:model-value="edit(row.index, String($event))"
            />
          </div>
        </div>
      </article>
    </div>
    <div
      v-if="filtered.length > 20"
      class="actions"
      aria-label="字幕分页"
    >
      <Button
        variant="outline"
        :disabled="disabled || page === 0"
        @click="select(filtered[(page - 1) * 20]!.index)"
      >
        上一页
      </Button>
      <span>{{ page + 1 }} / {{ Math.ceil(filtered.length / 20) }}</span>
      <Button
        variant="outline"
        :disabled="disabled || (page + 1) * 20 >= filtered.length"
        @click="select(filtered[(page + 1) * 20]!.index)"
      >
        下一页
      </Button>
    </div>
    <Collapsible
      v-model:open="rawOpen"
      :unmount-on-hide="false"
    >
      <CollapsibleTrigger class="disclosure-trigger">
        编辑完整 SRT 文本
      </CollapsibleTrigger>
      <CollapsibleContent>
        <label for="video-subtitles">SRT 字幕</label><Textarea
          id="video-subtitles"
          v-model="text"
          :disabled="disabled"
          rows="9"
          spellcheck="false"
          @click="rawPosition"
          @keyup="rawPosition"
          @input="rawPosition"
        />
      </CollapsibleContent>
    </Collapsible>
  </section>
</template>

<style scoped>
.subtitle-review { display: grid; gap: 1rem; min-width: 0; }
.subtitle-list { display: grid; gap: .75rem; max-height: 560px; overflow: auto; padding: 2px; }
.subtitle-row { padding: 1rem; border: 1px solid var(--border); border-radius: .75rem; min-width: 0; }
.subtitle-row-active { border-color: var(--primary); }
.subtitle-row-playing { background: var(--accent); box-shadow: inset 3px 0 var(--primary); }
.subtitle-review audio { width: 100%; }
.playback-status { margin: 0; font-size: 12px; font-variant-numeric: tabular-nums; }
.subtitle-columns { display: grid; grid-template-columns: 1fr 1fr; gap: 1rem; }
.subtitle-columns > div { min-width: 0; }
.subtitle-original { white-space: pre-wrap; overflow-wrap: anywhere; margin-top: .5rem; }
.subtitle-change { color: var(--primary); font-size: .875rem; }
.subtitle-time { max-width: 100%; font-size: .75rem; overflow-wrap: anywhere; }
@media (max-width: 700px) { .subtitle-columns { grid-template-columns: 1fr; } }
</style>
