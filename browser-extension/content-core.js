/* global document, location, sessionStorage, URL */
(() => {
  'use strict'

  const UI = {
    createNotebook: ['Create new notebook', 'Create notebook', 'New notebook', '新建笔记本', '创建笔记本', '新建'],
    createSubmit: ['Create notebook', 'Create', '创建笔记本', '创建'],
    signIn: ['Sign in', 'Log in', '登录', '登入'],
    addSource: ['Add source', 'Add sources', '添加来源', '添加资料来源'],
    uploadFiles: ['Upload files', 'Upload file', 'Choose files', '上传文件', '选择文件'],
    copiedText: ['Copied text', 'Paste text', 'Text', '复制的文字', '粘贴的文字', '文字'],
    insert: ['Insert', 'Add', 'Insert text', '插入', '添加'],
    audioOverview: ['Audio Overview', 'Audio overview', 'Audio overviews', '音频概览', '音频总览', '摘要音频'],
    customize: ['Customize', 'Customize audio', 'Customize Audio Overview', 'Customize audio overview', '自定义', '自定义音频'],
    generate: ['Generate', 'Generate audio', '生成', '生成音频'],
    download: ['Download', '下载'],
    more: ['More options', 'More', '更多', '更多选项'],
    deepDive: ['Deep Dive', 'Deep dive', '深入探索', '深度探索'],
    brief: ['Brief', '简报'],
    language: ['Language', 'Audio language', '语言', '音频语言'],
    format: ['Format', 'Audio format', '格式', '音频格式'],
    chinese: ['Chinese (Simplified)', 'Chinese', '中文（简体）', '中文(简体)', '简体中文'],
    defaultLength: ['Default', 'Default length', '默认', '默认时长'],
    longer: ['Longer', 'Longer audio', '更长', '详细'],
    shorter: ['Shorter', 'Shorter audio', '更短', '简短'],
    generating: ['Generating', 'Generating audio', 'Preparing', 'Preparing audio', '正在生成', '正在准备', '生成中'],
    failed: ["Can't generate", 'Could not generate', 'Something went wrong', '生成失败', '无法生成', '出现错误'],
    processing: ['Processing', 'Processing source', 'Analyzing', '正在处理', '正在分析'],
    play: ['Play', '播放'],
  }

  const INTERACTIVE = 'button, a, [role="button"], [role="menuitem"], [role="option"], [role="tab"], [role="radio"], [role="checkbox"], input[type="button"], input[type="submit"]'
  const TEXT_ELEMENTS = 'span, p, label, div, h1, h2, h3, h4, li, strong'
  const MAX_SOURCE_BYTES = 256 * 1024 * 1024

  class UiError extends Error {
    constructor(code, message) {
      super(message)
      this.name = 'UiError'
      this.code = code
    }
  }

  function normalize(value) {
    return String(value ?? '').replace(/\s+/g, ' ').trim()
  }

  function canonicalText(value) {
    return String(value ?? '').replace(/\r\n/g, '\n').replace(/\r/g, '\n').replace(/\s+$/u, '')
  }

  function visible(element) {
    if (!element || element.disabled || element.getAttribute('aria-disabled') === 'true') return false
    if (element.hidden || element.getAttribute('aria-hidden') === 'true') return false
    for (let parent = element.parentElement; parent; parent = parent.parentElement) {
      if (parent.hidden || parent.getAttribute('aria-hidden') === 'true') return false
    }
    const style = globalThis.getComputedStyle?.(element)
    if (style && (style.display === 'none' || style.visibility === 'hidden')) return false
    return true
  }

  function labelsOf(element) {
    return [
      element.innerText,
      element.textContent,
      element.getAttribute?.('aria-label'),
      element.getAttribute?.('title'),
      element.getAttribute?.('placeholder'),
      element.getAttribute?.('value'),
    ].map(normalize).filter(Boolean)
  }

  function matchScore(element, labels) {
    let best = 0
    const values = labelsOf(element).map(value => value.toLocaleLowerCase())
    for (const raw of labels) {
      const target = normalize(raw).toLocaleLowerCase()
      if (!target) continue
      for (const value of values) {
        if (value === target) best = Math.max(best, 100)
        else if (value.startsWith(target)) best = Math.max(best, 80)
        else if (value.includes(target)) best = Math.max(best, 60)
      }
    }
    return best
  }

  function findElements(labels, scope = document, selector = INTERACTIVE) {
    const root = scope?.querySelectorAll ? scope : document
    const matches = []
    for (const element of root.querySelectorAll(selector)) {
      if (!visible(element)) continue
      const score = matchScore(element, labels)
      if (score > 0) matches.push({ element, score })
    }
    return matches.sort((a, b) => b.score - a.score)
  }

  function findInteractive(labels, scope = document, options = {}) {
    const matches = findElements(labels, scope)
    if (!matches.length) return null
    const top = matches.filter(item => item.score === matches[0].score)
    if (top.length > 1 && !options.allowMultiple) {
      throw new UiError('ambiguous_control', `页面存在多个“${labels[0]}”控件，已停止自动操作。`)
    }
    return top[0].element
  }

  function findTextElements(labels, scope = document, exact = false) {
    const matches = []
    for (const element of scope.querySelectorAll(TEXT_ELEMENTS)) {
      if (!visible(element)) continue
      const text = normalize(element.textContent)
      if (!text || text.length > 500) continue
      const lower = text.toLocaleLowerCase()
      let score = 0
      for (const raw of labels) {
        const target = normalize(raw).toLocaleLowerCase()
        if (!target) continue
        if (lower === target) score = Math.max(score, 100)
        else if (!exact && lower.includes(target)) score = Math.max(score, 60)
      }
      if (score) matches.push({ element, score, depth: depthOf(element) })
    }
    return matches.sort((a, b) => b.score - a.score || b.depth - a.depth)
  }

  function depthOf(element) {
    let depth = 0
    for (let current = element; current; current = current.parentElement) depth += 1
    return depth
  }

  function hasAnyText(labels, scope = document) {
    return findTextElements(labels, scope).length > 0
  }

  function dialogScope() {
    const dialogs = [...document.querySelectorAll('[role="dialog"], dialog')].filter(visible)
    return dialogs.at(-1) ?? document
  }

  function findEditable(labels = [], scope = document) {
    const candidates = [...scope.querySelectorAll('textarea, input, [contenteditable="true"], [role="textbox"]')]
      .filter(element => visible(element))
      .filter(element => {
        const tag = element.tagName
        const type = String(element.getAttribute('type') ?? 'text').toLowerCase()
        return tag === 'TEXTAREA' || (tag === 'INPUT' && !['hidden', 'checkbox', 'radio', 'button', 'submit'].includes(type)) || element.getAttribute('contenteditable') === 'true' || element.getAttribute('role') === 'textbox'
      })
    if (!candidates.length) return null

    const scored = candidates.map(element => {
      let score = matchScore(element, labels)
      const id = element.id
      if (id) {
        const label = scope.querySelector?.(`label[for="${globalThis.CSS?.escape ? globalThis.CSS.escape(id) : id}"]`)
        if (label) score = Math.max(score, matchScore(label, labels))
      }
      if (element.tagName === 'TEXTAREA') score += 5
      const rows = Number(element.getAttribute('rows') ?? 0)
      if (rows >= 4) score += 3
      return { element, score }
    }).sort((a, b) => b.score - a.score)

    const top = scored.filter(item => item.score === scored[0].score)
    if (top.length === 1) return top[0].element
    if (!labels.length) {
      const textareas = top.filter(item => item.element.tagName === 'TEXTAREA')
      if (textareas.length === 1) return textareas[0].element
    }
    throw new UiError('ambiguous_input', '页面存在多个可能的文本输入框，已停止自动填写。')
  }

  function nativeSetValue(element, value) {
    const prototype = element.tagName === 'TEXTAREA' ? globalThis.HTMLTextAreaElement?.prototype : element.tagName === 'INPUT' ? globalThis.HTMLInputElement?.prototype : null
    const descriptor = prototype ? Object.getOwnPropertyDescriptor(prototype, 'value') : null
    if (descriptor?.set) descriptor.set.call(element, value)
    else element.value = value
  }

  function setEditable(element, value) {
    const maxLength = Number(element.getAttribute('maxlength') ?? 0)
    if (maxLength > 0 && value.length > maxLength) {
      throw new UiError('prompt_too_long', `网页输入框最多接受 ${maxLength} 个字符，本次内容未写入。`)
    }
    element.focus?.()
    element.dispatchEvent(new globalThis.Event('beforeinput', { bubbles: true, cancelable: true, inputType: 'insertText', data: value }))
    if (element.isContentEditable || element.getAttribute('contenteditable') === 'true') {
      element.textContent = value
      element.dispatchEvent(new globalThis.InputEvent('input', { bubbles: true, inputType: 'insertText', data: value }))
    } else {
      nativeSetValue(element, value)
      element.dispatchEvent(new globalThis.InputEvent('input', { bubbles: true, inputType: 'insertText', data: value }))
    }
    element.dispatchEvent(new globalThis.Event('change', { bubbles: true }))
  }

  function readEditable(element) {
    return canonicalText(element.isContentEditable || element.getAttribute('contenteditable') === 'true' ? element.textContent : element.value ?? element.textContent)
  }

  function writeAndVerify(element, value) {
    setEditable(element, value)
    const actual = readEditable(element)
    if (actual !== canonicalText(value)) {
      throw new UiError('prompt_readback_mismatch', '网页未完整保留音频指导，已停止生成；请打开浏览器人工核对。')
    }
  }

  function sleep(milliseconds) {
    return new Promise(resolve => globalThis.setTimeout(resolve, milliseconds))
  }

  async function waitFor(action, timeout = 5000, interval = 150) {
    const deadline = Date.now() + timeout
    while (Date.now() < deadline) {
      const result = action()
      if (result) return result
      await sleep(interval)
    }
    return null
  }

  function click(element, label) {
    if (!element) throw new UiError('control_missing', `网页未找到“${label}”控件，已停止自动操作。`)
    element.click()
  }

  function currentPath() {
    return location.pathname.replace(/\/$/, '')
  }

  function isNotebookPath(path = currentPath()) {
    return /^\/notebook\/[A-Za-z0-9_-]+/.test(path)
  }

  function sameRecordedNotebook(recorded, current = location.href) {
    if (!recorded) return false
    try {
      const a = new URL(recorded)
      const b = new URL(current)
      const allowed = new Set(['notebook.google.com', 'notebooklm.google.com'])
      return allowed.has(a.hostname) && allowed.has(b.hostname) && a.pathname.replace(/\/$/, '') === b.pathname.replace(/\/$/, '')
    } catch {
      return false
    }
  }

  function ensureTaskNotebook(job) {
    if (job.notebookUrl) {
      if (sameRecordedNotebook(job.notebookUrl)) return
      throw new UiError('wrong_notebook', '当前标签页不是本任务已记录的笔记本，已停止操作。')
    }
    if (isNotebookPath()) return
    throw new UiError('wrong_notebook', '当前标签页不是本任务已创建的笔记本，已停止操作。')
  }

  async function runLogin(job) {
    if (job.notebookUrl && sameRecordedNotebook(job.notebookUrl)) return { kind: 'login_ready' }
    if (!job.notebookUrl && isNotebookPath()) {
      return { kind: 'page_changed', message: '当前标签页已是某个既有笔记本，但本任务尚未记录新建笔记本；请返回主页重新开始。' }
    }
    const state = await waitFor(() => {
      const create = findInteractive(UI.createNotebook)
      if (create) return { kind: 'ready', element: create }
      const signIn = findInteractive(UI.signIn)
      if (signIn) return { kind: 'login' }
      return null
    }, 2500)
    if (state?.kind === 'ready') return { kind: 'login_ready' }
    if (state?.kind === 'login' || hasAnyText(UI.signIn)) return { kind: 'login_required' }
    return { kind: 'waiting', message: '等待 Gemini Notebook 页面加载完成。' }
  }

  function notebookTitleControl() {
    const headers = [...document.querySelectorAll('header, [role="banner"]')].filter(visible)
    const candidates = headers.flatMap(header => [...header.querySelectorAll('[contenteditable="true"], [role="textbox"], input:not([type="hidden"]):not([type="search"])')])
      .filter(visible)
    const explicit = candidates.filter(element => {
      const label = normalize([element.getAttribute('aria-label'), element.getAttribute('title'), element.getAttribute('placeholder')].filter(Boolean).join(' ')).toLocaleLowerCase()
      return /(?:notebook|document)?\s*title|标题|笔记本名称/u.test(label)
    })
    return explicit.length === 1 ? explicit[0] : null
  }

  function setNotebookTitle(job) {
    const title = notebookTitleControl()
    if (!title) return { kind: 'page_changed', message: '进入笔记本后未找到唯一的标题控件，已停止自动命名；请人工核对。' }
    const expected = `${job.projectTitle || '书引 AI 音频'} · 音频 ${String(job.id).slice(0, 8)}`
    try {
      title.click?.()
      writeAndVerify(title, expected)
      title.blur?.()
      return { kind: 'notebook_created', notebookUrl: location.href }
    } catch (error) {
      return { kind: 'page_changed', message: error?.message ?? '笔记本标题未能完整回读，已停止自动操作。' }
    }
  }

  function creationMarker(job) {
    return `${job.id}:${job.runId}`
  }

  function hasCreationMarker(job) {
    try {
      return sessionStorage.getItem('shuyin-create-intent') === creationMarker(job)
    } catch {
      return false
    }
  }

  function setCreationMarker(job, enabled) {
    try {
      if (enabled) sessionStorage.setItem('shuyin-create-intent', creationMarker(job))
      else sessionStorage.removeItem('shuyin-create-intent')
    } catch {
      // The next page will stop safely if the browser does not preserve session storage.
    }
  }

  async function createNotebook(job) {
    if (job.notebookUrl) {
      if (!sameRecordedNotebook(job.notebookUrl)) return { kind: 'page_changed', message: '当前标签页不是本任务记录的笔记本，已停止自动命名。' }
      return setNotebookTitle(job)
    }
    if (isNotebookPath()) {
      if (!hasCreationMarker(job)) return { kind: 'page_changed', message: '当前笔记本没有本任务的新建意图记录，已停止自动命名。' }
      setCreationMarker(job, false)
      return setNotebookTitle(job)
    }
    const create = await waitFor(() => findInteractive(UI.createNotebook), 3000)
    if (!create) throw new UiError('create_notebook_missing', '未找到新建笔记本入口；登录或页面版本可能已变化。')
    setCreationMarker(job, true)
    click(create, '新建笔记本')
    await sleep(350)
    const scope = dialogScope()
    if (scope !== document) {
      const title = findEditable(['Notebook title', 'Title', '笔记本标题', '标题'], scope)
      if (title) {
        const shortId = String(job.id).replace(/[^a-zA-Z0-9]/g, '').slice(0, 8)
        writeAndVerify(title, `${job.projectTitle || '书引 AI 音频'} · 音频 ${shortId}`)
      }
      const submit = findInteractive(UI.createSubmit, scope)
      if (submit && submit !== create) click(submit, '创建笔记本')
    }
    return { kind: 'navigating' }
  }

  function sourceName(job) {
    if (job.source?.kind === 'asset') return job.source.name
    return '书籍资料文本'
  }

  function mimeForName(name) {
    const extension = String(name).split('.').at(-1)?.toLowerCase()
    return {
      pdf: 'application/pdf', epub: 'application/epub+zip', txt: 'text/plain', md: 'text/markdown', docx: 'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
    }[extension] ?? 'application/octet-stream'
  }

  function decodeBase64(value) {
    const binary = globalThis.atob(value)
    const bytes = new Uint8Array(binary.length)
    for (let index = 0; index < binary.length; index += 1) bytes[index] = binary.charCodeAt(index)
    return bytes
  }

  async function readSourceFile(job, context) {
    if (!context?.sourceChunk) throw new UiError('source_transfer_missing', '本机音频桥接没有提供文件分块接口。')
    const chunks = []
    let offset = 0
    let totalSize = null
    let name = sourceName(job)
    let expectedHash = job.source?.sha256 ?? ''
    for (let count = 0; count < 4096; count += 1) {
      const response = await context.sourceChunk({ jobId: job.id, runId: job.runId, offset })
      if (!response?.ok) throw new UiError('source_transfer_failed', response?.error ?? '原书文件分块读取失败。')
      if (!Number.isInteger(response.totalSize) || response.totalSize <= 0) throw new UiError('source_invalid', '原书文件大小无效。')
      if (response.totalSize > MAX_SOURCE_BYTES) throw new UiError('source_too_large', '原书文件超过 256 MB，当前自动上传不支持。')
      if (totalSize !== null && response.totalSize !== totalSize) throw new UiError('source_changed', '原书文件在传输过程中发生变化，已停止上传。')
      totalSize = response.totalSize
      name = response.name || name
      expectedHash = response.sha256 || expectedHash
      const bytes = decodeBase64(response.data)
      if (!bytes.length && response.nextOffset < totalSize) throw new UiError('source_transfer_failed', '原书文件返回了空分块。')
      chunks.push(bytes)
      offset = response.nextOffset
      if (offset >= totalSize) break
      if (!Number.isInteger(offset) || offset <= 0) throw new UiError('source_transfer_failed', '原书文件分块游标无效。')
    }
    if (totalSize === null || offset !== totalSize) throw new UiError('source_transfer_failed', '原书文件未完整传输。')
    const combined = new Uint8Array(totalSize)
    let written = 0
    for (const chunk of chunks) {
      combined.set(chunk, written)
      written += chunk.length
    }
    if (written !== totalSize) throw new UiError('source_transfer_failed', '原书文件分块长度不一致，已停止上传。')
    return { file: new globalThis.File([combined], name, { type: mimeForName(name) }), name, totalSize, sha256: expectedHash }
  }

  function findSourceInput() {
    return [...document.querySelectorAll('input[type="file"]')].find(input => !input.disabled) ?? null
  }

  async function uploadAsset(job, scope, context) {
    const upload = await waitFor(() => findInteractive(UI.uploadFiles, scope), 3000)
    if (upload) click(upload, '上传文件')
    const input = await waitFor(findSourceInput, 5000)
    if (!input) throw new UiError('file_input_missing', '来源弹窗未提供文件上传控件。')
    const source = await readSourceFile(job, context)
    if (!globalThis.DataTransfer || !globalThis.File) throw new UiError('file_api_missing', '浏览器不支持安全文件注入。')
    const transfer = new globalThis.DataTransfer()
    transfer.items.add(source.file)
    input.files = transfer.files
    if (input.files.length !== 1 || input.files[0].size !== source.totalSize) {
      throw new UiError('file_injection_failed', '浏览器没有接收完整原书文件，已停止上传。')
    }
    input.dispatchEvent(new globalThis.Event('input', { bubbles: true }))
    input.dispatchEvent(new globalThis.Event('change', { bubbles: true }))
    return source
  }

  async function addTextSource(job, scope) {
    const copiedText = await waitFor(() => findInteractive(UI.copiedText, scope), 3000)
    if (!copiedText) throw new UiError('text_source_missing', '来源弹窗未提供“复制的文字”入口。')
    click(copiedText, '复制的文字')
    const input = await waitFor(() => findEditable(['Paste text', 'Copied text', 'Text', '粘贴的文字', '复制的文字', '文字'], scope), 3000)
    if (!input) throw new UiError('text_source_input_missing', '未找到资料文本输入框。')
    writeAndVerify(input, job.source.text)
    const insert = findInteractive(UI.insert, scope) ?? findInteractive(UI.insert)
    if (!insert) throw new UiError('text_source_insert_missing', '未找到插入资料文本按钮。')
    click(insert, '插入资料')
  }

  async function addSource(job, context) {
    ensureTaskNotebook(job)
    let scope = dialogScope()
    if (scope === document) {
      const add = await waitFor(() => findInteractive(UI.addSource), 3000)
      if (!add) throw new UiError('add_source_missing', '未找到添加来源入口，页面版本可能已变化。')
      click(add, '添加来源')
      await sleep(250)
      scope = await waitFor(() => (dialogScope() === document ? null : dialogScope()), 3000) ?? document
    }
    if (job.source.kind === 'text') await addTextSource(job, scope)
    else await uploadAsset(job, scope, context)
    await sleep(350)
    return { kind: 'source_submitted' }
  }

  function closestSourceRow(element, sourceNameValue) {
    let current = element
    for (let depth = 0; current && depth < 7; depth += 1, current = current.parentElement) {
      const text = normalize(current.textContent)
      if (text.includes(sourceNameValue) && (current.getAttribute('role') === 'listitem' || current.tagName === 'LI' || current.querySelector?.('[role="progressbar"], progress, [aria-busy="true"]') || text.length < sourceNameValue.length + 300)) {
        return current
      }
    }
    return element.parentElement ?? element
  }

  function sourceIsReady(job) {
    const name = sourceName(job)
    const markers = job.source.kind === 'text' && job.source.text ? [name, job.source.text.slice(0, 80)] : [name]
    const matches = findTextElements(markers, document)
    if (!matches.length) return hasAnyText(UI.processing) ? false : null
    const row = closestSourceRow(matches[0].element, name)
    const rowText = normalize(row.textContent)
    if (row.getAttribute('aria-busy') === 'true' || row.querySelector('[role="progressbar"], progress, [aria-busy="true"]')) return false
    if (UI.processing.some(label => rowText.toLocaleLowerCase().includes(label.toLocaleLowerCase()))) return false
    return true
  }

  async function waitForSource(job) {
    const state = await waitFor(() => {
      const ready = sourceIsReady(job)
      if (ready !== null) return ready
      return null
    }, 8000, 250)
    if (state === true) return { kind: 'source_ready' }
    if (state === false || hasAnyText(UI.processing)) return { kind: 'source_processing' }
    return { kind: 'waiting', message: '尚未确认原书来源处理完成。' }
  }

  function selectedChoice(element) {
    return element.getAttribute('aria-checked') === 'true' || element.getAttribute('aria-selected') === 'true' || element.getAttribute('aria-pressed') === 'true'
  }

  async function chooseDirectChoice(labels, options = {}) {
    const element = await waitFor(() => findInteractive(labels), options.timeout ?? 1200)
    if (!element) return false
    if (!selectedChoice(element)) click(element, labels[0])
    return true
  }

  async function chooseDropdown(triggerLabels, optionLabels) {
    const trigger = await waitFor(() => findInteractive(triggerLabels), 1500)
    if (!trigger) return false
    click(trigger, triggerLabels[0])
    const option = await waitFor(() => findInteractive(optionLabels), 2000)
    if (!option) throw new UiError('choice_missing', `下拉选项“${optionLabels[0]}”不可用，已停止自动操作。`)
    click(option, optionLabels[0])
    await sleep(150)
    return true
  }

  async function chooseFormat(job) {
    const labels = job.format === 'brief' ? UI.brief : UI.deepDive
    if (await chooseDirectChoice(labels)) return job.format
    if (await chooseDropdown(UI.format, labels)) return job.format
    throw new UiError('format_missing', '未找到音频格式选项，页面版本可能已变化。')
  }

  async function chooseLanguage() {
    if (await chooseDirectChoice(UI.chinese)) return 'zh-Hans'
    if (await chooseDropdown(UI.language, UI.chinese)) return 'zh-Hans'
    throw new UiError('language_missing', '未找到简体中文音频语言选项。')
  }

  async function chooseLength(job) {
    const minutes = Number(job.targetDurationSeconds ?? 0) / 60
    const labels = minutes >= 20 ? UI.longer : minutes > 0 && minutes <= 10 ? UI.shorter : UI.defaultLength
    await chooseDirectChoice(labels, { timeout: 500 }).catch(() => false)
    return labels[0]
  }

  function promptInput() {
    return findEditable(['Audio instructions', 'Instructions', 'Customize', 'Prompt', 'What should', '音频指导', '音频提示词', '自定义音频', '指导', '提示词'])
  }

  async function configureAudio(job) {
    ensureTaskNotebook(job)
    let prompt = promptInput()
    if (!prompt) {
      const audio = await waitFor(() => findInteractive(UI.audioOverview), 3000)
      if (!audio) throw new UiError('audio_overview_missing', '未找到 Audio Overview 入口，页面版本可能已变化。')
      click(audio, 'Audio Overview')
      await sleep(300)
      const customize = await waitFor(() => findInteractive(UI.customize), 3000)
      if (customize) click(customize, '自定义音频')
      prompt = await waitFor(() => promptInput(), 4000)
    }
    if (!prompt) throw new UiError('audio_prompt_missing', '未找到音频指导输入框。')
    const format = await chooseFormat(job)
    const language = await chooseLanguage()
    const length = await chooseLength(job)
    writeAndVerify(prompt, job.prompt)
    return { kind: 'audio_configured', format, language, length }
  }

  async function submitGeneration() {
    const scope = dialogScope()
    const generate = findInteractive(UI.generate, scope) ?? findInteractive(UI.generate)
    if (!generate) throw new UiError('generate_missing', '未找到生成音频按钮。')
    click(generate, '生成音频')
    await sleep(250)
    return { kind: 'generation_submitted' }
  }

  function audioArtifactScope() {
    for (const match of findTextElements(UI.audioOverview)) {
      let current = match.element
      for (let depth = 0; current && depth < 9; depth += 1, current = current.parentElement) {
        if (!current || ['BODY', 'HTML', 'MAIN'].includes(current.tagName)) break
        const eligible = ['SECTION', 'ARTICLE', 'LI', 'DIV'].includes(current.tagName) || ['region', 'listitem', 'dialog'].includes(current.getAttribute('role'))
        if (!eligible) continue
        const text = normalize(current.textContent)
        if (!text || text.length > 12000) continue
        const evidence = current.querySelector('audio, [role="progressbar"], progress') || findElements([...UI.download, ...UI.play], current).length > 0 || hasAnyText(UI.generating, current) || hasAnyText(UI.failed, current)
        if (evidence) return current
      }
    }
    return null
  }

  function detectGenerationState() {
    const scope = audioArtifactScope()
    if (!scope) return hasAnyText(UI.generating) ? 'generating' : 'unknown'
    if (hasAnyText(UI.failed, scope)) return 'failed'
    if (hasAnyText(UI.generating, scope)) return 'generating'
    if (scope.querySelector('audio')) return 'complete'
    if (findElements(UI.download, scope).length > 0) return 'complete'
    if (findElements(UI.play, scope).length > 0) return 'complete'
    return 'unknown'
  }

  function waitForGeneration() {
    const state = detectGenerationState()
    if (state === 'generating') return { kind: 'generating' }
    if (state === 'complete') return { kind: 'generation_complete' }
    if (state === 'failed') throw new UiError('generation_failed', 'Gemini Notebook 明确显示音频生成失败。')
    return { kind: 'waiting', message: '尚未检测到音频生成状态，继续低频核对。' }
  }

  function directDownloadAnchor(scope) {
    return [...scope.querySelectorAll('a[download][href]')]
      .filter(visible)
      .find(anchor => /^(https?:|blob:)/.test(anchor.href)) ?? null
  }

  function visibleMenu() {
    const menus = [...document.querySelectorAll('[role="menu"], [role="dialog"]')].filter(visible)
    return menus.at(-1) ?? null
  }

  async function captureDownloadTarget() {
    const scope = audioArtifactScope()
    if (!scope) throw new UiError('audio_artifact_missing', '未唯一识别到 Audio Overview 成品区域，已停止下载。')
    const existing = directDownloadAnchor(scope)
    if (existing) {
      return { kind: 'download_target', downloadUrl: existing.href, suggestedFilename: existing.download || '' }
    }
    let captured = null
    let audioActivatedAt = 0
    const capture = event => {
      const target = event.target
      if (target && scope.contains(target)) audioActivatedAt = Date.now()
      const anchor = target?.closest?.('a[href]') ?? target?.parentElement?.closest?.('a[href]')
      if (!anchor || !/^(https?:|blob:)/.test(anchor.href)) return
      if (scope.contains(anchor) || Date.now() - audioActivatedAt < 3000) {
        captured = { downloadUrl: anchor.href, suggestedFilename: anchor.download || '' }
      }
    }
    document.addEventListener('click', capture, true)
    try {
      const download = findInteractive(UI.download, scope)
      if (download) click(download, '下载')
      else {
        const more = findInteractive(UI.more, scope)
        if (!more) throw new UiError('download_missing', 'Audio Overview 区域未找到下载入口。')
        click(more, '更多')
        await sleep(250)
        const menu = await waitFor(visibleMenu, 1500)
        const menuDownload = menu ? findInteractive(UI.download, menu) : null
        if (!menuDownload) throw new UiError('download_missing', 'Audio Overview 菜单中未找到下载入口。')
        audioActivatedAt = Date.now()
        click(menuDownload, '下载')
      }
      const result = await waitFor(() => captured, 5000, 100)
      if (!result) throw new UiError('download_target_missing', 'Audio Overview 未暴露可精确关联的下载地址；本次不会猜测下载文件。')
      return { kind: 'download_target', ...result }
    } finally {
      document.removeEventListener('click', capture, true)
    }
  }

  function verifyRemote(job) {
    ensureTaskNotebook(job)
    const state = detectGenerationState()
    if (state === 'generating') return { kind: 'generating' }
    if (state === 'complete') return { kind: 'generation_complete' }
    if (state === 'failed') return { kind: 'page_changed', message: '原笔记本明确显示音频生成失败，请人工核对；不会自动重新生成。' }
    const source = sourceIsReady(job)
    if (source !== null) return { kind: 'source_processing', message: '已只读核对原笔记本来源，继续等待远端音频结果。' }
    return { kind: 'page_changed', message: '已定位原笔记本，但未确认来源或音频状态，请人工核对；不会重新添加来源或生成。' }
  }

  async function runStep(job, context = {}) {
    if (!job?.id || !Number.isInteger(Number(job.runId))) throw new UiError('invalid_job', '音频任务数据无效。')
    if (/accounts\.google\.com/i.test(location.href)) return { kind: 'login_required' }
    switch (job.stage) {
      case 'awaiting_extension':
      case 'login':
        return runLogin(job)
      case 'creating_notebook':
        return createNotebook(job)
      case 'adding_source':
        return addSource(job, context)
      case 'source_processing':
        return waitForSource(job)
      case 'configuring_audio':
        return configureAudio(job)
      case 'submitting':
        return submitGeneration()
      case 'generating':
        return waitForGeneration()
      case 'verify_remote':
        return verifyRemote(job)
      case 'downloading':
        ensureTaskNotebook(job)
        return captureDownloadTarget()
      default:
        return { kind: 'waiting', message: `未知页面阶段：${job.stage}` }
    }
  }

  globalThis.ShuyinAudioContentCore = Object.freeze({
    UiError,
    audioArtifactScope,
    captureDownloadTarget,
    configureAudio,
    detectGenerationState,
    findEditable,
    findInteractive,
    readEditable,
    readSourceFile,
    runStep,
    setNotebookTitle,
    setEditable,
    sourceIsReady,
    verifyRemote,
    writeAndVerify,
  })
})()








