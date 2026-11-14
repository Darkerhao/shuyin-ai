import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import test from 'node:test'
import { JSDOM } from 'jsdom'
const contentSource = readFileSync(join(import.meta.dirname, '../src-tauri/src/audio_page.js'), 'utf8')
test('ignores create controls inside CSS-hidden responsive panels', async () => {
  const { core } = page('<div style="display:none"><button>Create new notebook</button></div><button>Create new notebook</button>', 'https://notebook.google.com/')
  assert.equal((await core.runStep(textJob({ stage: 'opening_site' }))).kind, 'login_ready')
})

test('source name alone does not hide its row processing indicator', () => {
  const { core } = page('<div class="single-source-container"><span>原书.txt</span><span role="progressbar"></span></div>')
  assert.equal(core.sourceIsReady(textJob({ source: { kind: 'asset', name: '原书.txt' } })), false)
})
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

function chapterSourcePage({ processing = false, selected = true } = {}) {
  return page(`
    <section><h2>来源</h2><button aria-label="添加来源">来源</button>
      <div class="single-source-container">
        <button class="source-stretched-button" aria-label="第二章 张翼德怒鞭督邮"></button>
        <div aria-hidden="true"><span>第二章 张翼德怒鞭督邮</span></div>
        ${processing ? '<span role="progressbar"></span>' : `<input type="checkbox" aria-label="选择本章" ${selected ? 'checked' : ''}>`}
      </div>
    </section>
    <main><span>0 个来源</span><div>第二章 张翼德怒鞭督邮</div></main>
    <aside><h2>Studio &gt; 笔记</h2><button aria-label="关闭笔记视图">collapse_content</button>
      <form class="note-form"><input aria-label="可修改笔记名称" value="第二章 张翼德怒鞭督邮">
        <div contenteditable="true"><p>第二章 张翼德怒鞭督邮</p><p>且说董卓字仲颖，陇西临洮人也。</p></div>
      </form>
    </aside>
  `)
}

const chapterTextSource = { kind: 'text', name: '第二章：张翼德怒鞭督邮何国舅谋诛宦竖 · 完整章节', text: '第二章 张翼德怒鞭督邮\n\n且说董卓字仲颖，陇西临洮人也。' }

test('recognizes a pasted chapter by its actual source row despite a different local label and zero document count', () => {
  const { core, window } = chapterSourcePage()
  try {
    assert.equal(core.sourceIsReady(textJob({ source: chapterTextSource })), true)
  } finally { window.close() }
})

test('chapter note text cannot conceal source processing or substitute for the source row', () => {
  const { core, document, window } = chapterSourcePage({ processing: true })
  try {
    assert.equal(core.sourceIsReady(textJob({ source: chapterTextSource })), false)
    document.querySelector('.single-source-container').remove()
    assert.equal(core.sourceIsReady(textJob({ source: chapterTextSource })), null)
  } finally { window.close() }
})

test('a chapter title in note lists cannot substitute for the actual source row', () => {
  const { core, document, window } = chapterSourcePage()
  try {
    document.querySelector('.single-source-container').remove()
    document.querySelector('[contenteditable]').innerHTML = '<ul><li>第二章 张翼德怒鞭督邮</li></ul>'
    assert.equal(core.sourceIsReady(textJob({ source: chapterTextSource })), null)
  } finally { window.close() }
})

test('a real chapter source stays ready when note lists repeat its title', () => {
  const { core, document, window } = chapterSourcePage()
  try {
    document.querySelector('[contenteditable]').innerHTML = '<ul><li>第二章 张翼德怒鞭督邮</li></ul>'
    assert.equal(core.sourceIsReady(textJob({ source: chapterTextSource })), true)
  } finally { window.close() }
})

test('an unselected chapter stops with a specific error instead of waiting until timeout', async (t) => {
  const { core, window } = chapterSourcePage({ selected: false })
  t.after(() => window.close())
  const result = await core.runStep(textJob({ stage: 'source_processing', source: chapterTextSource }))
  assert.equal(result.kind, 'page_error')
  assert.equal(result.code, 'source_not_selected')
})

test('current audio dialog submits Generate now, never Generate later', async (t) => {
  const { core, document, window } = page('<section role="dialog"><button id="later">稍后生成</button><button id="now">立即生成</button></section>')
  t.after(() => window.close())
  const clicks = []
  document.querySelector('#later').addEventListener('click', () => clicks.push('later'))
  document.querySelector('#now').addEventListener('click', () => clicks.push('now'))
  const result = await core.runStep(textJob({ stage: 'submitting', notebookUrl: window.location.href }))
  assert.equal(result.kind, 'generation_submitted', JSON.stringify(result))
  assert.deepEqual(clicks, ['now'])
})

test('remote verification with a ready chapter never closes the note or generates again', async (t) => {
  const { core, document, window } = chapterSourcePage()
  t.after(() => window.close())
  let clicks = 0
  document.querySelector('[aria-label="关闭笔记视图"]').addEventListener('click', () => { clicks++ })
  const result = await core.runStep(textJob({ stage: 'verify_remote', notebookUrl: window.location.href, source: chapterTextSource }))
  assert.equal(result.kind, 'source_processing')
  assert.equal(clicks, 0)
})

test('an exact source title still waits for a sibling processing message', () => {
  const { core, window } = page('<div class="single-source-container"><span>原书.txt</span><span>Processing source</span></div>')
  try { assert.equal(core.sourceIsReady(textJob({ source: { kind: 'asset', name: '原书.txt' } })), false) }
  finally { window.close() }
})

test('a Generate later button alone must never submit an audio task', async (t) => {
  const { core, document, window } = page('<section role="dialog"><button>稍后生成</button></section>')
  t.after(() => window.close())
  let clicks = 0
  document.querySelector('button').addEventListener('click', () => { clicks++ })
  const result = await core.runStep(textJob({ stage: 'submitting', notebookUrl: window.location.href }))
  assert.equal(result.code, 'generate_missing')
  assert.equal(clicks, 0)
})

test('returns from the pasted chapter note to Studio before configuring and submitting audio once', async (t) => {
  const { core, document, window } = chapterSourcePage()
  t.after(() => window.close())
  let closeClicks = 0
  let generationClicks = 0
  const prompt = '请完整解释本章人物动机与事件经过。\n不要引用其他章节。'
  document.querySelector('[aria-label="关闭笔记视图"]').addEventListener('click', () => {
    closeClicks++
    document.querySelector('aside').innerHTML = '<h2>Studio</h2><button id="audio">音频概览</button>'
    document.querySelector('#audio').addEventListener('click', () => {
      document.querySelector('aside').innerHTML += '<section role="dialog"><button aria-checked="true">深入探究</button><button>中文（简体）</button><button aria-checked="true">默认</button><textarea aria-label="音频指导"></textarea><button id="generate">生成</button></section>'
      document.querySelector('#generate').addEventListener('click', () => { generationClicks++ })
    })
  })
  const job = textJob({ stage: 'configuring_audio', notebookUrl: window.location.href, source: chapterTextSource, prompt })
  const result = await core.runStep(job)
  assert.equal(result.kind, 'audio_configured', JSON.stringify(result))
  assert.equal(closeClicks, 1)
  assert.equal(document.querySelector('textarea').value, prompt)
  assert.equal(generationClicks, 0)
  assert.equal((await core.runStep({ ...job, stage: 'submitting' })).kind, 'generation_submitted')
  assert.equal(generationClicks, 1)
})

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
    const result = await core.runStep(textJob({ stage: 'submitting', notebookUrl: 'https://notebook.google.com/notebook/notebook-1' }), {})
    assert.equal(result.kind, 'page_error')
    assert.equal(result.code, 'wrong_notebook')
  }
})

test('a recorded notebook proceeds without changing its title', async () => {
  const job = textJob({ stage: 'creating_notebook', projectTitle: '测试书', notebookUrl: 'https://notebook.google.com/notebook/task-notebook' })
  const { core, document } = page(`
    <header><div id="title" role="textbox" contenteditable="true" aria-label="Notebook title">Untitled notebook</div></header>
    <main><button>Generate</button></main>
  `, job.notebookUrl)
  const result = await core.runStep(job, {})
  assert.equal(result.kind, 'notebook_created')
  assert.equal(document.querySelector('#title').textContent, 'Untitled notebook')
})

test('notebook creation requires the creation intent persisted across navigation', async () => {
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
  assert.equal(document.querySelector('#title').textContent, 'Untitled notebook')
  assert.equal(window.sessionStorage.getItem('shuyin-create-intent'), null)
})

test('notebook creation waits for navigation without clicking Create again', async () => {
  const { core, document } = page('<button>Create new notebook</button>', 'https://notebook.google.com/')
  let clicks = 0
  document.querySelector('button').addEventListener('click', () => clicks++)
  const job = textJob({ stage: 'creating_notebook' })
  assert.equal((await core.runStep(job)).kind, 'navigating')
  assert.equal((await core.runStep(job)).kind, 'waiting')
  assert.equal(clicks, 1)
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

test('created notebook identity is saved before proceeding even without a title input', async () => {
  const { core, window } = page('<main>未命名的笔记本</main>')
  const job = textJob({ stage: 'creating_notebook', projectTitle: '测试书' })
  window.sessionStorage.setItem('shuyin-create-intent', job.id)
  const identified = await core.runStep(job)
  assert.deepEqual(JSON.parse(JSON.stringify(identified)), { kind: 'notebook_identified', notebookUrl: window.location.href })
  job.notebookUrl = identified.notebookUrl
  assert.equal(window.sessionStorage.getItem('shuyin-create-intent'), job.id)
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

test('a missing notebook title control never blocks upload or changes other inputs', async () => {
  let generateClicks = 0
  const job = textJob({ stage: 'creating_notebook', projectTitle: '测试书', notebookUrl: 'https://notebook.google.com/notebook/notebook-1' })
  const { core, document } = page(`
    <header><input type="search" aria-label="Search notebooks"></header>
    <main><textarea aria-label="Audio instructions"></textarea><button id="generate">Generate</button></main>
  `)
  document.querySelector('#generate').addEventListener('click', () => { generateClicks += 1 })
  const result = await core.runStep(job, {})
  assert.equal(result.kind, 'notebook_created')
  assert.equal(document.querySelector('input').value, '')
  assert.equal(document.querySelector('textarea').value, '')
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

test('pastes a complete chapter after the Chinese Insert button becomes enabled', async (t) => {
  const { core, document, window } = page('<textarea id="prompt" aria-label="Audio instructions">保留音频指导</textarea><button id="add">添加来源</button><div id="portal"></div>')
  t.after(() => window.close())
  const text = `《测试书》·第三章\n\n${'章节原文保持完整。\n'.repeat(900)}末尾不丢失。`
  let inserted = ''
  let insertClicks = 0
  document.querySelector('#add').addEventListener('click', () => {
    document.querySelector('#portal').innerHTML = '<section role="dialog"><button id="copied">复制的文字</button></section>'
    document.querySelector('#copied').addEventListener('click', () => {
      document.querySelector('#portal').innerHTML = '<section role="dialog"><h2>粘贴复制的文字</h2><textarea placeholder="在此处粘贴文字"></textarea><button id="insert" disabled>插入</button></section>'
      document.querySelector('#portal textarea').addEventListener('input', () => {
        window.setTimeout(() => { document.querySelector('#insert').disabled = false }, 0)
      })
      document.querySelector('#insert').addEventListener('click', () => {
        insertClicks += 1
        inserted = document.querySelector('#portal textarea').value
      })
    })
  })
  const result = await core.runStep(textJob({ source: { kind: 'text', text } }))
  assert.equal(result.kind, 'source_submitted', JSON.stringify(result))
  assert.equal(insertClicks, 1)
  assert.equal(inserted, text)
  assert.equal(document.querySelector('#prompt').value, '保留音频指导')
})

test('a text source never uses an Insert button outside the current source dialog', async () => {
  const { core, document } = page('<button id="outside">插入</button><section role="dialog"><button id="copied">复制的文字</button><textarea placeholder="在此处粘贴文字" hidden></textarea></section>')
  let unrelatedClicks = 0
  document.querySelector('#outside').addEventListener('click', () => { unrelatedClicks += 1 })
  document.querySelector('#copied').addEventListener('click', () => { document.querySelector('textarea').hidden = false })
  const result = await core.runStep(textJob())
  assert.equal(unrelatedClicks, 0)
  assert.equal(result.kind, 'page_error')
  assert.equal(result.code, 'text_source_insert_missing')
})

test('a text Insert button that stays disabled times out without losing the source', async (t) => {
  const { core, document, window } = page('<section role="dialog"><button id="copied">复制的文字</button><textarea placeholder="在此处粘贴文字" hidden></textarea><button id="insert" disabled>插入</button></section>')
  t.after(() => window.close())
  let insertClicks = 0
  document.querySelector('#copied').addEventListener('click', () => { document.querySelector('textarea').hidden = false })
  document.querySelector('#insert').addEventListener('click', () => { insertClicks += 1 })
  const job = textJob()
  const result = await core.runStep(job)
  assert.equal(result.kind, 'page_error')
  assert.equal(result.code, 'text_source_insert_missing')
  assert.match(result.message, /插入.*未就绪/)
  assert.equal(insertClicks, 0)
  assert.equal(document.querySelector('#insert').disabled, true)
  assert.equal(document.querySelector('textarea').value, job.source.text)
})

test('switching source dialogs while waiting never submits a different source', async (t) => {
  for (const replaceDialog of [false, true]) {
    await t.test(replaceDialog ? 'replacement dialog' : 'same dialog with a different input', async (t) => {
      const { core, document, window } = page('<section role="dialog"><button id="copied">复制的文字</button><textarea placeholder="在此处粘贴文字" hidden></textarea><button id="insert" disabled>插入</button></section>')
      t.after(() => window.close())
      const input = document.querySelector('textarea')
      let unrelatedClicks = 0
      document.querySelector('#copied').addEventListener('click', () => { input.hidden = false })
      input.addEventListener('input', () => {
        window.setTimeout(() => {
          const otherSource = '<input aria-label="网站网址" value="https://example.com"><button id="other-insert">插入</button>'
          const dialog = document.querySelector('[role="dialog"]')
          if (replaceDialog) dialog.outerHTML = `<section role="dialog">${otherSource}</section>`
          else dialog.innerHTML = otherSource
          document.querySelector('#other-insert').addEventListener('click', () => { unrelatedClicks += 1 })
        }, 0)
      })
      const result = await core.runStep(textJob())
      assert.equal(unrelatedClicks, 0, JSON.stringify(result))
      assert.equal(result.kind, 'page_error')
      assert.equal(result.code, 'text_source_input_changed')
    })
  }
})

test('pausing while the text Insert button becomes enabled prevents submission', async (t) => {
  const { core, document, window } = page('<section role="dialog"><button id="copied">复制的文字</button><textarea placeholder="在此处粘贴文字" hidden></textarea><button id="insert" disabled>插入</button></section>')
  t.after(() => window.close())
  let insertClicks = 0
  document.querySelector('#copied').addEventListener('click', () => { document.querySelector('textarea').hidden = false })
  document.querySelector('textarea').addEventListener('input', () => {
    window.setTimeout(() => {
      window.__shuyinAudioRun = 'paused'
      document.querySelector('#insert').disabled = false
    }, 0)
  })
  document.querySelector('#insert').addEventListener('click', () => { insertClicks += 1 })
  const job = textJob()
  const result = await core.runStep(job)
  assert.equal(result.kind, 'page_error')
  assert.equal(result.code, 'stale_run')
  assert.equal(insertClicks, 0)
  assert.equal(document.querySelector('textarea').value, job.source.text)
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

test('configures audio and language and verifies the full prompt before generation', async () => {
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

test('configures the current Chinese audio dialog without submitting generation', async () => {
  for (const format of ['deep_dive', 'brief']) {
    const { core, document } = page(`
      <input aria-label="Notebook title" value="人生由我">
      <button>音频概览</button>
      <button>摘要</button>
      <section role="dialog" aria-label="自定义音频概览">
        <label><input id="deep" type="radio" name="format">深入探究</label>
        <label><input id="brief" type="radio" name="format">摘要</label>
        <div id="language" role="combobox" aria-label="选择语言" tabindex="0">${format === 'deep_dive' ? '中文（简体）' : 'English'}</div>
        <button id="short" role="radio" aria-checked="true">短</button>
        <button id="default" role="radio" aria-checked="false">默认</button>
        <label id="prompt-label" ${format === 'deep_dive' ? 'for="prompt"' : ''}>AI 主持人在本集节目中应着重于哪些方面？</label>
        <textarea id="prompt" ${format === 'brief' ? 'aria-labelledby="prompt-label"' : ''}></textarea>
        <button id="generate">生成</button>
      </section>
    `)
    let submissions = 0
    let languageOpens = 0
    let shortClicks = 0
    let defaultClicks = 0
    document.querySelector('#generate').addEventListener('click', () => { submissions += 1 })
    document.querySelector('#short').addEventListener('click', () => { shortClicks += 1 })
    document.querySelector('#default').addEventListener('click', () => { defaultClicks += 1 })
    document.querySelector('#language').addEventListener('click', () => {
      languageOpens += 1
      const option = document.createElement('div')
      option.setAttribute('role', 'option')
      option.textContent = '中文（简体）'
      option.addEventListener('click', () => { document.querySelector('#language').textContent = option.textContent; option.remove() })
      document.body.append(option)
    })
    const job = textJob({ stage: 'configuring_audio', format, targetDurationSeconds: format === 'brief' ? 1800 : 600 })
    const result = await core.runStep(job)
    assert.equal(result.kind, 'audio_configured', JSON.stringify(result))
    assert.equal(result.format, format)
    assert.equal(result.language, 'zh-Hans')
    assert.equal(document.querySelector(format === 'brief' ? '#brief' : '#deep').checked, true)
    assert.equal(document.querySelector('#language').textContent, '中文（简体）')
    assert.equal(languageOpens, format === 'deep_dive' ? 0 : 1)
    assert.equal(shortClicks, 0)
    assert.equal(defaultClicks, 1)
    assert.equal(result.length, 'default')
    assert.equal(document.querySelector('#prompt').value, job.prompt)
    assert.equal(document.querySelector('input[aria-label="Notebook title"]').value, '人生由我')
    assert.equal(submissions, 0)
  }
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
  const result = await core.runStep(textJob({ stage: 'downloading', notebookUrl: 'https://notebook.google.com/notebook/task-notebook' }), {})
  assert.equal(result.kind, 'page_error')
  assert.equal(result.code, 'wrong_notebook')
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

  fixture = page('<div class="single-source-container"><span>book.pdf</span><input type="checkbox" checked></div><button id="generate">Generate</button>', notebookUrl)
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

test('captures the audio menu window.open download without opening a popup', async () => {
  const { core, window, document } = page('<section><h2>音频概览</h2><button>播放</button><button id="more">更多</button></section>')
  const opened = []
  const originalOpen = window.open = (...args) => { opened.push(args); return null }
  document.querySelector('#more').addEventListener('click', () => {
    window.open('https://example.com/help', '_blank')
    const menu = document.createElement('div')
    menu.setAttribute('role', 'menu')
    menu.innerHTML = '<button role="menuitem">下载</button>'
    menu.querySelector('button').addEventListener('click', () => { window.open('https://content.example.com/episode.m4a?download=1', '_blank') })
    document.body.append(menu)
  })
  const result = await core.captureDownloadTarget()
  assert.equal(result.kind, 'download_target')
  assert.equal(result.downloadUrl, 'https://content.example.com/episode.m4a?download=1')
  assert.deepEqual(opened, [['https://example.com/help', '_blank']])
  assert.equal(window.open, originalOpen)
})

test('download interception restores window.open when the audio menu has no download', async () => {
  const { core, window, document } = page('<section><h2>Audio Overview</h2><button>Play</button><button id="more">More</button></section>')
  const originalOpen = window.open = () => null
  document.querySelector('#more').addEventListener('click', () => {
    const menu = document.createElement('div')
    menu.setAttribute('role', 'menu')
    menu.innerHTML = '<button>Something else</button>'
    document.body.append(menu)
  })
  await assert.rejects(core.captureDownloadTarget(), error => error.code === 'download_missing')
  assert.equal(window.open, originalOpen)
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
  const { core, document } = page('<section role="dialog"><button>上传文件</button><input id="source" type="file"></section>')
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
  const result = await core.runStep(textJob())
  assert.equal(result.kind, 'page_error')
  assert.equal(result.code, 'stale_run')
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
  const result = await core.runStep(textJob({stage:'generating',notebookUrl:'https://notebook.google.com/notebook/task'}))
  assert.equal(result.kind, 'page_error')
  assert.equal(result.code, 'wrong_notebook')
})

test('file upload follows the source-panel Add source then dialog Upload files sequence', async () => {
  const { core, document, window } = page(`
    <input value="未命名的笔记本">
    <section><nav><h2>来源</h2></nav>
      <button id="add"><span aria-hidden="true">add</span><span>添加来源</span></button>
      <p>将文件拖到此处或 <button id="empty-add">添加来源</button></p>
    </section>
    <main><button id="other-add">添加来源</button><span>0 个来源</span></main>
    <section role="dialog" hidden><button id="upload">上传文件</button><input id="source" type="file" hidden></section>
    <input id="unrelated-file" type="file">
  `)
  const clicks = []
  for (const id of ['add', 'empty-add', 'other-add', 'upload']) {
    document.getElementById(id).addEventListener('click', () => {
      clicks.push(id)
      if (id === 'add') document.querySelector('[role="dialog"]').hidden = false
    })
  }
  const job = textJob({ stage: 'creating_notebook', notebookUrl: window.location.href, source: { kind: 'asset', name: '原书.epub' } })
  assert.equal(core.canRecoverEmptyNotebook(), true)
  assert.equal((await core.runStep(job)).kind, 'notebook_created')
  assert.deepEqual(clicks, [])
  const result = await core.runStep({ ...job, stage: 'adding_source' })
  assert.equal(result.kind, 'file_input_ready')
  assert.deepEqual(clicks, ['add', 'upload'])
  assert.equal(document.getElementById('source').getAttribute('data-shuyin-audio-source'), 'job-1:1')
  assert.equal(document.getElementById('unrelated-file').hasAttribute('data-shuyin-audio-source'), false)
  assert.equal(document.querySelector('input').value, '未命名的笔记本')
})

test('an already open source dialog uploads without clicking Add source again', async () => {
  const { core, document } = page('<button id="add">添加来源</button><section role="dialog"><button id="upload">上传文件</button><input type="file"></section>')
  const clicks = []
  document.getElementById('add').addEventListener('click', () => clicks.push('add'))
  document.getElementById('upload').addEventListener('click', () => clicks.push('upload'))
  const result = await core.runStep(textJob({ source: { kind: 'asset', name: '原书.epub' } }))
  assert.equal(result.kind, 'file_input_ready')
  assert.deepEqual(clicks, ['upload'])
})

test('file upload initializes the real uploader when the native window is unfocused', async () => {
  const { core, document } = page('<section role="dialog"><button id="upload">上传文件</button></section><input id="unrelated" type="file">')
  const upload = document.getElementById('upload')
  const events = []
  upload.focus = () => {}
  upload.addEventListener('mouseover', () => {
    events.push('hover')
    const input = document.createElement('input')
    input.type = 'file'
    input.hidden = true
    input.addEventListener('click', () => events.push('file'))
    upload.after(input)
  }, { once: true })
  upload.addEventListener('click', () => {
    events.push('upload')
    upload.nextElementSibling?.click()
  })
  const result = await core.runStep(textJob({ source: { kind: 'asset', name: '原书.epub' } }))
  assert.equal(result.kind, 'file_input_ready', JSON.stringify(result))
  assert.deepEqual(events, ['hover', 'upload', 'file'])
  assert.equal(upload.nextElementSibling.getAttribute('data-shuyin-audio-source'), 'job-1:1')
  assert.equal(document.getElementById('unrelated').hasAttribute('data-shuyin-audio-source'), false)
})

test('file upload never skips a missing dialog or Upload files control', async () => {
  for (const html of [
    '<button>添加来源</button><button>上传文件</button><input type="file">',
    '<section role="dialog"><input type="file"></section>',
    '<section role="dialog"><button>上传文件</button></section><input type="file">',
  ]) {
    const { core, document, window } = page(html)
    // Keep timeout failures deterministic and fast without changing the real step.
    const now = window.Date.now.bind(window.Date)
    let elapsed = 0
    window.Date.now = () => now() + (elapsed += 1000)
    const result = await core.runStep(textJob({ source: { kind: 'asset', name: '原书.epub' } }))
    assert.equal(result.kind, 'page_error')
    assert.ok(['source_dialog_missing', 'upload_files_missing', 'file_input_missing'].includes(result.code))
    assert.equal(document.querySelector('[data-shuyin-audio-source]'), null)
  }
})

test('ambiguous upload controls return a serializable error code and full message', async () => {
  const { core, document } = page('<section role="dialog"><button>上传文件</button><button>上传文件</button><input type="file"></section>')
  let clicks = 0
  document.querySelectorAll('button').forEach(button => button.addEventListener('click', () => clicks++))
  const result = JSON.parse(JSON.stringify(await core.runStep(textJob({ source: { kind: 'asset', name: '原书.epub' } }))))
  assert.equal(result.kind, 'page_error')
  assert.equal(result.code, 'ambiguous_control')
  assert.match(result.message, /多个.*Upload files/)
  assert.notEqual(result.message, 'UiError')
  assert.equal(clicks, 0)
})

test('multiple unidentified source panels cannot recover a notebook or click a guessed entry', async () => {
  const { core, document, window } = page('<section><h2>来源</h2><button>添加来源</button></section><section><h2>来源</h2><button>添加来源</button></section><span>0 个来源</span>')
  let clicks = 0
  document.querySelectorAll('button').forEach(button => button.addEventListener('click', () => clicks++))
  assert.equal(core.canRecoverEmptyNotebook(), false)
  const now = window.Date.now.bind(window.Date)
  let elapsed = 0
  window.Date.now = () => now() + (elapsed += 1000)
  const result = await core.runStep(textJob({ source: { kind: 'asset', name: '原书.epub' } }))
  assert.equal(result.kind, 'page_error')
  assert.equal(clicks, 0)
})
