export default async function verifyCreatorDelivery(page) {
  const origin = new globalThis.URL(page.url()).origin;
  const assert = (condition, message) => { if (!condition) throw new Error(message); };
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  await page.addInitScript(() => {
    const { window, structuredClone } = globalThis;
    const createdAt = '2026-11-10T01:00:00Z';
    const book = { id: 'delivery-book', title: '文城', author: '余华', status: 'draft', publisher: '', isbn: '', description: '', sourceNotes: '本期分享寻找与守候。', theme: '', targetAudience: '', contentStyle: '', targetDurationSeconds: 600, audioDurationSource: 'manual', workspacePath: 'D:/视觉验收/文城', createdAt, updatedAt: createdAt, archivedAt: null };
    const content = { format: 'publishing', title: '一个人，为什么愿意等一生', description: '一个遥远的地名，让林祥福踏上了寻找的旅途。\n\n这一期，我们一起读余华的《文城》，从寻找、承诺与守候出发，聊聊普通人如何面对命运，也聊聊那些让我们愿意停下脚步的人。\n\n如果是你，会如何理解一个人的等待？', tags: '#读书 #文城 #余华 #好书分享', audioPrompt: '围绕寻找与守候，制作十分钟的读书分享。', coverTitle: '在风雨中\n寻找一个人', coverSubtitle: '余华《文城》：命运与温情', coverPromptLandscape: '横版 4:3，雨中的江南小镇，中间留白。', coverPromptPortrait: '竖版 3:4，一个人在雨中行走。', coverPromptWidescreen: '横版 16:9，远山与小镇。' };
    const version = { id: 'writing1', projectId: book.id, kind: 'publishing', versionNo: 1, content, sourceType: 'ai', modelName: '创作模型', promptVersion: 'publishing-v7', isConfirmed: false, createdAt };
    const assets = ['audio', 'video'].map((assetType, index) => ({ id: `${assetType}1`, projectId: book.id, assetType, originalName: index ? '文城-最终成片.mp4' : '文城-读书原声.m4a', filePath: index ? 'D:/视觉验收/文城-最终成片.mp4' : 'D:/视觉验收/文城-读书原声.m4a', fileSize: 1024000, modifiedAt: 1, existsStatus: 'available', createdAt }));
    const config = { schemaVersion: 1, templateId: 'night', width: 1440, height: 1080, title: content.coverTitle, subtitle: content.coverSubtitle, brand: '读书分享', fontFamily: 'sans', titleSize: 148, subtitleSize: 52, align: 'center', position: 'center', backgroundColor: '#183c32', textColor: '#fff8e8', subtitleColor: '#e6d29f', accentColor: '#d6bc7f', backgroundAssetId: null, overlayOpacity: .65, backgroundX: .5, backgroundY: .5, backgroundZoom: 1, backgroundPrompt: '' };
    const covers = [[1440, 1080], [1080, 1440]].map(([width, height], index) => ({ id: `cover${index + 1}`, projectId: book.id, versionNo: index + 1, config: { ...config, width, height }, exportedPath: `D:/视觉验收/封面${index}.png`, createdAt }));
    const task = { id: 'publish1', projectId: book.id, writingVersionId: version.id, title: content.title, description: content.description, tags: content.tags, audioAssetId: 'audio1', videoAssetId: 'video1', landscapeCoverId: 'cover1', portraitCoverId: 'cover2', titleConfirmed: true, coversConfirmed: true, allowDownload: true, scheduledAt: null, status: 'ready', taskDirectory: 'D:/视觉验收/任务包', createdAt, updatedAt: createdAt, checklist: [{ key: 'video', label: '成片文件与音轨已校验', passed: true }, { key: 'covers', label: '双封面已核对', passed: true }], record: null };
    window.__delivery = { book, versions: [version], assets, covers, tasks: [task], calls: [], unknown: [], drafts: {}, session: null };
    let callback = 0;
    window.isTauri = true;
    window.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener() {} };
    window.__TAURI_INTERNALS__ = {
      metadata: { currentWindow: { label: 'main' }, currentWebview: { label: 'main' } },
      transformCallback: () => ++callback, unregisterCallback() {},
      convertFileSrc: () => '/src/assets/voice/publish-ready.wav',
      invoke: async (command, args = {}) => {
        const fixture = window.__delivery;
        fixture.calls.push({ command, args });
        if (command === 'plugin:app|version') return '0.57.1';
        if (command === 'plugin:event|listen') return callback;
        if (command === 'plugin:event|unlisten') return;
        if (command === 'update_settings') return false;
        if (command === 'interface_preferences') return { sidebarPrompt: false };
        if (command === 'workspace_settings') return { root: 'D:/视觉验收', databasePath: 'D:/视觉验收/test.sqlite3' };
        if (command === 'workflow_snapshot') return { resume: [], running: [] };
        if (command === 'automation_snapshot' || command === 'list_video_jobs' || command === 'list_publish_queue') return [];
        if (command === 'project_progress') return ['', 'writing', 'covers', 'video', 'publishing'].map(path => ({ path, status: '已准备', tone: 'ready' }));
        if (command === 'get_project') return structuredClone(book);
        if (command === 'list_projects') return [structuredClone(book)];
        if (command === 'list_assets') return structuredClone(fixture.assets);
        if (command === 'list_model_providers') return [{ id: 'model1', modelKind: 'text', name: '创作模型', baseUrl: '', modelName: 'model', isDefault: true, isEnabled: true, credentialStatus: 'available' }];
        if (command === 'list_writing_versions') return structuredClone(fixture.versions);
        if (command === 'list_cover_versions') return structuredClone(fixture.covers);
        if (command === 'list_publish_tasks') return structuredClone(fixture.tasks);
        if (command === 'video_asset_metadata') return { width: 1920, height: 1080, durationSeconds: 625, hasAudio: true };
        if (command === 'audio_job_snapshot') return { job: null, activeJob: null, browser: { supported: true, open: false, url: null, message: null } };
        if (command === 'media_asset_path') return 'D:/视觉验收/原声.m4a';
        if (command === 'douyin_session') return structuredClone(fixture.session);
        if (command === 'read_editor_draft') return fixture.drafts[`${args.projectId}:${args.kind}`] ?? null;
        if (command === 'save_editor_draft') { fixture.drafts[`${args.projectId}:${args.kind}`] = args.draft; return; }
        if (command === 'open_asset' || command === 'open_douyin_window') return;
        if (command === 'read_cover_export') {
          const { coverPng } = await import('/src/services/coverRenderer.ts');
          return new Uint8Array(await coverPng(covers.find(cover => cover.id === args.id).config, null)).buffer;
        }
        fixture.unknown.push(command);
        throw new Error(`Unhandled delivery fixture: ${command}`);
      },
    };
  });
  await page.goto(`${origin}/#/projects/delivery-book/writing`);
  await page.reload();
  await page.locator('#result-title').waitFor();
  await page.locator('#result-title').fill('我的当前草稿，保持原样');
  const layouts = [];
  const checkLayout = async (name, screenshot = true) => {
    for (const theme of ['dark', 'light']) {
      await page.evaluate(value => globalThis.document.documentElement.classList.toggle('dark', value === 'dark'), theme);
      for (const width of [1440, 1120, 800, 390]) {
        await page.setViewportSize({ width, height: 1000 });
        assert(await page.evaluate(() => globalThis.document.documentElement.scrollWidth <= globalThis.innerWidth), `${name} overflow ${theme}/${width}`);
        if (name === 'cover-fields') {
          for (const field of ['coverTitle', 'coverSubtitle']) {
            const spacing = await page.locator(`[data-field="${field}"]`).evaluate(node => {
              const input = node.querySelector('textarea');
              const count = node.querySelector('.character-count');
              return { padding: parseFloat(globalThis.getComputedStyle(input).paddingRight), required: input.getBoundingClientRect().right - count.getBoundingClientRect().left };
            });
            assert(spacing.padding >= spacing.required, `${field} text overlaps character count ${theme}/${width}: ${JSON.stringify(spacing)}`);
          }
        }
        layouts.push(`${name}/${theme}/${width}`);
      }
      await page.setViewportSize({ width: 1440, height: 1000 });
      if (screenshot) await page.screenshot({ path: `output/playwright/delivery-${name}-${theme}.png`, fullPage: true });
    }
  };
  await checkLayout('editor');
  await page.getByRole('tab', { name: '封面素材', exact: true }).click();
  await page.locator('#result-coverTitle').fill('封面主标题需要完整显示且不能被计数遮挡');
  await page.locator('#result-coverSubtitle').fill('封面副标题也应当完整显示且不能被计数遮挡');
  await checkLayout('cover-fields');
  await page.getByRole('tab', { name: '发布文案', exact: true }).click();
  await page.evaluate(async () => {
    const { generationJobs, generationKey } = await import('/src/services/generation.ts');
    generationJobs.set(generationKey('delivery-book', 'writing'), { projectId: 'delivery-book', title: '文城', kind: 'writing', mode: 'writing', id: 'generation1', startedAt: Date.now(), status: 'running', cancelling: false, preview: {}, error: '' });
  });
  await page.getByText('等待返回', { exact: true }).waitFor();
  await page.evaluate(async () => {
    const { generationJobs, generationKey } = await import('/src/services/generation.ts');
    const key = generationKey('delivery-book', 'writing');
    generationJobs.set(key, { ...generationJobs.get(key), preview: { title: '候选：寻找的意义', description: '这些文字来自当前任务的返回片段。\n当前稿保留，等待完整结果校验。' } });
  });
  await page.getByText('正在接收', { exact: true }).waitFor();
  await checkLayout('candidate');
  assert(await page.locator('#result-title').inputValue() === '我的当前草稿，保持原样', 'Streaming overwrote editor');
  await page.evaluate(async () => {
    const { generationJobs, generationKey } = await import('/src/services/generation.ts');
    const fixture = globalThis.window.__delivery;
    const result = { ...fixture.versions[0], id: 'writing2', versionNo: 2, content: { ...fixture.versions[0].content, title: '候选：寻找的意义' } };
    fixture.versions.unshift(result);
    const key = generationKey('delivery-book', 'writing');
    generationJobs.set(key, { ...generationJobs.get(key), status: 'succeeded', result });
  });
  await page.getByText('生成完成', { exact: true }).waitFor();
  await page.getByRole('button', { name: '查看对照并取用', exact: true }).click();
  await page.locator('#history-candidate-title').waitFor();
  assert(await page.locator('#result-title').inputValue() === '我的当前草稿，保持原样', 'Viewing history overwrote editor');
  assert(await page.locator('#history-candidate-title').inputValue() === '候选：寻找的意义', 'History opened wrong version');
  await checkLayout('history');
  await page.getByRole('button', { name: '取用发布标题', exact: true }).click();
  assert(await page.locator('#result-title').inputValue() === '候选：寻找的意义', 'Explicit field adoption failed');
  await page.getByRole('button', { name: '按项调整', exact: true }).click();
  await page.evaluate(async () => {
    const { generationJobs, generationKey } = await import('/src/services/generation.ts');
    const content = globalThis.window.__delivery.versions[0].content;
    generationJobs.set(generationKey('delivery-book', 'writing'), { projectId: 'delivery-book', title: '文城', kind: 'writing', mode: 'revision', id: 'revision1', startedAt: Date.now(), status: 'succeeded', cancelling: false, preview: {}, error: '', result: { taskId: 'revision1', fields: ['title', 'description'], before: content, content: { ...content, title: '调整候选，待采用' } } });
  });
  await page.locator('#revision-after-title').waitFor();
  await checkLayout('revision');
  await page.getByRole('button', { name: '放弃候选结果', exact: true }).click();
  assert(await page.locator('#result-title').inputValue() === '候选：寻找的意义', 'Discarding revision changed editor');
  await page.goto(`${origin}/#/projects/delivery-book/publishing?task=publish1`);
  await page.locator('#publish-title').waitFor();
  await page.locator('.publish-cover-gallery img').first().waitFor();
  await checkLayout('publishing');
  await page.getByRole('button', { name: '预览成片', exact: true }).click();
  assert(await page.evaluate(() => globalThis.window.__delivery.calls.some(call => call.command === 'open_asset' && call.args.id === 'video1')), 'Preview did not use selected video');
  const coverButton = page.getByRole('button', { name: '放大查看封面 v1', exact: true });
  await coverButton.click();
  await page.getByRole('dialog', { name: '封面 v1 放大预览', exact: true }).waitFor();
  await page.keyboard.press('Escape');
  await page.getByRole('dialog').waitFor({ state: 'detached' });
  assert(await coverButton.evaluate(node => node === globalThis.document.activeElement), 'Cover preview did not restore focus');
  await page.evaluate(() => {
    const fixture = globalThis.window.__delivery;
    fixture.session = { runId: 1, taskId: 'publish1', status: 'review', step: 'review', message: '准备完成，请亲自检查并发布。', waitingFor: null, account: { key: 'demo', id: null, name: '本地验收账号' }, completed: ['connect', 'video', 'text', 'covers', 'collection', 'settings'], manual: [], collection: { name: null }, options: [] };
  });
  await page.goto(`${origin}/#/projects/delivery-book/writing`);
  await page.locator('#result-title').waitFor();
  await page.goto(`${origin}/#/projects/delivery-book/publishing?task=publish1`);
  await page.getByRole('button', { name: '去官方页面检查', exact: true }).waitFor();
  assert(await page.locator('.douyin-steps [data-state="done"]').count() === 5, 'Preparation phases incorrectly counted');
  assert(await page.locator('.douyin-steps [aria-current="step"]').innerText().then(text => text.includes('等待你检查并发布')), 'Final phase falsely complete');
  await page.getByRole('button', { name: '去官方页面检查', exact: true }).click();
  await checkLayout('review');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  assert(await page.locator('.douyin-workbench').evaluate(node => globalThis.getComputedStyle(node).animationName === 'none'), 'Reduced-motion animation remains');
  assert(await page.evaluate(() => !globalThis.window.__delivery.calls.some(call => call.command === 'record_publish_success')), 'Preview marked task published');
  await page.evaluate(() => {
    const fixture = globalThis.window.__delivery;
    fixture.assets = fixture.assets.map(asset => ({ ...asset, existsStatus: 'missing' }));
    fixture.covers = [];
    fixture.tasks[0].status = 'draft';
    fixture.tasks[0].checklist = [];
    fixture.session = null;
  });
  await page.getByRole('button', { name: '刷新素材与状态', exact: true }).click();
  await page.getByText('待选择横版封面', { exact: true }).waitFor();
  assert(await page.getByRole('button', { name: '确认并开始辅助发布', exact: true }).isDisabled(), 'Missing materials allowed publishing');
  await checkLayout('missing', false);
  assert(!errors.length, `Browser errors: ${errors.join('; ')}`);
  const unknown = await page.evaluate(() => globalThis.window.__delivery.unknown);
  assert(!unknown.length, `Unknown IPC: ${unknown.join(', ')}`);
  return { layouts: layouts.length, errors, checks: ['current draft preserved', 'cover title count clearance', 'waiting/streaming/completed candidate', 'explicit history adoption', 'revision discard', 'selected media preview', 'cover modal focus', 'five verified phases and final manual review', 'reduced motion', 'missing material blocking'], screenshots: 'output/playwright/delivery-*.png' };
}
