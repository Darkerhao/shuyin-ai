<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { isDesktop } from '../services/projects'
import { listProviders } from '../services/ai'
import { automationRuns, automationConsent, automationAccount, startAutomation, refreshAutomation, prepareAccount, confirmAccount, type AutomationAccount } from '../services/automation'
import type { ModelProvider } from '../types/ai'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'

const props = defineProps<{ saveSource: () => Promise<{ projectId: string; sourceAssetId: string }>; disabled?: boolean }>()
const emit = defineEmits<{ busy: [value: boolean] }>()
const providers = ref<ModelProvider[]>([])
const textProvider = ref('')
const imageProvider = ref('')
const account = ref<AutomationAccount | null>(automationAccount.value)
const accountReady = ref(!!automationAccount.value)
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
onMounted(async () => {
  if (!isDesktop()) return
  try {
    providers.value = await listProviders()
  } catch { error.value = '模型配置读取失败，请在模型设置中检查。' }
})
async function readAccount() {
  busy.value = true; error.value = ''; accountReady.value = false; account.value = null
  try { account.value = await prepareAccount() }
  catch (cause) { error.value = String(cause) }
  finally { busy.value = false }
}
async function acceptAccount() {
  if (!account.value) return
  busy.value = true; error.value = ''
  try { await confirmAccount(account.value); accountReady.value = true }
  catch (cause) { error.value = String(cause); accountReady.value = false }
  finally { busy.value = false }
}
async function start() {
  busy.value = true; error.value = ''
  try {
    const source = await props.saveSource()
    await startAutomation({ ...source, textProviderId: textProvider.value, imageProviderId: imageProvider.value, format: 'deep_dive', requestKey: requestKey.value })
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
      使用本书已保存的目标时长，未设置时由AI按书籍内容推荐；目标仅作创作指导，制作按实际音频完整时长继续。4:3、3:4与16:9各一张AI背景，北京时间最近未来01:11定时，不允许保存视频、不加入合集。
    </p>
    <div class="flex flex-wrap items-center gap-3">
      <Button
        variant="outline"
        :disabled="busy || active || !isDesktop()"
        @click="readAccount"
      >
        打开抖音并读取账号
      </Button>
      <template v-if="account">
        <span class="text-sm">当前账号：{{ account.name || '请核对官方窗口头像' }}</span><Button
          variant="outline"
          :disabled="busy || accountReady"
          @click="acceptAccount"
        >
          {{ accountReady ? '本次窗口账号已确认' : '确认使用此账号' }}
        </Button>
      </template>
    </div>
    <label class="flex items-start gap-3 text-sm">
      <Checkbox
        v-model="consent"
        :disabled="busy || active"
        aria-label="允许自动制作和上传"
      />
      <span>允许将书名、作者及已提供资料发送至文本模型，推荐时长并生成创作方案；将原书和完整音频指导发送至NotebookLM，再根据实际字幕校核发布文案，调用图片模型为三种比例生成背景，自动采用并渲染上传。模型失败后最多自动重试3次，再自动切换其他已启用的同类模型继续，可能产生额外费用；已完成成果保留，最终发布由我完成。</span>
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
      :disabled="busy || disabled || active || !consent || !accountReady || !textProvider || !imageProvider || !isDesktop()"
      @click="start"
    >
      {{ busy ? '正在准备…' : '自动制作并准备发布' }}
    </Button>
  </section>
</template>
