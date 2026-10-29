import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import test from 'node:test'
import { JSDOM } from 'jsdom'
const contentSource = readFileSync(join(import.meta.dirname, '../src-tauri/src/audio_page.js'), 'utf8')
function page(html, url = 'https://notebook.google.com/notebook/notebook-1') {
  const dom = new JSDOM(html, { url, runScripts: 'outside-only', pretendToBeVisual: true })
  dom.window.eval(contentSource)
  return { dom, window: dom.window, document: dom.window.document, core: dom.window.ShuyinAudioPage }
}

function textJob(overrides = {}) {
  return {
    id: 'job-1', runId: 1, stage: 'adding_source', status: 'preparing', format: 'deep_dive',
    targetDurationSeconds: 1500, prompt: '完整音频指导\n第二段', source: { kind: 'text', text: '用户提供的资料文本。' }, ...overrides,
  }
}

test('a website 404 stops opening immediately with an actionable recovery message', async () => {
  const { core } = page('<h1>Error: Not Found</h1><h2>The requested URL / was not found on this server.</h2>', 'https://notebook.google/')
  const result = await core.runStep(textJob({ stage: 'opening_site' }))
  assert.equal(result.kind, 'page_changed')
  assert.match(result.message, /刷新网站/)
})

test('website failures are detected before any generation or source action', async () => {
  for (const html of ['<h1>503 Service Unavailable</h1>', '<h1>This site can’t be reached</h1><p>ERR_CONNECTION_TIMED_OUT</p>', '<h1>403 Forbidden</h1>', '<h1>429 Too Many Requests</h1>']) {
    const { core, document } = page(`${html}<button id="generate">Generate</button>`)
    let clicks = 0
    document.querySelector('#generate').addEventListener('click', () => { clicks += 1 })
    const result = await core.runStep(textJob({ stage: 'submitting' }))
    assert.equal(result.kind, 'page_changed')
    assert.match(result.message, /刷新网站/)
    assert.equal(clicks, 0)
  }
})

test('unsupported regions provide network guidance even when the server returns a 404', async () => {
  const { core } = page('<h1>Error: Not Found</h1>', 'https://notebook.google/?location=unsupported')
  const result = await core.runStep(textJob({ stage: 'opening_site' }))
  assert.equal(result.kind, 'page_changed')
  assert.match(result.message, /地区不受支持/)
  assert.match(result.message, /重新打开/)
})

test('a human challenge stops safely while normal login still waits for the user', async () => {
  const { core } = page('<h1>Verify you are human</h1><iframe src="https://google.com/recaptcha/api2/anchor" title="reCAPTCHA"></iframe>')
  const result = await core.runStep(textJob({ stage: 'opening_site' }))
  assert.equal(result.kind, 'page_changed')
  assert.match(result.message, /人工验证/)
  const login = page('', 'https://accounts.google.com/signin')
  assert.equal((await login.core.runStep(textJob())).kind, 'login_required')
})

test('recorded notebook identity survives official domain redirects', async () => {
  for (const host of ['notebook.google', 'notebooklm.google', 'notebooklm.google.com']) {
    const { core } = page('', `https://${host}/notebook/notebook-1`)
    const result = await core.runStep(textJob({ stage: 'opening_site', notebookUrl: 'https://notebook.google.com/notebook/notebook-1' }), {})
    assert.equal(result.kind, 'login_ready')
  }
})

test('notebook identity rejects lookalike domains and different notebook IDs', async () => {
  for (const url of ['https://notebook.google.evil.test/notebook/notebook-1', 'https://notebook.google/notebook/other']) {
    const { core } = page('', url)
    await assert.rejects(core.runStep(textJob({ stage: 'submitting', notebookUrl: 'https://notebook.google.com/notebook/notebook-1' }), {}), error => error.code === 'wrong_notebook')
  }
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
  assert.equal(result.kind, 'notebook_identified')
  result = await core.runStep({ ...job, notebookUrl: result.notebookUrl }, {})
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

test('created notebook identity is saved before naming and survives a title failure', async () => {
  const { core, window, document } = page('<main>未命名的笔记本</main>')
  const job = textJob({ stage: 'creating_notebook', projectTitle: '测试书' })
  window.sessionStorage.setItem('shuyin-create-intent', job.id)
  const identified = await core.runStep(job)
  assert.deepEqual(JSON.parse(JSON.stringify(identified)), { kind: 'notebook_identified', notebookUrl: window.location.href })
  job.notebookUrl = identified.notebookUrl
  assert.equal((await core.runStep(job)).kind, 'page_changed')
  assert.equal(window.sessionStorage.getItem('shuyin-create-intent'), job.id)
  document.body.innerHTML = '<header><input aria-label="Notebook title"></header>'
  assert.equal((await core.runStep(job)).kind, 'notebook_created')
  assert.equal(window.sessionStorage.getItem('shuyin-create-intent'), null)
})

test('empty notebook recovery requires an explicit zero source count and no audio result', () => {
  for (const label of ['0 个来源', '0 sources']) {
    const { core } = page(`<button>Add source</button><span>${label}</span>`)
    assert.equal(core.canRecoverEmptyNotebook(), true)
  }
  for (const html of [
    '<button>Add source</button>',
    '<button>Add source</button><span>1 source</span>',
    '<button>Add source</button><span>10 sources</span>',
    '<button>Add source</button><span>0 sources</span><section><h2>Audio Overview</h2><button>Play</button></section>',
    '<button>Add source</button><span>0 sources</span><section><h2>Audio Overview</h2><p>Generating</p></section>',
    '<button>Add source</button><span>0 sources</span><section><h2>Audio Overview</h2><button>Play</button></section><section><h2>Audio Overview</h2><button>Play</button></section>',
  ]) {
    assert.equal(page(html).core.canRecoverEmptyNotebook(), false)
  }
  const outside = page('<button>Add source</button><span>0 sources</span>', 'https://notebook.google.evil.test/notebook/one')
  assert.equal(outside.core.canRecoverEmptyNotebook(), false)
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

test('a notebook title or unrelated textarea cannot stand in for the audio prompt', () => {
  const { core } = page('<header><input aria-label="Notebook title"></header><textarea aria-label="Chat"></textarea>')
  assert.equal(core.findEditable(['Audio instructions', 'Prompt', 'What should']), null)
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
  let fixture = page('<section><h2>Audio Overview</h2><p>Generating audio</p></section>')
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

test('native file selection gets one marked input and no source bytes in the page contract', async () => {
  const job = textJob({ source: { kind: 'asset', name: '原书.epub', size: 123, sha256: 'frozen' } })
  const { core, document } = page('<section role="dialog"><input id="source" type="file"></section>')
  const result = await core.runStep(job)
  assert.equal(result.kind, 'file_input_ready')
  assert.equal(document.querySelector('#source').getAttribute('data-shuyin-audio-source'), 'job-1:1')
  assert.equal(document.querySelector('#source').files.length, 0)
})

test('pausing during an asynchronous page step prevents the pending source write', async () => {
  const { core, window, document } = page('<section role="dialog"><button id="copied">Copied text</button></section>')
  document.querySelector('#copied').addEventListener('click', () => {
    window.setTimeout(() => {
      window.__shuyinAudioRun = null
      const input = document.createElement('textarea')
      input.setAttribute('aria-label', 'Copied text')
      document.querySelector('section').append(input)
    }, 10)
  })
  await assert.rejects(core.runStep(textJob()), error => error.code === 'stale_run')
  assert.equal(document.querySelector('textarea').value, '')
})

test('a recovered creation marker still identifies the same notebook after a run changes', async () => {
  const { core, window } = page('<header><input aria-label="Notebook title"></header>')
  window.sessionStorage.setItem('shuyin-create-intent', 'job-1')
  const result = await core.runStep(textJob({ stage: 'creating_notebook', runId: 3 }))
  assert.equal(result.kind, 'notebook_identified')
})

test('download discovery blocks the anchor default until the host arms its exact target', async () => {
  const { core, window, document } = page('<section><h2>Audio Overview</h2><button id="download">Download</button></section>')
  let prevented = false
  document.querySelector('#download').addEventListener('click', () => {
    const link = document.createElement('a')
    link.href = 'https://content.example.com/audio/one.m4a'
    document.body.append(link)
    const event = new window.MouseEvent('click', { bubbles: true, cancelable: true })
    prevented = !link.dispatchEvent(event)
  })
  const result = await core.captureDownloadTarget()
  assert.equal(result.downloadUrl, 'https://content.example.com/audio/one.m4a')
  assert.equal(prevented, true)
})

test('multiple audio artifacts and unrelated generating text cannot pick a task result', async () => {
  const { core } = page('<section><h2>Audio Overview</h2><button>Download</button></section><section><h2>Audio Overview</h2><button>Download</button></section>')
  assert.equal(core.audioArtifactScope(), null)
  await assert.rejects(core.captureDownloadTarget(), error => error.code === 'audio_artifact_missing')
  const other = page('<section><h2>Video Overview</h2><p>Generating</p></section>')
  assert.equal(other.core.detectGenerationState(), 'unknown')
})

test('generation observations also require the exact recorded notebook', async () => {
  const { core } = page('<section><h2>Audio Overview</h2><audio></audio></section>', 'https://notebook.google.com/notebook/other')
  await assert.rejects(core.runStep(textJob({stage:'generating',notebookUrl:'https://notebook.google.com/notebook/task'})), error => error.code === 'wrong_notebook')
})
