<script setup lang="ts">
import { computed } from 'vue'
import { now, parseDateTime, toCalendarDateTime, type DateValue } from '@internationalized/date'
import {
  DatePickerRoot, DatePickerTrigger, DatePickerContent, DatePickerCalendar,
  DatePickerHeader, DatePickerPrev, DatePickerHeading, DatePickerNext,
  DatePickerGrid, DatePickerGridHead, DatePickerGridRow, DatePickerHeadCell,
  DatePickerGridBody, DatePickerCell, DatePickerCellTrigger, DatePickerClose,
} from 'reka-ui'
import AppIcon from '@/components/AppIcon.vue'
import { Button } from '@/components/ui/button'
import { Select, SelectTrigger, SelectValue, SelectContent, SelectItem } from '@/components/ui/select'

defineOptions({ name: 'UiDateTimePicker', inheritAttrs: false })
const props = withDefaults(defineProps<{ id: string; modelValue: string; disabled?: boolean; timeZone?: string }>(), { timeZone: 'Asia/Shanghai' })
const emit = defineEmits<{ 'update:modelValue': [value: string] }>()
// Edit wall time only. Conversion to an instant stays at the business boundary.
const placeholder = toCalendarDateTime(now(props.timeZone)).set({ second: 0, millisecond: 0 })
const value = computed({
  get: () => props.modelValue ? parseDateTime(props.modelValue) : undefined,
  set: (date: DateValue | undefined) => emit('update:modelValue', date ? toCalendarDateTime(date).toString().slice(0, 16) : ''),
})
const pad = (value: number) => String(value).padStart(2, '0')
function setTime(part: 'hour' | 'minute', time: number) {
  value.value = (value.value ?? placeholder).set({ [part]: time })
}
</script>

<template>
  <DatePickerRoot
    v-model="value"
    :default-placeholder="value ?? placeholder"
    :disabled="disabled"
    :hour-cycle="24"
    granularity="minute"
    locale="zh-CN"
    :week-starts-on="1"
    prevent-deselect
    fixed-weeks
  >
    <DatePickerTrigger as-child>
      <Button
        :id="id"
        v-bind="$attrs"
        type="button"
        variant="outline"
        class="date-time-trigger h-[42px] w-full min-w-0 justify-between font-mono text-xs"
        :disabled="disabled"
      >
        <span :class="{ 'text-muted-foreground': !modelValue }">{{ modelValue ? modelValue.replace('T', ' ') : '请选择日期和时间' }}</span>
        <AppIcon name="clock" />
      </Button>
    </DatePickerTrigger>
    <DatePickerContent
      align="start"
      :side-offset="8"
      :collision-padding="12"
      class="date-time-panel z-50 max-h-(--reka-popover-content-available-height) w-72 max-w-[calc(100vw-24px)] overflow-y-auto rounded-xl border border-info/40 bg-popover p-3 font-mono text-xs text-popover-foreground shadow-[var(--overlay-shadow)]"
      aria-label="选择日期和时间"
    >
      <DatePickerCalendar v-slot="{ grid, weekDays }">
        <DatePickerHeader class="mb-2 flex items-center justify-between gap-2">
          <DatePickerPrev as-child>
            <Button
              type="button"
              variant="ghost"
              size="icon-sm"
              aria-label="上个月"
            >
              <AppIcon name="back" />
            </Button>
          </DatePickerPrev>
          <DatePickerHeading class="font-semibold" />
          <DatePickerNext as-child>
            <Button
              type="button"
              variant="ghost"
              size="icon-sm"
              aria-label="下个月"
            >
              <AppIcon name="arrow" />
            </Button>
          </DatePickerNext>
        </DatePickerHeader>
        <DatePickerGrid
          v-for="month in grid"
          :key="month.value.toString()"
          class="w-full border-collapse"
        >
          <DatePickerGridHead>
            <DatePickerGridRow>
              <DatePickerHeadCell
                v-for="day in weekDays"
                :key="day"
                class="h-8 text-center font-normal text-muted-foreground"
              >
                {{ day }}
              </DatePickerHeadCell>
            </DatePickerGridRow>
          </DatePickerGridHead>
          <DatePickerGridBody>
            <DatePickerGridRow
              v-for="(week, index) in month.rows"
              :key="index"
            >
              <DatePickerCell
                v-for="day in week"
                :key="day.toString()"
                :date="day"
                class="p-0.5 text-center"
              >
                <DatePickerCellTrigger
                  :day="day"
                  :month="month.value"
                  as="button"
                  type="button"
                  class="size-8 rounded-md outline-none hover:bg-accent focus-visible:ring-2 focus-visible:ring-ring data-[outside-view]:text-muted-foreground/50 data-[selected]:bg-primary data-[selected]:text-primary-foreground data-[today]:font-bold"
                />
              </DatePickerCell>
            </DatePickerGridRow>
          </DatePickerGridBody>
        </DatePickerGrid>
      </DatePickerCalendar>
      <div class="mt-3 grid grid-cols-2 gap-3 border-t border-border pt-3">
        <div
          v-for="(count, part) in { hour: 24, minute: 60 }"
          :key="part"
        >
          <label
            :for="`${id}-${part}`"
            class="mb-1 block text-muted-foreground"
          >{{ part === 'hour' ? '小时' : '分钟' }}</label>
          <Select
            :model-value="(value ?? placeholder)[part]"
            :disabled="disabled"
            @update:model-value="setTime(part, $event)"
          >
            <SelectTrigger
              :id="`${id}-${part}`"
              class="w-full"
            >
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem
                v-for="item in count"
                :key="item - 1"
                :value="item - 1"
              >
                {{ pad(item - 1) }}
              </SelectItem>
            </SelectContent>
          </Select>
        </div>
      </div>
      <div class="mt-3 flex items-center justify-between gap-3">
        <DatePickerClose as-child>
          <Button
            type="button"
            variant="ghost"
            size="sm"
            :disabled="disabled || !modelValue"
            @click="value = undefined"
          >
            清空
          </Button>
        </DatePickerClose>
        <DatePickerClose as-child>
          <Button
            type="button"
            size="sm"
            :disabled="disabled || !modelValue"
          >
            确定
          </Button>
        </DatePickerClose>
      </div>
    </DatePickerContent>
  </DatePickerRoot>
</template>
