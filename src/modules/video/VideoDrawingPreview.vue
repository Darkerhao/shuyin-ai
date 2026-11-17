<script setup lang="ts">
import { computed, ref, watchEffect } from 'vue'
import type { VideoBeat, VideoScene, VideoCue, PencilCharacter } from '../../types/video'
import '../../../src-tauri/video/pencil-drawing.js'
const props = defineProps<{ beats: VideoBeat[]; title: string; scene?: VideoScene; cues?: VideoCue[]; characters?: PencilCharacter[] }>()
const strokes = computed(() => props.beats.flatMap(beat => beat.drawing?.strokes ?? []))
const canvas = ref<globalThis.HTMLCanvasElement>()
const error = ref('')
watchEffect(() => {
  if (!canvas.value || !props.scene?.artwork || !props.cues?.length) return
  try {
    const scene = props.scene
    const start = props.cues[scene.first]!.start
    const end = props.cues[scene.last + 1]?.start ?? props.cues[scene.last]!.end
    const drawing = globalThis.SHUYIN_PENCIL_DRAWING.compile(scene.artwork!, props.characters ?? [], props.cues.map((c, index) => ({ ...c, index })), start, end - start, scene.first, scene.last)
    globalThis.SHUYIN_PENCIL_DRAWING.render(canvas.value, drawing, end - start, false)
    error.value = ''
  } catch (cause) { error.value = cause instanceof Error ? cause.message : '画稿预览失败，请重新读取。' }
})
</script>

<template>
  <canvas
    v-if="scene?.artwork"
    ref="canvas"
    width="1600"
    height="900"
    class="drawing-preview"
    role="img"
    :aria-label="`${title}精绘场景预览`"
  />
  <p
    v-if="error"
    role="alert"
  >
    {{ error }}
  </p>
  <svg
    v-if="!scene?.artwork && strokes.length"
    viewBox="0 0 1000 300"
    class="drawing-preview"
    role="img"
    :aria-label="`${title}完整场景笔画预览`"
  >
    <polyline
      v-for="(stroke, index) in strokes"
      :key="index"
      :points="stroke.map(point => point.join(',')).join(' ')"
    />
  </svg>
</template>

<style scoped>
.drawing-preview { width: 100%; background: #fbfaf7; border: 1px solid var(--border); border-radius: 8px; }
.drawing-preview polyline { fill: none; stroke: #414348; stroke-width: 1.2; stroke-linecap: round; stroke-linejoin: round; }
</style>
