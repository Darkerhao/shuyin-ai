<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { isDesktop } from '../services/projects'
import { listProviders } from '../services/ai'
import { automationRuns, automationConsent, automationAccount, startAutomation, refreshAutomation, prepareAccount, confirmAccount, type AutomationAccount } from '../services/automation'
import type { ModelProvider } from '../types/ai'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { Input } from '@/components/ui/input'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'

const props = defineProps<{ saveSource: () => Promise<{ projectId: string; sourceAssetId: string }>; disabled?: boolean }>()
const emit = defineEmits<{ busy: [value: boolean] }>()
const providers = ref<ModelProvider[]>([])
const textProvider = ref('')
const imageProvider = ref('')
const account = ref<AutomationAccount | null>(automationAccount.value ? { ...automationAccount.value } : null)
const accountReady = computed(() => !!account.value && !!automationAccount.value
  && account.value.id === automationAccount.value.id && account.value.key === automationAccount.value.key
  && account.value.name === automationAccount.value.name)
const collectionName = ref<string | null>(null)
const collectionMode = computed({
  get: () => collectionName.value === null ? 'none' : 'named',
  set: (value: string) => { collectionName.value = value === 'none' ? null : collectionName.value ?? '' },
})
const collectionValid = computed(() => collectionName.value === null || !!collectionName.value.trim())
const consent = automationConsent
const busy = ref(false)
watch(busy, value => emit('busy', value), { flush: 'sync' })
const error = ref('')
const requestKey = ref(globalThis.crypto.randomUUID())
const textModels = computed(() => providers.value.filter(provider => provider.modelKind === 'text' && provider.isEnabled))
const imageModels = computed(() => providers.value.filter(provider => provider.modelKind === 'image' && provider.isEnabled))
const activeRun = computed(() => automationRuns.value.find(run => !['completed', 'cancelled'].includes(run.status)))
const active = computed(() => !!activeRun.value)
watch([activeRun, providers], ([current]) => {
  textProvider.value = current?.input.selection.textProviderId ?? (textProvider.value || textModels.value.find(provider => provider.isDefault)?.id || textModels.value[0]?.id || '')
  imageProvider.value = current?.input.selection.imageProviderId ?? (imageProvider.value || imageModels.value.find(provider => provider.isDefault)?.id || imageModels.value[0]?.id || '')
}, { immediate: true })
watch(activeRun, current => {
  if (!current) return
  account.value = current.input.account ? { ...current.input.account } : null
  collectionName.value = current.input.selection.collectionName ?? null
}, { immediate: true })
onMounted(async () => {
  if (!isDesktop()) return
  try {
    providers.value = await listProviders()
  } catch { error.value = '模型配置读取失败，请在模型设置中检查。' }
})
async function readAccount() {
  busy.value = true; error.value = ''; account.value = null
  try { account.value = await prepareAccount() }
  catch (cause) { error.value = String(cause) }
  finally { busy.value = false }
}
async function acceptAccount() {
  if (!account.value?.name.trim() || busy.value || active.value) return
  busy.value = true; error.value = ''
  try { account.value = { ...await confirmAccount({ ...account.value, name: account.value.name.trim() }) } }
  catch (cause) { error.value = String(cause) }
  finally { busy.value = false }
}
async function start() {
  if (busy.value || props.disabled || active.value || !consent.value || !accountReady.value || !collectionValid.value || !textProvider.value || !imageProvider.value || !isDesktop()) return
  busy.value = true; error.value = ''
  try {
    const source = await props.saveSource()
    await startAutomation({ ...source, textProviderId: textProvider.value, imageProviderId: imageProvider.value, format: 'deep_dive', requestKey: requestKey.value, collectionName: collectionName.value?.trim() ?? null })
    requestKey.value = globalThis.crypto.randomUUID()
    await refreshAutomation()
  } catch (cause) { error.value = String(cause) }
  finally { busy.value = false }
}
</script>

<template>
  <section
    class="my-6 grid gap-4 rounded-xl border border-primary/30 bg-card p-5"
    aria-labelledby="automation-title"
  >
    <div>
      <h2
        id="automation-title"
        class="text-lg font-semibold"
      >
        自动制作并准备发布
      </h2>
      <p class="mt-1 text-sm text-muted-foreground">
        选择一本原书，先确定时长并保存文案、音频和封面提示词，再制作音频、三尺寸AI封面和视频，上传抖音。最后由你检查并点击发布。
      </p>
    </div>
    <div class="grid gap-4 sm:grid-cols-2">
      <div class="grid gap-2">
        <label for="automation-text">文本模型</label>
        <Select
          v-model="textProvider"
          :disabled="busy || active"
        >
          <SelectTrigger id="automation-text">
            <SelectValue placeholder="选择文本模型" />
          </SelectTrigger><SelectContent>
            <SelectItem
              v-for="provider in textModels"
              :key="provider.id"
              :value="provider.id"
            >
              {{ provider.name }}
            </SelectItem>
          </SelectContent>
        </Select>
      </div>
      <div class="grid gap-2">
        <label for="automation-image">图片模型</label>
        <Select
          v-model="imageProvider"
          :disabled="busy || active"
        >
          <SelectTrigger id="automation-image">
            <SelectValue placeholder="选择图片模型" />
          </SelectTrigger><SelectContent>
            <SelectItem
              v-for="provider in imageModels"
              :key="provider.id"
              :value="provider.id"
            >
              {{ provider.name }}
            </SelectItem>
          </SelectContent>
        </Select>
      </div>
    </div>
    <p class="text-sm text-muted-foreground">
      使用本书已保存的目标时长，未设置时由AI按书籍内容推荐；目标仅作创作指导，制作按实际音频完整时长继续。4:3、3:4与16:9各一张AI背景，北京时间最近未来01:11定时，不允许保存视频。
    </p>
    <div class="grid min-w-0 gap-4 rounded-lg border border-border bg-background/40 p-4">
      <div>
        <h3 class="text-sm font-semibold">
          本次发布账号与合集
        </h3>
        <p class="mt-1 text-sm text-muted-foreground">
          在这里确认一次，后续步骤自动沿用；账号变化、登录失效或合集无法匹配时才需要处理。
        </p>
      </div>
      <div class="flex flex-wrap items-center gap-3">
        <Button
          variant="outline"
          :disabled="busy || active || !isDesktop()"
          @click="readAccount"
        >
          打开抖音并读取账号
        </Button>
        <span
          v-if="accountReady || active"
          role="status"
          class="text-sm text-primary"
        >{{ active ? '已保存本次任务的账号与合集' : '账号已确认，后续步骤自动沿用' }}</span>
      </div>
      <div
        v-if="account"
        class="grid min-w-0 gap-2"
      >
        <label for="automation-account-name">官方窗口显示的账号昵称</label>
        <div class="flex flex-wrap items-center gap-3">
          <Input
            id="automation-account-name"
            v-model="account.name"
            class="min-w-0 flex-1 basis-48"
            placeholder="核对官方窗口后填写昵称"
            maxlength="200"
            :disabled="busy || active || accountReady"
          />
          <Button
            v-if="!accountReady && !active"
            variant="outline"
            :disabled="busy || !account.name.trim()"
            @click="acceptAccount"
          >
            确认使用此账号
          </Button>
        </div>
      </div>
      <div class="grid min-w-0 gap-2">
        <label for="automation-collection">加入合集</label>
        <Select
          v-model="collectionMode"
          :disabled="busy || active"
        >
          <SelectTrigger id="automation-collection">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="none">
              不加入合集
            </SelectItem>
            <SelectItem value="named">
              加入指定合集
            </SelectItem>
          </SelectContent>
        </Select>
        <template v-if="collectionName !== null">
          <label for="automation-collection-name">合集完整名称</label>
          <Input
            id="automation-collection-name"
            v-model="collectionName"
            placeholder="填写此账号已有的合集名称"
            maxlength="200"
            :disabled="busy || active"
            aria-describedby="automation-collection-hint"
          />
          <p
            id="automation-collection-hint"
            class="text-sm text-muted-foreground"
          >
            上传后自动匹配官方同名合集并选中，无需再次选择。请填写完整名称；同名多项或名称不匹配时会暂停提示。
          </p>
        </template>
      </div>
    </div>
    <label class="flex items-start gap-3 text-sm">
      <Checkbox
        v-model="consent"
        :disabled="busy || active"
        aria-label="允许自动制作和上传"
      />
      <span>允许将书名、作者及已提供资料发送至文本模型，推荐时长并生成创作方案；将原书和完整音频指导发送至NotebookLM，再根据实际字幕校核发布文案，调用图片模型为三种比例生成背景，自动采用并渲染，按上方账号与合集设置上传。模型失败后最多自动重试3次，再自动切换其他已启用的同类模型继续，可能产生额外费用；已完成成果保留，最终发布由我完成。</span>
    </label>
    <p
      v-if="error"
      role="alert"
      class="text-sm text-destructive"
    >
      {{ error }}
    </p>
    <Button
      class="justify-self-start"
      :disabled="busy || disabled || active || !consent || !accountReady || !collectionValid || !textProvider || !imageProvider || !isDesktop()"
      @click="start"
    >
      {{ busy ? '正在准备…' : '自动制作并准备发布' }}
    </Button>
  </section>
</template>
