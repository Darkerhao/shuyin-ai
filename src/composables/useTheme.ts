import { readonly, ref } from 'vue'

type Theme = 'dark' | 'light'
const storageKey = 'shuyin.theme'
const theme = ref<Theme>('dark')
const error = ref('')

function applyTheme(value: Theme) {
  theme.value = value
  globalThis.document.documentElement.classList.toggle('dark', value === 'dark')
}

export function initializeTheme() {
  try {
    applyTheme(globalThis.localStorage.getItem(storageKey) === 'light' ? 'light' : 'dark')
    error.value = ''
  } catch {
    applyTheme('dark')
    error.value = '主题偏好读取失败，本次使用深色风格。'
  }
}

function toggleTheme() {
  const next = theme.value === 'dark' ? 'light' : 'dark'
  try {
    globalThis.localStorage.setItem(storageKey, next)
    applyTheme(next)
    error.value = ''
  } catch {
    error.value = '主题偏好未保存，请重试。'
  }
}

export const useTheme = () => ({ theme: readonly(theme), error: readonly(error), toggleTheme })
