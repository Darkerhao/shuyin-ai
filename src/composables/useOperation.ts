import { ref } from 'vue'
import { errorMessage } from '../services/verification'
export function useOperation() {
  const busy = ref(false)
  const error = ref('')
  const message = ref('')
  const operation = ref('')
  async function run(action: () => Promise<void>, label = '') {
    if (busy.value) return
    busy.value = true
    operation.value = label
    error.value = ''
    message.value = ''
    try { await action() } catch (cause) { error.value = errorMessage(cause) } finally { busy.value = false; operation.value = '' }
  }
  return { busy, error, message, operation, run }
}
