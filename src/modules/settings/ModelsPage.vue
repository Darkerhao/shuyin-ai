<script setup lang="ts">
import { computed, onMounted, onBeforeUnmount, ref } from 'vue'
import { onBeforeRouteLeave } from 'vue-router'
import * as api from '../../services/ai'
import { isDesktop } from '../../services/projects'
import { emptyProvider, type ModelProvider, type ProviderInput } from '../../types/ai'
import { useOperation } from '../../composables/useOperation'
import AppIcon from '../../components/AppIcon.vue'
import OperationFeedback from '../../components/OperationFeedback.vue'
const desktop = isDesktop()
const providers = ref<ModelProvider[]>([])
const id = ref<string | null>(null)
const form = ref(emptyProvider())
const apiKey = ref('')
const original = ref(JSON.stringify(form.value))
const dirty = computed(() => JSON.stringify(form.value) !== original.value || !!apiKey.value)
const deleting = ref(false)
let initialized = false
const { busy, error, message, run } = useOperation()
const canLeave = () => !busy.value && (!dirty.value || globalThis.confirm('模型配置尚未保存，确定放弃修改？'))
onBeforeRouteLeave(canLeave)
function beforeUnload(event: globalThis.BeforeUnloadEvent) { if (dirty.value || busy.value) event.preventDefault() }
globalThis.addEventListener('beforeunload', beforeUnload)
onBeforeUnmount(() => { apiKey.value = ''; globalThis.removeEventListener('beforeunload', beforeUnload) })
function edit(provider?: ModelProvider) {
  if (!canLeave()) return
  selectProvider(provider)
}
function selectProvider(provider?: ModelProvider) {
  id.value = provider?.id ?? null
  form.value = provider ? { modelKind: provider.modelKind, name: provider.name, baseUrl: provider.baseUrl, modelName: provider.modelName, isEnabled: provider.isEnabled, isDefault: provider.isDefault } : emptyProvider()
  apiKey.value = ''; original.value = JSON.stringify(form.value); deleting.value = false
}
const load = () => run(async () => {
  providers.value = await api.listProviders()
  if (!initialized && !dirty.value) selectProvider(providers.value.find(p => p.modelKind === 'text' && p.isDefault) ?? providers.value[0])
  initialized = true
})
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
  <header class="page-heading">
    <div>
      <p class="eyebrow">
        为灵感接上 AI
      </p><h1>模型设置</h1><p class="muted">
        连接你信赖的服务，让文字与画面各得其所。
      </p>
    </div>
    <span class="badge"><AppIcon name="shield" />密钥由系统安全保管</span>
  </header>
  <p
    v-if="!desktop"
    class="notice"
  >
    请在桌面应用中配置模型与系统凭据。
  </p>
  <OperationFeedback
    :error="error"
    :message="message"
    @dismiss="message = ''"
  />
  <div class="model-layout">
    <section
      class="panel model-list"
      aria-label="已配置服务"
    >
      <div class="section-heading">
        <h2>我的模型 <span class="badge">{{ providers.length }}</span></h2><button
          class="icon-button ghost"
          :disabled="!desktop || busy"
          aria-label="刷新列表"
          title="刷新列表"
          @click="load"
        >
          <AppIcon
            name="refresh"
            :class="{ spin: busy }"
          />
        </button>
      </div>
      <button
        v-for="provider in providers"
        :key="provider.id"
        class="provider-card"
        :class="{ selected: id === provider.id }"
        :disabled="busy"
        :aria-pressed="id === provider.id"
        :aria-label="`编辑模型 ${provider.name}`"
        @click="edit(provider)"
      >
        <span
          class="provider-icon"
          :class="provider.modelKind"
        ><AppIcon :name="provider.modelKind === 'image' ? 'image' : 'pen'" /></span>
        <span class="provider-description">
          <strong>{{ provider.name }}</strong>
          <span class="provider-meta"><span class="badge">{{ provider.modelKind === 'image' ? '图片' : '文本' }}</span><span
            v-if="provider.isDefault"
            class="badge green"
          >默认</span><span
            v-if="!provider.isEnabled"
            class="badge warm"
          >已停用</span></span>
          <small>{{ { available: '密钥已保存', missing: '无密钥', unavailable: '凭据库不可用' }[provider.credentialStatus] }}</small>
        </span>
      </button>
      <button
        class="secondary model-new"
        :disabled="!desktop || busy"
        @click="edit()"
      >
        <AppIcon name="plus" />新增模型
      </button>
      <p class="model-list-note">
        文本模型用于内容创作，图片模型用于生成封面背景。每种用途可设置一个默认模型。
      </p>
    </section>
    <div>
      <form
        class="panel model-editor"
        @submit.prevent="save"
      >
        <div class="section-heading">
          <h2>{{ id ? '编辑模型' : '添加一个新模型' }}</h2><span
            v-if="dirty"
            class="badge warm"
          >未保存</span>
        </div>
        <p class="muted">
          {{ id ? '调整连接信息，或测试当前服务是否可用。' : '填好服务信息，开始你的第一次 AI 创作。' }}
        </p>
        <fieldset :disabled="!desktop || busy">
          <div class="form-grid">
            <div>
              <label for="provider-name">服务名称 *</label><input
                id="provider-name"
                v-model="form.name"
                required
                maxlength="100"
                placeholder="给服务起一个好记的名字"
              >
            </div>
            <div>
              <label for="model-kind">模型用途</label><select
                id="model-kind"
                v-model="form.modelKind"
              >
                <option value="text">
                  文本 · 发布文案
                </option><option value="image">
                  图片 · AI 封面背景
                </option>
              </select>
            </div>
          </div>
          <label for="base-url">服务地址 · Base URL *</label><input
            id="base-url"
            v-model="form.baseUrl"
            type="url"
            required
            placeholder="https://可信服务/v1"
          >
          <p class="muted">
            密钥和书籍资料会发送到此地址，请仅连接可信服务。
          </p>
          <label for="model-name">模型名称 *</label><input
            id="model-name"
            v-model="form.modelName"
            required
            maxlength="200"
            placeholder="填写服务实际支持的模型 ID"
          >
          <label for="api-key">API Key <span class="optional">{{ id ? '· 留空保留已有密钥' : '· 本机免密服务可留空' }}</span></label>
          <input
            id="api-key"
            v-model="apiKey"
            type="password"
            autocomplete="new-password"
            spellcheck="false"
            maxlength="4096"
            :placeholder="id ? '已保存的密钥不会回显' : '输入服务提供的 API Key'"
          >
          <details class="connection-help">
            <summary>连接与密钥说明</summary>
            <p class="muted">
              填写 API 根地址，系统会追加 {{ form.modelKind === 'image' ? '/images/generations' : '/chat/completions' }}，本机服务支持 HTTP。更换服务地址时请重新输入密钥。
            </p>
            <p class="muted">
              密钥存入 Windows 凭据管理器 / macOS 钥匙串 / Linux Secret Service，不随数据库备份。
            </p>
            <p
              v-if="form.modelKind === 'image'"
              class="muted"
            >
              支持兼容 GPT Image、返回 base64 PNG、JPEG 或 WebP 的服务。保存后到封面工坊生成背景验证，可能产生服务费用。不配置图片模型也可使用本地模板。
            </p>
          </details>
          <div class="model-options">
            <label class="check-label"><input
              v-model="form.isEnabled"
              type="checkbox"
              @change="!form.isEnabled && (form.isDefault = false)"
            >启用模型</label>
            <label class="check-label"><input
              v-model="form.isDefault"
              type="checkbox"
              :disabled="!form.isEnabled"
            >设为默认{{ form.modelKind === 'image' ? '图片' : '文本' }}模型</label>
          </div>
          <div class="actions wrap">
            <button type="submit">
              <AppIcon name="save" />保存模型
            </button>
            <button
              v-if="form.modelKind === 'text'"
              type="button"
              class="secondary"
              :disabled="!id || dirty"
              :title="!id || dirty ? '先保存配置，再测试连接' : '测试当前服务'"
              @click="test"
            >
              测试连接
            </button>
            <button
              v-if="id"
              type="button"
              class="ghost delete-model"
              @click="deleting = true"
            >
              删除模型
            </button>
          </div>
          <p
            v-if="busy"
            class="muted"
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
      <p class="settings-footer muted">
        <AppIcon name="folder" /><a href="#/settings">数据备份与应用目录</a>
      </p>
    </div>
  </div>
</template>
<style scoped>
.connection-help { margin-top: 12px; color: #8a9780; font-size: 11px; }
.connection-help summary { cursor: pointer; }
.delete-model { margin-left: auto; }
</style>
