import { test } from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { Buffer } from 'node:buffer';
import { spawnSync } from 'node:child_process';
import { URL, fileURLToPath, pathToFileURL } from 'node:url';
import process from 'node:process';
import { buildEpisode, measureRenderProgress } from '../src-tauri/video/render.mjs';

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

test('render progress counts cached and finalized fragments, not a growing encoder file', t => {
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), 'shuyin-render-progress-'));
  t.after(() => {
    for (const file of fs.readdirSync(directory)) fs.unlinkSync(path.join(directory, file));
    fs.rmdirSync(directory);
  });
  const box = (type, content = Buffer.alloc(0)) => {
    const header = Buffer.alloc(8);
    header.writeUInt32BE(header.length + content.length);
    header.write(type, 4);
    return Buffer.concat([header, content]);
  };
  const finalized = Buffer.concat([box('ftyp'), box('mdat', Buffer.alloc(16)), box('moov', Buffer.alloc(16))]);
  const segments = [
    { a: 0, b: 24, file: 'seg-cached.mp4' },
    { a: 24, b: 96, file: 'seg-finished.mp4' },
    { a: 96, b: 144, file: 'seg-working.mp4' },
    { a: 144, b: 192, file: 'seg-pending.mp4' },
  ];
  fs.writeFileSync(path.join(directory, 'seg-cached.mp4'), finalized);
  fs.writeFileSync(path.join(directory, 'seg-finished.mp4.part.mp4'), finalized);
  const working = path.join(directory, 'seg-working.mp4.part.mp4');
  fs.writeFileSync(working, Buffer.concat([box('ftyp'), Buffer.from([0, 0, 0, 0, 109, 100, 97, 116]), Buffer.alloc(32)]));
  assert.deepEqual(measureRenderProgress(segments, directory, 0), { completed: 96, total: 192 });

  fs.writeFileSync(working, finalized.subarray(0, finalized.length - 4));
  assert.deepEqual(measureRenderProgress(segments, directory, 0), { completed: 96, total: 192 });
  fs.writeFileSync(working, finalized);
  assert.deepEqual(measureRenderProgress(segments, directory, 0), { completed: 144, total: 192 });

  fs.renameSync(working, path.join(directory, 'seg-working.mp4'));
  fs.writeFileSync(path.join(directory, 'seg-pending.mp4'), finalized);
  assert.deepEqual(measureRenderProgress(segments, directory, 0), { completed: 192, total: 192 });
  assert.deepEqual(measureRenderProgress([], directory, 0), { completed: 0, total: 0 });
});

test('render progress reads extended mdat lengths without loading video contents', t => {
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), 'shuyin-render-progress-'));
  const file = path.join(directory, 'seg-large.mp4.part.mp4');
  t.after(() => { fs.unlinkSync(file); fs.rmdirSync(directory); });
  const data = Buffer.alloc(32);
  data.writeUInt32BE(1);
  data.write('mdat', 4);
  data.writeBigUInt64BE(24n, 8);
  data.writeUInt32BE(8, 24);
  data.write('moov', 28);
  fs.writeFileSync(file, data);
  assert.deepEqual(measureRenderProgress([{ a: 0, b: 48, file: 'seg-large.mp4' }], directory, 0), { completed: 48, total: 48 });
});

test('a new run ignores finalized temporary fragments left by a cancelled worker', t => {
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), 'shuyin-render-progress-'));
  const cached = path.join(directory, 'seg-cached.mp4');
  const stale = path.join(directory, 'seg-stale.mp4.part.mp4');
  t.after(() => { fs.unlinkSync(cached); fs.unlinkSync(stale); fs.rmdirSync(directory); });
  const moov = Buffer.from([0, 0, 0, 8, 109, 111, 111, 118]);
  fs.writeFileSync(cached, moov);
  fs.writeFileSync(stale, moov);
  fs.utimesSync(stale, 1, 1);
  assert.deepEqual(measureRenderProgress([
    { a: 0, b: 24, file: 'seg-cached.mp4' },
    { a: 24, b: 96, file: 'seg-stale.mp4' },
  ], directory, Date.now()), { completed: 24, total: 96 });
});

test('stage initialization errors exit promptly and browser dependencies use canonical file URLs', t => {
  const directory = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), 'shuyin-stage-error-')));
  assert.equal(path.dirname(directory), fs.realpathSync(os.tmpdir()));
  t.after(() => fs.rmSync(directory, { recursive: true }));
  const engine = path.join(directory, 'engine & runtime');
  const modules = path.join(directory, 'dependencies & fonts');
  fs.mkdirSync(path.join(engine, 'tools'), { recursive: true });
  fs.mkdirSync(modules);
  fs.writeFileSync(path.join(engine, 'package.json'), JSON.stringify({ name: '@frametell/engine', version: '0.7.3' }));
  fs.writeFileSync(path.join(engine, 'tools/bins.mjs'), 'export const FFMPEG = process.execPath; export const findChrome = () => true; export const probe = () => ({ duration: 12 });');
  fs.writeFileSync(path.join(engine, 'tools/project.mjs'), `import fs from 'node:fs'; import path from 'node:path'; export function linkProject(dir) { fs.symlinkSync(${JSON.stringify(modules)}, path.join(dir, 'engine_modules'), process.platform === 'win32' ? 'junction' : 'dir'); }`);
  fs.writeFileSync(path.join(engine, 'tools/lib.mjs'), `export async function openStage() { setInterval(() => {}, 1000); throw new Error('fixture stage initialization failed'); } export const closeStage = async () => {}; export const loadEpisode = () => ({});`);
  fs.writeFileSync(path.join(engine, 'tools/checks.mjs'), 'export const lintPages = async () => [];');
  const input = path.join(directory, 'input.json');
  fs.writeFileSync(input, JSON.stringify({ action: 'render', engineRoot: engine, audioPath: 'fixture.mp3', plan }));
  const script = fileURLToPath(new URL('../src-tauri/video/render.mjs', import.meta.url));
  const result = spawnSync(process.execPath, [script, input, path.join(directory, 'output.json')], { encoding: 'utf8', windowsHide: true, timeout: 4000 });
  const html = fs.readFileSync(path.join(directory, 'episode/index.html'), 'utf8');
  assert.equal(result.status, 1, `The failed worker must exit rather than wait on browser handles: ${result.stderr}`);
  assert.match(result.stderr, /fixture stage initialization failed/);
  assert(html.includes(`content="${pathToFileURL(modules + path.sep).href.replaceAll('&', '&amp;')}"`));
  assert(html.includes(`src="${pathToFileURL(path.join(engine, 'boot.js')).href.replaceAll('&', '&amp;')}"`));
});
