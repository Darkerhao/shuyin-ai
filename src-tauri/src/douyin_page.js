(() => {
  try {
    if (location.origin !== 'https://creator.douyin.com' || document.readyState === 'loading') return null
    const visible = element => element.getClientRects().length > 0 && getComputedStyle(element).visibility === 'visible'
    const loginVisible = [...document.querySelectorAll('input[placeholder="请输入手机号"], input[placeholder="请输入验证码"]')].some(visible)
      || [...document.querySelectorAll('article')].some(element => visible(element) && element.textContent.includes('扫码登录'))
    const editorReady = Boolean(document.querySelector('input[placeholder="填写作品标题，为作品获得更多流量"]') && document.querySelector('[contenteditable="true"][data-slate-editor="true"].editor-comp-publish') && document.querySelector('[class^="coverControl-"]') && [...document.querySelectorAll('[class^="tabItem-"]')].some(e => visible(e) && e.textContent.trim() === '预览视频'))
    const videoInputs = [...document.querySelectorAll('input[type="file"][accept*="video/"]')]
    const uploadReady = videoInputs.length === 1 && !videoInputs[0].disabled && videoInputs[0].files.length === 0
    const text = [...document.querySelectorAll('body *')].filter(e => visible(e) && [...e.childNodes].some(node => node.nodeType === 3 && node.textContent.trim())).map(e => e.textContent.trim())
    const avatar = document.querySelector('#header-avatar')
    const authenticated = !loginVisible && Boolean(avatar && visible(avatar) && document.querySelector('#douyin-creator-master-side-upload'))
    // Captured official DOM exposes a preview nickname, not a stable public ID.
    const names = [...document.querySelectorAll('[class^="bottom-binder-"] [class^="user-name-"]')].filter(visible).map(e => e.textContent.trim().replace(/^@/, ''))
    const avatarImage = avatar?.querySelector('[class^="avatar-"]')
    const background = avatarImage ? getComputedStyle(avatarImage).backgroundImage : ''
    const identity = background && background !== 'none' ? background.split('?')[0] : ''
    let identityKey = 0
    for (const c of identity) identityKey = (Math.imul(identityKey, 31) + c.charCodeAt(0)) | 0
    const account = authenticated ? { id: null, name: names.length === 1 ? names[0] : '', key: String(identityKey) } : null
    const draft = text.some(t => t === '你还有上次未发布的视频，是否继续编辑？' || t === '继续编辑')
    const blocked = [...document.querySelectorAll('[role="dialog"]')].some(visible)
    const uploading = text.some(t => /^上传中|^正在上传|^\d+(\.\d+)?%$/.test(t)) || [...document.querySelectorAll('[role="progressbar"]')].some(visible)
    const processing = text.some(t => /^视频处理中|^正在处理|^转码中/.test(t))
    // A playable preview alone never proves platform processing is complete.
    const processed = !uploading && !processing && text.some(t => t === '上传成功' || t === '上传完成')
    const knownHome = ['/', '/creator-micro/home', '/creator-micro/content/manage'].includes(location.pathname)
    const kind = loginVisible ? 'login' : draft ? 'draft' : location.pathname === '/creator-micro/content/post/video' ? 'editor'
      : location.pathname === '/creator-micro/content/upload' ? 'upload' : authenticated && knownHome ? 'home' : 'unknown'
    return { open: true, path: location.pathname, loginVisible, online: navigator.onLine, editorReady, uploadReady, authenticated, account, kind, blocked, uploading, processing, processed }
  } catch {
    return null
  }
})()
