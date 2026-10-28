<script setup lang="ts">
import OperationFeedback from '../../components/OperationFeedback.vue'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { onMounted, ref } from 'vue'
import * as api from '../../services/projects'
import type { Asset } from '../../types/project'
import { useOperation } from '../../composables/useOperation'
const props = defineProps<{ projectId: string }>()
const emit = defineEmits<{ changed: [] }>()
const assets = ref<Asset[]>([])
const loaded = ref(false)
const removing = ref<Asset | null>(null)
const { busy, error, message, run } = useOperation()
const openFile = (id: string, directory = false) => run(() => api.openAsset(id, directory))
const labels = { video: '视频', audio: '音频', image: '图片', cover: '封面', subtitle: '字幕', document: '文档' }
async function refresh() { assets.value = await api.listAssets(props.projectId); loaded.value = true }
const select = (asset?: Asset) => run(async () => {
  const path = await api.chooseAsset()
  if (!path) return
  await api.linkAsset(props.projectId, path, asset?.id ?? null)
  await refresh()
  emit('changed')
  message.value = asset ? '已重新定位素材。' : '素材已关联，源文件保持原位。'
})
const remove = () => run(async () => {
  if (!removing.value) return
  await api.unlinkAsset(removing.value.id)
  removing.value = null
  await refresh()
  emit('changed')
})
onMounted(() => { void run(refresh) })
</script>
<template>
  <section
    class="panel"
    aria-labelledby="assets-title"
    :aria-busy="busy"
  >
    <h2 id="assets-title">
      项目素材
    </h2><p class="muted">
      关联本地视频、音频、图片、字幕与文档。移除关联或删除项目都会保留源文件。
    </p>
    <div class="actions">
      <Button
        :disabled="busy"
        type="button"
        @click="select()"
      >
        关联素材
      </Button><Button
        :disabled="busy"
        variant="outline"
        type="button"
        @click="run(refresh)"
      >
        刷新文件状态
      </Button>
    </div>
    <OperationFeedback
      :error="error"
      :message="message"
    />
    <p
      v-if="busy"
      role="status"
    >
      正在处理素材…
    </p>
    <p
      v-else-if="loaded && !assets.length"
      class="muted"
    >
      还没有关联素材。
    </p>
    <article
      v-for="asset in assets"
      :key="asset.id"
      class="asset-row"
    >
      <h3>
        {{ asset.originalName }} <Badge variant="secondary">
          {{ labels[asset.assetType] }}
        </Badge>
      </h3>
      <p class="file-path">
        {{ asset.filePath }}
      </p>
      <p :class="asset.existsStatus === 'available' ? 'muted' : 'error'">
        {{ asset.existsStatus === 'available' ? '文件可用' : asset.existsStatus === 'missing' ? '文件已丢失，请重新定位' : '无法访问文件，请检查权限或磁盘连接' }} · {{ (asset.fileSize / 1024 / 1024).toFixed(2) }} MB · {{ new Date(asset.modifiedAt * 1000).toLocaleString() }}
      </p>
      <div class="actions wrap">
        <Button
          :disabled="busy || asset.existsStatus !== 'available'"
          variant="outline"
          type="button"
          @click="openFile(asset.id)"
        >
          打开文件
        </Button><Button
          :disabled="busy"
          variant="outline"
          type="button"
          @click="openFile(asset.id, true)"
        >
          打开所在目录
        </Button><Button
          :disabled="busy"
          variant="outline"
          type="button"
          @click="select(asset)"
        >
          重新定位
        </Button><Button
          :disabled="busy"
          variant="destructive"
          type="button"
          @click="removing = asset"
        >
          移除关联
        </Button>
      </div>
    </article>
    <div
      v-if="removing"
      class="notice"
      role="alertdialog"
      aria-labelledby="unlink-title"
    >
      <h3 id="unlink-title">
        移除「{{ removing.originalName }}」的关联？
      </h3><p>源文件将保留在原位置。</p><div class="actions">
        <Button
          :disabled="busy"
          type="button"
          @click="remove"
        >
          确认移除
        </Button><Button
          :disabled="busy"
          variant="outline"
          type="button"
          @click="removing = null"
        >
          取消
        </Button>
      </div>
    </div>
  </section>
</template>
