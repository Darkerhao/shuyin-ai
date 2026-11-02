<script setup lang="ts">
import { onBeforeUnmount } from 'vue'
import ConfirmAction from './ConfirmAction.vue'
import { confirmation, settleConfirmation } from '../composables/useConfirmation'

onBeforeUnmount(() => settleConfirmation(false))
</script>

<template>
  <ConfirmAction
    v-if="confirmation"
    title="请确认操作"
    confirm-label="确认继续"
    :destructive="false"
    :return-focus="confirmation.returnFocus"
    @confirm="settleConfirmation(true)"
    @cancel="settleConfirmation(false)"
  >
    <p>{{ confirmation.message }}</p>
  </ConfirmAction>
</template>
