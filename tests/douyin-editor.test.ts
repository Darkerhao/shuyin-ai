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
  document.execCommand = vi.fn((_command, _show, text) => { document.querySelector('[contenteditable]')!.textContent = text ?? ''; return true })
  act = runInNewContext(source, { document, window, location: pageLocation, navigator, getComputedStyle, HTMLInputElement, Event })
})
afterEach(() => {
  vi.restoreAllMocks()
  document.body.innerHTML = ''
  if (innerTextDescriptor) Object.defineProperty(HTMLElement.prototype, 'innerText', innerTextDescriptor)
  else Reflect.deleteProperty(HTMLElement.prototype, 'innerText')
})

describe('official editor adapter safeguards (local DOM fixtures)', () => {
  it('preserves Chinese and line breaks, fires title input events and never clicks publish', () => {
    const title = document.querySelector('input')!
    const input = vi.fn(); title.addEventListener('input', input)
    const publish = vi.fn(); document.getElementById('publish')!.addEventListener('click', publish)
    expect(act('fill', payload)).toMatchObject({ ok: true, title: payload.title, description: '第一段\n第二段\n\n#读书' })
    expect(input).toHaveBeenCalledOnce()
    expect(act('fill', payload).ok).toBe(true)
    expect(input).toHaveBeenCalledOnce()
    expect(act('publish').ok).toBe(false)
    expect(publish).not.toHaveBeenCalled()
  })
  it('refuses a different existing description before changing either field', () => {
    document.querySelector('[contenteditable]')!.removeAttribute('data-placeholder')
    document.querySelector('[contenteditable]')!.textContent = '人工编辑的内容'
    expect(act('fill', payload)).toMatchObject({ ok: false })
    expect(document.querySelector('input')!.value).toBe('')
    expect(document.querySelector('[contenteditable]')!.textContent).toBe('人工编辑的内容')
  })
  it('refuses missing or duplicate controls without changing existing text', () => {
    document.body.append(document.querySelector('input')!.cloneNode())
    expect(act('fill', payload).ok).toBe(false)
    expect(document.querySelector('[contenteditable]')!.textContent).toBe('')
    document.querySelectorAll('input').forEach(input => input.remove())
    expect(act('fill', payload).ok).toBe(false)
  })
  it('pauses for offline state, a modal and unprocessed video', () => {
    vi.mocked(vi.spyOn(navigator, 'onLine', 'get')).mockReturnValue(false)
    expect(act('fill', payload).ok).toBe(false)
    vi.mocked(vi.spyOn(navigator, 'onLine', 'get')).mockReturnValue(true)
    document.body.insertAdjacentHTML('beforeend', '<div role="dialog">需要人工确认</div>')
    expect(act('fill', payload).ok).toBe(false)
    document.querySelector('[role=dialog]')!.remove()
    document.querySelector('video')!.remove()
    expect(act('fill', payload).ok).toBe(false)
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
    expect(act('fill', payload).ok).toBe(true)
    expect(act('read')).toMatchObject({ coverDialog: false, description: '第一段\n第二段\n\n#读书' })
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
