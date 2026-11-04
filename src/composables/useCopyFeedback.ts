import { onBeforeUnmount, ref } from 'vue'
export function useCopyFeedback() {
  const copied = ref('')
  const error = ref('')
  let timer: ReturnType<typeof globalThis.setTimeout> | undefined
  let request = 0
  async function copy(text: string, key: string) {
    const current = ++request
    globalThis.clearTimeout(timer); copied.value = ''; error.value = ''
    try {
      await globalThis.navigator.clipboard.writeText(text)
      if (current !== request) return
      copied.value = key
      timer = globalThis.setTimeout(() => { copied.value = '' }, 800)
    } catch { if (current === request) error.value = '复制失败，请选中文字后手动复制。' }
  }
  onBeforeUnmount(() => { request++; globalThis.clearTimeout(timer) })
  return { copied, error, copy }
}
