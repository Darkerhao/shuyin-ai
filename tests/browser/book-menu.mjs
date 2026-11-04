// Import with Playwright CLI run-code after opening the local Vite server.
// Only Tauri IPC is simulated; the Vue components, Reka menu and CSS are real.
export default async function verifyBookMenu(page) {
  const origin = new globalThis.URL(page.url()).origin;
  const assert = (condition, message) => { if (!condition) throw new Error(message); };
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  await page.addInitScript(() => {
    const now = '2026-11-04T00:00:00Z';
    const books = ['文城', '明朝那些事儿', '生死疲劳', '一本标题很长的书：关于阅读与创作的思考'].map((title, index) => ({
      id: 'menu-' + index, title, author: ['余华', '当年明月', '莫言', ''][index], status: 'draft',
      publisher: '', isbn: '', description: '', sourceNotes: '', theme: '', targetAudience: '',
      contentStyle: '', targetDurationSeconds: 600, audioDurationSource: 'manual',
      workspacePath: 'D:/界面回归/书籍', createdAt: now, updatedAt: now, archivedAt: null,
    }));
    let callbackId = 0;
    globalThis.window.isTauri = true;
    globalThis.window.__bookMenuCalls = [];
    globalThis.window.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener: () => {} };
    globalThis.window.__TAURI_INTERNALS__ = {
      metadata: { currentWindow: { label: 'main' }, currentWebview: { label: 'main' } },
      transformCallback: () => ++callbackId, unregisterCallback: () => {},
      invoke: async (command, args = {}) => {
        globalThis.window.__bookMenuCalls.push({ command, args });
        if (command === 'plugin:app|version') return '0.45.0';
        if (command === 'update_settings') return false;
        if (command === 'plugin:event|listen') return callbackId;
        if (command === 'plugin:event|unlisten') return;
        if (command === 'interface_preferences') return { sidebarPrompt: false };
        if (command === 'workspace_settings') return { root: 'D:/界面回归', databasePath: 'D:/界面回归/test.sqlite3' };
        if (command === 'workflow_snapshot') return { resume: [], running: [] };
        if (command === 'list_projects') return globalThis.structuredClone(books);
        if (command === 'list_assets') return args.projectId === 'menu-0' ? [{ id: 'video-0', projectId: args.projectId, assetType: 'video', existsStatus: 'available', filePath: 'D:/界面回归/成片.mp4', originalName: '成片.mp4', fileSize: 100, modifiedAt: 1, createdAt: now }] : [];
        if (command === 'open_project_directory') return;
        throw new Error('Unhandled menu fixture command: ' + command);
      },
    };
  });
  const results = [];
  for (const [width, height] of [[1440, 900], [1366, 768], [1120, 900], [800, 1000], [360, 900]]) {
    await page.setViewportSize({ width, height });
    await page.goto(origin + '/#/projects');
    await page.reload();
    const more = page.getByRole('button', { name: '更多操作：文城', exact: true });
    await more.click();
    const menu = page.getByRole('menu', { name: '更多操作：文城', exact: true });
    await menu.waitFor();
    await page.getByRole('menuitem', { name: '导出成片', exact: true }).waitFor();
    // Floating UI may place the content one frame after it is mounted.
    await page.waitForFunction(() => {
      const node = globalThis.document.querySelector('[role="menu"]');
      return node && globalThis.getComputedStyle(node).visibility !== 'hidden';
    });
    const visual = await menu.evaluate(node => {
      const style = globalThis.getComputedStyle(node);
      const box = node.getBoundingClientRect();
      const trigger = globalThis.document.querySelector('button[aria-label="更多操作：文城"]').getBoundingClientRect();
      return { background: style.backgroundColor, border: style.borderTopWidth, radius: style.borderRadius, shadow: style.boxShadow, padding: style.padding, side: node.getAttribute('data-side'), box: box.toJSON(), trigger: trigger.toJSON(), scrollWidth: node.scrollWidth, clientWidth: node.clientWidth };
    });
    assert(visual.background === 'rgb(255, 255, 255)', `Transparent menu at ${width}: ${JSON.stringify(visual)}`);
    assert(visual.border !== '0px' && visual.radius !== '0px' && visual.shadow !== 'none' && visual.padding !== '0px', `Missing menu surface at ${width}`);
    assert(visual.box.left >= 11 && visual.box.right <= width - 11 && visual.box.top >= 11 && visual.box.bottom <= height - 11, `Menu clipped at ${width}`);
    assert(Math.abs(visual.box.right - visual.trigger.right) < 2, `More menu lost right alignment at ${width}`);
    const roomBelow = height - 12 - visual.trigger.bottom >= visual.box.height + 6;
    assert(roomBelow ? visual.side === 'bottom' && Math.abs(visual.box.top - visual.trigger.bottom - 6) < 2 : visual.side === 'top' && Math.abs(visual.trigger.top - visual.box.bottom - 6) < 2, `More menu lost its anchor or collision handling at ${width}: ${JSON.stringify(visual)}`);
    assert(visual.scrollWidth <= visual.clientWidth, `Menu contents overflow at ${width}`);
    await page.keyboard.press('Escape');
    assert(await more.evaluate(node => node === globalThis.document.activeElement), `Escape did not restore More focus at ${width}`);
    results.push({ width, height, ...visual });
  }
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto(origin + '/#/projects');
  const cards = page.locator('.project-card');
  const first = cards.first();
  const link = first.locator('a');
  const more = first.getByRole('button');
  await link.focus();
  await page.keyboard.press('Shift+F10');
  await page.getByRole('menu').waitFor();
  await page.keyboard.press('Home');
  assert(await page.getByRole('menuitem', { name: '重命名', exact: true }).evaluate(node => node === globalThis.document.activeElement), 'Home did not focus Rename');
  await page.keyboard.press('ArrowDown');
  assert(await page.getByRole('menuitem', { name: '在文件管理器中显示目录', exact: true }).evaluate(node => node === globalThis.document.activeElement), 'ArrowDown did not focus Directory');
  await page.keyboard.press('Escape');
  assert(await link.evaluate(node => node === globalThis.document.activeElement), 'Shift+F10 lost its original focus');
  await link.focus();
  await page.keyboard.press('ContextMenu');
  await page.getByRole('menu').waitFor();
  await page.keyboard.press('Escape');
  assert(await link.evaluate(node => node === globalThis.document.activeElement), 'ContextMenu key lost its original focus');
  await more.focus();
  await page.keyboard.press('ArrowDown');
  await page.getByRole('menu').waitFor();
  await page.keyboard.press('Escape');
  assert(await more.evaluate(node => node === globalThis.document.activeElement), 'ArrowDown menu did not restore More focus');
  const box = await first.boundingBox();
  const point = { x: box.x + 60, y: box.y + 80 };
  await page.mouse.click(point.x, point.y, { button: 'right' });
  const menu = page.getByRole('menu', { name: '更多操作：文城', exact: true });
  await menu.waitFor();
  await page.waitForFunction(x => Math.abs(globalThis.document.querySelector('[role="menu"]').getBoundingClientRect().left - x - 2) < 2, point.x);
  const pointerMenu = await menu.boundingBox();
  assert(Math.abs(pointerMenu.y - point.y) < 2, 'Right-click menu did not follow pointer');
  await page.keyboard.press('Escape');
  for (const key of ['Enter', 'Space']) {
    await more.focus();
    await page.keyboard.press(key);
    await menu.waitFor();
    await page.keyboard.press('Escape');
  }
  await more.click();
  await page.getByRole('menuitem', { name: '在文件管理器中显示目录', exact: true }).click();
  assert(await page.evaluate(() => globalThis.window.__bookMenuCalls.some(call => call.command === 'open_project_directory' && call.args.id === 'menu-0')), 'Directory action lost the selected project');
  await more.click();
  await page.getByRole('menuitem', { name: '删除书籍', exact: true }).click();
  await page.getByRole('alertdialog').waitFor();
  await page.getByRole('button', { name: '取消', exact: true }).click();
  assert(await cards.count() === 4, 'Cancel deletion removed a book');
  await more.click();
  await page.getByRole('heading', { name: '我的书籍', exact: true }).click();
  assert(await page.getByRole('menu').count() === 0, 'Outside click did not close the menu');
  await page.getByRole('button', { name: '列表视图', exact: true }).click();
  await more.click();
  await page.getByRole('menu').waitFor();
  const listMenu = await page.getByRole('menu').boundingBox();
  const listTrigger = await more.boundingBox();
  assert(Math.abs(listMenu.x + listMenu.width - listTrigger.x - listTrigger.width) < 2, 'List view menu lost its anchor');
  await page.keyboard.press('Escape');
  await page.getByRole('button', { name: '网格视图', exact: true }).click();
  const longMore = cards.nth(3).getByRole('button');
  await longMore.click();
  const longMenu = page.getByRole('menu');
  await longMenu.waitFor();
  assert(await longMenu.evaluate(node => node.scrollWidth <= node.clientWidth), 'Long title overflowed the menu');
  assert(await longMenu.getByRole('menuitem', { name: '暂无可导出的成片', exact: true }).getAttribute('aria-disabled') === 'true', 'Empty export action must be disabled');
  await page.keyboard.press('Escape');
  const edge = await cards.nth(2).boundingBox();
  await page.mouse.click(edge.x + edge.width - 30, edge.y + edge.height - 20, { button: 'right' });
  await page.getByRole('menu').waitFor();
  await page.waitForFunction(() => globalThis.document.querySelector('[role="menu"]').getBoundingClientRect().right <= globalThis.innerWidth - 11);
  const edgeMenu = await page.getByRole('menu').boundingBox();
  assert(edgeMenu.y + edgeMenu.height <= 889 && edgeMenu.x >= 11, 'Right-click menu did not avoid the viewport edge');
  await page.keyboard.press('Escape');
  assert(errors.length === 0, 'Page errors: ' + errors.join('; '));
  return { results, interactions: 'More, Shift+F10, ContextMenu, Enter/Space/ArrowDown, pointer/edge positioning, directory, cancel delete, outside click, list view, long title, disabled export, accessible name', errors };
}
