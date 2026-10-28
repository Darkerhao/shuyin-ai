/* global chrome, document */
(() => {
  'use strict'

  const connection = document.querySelector('#connection')
  const message = document.querySelector('#message')
  const host = document.querySelector('#host')
  const job = document.querySelector('#job')
  const refresh = document.querySelector('#refresh')

  function render(status) {
    const connected = Boolean(status?.connected)
    connection.textContent = connected ? '已连接' : '未连接'
    message.textContent = status?.message || (connected ? '等待书引 AI 下发音频任务。' : '请打开书引 AI，然后点击立即检查。')
    host.textContent = status?.hostName || '未连接'
    job.textContent = status?.job ? `${status.job.projectTitle || '当前书籍'} · ${status.job.stage} · ${status.job.status}` : '无'
  }

  async function run(check) {
    refresh.disabled = true
    try {
      const status = await chrome.runtime.sendMessage({ type: check ? 'RUN_NOW' : 'GET_STATUS' })
      render(status)
    } catch (error) {
      render({ connected: false, message: error?.message || '无法连接书引 AI。' })
    } finally {
      refresh.disabled = false
    }
  }

  refresh.addEventListener('click', () => run(true))
  run(false)
})()

