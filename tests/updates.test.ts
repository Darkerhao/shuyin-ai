import { beforeEach, describe, expect, it, vi } from 'vitest'
import { defineComponent } from 'vue'
import { flushPromises, mount } from '@vue/test-utils'
import { createMemoryHistory, createRouter } from 'vue-router'
import type { DownloadEvent, Update } from '@tauri-apps/plugin-updater'
import * as api from '../src/services/updates'

vi.mock('../src/services/updates', () => ({
  isDesktop: vi.fn(), appVersion: vi.fn(), autoUpdateEnabled: vi.fn(), setAutoUpdate: vi.fn(),
  checkUpdate: vi.fn(), restartApp: vi.fn(), currentReleaseNotes: '- 当前版本：新增关于页面',
}))

beforeEach(() => {
  vi.resetModules()
  vi.resetAllMocks()
  vi.mocked(api.isDesktop).mockReturnValue(true)
  vi.mocked(api.appVersion).mockResolvedValue('0.11.0')
  vi.mocked(api.autoUpdateEnabled).mockResolvedValue(true)
  vi.mocked(api.setAutoUpdate).mockResolvedValue(undefined)
  vi.mocked(api.checkUpdate).mockResolvedValue(null)
  vi.mocked(api.restartApp).mockResolvedValue(undefined)
})

function candidate() {
  return {
    version: '0.12.0', body: '- 自动更新\n- 修复导出 <script>alert(1)</script>', date: '2026-10-25T00:00:00Z',
    close: vi.fn().mockResolvedValue(undefined),
    download: vi.fn().mockResolvedValue(undefined), install: vi.fn().mockResolvedValue(undefined),
  }
}
function offer(update: ReturnType<typeof candidate>) { vi.mocked(api.checkUpdate).mockResolvedValue(update as unknown as Update) }
async function updates() { return (await import('../src/composables/useAppUpdates')).useAppUpdates() }
const PageHost = defineComponent({ template: '<RouterView />' })

describe('application updates', () => {
  it('checks and downloads once at startup, waiting for confirmation to install', async () => {
    const update = candidate(); offer(update)
    update.download.mockImplementation(async (onEvent: (event: DownloadEvent) => void) => {
      onEvent({ event: 'Started', data: { contentLength: 100 } })
      onEvent({ event: 'Progress', data: { chunkLength: 40 } })
      onEvent({ event: 'Progress', data: { chunkLength: 60 } })
      onEvent({ event: 'Finished' })
    })
    const state = await updates()
    await Promise.all([state.initialize(), state.initialize()])
    expect(api.checkUpdate).toHaveBeenCalledTimes(1)
    expect(update.download).toHaveBeenCalledTimes(1)
    expect(state.status.value).toBe('ready')
    expect(state.progress.value).toBe(100)
    expect(update.install).not.toHaveBeenCalled()
    await state.checkForUpdates()
    expect(update.close).not.toHaveBeenCalled()
    await state.installUpdate()
    expect(update.install).toHaveBeenCalledTimes(1)
    expect(api.restartApp).toHaveBeenCalledTimes(1)
  })

  it('retains manual checking and downloading when automatic updates are disabled', async () => {
    vi.mocked(api.autoUpdateEnabled).mockResolvedValue(false)
    const update = candidate(); offer(update)
    const state = await updates(); await state.initialize()
    expect(api.checkUpdate).not.toHaveBeenCalled()
    await state.checkForUpdates()
    expect(state.status.value).toBe('available')
    expect(update.download).not.toHaveBeenCalled()
    await state.downloadUpdate()
    expect(state.status.value).toBe('ready')
  })

  it('releases old update resources and prevents overlapping requests', async () => {
    vi.mocked(api.autoUpdateEnabled).mockResolvedValue(false)
    const update = candidate(); offer(update)
    const state = await updates(); await state.initialize(); await state.checkForUpdates()
    let finish!: (result: null) => void
    vi.mocked(api.checkUpdate).mockReturnValue(new Promise(resolve => { finish = resolve }))
    const pending = state.checkForUpdates()
    await flushPromises()
    await state.checkForUpdates()
    expect(api.checkUpdate).toHaveBeenCalledTimes(2)
    expect(update.close).toHaveBeenCalledTimes(1)
    finish(null); await pending
    expect(state.status.value).toBe('current')
  })

  it('allows retry after network and signature failures without installing', async () => {
    const state = await updates()
    vi.mocked(api.checkUpdate).mockRejectedValueOnce(new Error('offline'))
    await state.initialize()
    expect(state.error.value).toContain('offline')
    const update = candidate(); offer(update)
    await state.checkForUpdates()
    update.download.mockRejectedValueOnce(new Error('signature mismatch'))
    await state.downloadUpdate()
    expect(state.status.value).toBe('available')
    expect(state.error.value).toContain('signature mismatch')
    await state.installUpdate()
    expect(update.install).not.toHaveBeenCalled()
    await state.downloadUpdate()
    expect(state.status.value).toBe('ready')
    expect(state.error.value).toBe('')
  })

  it('does not reinstall after an installed update fails to restart', async () => {
    const update = candidate(); offer(update)
    const state = await updates(); await state.initialize()
    vi.mocked(api.restartApp).mockRejectedValueOnce(new Error('restart failed'))
    await state.installUpdate()
    expect(state.status.value).toBe('restart')
    expect(state.error.value).toContain('关闭后重新打开')
    await state.installUpdate()
    expect(update.install).toHaveBeenCalledTimes(1)
    expect(api.restartApp).toHaveBeenCalledTimes(2)
  })

  it('keeps the downloaded update available after an installation failure', async () => {
    const update = candidate(); offer(update)
    const state = await updates(); await state.initialize()
    update.install.mockRejectedValueOnce(new Error('installer blocked'))
    await state.installUpdate()
    expect(state.status.value).toBe('ready')
    expect(state.error.value).toContain('installer blocked')
    expect(api.restartApp).not.toHaveBeenCalled()
    await state.installUpdate()
    expect(update.install).toHaveBeenCalledTimes(2)
  })

  it('can retry loading update preferences without starting checks before settings are known', async () => {
    vi.mocked(api.autoUpdateEnabled).mockRejectedValueOnce(new Error('database unavailable'))
    const state = await updates(); await state.initialize()
    expect(state.initialized.value).toBe(false)
    expect(api.checkUpdate).not.toHaveBeenCalled()
    expect(state.settingsError.value).toContain('database unavailable')
    await state.initialize()
    expect(state.initialized.value).toBe(true)
    expect(state.settingsError.value).toBe('')
    expect(api.checkUpdate).toHaveBeenCalledTimes(1)
  })

  it('does not download when the preference is disabled during a pending background check', async () => {
    let finish!: (result: Update) => void
    vi.mocked(api.checkUpdate).mockReturnValue(new Promise(resolve => { finish = resolve }))
    const state = await updates()
    const startup = state.initialize(); await flushPromises()
    await state.setAutomatic(false)
    const update = candidate(); finish(update as unknown as Update); await startup
    expect(state.automatic.value).toBe(false)
    expect(update.download).not.toHaveBeenCalled()
    vi.mocked(api.setAutoUpdate).mockRejectedValueOnce(new Error('database locked'))
    await state.setAutomatic(true)
    expect(state.automatic.value).toBe(false)
    expect(state.settingsError.value).toContain('database locked')
  })

  it('never invokes desktop commands in web preview', async () => {
    vi.mocked(api.isDesktop).mockReturnValue(false)
    const state = await updates(); await state.initialize(); await state.checkForUpdates()
    expect(api.appVersion).not.toHaveBeenCalled()
    expect(api.checkUpdate).not.toHaveBeenCalled()
  })

  it('renders safe release notes and installs only after explicit confirmation', async () => {
    const update = candidate(); offer(update)
    const AboutPage = (await import('../src/modules/settings/AboutPage.vue')).default
    const router = createRouter({ history: createMemoryHistory(), routes: [{ path: '/about', component: AboutPage }] })
    await router.push('/about'); await router.isReady()
    const wrapper = mount(PageHost, { global: { plugins: [router] } })
    try {
      await flushPromises()
      expect(wrapper.text()).toContain('v0.11.0')
      expect(wrapper.text()).toContain('自动更新')
      expect(wrapper.find('script').exists()).toBe(false)
      const button = (label: string) => wrapper.findAll('button').find(item => item.text() === label)!
      await button('安装并重启').trigger('click')
      expect(update.install).not.toHaveBeenCalled()
      await button('稍后再说').trigger('click')
      expect(wrapper.find('[role="alertdialog"]').exists()).toBe(false)
      await button('安装并重启').trigger('click')
      await button('确认并重启').trigger('click'); await flushPromises()
      expect(update.install).toHaveBeenCalledTimes(1)
    } finally { wrapper.unmount() }
  })

  it('shows current release notes even when no newer version exists', async () => {
    const AboutPage = (await import('../src/modules/settings/AboutPage.vue')).default
    const router = createRouter({ history: createMemoryHistory(), routes: [{ path: '/about', component: AboutPage }] })
    await router.push('/about'); await router.isReady()
    const wrapper = mount(PageHost, { global: { plugins: [router] } })
    try {
      await flushPromises()
      expect(wrapper.text()).toContain('当前已是最新版本')
      expect(wrapper.text()).toContain('当前版本：新增关于页面')
    } finally { wrapper.unmount() }
  })
})
