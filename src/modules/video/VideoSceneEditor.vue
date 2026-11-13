<script setup lang="ts">
import VideoImageEditor from './VideoImageEditor.vue'
import AppIcon from '../../components/AppIcon.vue'
import { Input } from '@/components/ui/input'
import { Button } from '@/components/ui/button'
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '@/components/ui/collapsible'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import type { Asset } from '../../types/project'
import type { ModelProvider } from '../../types/ai'
import { visualCatalog, type VideoCue, type VideoScene, type VideoSceneImage } from '../../types/video'

defineProps<{
  projectId: string; chapterId?: string | null; sceneIndex: number; cues: VideoCue[]
  assets: Asset[]; providers: ModelProvider[]; disabled: boolean; imageDisabled: boolean
  revisionDisabled: boolean
  playing?: boolean; audioAvailable?: boolean
}>()
const scene = defineModel<VideoScene>('scene', { required: true })
const emit = defineEmits<{ 'refresh-assets': []; working: [value: boolean]; revise: []; play: [] }>()
const sceneKindLabel = (kind: VideoScene['kind']) => Object.hasOwn(visualCatalog.kinds, kind) ? visualCatalog.kinds[kind as keyof typeof visualCatalog.kinds] : '旧版文字图解'
function setSceneImage(image: VideoSceneImage | undefined) {
  if (image) { scene.value.image = image; scene.value.beats = [] }
  else { delete scene.value.image; scene.value.beats = [{ cue: scene.value.first, object: 'book', action: 'draw', label: '书籍' }] }
}
</script>

<template>
  <article
    :id="`video-scene-${sceneIndex}`"
    class="video-scene"
    :class="{ 'video-scene-playing': playing }"
  >
    <div class="video-scene-actions">
      <Button
        variant="ghost"
        size="sm"
        :disabled="disabled || !audioAvailable"
        :aria-label="`试听第${sceneIndex + 1}镜`"
        @click="emit('play')"
      >
        <AppIcon name="play" />{{ playing ? '正在播放这一镜' : '试听这一镜' }}
      </Button>
      <Button
        variant="outline"
        :disabled="disabled || revisionDisabled"
        :aria-label="`重新生成第${sceneIndex + 1}镜`"
        @click="emit('revise')"
      >
        <AppIcon name="refresh" />重新生成这一镜
      </Button>
    </div>
    <p class="muted">
      第 {{ sceneIndex + 1 }} 镜 · 字幕 {{ scene.first + 1 }}—{{ scene.last + 1 }} · {{ sceneKindLabel(scene.kind) }}
    </p><Input
      v-model="scene.title"
      class="video-scene-title"
      :disabled="disabled || (!scene.beats?.length && !scene.image)"
      :aria-label="`第${sceneIndex + 1}镜标题`"
      maxlength="20"
    />
    <div
      v-if="scene.beats?.length"
      class="video-scene-controls"
    >
      <div class="video-field video-scene-kind">
        <label :for="`scene-${sceneIndex}-kind`">叙事构图</label>
        <Select
          v-model="scene.kind"
          :disabled="disabled"
        >
          <SelectTrigger
            :id="`scene-${sceneIndex}-kind`"
            :aria-label="`第${sceneIndex + 1}镜构图`"
          >
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem
              v-for="(name, key) in visualCatalog.kinds"
              :key="key"
              :value="key"
            >
              {{ name }}
            </SelectItem>
          </SelectContent>
        </Select>
      </div>
      <div class="video-field">
        <label :for="`scene-${sceneIndex}-environment`">环境场景</label>
        <Select
          :model-value="scene.environment ?? 'plain'"
          :disabled="disabled"
          @update:model-value="scene.environment = $event as NonNullable<VideoScene['environment']>"
        >
          <SelectTrigger
            :id="`scene-${sceneIndex}-environment`"
            :aria-label="`第${sceneIndex + 1}镜场景`"
          >
            <SelectValue>{{ visualCatalog.environments[scene.environment ?? 'plain'] }}</SelectValue>
          </SelectTrigger>
          <SelectContent>
            <SelectItem
              v-for="(name, key) in visualCatalog.environments"
              :key="key"
              :value="key"
            >
              {{ name }}
            </SelectItem>
          </SelectContent>
        </Select>
      </div>
      <div class="video-field">
        <label :for="`scene-${sceneIndex}-framing`">画面景别</label>
        <Select
          :model-value="scene.framing ?? 'reference'"
          :disabled="disabled"
          @update:model-value="scene.framing = $event as NonNullable<VideoScene['framing']>"
        >
          <SelectTrigger
            :id="`scene-${sceneIndex}-framing`"
            :aria-label="`第${sceneIndex + 1}镜景别`"
          >
            <SelectValue>{{ visualCatalog.framings[scene.framing ?? 'reference'] }}</SelectValue>
          </SelectTrigger>
          <SelectContent>
            <SelectItem
              v-for="(name, key) in visualCatalog.framings"
              :key="key"
              :value="key"
            >
              {{ name }}
            </SelectItem>
          </SelectContent>
        </Select>
      </div>
    </div>
    <VideoImageEditor
      :project-id="projectId"
      :chapter-id="chapterId"
      :scene-index="sceneIndex"
      :image="scene.image"
      :assets="assets"
      :providers="providers"
      :disabled="imageDisabled"
      @update:image="setSceneImage($event)"
      @refresh-assets="emit('refresh-assets')"
      @working="emit('working', $event)"
    />
    <div
      v-for="(beat, i) in scene.beats"
      :key="i"
      class="video-beat"
    >
      <label :for="`scene-${sceneIndex}-beat-${i}`">字幕 {{ beat.cue + 1 }} · {{ visualCatalog.objects[beat.object] }} · {{ visualCatalog.actions[beat.action] }}</label>
      <Input
        :id="`scene-${sceneIndex}-beat-${i}`"
        v-model="beat.label"
        :disabled="disabled"
        :aria-label="`第${sceneIndex + 1}镜物体${i + 1}标签`"
        maxlength="8"
      />
      <Collapsible class="video-beat-settings">
        <CollapsibleTrigger class="disclosure-trigger py-1.5">
          调整物体、动作与字幕锚点
        </CollapsibleTrigger>
        <CollapsibleContent>
          <div class="video-beat-controls">
            <div class="video-field">
              <label :for="`scene-${sceneIndex}-object-${i}`">具体物体</label>
              <Select
                v-model="beat.object"
                :disabled="disabled"
              >
                <SelectTrigger
                  :id="`scene-${sceneIndex}-object-${i}`"
                  :aria-label="`第${sceneIndex + 1}镜物体${i + 1}`"
                >
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem
                    v-for="(name, key) in visualCatalog.objects"
                    :key="key"
                    :value="key"
                  >
                    {{ name }}
                  </SelectItem>
                </SelectContent>
              </Select>
            </div>
            <div class="video-field">
              <label :for="`scene-${sceneIndex}-action-${i}`">语义动作</label>
              <Select
                v-model="beat.action"
                :disabled="disabled"
              >
                <SelectTrigger
                  :id="`scene-${sceneIndex}-action-${i}`"
                  :aria-label="`第${sceneIndex + 1}镜物体${i + 1}动作`"
                >
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem
                    v-for="(name, key) in visualCatalog.actions"
                    :key="key"
                    :value="key"
                  >
                    {{ name }}
                  </SelectItem>
                </SelectContent>
              </Select>
            </div>
            <div class="video-field video-anchor">
              <label :for="`scene-${sceneIndex}-cue-${i}`">在哪句开始出现</label>
              <Select
                :model-value="String(beat.cue)"
                :disabled="disabled"
                @update:model-value="beat.cue = Number($event)"
              >
                <SelectTrigger
                  :id="`scene-${sceneIndex}-cue-${i}`"
                  :aria-label="`第${sceneIndex + 1}镜物体${i + 1}字幕锚点`"
                >
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem
                    v-for="(cue, offset) in cues.slice(scene.first, scene.last + 1)"
                    :key="offset"
                    :value="String(scene.first + offset)"
                  >
                    {{ scene.first + offset + 1 }} · {{ cue.text }}
                  </SelectItem>
                </SelectContent>
              </Select>
            </div>
          </div>
        </CollapsibleContent>
      </Collapsible>
    </div>
    <p
      v-if="!scene.beats?.length"
      class="muted"
    >
      {{ scene.labels?.join(' · ') }}
    </p>
  </article>
</template>

<style scoped>
.video-scene { display: grid; align-content: start; gap: 14px; padding: 20px; border: 1px solid var(--border); border-radius: 10px; background: var(--card); scroll-margin-block: 16px; min-width: 0; font: 400 14px/1.75 var(--font-sans); }
.video-scene-playing { background: color-mix(in srgb, var(--info) 5%, transparent); border-color: var(--info); }
.video-scene-actions { display: flex; justify-content: space-between; align-items: center; flex-wrap: wrap; gap: 8px; }
.video-scene-actions button { min-height: 32px; padding: 4px 8px; font-size: 12px; }
.video-scene-actions button:first-child { padding-left: 0; color: var(--info); font-weight: 700; }
.video-scene-actions button:last-child { color: var(--text-secondary); }
.video-scene > .muted { margin: 0 0 -6px; font-size: 12px; }
.video-scene-title { min-height: 42px; padding: 12px; color: var(--foreground); font-weight: 700; }
.video-scene-controls { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 12px; }
.video-scene-kind { grid-column: 1 / -1; }
.video-scene-kind :deep([role='combobox']) { border-color: var(--video-border); color: var(--video-accent); }
.video-scene :deep(.scene-image-editor > button) { justify-content: flex-start; height: auto; padding: 4px 0; color: var(--info); font-size: 12px; }
.video-beat { display: grid; gap: 6px; min-width: 0; padding-top: 12px; border-top: 1px solid var(--border); }
.video-beat label { color: var(--muted-foreground); font-size: 12px; overflow-wrap: anywhere; }
.video-beat input { min-height: 36px; font-size: 12px; }
.video-beat-settings > button { color: var(--info); font-size: 12px; font-weight: 400; }
.video-beat-controls { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 12px; padding-block: 10px; }
.video-anchor { grid-column: 1 / -1; }
.video-field { display: grid; align-content: start; gap: 6px; min-width: 0; }
.video-field label { color: var(--muted-foreground); font-size: 12px; }
.video-field [role='combobox'] { width: 100%; min-width: 0; min-height: 34px; height: auto; font-size: 12px; }
.video-field .muted { margin: 4px 0 0; }
@media (max-width: 520px) { .video-scene { padding: 12px; } .video-scene-controls, .video-beat-controls { grid-template-columns: minmax(0, 1fr); } }
</style>
