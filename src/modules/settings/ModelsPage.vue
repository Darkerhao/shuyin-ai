<script setup lang="ts">
import { computed, onMounted, onBeforeUnmount, ref } from 'vue'
import { onBeforeRouteLeave } from 'vue-router'
import * as api from '../../services/ai'
import { isDesktop } from '../../services/projects'
import { emptyProvider, type ModelProvider, type ProviderInput } from '../../types/ai'
import { useOperation } from '../../composables/useOperation'
const desktop = isDesktop()
const providers = ref<ModelProvider[]>([])
const id = ref<string | null>(null)
const form = ref(emptyProvider())
const apiKey = ref('')
const original = ref(JSON.stringify(form.value))
const dirty = computed(() => JSON.stringify(form.value) !== original.value || !!apiKey.value)
const deleting = ref(false)
const { busy, error, message, run } = useOperation()
const canLeave = () => !busy.value && (!dirty.value || globalThis.confirm('模型配置尚未保存，确定放弃修改？'))
onBeforeRouteLeave(canLeave)
function beforeUnload(event: globalThis.BeforeUnloadEvent) { if (dirty.value || busy.value) event.preventDefault() }
globalThis.addEventListener('beforeunload', beforeUnload)
onBeforeUnmount(() => { apiKey.value = ''; globalThis.removeEventListener('beforeunload', beforeUnload) })
function edit(provider?: ModelProvider) {
  if (!canLeave()) return
  id.value = provider?.id ?? null
  form.value = provider ? { name: provider.name, baseUrl: provider.baseUrl, modelName: provider.modelName, isEnabled: provider.isEnabled, isDefault: provider.isDefault } : emptyProvider()
  apiKey.value = ''; original.value = JSON.stringify(form.value); deleting.value = false
}
const load = () => run(async () => { providers.value = await api.listProviders() })
const save = () => run(async () => {
  const input: ProviderInput = { ...form.value }
  const result = await api.saveProvider(id.value, input, apiKey.value || null)
  apiKey.value = ''; id.value = result.provider.id
  form.value = { ...input, baseUrl: result.provider.baseUrl }; original.value = JSON.stringify(form.value)
  message.value = result.warning ?? '模型配置已保存。密钥仅保存在系统凭据库中。'
  providers.value = await api.listProviders()
})
const test = () => run(async () => { if (id.value) message.value = await api.testConnection(id.value) })
const remove = () => run(async () => {
  if (!id.value) return
  await api.deleteProvider(id.value)
  id.value = null; form.value = emptyProvider(); original.value = JSON.stringify(form.value); apiKey.value = ''; deleting.value = false
  message.value = '模型配置与对应系统凭据已删除。'; providers.value = await api.listProviders()
})
onMounted(() => { if (desktop) void load() })
</script>
<template>
  <header>
    <p class="eyebrow">
      AI 创作设置
    </p><h1>文本模型</h1><p class="muted">
      配置兼容 Chat Completions 的服务，供所有书籍项目使用。
    </p>
  </header>
  <p
    v-if="!desktop"
    class="notice"
  >
    请在桌面应用中配置模型与系统凭据。
  </p>
  <p
    v-if="error"
    role="alert"
    class="error"
  >
    {{ error }}
  </p>
  <p
    v-if="message"
    role="status"
    class="notice"
  >
    {{ message }}
  </p>
  <section class="panel">
    <h2>已配置服务</h2>
    <p
      v-if="!providers.length"
      class="muted"
    >
      尚未配置文本模型。填写下方表单后保存即可开始。
    </p>
    <div class="actions wrap">
      <button
        v-for="provider in providers"
        :key="provider.id"
        class="secondary"
        :disabled="busy"
        @click="edit(provider)"
      >
        {{ provider.name }}{{ provider.isDefault ? ' · 默认' : '' }}{{ provider.isEnabled ? '' : ' · 已停用' }} · {{ { available: '密钥已保存', missing: '无密钥', unavailable: '凭据库不可用' }[provider.credentialStatus] }}
      </button>
      <button
        :disabled="!desktop || busy"
        @click="edit()"
      >
        新增模型
      </button>
      <button
        class="secondary"
        :disabled="!desktop || busy"
        @click="load"
      >
        刷新列表
      </button>
    </div>
  </section>
  <form
    class="panel"
    @submit.prevent="save"
  >
    <h2>{{ id ? '编辑模型' : '新增模型' }}</h2>
    <fieldset :disabled="!desktop || busy">
      <label for="provider-name">服务名称 *</label><input
        id="provider-name"
        v-model="form.name"
        required
        maxlength="100"
      >
      <label for="base-url">Base URL *</label><input
        id="base-url"
        v-model="form.baseUrl"
        type="url"
        required
        placeholder="https://可信服务/v1"
      >
      <p class="muted">
        填写 API 根地址，系统会追加 /chat/completions。密钥和书籍资料将发送到此地址，请仅连接可信服务。本机服务支持 HTTP。
      </p>
      <label for="model-name">模型名 *</label><input
        id="model-name"
        v-model="form.modelName"
        required
        maxlength="200"
        placeholder="填写服务实际支持的模型 ID"
      >
      <label for="api-key">API Key{{ id ? '（留空保留已有密钥）' : '（本机免密服务可留空）' }}</label>
      <input
        id="api-key"
        v-model="apiKey"
        type="password"
        autocomplete="new-password"
        spellcheck="false"
        maxlength="4096"
      >
      <p class="muted">
        密钥存入 Windows 凭据管理器 / macOS 钥匙串 / Linux Secret Service，不随数据库备份。更换服务地址时请重新输入密钥。
      </p>
      <label class="check-label"><input
        v-model="form.isEnabled"
        type="checkbox"
        @change="!form.isEnabled && (form.isDefault = false)"
      >启用模型</label>
      <label class="check-label"><input
        v-model="form.isDefault"
        type="checkbox"
        :disabled="!form.isEnabled"
      >设为默认文本模型</label>
      <div class="actions wrap">
        <button type="submit">
          保存模型
        </button>
        <button
          type="button"
          class="secondary"
          :disabled="!id || dirty"
          @click="test"
        >
          测试连接
        </button>
        <button
          v-if="id"
          type="button"
          class="secondary danger"
          @click="deleting = true"
        >
          删除模型
        </button>
      </div>
      <p
        v-if="busy"
        role="status"
      >
        正在处理，请稍候。连接测试最多等待 120 秒。
      </p>
    </fieldset>
  </form>
  <section
    v-if="deleting"
    class="panel confirmation"
    role="alertdialog"
    aria-labelledby="delete-model-title"
  >
    <h2 id="delete-model-title">
      确认删除此模型及密钥？
    </h2><p>历史创作版本保留。使用此模型的运行任务需要先结束。</p>
    <div class="actions">
      <button
        class="danger"
        :disabled="busy"
        @click="remove"
      >
        确认删除
      </button><button
        class="secondary"
        :disabled="busy"
        @click="deleting = false"
      >
        取消
      </button>
    </div>
  </section>
  <p class="muted">
    <a href="#/settings">数据备份与应用目录</a>
  </p>
</template>
