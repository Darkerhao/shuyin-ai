// Editor controls: retained official DOM (2026-10-26) and its public bundles checked 2026-11-06.
((action, payload = {}) => {
  const visible = element => element.getClientRects().length > 0 && getComputedStyle(element).visibility === 'visible'
  const unique = (elements, message) => {
    if (elements.length !== 1) throw new Error(message)
    return elements[0]
  }
  const exact = (root, text) => [...root.querySelectorAll('*')].filter(e => e.children.length === 0 && e.textContent.trim() === text && visible(e))
  const clean = text => text.replace(/\u200b/g, '').replace(/\r\n/g, '\n').replace(/\n+/g, '\n').trim()
  const setting = (heading, names) => {
    const root = unique(exact(document, heading), `${heading}入口已变化，请人工核对。`).closest('[class^="content-"]')
    if (!root) throw new Error(`${heading}区域无法读取。`)
    const options = names.map(name => unique([...root.querySelectorAll('label[data-checked]')].filter(e => visible(e) && e.textContent.trim() === name), `${heading}选项不唯一。`))
    const selected = unique(options.filter(e => e.dataset.checked === 'true'), `${heading}当前选择无法读取。`)
    return { root, options, selected: options.indexOf(selected) }
  }
  const chooseSetting = (control, index) => {
    const label = control.options[index]
    if (label.querySelector('input')?.disabled || label.closest('[aria-disabled="true"], [class*="disabled-"]')) throw new Error('官方页不允许修改此设置，请人工检查。')
    if (control.selected !== index) label.click()
  }
  const scheduleInput = root => unique([...root.querySelectorAll('[class^="date-picker-"] input[placeholder="日期和时间"]')].filter(visible), '定时日期控件未就绪，请人工设置。')
  const readSetting = read => { try { return read() } catch { return null } }
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
    if (['settings', 'setDownload', 'setPublishMode', 'setSchedule'].includes(action)) {
      if ([...document.querySelectorAll('[role="dialog"]')].some(visible)) throw new Error('请先处理官方页弹窗，再核对发布设置。')
      if (action === 'setDownload') {
        if (typeof payload.allowDownload !== 'boolean') throw new Error('保存权限无效。')
        chooseSetting(setting('保存权限', ['允许', '不允许']), payload.allowDownload ? 0 : 1)
      } else if (action === 'setPublishMode') {
        if (typeof payload.scheduled !== 'boolean') throw new Error('发布时间模式无效。')
        chooseSetting(setting('发布时间', ['立即发布', '定时发布']), payload.scheduled ? 1 : 0)
      } else if (action === 'setSchedule') {
        const control = setting('发布时间', ['立即发布', '定时发布'])
        if (control.selected !== 1 || !/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:00\+08:00$/.test(payload.scheduledAt)) throw new Error('请先选择有效的定时发布。')
        const date = new Date(payload.scheduledAt)
        if (!Number.isFinite(date.getTime())) throw new Error('定时时间无效。')
        // The official DatePicker uses the device timezone; the saved task uses Beijing time.
        const pad = value => String(value).padStart(2, '0')
        const text = `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} ${pad(date.getHours())}:${pad(date.getMinutes())}`
        const input = scheduleInput(control.root)
        if (input.disabled || input.readOnly) throw new Error('官方页定时时间不可编辑，请人工设置。')
        if (input.value !== text) {
          input.focus()
          Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set.call(input, text)
          input.dispatchEvent(new Event('input', { bubbles: true }))
          input.dispatchEvent(new Event('change', { bubbles: true }))
          // The official Semi Input commits through onEnterPress, which listens to keypress.
          input.dispatchEvent(new KeyboardEvent('keypress', { key: 'Enter', code: 'Enter', charCode: 13, bubbles: true }))
          input.blur()
          unique(exact(control.root, '发布时间'), '发布时间标题无法定位。').click()
        }
      } else {
        const download = readSetting(() => setting('保存权限', ['允许', '不允许']))
        const schedule = readSetting(() => setting('发布时间', ['立即发布', '定时发布']))
        const scheduled = schedule ? schedule.selected === 1 : null
        let scheduledAt = null
        if (scheduled) {
          scheduledAt = readSetting(() => {
            const value = scheduleInput(schedule.root).value
            const time = /^\d{4}-\d{2}-\d{2} \d{2}:\d{2}$/.test(value) ? Date.parse(`${value.replace(' ', 'T')}:00`) : NaN
            return Number.isFinite(time) ? `${new Date(time + 8 * 3600000).toISOString().slice(0, 16)}:00+08:00` : ''
          })
        }
        return { ok: true, allowDownload: download ? download.selected === 0 : null, scheduled, scheduledAt }
      }
      return { ok: true }
    }
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
      const collectionName = element => element.querySelector('[class^="selected-item-title-"], [class^="option-title-"]')?.textContent.trim() || element.textContent.trim()
      const selection = control.querySelector('.semi-select-selection-text')
      const selected = selection ? collectionName(selection) : ''
      const name = selected && selected !== '请选择合集' ? selected : null
      if (action === 'openCollections') {
        if (control.getAttribute('aria-expanded') !== 'true' && ![...document.querySelectorAll('[role="listbox"], .semi-select-option-list')].some(visible)) control.click()
        return { ok: true }
      }
      const popups = [...document.querySelectorAll('[role="listbox"], .semi-select-option-list')].filter(visible)
      const popup = popups.length === 1 ? popups[0] : null
      const options = popup ? [...popup.querySelectorAll('[role="option"]')].filter(e => visible(e) && e.getAttribute('aria-disabled') !== 'true').map(e => ({ id: null, name: collectionName(e) })) : []
      const filtered = popup && [...popup.querySelectorAll('input')].some(input => input.value.trim())
      const loaded = Boolean(popup && options.length && !filtered && popup.getAttribute('aria-busy') !== 'true' && !popup.querySelector('[role="progressbar"], .semi-spin-spinning'))
      if (action === 'selectCollection') {
        if (!payload.name) {
          if (name) throw new Error('网页已加入合集，请先在官方页移除后重新核对。')
        } else {
          if (!loaded) throw new Error('合集选项尚未完整显示，请人工核对。')
          const matches = [...popup.querySelectorAll('[role="option"]')].filter(e => visible(e) && e.getAttribute('aria-disabled') !== 'true' && collectionName(e) === payload.name)
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
    if (action === 'prepareText') {
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
      const needsInput = oldDescription !== clean(expectedDescription)
      if (needsInput) {
        descriptionInput.focus()
        const range = document.createRange()
        range.selectNodeContents(descriptionInput)
        const selection = window.getSelection()
        selection.removeAllRanges()
        selection.addRange(range)
      }
      return { ok: true, needsInput, text: clean(expectedDescription) }
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
