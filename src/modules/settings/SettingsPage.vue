<script setup lang="ts">
import OperationFeedback from '../../components/OperationFeedback.vue'
import ConfirmAction from '../../components/ConfirmAction.vue'
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '@/components/ui/collapsible'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { computed, onMounted, ref } from 'vue'
import { onBeforeRouteLeave } from 'vue-router'
import * as api from '../../services/projects'
import * as fullBackup from '../../services/backup'
import type { WorkspaceSettings } from '../../types/project'
import { useOperation } from '../../composables/useOperation'
import { useWindowCloseGuard } from '../../composables/useWindowCloseGuard'
import { workspaceOperation } from './workspaceOperation'
const desktop = api.isDesktop()
const settings = ref<WorkspaceSettings | null>(null)
const restorePath = ref('')
const includeVideos = ref(false)
const fullRestore = ref<fullBackup.BackupSummary | null>(null)
const restoreDirectory = ref('')
const { busy: operationBusy, error, message, operation, run } = workspaceOperation
const loading = useOperation()
const busy = computed(() => operationBusy.value || loading.busy.value)
const closeGuard = useWindowCloseGuard(() => operationBusy.value, () => !operationBusy.value)
onBeforeRouteLeave(() => !operationBusy.value)
const load = () => loading.run(async () => { settings.value = await api.workspaceSettings() })
const chooseRoot = () => run(async () => { const path = await api.chooseDirectory(); if (path) { settings.value = await api.setWorkspaceRoot(path); message.value = '新项目将使用此目录，已有项目保持原位置。' } }, '正在设置工作目录…')
const backup = () => run(async () => { const path = await api.chooseDirectory(); if (path) message.value = `备份已保存：${await api.createBackup(path)}` }, '正在创建数据库备份，请等待完成后离开…')
const chooseRestore = () => run(async () => { const path = await api.chooseBackup(); if (path) { restorePath.value = path; fullRestore.value = null; restoreDirectory.value = '' } })
const backupAll = () => run(async () => {
  const directory = await api.chooseDirectory()
  if (!directory) return
  const result = await fullBackup.createFullBackup(directory, includeVideos.value)
  message.value = `完整备份已保存（${result.fileCount} 个文件）：${result.manifestPath}。请保留整个备份目录。`
}, '正在创建完整备份，请等待完成后离开…')
const chooseFullRestore = () => run(async () => {
  const path = await fullBackup.chooseFullBackup()
  if (!path) return
  const preview = await fullBackup.inspectFullBackup(path)
  fullRestore.value = preview; restoreDirectory.value = ''; restorePath.value = ''
}, '正在校验完整备份…')
const chooseRestoreDirectory = () => run(async () => { const directory = await api.chooseDirectory(); if (directory) restoreDirectory.value = directory })
const restoreAll = () => run(async () => {
  if (!fullRestore.value || !restoreDirectory.value) return
  const result = await fullBackup.restoreFullBackup(fullRestore.value.manifestPath, restoreDirectory.value)
  fullRestore.value = null
  message.value = `完整恢复完成，资料已放到：${result.workspacePath}。恢复前数据库：${result.previousDatabase}。请到模型设置检查密钥。`
  settings.value = await api.workspaceSettings()
}, '正在恢复完整备份，请等待完成后离开…')
const restore = () => run(async () => {
  const previous = await api.restoreBackup(restorePath.value)
  restorePath.value = ''
  message.value = `恢复完成。恢复前的数据已备份到：${previous}`
  settings.value = await api.workspaceSettings()
}, '正在恢复数据库，请等待完成后离开…')
onMounted(() => { if (desktop) void load() })
</script>
<template>
  <header class="page-heading">
    <div>
      <p class="eyebrow">
        本地数据与文件
      </p><h1>工作目录与备份</h1><p class="muted">
        将项目资料保存在本机，定期备份资料与素材。
      </p>
    </div>
  </header>
  <p
    v-if="!desktop"
    class="notice"
  >
    请在桌面应用中管理目录和备份。
  </p>
  <OperationFeedback
    :error="error"
    :message="message"
  />
  <OperationFeedback :error="loading.error.value || closeGuard.error.value" />
  <section
    v-if="operationBusy || message || error"
    class="panel workspace-result"
    aria-label="最近一次数据操作"
    aria-live="polite"
  >
    <h2>{{ operationBusy ? '数据操作进行中' : error ? '操作未完成' : '最近一次操作结果' }}</h2>
    <p
      v-if="operationBusy"
      role="status"
    >
      {{ operation }}
    </p>
    <p
      v-else
      :class="error ? 'error' : 'file-path'"
    >
      {{ error || message }}
    </p>
    <p
      v-if="operationBusy"
      class="muted"
    >
      完成后可离开此页，返回仍可查看本次结果。
    </p>
  </section>
  <section class="panel">
    <h2>默认工作目录</h2><p class="muted">
      新项目会在此目录下创建独立文件夹。修改默认目录不会移动已有项目。
    </p><p
      v-if="settings"
      class="file-path"
    >
      {{ settings.root }}
    </p><div class="actions">
      <Button
        :disabled="!desktop || busy"
        type="button"
        @click="chooseRoot"
      >
        选择工作目录
      </Button><Button
        :disabled="!desktop || busy"
        variant="outline"
        type="button"
        @click="load"
      >
        重新读取
      </Button>
    </div><dl v-if="settings">
      <dt>数据库位置</dt><dd>{{ settings.databasePath }}</dd>
    </dl>
  </section>
  <section class="panel backup-primary">
    <h2>完整备份与恢复</h2>
    <p class="muted">
      备份数据库、自动草稿、个人样式、已关联图片和文档、已导出封面以及已登记任务包。备份是一个目录，请整体保管或搬移；不包含 API Key。文件缺失或校验失败时明确中止。
    </p>
    <label class="my-4 flex items-center gap-2 text-sm">
      <Checkbox
        v-model="includeVideos"
        :disabled="!desktop || busy"
      />同时包含已关联的原始视频（可能较大）
    </label>
    <div class="actions wrap">
      <Button
        type="button"
        :disabled="!desktop || busy"
        @click="backupAll"
      >
        创建完整备份
      </Button>
      <Button
        type="button"
        variant="outline"
        :disabled="!desktop || busy"
        @click="chooseFullRestore"
      >
        选择完整备份恢复
      </Button>
    </div>
    <p class="muted">
      不勾选视频时只保留原路径，换电脑后可能需要重新定位。未登记到应用的文件不在备份范围。
    </p>
  </section>
  <ConfirmAction
    v-if="fullRestore"
    title="确认恢复完整备份？"
    confirm-label="确认完整恢复"
    :busy="operationBusy"
    :disabled="!restoreDirectory"
    @confirm="restoreAll"
    @cancel="fullRestore = null"
  >
    <p class="file-path">
      {{ fullRestore.manifestPath }}
    </p>
    <p>{{ fullRestore.fileCount }} 个文件 · {{ (fullRestore.totalBytes / 1024 / 1024).toFixed(1) }} MB · 未包含 {{ fullRestore.omittedVideos }} 个视频。</p>
    <p>将替换当前数据库，并在所选目录内新建恢复文件夹、重建素材和任务包引用；现有文件保留，替换前自动备份当前数据库。密钥需在模型设置核验。</p>
    <p
      v-if="restoreDirectory"
      class="file-path"
    >
      恢复位置：{{ restoreDirectory }}
    </p>
    <template #actions>
      <Button
        type="button"
        variant="outline"
        :disabled="busy"
        @click="chooseRestoreDirectory"
      >
        选择恢复目录
      </Button>
    </template>
  </ConfirmAction>
  <Collapsible
    class="history-panel"
    :unmount-on-hide="false"
  >
    <CollapsibleTrigger class="disclosure-trigger">
      高级维护 · 仅备份或恢复数据库
    </CollapsibleTrigger>
    <CollapsibleContent>
      <section class="mt-5">
        <h2>数据库备份与恢复</h2><p class="muted">
          备份包含项目资料、创作版本、模型配置、素材路径和普通设置，不包含 API Key，也不复制视频、图片或工作目录内的文件。恢复后请在模型设置中检查并重新输入缺失的密钥。
        </p><div class="actions">
          <Button
            :disabled="!desktop || busy"
            type="button"
            @click="backup"
          >
            创建备份
          </Button><Button
            :disabled="!desktop || busy"
            variant="outline"
            type="button"
            @click="chooseRestore"
          >
            选择备份恢复
          </Button>
        </div>
      </section>
      <ConfirmAction
        v-if="restorePath"
        title="确认替换当前项目数据？"
        confirm-label="确认恢复"
        :busy="operationBusy"
        @confirm="restore"
        @cancel="restorePath = ''"
      >
        <p class="file-path">
          {{ restorePath }}
        </p><p>恢复将替换全部项目、创作版本、模型配置、任务、素材关联和普通设置。系统会先校验备份并自动保存当前数据库；原始素材文件保持原位，失效路径可在项目内重新定位。密钥不随备份恢复。</p>
      </ConfirmAction>
    </CollapsibleContent>
  </Collapsible>
</template>
