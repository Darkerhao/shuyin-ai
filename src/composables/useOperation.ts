import { ref } from 'vue'
import { errorMessage } from '../services/verification'
export function useOperation() {
  const busy = ref(false)
  const error = ref('')
  const message = ref('')
  async function run(action: () => Promise<void>) {
    if (busy.value) return
    busy.value = true
    error.value = ''
    message.value = ''
    try { await action() } catch (cause) { error.value = errorMessage(cause) } finally { busy.value = false }
  }
  return { busy, error, message, run }
}
