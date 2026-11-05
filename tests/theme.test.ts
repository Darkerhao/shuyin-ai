import { beforeEach, expect, it, vi } from 'vitest'
import { initializeTheme, useTheme } from '../src/composables/useTheme'

beforeEach(() => {
  localStorage.clear()
  document.documentElement.className = 'dark'
  initializeTheme()
})

it('keeps the existing dark appearance until a theme is chosen', () => {
  expect(useTheme().theme.value).toBe('dark')
  expect(document.documentElement.classList.contains('dark')).toBe(true)
  expect(localStorage.getItem('shuyin.theme')).toBeNull()
})

it('restores the chosen theme on startup and toggles both ways', () => {
  const { theme, toggleTheme } = useTheme()
  toggleTheme()
  expect(theme.value).toBe('light')
  expect(document.documentElement.classList.contains('dark')).toBe(false)
  expect(localStorage.getItem('shuyin.theme')).toBe('light')
  document.documentElement.classList.add('dark')
  initializeTheme()
  expect(document.documentElement.classList.contains('dark')).toBe(false)
  toggleTheme()
  expect(theme.value).toBe('dark')
  expect(document.documentElement.classList.contains('dark')).toBe(true)
  expect(localStorage.getItem('shuyin.theme')).toBe('dark')
})

it('ignores an unknown stored value without affecting other document classes', () => {
  localStorage.setItem('shuyin.theme', 'unknown')
  document.documentElement.className = 'custom-class'
  initializeTheme()
  expect(useTheme().theme.value).toBe('dark')
  expect(document.documentElement.classList.contains('custom-class')).toBe(true)
})

it('leaves the saved appearance in place on write failure and allows retry', () => {
  const { theme, toggleTheme, error } = useTheme()
  const write = vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => { throw new Error('unavailable') })
  toggleTheme()
  expect(theme.value).toBe('dark')
  expect(document.documentElement.classList.contains('dark')).toBe(true)
  expect(error.value).toBe('主题偏好未保存，请重试。')
  write.mockRestore()
  toggleTheme()
  expect(theme.value).toBe('light')
  expect(error.value).toBe('')
})

it('can start when local storage cannot be read', () => {
  vi.spyOn(Storage.prototype, 'getItem').mockImplementation(() => { throw new Error('unavailable') })
  expect(initializeTheme).not.toThrow()
  expect(useTheme().theme.value).toBe('dark')
  expect(useTheme().error.value).toContain('读取失败')
})
