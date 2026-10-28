/* global chrome */
(() => {
  'use strict'

  let activeRun = null
  let observer = null
  let observerTimer = null
  let lastNotifyAt = 0

  function currentCore() {
    const core = globalThis.ShuyinAudioContentCore
    if (!core) throw new Error('音频页面适配器未加载。')
    return core
  }

  function stopObserver() {
    observer?.disconnect()
    observer = null
    globalThis.clearTimeout(observerTimer)
    observerTimer = null
  }

  function watchForChanges(job) {
    stopObserver()
    if (!['generating', 'downloading', 'source_processing'].includes(job.stage)) return
    observer = new globalThis.MutationObserver(() => {
      if (Date.now() - lastNotifyAt < 5000) return
      globalThis.clearTimeout(observerTimer)
      observerTimer = globalThis.setTimeout(() => {
        lastNotifyAt = Date.now()
        chrome.runtime.sendMessage({ type: 'TASK_DOM_CHANGED', jobId: job.id, runId: job.runId }).catch(() => {})
      }, 600)
    })
    observer.observe(globalThis.document.documentElement, { childList: true, subtree: true, characterData: true, attributes: true })
  }

  async function sourceChunk(job, offset) {
    const response = await chrome.runtime.sendMessage({
      type: 'SOURCE_CHUNK_REQUEST',
      jobId: job.id,
      runId: job.runId,
      offset,
    })
    return response
  }

  chrome.runtime.onMessage.addListener((message, _sender, sendResponse) => {
    if (message?.type !== 'RUN_STEP') return false
    const job = message.job
    if (!job?.id) {
      sendResponse({ ok: false, error: '任务数据无效。', code: 'invalid_job' })
      return false
    }
    if (activeRun && (activeRun.jobId !== job.id || Number(activeRun.runId) !== Number(job.runId))) {
      sendResponse({ ok: false, stale: true, error: '旧运行任务已失效。', code: 'stale_run' })
      return false
    }
    activeRun = { jobId: job.id, runId: Number(job.runId) }
    watchForChanges(job)
    ;(async () => {
      try {
        const result = await currentCore().runStep(job, {
          sourceChunk: payload => sourceChunk(job, payload.offset),
        })
        if (result?.kind === 'navigating' || result?.kind === 'waiting' || result?.kind === 'source_processing') {
          sendResponse({ ok: true, step: result })
          return
        }
        sendResponse({ ok: true, step: result })
      } catch (error) {
        sendResponse({
          ok: false,
          code: error?.code ?? 'page_adapter_failed',
          error: error?.message ?? '页面自动操作失败。',
        })
      }
    })()
    return true
  })

  globalThis.addEventListener('beforeunload', stopObserver)
})()

