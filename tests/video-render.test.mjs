import { test } from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import { buildEpisode } from '../src-tauri/video/render.mjs';

const plan = {
  title: '书籍原声', duration: 12,
  cues: [{ start: 0.5, end: 2, text: '第一句' }, { start: 5, end: 9, text: '第二句' }],
  scenes: [{ first: 0, last: 0, kind: 'book', title: '开场', labels: ['一本书'] }, { first: 1, last: 1, kind: 'flow', title: '变化', labels: ['之前', '之后'] }],
};
test('original audio timing is identical in Node and browser, including silence and tail', () => {
  const result = buildEpisode(plan);
  const node = { TIMING: {} };
  vm.runInNewContext(result.script, node);
  const layout = node.TIMING.layout(node.SCRIPT.PAGES);
  assert.equal(layout.duration, 12);
  assert.deepEqual(JSON.parse(JSON.stringify(layout.pages.map(p => [p.start, p.end]))), [[0, 5], [5, 12]]);
  assert.equal(layout.pages[1].cues[0].start, 5);
  assert.equal(layout.pages[1].cues[0].end, 9);
  const browser = { window: { TIMING: {} } };
  vm.runInNewContext(result.script, browser);
  assert.equal(browser.window.TIMING.layout(browser.window.SCRIPT.PAGES).duration, layout.duration);
  assert.equal(node.SCRIPT.META.voice, undefined);
});
test('model text remains data and does not become JavaScript', () => {
  const result = buildEpisode({ ...plan, title: "'; globalThis.pwned=true; //" });
  const ctx = { TIMING: {} };
  vm.runInNewContext(result.script, ctx);
  assert.equal(ctx.pwned, undefined);
  assert.equal(ctx.SCRIPT.META.title, "'; globalThis.pwned=true; //");
  assert.equal(result.pages.length, 2);
});
