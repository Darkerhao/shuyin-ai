/* global URL, chrome, importScripts */
importScripts('background-core.js')

const CORE = globalThis.ShuyinAudioBackgroundCore
const HOSTS = ['com.shuyin.desktop.audio', 'com.shuyin.desktop.audio.dev']
const TICK_ALARM = 'shuyin-audio-tick'
const NEXT_ALARM = 'shuyin-audio-next'
const BINDING_KEY = 'taskBinding'
const STATUS_KEY = 'bridgeState'
const HOST_KEY = 'preferredNativeHost'
const HOME_URL = 'https://notebook.google.com/'
const NOTEBOOK_PATTERN = ['https://notebook.google.com/*', 'https://notebooklm.google.com/*']
const TERMINAL = new Set(['succeeded', 'cancelled'])
const WAITING = new Set(['paused', 'failed', 'waiting_user'])

let tickInFlight = null

function sleep(milliseconds) {
  return new Promise(resolve => globalThis.setTimeout(resolve, milliseconds))
}

function requestId() {
  return globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random().toString(16).slice(2)}`
}

async function stored(key) {
  const result = await chrome.storage.local.get(key)
  return result[key]
}

async function store(key, value) {
  await chrome.storage.local.set({ [key]: value })
}

async function preferredHosts() {
  const preferred = await stored(HOST_KEY)
  return preferred && HOSTS.includes(preferred) ? [preferred, ...HOSTS.filter(host => host !== preferred)] : HOSTS
}

function requestHost(host, message) {
  return new Promise((resolve, reject) => {
    let port
    let settled = false
    let timer
    const finish = (error, value) => {
      if (settled) return
      settled = true
      globalThis.clearTimeout(timer)
      try { port?.disconnect() } catch { /* The host may already be closed. */ }
      if (error) reject(error instanceof Error ? error : new Error(String(error)))
      else resolve(value)
    }
    try {
      port = chrome.runtime.connectNative(host)
    } catch (error) {
      reject(error)
      return
    }
    timer = globalThis.setTimeout(() => finish(new Error('本机音频桥接响应超时。')), 15000)
    port.onMessage.addListener(value => finish(null, value))
    port.onDisconnect.addListener(() => {
      const error = chrome.runtime.lastError?.message
      finish(new Error(error || '本机音频桥接进程已断开。'))
    })
    try { port.postMessage(message) } catch (error) { finish(error) }
  })
}

async function nativeRequest(payload) {
  const message = { ...payload, requestId: requestId() }
  const errors = []
  for (const host of await preferredHosts()) {
    try {
      const response = await requestHost(host, message)
      await store(HOST_KEY, host)
      return { host, response }
    } catch (error) {
      const label = host.endsWith('.dev') ? '开发版' : '正式版'
      errors.push(`${label}：${error?.message ?? String(error)}`)
    }
  }
  throw new Error(`无法连接书引 AI 本机桥接。\n${errors.join('\n')}`)
}

async function setStatus(patch) {
  const current = await stored(STATUS_KEY) ?? {}
  await store(STATUS_KEY, {
    connected: false,
    hostName: null,
    message: null,
    job: null,
    ...current,
    ...patch,
    updatedAt: new Date().toISOString(),
  })
}

async function getBinding() {
  return stored(BINDING_KEY) ?? null
}

async function setBinding(binding) {
  await store(BINDING_KEY, binding)
}

async function clearTerminalBinding(job) {
  const binding = await getBinding()
  if (!CORE.bindingMatchesJob(binding, job)) return
  await store(BINDING_KEY, {
    jobId: job.id,
    runId: Number(job.runId),
    tabId: binding.tabId ?? null,
    notebookUrl: job.notebookUrl ?? binding.notebookUrl ?? null,
    completed: true,
  })
}

function jobSummary(job) {
  if (!job) return null
  return { id: job.id, projectTitle: job.projectTitle, status: job.status, stage: job.stage }
}

function schedule(milliseconds, name = NEXT_ALARM) {
  chrome.alarms.create(name, { when: Date.now() + milliseconds })
}

function setupAlarms() {
  chrome.alarms.create(TICK_ALARM, { periodInMinutes: 1 })
  schedule(1000)
}

function notebookPath(value) {
  try {
    return new URL(value).pathname
  } catch {
    return ''
  }
}

async function getUsableTab(tabId, job) {
  if (!Number.isInteger(tabId)) return null
  try {
    const tab = await chrome.tabs.get(tabId)
    if (/^https:\/\/accounts\.google\.com\//i.test(tab.url ?? '')) return tab
    if (!CORE.isNotebookUrl(tab.url)) return null
    if (job.notebookUrl && !CORE.sameNotebook(tab.url, job.notebookUrl)) return null
    return tab
  } catch {
    return null
  }
}

async function ensureTaskTab(job) {
  const binding = await getBinding()
  if (CORE.bindingMatchesJob(binding, job)) {
    const existing = await getUsableTab(binding.tabId, job)
    if (existing) return { binding, tab: existing, fresh: false }
  }

  const tabs = await chrome.tabs.query({ url: NOTEBOOK_PATTERN })
  let tab
  if (job.notebookUrl) {
    tab = tabs.find(item => CORE.sameNotebook(item.url, job.notebookUrl)) ?? null
  } else {
    tab = tabs.find(item => CORE.isNotebookUrl(item.url) && notebookPath(item.url) === '/') ?? null
  }

  const next = {
    ...(CORE.bindingMatchesJob(binding, job) ? binding : {}),
    jobId: job.id,
    runId: Number(job.runId),
    tabId: tab?.id ?? null,
    notebookUrl: job.notebookUrl ?? tab?.url ?? null,
    completed: false,
  }
  if (!tab) {
    tab = await chrome.tabs.create({ url: job.notebookUrl || HOME_URL, active: true })
    next.tabId = tab.id
    next.notebookUrl = job.notebookUrl || tab.url || HOME_URL
    await waitForTab(tab.id)
  }
  await setBinding(next)
  return { binding: next, tab, fresh: true }
}

async function waitForTab(tabId) {
  for (let attempt = 0; attempt < 20; attempt += 1) {
    try {
      const tab = await chrome.tabs.get(tabId)
      if (tab.status === 'complete') return tab
    } catch { /* The tab may still be opening. */ }
    await sleep(250)
  }
  return null
}

async function sendStep(tabId, job) {
  for (let attempt = 0; attempt < 2; attempt += 1) {
    try {
      return await chrome.tabs.sendMessage(tabId, { type: 'RUN_STEP', job })
    } catch (error) {
      if (attempt === 0) await sleep(800)
      else throw error
    }
  }
  throw new Error('无法连接 Gemini Notebook 标签页。')
}

async function observe(job, observation) {
  const binding = await getBinding()
  if (!CORE.bindingMatchesJob(binding, job)) return null
  const { response } = await nativeRequest({ action: 'observe', jobId: job.id, runId: job.runId, observation })
  return response
}

async function reportError(job, code, message, stage = job.stage) {
  const binding = await getBinding()
  if (!CORE.bindingMatchesJob(binding, job)) return
  await nativeRequest({ action: 'report_error', jobId: job.id, runId: job.runId, code, message, stage })
}

async function sourceChunk(job, offset) {
  const binding = await getBinding()
  if (!CORE.bindingMatchesJob(binding, job)) throw new Error('旧运行任务已失效。')
  const { response } = await nativeRequest({ action: 'source_chunk', jobId: job.id, runId: job.runId, offset })
  return response
}

async function recordDownloadStarted(job, downloadId) {
  const binding = await getBinding()
  if (!CORE.bindingMatchesJob(binding, job)) return
  const next = { ...binding, downloadId, downloadStartedAt: Date.now() }
  await setBinding(next)
  await nativeRequest({ action: 'download_started', jobId: job.id, runId: job.runId, downloadId })
}

async function startDownload(job, target) {
  const binding = await getBinding()
  if (!CORE.bindingMatchesJob(binding, job)) return
  if (!/^(https?:|blob:)/i.test(target.downloadUrl ?? '')) throw new Error('网页下载地址不受支持。')
  const filename = CORE.downloadFilename(job.id, target.downloadUrl, target.suggestedFilename)
  const downloadId = await chrome.downloads.download({ url: target.downloadUrl, filename, conflictAction: 'uniquify', saveAs: false })
  if (!Number.isInteger(downloadId)) throw new Error('浏览器未返回精确下载编号。')
  await recordDownloadStarted(job, downloadId)
  schedule(3000)
}

async function reportExactDownload(job, binding) {
  if (!binding?.downloadId) return false
  const exact = CORE.exactDownloadForJob(binding, binding.downloadId, job)
  if (!exact) return false
  const items = await chrome.downloads.search({ id: binding.downloadId })
  const item = items[0]
  if (!item) return false
  if (item.state === 'in_progress') {
    schedule(5000)
    return true
  }
  if (item.state === 'interrupted') {
    await reportError(job, 'download_interrupted', item.error || '浏览器下载被中断，请重试下载。', 'downloading')
    return true
  }
  if (item.state !== 'complete' || !item.filename) return false
  try {
    const { response } = await nativeRequest({ action: 'download_complete', jobId: job.id, runId: job.runId, path: item.filename })
    if (response?.ok) {
      const current = await getBinding()
      if (CORE.bindingMatchesJob(current, job)) await setBinding({ ...current, downloadReportedAt: Date.now() })
      schedule(2500)
    } else {
      schedule(5000)
    }
  } catch {
    schedule(5000)
  }
  return true
}

async function handleStep(job, response) {
  if (!response?.ok) {
    if (response?.stale) return
    await reportError(job, response?.code ?? 'page_adapter_failed', response?.error ?? '页面自动操作失败。')
    return
  }
  const step = response.step ?? {}
  switch (step.kind) {
    case 'login_required':
    case 'login_ready':
    case 'notebook_created':
    case 'source_submitted':
    case 'source_ready':
    case 'audio_configured':
    case 'generation_submitted':
    case 'page_changed':
      await observe(job, step)
      schedule(job.stage === 'verify_remote' ? 60000 : 2500)
      break
    case 'generation_complete':
      await observe(job, { kind: 'generation_complete' })
      schedule(2500)
      break
    case 'generating':
      await observe(job, { kind: 'generating' })
      schedule(60000)
      break
    case 'download_target':
      await startDownload(job, step)
      break
    case 'navigating':
      schedule(2500)
      break
    case 'source_processing':
      schedule(job.stage === 'verify_remote' ? 60000 : 15000)
      break
    case 'waiting':
      schedule(10000)
      break
    default:
      await reportError(job, 'unknown_step', `页面适配器返回了未知步骤：${step.kind ?? '空'}`)
  }
}

async function tick() {
  if (tickInFlight) return tickInFlight
  tickInFlight = (async () => {
    try {
      const { host, response } = await nativeRequest({ action: 'poll' })
      const job = response?.job ?? null
      await setStatus({ connected: true, hostName: host, message: null, job: jobSummary(job) })
      if (!job) {
        schedule(60000)
        return
      }
      const binding = await getBinding()
      if (TERMINAL.has(job.status)) {
        await clearTerminalBinding(job)
        return
      }
      if (job.status === 'paused' || job.status === 'failed') return
      const inheritedDownload = CORE.recoverableDownloadBinding(binding, job)
      const sameJob = binding?.jobId === job.id
      const nextBinding = inheritedDownload ?? (sameJob
        ? { ...binding, runId: Number(job.runId), notebookUrl: job.notebookUrl ?? binding.notebookUrl ?? null, completed: false }
        : { jobId: job.id, runId: Number(job.runId), tabId: null, notebookUrl: job.notebookUrl ?? null, completed: false })
      if (!inheritedDownload) {
        delete nextBinding.downloadId
        delete nextBinding.downloadStartedAt
        delete nextBinding.downloadReportedAt
      }
      await setBinding(nextBinding)
      if (nextBinding.downloadId) {
        if (await reportExactDownload(job, nextBinding)) return
        await reportError(job, 'download_history_missing', '已记录的精确下载无法再查询；不会重新下载或关联其他文件，请人工核对。', job.stage)
        schedule(60000)
        return
      }
      if (WAITING.has(job.status) && job.stage !== 'verify_remote') return

      const current = await ensureTaskTab(job)
      if (job.stage === 'awaiting_extension' || job.stage === 'login') {
        if (!CORE.isNotebookUrl(current.tab.url)) {
          await observe(job, { kind: 'login_required' })
          schedule(60000)
          return
        }
        if (notebookPath(current.tab.url) !== '/') {
          await observe(job, { kind: 'login_ready' })
          schedule(2500)
          return
        }
      }
      if (job.stage === 'verify_remote' && /^https:\/\/accounts\.google\.com\//i.test(current.tab.url ?? '')) {
        await observe(job, { kind: 'login_required' })
        schedule(60000)
        return
      }

      let step
      try {
        step = await sendStep(current.tab.id, job)
      } catch {
        await setStatus({ connected: true, hostName: host, message: '等待 Gemini Notebook 页面加载完成。', job: jobSummary(job) })
        schedule(3000)
        return
      }
      await handleStep(job, step)
    } catch (error) {
      await setStatus({ connected: false, message: error?.message ?? '本机音频桥接不可用。' })
      schedule(60000)
    } finally {
      tickInFlight = null
    }
  })()
  return tickInFlight
}

chrome.runtime.onInstalled.addListener(setupAlarms)
chrome.runtime.onStartup.addListener(setupAlarms)
chrome.alarms.onAlarm.addListener(alarm => {
  if (alarm.name === TICK_ALARM || alarm.name === NEXT_ALARM) tick().catch(() => {})
})

chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
  if (message?.type === 'SOURCE_CHUNK_REQUEST') {
    ;(async () => {
      try {
        const binding = await getBinding()
        if (!binding || sender.tab?.id !== binding.tabId || binding.jobId !== message.jobId || Number(binding.runId) !== Number(message.runId)) {
          sendResponse({ ok: false, error: '文件分块请求不属于当前任务。' })
          return
        }
        sendResponse(await sourceChunk({ id: message.jobId, runId: message.runId }, message.offset))
      } catch (error) {
        sendResponse({ ok: false, error: error?.message ?? '文件分块读取失败。' })
      }
    })()
    return true
  }
  if (message?.type === 'TASK_DOM_CHANGED') {
    ;(async () => {
      const binding = await getBinding()
      if (binding && sender.tab?.id === binding.tabId && binding.jobId === message.jobId && Number(binding.runId) === Number(message.runId)) schedule(800)
      sendResponse({ ok: true })
    })()
    return true
  }
  if (message?.type === 'GET_STATUS') {
    ;(async () => {
      const status = await stored(STATUS_KEY) ?? { connected: false, hostName: null, message: '请打开书引 AI。', job: null }
      sendResponse(status)
    })()
    return true
  }
  if (message?.type === 'RUN_NOW') {
    tick().then(async () => sendResponse(await stored(STATUS_KEY))).catch(error => sendResponse({ connected: false, message: error?.message ?? '检查失败。' }))
    return true
  }
  return false
})

chrome.downloads.onChanged.addListener(delta => {
  if (!('state' in delta)) return
  ;(async () => {
    const binding = await getBinding()
    if (!binding?.downloadId || binding.downloadId !== delta.id) return
    const job = { id: binding.jobId, runId: binding.runId }
    try { await reportExactDownload(job, binding) } catch { schedule(5000) }
  })()
})

const testHooks = globalThis.__SHUYIN_AUDIO_TEST_HOOKS__
if (testHooks) {
  testHooks.tick = tick
} else {
  setupAlarms()
  tick().catch(() => {})
}








