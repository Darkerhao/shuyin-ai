import { readFileSync } from 'node:fs'
import { runInNewContext } from 'node:vm'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

const source = readFileSync('src-tauri/src/douyin_editor.js', 'utf8')
const innerTextDescriptor = Object.getOwnPropertyDescriptor(HTMLElement.prototype, 'innerText')
const pageLocation = { origin: 'https://creator.douyin.com', pathname: '/creator-micro/content/post/video' }
let act: (action: string, payload?: Record<string, unknown>) => Record<string, unknown>
const payload = { title: '中文书名', description: '第一段\n第二段', tags: '#读书' }
beforeEach(() => {
  Object.assign(pageLocation, { origin: 'https://creator.douyin.com', pathname: '/creator-micro/content/post/video' })
  document.body.innerHTML = `<input placeholder="填写作品标题，为作品获得更多流量"><div contenteditable="true" data-slate-editor="true" class="editor-comp-publish"></div><video></video>
    <div class="coverControl-a"><span>选择封面</span><div>横封面4:3</div></div>
    <div class="coverControl-b"><span>选择封面</span><div>竖封面3:4</div></div><span>预览视频</span><button id="publish">发布</button>`
  vi.spyOn(HTMLElement.prototype, 'getClientRects').mockReturnValue([1] as unknown as DOMRectList)
  Object.defineProperty(HTMLElement.prototype, 'innerText', { configurable: true, get() { return this.textContent }, set(value: string) { this.textContent = value } })
  vi.spyOn(HTMLMediaElement.prototype, 'readyState', 'get').mockReturnValue(2)
  vi.spyOn(HTMLMediaElement.prototype, 'duration', 'get').mockReturnValue(60)
  vi.spyOn(navigator, 'onLine', 'get').mockReturnValue(true)
  document.execCommand = vi.fn(() => false)
  act = runInNewContext(source, { document, window, location: pageLocation, navigator, getComputedStyle, HTMLInputElement, Event, KeyboardEvent })
})
afterEach(() => {
  vi.restoreAllMocks()
  document.body.innerHTML = ''
  if (innerTextDescriptor) Object.defineProperty(HTMLElement.prototype, 'innerText', innerTextDescriptor)
  else Reflect.deleteProperty(HTMLElement.prototype, 'innerText')
})

describe('official editor adapter safeguards (local DOM fixtures)', () => {
  function settings(download = true, scheduled = false) {
    document.body.insertAdjacentHTML('beforeend', `<div class="content-obt4oA"><div><span>保存权限</span></div><div class="download-content-Lci5tL"><label data-checked="${download}"><input type="checkbox" value="1"><span>允许 </span></label><label data-checked="${!download}"><input type="checkbox" checked value="0"><span>不允许 </span></label></div></div>
      <div class="content-obt4oA"><div><span>发布时间</span></div><div class="schedule-part-V615Zn"><div class="row-suTOx_"><label data-checked="${!scheduled}"><input type="checkbox" value="0"><span>立即发布 </span></label><label data-checked="${scheduled}"><input type="checkbox" value="1"><span>定时发布 </span></label></div><div class="date-picker-ioPchj"><input class="semi-input" placeholder="日期和时间"></div></div></div>`)
    document.querySelectorAll('label[data-checked]').forEach(label => label.addEventListener('click', event => {
      if (event.target instanceof HTMLInputElement) return
      label.parentElement!.querySelectorAll('label').forEach(item => item.setAttribute('data-checked', String(item === label)))
    }))
  }
  it('reads save permissions from the official selected label and applies both settings without submitting', () => {
    settings()
    const publish = vi.fn()
    document.querySelector('#publish')!.addEventListener('click', publish)
    expect(act('settings')).toMatchObject({ ok: true, allowDownload: true, scheduledAt: null })
    expect(act('setDownload', { allowDownload: false })).toMatchObject({ ok: true })
    expect(act('settings')).toMatchObject({ allowDownload: false })
    expect(act('setPublishMode', { scheduled: true })).toMatchObject({ ok: true })
    const scheduledAt = '2099-01-05T18:30:00+08:00'
    const input = document.querySelector<HTMLInputElement>('input[placeholder="日期和时间"]')!
    let committed = ''
    // Semi Input commits through keypress; merely changing the DOM value is insufficient.
    input.addEventListener('keypress', event => { if (event.key === 'Enter') committed = input.value })
    input.addEventListener('blur', () => { input.value = committed })
    expect(act('setSchedule', { scheduledAt })).toMatchObject({ ok: true })
    expect(committed).not.toBe('')
    expect(act('settings')).toMatchObject({ allowDownload: false, scheduledAt })
    expect(act('setPublishMode', { scheduled: false })).toMatchObject({ ok: true })
    expect(act('settings')).toMatchObject({ allowDownload: false, scheduledAt: null })
    expect(publish).not.toHaveBeenCalled()
  })
  it('never toggles already matching options and refuses missing, ambiguous, disabled or modal controls', () => {
    settings()
    const click = vi.fn()
    document.querySelectorAll('label').forEach(label => label.addEventListener('click', click))
    expect(act('setDownload', { allowDownload: true })).toMatchObject({ ok: true })
    expect(act('setPublishMode', { scheduled: false })).toMatchObject({ ok: true })
    expect(click).not.toHaveBeenCalled()
    document.querySelector('.download-content-Lci5tL input[value="0"]')!.setAttribute('disabled', '')
    expect(act('setDownload', { allowDownload: false })).toMatchObject({ ok: false })
    expect(act('setSchedule', { scheduledAt: 'invalid' })).toMatchObject({ ok: false })
    document.body.insertAdjacentHTML('beforeend', '<div role="dialog">官方提示</div>')
    expect(act('settings')).toMatchObject({ ok: false })
    document.querySelector('[role="dialog"]')!.remove()
    document.body.append(document.querySelector('.content-obt4oA')!.cloneNode(true))
    expect(act('settings')).toMatchObject({ ok: true, allowDownload: null, scheduled: false, scheduledAt: null })
    expect(act('setDownload', { allowDownload: true })).toMatchObject({ ok: false })
    expect(click).not.toHaveBeenCalled()
  })
  it('does not treat an empty or rejected scheduled time as immediate publication', () => {
    settings(true, true)
    expect(act('settings')).toMatchObject({ ok: true, scheduledAt: '' })
    const input = document.querySelector<HTMLInputElement>('input[placeholder="日期和时间"]')!
    input.addEventListener('input', () => { input.value = '2099-01-05 16:00' })
    const scheduledAt = '2099-01-05T18:30:00+08:00'
    act('setSchedule', { scheduledAt })
    expect(act('settings').scheduledAt).not.toBe(scheduledAt)
  })
  it('preserves readable save permission when publish-time controls are unavailable', () => {
    settings(true)
    document.querySelectorAll('.content-obt4oA')[1]!.remove()
    expect(act('settings')).toMatchObject({ ok: true, allowDownload: true, scheduled: null, scheduledAt: null })
    expect(act('setPublishMode', { scheduled: true })).toMatchObject({ ok: false })
  })
  it('preserves readable publish mode and time when save permission or the date input is unavailable', () => {
    settings(false, true)
    document.querySelector('.content-obt4oA')!.remove()
    const input = document.querySelector<HTMLInputElement>('input[placeholder="日期和时间"]')!
    input.value = '2099-01-05 18:30'
    expect(act('settings')).toMatchObject({ ok: true, allowDownload: null, scheduled: true })
    expect(act('settings').scheduledAt).toBeTruthy()
    input.remove()
    expect(act('settings')).toMatchObject({ ok: true, allowDownload: null, scheduled: true, scheduledAt: null })
  })
  it('waits for the official document and the unique upload control to mount', () => {
    const inspect = () => runInNewContext(readFileSync('src-tauri/src/douyin_page.js', 'utf8'), { document, location: pageLocation, navigator, getComputedStyle })
    pageLocation.pathname = '/creator-micro/content/upload'
    document.body.innerHTML = ''
    const readyState = vi.spyOn(document, 'readyState', 'get').mockReturnValue('loading')
    expect(inspect()).toBeNull()
    readyState.mockReturnValue('complete')
    expect(inspect()).toMatchObject({ kind: 'upload', uploadReady: false })
    document.body.innerHTML = '<input type="file" accept="video/*" disabled>'
    expect(inspect()).toMatchObject({ uploadReady: false })
    document.querySelector('input')!.disabled = false
    expect(inspect()).toMatchObject({ kind: 'upload', uploadReady: true })
    document.body.append(document.querySelector('input')!.cloneNode())
    expect(inspect()).toMatchObject({ uploadReady: false })
  })

  it('recognizes the creator home without requiring its publish menu to be expanded', () => {
    const inspect = () => runInNewContext(readFileSync('src-tauri/src/douyin_page.js', 'utf8'), { document, location: pageLocation, navigator, getComputedStyle })
    pageLocation.pathname = '/creator-micro/home'
    document.body.innerHTML = '<div id="header-avatar"><div class="avatar-a" style="background-image:url(https://example.test/avatar.png)"></div></div><div id="douyin-creator-master-side-upload"><svg></svg>作品发布</div>'
    expect(inspect()).toMatchObject({ kind: 'home', authenticated: true, blocked: false })
    document.body.insertAdjacentHTML('beforeend', '<div role="dialog">你还有上次未发布的视频，是否继续编辑？</div>')
    expect(inspect()).toMatchObject({ kind: 'draft', blocked: true })
    pageLocation.origin = 'https://example.com'
    expect(inspect()).toBeNull()
  })

  it('reads platform status text beside decorative child icons', () => {
    const inspect = () => runInNewContext(readFileSync('src-tauri/src/douyin_page.js', 'utf8'), { document, location: pageLocation, navigator, getComputedStyle })
    document.body.insertAdjacentHTML('beforeend', '<span id="status"><svg></svg>上传成功</span>')
    expect(inspect()).toMatchObject({ processed: true, uploading: false })
    document.querySelector('#status')!.innerHTML = '<svg></svg>上传中'
    expect(inspect()).toMatchObject({ processed: false, uploading: true })
  })
  it('prepares exact Chinese and line breaks for native input, fires title events and never clicks publish', () => {
    const title = document.querySelector('input')!
    const input = vi.fn(); title.addEventListener('input', input)
    const publish = vi.fn(); document.getElementById('publish')!.addEventListener('click', publish)
    const prepared = act('prepareText', payload)
    expect(prepared).toMatchObject({ ok: true, needsInput: true, text: '第一段\n第二段\n#读书' })
    expect(input).toHaveBeenCalledOnce()
    expect(document.querySelector('[contenteditable]')!.textContent).toBe('')
    document.querySelector('[contenteditable]')!.textContent = prepared.text as string
    expect(act('prepareText', payload)).toMatchObject({ ok: true, needsInput: false })
    expect(input).toHaveBeenCalledOnce()
    expect(act('publish').ok).toBe(false)
    expect(publish).not.toHaveBeenCalled()
  })
  it('prepares all generated hashtags once and does not request native insertion on continuation', () => {
    const generated = { title: '如何把钱花在值得的地方', description: '读《高性价比人生指南》，一起看清生活中的取舍。', tags: '#读书 #高性价比人生指南 #生活选择 #好书分享' }
    const expected = `${generated.description}\n${generated.tags}`
    expect(act('prepareText', generated)).toMatchObject({ ok: true, needsInput: true, text: expected })
    document.querySelector('[contenteditable]')!.textContent = expected
    expect(act('read')).toMatchObject({ description: expected.replace(/\n+/g, '\n') })
    expect(act('prepareText', generated)).toMatchObject({ ok: true, needsInput: false, text: expected })
    expect(document.execCommand).not.toHaveBeenCalled()
  })
  it('selects only the description and sends paragraph boundaries without empty placeholder lines', () => {
    const editor = document.querySelector<HTMLElement>('[contenteditable]')!
    const multiline = { ...payload, description: '第一段\n\n第二段\n第三段', tags: '#读书 #阅读' }
    const expected = `${multiline.description}\n\n${multiline.tags}`.replace(/\n+/g, '\n')
    editor.setAttribute('tabindex', '0')
    expect(act('prepareText', multiline)).toMatchObject({ ok: true, needsInput: true, text: expected })
    const range = window.getSelection()!.getRangeAt(0)
    expect(range.commonAncestorContainer).toBe(editor)
    expect(document.activeElement).toBe(editor)
    expect(editor.textContent).toBe('')
    expect(document.execCommand).not.toHaveBeenCalled()
  })
  it('refuses a different existing description before changing either field', () => {
    document.querySelector('[contenteditable]')!.removeAttribute('data-placeholder')
    document.querySelector('[contenteditable]')!.textContent = '人工编辑的内容'
    expect(act('prepareText', payload)).toMatchObject({ ok: false })
    expect(document.querySelector('input')!.value).toBe('')
    expect(document.querySelector('[contenteditable]')!.textContent).toBe('人工编辑的内容')
  })
  it('accepts equivalent paragraph spacing but refuses text with missing or reordered paragraphs', () => {
    const editor = document.querySelector<HTMLElement>('[contenteditable]')!
    editor.textContent = '第一段\n第二段\u200b\n#读书'
    expect(act('prepareText', payload)).toMatchObject({ ok: true, needsInput: false })
    editor.textContent = '第一段第二段#读书'
    expect(act('prepareText', payload)).toMatchObject({ ok: false })
    editor.textContent = '第二段\n第一段\n#读书'
    expect(act('prepareText', payload)).toMatchObject({ ok: false })
  })
  it('refuses missing or duplicate controls without changing existing text', () => {
    document.body.append(document.querySelector('input')!.cloneNode())
    expect(act('prepareText', payload).ok).toBe(false)
    expect(document.querySelector('[contenteditable]')!.textContent).toBe('')
    document.querySelectorAll('input').forEach(input => input.remove())
    expect(act('prepareText', payload).ok).toBe(false)
  })
  it('pauses for offline state, a modal and unprocessed video', () => {
    vi.mocked(vi.spyOn(navigator, 'onLine', 'get')).mockReturnValue(false)
    expect(act('prepareText', payload).ok).toBe(false)
    vi.mocked(vi.spyOn(navigator, 'onLine', 'get')).mockReturnValue(true)
    document.body.insertAdjacentHTML('beforeend', '<div role="dialog">需要人工确认</div>')
    expect(act('prepareText', payload).ok).toBe(false)
    document.querySelector('[role=dialog]')!.remove()
    document.querySelector('video')!.remove()
    expect(act('prepareText', payload).ok).toBe(false)
  })
  it('cannot select a video over a retained draft or on the editor route', () => {
    expect(act('videoInput').ok).toBe(false)
    pageLocation.pathname = '/creator-micro/content/upload'
    document.body.innerHTML = '<input type="file" accept="video/*"><span>你还有上次未发布的视频，是否继续编辑？</span>'
    expect(act('videoInput')).toMatchObject({ ok: false })
    document.querySelector('span')!.remove()
    expect(act('videoInput')).toBeInstanceOf(HTMLInputElement)
    pageLocation.origin = 'https://example.com'
    expect(act('videoInput').ok).toBe(false)
  })
  it('requires the matching cover ratio and a unique visible upload input', () => {
    document.body.insertAdjacentHTML('beforeend', '<div role="dialog"><span>横封面预览（4:3）</span><div class="semi-upload">点击上传文件<input type="file" class="semi-upload-hidden-input"></div><button>完成</button></div>')
    expect(act('coverInput', { ratio: 'portrait' }).ok).toBe(false)
    expect(act('coverInput', { ratio: 'landscape' })).toBeInstanceOf(HTMLInputElement)
    document.querySelector('.semi-upload')!.append(document.querySelector('input[type=file]')!.cloneNode())
    expect(act('coverInput', { ratio: 'landscape' }).ok).toBe(false)
    expect(act('coverReady', { ratio: 'landscape' })).toMatchObject({ ok: true, ready: false })
  })
  it('preserves already selected covers', () => {
    document.querySelector('.coverControl-a span')!.remove()
    const click = vi.fn(); document.querySelector('.coverControl-a')!.addEventListener('click', click)
    expect(act('openCover', { ratio: 'landscape' }).ok).toBe(false)
    expect(click).not.toHaveBeenCalled()
    expect(act('read')).toMatchObject({ covers: [{ ratio: 'landscape', selected: true }, { ratio: 'portrait', selected: false }] })
  })
  it('selects the video preview tab with its nested help icon while the video is processing', () => {
    document.querySelector('video')!.remove()
    document.body.insertAdjacentHTML('beforeend', '<div class="tabItem-a">预览视频<div class="tooltip-icon-a"><svg></svg></div></div><div class="tabItem-b">预览封面/标题</div>')
    const click = vi.fn(); document.querySelector('.tabItem-a')!.addEventListener('click', click)
    expect(act('previewVideo')).toMatchObject({ ok: true })
    expect(click).toHaveBeenCalledOnce()
  })
  it('waits for the preview panel to mount after the left editor controls', () => {
    const inspect = () => runInNewContext(readFileSync('src-tauri/src/douyin_page.js', 'utf8'), { document, location: pageLocation, navigator, getComputedStyle })
    expect(inspect()).toMatchObject({ editorReady: false })
    document.body.insertAdjacentHTML('beforeend', '<div class="tabItem-a">预览视频<div><svg></svg></div></div>')
    expect(inspect()).toMatchObject({ editorReady: true })
  })
  it('ignores hidden platform dialogs and reads a restored editor without its empty placeholder', () => {
    document.body.insertAdjacentHTML('beforeend', '<div role="dialog" style="visibility:hidden">平台隐藏的充值窗口</div>')
    expect(act('prepareText', payload)).toMatchObject({ ok: true, needsInput: true })
    document.querySelector('[contenteditable]')!.textContent = '第一段\n第二段\n\n#读书'
    expect(act('read')).toMatchObject({ coverDialog: false, description: '第一段\n第二段\n#读书' })
  })
  it('does not infer authentication from a missing login form or processing from playable video', () => {
    const inspect = () => runInNewContext(readFileSync('src-tauri/src/douyin_page.js', 'utf8'), { document, location: pageLocation, navigator, getComputedStyle })
    expect(inspect()).toMatchObject({ authenticated: false, processed: false })
    document.body.insertAdjacentHTML('beforeend', '<div id="header-avatar"><div class="avatar-a" style="background-image:url(https://example.test/avatar.png)"></div></div><span id="douyin-creator-master-side-upload">作品发布</span>')
    expect(inspect()).toMatchObject({ authenticated: true, account: { id: null } })
    const avatar = document.querySelector('.avatar-a') as HTMLElement
    const background = avatar.style.backgroundImage
    avatar.style.backgroundImage = 'none'
    expect(inspect()).toMatchObject({ account: { key: '0' } })
    avatar.style.backgroundImage = background
    document.body.insertAdjacentHTML('beforeend', '<span>上传成功</span><div role="progressbar"></div>')
    expect(inspect()).toMatchObject({ processed: false, uploading: true })
    document.querySelector('[role=progressbar]')!.remove()
    expect(inspect()).toMatchObject({ processed: true })
    document.body.insertAdjacentHTML('beforeend', '<input placeholder="请输入手机号">')
    expect(inspect()).toMatchObject({ authenticated: false, loginVisible: true, account: null })
  })
  it('reads the captured collection control and selects only an exact unique option', () => {
    document.body.insertAdjacentHTML('beforeend', '<div class="semi-select select-collection-a"><span class="semi-select-selection-text">请选择合集</span></div><div role="listbox"><div role="option"><span class="option-title-a">读书</span></div><div role="option"><span class="option-title-b">历史</span></div></div>')
    expect(act('collections')).toMatchObject({ available: true, loaded: true, selected: null, options: [{ id: null, name: '读书' }, { id: null, name: '历史' }] })
    const click = vi.fn(); document.querySelector('[role=option]')!.addEventListener('click', click)
    expect(act('selectCollection', { name: '读书' })).toMatchObject({ ok: true })
    expect(click).toHaveBeenCalledOnce()
    document.querySelector('[role=listbox]')!.append(document.querySelector('[role=option]')!.cloneNode(true))
    expect(act('selectCollection', { name: '读书' })).toMatchObject({ ok: false })
    expect(act('selectCollection', { name: '失效合集' })).toMatchObject({ ok: false })
    expect(click).toHaveBeenCalledOnce()
  })
  it('reads only the collection title when the selected value also shows its work count', () => {
    document.body.insertAdjacentHTML('beforeend', '<div class="semi-select select-collection-a"><span class="semi-select-selection-text"><div class="selected-item-uCP2L_"><div class="selected-item-title-ebAmMX">甲子读书 | 读懂一本书</div><div class="selected-item-extra-text-M2lci5">共24个作品</div></div></span></div>')
    expect(act('collections')).toMatchObject({ available: true, loaded: false, options: [], selected: '甲子读书 | 读懂一本书' })
    document.querySelector('.selected-item-extra-text-M2lci5')!.textContent = '共25个作品'
    expect(act('collections')).toMatchObject({ selected: '甲子读书 | 读懂一本书' })
  })
  it('keeps the same account key when only avatar query parameters or fallback images change', () => {
    const inspect = () => runInNewContext(readFileSync('src-tauri/src/douyin_page.js', 'utf8'), { document, location: pageLocation, navigator, getComputedStyle })
    document.body.insertAdjacentHTML('beforeend', '<div id="header-avatar"><div class="avatar-a"></div></div><span id="douyin-creator-master-side-upload">作品发布</span>')
    const avatar = document.querySelector('.avatar-a') as HTMLElement
    avatar.style.backgroundImage = 'url("https://example.test/avatar.png")'
    const original = inspect().account.key
    avatar.style.backgroundImage = 'url("https://example.test/avatar.png?size=80"), url("https://example.test/fallback.png")'
    expect(inspect().account.key).toBe(original)
    avatar.style.backgroundImage = 'url("https://example.test/another.png?size=80")'
    expect(inspect().account.key).not.toBe(original)
  })
  it('keeps one account identity when Douyin serves the same avatar from another CDN or at another size', () => {
    const inspect = () => runInNewContext(readFileSync('src-tauri/src/douyin_page.js', 'utf8'), { document, location: pageLocation, navigator, getComputedStyle })
    document.body.insertAdjacentHTML('beforeend', '<div id="header-avatar"><div class="avatar-a"></div></div><span id="douyin-creator-master-side-upload">作品发布</span>')
    const avatar = document.querySelector('.avatar-a') as HTMLElement
    avatar.style.backgroundImage = 'url("https://p11.douyinpic.com/aweme/100x100/aweme-avatar/tos-cn-i-account-a.jpeg?from=2956013662")'
    const original = inspect().account.key
    avatar.style.backgroundImage = 'url("https://p3.douyinpic.com/aweme/200x200/aweme-avatar/tos-cn-i-account-a.jpeg?from=another-page")'
    expect(inspect().account.key).toBe(original)
    avatar.style.backgroundImage = 'url("https://p3.douyinpic.com/aweme/200x200/aweme-avatar/tos-cn-i-account-b.jpeg")'
    expect(inspect().account.key).not.toBe(original)
  })
  it('cannot claim no collection while another selection is present or accept an unloaded list', () => {
    document.body.insertAdjacentHTML('beforeend', '<div class="semi-select select-collection-a"><span class="semi-select-selection-text">手工合集</span></div>')
    expect(act('selectCollection', { name: null })).toMatchObject({ ok: false })
    expect(act('selectCollection', { name: '读书' })).toMatchObject({ ok: false })
    expect(act('collections')).toMatchObject({ selected: '手工合集', loaded: false })
    expect(act('read')).toMatchObject({ title: '', description: '' })
  })
  it('detects background-image cover changes and retains a video identity for recovery', () => {
    document.querySelector('.coverControl-a')!.insertAdjacentHTML('afterbegin', '<div class="background-a" style="background-image:url(https://example.test/cover.png?identity=1)"></div>')
    document.querySelector('video')!.src = 'blob:current-video'
    expect(act('read')).toMatchObject({ videoKey: 'blob:current-video', covers: [{ preview: 'url("https://example.test/cover.png?identity=1")' }, { preview: '' }] })
  })
  it('restores video preview before checking the confirmed video at the collection step', () => {
    const video = document.querySelector('video')!
    video.src = 'blob:current-video'
    const confirmed = act('read')
    video.remove()
    document.body.insertAdjacentHTML('beforeend', '<div class="tabItem-a">预览视频<div><svg></svg></div></div><div class="tabItem-b active-a">预览封面/标题</div>')
    document.querySelector('.tabItem-a')!.addEventListener('click', () => document.body.append(video))
    const publish = vi.fn(); document.getElementById('publish')!.addEventListener('click', publish)

    expect(act('read')).toMatchObject({ videoPreview: false, videoKey: '' })
    expect(act('read', { video: true })).toMatchObject({ ok: true, videoPreview: true, videoKey: confirmed.videoKey })
    video.src = 'blob:another-video'
    expect(act('read', { video: true }).videoKey).not.toBe(confirmed.videoKey)
    expect(publish).not.toHaveBeenCalled()
  })
  it('does not use a cover-dialog video as proof of the publishing video or switch tabs through a dialog', () => {
    document.querySelector('video')!.src = 'blob:current-video'
    document.body.insertAdjacentHTML('beforeend', '<div class="tabItem-a">预览视频</div><div role="dialog"><video src="blob:cover-preview"></video></div>')
    const click = vi.fn(); document.querySelector('.tabItem-a')!.addEventListener('click', click)

    expect(act('read')).toMatchObject({ videoKey: 'blob:current-video', coverDialog: true })
    expect(act('read', { video: true })).toMatchObject({ ok: false })
    expect(click).not.toHaveBeenCalled()
  })
  it('reads a player mounted after the preview tab becomes active without clicking the tab again', () => {
    const video = document.querySelector('video')!
    video.src = 'blob:current-video'
    video.remove()
    document.body.insertAdjacentHTML('beforeend', '<div class="tabItem-a">预览视频</div>')
    const tab = document.querySelector('.tabItem-a')!
    const click = vi.fn(() => tab.classList.add('active-a'))
    tab.addEventListener('click', click)

    expect(act('read', { video: true })).toMatchObject({ ok: true, videoPreview: false, videoKey: '' })
    document.body.append(video)
    expect(act('read', { video: true })).toMatchObject({ ok: true, videoPreview: true, videoKey: 'blob:current-video' })
    expect(click).toHaveBeenCalledOnce()
  })
})
