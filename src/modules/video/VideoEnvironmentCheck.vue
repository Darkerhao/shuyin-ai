<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { Button } from '@/components/ui/button'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '@/components/ui/collapsible'
import { checkEnvironment } from '../../services/video'
import { errorMessage } from '../../services/verification'
import type { VideoEnvironmentItem, VideoEnvironmentReport, VideoSettings } from '../../types/video'

const props = defineProps<{ settings: VideoSettings; hasSubtitles: boolean; disabled: boolean }>()
const emit = defineEmits<{ working: [value: boolean]; apply: [settings: VideoSettings]; choose: [key: keyof VideoSettings] }>()
const mode = ref('transcription')
const report = ref<VideoEnvironmentReport>()
const error = ref('')
const checking = ref(false)
const labels: Record<VideoEnvironmentItem['id'], string> = { node: 'Node 程序', engine: 'Frametell 引擎', browser: 'Chrome / Edge', ffmpeg: 'FFmpeg', fonts: '视频字体', python: 'Python 语音环境', model: '本地语音模型' }
const fields: Partial<Record<VideoEnvironmentItem['id'], keyof VideoSettings>> = { node: 'nodePath', engine: 'engineRoot', python: 'pythonPath', model: 'modelPath' }
const failures = computed(() => report.value?.items.filter(item => item.status === 'error').length ?? 0)
const canApply = computed(() => report.value && Object.entries(report.value.settings).some(([key, value]) => !props.settings[key as keyof VideoSettings] && value))
watch(() => props.hasSubtitles, value => { mode.value = value ? 'existing' : 'transcription' }, { immediate: true })
watch([() => props.settings, mode], () => { report.value = undefined; error.value = '' }, { deep: true })
async function check() {
  if (checking.value || props.disabled) return
  checking.value = true; emit('working', true); error.value = ''; report.value = undefined
  try { report.value = await checkEnvironment({ ...props.settings }, mode.value === 'transcription') }
  catch (cause) { error.value = errorMessage(cause) }
  finally { checking.value = false; emit('working', false) }
}
function apply() {
  if (!report.value) return
  const settings = { ...props.settings }
  for (const key of Object.keys(settings) as (keyof VideoSettings)[]) if (!settings[key]) settings[key] = report.value.settings[key]
  emit('apply', settings)
}
</script>

<template>
  <section
    class="environment-check"
    aria-label="视频环境检查"
  >
    <div class="actions">
      <label for="environment-mode">检查用途</label>
      <Select
        v-model="mode"
        :disabled="disabled || checking"
      >
        <SelectTrigger
          id="environment-mode"
          class="environment-mode"
        >
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          <SelectItem value="transcription">
            从原声新转写字幕
          </SelectItem>
          <SelectItem value="existing">
            使用已有 SRT / 已保存字幕
          </SelectItem>
        </SelectContent>
      </Select>
      <Button
        variant="outline"
        :disabled="disabled || checking"
        @click="check"
      >
        {{ checking ? '正在检查环境…' : '一键检查环境' }}
      </Button>
      <Button
        v-if="canApply"
        variant="outline"
        :disabled="disabled || checking"
        @click="apply"
      >
        填入识别到的路径
      </Button>
    </div>
    <p class="muted">
      检查当前填写的路径；空项会查找 PATH 和工作目录附近的已有安装。检查不下载模型、不调用 AI，也不自动保存配置。检查可能需要约一分钟。
    </p>
    <p
      v-if="error"
      role="alert"
      class="notice"
    >
      {{ error }} 请修正后重新检查。
    </p>
    <p
      role="status"
      aria-live="polite"
      class="muted"
    >
      {{ checking ? '正在启动运行环境并验证依赖，请稍候。' : report ? failures ? `有 ${failures} 项需要处理，请按下方提示修复。` : '本次用途所需环境检查通过。修改配置后请重新检查并保存。' : '' }}
    </p>
    <ul
      v-if="report"
      class="environment-results"
    >
      <li
        v-for="item in report.items"
        :key="item.id"
      >
        <div class="actions">
          <strong>{{ labels[item.id] }}</strong><span>{{ { ok: '可用', error: '需处理', skipped: '不需要' }[item.status] }}</span><span
            v-if="item.version"
            class="muted"
          >{{ item.version }}</span>
        </div>
        <p>{{ item.detail }}</p><p
          v-if="item.path"
          class="muted environment-path"
        >
          {{ item.path }}
        </p>
        <Button
          v-if="item.status === 'error' && fields[item.id]"
          variant="outline"
          :disabled="disabled"
          @click="emit('choose', fields[item.id]!)"
        >
          重新选择{{ labels[item.id] }}
        </Button>
        <Collapsible
          v-if="item.status === 'error'"
          class="environment-help"
        >
          <CollapsibleTrigger class="disclosure-trigger">
            查看{{ labels[item.id] }}修复说明
          </CollapsibleTrigger>
          <CollapsibleContent>
            <p v-if="item.id === 'node'">
              安装 Node.js 24 LTS（至少24.13.1，推荐24.22.2），选择安装目录中的 node.exe。旧版24.11在中文路径下存在崩溃问题。
            </p>
            <p v-else-if="item.id === 'engine'">
              选择独立 whiteboard-teacher-video 工程内的 engine 目录，版本须为0.7.3。确认 package.json、tools 和 runtime 齐全，并在独立工程中安装其锁定依赖。
            </p>
            <p v-else-if="item.id === 'browser'">
              安装 Chrome 或 Edge；若设置过 CHROME_PATH，请确认指向可启动的浏览器。缺少 playwright-core 时，在独立引擎工程中安装锁定依赖。
            </p>
            <p v-else-if="item.id === 'ffmpeg'">
              在独立引擎工程中安装其锁定依赖（包含 ffmpeg-static）。若设置过 FFMPEG_PATH，请确认指向可启动的 ffmpeg.exe，然后重开书引再检查。
            </p>
            <p v-else-if="item.id === 'fonts'">
              在独立引擎工程中重新安装字体依赖：lxgw-wenkai-webfont、@fontsource/caveat、@fontsource/jetbrains-mono、@fontsource/fredoka。修复浏览器启动问题后再检查字体加载。
            </p>
            <p v-else-if="item.id === 'python'">
              选择已安装 faster-whisper==1.2.1、av==16.1.0 的虚拟环境内 python.exe。源码用户可按 scripts/setup-video.ps1 的说明，在选定目录准备运行环境；已有SRT可改为“使用已有字幕”检查。
            </p>
            <p v-else>
              选择多语言 small 模型目录，须包含 model.bin、config.json、tokenizer.json、vocabulary.txt。使用 setup-video.ps1 下载完整模型后重新选择；检查过程不会联网下载。
            </p>
          </CollapsibleContent>
        </Collapsible>
      </li>
    </ul>
  </section>
</template>

<style scoped>
.environment-check { display: grid; gap: .75rem; min-width: 0; }
.environment-mode { width: min(100%, 280px); }
.environment-results { display: grid; gap: .75rem; padding: 0; list-style: none; }
.environment-results li { display: grid; gap: .5rem; padding: 1rem; border: 1px solid var(--border); border-radius: .75rem; min-width: 0; }
.environment-results p { overflow-wrap: anywhere; }
.environment-path { font-size: .8rem; }
</style>
