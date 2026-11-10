export default async function verifyVisualUpgrade(page) {
  const origin = new globalThis.URL(page.url()).origin;
  const assert = (condition, message) => { if (!condition) throw new Error(message); };
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  page.on('console', message => { if (message.type() === 'warning' && /Duplicate keys|TransitionGroup/.test(message.text())) errors.push(message.text()); });
  await page.addInitScript(() => {
    const { window, structuredClone } = globalThis;
    const createdAt = '2026-11-10T01:00:00Z';
    const books = ['文城', '明朝那些事儿', '生死疲劳', '漫长的余生'].map((title, index) => ({
      id: `visual-${index}`, title, author: ['余华', '当年明月', '莫言', '罗新'][index], status: index === 2 ? 'published' : 'draft',
      publisher: '', isbn: '', description: '', sourceNotes: '', theme: '', targetAudience: '', contentStyle: '',
      targetDurationSeconds: 600, audioDurationSource: 'manual', workspacePath: 'D:/视觉验收/书籍', createdAt, updatedAt: createdAt, archivedAt: null,
    }));
    const config = { schemaVersion: 1, templateId: 'night', width: 1440, height: 1080, title: '在风雨中\n寻找一个人', subtitle: '余华《文城》：命运与温情', brand: '书引 AI · 读书分享', fontFamily: 'sans', titleSize: 148, subtitleSize: 52, align: 'center', position: 'center', backgroundColor: '#183c32', textColor: '#fff8e8', subtitleColor: '#e6d29f', accentColor: '#d6bc7f', backgroundAssetId: null, overlayOpacity: .65, backgroundX: .5, backgroundY: .5, backgroundZoom: 1, backgroundPrompt: '' };
    const covers = [[1440, 1080], [1080, 1440], [1920, 1080]].map(([width, height], index) => ({ id: `cover-${index}`, projectId: books[0].id, versionNo: 3 - index, config: { ...config, width, height }, exportedPath: `D:/视觉验收/封面-${index}.png`, createdAt }));
    const run = { id: 'run1', projectId: books[0].id, input: { workflowVersion: 2, title: '文城', author: '余华', targetDurationSeconds: 600, durationReason: '', selection: {} }, status: 'running', stage: 'portrait_image', runId: 1, error: '', audioJobId: 'audio1', videoJobId: 'video1', writingTaskId: 'text1', writingVersionId: 'version1', finalWritingVersionId: 'version2', actualDurationSeconds: 625, landscapeTaskId: 'image1', portraitTaskId: null, widescreenTaskId: null, landscapeCoverId: 'cover-0', portraitCoverId: null, widescreenCoverId: null, publishTaskId: null, createdAt, updatedAt: createdAt };
    window.__visualFixture = { books, covers, run, showRun: false, calls: [], unknown: [], draft: null };
    let callback = 0;
    window.isTauri = true;
    window.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener() {} };
    window.__TAURI_INTERNALS__ = {
      metadata: { currentWindow: { label: 'main' }, currentWebview: { label: 'main' } },
      transformCallback: () => ++callback, unregisterCallback() {},
      invoke: async (command, args = {}) => {
        const fixture = window.__visualFixture;
        fixture.calls.push({ command, args });
        if (command === 'plugin:app|version') return '0.57.0';
        if (command === 'plugin:event|listen') return callback;
        if (command === 'plugin:event|unlisten') return;
        if (command === 'update_settings') return false;
        if (command === 'interface_preferences') return { sidebarPrompt: false };
        if (command === 'workspace_settings') return { root: 'D:/视觉验收', databasePath: 'D:/视觉验收/test.sqlite3' };
        if (command === 'workflow_snapshot') return { resume: [], running: [] };
        if (command === 'automation_snapshot') return fixture.showRun ? [fixture.run] : [];
        if (command === 'project_progress') return ['', 'writing', 'covers', 'video', 'publishing'].map(path => ({ path, status: path === 'covers' ? '已导出 3/3' : '未开始', tone: path === 'covers' ? 'ready' : 'neutral' }));
        if (command === 'list_projects') return structuredClone(books);
        if (command === 'get_project') return books.find(book => book.id === args.id);
        if (['list_assets', 'list_providers', 'list_writing_versions', 'list_cover_presets'].includes(command)) return [];
        if (command === 'list_cover_versions') return structuredClone(covers);
        if (command === 'read_editor_draft') return fixture.draft;
        if (command === 'save_editor_draft') { fixture.draft = args.draft; return; }
        if (command === 'read_cover_export') {
          const { coverPng } = await import('/src/services/coverRenderer.ts');
          return new Uint8Array(await coverPng(covers.find(cover => cover.id === args.id).config, null)).buffer;
        }
        fixture.unknown.push(command);
        throw new Error(`Unhandled visual fixture: ${command}`);
      },
    };
  });
  await page.goto(`${origin}/#/projects`);
  await page.reload();
  await page.getByRole('heading', { name: '我的书籍', exact: false }).waitFor();
  await page.locator('.book-jacket').first().waitFor();
  const layouts = [];
  for (const theme of ['dark', 'light']) {
    await page.evaluate(value => globalThis.document.documentElement.classList.toggle('dark', value === 'dark'), theme);
    for (const width of [1440, 1120, 800, 390]) {
      await page.setViewportSize({ width, height: 1000 });
      const fits = await page.evaluate(() => globalThis.document.documentElement.scrollWidth <= globalThis.innerWidth);
      assert(fits, `Shelf overflow at ${theme}/${width}`);
      layouts.push(`shelf ${theme} ${width}`);
    }
    await page.setViewportSize({ width: 1440, height: 1000 });
    await page.screenshot({ path: `output/playwright/visual-shelf-${theme}.png`, fullPage: true });
  }
  const first = page.locator('.project-card').first();
  await first.hover({ position: { x: 40, y: 90 } });
  assert(await first.evaluate(node => !!node.style.getPropertyValue('--book-rotate-y')), 'Book tilt did not respond');
  await page.mouse.move(0, 0);
  assert(await first.evaluate(node => !node.style.getPropertyValue('--book-rotate-y')), 'Book tilt did not reset');
  await page.getByRole('searchbox').fill('生死疲劳');
  await page.waitForFunction(() => globalThis.document.querySelectorAll('.project-card').length === 1);
  assert(await page.locator('.project-card').count() === 1, 'Shelf filtering changed');
  await page.getByRole('searchbox').fill('');
  await page.waitForFunction(() => globalThis.document.querySelectorAll('.project-card').length === 4);
  const expectedOrder = await page.locator('.project-card-title').allTextContents();
  expectedOrder.sort((left, right) => left.localeCompare(right, 'zh-CN'));
  await page.getByRole('combobox', { name: '书籍排序' }).click();
  await page.getByRole('option', { name: '书名排序', exact: true }).click();
  await page.waitForFunction(expected => JSON.stringify([...globalThis.document.querySelectorAll('.project-card-title')].map(node => node.textContent.trim())) === JSON.stringify(expected.map(title => title.trim())), expectedOrder);
  await page.getByRole('button', { name: '列表视图', exact: true }).click();
  await page.locator('.library-grid.is-list').waitFor();
  assert(await page.locator('.library-grid').evaluate(node => node.classList.contains('is-list')), 'List view unavailable');
  await page.getByRole('button', { name: '网格视图', exact: true }).click();
  const more = page.getByRole('button', { name: '更多操作：文城', exact: true });
  await more.click();
  await page.getByRole('menu').waitFor();
  await page.keyboard.press('Escape');
  await page.getByRole('menu').waitFor({ state: 'detached' });
  assert(await more.evaluate(node => node === globalThis.document.activeElement), 'Book menu focus failed after sorting');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await first.hover();
  assert(await first.locator('.book-jacket').evaluate(node => globalThis.getComputedStyle(node).transform === 'none'), 'Reduced motion tilt remains');
  await page.emulateMedia({ reducedMotion: 'no-preference' });
  await page.evaluate(() => { globalThis.window.__visualFixture.showRun = true; });
  await page.locator('.automation-run').waitFor();
  const originalOutput = page.locator('.automation-output').filter({ hasText: '本次创作方案' });
  const checkedOutput = page.locator('.automation-output').filter({ hasText: '已校核发布材料' });
  await checkedOutput.evaluate(node => { globalThis.window.__checkedOutputNode = node; });
  await page.evaluate(() => { globalThis.window.__visualFixture.run.finalWritingVersionId = 'version1'; });
  await page.waitForFunction(() => [...globalThis.document.querySelectorAll('.automation-output')].find(node => node.textContent.includes('已校核发布材料'))?.getAttribute('href').endsWith('writingVersion=version1'));
  assert(await originalOutput.getAttribute('href') === await checkedOutput.getAttribute('href'), 'Reused writing version lost an output');
  assert(await checkedOutput.evaluate(node => node === globalThis.window.__checkedOutputNode), 'Checked output identity changed during update');
  await page.evaluate(() => { globalThis.window.__visualFixture.run.status = 'paused'; });
  await page.locator('.automation-run[data-status="paused"]').waitFor();
  assert((await page.locator('.automation-phases [aria-current="step"]').innerText()).includes('已暂停'), 'Paused task reported as running');
  await page.evaluate(() => { globalThis.window.__visualFixture.run.status = 'running'; });
  await page.locator('.automation-run[data-status="running"]').waitFor();
  for (const theme of ['dark', 'light']) {
    await page.evaluate(value => globalThis.document.documentElement.classList.toggle('dark', value === 'dark'), theme);
    for (const width of [1440, 800, 390]) {
      await page.setViewportSize({ width, height: 1000 });
      assert(await page.evaluate(() => globalThis.document.documentElement.scrollWidth <= globalThis.innerWidth), `Task overflow ${theme}/${width}`);
      layouts.push(`task ${theme} ${width}`);
    }
    await page.setViewportSize({ width: 1440, height: 1000 });
    await page.locator('.automation-studio').screenshot({ path: `output/playwright/visual-tasks-${theme}.png` });
  }
  await page.evaluate(() => { const fixture = globalThis.window.__visualFixture; fixture.run.status = 'awaiting_publish'; fixture.run.stage = 'publishing'; fixture.run.publishTaskId = 'publish1'; });
  await page.getByText('等待最终发布', { exact: true }).waitFor();
  assert((await page.locator('.automation-phases [aria-current="step"]').innerText()).includes('待确认'), 'Final publishing incorrectly complete');
  await page.evaluate(() => { globalThis.window.__visualFixture.showRun = false; });
  await page.locator('.automation-run').waitFor({ state: 'detached' });
  for (const [status, expected] of [['running', '生成中，可继续其他创作'], ['failed', '未完成，查看原因'], ['succeeded', '已完成，查看结果'], ['cancelled', null]]) {
    await page.evaluate(async status => {
      const { generationJobs, generationKey } = await import('/src/services/generation.ts');
      generationJobs.set(generationKey('visual-0', 'writing'), { projectId: 'visual-0', title: '文城', kind: 'writing', mode: 'writing', id: 'manual-writing', startedAt: Date.now(), status, cancelling: false, preview: {}, error: status === 'failed' ? '测试错误' : '' });
    }, status);
    if (expected) {
      await page.locator('.running-tasks').getByText(expected, { exact: true }).waitFor();
      await page.setViewportSize({ width: 390, height: 900 });
      assert(await page.evaluate(() => globalThis.document.documentElement.scrollWidth <= globalThis.innerWidth), `Background task overflow ${status}`);
    } else await page.locator('.running-tasks').waitFor({ state: 'detached' });
  }
  await page.setViewportSize({ width: 1440, height: 1000 });
  await page.goto(`${origin}/#/projects/visual-0/covers?tab=text`);
  await page.getByRole('heading', { name: '封面展示台', exact: true }).waitFor();
  assert(await page.evaluate(() => !globalThis.window.__visualFixture.calls.some(call => call.command === 'read_cover_export')), 'Closed history eagerly read exports');
  for (const theme of ['dark', 'light']) {
    await page.evaluate(value => globalThis.document.documentElement.classList.toggle('dark', value === 'dark'), theme);
    for (const width of [1440, 1120, 800, 390]) {
      await page.setViewportSize({ width, height: 1000 });
      assert(await page.evaluate(() => globalThis.document.documentElement.scrollWidth <= globalThis.innerWidth), `Cover overflow ${theme}/${width}`);
      layouts.push(`covers ${theme} ${width}`);
      if (width < 1024) {
        await page.getByRole('button', { name: '查看实时预览', exact: true }).click();
        await page.locator('#cover-preview').waitFor({ state: 'visible' });
        assert(await page.evaluate(() => globalThis.document.documentElement.scrollWidth <= globalThis.innerWidth), `Visible cover preview overflow ${theme}/${width}`);
        for (const thumbnail of await page.locator('.size-preview canvas').all()) {
          const dimensions = await thumbnail.evaluate(node => { const rect = node.getBoundingClientRect(); return { width: rect.width, height: rect.height, ratio: node.width / node.height }; });
          assert(dimensions.width > 0 && dimensions.height > 0 && Math.abs(dimensions.width / dimensions.height - dimensions.ratio) < .02, 'Thumbnail is hidden or distorted');
        }
        layouts.push(`cover preview ${theme} ${width}`);
        await page.getByRole('button', { name: '返回编辑', exact: true }).click();
      }
    }
    await page.setViewportSize({ width: 1440, height: 1100 });
    await page.screenshot({ path: `output/playwright/visual-covers-${theme}.png`, fullPage: true });
  }
  const title = page.locator('#cover-title');
  await title.fill('保留我的未保存修改');
  await page.getByRole('button', { name: '编辑竖版 3:4 · 1080 × 1440', exact: true }).click();
  assert(await title.inputValue() !== '保留我的未保存修改', 'Cover sizes lost independence');
  await page.getByRole('button', { name: '编辑横版 4:3 · 1440 × 1080', exact: true }).click();
  assert(await title.inputValue() === '保留我的未保存修改', 'Size switch lost draft');
  await page.getByRole('button', { name: /封面历史/ }).click();
  const enlarge = page.getByRole('button', { name: '放大查看封面 v3', exact: true });
  await enlarge.click();
  await page.getByRole('dialog', { name: '封面 v3 放大预览', exact: true }).waitFor();
  await page.keyboard.press('Control+1');
  assert(page.url().includes('/covers'), 'Preview did not block the global navigation shortcut');
  await page.keyboard.press('Control+s');
  await page.keyboard.press('Meta+s');
  assert(await title.inputValue() === '保留我的未保存修改', 'History preview overwrote draft');
  await page.screenshot({ path: 'output/playwright/visual-cover-preview.png' });
  await page.keyboard.press('Escape');
  await page.getByRole('dialog').waitFor({ state: 'detached' });
  assert(await enlarge.evaluate(node => node === globalThis.document.activeElement), 'Preview failed to restore focus');
  await page.setViewportSize({ width: 390, height: 800 });
  await enlarge.focus();
  await page.keyboard.press('Enter');
  const preview = page.getByRole('dialog', { name: '封面 v3 放大预览', exact: true });
  await preview.waitFor();
  const bounds = await preview.boundingBox();
  assert(bounds.x >= 0 && bounds.x + bounds.width <= 390 && bounds.y >= 0 && bounds.y + bounds.height <= 800, 'Narrow preview overflow');
  await page.keyboard.press('Escape');
  const diagnostics = await page.evaluate(() => ({ unknown: globalThis.window.__visualFixture.unknown, writes: globalThis.window.__visualFixture.calls.filter(call => /^(save_cover_version|start_|control_automation)/.test(call.command)) }));
  assert(diagnostics.unknown.length === 0, `Unhandled IPC: ${diagnostics.unknown.join(', ')}`);
  assert(diagnostics.writes.length === 0, 'Visual interactions triggered business writes');
  assert(errors.length === 0, errors.join('\n'));
  return { layouts, checks: ['book tilt and reset', 'search, real title sorting, menu focus and view switch', 'reduced motion', 'shared writing version stable output', 'paused task and manual publishing boundary', 'background generation running/failed/succeeded/cancelled states', 'visible narrow preview and true thumbnail ratios', 'lazy history reads', 'independent size drafts', 'history preview preserves edits, blocks save shortcuts and returns focus'], errors, diagnostics };
}
