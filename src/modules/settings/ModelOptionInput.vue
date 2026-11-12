<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'
import { PopoverRoot, PopoverAnchor, PopoverPortal, PopoverContent } from 'reka-ui'
import { Input } from '@/components/ui/input'
import { Button } from '@/components/ui/button'
import AppIcon from '@/components/AppIcon.vue'

const props = defineProps<{
  id: string
  label: string
  modelValue: string
  options: string[]
  saveOptions: (values: string[]) => Promise<string[]>
  disabled: boolean
  placeholder: string
  type?: string
  maxlength?: number
  describedBy?: string
}>()
const emit = defineEmits<{ 'update:modelValue': [value: string] }>()
const anchor = ref<globalThis.HTMLElement>()
const content = ref<InstanceType<typeof PopoverContent>>()
const optionButtons = () => [...((content.value?.$el as globalThis.HTMLElement | undefined)?.querySelectorAll<globalThis.HTMLButtonElement>('[data-option-select]') ?? [])]
const open = ref(false)
const query = ref('')
const editing = ref<string | null>(null)
const editValue = ref('')
const saving = ref(false)
const error = ref('')
const message = ref('')
const blocked = computed(() => props.disabled || saving.value)
const filtered = computed(() => props.options.filter(value => value.toLocaleLowerCase().includes(query.value.trim().toLocaleLowerCase())))
const newValue = computed(() => props.modelValue.trim())
const canAdd = computed(() => !!newValue.value && !props.options.includes(newValue.value))
const focusInput = () => globalThis.document.getElementById(props.id)?.focus({ preventScroll: true })
watch(open, value => { if (!value) { editing.value = null; query.value = '' } })
watch(() => props.modelValue, () => { error.value = ''; message.value = '' })

function showOptions(resetQuery = true) {
  if (blocked.value) return
  if (resetQuery) query.value = ''
  open.value = true
}
function input(value: string | number) {
  emit('update:modelValue', String(value))
  query.value = String(value)
  open.value = true
  editing.value = null
}
function choose(value: string) {
  emit('update:modelValue', value)
  open.value = false
  focusInput()
}
async function persist(values: string[], onSuccess: (saved: string[]) => void) {
  if (blocked.value) return
  saving.value = true
  error.value = ''; message.value = ''
  try {
    const saved = await props.saveOptions(values)
    onSuccess(saved)
    await nextTick()
    message.value = '选项已保存'
  } catch (cause) {
    error.value = cause instanceof Error ? cause.message : String(cause)
  } finally { saving.value = false; await nextTick() }
}
async function add() {
  if (!newValue.value || blocked.value) return
  if (props.options.includes(newValue.value)) {
    choose(newValue.value)
    return
  }
  await persist([newValue.value, ...props.options], saved => {
    emit('update:modelValue', saved[0]!)
    query.value = ''; open.value = false
  })
  focusInput()
}
async function startEdit(value: string) {
  editing.value = value; editValue.value = value; error.value = ''; message.value = ''
  await nextTick()
  const element = globalThis.document.getElementById(`${props.id}-edit`) as globalThis.HTMLInputElement | null
  element?.focus(); element?.select()
}
async function saveEdit() {
  if (editing.value === null || !editValue.value.trim()) return
  const previous = editing.value
  await persist([editValue.value, ...props.options.filter(value => value !== previous)], saved => {
    if (props.modelValue === previous) emit('update:modelValue', saved[0]!)
    editing.value = null; query.value = ''
  })
  if (!error.value) focusInput()
}
async function remove(value: string) {
  await persist(props.options.filter(option => option !== value), () => {
    editing.value = null
  })
  focusInput()
}
async function onKeydown(event: globalThis.KeyboardEvent) {
  if (event.key === 'Enter') {
    event.preventDefault()
    if (!event.isComposing && event.keyCode !== 229) await add()
  } else if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
    event.preventDefault()
    showOptions(false)
    await nextTick()
    const buttons = optionButtons()
    buttons[event.key === 'ArrowDown' ? 0 : buttons.length - 1]?.focus()
  } else if (event.key === 'Escape') { open.value = false }
}
function moveOption(event: globalThis.KeyboardEvent) {
  if (!['ArrowDown', 'ArrowUp'].includes(event.key) || !(event.target instanceof globalThis.HTMLElement) || !event.target.hasAttribute('data-option-select')) return
  event.preventDefault()
  const buttons = optionButtons()
  const index = buttons.indexOf(event.target as globalThis.HTMLButtonElement) + (event.key === 'ArrowDown' ? 1 : -1)
  if (buttons[index]) buttons[index].focus()
  else focusInput()
}
function cancelEdit() {
  editing.value = null; open.value = false; error.value = ''
  focusInput()
}
</script>

<template>
  <div class="model-option-input">
    <PopoverRoot v-model:open="open">
      <PopoverAnchor as-child>
        <div
          ref="anchor"
          class="option-anchor"
        >
          <Input
            :id="id"
            :model-value="modelValue"
            :type="type ?? 'text'"
            :maxlength="maxlength"
            :placeholder="placeholder"
            :disabled="blocked"
            :aria-describedby="[describedBy, `${id}-option-help`, `${id}-option-feedback`].filter(Boolean).join(' ')"
            :aria-invalid="!!error"
            :aria-expanded="open"
            :aria-controls="open ? `${id}-options` : undefined"
            role="combobox"
            aria-haspopup="dialog"
            aria-autocomplete="list"
            autocomplete="off"
            spellcheck="false"
            required
            class="pr-12"
            @update:model-value="input"
            @click="showOptions"
            @keydown="onKeydown"
          />
          <Button
            type="button"
            variant="ghost"
            size="icon-sm"
            :disabled="blocked"
            :aria-label="`展开${label}选项`"
            :aria-expanded="open"
            @click="open ? open = false : showOptions()"
          >
            <AppIcon
              name="chevron"
              class="rotate-90"
            />
          </Button>
        </div>
      </PopoverAnchor>
      <PopoverPortal>
        <PopoverContent
          ref="content"
          :data-model-options="id"
          :aria-label="`${label}选项`"
          :aria-busy="saving"
          class="model-options-popover"
          align="start"
          :side-offset="6"
          :collision-padding="12"
          @open-auto-focus.prevent
          @close-auto-focus.prevent
          @interact-outside="anchor?.contains($event.target as Node) && $event.preventDefault()"
          @escape-key-down="cancelEdit"
          @keydown="moveOption"
        >
          <div :id="`${id}-options`">
            <p class="option-heading">
              已保存的{{ label }} <span>{{ options.length }}</span>
            </p>
            <div class="option-list">
              <div
                v-for="value in filtered"
                :key="value"
                class="option-row"
              >
                <template v-if="editing === value">
                  <Input
                    :id="`${id}-edit`"
                    v-model="editValue"
                    :aria-label="`编辑${label}选项`"
                    :disabled="blocked"
                    :maxlength="maxlength"
                    :aria-describedby="`${id}-option-feedback`"
                    @keydown.enter.prevent="!$event.isComposing && $event.keyCode !== 229 && saveEdit()"
                  />
                  <Button
                    type="button"
                    variant="ghost"
                    size="icon-sm"
                    :disabled="blocked || !editValue.trim()"
                    aria-label="保存选项修改"
                    title="保存修改"
                    @click="saveEdit"
                  >
                    <AppIcon name="check" />
                  </Button>
                  <Button
                    type="button"
                    variant="ghost"
                    size="icon-sm"
                    :disabled="blocked"
                    aria-label="取消选项修改"
                    title="取消修改"
                    @click="cancelEdit"
                  >
                    <AppIcon name="close" />
                  </Button>
                </template>
                <template v-else>
                  <Button
                    type="button"
                    variant="ghost"
                    class="option-value"
                    data-option-select
                    :disabled="blocked"
                    :title="value"
                    :aria-label="`选择 ${value}`"
                    @click="choose(value)"
                  >
                    <span>{{ value }}</span><AppIcon
                      v-if="value === modelValue"
                      name="check"
                    />
                  </Button>
                  <Button
                    type="button"
                    variant="ghost"
                    size="icon-sm"
                    :disabled="blocked"
                    :aria-label="`编辑选项 ${value}`"
                    title="编辑选项"
                    @click="startEdit(value)"
                  >
                    <AppIcon name="pen" />
                  </Button>
                  <Button
                    type="button"
                    variant="ghost"
                    size="icon-sm"
                    :disabled="blocked"
                    :aria-label="`删除选项 ${value}`"
                    title="删除选项"
                    @click="remove(value)"
                  >
                    <AppIcon name="trash" />
                  </Button>
                </template>
              </div>
              <p
                v-if="!filtered.length"
                class="option-empty"
              >
                {{ options.length ? '没有匹配的选项，输入后按回车保存。' : '暂无选项，输入后按回车保存。' }}
              </p>
            </div>
            <Button
              v-if="canAdd"
              type="button"
              variant="ghost"
              class="option-add"
              :disabled="blocked"
              @click="add"
            >
              <AppIcon name="plus" /><span>保存“{{ newValue }}”</span><kbd>Enter</kbd>
            </Button>
            <p class="option-footer">
              编辑或删除选项不改变已保存的模型配置。
            </p>
            <p
              v-if="error"
              class="option-error"
            >
              {{ error }}
            </p>
          </div>
        </PopoverContent>
      </PopoverPortal>
    </PopoverRoot>
    <div class="option-help-row">
      <span :id="`${id}-option-help`">输入后回车保存选项，下拉可选择、编辑或删除。</span>
      <span
        :id="`${id}-option-feedback`"
        :role="error ? 'alert' : 'status'"
        :class="error ? 'option-error' : 'option-saved'"
      >{{ error || (saving ? '正在保存…' : message) }}</span>
    </div>
  </div>
</template>

<style scoped>
.model-option-input { min-width: 0; }
.option-anchor { position: relative; }
.option-anchor > button { position: absolute; right: 5px; top: 5px; color: var(--muted-foreground); }
:global(.model-options-popover) { z-index: 50; width: var(--reka-popover-trigger-width); max-width: calc(100vw - 24px); max-height: var(--reka-popover-content-available-height); overflow-y: auto; border: 1px solid var(--border); border-radius: 12px; padding: 8px; color: var(--foreground); background: var(--popover); box-shadow: var(--overlay-shadow); }
.option-heading, .option-footer, .option-empty { margin: 0; padding: 8px; color: var(--muted-foreground); font-size: 12px; line-height: 18px; }
.option-heading { display: flex; justify-content: space-between; }
.option-list { max-height: 240px; overflow-y: auto; }
.option-row { display: flex; align-items: center; gap: 4px; padding-block: 2px; }
.option-row > input { height: 36px; padding: 8px; }
.option-value { min-width: 0; flex: 1; justify-content: space-between; padding: 9px 8px; font-family: var(--font-mono); }
.option-value > span, .option-add > span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.option-add { width: 100%; justify-content: flex-start; margin-top: 4px; padding: 10px 8px; color: var(--primary); }
.option-add kbd { margin-left: auto; font-size: 11px; color: var(--muted-foreground); }
.option-footer { margin-top: 4px; border-top: 1px solid var(--border); }
.option-help-row { display: flex; flex-wrap: wrap; gap: 3px 8px; margin-top: 6px; color: var(--muted-foreground); font-size: 12px; line-height: 18px; }
.option-error { color: var(--destructive); font-size: 12px; overflow-wrap: anywhere; }
.option-saved { color: var(--primary); }
</style>
