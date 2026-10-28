<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { onBeforeRouteLeave } from 'vue-router'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import AppIcon from '../../components/AppIcon.vue'
import ConfirmAction from '../../components/ConfirmAction.vue'
import appLogo from '../../../assets/app-icon.svg'
import { useAppUpdates } from '../../composables/useAppUpdates'
import { currentReleaseNotes, isDesktop } from '../../services/updates'

const desktop = isDesktop()
const { version, automatic, initialized, saving, status, update, error, settingsError, checkedAt, downloaded, progress, busy,
  initialize, checkForUpdates, downloadUpdate, installUpdate, setAutomatic } = useAppUpdates()
const confirmInstall = ref(false)
const statusText = computed(() => ({
  idle: '检查是否有可用的新版本', checking: '正在检查更新…', current: '当前已是最新版本',
  available: '发现新版本', downloading: '正在下载并校验更新…', ready: '更新已下载，等待安装',
  installing: '正在安装更新…', restart: '更新已安装，请重启应用',
})[status.value])
const publishedDate = computed(() => {
  const date = update.value?.date && new Date(update.value.date)
  return date && !Number.isNaN(date.getTime()) ? date.toLocaleDateString('zh-CN') : ''
})
onMounted(() => { void initialize() })
onBeforeRouteLeave(() => status.value !== 'installing')
async function confirmAndInstall() { confirmInstall.value = false; await installUpdate() }
</script>

<template>
  <header class="page-heading">
    <div>
      <p class="eyebrow">
        应用与版本
      </p>
      <h1>关于书引 AI</h1>
      <p class="muted">
        让阅读，成为创作。为每一本书准备值得分享的内容。
      </p>
    </div>
  </header>
  <p
    v-if="!desktop"
    class="notice"
  >
    请在桌面应用中查看版本并更新。
  </p>
  <section class="panel">
    <div class="flex flex-wrap items-center gap-5">
      <img
        :src="appLogo"
        alt=""
        width="64"
        height="64"
        class="size-16"
      >
      <div class="min-w-0 flex-1">
        <h2 class="!mb-1 !mt-0">
          书引 AI
        </h2>
        <p class="muted !my-0">
          当前版本：{{ version ? `v${version}` : '—' }}
        </p>
      </div>
      <Button
        type="button"
        variant="outline"
        :disabled="!desktop || busy || status === 'ready' || status === 'restart'"
        @click="checkForUpdates(false)"
      >
        <AppIcon name="refresh" />{{ status === 'checking' ? '检查中…' : '检查更新' }}
      </Button>
    </div>
    <div class="mt-6 border-t pt-5">
      <label class="flex items-center gap-2 text-sm font-medium">
        <Checkbox
          :model-value="automatic"
          :disabled="!desktop || !initialized || saving || status === 'installing'"
          @update:model-value="setAutomatic($event === true)"
        />
        自动检查并下载更新
      </label>
      <p class="muted !mb-0">
        启动时检查新版本并在后台下载，安装前由你确认。关闭后仍可手动检查和更新。
      </p>

      <Button
        v-if="!initialized && settingsError"
        type="button"
        variant="outline"
        @click="initialize"
      >
        重新读取设置
      </Button>
    </div>
  </section>
  <section
    class="panel"
    aria-labelledby="update-status-title"
  >
    <h2 id="update-status-title">
      应用更新
    </h2>
    <p class="muted">
      应用内安装适用于 Windows 安装版、macOS 应用和 Linux AppImage。ZIP、DEB、RPM 请使用对应安装包更新。
    </p>
    <p
      role="status"
      aria-live="polite"
    >
      {{ statusText }}<span v-if="update"> · v{{ update.version }}</span>
    </p>
    <p
      v-if="checkedAt"
      class="muted"
    >
      上次检查：{{ checkedAt }}
    </p>

    <div
      v-if="status === 'downloading'"
      class="my-4"
    >
      <progress
        :value="progress"
        max="100"
        aria-label="更新下载进度"
        class="h-2 w-full accent-primary"
      />
      <p class="muted">
        {{ progress === undefined ? '已下载' : `${progress}% · 已下载` }} {{ (downloaded / 1024 / 1024).toFixed(1) }} MB
      </p>
    </div>
    <div
      v-if="update"
      class="actions wrap"
    >
      <Button
        v-if="status === 'available'"
        type="button"
        @click="downloadUpdate"
      >
        <AppIcon name="download" />{{ error ? '重试下载' : '下载更新' }}
      </Button>
      <Button
        v-if="status === 'ready' || status === 'restart'"
        type="button"
        @click="confirmInstall = true"
      >
        {{ status === 'restart' ? '重启应用' : '安装并重启' }}
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
    class="panel"
    aria-labelledby="release-notes-title"
  >
    <h2 id="release-notes-title">
      更新内容<span v-if="update"> · v{{ update.version }}</span>
    </h2>
    <p
      v-if="publishedDate"
      class="muted"
    >
      发布时间：{{ publishedDate }}
    </p>
    <div
      v-if="!update && currentReleaseNotes"
      class="whitespace-pre-wrap break-words text-sm leading-7"
    >
      {{ currentReleaseNotes }}
    </div>
    <p
      v-else-if="!update"
      class="muted"
    >
      此构建暂未附带更新说明。检查更新后，这里会显示新版本的更新内容。
    </p>
    <div
      v-else
      class="whitespace-pre-wrap break-words text-sm leading-7"
    >
      {{ update.body?.trim() || '此版本暂未提供更新说明。' }}
    </div>
  </section>
</template>
