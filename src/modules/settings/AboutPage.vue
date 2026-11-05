<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { onBeforeRouteLeave } from 'vue-router'
import { Button } from '@/components/ui/button'
import { Progress } from '@/components/ui/progress'
import AppIcon from '../../components/AppIcon.vue'
import ConfirmAction from '../../components/ConfirmAction.vue'
import { useAppUpdates } from '../../composables/useAppUpdates'
import { currentReleaseNotes, isDesktop } from '../../services/updates'

const desktop = isDesktop()
const { version, automatic, initialized, saving, status, update, error, settingsError, checkedAt, downloaded, progress, busy,
  initialize, checkForUpdates, downloadUpdate, installUpdate, setAutomatic } = useAppUpdates()
const confirmInstall = ref(false)
const statusText = computed(() => !desktop ? '请在桌面应用中检查更新' : error.value ? '更新未完成，请重试' : ({
  idle: '检查是否有可用的新版本', checking: '正在检查更新…', current: '当前已是最新版本',
  available: '发现新版本', downloading: '正在下载并校验更新…', ready: '更新已下载，等待安装',
  installing: '正在安装更新…', restart: '更新已安装，请重启应用',
})[status.value])
const publishedDate = computed(() => {
  const date = update.value?.date && new Date(update.value.date)
  return date && !Number.isNaN(date.getTime()) ? date.toLocaleDateString('zh-CN') : ''
})
const releaseVersion = computed(() => update.value?.version || version.value)
const releaseLines = computed(() => {
  const notes = update.value ? update.value.body?.trim() : currentReleaseNotes?.trim()
  if (!notes) return undefined
  return notes.split(/\r?\n/).map(line => ({
    bullet: /^\s*[-*]\s+/.test(line),
    parts: line.replace(/^\s*[-*]\s+/, '').split(/((?:feat|fix|perf|style|refactor|docs|test|build|ci|chore|revert|sec)(?:\([\w./-]+\))?!?)(?=:)/g),
  }))
})
onMounted(() => { void initialize() })
onBeforeRouteLeave(() => status.value !== 'installing')
async function confirmAndInstall() { confirmInstall.value = false; await installUpdate() }
</script>

<template>
  <div class="about-page">
    <header class="about-heading">
      <p class="about-eyebrow">
        <span aria-hidden="true" />应用与版本
      </p>
      <h1>关于书引 AI</h1>
      <p class="about-description">
        让阅读，成为创作。为每一本书准备值得分享的深度多模态内容。
      </p>
    </header>
    <p
      v-if="!desktop"
      class="notice"
    >
      请在桌面应用中查看版本并更新。
    </p>
    <section
      class="panel about-identity"
      aria-labelledby="about-app-title"
    >
      <div class="about-identity-row">
        <div class="about-brand">
          <span
            class="about-symbol"
            aria-hidden="true"
          >
            <svg
              viewBox="0 0 24 24"
              fill="currentColor"
            ><path d="M12 3L2 8l10 5 10-5-10-5zm0 7.5L4.5 9 12 5.5 19.5 9 12 10.5zm-8 4.2v3.8c0 2.2 4 4 8 4s8-1.8 8-4v-3.8l-8 4-8-4z" /></svg>
          </span>
          <div class="about-brand-copy">
            <div class="about-brand-title">
              <h2 id="about-app-title">
                书引 AI
              </h2>
              <span class="about-edition">DESKTOP</span>
            </div>
            <p class="about-version">
              <span>当前版本:</span>
              <span class="about-version-number">{{ version ? `v${version}` : '—' }}</span>
              <span
                class="about-version-divider"
                aria-hidden="true"
              >·</span>
              <span class="about-version-caption">桌面创作工作台</span>
            </p>
          </div>
        </div>
        <Button
          type="button"
          variant="outline"
          class="about-check"
          :disabled="!desktop || busy || status === 'ready' || status === 'restart'"
          :aria-busy="status === 'checking'"
          @click="checkForUpdates(false)"
        >
          <AppIcon
            name="refresh"
            :class="{ 'about-checking': status === 'checking' }"
          />{{ status === 'checking' ? '检查中…' : '检查更新' }}
        </Button>
      </div>
      <div class="about-preference">
        <Button
          id="automatic-updates"
          type="button"
          variant="ghost"
          role="switch"
          class="about-switch"
          :aria-checked="automatic"
          aria-labelledby="automatic-updates-label"
          aria-describedby="automatic-updates-description"
          :disabled="!desktop || !initialized || saving || status === 'installing'"
          @click="setAutomatic(!automatic)"
        >
          <span aria-hidden="true" />
        </Button>
        <div>
          <label
            id="automatic-updates-label"
            for="automatic-updates"
          >自动检查并下载更新</label>
          <p id="automatic-updates-description">
            启动时检查新版本并在后台下载，安装前由你确认。关闭后仍可手动检查和更新。
          </p>
          <Button
            v-if="!initialized && settingsError"
            type="button"
            variant="outline"
            class="mt-3"
            @click="initialize"
          >
            重新读取设置
          </Button>
        </div>
      </div>
    </section>
    <section
      class="panel about-channel"
      aria-labelledby="update-status-title"
    >
      <div class="about-section-heading">
        <h2 id="update-status-title">
          <AppIcon name="shield" />应用更新通道
        </h2>
        <p
          class="about-status"
          :data-status="error ? 'error' : status"
          role="status"
          aria-live="polite"
          aria-atomic="true"
        >
          <span
            class="about-status-dot"
            aria-hidden="true"
          />
          <span>{{ statusText }}<template v-if="update"> · v{{ update.version }}</template></span>
        </p>
      </div>
      <p class="about-platform-description">
        应用内安装适用于 Windows 安装版、macOS 应用和 Linux AppImage。ZIP、DEB、RPM 请使用对应安装包更新。
      </p>
      <div class="about-check-details">
        <p class="about-last-check">
          <span>上次检查：</span><span>{{ checkedAt || '尚未检查' }}</span>
        </p>
        <ul
          class="about-platforms"
          aria-label="支持应用内更新的平台"
        >
          <li>Windows x64</li>
          <li>macOS Intel / Apple Silicon</li>
          <li>Linux AppImage</li>
        </ul>
      </div>
      <div
        v-if="status === 'downloading'"
        class="about-download"
      >
        <Progress
          :model-value="progress"
          aria-label="更新下载进度"
        />
        <p>
          {{ progress === undefined ? '已下载' : `${progress}% · 已下载` }} {{ (downloaded / 1024 / 1024).toFixed(1) }} MB
        </p>
      </div>
      <div
        v-if="update && ['available', 'ready', 'restart'].includes(status)"
        class="about-actions"
      >
        <Button
          v-if="status === 'available'"
          type="button"
          @click="downloadUpdate"
        >
          <AppIcon name="download" />{{ error ? '重试下载' : '下载更新' }}
        </Button>
        <Button
          v-else
          type="button"
          @click="confirmInstall = true"
        >
          <AppIcon name="refresh" />{{ status === 'restart' ? '重启应用' : '安装并重启' }}
        </Button>
      </div>
    </section>
    <ConfirmAction
      v-if="confirmInstall"
      title="确认安装并重启？"
      confirm-label="确认并重启"
      cancel-label="稍后再说"
      :destructive="false"
      @confirm="confirmAndInstall"
      @cancel="confirmInstall = false"
    >
      <p>应用将关闭并更新到 v{{ update?.version }}。请确认当前工作已保存。</p>
    </ConfirmAction>
    <section
      class="panel about-release"
      aria-labelledby="release-notes-title"
    >
      <div class="about-section-heading">
        <h2 id="release-notes-title">
          <AppIcon name="list" />更新内容
        </h2>
        <span class="about-release-version">{{ releaseVersion ? `v${releaseVersion} ` : '' }}Release Notes</span>
      </div>
      <p
        v-if="publishedDate"
        class="about-release-date"
      >
        发布时间：{{ publishedDate }}
      </p>
      <div class="about-release-log">
        <template v-if="releaseLines">
          <div
            v-for="(line, index) in releaseLines"
            :key="index"
            class="about-release-line"
          >
            <span
              v-if="line.bullet"
              class="about-release-marker"
              aria-hidden="true"
            >-</span>
            <div class="about-release-text">
              <span
                v-for="(part, partIndex) in line.parts"
                :key="partIndex"
                :class="{ 'about-commit': partIndex % 2 === 1 }"
                :data-kind="partIndex % 2 === 1 ? part : undefined"
              >{{ part }}</span>
            </div>
          </div>
        </template>
        <p
          v-else
          class="about-release-empty"
        >
          {{ update ? '此版本暂未提供更新说明。' : '此构建暂未附带更新说明。检查更新后，这里会显示新版本的更新内容。' }}
        </p>
      </div>
    </section>
  </div>
</template>

<style scoped>
.about-page { max-width: 960px; margin-inline: auto; padding-bottom: 48px; }
.about-heading { margin-bottom: 24px; }
.about-eyebrow { display: flex; align-items: center; gap: 8px; margin: 0 0 4px; color: var(--info); font: 400 12px/16px var(--font-mono); letter-spacing: 1.2px; }
.about-eyebrow > span { width: 6px; height: 6px; border-radius: 50%; background: currentColor; }
.about-description { margin: 4px 0 0; color: var(--muted-foreground); font-size: 14px; line-height: 20px; }
.about-page > .panel { margin: 0 0 24px; padding: 24px; }
.about-page > .about-identity { position: relative; isolation: isolate; overflow: hidden; padding: 28px; box-shadow: var(--panel-shadow); }
.about-identity::before { content: ''; position: absolute; z-index: -1; top: -64px; right: -64px; width: 208px; height: 208px; border-radius: 50%; background: color-mix(in srgb, var(--primary) 10%, transparent); filter: blur(64px); pointer-events: none; }
.about-identity-row, .about-brand, .about-brand-title { display: flex; align-items: center; }
.about-identity-row { justify-content: space-between; gap: 24px; padding-top: 24px; }
.about-brand { min-width: 0; gap: 20px; }
.about-symbol { display: grid; place-items: center; width: 64px; height: 64px; flex-shrink: 0; border: 2px solid transparent; border-radius: 16px; background: var(--brand-background); color: var(--brand-foreground); box-shadow: var(--primary-glow); }
.about-symbol svg { width: 36px; height: 36px; }
.about-brand-copy { min-width: 0; }
.about-brand-title { flex-wrap: wrap; gap: 10px; }
.about-brand-title h2 { margin: 0; color: var(--heading); font-size: 24px; line-height: 32px; font-weight: 900; letter-spacing: .6px; }
.about-edition { padding: 2px 8px; border: 1px solid var(--active-border); border-radius: 999px; background: color-mix(in srgb, var(--primary) 20%, transparent); color: var(--primary); box-shadow: 0 0 10px color-mix(in srgb, var(--primary) 25%, transparent); font: 700 10px/15px var(--font-mono); }
.about-version { display: flex; align-items: center; flex-wrap: wrap; gap: 8px; margin: 4px 0 0; color: var(--muted-foreground); font: 400 12px/16px var(--font-mono); }
.about-version-number { padding: 2px 8px; border: 1px solid var(--input-border); border-radius: 4px; background: var(--input-surface); color: var(--info); font-weight: 700; overflow-wrap: anywhere; }
.about-version-caption { font-size: 11px; }
.about-version-divider { color: var(--muted-foreground); }
.about-check { min-height: 40px; border-radius: 12px; font-family: var(--font-mono); box-shadow: var(--panel-shadow); }
.about-check .app-icon { color: var(--info); }
.about-check:hover:not(:disabled) { border-color: color-mix(in srgb, var(--info) 50%, transparent); }
.about-checking { animation: about-spin 1s linear infinite; }
.about-preference { display: flex; align-items: flex-start; gap: 14px; margin-top: 24px; padding-top: 16px; border-top: 1px solid var(--border); }
.about-switch { position: relative; width: 36px; height: 20px; min-height: 0; margin-top: 2px; padding: 2px; border: 0; border-radius: 999px; background: var(--input-border); }
.about-switch[aria-checked='true'] { background: var(--primary); }
.about-switch > span { position: absolute; left: 2px; top: 2px; width: 16px; height: 16px; border-radius: 50%; background: #fff; transition: transform 150ms ease; }
.about-switch[aria-checked='true'] > span { transform: translateX(16px); }
.about-preference label { margin: 0; color: var(--heading); font: 700 12px/18px var(--font-mono); cursor: pointer; }
.about-preference label:hover { color: var(--primary); }
.about-preference p { margin: 4px 0 0; color: var(--muted-foreground); font: 400 11px/18px var(--font-mono); }
.about-section-heading { display: flex; flex-wrap: wrap; align-items: center; justify-content: space-between; gap: 12px 20px; padding-bottom: 12px; border-bottom: 1px solid var(--border); }
.about-section-heading h2 { display: flex; align-items: center; gap: 8px; margin: 0; color: var(--heading); font-family: var(--font-mono); }
.about-channel h2 .app-icon { color: var(--info); }
.about-release h2 .app-icon { color: var(--primary); }
.about-status { display: flex; align-items: center; gap: 6px; margin: 0; color: var(--muted-foreground); font: 400 12px/18px var(--font-mono); }
.about-status-dot { width: 8px; height: 8px; flex-shrink: 0; border-radius: 50%; background: currentColor; }
.about-status[data-status='current'], .about-status[data-status='ready'] { color: var(--primary); }
.about-status[data-status='available'], .about-status[data-status='checking'], .about-status[data-status='downloading'] { color: var(--info); }
.about-status[data-status='installing'], .about-status[data-status='restart'] { color: var(--warning); }
.about-status[data-status='error'] { color: var(--destructive); }
.about-platform-description { margin: 16px 0; color: var(--muted-foreground); font: 400 12px/20px var(--font-mono); }
.about-check-details { display: flex; align-items: center; justify-content: space-between; flex-wrap: wrap; gap: 16px; padding: 14px; border: 1px solid var(--border); border-radius: 12px; background: color-mix(in srgb, var(--terminal) 80%, transparent); }
.about-last-check { display: flex; flex-wrap: wrap; gap: 6px 12px; margin: 0; color: var(--editor-foreground); font: 400 12px/18px var(--font-mono); }
.about-last-check > span:first-child { color: var(--muted-foreground); }
.about-platforms { display: flex; flex-wrap: wrap; gap: 8px; padding: 0; margin: 0; list-style: none; }
.about-platforms li { padding: 2px 8px; border: 1px solid var(--border); border-radius: 4px; background: var(--input-surface); color: var(--muted-foreground); font: 400 11px/16px var(--font-mono); }
.about-download, .about-actions { margin-top: 16px; }
.about-download p { margin: 8px 0 0; color: var(--muted-foreground); font: 400 12px/18px var(--font-mono); }
.about-actions { display: flex; flex-wrap: wrap; gap: 12px; }
.about-actions button { border-radius: 12px; font-family: var(--font-mono); }
.about-release-version, .about-release-date { color: var(--muted-foreground); font: 400 12px/18px var(--font-mono); overflow-wrap: anywhere; }
.about-release-date { margin: 16px 0 0; }
.about-release-log { padding: 16px; margin-top: 16px; border: 1px solid var(--border); border-radius: 12px; background: var(--terminal); color: var(--text-secondary); font: 400 12px/20px var(--font-mono); }
.about-release-line { display: flex; align-items: flex-start; gap: 10px; min-height: 20px; }
.about-release-line + .about-release-line { margin-top: 10px; padding-top: 4px; border-top: 1px solid var(--input-surface); }
.about-release-marker { flex-shrink: 0; color: var(--info); font-weight: 700; }
.about-release-line:nth-child(3n + 2) .about-release-marker { color: var(--primary); }
.about-release-line:nth-child(3n) .about-release-marker { color: var(--warning); }
.about-release-text { min-width: 0; white-space: pre-wrap; overflow-wrap: anywhere; }
.about-commit { padding: 2px 6px; border: 1px solid color-mix(in srgb, currentColor 30%, transparent); border-radius: 4px; background: color-mix(in srgb, currentColor 8%, transparent); color: var(--info); font-size: 11px; box-decoration-break: clone; -webkit-box-decoration-break: clone; }
.about-commit[data-kind^='test'], .about-commit[data-kind^='fix(test'] { color: var(--video-accent); }
.about-commit[data-kind^='perf'], .about-commit[data-kind^='feat'] { color: var(--primary); }
.about-commit[data-kind^='sec'] { color: var(--warning); }
.about-release-empty { margin: 0; color: var(--muted-foreground); }
@keyframes about-spin { to { transform: rotate(360deg); } }
@media (max-width: 1023px) { .about-identity-row { flex-wrap: wrap; } }
@media (max-width: 639px) {
  .about-page > .panel { padding: 20px; }
  .about-identity-row { align-items: stretch; flex-direction: column; }
  .about-brand { gap: 16px; }
  .about-brand-title h2 { font-size: 22px; }
  .about-version-divider { display: none; }
  .about-version-caption { flex-basis: 100%; }
  .about-check { width: 100%; }
  .about-release-log { padding: 12px; }
}
@media (prefers-reduced-motion: reduce) { .about-checking { animation: none; } .about-switch > span { transition: none; } }
@media (prefers-reduced-transparency: reduce) { .about-identity::before { display: none; } }
</style>
