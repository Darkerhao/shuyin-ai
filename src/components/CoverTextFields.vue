<script setup lang="ts">
import { Input } from '@/components/ui/input'
import { Textarea } from '@/components/ui/textarea'

defineProps<{ idPrefix: string }>()
const title = defineModel<string>('title', { required: true })
const subtitle = defineModel<string>('subtitle', { required: true })
const brand = defineModel<string>('brand', { required: true })
</script>

<template>
  <label :for="`${idPrefix}-title`">主标题 *</label>
  <Textarea
    :id="`${idPrefix}-title`"
    v-model="title"
    rows="2"
    maxlength="200"
    required
    placeholder="一句话，让人想了解这本书"
  />
  <label :for="`${idPrefix}-subtitle`">副标题</label>
  <Textarea
    :id="`${idPrefix}-subtitle`"
    v-model="subtitle"
    rows="2"
    maxlength="300"
    placeholder="补充书名或这一期的看点，可留空"
  />
  <label :for="`${idPrefix}-brand`">品牌标识</label>
  <Input
    :id="`${idPrefix}-brand`"
    v-model="brand"
    maxlength="60"
    placeholder="你的名字或栏目名称，可留空"
  />
</template>
