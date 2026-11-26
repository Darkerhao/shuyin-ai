<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { isDesktop } from '../services/projects'
import { listProviders } from '../services/ai'
import { douyinCollectionOptions, saveDouyinCollectionOptions } from '../services/publishing'
import { automationRuns, automationConsent, automationAccount, startAutomation, refreshAutomation, prepareAccount, confirmAccount, type AutomationAccount } from '../services/automation'
import type { ModelProvider } from '../types/ai'
import type { AudioSourceInput } from '../types/audio'
import type { CoverConfig } from '../types/cover'
import { videoThemes, videoThemeLabel } from '../types/video'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { Input } from '@/components/ui/input'
import SavedOptionInput from '@/components/SavedOptionInput.vue'
import AutomationAccountPreference from './AutomationAccountPreference.vue'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'

const props = defineProps<{ saveSource: () => Promise<{ projectId: string; source: AudioSourceInput }>; disabled?: boolean }>()
const emit = defineEmits<{ busy: [value: boolean] }>()
const providers = ref<ModelProvider[]>([])
const textProvider = ref('')
const imageProvider = ref('')
const theme = ref<keyof typeof videoThemes>('classic')
const coverTextMode = ref<CoverConfig['textMode']>('overlay')
const account = ref<AutomationAccount | null>(automationAccount.value ? { ...automationAccount.value } : null)
const accountReady = computed(() => !!account.value && !!automationAccount.value
  && account.value.id === automationAccount.value.id && account.value.key === automationAccount.value.key
  && account.value.name === automationAccount.value.name)
const collectionName = ref<string | null>(null)
const collectionOptions = ref<string[]>([])
const collectionOptionsReady = ref(false)
const collectionOptionsError = ref('')
const collectionMode = computed({
  get: () => collectionName.value === null ? 'none' : 'named',
  set: (value: string) => { collectionName.value = value === 'none' ? null : collectionName.value ?? '' },
})
const collectionValid = computed(() => collectionName.value === null || !!collectionName.value.trim())
const consent = automationConsent
const busy = ref(false)
const preferenceBusy = ref(false)
watch([busy, preferenceBusy], ([working, saving]) => emit('busy', working || saving), { flush: 'sync' })
const error = ref('')
const requestKey = ref(globalThis.crypto.randomUUID())
const textModels = computed(() => providers.value.filter(provider => provider.modelKind === 'text' && provider.isEnabled))
const imageModels = computed(() => providers.value.filter(provider => provider.modelKind === 'image' && provider.isEnabled))
const activeRun = computed(() => automationRuns.value.find(run => !['completed', 'cancelled'].includes(run.status)))
const active = computed(() => !!activeRun.value)
const selectedCoverTextMode = computed(() => activeRun.value?.input.selection.coverTextMode ?? (active.value ? 'overlay' : coverTextMode.value))
const coverImageLabel = computed(() => selectedCoverTextMode.value === 'embedded' ? 'AI 整图封面' : 'AI 背景')
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
  await loadCollectionOptions()
})
async function loadCollectionOptions() {
  busy.value = true; collectionOptionsError.value = ''
  try {
    collectionOptions.value = await douyinCollectionOptions()
    collectionOptionsReady.value = true
  } catch (cause) { collectionOptionsError.value = `合集选项读取失败：${String(cause)}` }
  finally { busy.value = false }
}
async function saveCollections(values: string[]) {
  busy.value = true
  try {
    const saved = await saveDouyinCollectionOptions(values)
    collectionOptions.value = saved
    return saved
  } finally { busy.value = false }
}
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
  if (busy.value || preferenceBusy.value || props.disabled || active.value || !consent.value || !textProvider.value || !accountReady.value || !collectionValid.value || !imageProvider.value || !isDesktop()) return
  busy.value = true; error.value = ''
  try {
    const source = await props.saveSource()
    await startAutomation({ ...source, textProviderId: textProvider.value, imageProviderId: imageProvider.value, format: 'deep_dive', requestKey: requestKey.value, theme: theme.value, coverTextMode: coverTextMode.value, collectionName: collectionName.value?.trim() ?? null })
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
        按当前选择的整本书或章节内容，生成创作方案、音频、三尺寸封面和视频，再准备抖音网页，最后由你检查并发布。
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
      <div class="grid gap-2">
        <label for="automation-theme">视频主题</label>
        <Select
          v-model="theme"
          :disabled="busy || active"
        >
          <SelectTrigger id="automation-theme">
            <SelectValue>{{ videoThemeLabel(activeRun?.input.selection.theme ?? theme) }}</SelectValue>
          </SelectTrigger><SelectContent>
            <SelectItem
              v-for="(name, key) in videoThemes"
              :key="key"
              :value="key"
            >
              {{ name }}
            </SelectItem>
          </SelectContent>
        </Select>
        <p class="text-xs text-muted-foreground">
          像素、漫画使用专属画布与分格；三维主题按字幕选用专属主体，普通物体保留原图解。
        </p>
      </div>
      <div class="grid gap-2">
        <label for="automation-cover-mode">封面生成方式</label>
        <Select
          v-model="coverTextMode"
          :disabled="busy || active"
        >
          <SelectTrigger
            id="automation-cover-mode"
            aria-describedby="automation-cover-mode-hint"
          >
            <SelectValue>{{ selectedCoverTextMode === 'embedded' ? 'AI 整图封面' : 'AI 背景 + 本地排字' }}</SelectValue>
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="overlay">
              AI 背景 + 本地排字
            </SelectItem>
            <SelectItem value="embedded">
              AI 整图封面
            </SelectItem>
          </SelectContent>
        </Select>
        <p
          id="automation-cover-mode-hint"
          class="text-xs text-muted-foreground"
        >
          {{ selectedCoverTextMode === 'embedded' ? '按校核后的主标题、副标题和品牌文字生成完整封面，自动采用并导出，不再叠字。AI 中文可能有误，发布前请检查三张封面。' : 'AI 生成无字背景，再由本地排版叠加标题、副标题和品牌文字。' }}
        </p>
      </div>
    </div>
    <p class="text-sm text-muted-foreground">
      根据整书或章节内容自然展开，不设目标时长；制作按实际音频完整时长继续。4:3、3:4与16:9各一张{{ coverImageLabel }}，北京时间最近未来01:11定时，不允许保存视频。
    </p>
    <div class="grid min-w-0 gap-4 rounded-lg border border-border bg-background/40 p-4">
      <div>
        <h3 class="text-sm font-semibold">
          本次发布账号与合集
        </h3>
        <p class="mt-1 text-sm text-muted-foreground">
          在这里确认一次；开启下方自动沿用后，重开窗口或重新登录仍会核对账号并自动继续。账号变化或合集无法匹配时会暂停。
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
          <SavedOptionInput
            id="automation-collection-name"
            v-model="collectionName"
            label="合集"
            :options="collectionOptions"
            :save-options="saveCollections"
            placeholder="选择或输入已有合集名称，回车保存"
            :maxlength="200"
            :disabled="busy || active || !collectionOptionsReady"
            described-by="automation-collection-hint"
            footer="仅管理本地候选，不会创建、重命名或删除抖音上的合集，也不改变已启动任务。"
          />
          <div
            v-if="collectionOptionsError"
            class="flex flex-wrap items-center gap-2"
          >
            <p
              role="alert"
              class="text-sm text-destructive"
            >
              {{ collectionOptionsError }}
            </p>
            <Button
              variant="outline"
              size="sm"
              :disabled="busy || active"
              @click="loadCollectionOptions"
            >
              重新读取合集选项
            </Button>
          </div>
          <p
            id="automation-collection-hint"
            class="text-sm text-muted-foreground"
          >
            候选保存在本地，可跨书籍复用。请选择当前账号已有的完整合集名称；上传后自动匹配官方唯一同名合集，无法匹配时会暂停提示。
          </p>
        </template>
      </div>
    </div>
    <AutomationAccountPreference
      v-if="isDesktop()"
      :disabled="busy"
      @busy="preferenceBusy = $event"
    />
    <label class="flex items-start gap-3 text-sm">
      <Checkbox
        v-model="consent"
        :disabled="busy || active"
        aria-label="允许自动制作和上传"
      />
      <span>允许将本次书名、作者及整书资料或章节正文发送至文本模型，生成创作方案；将原书文件或章节文字及完整音频指导发送至 Notebook，再根据实际字幕校核并渲染视频。{{ activeRun?.input.selection.detailedInk ? '本历史任务仍按原授权逐镜生成水墨画作，费用与镜头张数有关；继续时只处理未完成画面。' : '' }}允许按所选方式生成并采用三种比例的{{ coverImageLabel }}、导出封面和按上方账号与合集准备上传，最终发布由我完成。模型失败后最多自动重试3次，再切换其他已启用的同类模型，可能产生额外费用；已完成成果保留。</span>
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
      :disabled="busy || preferenceBusy || disabled || active || !consent || !textProvider || !accountReady || !collectionValid || !imageProvider || !isDesktop()"
      @click="start"
    >
      {{ busy ? '正在准备…' : '自动制作并准备发布' }}
    </Button>
  </section>
</template>
