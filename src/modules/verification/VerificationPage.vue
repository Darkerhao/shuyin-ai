<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { errorMessage, isDesktop, readProbe, saveProbe, selectVideo } from '../../services/verification'
import type { ProbeSnapshot } from '../../services/verification'

const desktop = isDesktop()
const note = ref('')
const snapshot = ref<ProbeSnapshot | null>(null)
const databaseBusy = ref(false)
const databaseError = ref('')
const databaseStatus = ref('')
const selectedVideo = ref('')
const selecting = ref(false)
const fileError = ref('')
const fileStatus = ref('')

async function load() {
  databaseBusy.value = true
  databaseError.value = ''
  databaseStatus.value = ''
  try {
    snapshot.value = await readProbe()
    note.value = snapshot.value.record?.note ?? ''
    databaseStatus.value = snapshot.value.record ? '已读取上次保存的内容。' : '数据库已就绪，尚无验证记录。'
  } catch (error) {
    databaseError.value = errorMessage(error)
  } finally {
    databaseBusy.value = false
  }
}

async function save() {
  databaseBusy.value = true
  databaseError.value = ''
  databaseStatus.value = ''
  try {
    snapshot.value = await saveProbe(note.value)
    databaseStatus.value = '写入并回读成功。请关闭应用后重新打开，核对内容。'
  } catch (error) {
    databaseError.value = errorMessage(error)
  } finally {
    databaseBusy.value = false
  }
}

async function chooseVideo() {
  selecting.value = true
  fileError.value = ''
  fileStatus.value = ''
  try {
    const path = await selectVideo()
    if (path !== null) {
      selectedVideo.value = path
      fileStatus.value = '已取得本地视频路径。'
    } else {
      fileStatus.value = '已取消选择，保留原路径。'
    }
  } catch (error) {
    fileError.value = errorMessage(error)
  } finally {
    selecting.value = false
  }
}

onMounted(() => { if (desktop) void load() })
</script>

<template>
  <header>
    <p class="eyebrow">
      M0 / 工程初始化与技术验证
    </p>
    <h1>先让基础能力可靠运行</h1>
    <p class="muted">
      验证桌面容器、本地数据库和文件选择。此页面不包含创作业务。
    </p>
    <span class="runtime-badge">{{ desktop ? 'Tauri 桌面环境' : '浏览器预览' }}</span>
  </header>

  <p
    v-if="!desktop"
    class="notice"
    role="status"
  >
    当前为浏览器预览。请使用桌面应用验证 SQLite 和原生文件选择。
  </p>

  <section
    class="panel"
    aria-labelledby="database-heading"
    :aria-busy="databaseBusy"
  >
    <h2 id="database-heading">
      01 · SQLite 持久化
    </h2>
    <p class="muted">
      保存一段文字，关闭并重新打开应用后检查是否仍然存在。
    </p>
    <form @submit.prevent="save">
      <label for="probe-note">验证文字</label>
      <textarea
        id="probe-note"
        v-model="note"
        rows="3"
        maxlength="2000"
        :disabled="!desktop || databaseBusy"
        placeholder="例如：书引 AI，重启后见。"
      />
      <div class="actions">
        <button
          type="submit"
          :disabled="!desktop || databaseBusy"
        >
          {{ databaseBusy ? '处理中…' : '写入并回读' }}
        </button>
        <button
          type="button"
          class="secondary"
          :disabled="!desktop || databaseBusy"
          @click="load"
        >
          重新读取
        </button>
      </div>
    </form>
    <p
      v-if="databaseError"
      class="error"
      role="alert"
    >
      {{ databaseError }}
    </p>
    <p
      v-if="databaseStatus"
      role="status"
    >
      {{ databaseStatus }}
    </p>
    <dl v-if="snapshot">
      <dt>数据库文件</dt><dd>{{ snapshot.databasePath }}</dd>
      <dt>已保存内容</dt><dd>{{ snapshot.record?.note ?? '尚未保存' }}</dd>
      <dt>写入时间（UTC）</dt><dd>{{ snapshot.record?.updatedAt ?? '—' }}</dd>
    </dl>
  </section>

  <section
    class="panel"
    aria-labelledby="file-heading"
    :aria-busy="selecting"
  >
    <h2 id="file-heading">
      02 · 本地视频选择
    </h2>
    <p class="muted">
      通过 Windows 原生对话框选择视频，仅显示路径，不复制、修改或删除文件。
    </p>
    <button
      type="button"
      :disabled="!desktop || selecting"
      @click="chooseVideo"
    >
      {{ selecting ? '选择中…' : '选择本地视频' }}
    </button>
    <p
      v-if="selectedVideo"
      class="file-path"
    >
      {{ selectedVideo }}
    </p>
    <p
      v-else
      class="muted"
    >
      尚未选择视频
    </p>
    <p
      v-if="fileError"
      class="error"
      role="alert"
    >
      {{ fileError }}
    </p>
    <p
      v-if="fileStatus"
      role="status"
    >
      {{ fileStatus }}
    </p>
  </section>
</template>
