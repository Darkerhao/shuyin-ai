/* global URL */
(() => {
  'use strict'

  const AUDIO_EXTENSIONS = new Set(['aac', 'flac', 'm4a', 'mp3', 'ogg', 'wav'])
  const NOTEBOOK_HOSTS = new Set(['notebook.google.com', 'notebooklm.google.com'])
  const DOWNLOAD_RECOVERY_STAGES = new Set(['verify_remote', 'downloading', 'importing'])

  function safeSegment(value) {
    return String(value ?? '').replace(/[^a-zA-Z0-9_-]/g, '').slice(0, 80)
  }

  function inferSuffix(url, suggestedFilename = '') {
    for (const candidate of [suggestedFilename, url]) {
      let pathname
      try {
        pathname = new URL(candidate, 'https://notebook.google.com/').pathname
      } catch {
        continue
      }
      const match = pathname.toLowerCase().match(/\.([a-z0-9]{2,5})$/)
      if (match && AUDIO_EXTENSIONS.has(match[1])) return match[1]
    }
    return 'download'
  }

  function downloadFilename(jobId, url, suggestedFilename) {
    const id = safeSegment(jobId)
    if (!id) throw new Error('音频任务编号无效。')
    return `ShuyinAI/${id}/audio.${inferSuffix(url, suggestedFilename)}`
  }

  function bindingMatchesJob(binding, job) {
    return Boolean(binding && job && binding.jobId === job.id && Number(binding.runId) === Number(job.runId))
  }

  function recoverableDownloadBinding(binding, job) {
    if (!binding || !job || binding.jobId !== job.id) return null
    if (!Number.isInteger(binding.downloadId)) return null
    if (!DOWNLOAD_RECOVERY_STAGES.has(job.stage)) return null
    return { ...binding, runId: Number(job.runId), completed: false }
  }

  function exactDownloadForJob(binding, downloadId, job) {
    if (!bindingMatchesJob(binding, job)) return null
    if (!Number.isInteger(binding.downloadId) || binding.downloadId !== downloadId) return null
    return binding
  }

  function isNotebookUrl(value) {
    try {
      const url = new URL(value)
      return url.protocol === 'https:' && NOTEBOOK_HOSTS.has(url.hostname)
    } catch {
      return false
    }
  }

  function sameNotebook(left, right) {
    if (!isNotebookUrl(left) || !isNotebookUrl(right)) return false
    const a = new URL(left)
    const b = new URL(right)
    const aliases = new Set([a.hostname, b.hostname])
    if (![...aliases].every(host => NOTEBOOK_HOSTS.has(host))) return false
    return a.pathname.replace(/\/$/, '') === b.pathname.replace(/\/$/, '')
  }

  globalThis.ShuyinAudioBackgroundCore = Object.freeze({
    bindingMatchesJob,
    downloadFilename,
    exactDownloadForJob,
    inferSuffix,
    isNotebookUrl,
    recoverableDownloadBinding,
    sameNotebook,
  })
})()


