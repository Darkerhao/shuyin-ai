<script setup lang="ts">
import type { TabsTriggerProps } from "reka-ui"
import type { HTMLAttributes } from "vue"
import { reactiveOmit } from "@vueuse/core"
import { cn } from "@/lib/utils"
import { TabsTrigger, useForwardProps } from "reka-ui"

const props = defineProps<TabsTriggerProps & { class?: HTMLAttributes["class"] }>()

const delegatedProps = reactiveOmit(props, "class")

const forwardedProps = useForwardProps(delegatedProps)
</script>

<template>
  <TabsTrigger
    data-slot="tabs-trigger"
    :class="cn(
      `data-[state=active]:bg-secondary data-[state=active]:text-primary data-[state=active]:border data-[state=active]:border-primary/40 data-[state=active]:shadow-[var(--primary-glow)] focus-visible:border-ring focus-visible:ring-ring/50 focus-visible:outline-ring text-muted-foreground inline-flex h-[38px] flex-1 items-center justify-center gap-2 rounded-lg border-0 px-0 py-2.5 text-xs font-semibold whitespace-nowrap transition-[color,background-color,border-color,box-shadow] hover:text-foreground focus-visible:ring-3 focus-visible:outline-1 disabled:pointer-events-none disabled:opacity-50 [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4`,
      props.class,
    )"
    v-bind="forwardedProps"
  >
    <slot />
  </TabsTrigger>
</template>
