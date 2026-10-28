/* global URL, Buffer */
import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import test from 'node:test'
import vm from 'node:vm'
import { JSDOM } from 'jsdom'

const root = join(import.meta.dirname, '..')
const extension = join(root, 'browser-extension')
const contentSource = readFileSync(join(extension, 'content-core.js'), 'utf8')
const backgroundSource = readFileSync(join(extension, 'background-core.js'), 'utf8')
const backgroundWorkerSource = readFileSync(join(extension, 'background.js'), 'utf8')
const manifest = JSON.parse(readFileSync(join(extension, 'manifest.json'), 'utf8'))
const identity = JSON.parse(readFileSync(join(extension, 'identity.json'), 'utf8'))

function page(html, url = 'https://notebook.google.com/notebook/notebook-1') {
  const dom = new JSDOM(html, { url, runScripts: 'outside-only', pretendToBeVisual: true })
  dom.window.eval(contentSource)
  return { dom, window: dom.window, document: dom.window.document, core: dom.window.ShuyinAudioContentCore }
}

function background() {
  const context = vm.createContext({ URL })
  context.globalThis = context
  vm.runInContext(backgroundSource, context)
  return context.ShuyinAudioBackgroundCore
}

function workerHarness(job, tabUrl = job.notebookUrl) {
  const calls = { sendMessage: [], native: [] }
  const storage = {
    taskBinding: { jobId: job.id, runId: job.runId, tabId: 7, notebookUrl: job.notebookUrl, completed: false },
    preferredNativeHost: 'com.shuyin.desktop.audio',
  }
  const tab = { id: 7, url: tabUrl, status: 'complete' }
  const chromeMock = {
    storage: {
      local: {
        async get(key) { return { [key]: storage[key] } },
        async set(values) { Object.assign(storage, values) },
      },
    },
    runtime: {
      lastError: null,
      connectNative() {
        let listener = null
        return {
          onMessage: { addListener(callback) { listener = callback } },
          onDisconnect: { addListener() {} },
          postMessage(message) {
            calls.native.push(message)
            globalThis.setTimeout(() => {
              if (message.action === 'poll') listener({ ok: true, job })
              else listener({ ok: true })
            }, 0)
          },
          disconnect() {},
        }
      },
      onInstalled: { addListener() {} },
      onStartup: { addListener() {} },
      onMessage: { addListener() {} },
    },
    tabs: {
      async query() { return [tab] },
      async get(id) { if (id !== tab.id) throw new Error('missing tab'); return tab },
      async create() { return tab },
      async sendMessage(tabId, message) {
        calls.sendMessage.push({ tabId, message })
        return { ok: true, step: { kind: 'generating' } }
      },
    },
    alarms: { create() {}, onAlarm: { addListener() {} } },
    downloads: { async search() { return [] }, async download() { return 1 }, onChanged: { addListener() {} } },
  }
  const hooks = {}
  const context = vm.createContext({
    URL: globalThis.URL,
    __SHUYIN_AUDIO_TEST_HOOKS__: hooks,
    chrome: chromeMock,
    clearTimeout: globalThis.clearTimeout,
    crypto: globalThis.crypto,
    setTimeout: globalThis.setTimeout,
  })
  context.globalThis = context
  context.importScripts = () => vm.runInContext(backgroundSource, context)
  vm.runInContext(backgroundWorkerSource, context)
  return { calls, hooks }
}

function textJob(overrides = {}) {
  return {
    id: 'job-1', runId: 1, stage: 'adding_source', status: 'preparing', format: 'deep_dive',
    targetDurationSeconds: 1500, prompt: '完整音频指导\n第二段', source: { kind: 'text', text: '用户提供的资料文本。' }, ...overrides,
  }
}

test('manifest uses the fixed identity and both Gemini Notebook hosts', () => {
  const publicKey = Buffer.from(identity.publicKey, 'base64')
  const digest = createHash('sha256').update(publicKey).digest('hex').slice(0, 32)
  const extensionId = [...digest].map(character => String.fromCharCode(97 + Number.parseInt(character, 16))).join('')
  assert.equal(identity.extensionId, 'nplahghodghepogenmdekjafojiljohf')
  assert.equal(extensionId, identity.extensionId)
  assert.equal(manifest.key, identity.publicKey)
  assert.deepEqual(manifest.permissions.sort(), ['alarms', 'downloads', 'nativeMessaging', 'storage', 'tabs'])
  assert.deepEqual(manifest.host_permissions.sort(), ['https://notebook.google.com/*', 'https://notebooklm.google.com/*'])
  assert.deepEqual(manifest.content_scripts[0].matches.sort(), manifest.host_permissions.sort())
})

test('creating a notebook writes and reads back the required title before reporting success', async () => {
  const job = textJob({ stage: 'creating_notebook', projectTitle: '测试书', notebookUrl: 'https://notebook.google.com/notebook/task-notebook' })
  const { core, document } = page(`
    <header><div id="title" role="textbox" contenteditable="true" aria-label="Notebook title">Untitled notebook</div></header>
    <main><button>Generate</button></main>
  `, job.notebookUrl)
  const result = await core.runStep(job, {})
  assert.equal(result.kind, 'notebook_created')
  assert.equal(document.querySelector('#title').textContent, '测试书 · 音频 job-1')
})

test('notebook naming requires the creation intent persisted across navigation', async () => {
  const job = textJob({ stage: 'creating_notebook', projectTitle: '测试书' })
  const { core, window, document } = page(`
    <button id="create">Create new notebook</button>
    <header><div id="title" role="textbox" contenteditable="true" aria-label="Notebook title">Untitled notebook</div></header>
  `, 'https://notebook.google.com/')
  document.querySelector('#create').addEventListener('click', () => {})
  let result = await core.runStep(job, {})
  assert.equal(result.kind, 'navigating')
  window.history.pushState({}, '', '/notebook/new-notebook')
  result = await core.runStep(job, {})
  assert.equal(result.kind, 'notebook_created')
  assert.equal(document.querySelector('#title').textContent, '测试书 · 音频 job-1')
  assert.equal(window.sessionStorage.getItem('shuyin-create-intent'), null)
})

test('an untracked existing notebook is never renamed or treated as the new task notebook', async () => {
  const job = textJob({ stage: 'creating_notebook', projectTitle: '测试书' })
  const { core, document } = page(`
    <header><div id="title" role="textbox" contenteditable="true" aria-label="Notebook title">Somebody else's notebook</div></header>
  `)
  const result = await core.runStep(job, {})
  assert.equal(result.kind, 'page_changed')
  assert.match(result.message, /新建意图记录/)
  assert.equal(document.querySelector('#title').textContent, "Somebody else's notebook")
})

test('creating a notebook stops when no unique explicit title control exists', async () => {
  let generateClicks = 0
  const job = textJob({ stage: 'creating_notebook', projectTitle: '测试书', notebookUrl: 'https://notebook.google.com/notebook/notebook-1' })
  const { core, document } = page(`
    <header><input type="search" aria-label="Search notebooks"></header>
    <main><textarea aria-label="Audio instructions"></textarea><button id="generate">Generate</button></main>
  `)
  document.querySelector('#generate').addEventListener('click', () => { generateClicks += 1 })
  const result = await core.runStep(job, {})
  assert.equal(result.kind, 'page_changed')
  assert.match(result.message, /唯一的标题控件/)
  assert.equal(generateClicks, 0)
})

test('background delegates notebook naming and keeps verify_remote read-only polling active', () => {
  assert.doesNotMatch(backgroundWorkerSource, /creating_notebook[\s\S]{0,120}startsWith\('\/notebook\/'\)/)
  assert.match(backgroundWorkerSource, /WAITING\.has\(job\.status\) && job\.stage !== 'verify_remote'/)
  assert.match(backgroundWorkerSource, /job\.stage === 'awaiting_extension' \|\| job\.stage === 'login'/)
  assert.ok(backgroundWorkerSource.includes("job.stage === 'verify_remote' && /^https:"))
  assert.match(backgroundWorkerSource, /recoverableDownloadBinding\(binding, job\)/)
  assert.match(backgroundWorkerSource, /download_history_missing/)
})

test('verify_remote sends RUN_STEP instead of short-circuiting as login_ready', async () => {
  const job = {
    id: 'job-1', runId: 2, status: 'preparing', stage: 'verify_remote',
    notebookUrl: 'https://notebook.google.com/notebook/task-notebook', projectTitle: '测试书',
    prompt: '完整音频指导', targetDurationSeconds: 1500, format: 'deep_dive',
    source: { kind: 'text', text: '用户资料' },
  }
  const harness = workerHarness(job)
  await harness.hooks.tick()
  assert.equal(harness.calls.sendMessage.length, 1)
  assert.equal(harness.calls.sendMessage[0].tabId, 7)
  assert.equal(harness.calls.sendMessage[0].message.type, 'RUN_STEP')
  assert.equal(harness.calls.sendMessage[0].message.job.stage, 'verify_remote')
  assert.equal(harness.calls.native.some(message => message.observation?.kind === 'login_ready'), false)

  const loginHarness = workerHarness(job, 'https://accounts.google.com/signin')
  await loginHarness.hooks.tick()
  assert.equal(loginHarness.calls.sendMessage.length, 0)
  assert.equal(loginHarness.calls.native.some(message => message.observation?.kind === 'login_required'), true)
})

test('finds English and Chinese controls by semantic labels and refuses ambiguous controls', () => {
  const { core } = page(`<button id="upload">Upload files</button><button id="text">复制的文字</button><button hidden>Upload files</button><button id="download-a">Download</button><button id="download-b">下载</button>`)
  assert.equal(core.findInteractive(['Upload files', '上传文件']).id, 'upload')
  assert.equal(core.findInteractive(['Copied text', '复制的文字']).id, 'text')
  assert.throws(() => core.findInteractive(['Download', '下载']), error => error.code === 'ambiguous_control')
  assert.equal(core.findInteractive(['Not present']), null)
})

test('long prompts round-trip exactly and over-limit prompts are not truncated', () => {
  const prompt = `${'第一段内容。\n'.repeat(400)}结尾`
  const { core, document } = page(`<textarea id="prompt" aria-label="Audio instructions"></textarea><textarea id="short" aria-label="Short" maxlength="10"></textarea>`)
  const input = document.querySelector('#prompt')
  core.writeAndVerify(input, prompt)
  assert.equal(core.readEditable(input), prompt)
  assert.throws(() => core.writeAndVerify(document.querySelector('#short'), '超过限制超过限制超过限制'), error => error.code === 'prompt_too_long')
  assert.equal(document.querySelector('#short').value, '')
})

test('adds a text source through the copied-text dialog without changing the prompt', async () => {
  const { core, document } = page(`<button id="add">Add source</button><section id="dialog" role="dialog" hidden><button id="copied">Copied text</button><textarea id="source" aria-label="Copied text" hidden></textarea><button id="insert" hidden>Insert</button></section>`)
  document.querySelector('#add').addEventListener('click', () => { document.querySelector('#dialog').hidden = false })
  document.querySelector('#copied').addEventListener('click', () => { document.querySelector('#source').hidden = false; document.querySelector('#insert').hidden = false })
  document.querySelector('#insert').addEventListener('click', () => { document.querySelector('#dialog').hidden = true })
  const result = await core.runStep(textJob(), {})
  assert.equal(result.kind, 'source_submitted')
  assert.equal(document.querySelector('#source').value, '用户提供的资料文本。')
})

test('configures audio, language, duration and verifies the full prompt before generation', async () => {
  const longPrompt = `${'主持人甲子需要完整讲述这一段。\n'.repeat(250)}结束`
  const { core, document } = page(`<button id="audio">Audio Overview</button><section id="panel" hidden><button id="customize">Customize</button><button id="deep" aria-checked="true">Deep Dive</button><button id="brief" aria-checked="false">Brief</button><button id="language" aria-label="Language">English</button><div id="languages" hidden><button id="chinese" role="option">Chinese (Simplified)</button></div><button id="default" aria-checked="true">Default</button><textarea id="prompt" aria-label="Audio instructions"></textarea><button id="generate">Generate</button></section>`)
  document.querySelector('#audio').addEventListener('click', () => { document.querySelector('#panel').hidden = false })
  document.querySelector('#language').addEventListener('click', () => { document.querySelector('#languages').hidden = false })
  document.querySelector('#chinese').addEventListener('click', () => { document.querySelector('#languages').hidden = true })
  const result = await core.runStep(textJob({ stage: 'configuring_audio', prompt: longPrompt }), {})
  assert.equal(result.kind, 'audio_configured')
  assert.equal(result.format, 'deep_dive')
  assert.equal(result.language, 'zh-Hans')
  assert.equal(document.querySelector('#prompt').value, longPrompt)
})

test('unknown controls are never clicked by guessed coordinates or labels', () => {
  let clicks = 0
  const { core, document } = page(`<button id="unrelated">Something else</button>`)
  document.querySelector('#unrelated').addEventListener('click', () => { clicks += 1 })
  assert.equal(core.findInteractive(['Upload files', '上传文件', 'Add source', '添加来源']), null)
  assert.equal(clicks, 0)
})

test('reads a source in bounded chunks without dropping bytes', async () => {
  const totalSize = 600_123
  const original = Buffer.alloc(totalSize)
  for (let index = 0; index < totalSize; index += 1) original[index] = index % 251
  const { core, window } = page('')
  const offsets = []
  const source = await core.readSourceFile(textJob({ source: { kind: 'asset', name: 'book.pdf', size: totalSize, sha256: 'a'.repeat(64) } }), {
    sourceChunk: async ({ offset }) => {
      offsets.push(offset)
      const end = Math.min(offset + 256 * 1024, totalSize)
      return { ok: true, data: original.subarray(offset, end).toString('base64'), nextOffset: end, totalSize, name: 'book.pdf', sha256: 'a'.repeat(64) }
    },
  })
  assert.deepEqual(offsets, [0, 262144, 524288])
  assert.equal(source.file.name, 'book.pdf')
  assert.equal(source.file.size, totalSize)
  const bytes = await new Promise((resolve, reject) => {
    const reader = new window.FileReader()
    reader.onload = () => resolve(Buffer.from(reader.result))
    reader.onerror = () => reject(reader.error)
    reader.readAsArrayBuffer(source.file)
  })
  assert.equal(bytes.length, totalSize)
  assert.equal(bytes.equals(original), true)
})

test('refuses a different notebook when the task already recorded its notebook URL', async () => {
  const { core } = page('', 'https://notebook.google.com/notebook/other-notebook')
  await assert.rejects(
    () => core.runStep(textJob({ stage: 'downloading', notebookUrl: 'https://notebook.google.com/notebook/task-notebook' }), {}),
    error => error.code === 'wrong_notebook',
  )
})

test('verify_remote only observes the recorded notebook and never resubmits generation', async () => {
  let clicks = 0
  const notebookUrl = 'https://notebook.google.com/notebook/task-notebook'
  let fixture = page(`
    <section><h2>Audio Overview</h2><p>Generating audio</p><button id="generate">Generate</button></section>
  `, notebookUrl)
  fixture.document.querySelector('#generate').addEventListener('click', () => { clicks += 1 })
  let result = await fixture.core.runStep(textJob({ stage: 'verify_remote', status: 'waiting_user', notebookUrl }), {})
  assert.equal(result.kind, 'generating')
  assert.equal(clicks, 0)

  fixture = page('<section><h2>Audio Overview</h2><audio controls></audio><button id="generate">Generate</button></section>', notebookUrl)
  fixture.document.querySelector('#generate').addEventListener('click', () => { clicks += 1 })
  result = await fixture.core.runStep(textJob({ stage: 'verify_remote', status: 'waiting_user', notebookUrl }), {})
  assert.equal(result.kind, 'generation_complete')
  assert.equal(clicks, 0)

  fixture = page('<div role="listitem">book.pdf</div><button id="generate">Generate</button>', notebookUrl)
  fixture.document.querySelector('#generate').addEventListener('click', () => { clicks += 1 })
  result = await fixture.core.runStep(textJob({ stage: 'verify_remote', status: 'waiting_user', notebookUrl, source: { kind: 'asset', name: 'book.pdf', size: 10, sha256: 'a'.repeat(64) } }), {})
  assert.equal(result.kind, 'source_processing')
  assert.equal(clicks, 0)
})

test('detects generation only inside the Audio Overview artifact region', () => {
  let fixture = page('<p>Generating audio</p>')
  assert.equal(fixture.core.detectGenerationState(), 'generating')
  fixture = page('<section><h2>Audio Overview</h2><audio controls></audio></section>')
  assert.equal(fixture.core.detectGenerationState(), 'complete')
  fixture = page('<section><h2>Audio Overview</h2><p>Something went wrong</p></section>')
  assert.equal(fixture.core.detectGenerationState(), 'failed')
  fixture = page('<section><h2>Video Overview</h2><audio controls></audio><button>Download</button></section>')
  assert.equal(fixture.core.detectGenerationState(), 'unknown')
  fixture = page('<p>Ready to start</p>')
  assert.equal(fixture.core.detectGenerationState(), 'unknown')
})

test('captures the actual clicked download anchor target from the Audio Overview region', async () => {
  const { core, document } = page(`<section><h2>Audio Overview</h2><button id="download">Download</button></section>`)
  document.querySelector('#download').addEventListener('click', () => {
    const anchor = document.createElement('a')
    anchor.href = 'https://content.example.com/audio/episode.m4a?token=one'
    anchor.download = 'episode.m4a'
    anchor.addEventListener('click', event => event.preventDefault())
    document.body.append(anchor)
    anchor.click()
  })
  const result = await core.captureDownloadTarget()
  assert.equal(result.kind, 'download_target')
  assert.equal(result.downloadUrl, 'https://content.example.com/audio/episode.m4a?token=one')
  assert.equal(result.suggestedFilename, 'episode.m4a')
})

test('download uses only the Audio Overview Download control, not another artifact', async () => {
  let wrongClicks = 0
  let rightClicks = 0
  const { core, document } = page(`
    <section><h2>Video Overview</h2><button id="wrong">Download</button></section>
    <section><h2>Audio Overview</h2><button id="right">Download</button></section>
  `)
  document.querySelector('#wrong').addEventListener('click', () => { wrongClicks += 1 })
  document.querySelector('#right').addEventListener('click', () => {
    rightClicks += 1
    const anchor = document.createElement('a')
    anchor.href = 'https://content.example.com/audio/right.mp3'
    anchor.download = 'right.mp3'
    anchor.addEventListener('click', event => event.preventDefault())
    document.body.append(anchor)
    anchor.click()
  })
  const result = await core.captureDownloadTarget()
  assert.equal(result.downloadUrl, 'https://content.example.com/audio/right.mp3')
  assert.equal(wrongClicks, 0)
  assert.equal(rightClicks, 1)
})

test('download binding accepts only the exact download id and current run', () => {
  const core = background()
  const binding = { jobId: 'job-1', runId: 2, downloadId: 42 }
  assert.equal(core.exactDownloadForJob(binding, 42, { id: 'job-1', runId: 2 })?.downloadId, 42)
  assert.equal(core.exactDownloadForJob(binding, 41, { id: 'job-1', runId: 2 }), null)
  assert.equal(core.exactDownloadForJob(binding, 42, { id: 'job-1', runId: 3 }), null)
  assert.equal(core.exactDownloadForJob(binding, 42, { id: 'job-2', runId: 2 }), null)
  assert.equal(core.bindingMatchesJob(binding, { id: 'job-1', runId: 2 }), true)
  assert.equal(core.bindingMatchesJob(binding, { id: 'job-1', runId: 3 }), false)
})

test('carries an exact download across run changes only for the same recoverable job', () => {
  const core = background()
  const binding = { jobId: 'job-1', runId: 1, downloadId: 42, downloadStartedAt: 100, tabId: 9 }
  for (const stage of ['verify_remote', 'downloading', 'importing']) {
    const next = core.recoverableDownloadBinding(binding, { id: 'job-1', runId: 2, stage })
    assert.equal(next.runId, 2)
    assert.equal(next.downloadId, 42)
    assert.equal(next.downloadStartedAt, 100)
    assert.equal(core.exactDownloadForJob(next, 42, { id: 'job-1', runId: 2 }), next)
  }
  assert.equal(core.recoverableDownloadBinding(binding, { id: 'job-1', runId: 2, stage: 'preparing' }), null)
  assert.equal(core.recoverableDownloadBinding(binding, { id: 'job-2', runId: 2, stage: 'importing' }), null)
  assert.equal(core.recoverableDownloadBinding({ ...binding, downloadId: null }, { id: 'job-1', runId: 2, stage: 'importing' }), null)
})

test('download filename is task-scoped and preserves a real audio suffix', () => {
  const core = background()
  assert.equal(core.downloadFilename('abc123', 'https://example.com/a.M4A?x=1', ''), 'ShuyinAI/abc123/audio.m4a')
  assert.equal(core.downloadFilename('abc123', 'blob:https://notebook.google.com/id', 'episode.mp3'), 'ShuyinAI/abc123/audio.mp3')
  assert.equal(core.downloadFilename('abc123', 'https://example.com/download', ''), 'ShuyinAI/abc123/audio.download')
  assert.equal(core.sameNotebook('https://notebooklm.google.com/notebook/id-1?x=1', 'https://notebook.google.com/notebook/id-1?x=2'), true)
  assert.equal(core.sameNotebook('https://notebook.google.com/notebook/id-1', 'https://notebook.google.com/notebook/id-2'), false)
  assert.equal(core.isNotebookUrl('https://example.com/notebook/id-1'), false)
})














