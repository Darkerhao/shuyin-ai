(() => {
  try {
    if (location.origin !== 'https://creator.douyin.com') return null
    const visible = element => element.getClientRects().length > 0 && getComputedStyle(element).visibility === 'visible'
    const loginVisible = [...document.querySelectorAll('input[placeholder="请输入手机号"], input[placeholder="请输入验证码"]')].some(visible)
      || [...document.querySelectorAll('article')].some(element => visible(element) && element.textContent.includes('扫码登录'))
    const editorReady = Boolean(document.querySelector('input[placeholder="填写作品标题，为作品获得更多流量"]') && document.querySelector('[contenteditable="true"][data-slate-editor="true"].editor-comp-publish') && document.querySelector('[class^="coverControl-"]') && [...document.querySelectorAll('[class^="tabItem-"]')].some(e => visible(e) && e.textContent.trim() === '预览视频'))
    return { open: true, path: location.pathname, loginVisible, online: navigator.onLine, editorReady }
  } catch {
    return null
  }
})()
