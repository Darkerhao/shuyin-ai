// Page controls observed in Douyin Creator Center on 2026-10-26.
((action, payload = {}) => {
  const visible = element => element.getClientRects().length > 0 && getComputedStyle(element).visibility === 'visible'
  const unique = (elements, message) => {
    if (elements.length !== 1) throw new Error(message)
    return elements[0]
  }
  const exact = (root, text) => [...root.querySelectorAll('*')].filter(e => e.children.length === 0 && e.textContent.trim() === text && visible(e))
  const clean = text => text.replace(/\u200b/g, '').replace(/\r\n/g, '\n').trim()
  const previewVideo = () => {
    if ([...document.querySelectorAll('[role="dialog"]')].some(visible)) throw new Error('请先处理官方页弹窗，再核对视频。')
    const tab = unique([...document.querySelectorAll('[class^="tabItem-"]')].filter(e => visible(e) && e.textContent.trim() === '预览视频'), '视频预览入口已变化。')
    if (![...tab.classList].some(name => name.startsWith('active-'))) tab.click()
  }
  const modal = () => unique([...document.querySelectorAll('[role="dialog"]')].filter(e => visible(e) && /[横竖]封面预览（[43]:[34]）/.test(e.innerText)), '封面窗口未就绪，请人工接管。')
  const tile = ratio => {
    if (!['landscape', 'portrait'].includes(ratio)) throw new Error('不支持的封面比例。')
    const text = ratio === 'landscape' ? '横封面4:3' : '竖封面3:4'
    const label = unique(exact(document, text), '封面比例入口已变化，请人工核对。')
    const root = label.closest('[class^="coverControl-"]')
    if (!root) throw new Error('封面预览结构已变化，请人工接管。')
    return root
  }
  try {
    if (location.origin !== 'https://creator.douyin.com' || !navigator.onLine) throw new Error('抖音页面或网络不可用。')
    if (action === 'videoInput') {
      if (location.pathname !== '/creator-micro/content/upload') throw new Error('请进入空白视频上传页；已有网页草稿请先人工处理。')
      if (exact(document, '你还有上次未发布的视频，是否继续编辑？').length || exact(document, '继续编辑').length) throw new Error('抖音保留着上次未发布的视频，请先继续编辑或人工处理，不会重复上传。')
      const input = unique([...document.querySelectorAll('input[type="file"][accept*="video/"]')], '视频上传控件不唯一。')
      if (input.disabled || input.files.length) throw new Error('网页已选择视频，请勿重复上传。')
      return input
    }
    if (location.pathname !== '/creator-micro/content/post/video') throw new Error('尚未进入视频编辑页。')
    const titleInput = unique([...document.querySelectorAll('input[placeholder="填写作品标题，为作品获得更多流量"]')].filter(visible), '标题控件已变化。')
    const descriptionInput = unique([...document.querySelectorAll('[contenteditable="true"][data-slate-editor="true"].editor-comp-publish')].filter(visible), '简介控件已变化。')
    if (action === 'read') {
      if (payload.video) previewVideo()
      const videos = [...document.querySelectorAll('video')].filter(v => !v.closest('[role="dialog"]'))
      return {
        ok: true,
        title: titleInput.value,
        description: clean(descriptionInput.innerText),
        videoPreview: videos.some(v => v.readyState >= 2 && Number.isFinite(v.duration) && v.duration > 0),
        coverDialog: [...document.querySelectorAll('[role="dialog"]')].some(visible),
        covers: ['landscape', 'portrait'].map(ratio => {
          const item = tile(ratio)
          const image = item.querySelector('img')
          const background = item.querySelector('[class^="background-"]')
          const preview = image?.currentSrc || (background ? getComputedStyle(background).backgroundImage : '')
          return { ratio, selected: !item.innerText.includes('选择封面'), preview: preview === 'none' ? '' : preview }
        }),
        videoKey: videos.map(v => v.currentSrc || v.src).filter(Boolean).join('|'),
      }
    }
    if (['collections', 'openCollections', 'selectCollection'].includes(action)) {
      if ([...document.querySelectorAll('[role="dialog"]')].some(visible)) throw new Error('请先处理官方页弹窗。')
      const controls = [...document.querySelectorAll('.semi-select[class*="select-collection-"]')].filter(visible)
      if (controls.length !== 1) return { ok: true, available: false, loaded: false, options: [], selected: null }
      const control = controls[0]
      const selected = control.querySelector('.semi-select-selection-text')?.textContent.trim()
      const name = selected && selected !== '请选择合集' ? selected : null
      if (action === 'openCollections') {
        if (control.getAttribute('aria-expanded') !== 'true' && ![...document.querySelectorAll('[role="listbox"], .semi-select-option-list')].some(visible)) control.click()
        return { ok: true }
      }
      const popups = [...document.querySelectorAll('[role="listbox"], .semi-select-option-list')].filter(visible)
      const popup = popups.length === 1 ? popups[0] : null
      const options = popup ? [...popup.querySelectorAll('[role="option"]')].filter(e => visible(e) && e.getAttribute('aria-disabled') !== 'true').map(e => ({ id: null, name: e.querySelector('[class^="option-title-"]')?.textContent.trim() || e.textContent.trim() })) : []
      const filtered = popup && [...popup.querySelectorAll('input')].some(input => input.value.trim())
      const loaded = Boolean(popup && options.length && !filtered && popup.getAttribute('aria-busy') !== 'true' && !popup.querySelector('[role="progressbar"], .semi-spin-spinning'))
      if (action === 'selectCollection') {
        if (!payload.name) {
          if (name) throw new Error('网页已加入合集，请先在官方页移除后重新核对。')
        } else {
          if (!loaded) throw new Error('合集选项尚未完整显示，请人工核对。')
          const matches = [...popup.querySelectorAll('[role="option"]')].filter(e => visible(e) && e.getAttribute('aria-disabled') !== 'true' && (e.querySelector('[class^="option-title-"]')?.textContent.trim() || e.textContent.trim()) === payload.name)
          unique(matches, '目标合集重名或已失效，请重新选择。').click()
        }
        return { ok: true }
      }
      return { ok: true, available: true, loaded, options, selected: name }
    }
    if (action === 'previewVideo') {
      previewVideo()
      return { ok: true }
    }
    if (action === 'fill') {
      if ([...document.querySelectorAll('[role="dialog"]')].some(visible)) throw new Error('请先检查并关闭封面窗口。')
      if (![...document.querySelectorAll('video')].some(v => v.readyState >= 2 && v.duration > 0)) throw new Error('视频仍在上传或处理中，请稍后检查。')
      const expectedDescription = [payload.description, payload.tags].filter(Boolean).join('\n\n')
      const oldTitle = titleInput.value
      const oldDescription = clean(descriptionInput.innerText)
      if ((oldTitle && oldTitle !== payload.title) || (oldDescription && oldDescription !== clean(expectedDescription))) throw new Error('官方页面已有不同文案，已停止填写，请人工核对。')
      if (oldTitle !== payload.title) {
        Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set.call(titleInput, payload.title)
        titleInput.dispatchEvent(new Event('input', { bubbles: true }))
        titleInput.dispatchEvent(new Event('change', { bubbles: true }))
      }
      if (oldDescription !== clean(expectedDescription)) {
        descriptionInput.focus()
        const range = document.createRange()
        range.selectNodeContents(descriptionInput)
        const selection = window.getSelection()
        selection.removeAllRanges()
        selection.addRange(range)
        if (!document.execCommand('insertText', false, expectedDescription)) throw new Error('简介编辑器未接受内容，请人工填写。')
        descriptionInput.blur()
      }
      if (titleInput.value !== payload.title || clean(descriptionInput.innerText) !== clean(expectedDescription)) throw new Error('文案回读不一致，请人工检查。')
      return { ok: true, title: titleInput.value, description: clean(descriptionInput.innerText) }
    }
    if (action === 'openCover') {
      if ([...document.querySelectorAll('[role="dialog"]')].some(visible)) throw new Error('已打开封面设置，请先人工检查。')
      const item = tile(payload.ratio)
      if (!item.innerText.includes('选择封面')) throw new Error('网页已有封面，请先人工检查，避免覆盖。')
      unique(exact(item, '选择封面'), '选择封面按钮已变化。').click()
      return { ok: true }
    }
    if (action === 'coverInput') {
      const popup = modal()
      const expected = payload.ratio === 'landscape' ? '横封面预览（4:3）' : '竖封面预览（3:4）'
      if (!popup.innerText.includes(expected)) throw new Error('封面设置比例不符，请人工核对。')
      return unique([...popup.querySelectorAll('input[type="file"].semi-upload-hidden-input')]
        .filter(input => input.closest('.semi-upload')?.innerText.includes('点击上传文件') && visible(input.parentElement)), '封面上传控件已变化。')
    }
    if (action === 'coverReady') {
      const popup = modal()
      const label = payload.ratio === 'landscape' ? '横封面预览（4:3）' : '竖封面预览（3:4）'
      if (!popup.innerText.includes(label)) throw new Error('封面设置比例不符，请人工核对。')
      const expected = payload.ratio === 'landscape' ? [1440, 1080] : [1080, 1440]
      const image = [...popup.querySelectorAll('[class^="outer-img-wrap-"] > img')].find(img => img.complete && img.naturalWidth === expected[0] && img.naturalHeight === expected[1])
      const finish = [...popup.querySelectorAll('button')].find(button => visible(button) && button.innerText.trim() === '完成')
      let previewKey = 0
      for (const c of image?.currentSrc ?? '') previewKey = (Math.imul(previewKey, 31) + c.charCodeAt(0)) | 0
      return { ok: true, ready: Boolean(image && finish && !finish.disabled), previewKey, size: image ? [image.naturalWidth, image.naturalHeight] : null }
    }
    if (action === 'switchCover') {
      const popup = modal()
      const label = payload.ratio === 'landscape' ? '设置横封面' : '设置竖封面'
      unique([...popup.querySelectorAll('[class^="step-"]')].filter(e => e.innerText.trim() === label), '封面切换入口已变化。').click()
      return { ok: true }
    }
    if (action === 'confirmCover') {
      const popup = modal()
      const finish = unique([...popup.querySelectorAll('button')].filter(button => visible(button) && button.innerText.trim() === '完成'), '封面确认按钮已变化。')
      if (finish.disabled) throw new Error('封面仍在处理。')
      finish.click()
      return { ok: true }
    }
    throw new Error('不支持的页面操作。')
  } catch (error) {
    return { ok: false, error: error.message }
  }
})
