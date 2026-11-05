<script setup lang="ts">
import type { TabsListProps } from "reka-ui"
import type { HTMLAttributes } from "vue"
import { reactiveOmit } from "@vueuse/core"
import { cn } from "@/lib/utils"
import { TabsList } from "reka-ui"

const props = defineProps<TabsListProps & { class?: HTMLAttributes["class"] }>()

const delegatedProps = reactiveOmit(props, "class")
</script>

<template>
  <TabsList
    data-slot="tabs-list"
    v-bind="delegatedProps"
    :class="cn(
      'bg-[var(--panel-surface)] text-muted-foreground inline-flex min-h-[52px] w-fit items-center justify-center gap-1.5 rounded-xl border p-1.5 backdrop-blur-xl',
      props.class,
    )"
  >
    <slot />
  </TabsList>
</template>
