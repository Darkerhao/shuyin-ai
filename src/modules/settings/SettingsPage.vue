<script setup lang="ts">
import { Button } from '@/components/ui/button'
import { onMounted, ref } from 'vue'
import * as api from '../../services/projects'
import type { WorkspaceSettings } from '../../types/project'
import { useOperation } from '../../composables/useOperation'
const desktop = api.isDesktop()
const settings = ref<WorkspaceSettings | null>(null)
const restorePath = ref('')
const { busy, error, message, run } = useOperation()
const load = () => run(async () => { settings.value = await api.workspaceSettings() })
const chooseRoot = () => run(async () => { const path = await api.chooseDirectory(); if (path) { settings.value = await api.setWorkspaceRoot(path); message.value = '新项目将使用此目录，已有项目保持原位置。' } })
const backup = () => run(async () => { const path = await api.chooseDirectory(); if (path) message.value = `备份已保存：${await api.createBackup(path)}` })
const chooseRestore = () => run(async () => { const path = await api.chooseBackup(); if (path) restorePath.value = path })
const restore = () => run(async () => {
  const previous = await api.restoreBackup(restorePath.value)
  restorePath.value = ''
  message.value = `恢复完成。恢复前的数据已备份到：${previous}`
  settings.value = await api.workspaceSettings()
})
onMounted(() => { if (desktop) void load() })
</script>
<template>
  <header>
    <p class="eyebrow">
      本地数据与文件
    </p><h1>工作目录与备份</h1><p class="muted">
      将项目资料保存在本机，定期备份数据库。
    </p>
  </header>
  <p
    v-if="!desktop"
    class="notice"
  >
    请在桌面应用中管理目录和备份。
  </p>
  <p
    v-if="error"
    class="error"
    role="alert"
  >
    {{ error }}
  </p><p
    v-if="message"
    class="file-path"
    role="status"
  >
    {{ message }}
  </p>
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
  <section class="panel">
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
  <section
    v-if="restorePath"
    class="panel confirmation"
    role="alertdialog"
    aria-labelledby="restore-title"
  >
    <h2 id="restore-title">
      确认替换当前项目数据？
    </h2><p class="file-path">
      {{ restorePath }}
    </p><p>恢复将替换全部项目、创作版本、模型配置、任务、素材关联和普通设置。系统会先校验备份并自动保存当前数据库；原始素材文件保持原位，失效路径可在项目内重新定位。密钥不随备份恢复。</p><div class="actions">
      <Button
        :disabled="busy"
        variant="destructive"
        type="button"
        @click="restore"
      >
        确认恢复
      </Button><Button
        :disabled="busy"
        variant="outline"
        type="button"
        @click="restorePath = ''"
      >
        取消
      </Button>
    </div>
  </section>
</template>
