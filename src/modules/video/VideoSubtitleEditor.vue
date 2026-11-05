<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue'
import { Button } from '@/components/ui/button'
import { Textarea } from '@/components/ui/textarea'
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '@/components/ui/collapsible'
import type { VideoCue } from '../../types/video'
import { cueKey, replaceSubtitleText, subtitleClock, subtitleRows, type VideoEditorPosition } from './subtitles'
import { revealInScroller } from '../../lib/scroll'
import AudioPlayer from '../../components/AudioPlayer.vue'
import AppIcon from '../../components/AppIcon.vue'

const props = defineProps<{ originals: VideoCue[]; originalError: string; projectId: string; audioAssetId: string | null; disabled: boolean }>()
const emit = defineEmits<{ playback: [index: number, follow: boolean] }>()
const text = defineModel<string>({ required: true })
const position = defineModel<VideoEditorPosition>('position', { required: true })
const editorRoot = ref<globalThis.HTMLElement>()
const player = ref<InstanceType<typeof AudioPlayer>>()
const audio = computed(() => player.value?.media)
const playing = ref<number | null>(null)
const isPlaying = ref(false)
const playbackCue = ref(-1)
const playbackTime = ref(0)
const followPlayback = ref(false)
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
  if (playing.value !== null || isPlaying.value) player.value?.pause()
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
function checkPlayback() {
  stopAtEnd()
  if (playing.value !== null) stopTimer = globalThis.setTimeout(checkPlayback, 50)
}
async function play(index: number) {
  const row = rows.value[index]
  stop(); select(index)
  if (!audio.value || !row) return
  const revision = playRevision
  playing.value = index
  isPlaying.value = true
  const started = await player.value!.play(row.start)
  if (revision !== playRevision) return
  if (started) checkPlayback()
  else stop()
}
function clearBoundary() { playing.value = null; globalThis.clearTimeout(stopTimer) }
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
defineExpose({ restorePosition, play, audio })
watch(() => props.audioAssetId, stop)
watch(() => props.disabled, disabled => { if (disabled) stop() })
onBeforeUnmount(stop)
</script>

<template>
  <section
    ref="editorRoot"
    class="subtitle-review"
    aria-label="字幕逐句对照"
  >
    <div class="actions subtitle-toolbar">
      <Button
        variant="ghost"
        class="subtitle-follow"
        :aria-pressed="followPlayback"
        :disabled="disabled || !audio"
        @click="toggleFollow"
      >
        <span
          class="subtitle-follow-mark"
          aria-hidden="true"
        ><AppIcon
          v-if="followPlayback"
          name="check"
        /></span>跟随播放
      </Button>
      <Button
        variant="outline"
        class="subtitle-filter"
        :aria-pressed="position.onlyChanges"
        :disabled="disabled || !originals.length"
        @click="filterChanges"
      >
        只看修改项（{{ changed.length }}）
      </Button>
      <span class="muted subtitle-count">共 {{ rows.length }} 句 · <strong>{{ changed.length }} 句与原文不同</strong></span>
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
    <AudioPlayer
      v-if="audioAssetId"
      ref="player"
      :project-id="projectId"
      :asset-id="audioAssetId"
      title="字幕与分镜试听"
      subtitle="当前任务原声 · 按原时间码核对"
      :disabled="disabled"
      aria-label="当前视频任务原声"
      @timeupdate="updatePlayback"
      @play="isPlaying = true"
      @pause="pausePlayback"
      @ended="stop"
      @seek="clearBoundary"
      @error="stop"
    />
    <p
      v-if="audioAssetId"
      class="muted playback-status"
      role="status"
    >
      原声 {{ subtitleClock(playbackTime) }}{{ isPlaying && playbackCue >= 0 ? ` · 第 ${playbackCue + 1} 句` : '' }}
    </p>
    <p
      v-if="!audioAssetId"
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
        <div class="actions subtitle-row-heading">
          <Button
            variant="ghost"
            class="subtitle-listen"
            :disabled="disabled || !audio"
            :aria-label="`试听第${row.index + 1}句`"
            @click="play(row.index)"
          >
            <AppIcon name="play" />{{ playing === row.index ? '正在试听' : '试听' }} · 第 {{ row.index + 1 }} 句
          </Button>
          <span class="muted subtitle-time">{{ subtitleClock(row.start) }}—<wbr>{{ subtitleClock(row.end) }}</span>
          <span
            v-if="row.original && row.text !== row.original.text"
            class="subtitle-change"
          >已修改</span>
          <Button
            variant="ghost"
            class="subtitle-restore"
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
      class="actions subtitle-pagination"
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
      class="subtitle-raw"
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
.subtitle-review { display: grid; gap: 20px; min-width: 0; font: 400 12px/1.65 var(--font-mono); }
.subtitle-toolbar, .subtitle-row-heading { margin: 0; gap: 12px; }
.subtitle-follow { padding: 6px 0; }
.subtitle-follow-mark { display: grid; place-items: center; width: 14px; height: 14px; border: 1px solid var(--input-border); border-radius: 3px; }
.subtitle-follow[aria-pressed='true'] .subtitle-follow-mark { border-color: var(--primary); background: var(--primary); color: #020617; }
.subtitle-follow-mark .app-icon { width: 12px; height: 12px; }
.subtitle-filter { padding: 6px 12px; color: var(--info); border-color: #00e5ff66; }
.subtitle-filter[aria-pressed='true'] { background: #00e5ff26; }
.subtitle-count { margin-left: auto; }
.subtitle-count strong { color: var(--info); font-weight: 700; }
.subtitle-review :deep(.audio-player:not([data-theme='oled'])) { --player-accent: var(--info); --player-accent-light: var(--info); --player-glow: #00e5ff59; background: #0a121a; border-color: var(--border); }
.subtitle-list { display: grid; gap: 16px; max-height: 680px; overflow: auto; padding: 3px; }
.subtitle-row { display: grid; gap: 12px; padding: 16px; border: 1px solid var(--border); border-radius: 16px; background: #02061766; min-width: 0; }
.subtitle-row-active { border-color: #00e5ff80; }
.subtitle-row-playing { background: #00e5ff0d; box-shadow: inset 3px 0 var(--info); }
.subtitle-row-heading button { height: auto; min-height: 28px; padding: 2px 0; font-size: 12px; }
.subtitle-listen { color: var(--info); font-weight: 700; }
.subtitle-restore { margin-left: auto; color: var(--muted-foreground); }
.playback-status { margin: 0; font-size: 12px; font-variant-numeric: tabular-nums; }
.subtitle-columns { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 16px; }
.subtitle-columns > div { display: flex; flex-direction: column; gap: 6px; min-width: 0; }
.subtitle-columns label { margin: 0; color: var(--info); font-size: 11px; font-weight: 700; }
.subtitle-columns > div > span { font-size: 11px; }
.subtitle-original { flex: 1; white-space: pre-wrap; overflow-wrap: anywhere; margin: 0; min-height: 68px; padding: 10px; border: 1px solid var(--border); border-radius: 12px; color: var(--muted-foreground); background: #0f172a80; }
.subtitle-columns textarea { flex: 1; min-height: 68px; padding: 10px; border-radius: 12px; font-size: 12px; line-height: 1.65; }
.subtitle-change { padding: 2px 8px; border: 1px solid #00e5ff4d; border-radius: 6px; color: var(--info); background: #00e5ff1a; font-size: 10px; }
.subtitle-time { max-width: 100%; padding: 2px 8px; border-radius: 6px; background: var(--input-surface); font-size: 11px; overflow-wrap: anywhere; font-variant-numeric: tabular-nums; }
.subtitle-pagination { margin: 0; gap: 12px; }
.subtitle-pagination button { padding: 6px 12px; }
.subtitle-raw { padding-top: 12px; border-top: 1px solid var(--border); }
.subtitle-raw > button { padding: 4px 0; color: var(--muted-foreground); font-size: 12px; }
@media (max-width: 980px) { .subtitle-columns { grid-template-columns: minmax(0, 1fr); } .subtitle-count { flex-basis: 100%; margin-left: 0; } }
@media (max-width: 520px) { .subtitle-row { padding: 12px; } .subtitle-row-heading { gap: 8px; } }
</style>
