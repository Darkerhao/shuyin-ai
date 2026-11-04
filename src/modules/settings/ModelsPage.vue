<script setup lang="ts">
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '@/components/ui/collapsible'
import { Input } from '@/components/ui/input'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { computed, nextTick, onMounted, onBeforeUnmount, ref } from 'vue'
import { onBeforeRouteLeave, useRoute, useRouter } from 'vue-router'
import ConfirmAction from '../../components/ConfirmAction.vue'
import { requestConfirmation } from '../../composables/useConfirmation'
import * as api from '../../services/ai'
import { isDesktop } from '../../services/projects'
import { emptyProvider, type ModelProvider, type ProviderInput, type ReasoningEffort } from '../../types/ai'
import { useOperation } from '../../composables/useOperation'
import AppIcon from '../../components/AppIcon.vue'
import OperationFeedback from '../../components/OperationFeedback.vue'
const reasoningEffortOptions: { value: ReasoningEffort; label: string }[] = [
  { value: 'none', label: '关闭 · none' },
  { value: 'minimal', label: '极低 · minimal' },
  { value: 'low', label: '低 · low' },
  { value: 'medium', label: '中 · medium' },
  { value: 'high', label: '高 · high' },
  { value: 'xhigh', label: '极高 · xhigh' },
  { value: 'max', label: '最高 · max' },
]
const desktop = isDesktop()
const route = useRoute()
const router = useRouter()
const providers = ref<ModelProvider[]>([])
const id = ref<string | null>(null)
const form = ref(emptyProvider())
const apiKey = ref('')
const showApiKey = ref(false)
const search = ref('')
const modelKinds = [
  { kind: 'text', label: '文本模型', icon: 'pen', usage: '内容创作' },
  { kind: 'image', label: '图片模型', icon: 'image', usage: '封面背景' },
] as const
const selectedProvider = computed(() => providers.value.find(provider => provider.id === id.value))
const providerGroups = computed(() => {
  const query = search.value.trim().toLocaleLowerCase()
  return modelKinds.map(group => ({
    ...group,
    providers: providers.value.filter(provider => provider.modelKind === group.kind &&
      (!query || `${provider.name} ${provider.modelName}`.toLocaleLowerCase().includes(query))),
  })).filter(group => group.providers.length)
})
const defaultModels = computed(() => modelKinds.map(group => ({
  ...group,
  provider: providers.value.find(provider => provider.modelKind === group.kind && provider.isDefault && provider.isEnabled),
})))
const original = ref(JSON.stringify(form.value))
const dirty = computed(() => JSON.stringify(form.value) !== original.value || !!apiKey.value)
const deleting = ref(false)
const loaded = ref(false)
let initialized = false
const { busy, error, message, operation, run } = useOperation()
const controlsDisabled = computed(() => !desktop || busy.value || !loaded.value)
const canLeave = () => !busy.value && (!dirty.value || requestConfirmation('模型配置尚未保存，确定放弃修改？'))
onBeforeRouteLeave(canLeave)
function beforeUnload(event: globalThis.BeforeUnloadEvent) { if (dirty.value || busy.value) event.preventDefault() }
globalThis.addEventListener('beforeunload', beforeUnload)
onBeforeUnmount(() => { apiKey.value = ''; globalThis.removeEventListener('beforeunload', beforeUnload) })
async function edit(provider?: ModelProvider) {
  if (!await canLeave()) return
  selectProvider(provider)
  await router.replace({ query: { ...route.query, model: provider?.id ?? 'new' } })
  if (!provider) {
    await nextTick()
    globalThis.document.getElementById('provider-name')?.focus()
  }
}
function selectProvider(provider?: ModelProvider) {
  id.value = provider?.id ?? null
  form.value = provider ? { modelKind: provider.modelKind, name: provider.name, baseUrl: provider.baseUrl, modelName: provider.modelName, reasoningEffort: provider.reasoningEffort, isEnabled: provider.isEnabled, isDefault: provider.isDefault } : emptyProvider()
  apiKey.value = ''; showApiKey.value = false; original.value = JSON.stringify(form.value); deleting.value = false
  error.value = ''; message.value = ''
}
const load = () => run(async () => {
  providers.value = await api.listProviders()
  if (!initialized && !dirty.value) selectProvider(route.query.model === 'new' ? undefined : providers.value.find(p => p.id === route.query.model) ?? providers.value.find(p => p.modelKind === 'text' && p.isDefault) ?? providers.value[0])
  initialized = true
  loaded.value = true
}, '正在读取模型列表…')
const save = () => run(async () => {
  const input: ProviderInput = { ...form.value, reasoningEffort: form.value.modelKind === 'text' ? form.value.reasoningEffort : null }
  const result = await api.saveProvider(id.value, input, apiKey.value || null)
  selectProvider(result.provider)
  message.value = result.warning ?? '模型配置已保存。密钥仅保存在系统凭据库中。'
  providers.value = await api.listProviders()
  await router.replace({ query: { ...route.query, model: id.value } })
}, '正在保存模型配置…')
const test = () => run(async () => { if (id.value) message.value = await api.testConnection(id.value) }, '正在测试连接，最多等待 300 秒（5 分钟）…')
const remove = () => run(async () => {
  if (!id.value) return
  await api.deleteProvider(id.value)
  selectProvider()
  message.value = '模型配置与对应系统凭据已删除。'; providers.value = await api.listProviders()
  await router.replace({ query: { ...route.query, model: 'new' } })
}, '正在删除模型及密钥…')
async function discard() {
  if (await canLeave()) selectProvider(selectedProvider.value)
}
onMounted(() => { if (desktop) void load() })
</script>
<template>
  <div class="models-page">
    <header class="models-heading">
      <div>
        <h1>模型设置</h1>
        <p>为内容创作与封面制作，连接合适的 AI 模型。</p>
      </div>
      <Button
        :disabled="controlsDisabled"
        type="button"
        class="add-model-button"
        @click="edit()"
      >
        <AppIcon name="plus" />新增模型
      </Button>
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
    <div
      v-if="!loaded && desktop && !busy"
      class="models-load-error"
      role="alert"
    >
      <div>
        <strong>暂时无法读取模型列表</strong>
        <p>请重试，读取成功后即可编辑配置。</p>
      </div>
      <Button
        type="button"
        variant="outline"
        @click="load"
      >
        <AppIcon name="refresh" />重新读取
      </Button>
    </div>

    <div
      class="model-layout"
      :aria-busy="busy"
    >
      <aside
        class="model-library"
        aria-label="已配置服务"
      >
        <div class="library-title">
          <h2>模型库 <span>{{ providers.length }}</span></h2>
          <Button
            :disabled="!desktop || busy"
            aria-label="刷新列表"
            title="刷新列表"
            variant="ghost"
            size="icon-sm"
            type="button"
            @click="load"
          >
            <AppIcon
              name="refresh"
              :class="{ spin: busy && !loaded }"
            />
          </Button>
        </div>
        <div class="model-search">
          <AppIcon name="search" />
          <Input
            id="model-search"
            v-model="search"
            aria-label="搜索模型"
            placeholder="搜索名称或模型 ID"
            autocomplete="off"
            :disabled="!desktop || !loaded"
            @keydown.esc="search = ''"
          />
          <Button
            v-if="search"
            type="button"
            variant="ghost"
            size="icon-sm"
            aria-label="清空搜索"
            @click="search = ''"
          >
            <AppIcon name="close" />
          </Button>
        </div>
        <div class="provider-list">
          <section
            v-for="group in providerGroups"
            :key="group.kind"
            class="provider-group"
            :aria-label="group.label"
          >
            <h3>{{ group.label }} <span>{{ group.providers.length }}</span></h3>
            <div class="provider-group-items">
              <Button
                v-for="provider in group.providers"
                :key="provider.id"
                :disabled="busy"
                :aria-pressed="id === provider.id"
                :aria-label="`编辑模型 ${provider.name}`"
                variant="ghost"
                class="provider-card"
                type="button"
                @click="edit(provider)"
              >
                <span
                  class="provider-icon"
                  :class="provider.modelKind"
                ><AppIcon :name="group.icon" /></span>
                <span class="provider-description">
                  <span class="provider-name"><strong>{{ provider.name }}</strong><span
                    v-if="provider.isDefault"
                    class="default-label"
                  >默认</span></span>
                  <span class="provider-model-id">{{ provider.modelName }}</span>
                  <span
                    class="provider-status"
                    :class="{ 'needs-attention': provider.credentialStatus === 'unavailable' || !provider.isEnabled }"
                  >
                    <span
                      class="status-dot"
                      :class="{ 'is-muted': !provider.isEnabled || provider.credentialStatus === 'missing' }"
                    />
                    {{ !provider.isEnabled ? '已停用' : { available: '密钥已保存', missing: '未设置密钥', unavailable: '凭据库不可用' }[provider.credentialStatus] }}
                  </span>
                </span>
                <AppIcon
                  v-if="id === provider.id"
                  name="chevron"
                  class="provider-chevron"
                />
              </Button>
            </div>
          </section>
          <div
            v-if="loaded && !providerGroups.length"
            class="model-empty"
          >
            <AppIcon :name="search.trim() ? 'search' : 'sparkle'" />
            <strong>{{ search.trim() ? '没有找到匹配的模型' : '添加你的第一个模型' }}</strong>
            <p>{{ search.trim() ? '试试其他名称或模型 ID。' : '在右侧填写服务信息，即可开始使用。' }}</p>
            <Button
              v-if="search.trim()"
              type="button"
              variant="link"
              @click="search = ''"
            >
              清空搜索
            </Button>
          </div>
          <p
            v-else-if="!loaded"
            class="library-placeholder"
          >
            {{ busy ? '正在读取模型…' : '尚未读取模型' }}
          </p>
        </div>
        <div
          v-if="providers.length"
          class="library-defaults"
        >
          <h3>创作时默认使用</h3>
          <div
            v-for="item in defaultModels"
            :key="item.kind"
            class="default-model-row"
          >
            <span><AppIcon :name="item.icon" />{{ item.usage }}</span>
            <strong>{{ item.provider?.name ?? '未设置' }}</strong>
          </div>
          <p>每种用途可设置一个默认模型。</p>
        </div>
        <div class="model-security">
          <AppIcon name="shield" />
          <div><strong>密钥由本机安全保管</strong><p>存入系统凭据库，不随数据库备份。</p></div>
        </div>
      </aside>

      <div class="model-detail">
        <form
          class="model-editor"
          @submit.prevent="save"
        >
          <div class="model-editor-heading">
            <span
              class="editor-symbol"
              :class="form.modelKind"
            ><AppIcon :name="form.modelKind === 'image' ? 'image' : 'pen'" /></span>
            <div>
              <h2>{{ id ? selectedProvider?.name ?? form.name : '添加模型' }}</h2>
              <p>{{ id ? '管理连接信息与使用偏好' : '接入兼容 OpenAI 协议的服务' }}</p>
            </div>
            <span class="editor-kind">{{ form.modelKind === 'image' ? '图片模型' : '文本模型' }}</span>
          </div>

          <div class="model-editor-body">
            <fieldset :disabled="controlsDisabled">
              <section
                class="model-section"
                aria-labelledby="connection-title"
              >
                <div class="model-section-heading">
                  <h3 id="connection-title">
                    连接配置
                  </h3><span>填写服务提供的接口信息</span>
                </div>
                <div class="model-fields">
                  <div class="model-field">
                    <label for="provider-name">服务名称 <span
                      class="required-mark"
                      aria-hidden="true"
                    >*</span></label>
                    <Input
                      id="provider-name"
                      v-model="form.name"
                      required
                      maxlength="100"
                      placeholder="例如：我的创作模型"
                    />
                  </div>
                  <div class="model-field">
                    <label for="model-kind">模型用途</label>
                    <Select
                      v-model="form.modelKind"
                      :disabled="controlsDisabled"
                    >
                      <SelectTrigger
                        id="model-kind"
                        class="model-control"
                      >
                        <SelectValue placeholder="请选择" />
                      </SelectTrigger>
                      <SelectContent>
                        <SelectItem value="text">
                          文本 · 发布文案
                        </SelectItem>
                        <SelectItem value="image">
                          图片 · AI 封面背景
                        </SelectItem>
                      </SelectContent>
                    </Select>
                  </div>
                  <div class="model-field field-full">
                    <label for="base-url">服务地址 <span class="field-technical">Base URL</span><span
                      class="required-mark"
                      aria-hidden="true"
                    >*</span></label>
                    <Input
                      id="base-url"
                      v-model="form.baseUrl"
                      type="url"
                      required
                      placeholder="https://api.example.com/v1"
                      aria-describedby="base-url-help"
                      spellcheck="false"
                    />
                    <p
                      id="base-url-help"
                      class="field-hint"
                    >
                      密钥和书籍资料会发送到此地址，请仅连接可信服务。
                    </p>
                  </div>
                  <div class="model-field field-full">
                    <label for="model-name">模型名称 <span class="field-technical">Model ID</span><span
                      class="required-mark"
                      aria-hidden="true"
                    >*</span></label>
                    <Input
                      id="model-name"
                      v-model="form.modelName"
                      required
                      maxlength="200"
                      placeholder="填写服务实际支持的模型 ID"
                      spellcheck="false"
                    />
                  </div>
                  <div class="model-field field-full">
                    <div class="field-label-row">
                      <label for="api-key">API Key</label>
                      <span
                        v-if="selectedProvider?.credentialStatus === 'available'"
                        class="credential-label"
                      ><AppIcon name="shield" />已有密钥</span>
                    </div>
                    <div class="api-key-input">
                      <Input
                        id="api-key"
                        v-model="apiKey"
                        :type="showApiKey ? 'text' : 'password'"
                        autocomplete="new-password"
                        spellcheck="false"
                        maxlength="4096"
                        :placeholder="id ? '留空保留已保存的密钥' : '输入服务提供的 API Key'"
                        aria-describedby="api-key-help"
                      />
                      <Button
                        type="button"
                        variant="ghost"
                        size="icon-sm"
                        :disabled="controlsDisabled"
                        :aria-label="showApiKey ? '隐藏输入的密钥' : '显示输入的密钥'"
                        :aria-pressed="showApiKey"
                        aria-controls="api-key"
                        @click="showApiKey = !showApiKey"
                      >
                        <AppIcon :name="showApiKey ? 'eyeOff' : 'eye'" />
                      </Button>
                    </div>
                    <p
                      id="api-key-help"
                      class="field-hint"
                    >
                      {{ id ? '已保存的密钥不会回显。更换服务地址时，请重新输入密钥。' : '本机免密服务可留空；密钥仅保存在本机系统凭据库。' }}
                    </p>
                  </div>
                </div>
                <Collapsible
                  class="connection-help"
                  :unmount-on-hide="false"
                >
                  <CollapsibleTrigger class="disclosure-trigger">
                    接口与密钥说明
                  </CollapsibleTrigger>
                  <CollapsibleContent>
                    <p>填写 API 根地址，系统会追加 <code>{{ form.modelKind === 'image' ? '/images/generations' : '/chat/completions' }}</code>；本机服务支持 HTTP。</p>
                    <p>密钥存入 Windows 凭据管理器 / macOS 钥匙串 / Linux Secret Service，不随数据库备份。</p>
                    <p v-if="form.modelKind === 'text'">
                      推理档位以服务说明为准；强度越高通常耗时、用量越多。不支持时请选择「服务默认」。
                    </p>
                    <p v-else>
                      支持兼容 GPT Image、返回 base64 PNG、JPEG 或 WebP 的服务。生成图片可能产生服务费用；不配置图片模型也可使用本地模板。
                    </p>
                  </CollapsibleContent>
                </Collapsible>
              </section>

              <section
                class="model-section preferences-section"
                aria-labelledby="preferences-title"
              >
                <div class="model-section-heading">
                  <h3 id="preferences-title">
                    使用偏好
                  </h3>
                </div>
                <div
                  v-if="form.modelKind === 'text'"
                  class="reasoning-row"
                >
                  <div>
                    <label for="reasoning-effort">推理强度</label><p
                      id="reasoning-effort-help"
                      class="field-hint"
                    >
                      默认由服务决定，请按模型支持的档位选择。
                    </p>
                  </div>
                  <Select
                    :model-value="form.reasoningEffort ?? 'default'"
                    :disabled="controlsDisabled"
                    @update:model-value="form.reasoningEffort = $event === 'default' ? null : $event as ReasoningEffort"
                  >
                    <SelectTrigger
                      id="reasoning-effort"
                      class="model-control"
                      aria-describedby="reasoning-effort-help"
                    >
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="default">
                        服务默认（推荐）
                      </SelectItem>
                      <SelectItem
                        v-for="option in reasoningEffortOptions"
                        :key="option.value"
                        :value="option.value"
                      >
                        {{ option.label }}
                      </SelectItem>
                    </SelectContent>
                  </Select>
                </div>
                <div class="model-option-row">
                  <div>
                    <label for="model-enabled">启用模型</label><p class="field-hint">
                      {{ form.modelKind === 'image' ? '在封面工坊中可选择此模型' : '在内容创作中可选择此模型' }}
                    </p>
                  </div>
                  <Checkbox
                    id="model-enabled"
                    aria-label="启用模型"
                    :model-value="form.isEnabled"
                    :disabled="controlsDisabled"
                    @update:model-value="form.isEnabled = $event === true; if (!form.isEnabled) form.isDefault = false"
                  />
                </div>
                <div class="model-option-row">
                  <div>
                    <label for="model-default">设为默认{{ form.modelKind === 'image' ? '图片' : '文本' }}模型</label><p class="field-hint">
                      {{ form.isEnabled ? '开始创作时优先选择此模型' : '启用模型后可设置为默认' }}
                    </p>
                  </div>
                  <Checkbox
                    id="model-default"
                    :aria-label="`设为默认${form.modelKind === 'image' ? '图片' : '文本'}模型`"
                    :model-value="form.isDefault"
                    :disabled="controlsDisabled || !form.isEnabled"
                    @update:model-value="form.isDefault = $event === true"
                  />
                </div>
                <p
                  v-if="form.modelKind === 'image'"
                  class="image-test-hint"
                >
                  <AppIcon name="info" />保存后，前往封面工坊生成背景验证效果。
                </p>
              </section>
            </fieldset>
          </div>

          <div class="model-actions">
            <div class="model-save-status">
              <p
                v-if="busy"
                class="loading-state"
                role="status"
              >
                <AppIcon
                  name="refresh"
                  class="spin"
                />{{ operation }}
              </p>
              <p
                v-else
                :class="{ 'is-dirty': dirty }"
              >
                <AppIcon :name="dirty ? 'pen' : id ? 'check' : 'info'" />{{ dirty ? '有修改尚未保存' : id ? '配置已保存' : '填写完成后保存配置' }}
              </p>
              <span v-if="!busy && form.modelKind === 'text' && (!id || dirty)">保存后即可测试连接</span>
            </div>
            <div class="model-action-buttons">
              <Button
                v-if="dirty"
                type="button"
                variant="ghost"
                :disabled="controlsDisabled"
                @click="discard"
              >
                撤销修改
              </Button>
              <Button
                v-if="form.modelKind === 'text'"
                type="button"
                variant="outline"
                :disabled="controlsDisabled || !id || dirty"
                :title="!id || dirty ? '先保存配置，再测试连接' : '测试当前服务'"
                @click="test"
              >
                <AppIcon name="refresh" />测试连接
              </Button>
              <Button
                type="submit"
                :disabled="controlsDisabled"
              >
                <AppIcon name="save" />保存模型
              </Button>
            </div>
          </div>
        </form>

        <div
          v-if="id"
          class="model-danger-zone"
        >
          <div><strong>删除此模型</strong><p>同时移除本机密钥，保留历史创作。</p></div>
          <Button
            type="button"
            variant="ghost"
            class="delete-model"
            :disabled="controlsDisabled"
            @click="deleting = true"
          >
            删除模型
          </Button>
        </div>
        <ConfirmAction
          v-if="deleting"
          :title="`删除模型「${form.name}」及密钥？`"
          :busy="busy"
          @confirm="remove"
          @cancel="deleting = false"
        >
          <p>历史创作版本保留。使用此模型的运行任务需要先结束。</p>
        </ConfirmAction>
        <p class="models-footer">
          <AppIcon name="folder" /><a href="#/settings">数据备份与应用目录</a>
        </p>
      </div>
    </div>
  </div>
</template>

<style scoped>
.models-page {
  --model-surface: #f8faf8;
  --model-selected: #edf5f0;
  --model-selected-border: #aac9b5;
  --model-image-surface: #f4f0e7;
  --model-image-ink: #80663b;
  --model-warning: #896023;
  --model-destructive: #a53d35;
  --model-control-border: var(--input-border);
  color: var(--foreground);
}
.models-heading { display: flex; align-items: center; justify-content: space-between; gap: 24px; margin: 0 0 28px; }
.models-heading h1 { margin: 0 0 8px; font-size: 28px; font-weight: 650; letter-spacing: -.7px; }
.models-heading p { margin: 0; color: var(--muted-foreground); font-size: 14px; }
.add-model-button { height: 40px; padding-inline: 16px; border-radius: 9px; }
.model-layout { display: grid; grid-template-columns: 264px minmax(0, 1fr); align-items: start; gap: 24px; }
.model-library { position: sticky; top: var(--workspace-sticky-top); min-width: 0; border: 0; box-shadow: var(--panel-shadow); border-radius: 14px; background: var(--card); padding: 16px 12px 0; }
.library-title { display: flex; align-items: center; justify-content: space-between; padding: 0 6px 12px; }
.library-title h2 { display: flex; align-items: center; gap: 8px; margin: 0; font-size: 14px; }
.library-title h2 > span { display: grid; min-width: 22px; height: 22px; padding-inline: 5px; place-items: center; border: 1px solid var(--border); border-radius: 6px; color: var(--muted-foreground); font-size: 12px; font-weight: 500; font-variant-numeric: tabular-nums; }
.model-search { position: relative; margin-inline: 4px; }
.model-search > .app-icon { position: absolute; z-index: 1; top: 12px; left: 11px; width: 16px; height: 16px; color: var(--muted-foreground); pointer-events: none; }
.model-search :deep(input) { padding-left: 34px; padding-right: 32px; font-size: 13px; }
.model-search > button { position: absolute; right: 3px; top: 4px; }
.provider-list { max-height: min(540px, 54vh); overflow-y: auto; padding: 2px; margin-top: 8px; scrollbar-gutter: stable; }
.provider-group + .provider-group { margin-top: 14px; }
.provider-group h3 { display: flex; justify-content: space-between; margin: 12px 8px 8px; color: var(--muted-foreground); font-size: 12px; font-weight: 500; }
.provider-group h3 span { font-variant-numeric: tabular-nums; }
.provider-group-items { display: grid; gap: 5px; }
.provider-card { display: flex; width: 100%; min-height: 98px; height: auto; justify-content: flex-start; align-items: flex-start; gap: 10px; white-space: normal; padding: 12px 10px; border: 1px solid transparent; border-radius: 9px; text-align: left; background: transparent; box-shadow: none; }
.provider-card:hover { background: var(--model-surface); }
.provider-card[aria-pressed="true"] { border-color: var(--model-selected-border); background: var(--model-selected); }
.provider-icon, .editor-symbol { display: grid; place-items: center; flex-shrink: 0; background: var(--accent); color: var(--primary); }
.provider-icon { width: 34px; height: 34px; border: 1px solid color-mix(in srgb, var(--primary) 12%, transparent); border-radius: 9px; }
.provider-icon.image, .editor-symbol.image { background: var(--model-image-surface); color: var(--model-image-ink); }
.provider-description { display: grid; flex: 1; min-width: 0; gap: 4px; }
.provider-name { display: flex; align-items: baseline; flex-wrap: wrap; gap: 4px 6px; line-height: 1.5; }
.provider-name strong { min-width: 0; font-size: 13px; font-weight: 600; overflow-wrap: anywhere; }
.default-label { flex-shrink: 0; color: var(--primary); font-size: 10px; line-height: 1.5; padding: 1px 5px; border-radius: 4px; background: var(--accent); }
.provider-model-id { color: var(--muted-foreground); font-size: 12px; line-height: 1.5; overflow-wrap: anywhere; }
.provider-status { display: flex; align-items: center; gap: 5px; margin-top: 2px; color: var(--muted-foreground); font-size: 11px; font-weight: 400; }
.provider-status .status-dot { width: 5px; height: 5px; background: var(--primary); }
.provider-status .status-dot.is-muted { background: var(--muted-foreground); }
.provider-status.needs-attention { color: var(--model-warning); }
.provider-chevron { width: 13px; height: 13px; margin: 4px -5px 0 0; color: var(--primary); }
.library-defaults { margin: 18px 8px 0; padding-top: 16px; border-top: 1px solid var(--border); }
.library-defaults h3 { margin: 0 0 12px; color: var(--muted-foreground); font-size: 12px; font-weight: 500; }
.default-model-row { display: flex; align-items: baseline; justify-content: space-between; gap: 12px; margin-top: 9px; font-size: 12px; }
.default-model-row > span { display: inline-flex; flex-shrink: 0; align-items: center; gap: 6px; color: var(--muted-foreground); }
.default-model-row .app-icon { width: 13px; height: 13px; }
.default-model-row strong { min-width: 0; text-align: right; font-weight: 500; overflow-wrap: anywhere; }
.library-defaults > p { margin: 14px 0 0; color: var(--muted-foreground); font-size: 11px; }
.model-security { display: flex; align-items: flex-start; gap: 8px; margin: 20px -12px 0; padding: 16px 18px; border-top: 1px solid var(--border); border-radius: 0 0 14px 14px; background: var(--model-surface); }
.model-security > .app-icon { width: 16px; height: 16px; margin-top: 2px; color: var(--primary); }
.model-security strong { font-size: 12px; font-weight: 500; }
.model-security p { margin: 3px 0 0; color: var(--muted-foreground); font-size: 11px; line-height: 1.8; }
.model-empty { display: grid; justify-items: center; gap: 9px; padding: 36px 12px; text-align: center; }
.model-empty > .app-icon { width: 26px; height: 26px; color: var(--muted-foreground); margin-bottom: 4px; }
.model-empty strong { font-size: 13px; font-weight: 500; }
.model-empty p, .library-placeholder { margin: 0; font-size: 12px; line-height: 1.8; color: var(--muted-foreground); }
.library-placeholder { padding: 28px 10px; text-align: center; }
.model-detail { min-width: 0; }
.model-editor { display: flex; flex-direction: column; max-height: calc(100dvh - 230px); border: 0; box-shadow: var(--panel-shadow); border-radius: 14px; background: var(--card); }
.model-editor-body { min-height: 0; overflow-y: auto; overscroll-behavior-y: contain; scroll-padding-block: 20px; }
.model-editor-heading { flex-shrink: 0; display: flex; align-items: center; gap: 12px; padding: 24px 28px; border-bottom: 1px solid var(--border); }
.editor-symbol { width: 42px; height: 42px; border-radius: 12px; }
.editor-symbol .app-icon { width: 20px; height: 20px; }
.model-editor-heading > div { min-width: 0; }
.model-editor-heading h2 { margin: 0; font-size: 17px; font-weight: 600; overflow-wrap: anywhere; }
.model-editor-heading p { margin: 3px 0 0; font-size: 12px; color: var(--muted-foreground); }
.editor-kind { flex-shrink: 0; margin-left: auto; padding: 3px 9px; border: 1px solid var(--border); border-radius: 6px; color: var(--muted-foreground); font-size: 11px; }
.model-section { padding: 24px 28px; }
.model-section + .model-section { padding-top: 8px; }
.model-section-heading { display: flex; align-items: baseline; justify-content: space-between; flex-wrap: wrap; gap: 5px 12px; margin-bottom: 20px; }
.model-section-heading h3 { margin: 0; font-size: 14px; font-weight: 600; }
.model-section-heading > span { color: var(--muted-foreground); font-size: 12px; }
.model-fields { display: grid; grid-template-columns: minmax(0, 1fr) minmax(0, 1fr); gap: 24px 18px; }
.model-field { min-width: 0; }
.field-full { grid-column: 1 / -1; }
.model-field label, .reasoning-row label, .model-option-row label { margin: 0 0 6px; font-size: 13px; font-weight: 500; }
.model-control { width: 100%; min-width: 0; font-size: 13px; }
.model-field :deep(input:focus-visible), .model-control:focus-visible { border-color: var(--primary); }
.model-field :deep(input), .model-control, .model-option-row :deep([data-slot="checkbox"]) { scroll-margin-block: 20px; }
.required-mark { margin-left: 3px; color: var(--muted-foreground); font-size: 12px; }
.field-technical { margin-left: 7px; color: var(--muted-foreground); font-size: 11px; font-weight: 400; }
.field-hint { margin: 7px 0 0; color: var(--muted-foreground); font-size: 12px; line-height: 1.7; }
.field-label-row { display: flex; justify-content: space-between; align-items: baseline; gap: 8px; }
.credential-label { display: inline-flex; align-items: center; gap: 4px; color: var(--primary); font-size: 11px; }
.credential-label .app-icon { width: 12px; height: 12px; }
.api-key-input { position: relative; }
.api-key-input :deep(input) { padding-right: 42px; }
.api-key-input > button { position: absolute; top: 5px; right: 5px; color: var(--muted-foreground); }
.connection-help { margin-top: 20px; padding: 0; border: 0; }
.connection-help :deep(.disclosure-trigger) { min-height: 20px; font-size: 12px; }
.connection-help p { margin: 12px 0 0; color: var(--muted-foreground); font-size: 12px; line-height: 1.8; }
.connection-help code { color: var(--foreground); font-size: 11px; overflow-wrap: anywhere; }
.preferences-section { padding-top: 22px; padding-bottom: 8px; }
.preferences-section .model-section-heading { margin-bottom: 4px; }
.reasoning-row { display: grid; grid-template-columns: minmax(0, 1fr) 190px; align-items: center; gap: 16px; padding-block: 16px; border-bottom: 1px solid var(--border); }
.reasoning-row label, .model-option-row label { margin-bottom: 0; cursor: pointer; }
.reasoning-row .field-hint, .model-option-row .field-hint { margin-top: 4px; }
.model-option-row { display: flex; align-items: center; justify-content: space-between; gap: 20px; padding-block: 16px; }
.model-option-row + .model-option-row { border-top: 1px solid var(--border); }
.model-option-row :deep([data-slot="checkbox"]) { width: 20px; height: 20px; border-color: var(--model-control-border); }
.model-option-row :deep([data-state="checked"]) { border-color: var(--primary); }
.image-test-hint { display: flex; gap: 7px; align-items: flex-start; padding: 12px; margin: 0 0 14px; color: var(--muted-foreground); background: var(--model-surface); border-radius: 8px; font-size: 12px; }
.image-test-hint .app-icon { width: 15px; height: 15px; margin-top: 3px; }
.model-actions { flex-shrink: 0; display: flex; align-items: center; justify-content: space-between; flex-wrap: wrap; gap: 16px; min-height: 80px; padding: 18px 28px; border-top: 1px solid var(--border); border-radius: 0 0 14px 14px; background: var(--card); box-shadow: 0 -4px 14px #202d2803; }
.model-save-status { min-width: 0; flex: 1 1 150px; }
.model-save-status p { display: flex; align-items: center; gap: 7px; margin: 0; padding: 0; color: var(--muted-foreground); font-size: 12px; line-height: 1.65; }
.model-save-status p.is-dirty { color: var(--model-warning); }
.model-save-status .app-icon { width: 14px; height: 14px; }
.model-save-status > span { display: block; margin: 4px 0 0 21px; font-size: 11px; color: var(--muted-foreground); }
.model-action-buttons { display: flex; align-items: center; flex-wrap: wrap; gap: 8px; }
.model-action-buttons > button { min-height: 38px; border-radius: 8px; font-size: 12px; }
.model-danger-zone { display: flex; align-items: center; justify-content: space-between; gap: 16px; margin: 24px 0 0; padding: 18px 4px; border: 0; }
.model-danger-zone strong { font-size: 13px; font-weight: 500; }
.model-danger-zone p { margin: 3px 0 0; color: var(--muted-foreground); font-size: 12px; }
.delete-model { color: var(--model-destructive); font-size: 12px; }
.delete-model:hover { color: var(--model-destructive); background: color-mix(in srgb, var(--model-destructive) 7%, var(--card)); }
.models-footer { display: flex; align-items: center; justify-content: flex-end; gap: 6px; margin: 20px 2px 0; color: var(--muted-foreground); font-size: 12px; }
.models-footer .app-icon { width: 14px; height: 14px; }
.models-footer a { color: inherit; text-decoration: none; }
.models-footer a:hover { color: var(--primary); text-decoration: underline; }
.models-load-error { display: flex; justify-content: space-between; align-items: center; gap: 16px; margin-bottom: 20px; padding: 16px 20px; border: 1px solid var(--border); border-radius: 10px; background: var(--card); font-size: 13px; }
.models-load-error p { margin: 3px 0 0; color: var(--muted-foreground); font-size: 12px; }
@media (max-width: 1180px) {
  .model-layout { grid-template-columns: 228px minmax(0, 1fr); gap: 20px; }
  .model-editor-heading, .model-section { padding-inline: 22px; }
  .model-actions { padding-inline: 22px; }
  .reasoning-row { grid-template-columns: minmax(0, 1fr); gap: 12px; }
  .reasoning-row .model-control { max-width: 250px; }
}
@media (max-width: 960px) {
  .model-layout { grid-template-columns: minmax(0, 1fr); }
  .model-library { position: static; }
  .model-editor { max-height: none; }
  .model-editor-body { overflow: visible; overscroll-behavior-y: auto; }
  .provider-list { max-height: 280px; }
  .provider-group-items { grid-template-columns: repeat(2, minmax(0, 1fr)); }
  .library-defaults { display: none; }
  .model-security { margin-top: 14px; padding-block: 12px; }
  .model-security > div { display: flex; align-items: baseline; flex-wrap: wrap; gap: 2px 12px; }
  .model-security p { margin-top: 0; }
  .reasoning-row { grid-template-columns: minmax(0, 1fr) 190px; }
}
@media (max-width: 600px) {
  .models-heading { flex-wrap: wrap; gap: 16px; margin-bottom: 22px; }
  .models-heading h1 { font-size: 25px; }
  .models-heading p { font-size: 13px; }
  .provider-group-items, .model-fields, .reasoning-row { grid-template-columns: minmax(0, 1fr); }
  .model-editor-heading, .model-section { padding-inline: 18px; }
  .model-editor-heading { flex-wrap: wrap; padding-block: 20px; }
  .editor-kind { display: none; }
  .model-section-heading > span { display: none; }
  .model-actions { padding: 16px 18px; gap: 12px; }
  .model-action-buttons { width: 100%; }
  .model-action-buttons > button { flex: 1; }
  .model-save-status { flex-basis: 100%; }
  .model-danger-zone, .models-load-error { align-items: flex-start; flex-wrap: wrap; padding: 16px 18px; }
  .models-footer { justify-content: flex-start; }
}
</style>
