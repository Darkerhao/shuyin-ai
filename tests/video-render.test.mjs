import { test } from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { Buffer } from 'node:buffer';
import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { URL, fileURLToPath, pathToFileURL } from 'node:url';
import process from 'node:process';
import { buildEpisode, measureRenderProgress } from '../src-tauri/video/render.mjs';

const plan = {
  title: '书籍原声', duration: 12,
  cues: [{ start: 0.5, end: 2, text: '第一句' }, { start: 5, end: 9, text: '第二句' }],
  scenes: [{ first: 0, last: 0, kind: 'opening', title: '开场', beats: [{ cue: 0, object: 'book', action: 'draw', label: '一本书' }] }, { first: 1, last: 1, kind: 'route', title: '变化', beats: [{ cue: 1, object: 'steps', action: 'slide', label: '变化' }] }],
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

test('a preview keeps global cue identity and original drawing time while clipping only its viewing window', () => {
  const original = JSON.stringify(plan);
  const result = buildEpisode(plan, { start: 5, end: 8 });
  const context = { TIMING: {} };
  vm.runInNewContext(result.script, context);
  const layout = context.TIMING.layout();
  assert.equal(layout.duration, 3);
  assert.equal(layout.pages.length, 1);
  const page = layout.pages[0];
  assert.equal(page.id, 'p0002');
  assert.equal(page.index, 0, 'Engine indices stay local to the generated episode.');
  assert.equal(page.sourceIndex, 1);
  assert.equal(page.start, 0);
  assert.equal(page.end, 3);
  assert.equal(page.drawDuration, 7, 'A shortened preview must not speed up the source scene.');
  assert.equal(page.cues[0].id, 1);
  assert.equal(page.cues[0].start, 0);
  assert.equal(page.cues[0].end, 3);
  assert.equal(page.beats[0].cue, 1);
  assert.equal(page.beats[0].at, 0);
  assert.equal(JSON.stringify(plan), original, 'Previewing must not rewrite the saved plan.');
});

test('preview windows reject invalid times, mid-scene starts and durations over one minute', () => {
  for (const range of [{ start: 2, end: 8 }, { start: -1, end: 8 }, { start: 5, end: 5 }, { start: 5, end: 13 }, { start: NaN, end: 8 }, { end: 8 }]) {
    assert.throws(() => buildEpisode(plan, range), /preview/i);
  }
  assert.throws(() => buildEpisode({ ...plan, duration: 90 }, { start: 0, end: 61 }), /preview/i);
});

function drawing(input, index = 0, range) {
  const result = buildEpisode(input, range);
  const page = result.pages.find(p => p.sourceIndex === index);
  const calls = [], moves = [], live = [];
  let layers = 0;
  const builder = (layer = 0) => new Proxy({ page, c: i => page.cues[i].start - page.start, layer: () => builder(++layers), move: (_, fn) => moves.push(fn), liveRough: fn => live.push(fn) }, {
    get(target, key) { return key in target ? target[key] : (...args) => { calls.push({ layer, method: key, args }); }; },
  });
  const context = { SCRIPT: { META: { title: input.title } }, PV: { K: (t, at, duration) => Math.max(0, Math.min(1, (t - at) / duration)), col: () => '#FBF8F1' }, window: {}, document: { createElement: () => ({}), head: { appendChild() {} } } };
  vm.runInNewContext(result.visuals, context);
  context.window.EPISODE.draw(builder());
  return { calls: JSON.parse(JSON.stringify(calls)), at: t => JSON.parse(JSON.stringify([...moves.map(fn => fn(t)), ...live.map(fn => fn(t, { line: (...args) => args }))])) };
}

test('the first object and its semantic motion start at the actual cue, not before the page', () => {
  const delayed = { ...plan, scenes: [{ first: 0, last: 1, kind: 'collapse', title: '改变', beats: [{ cue: 1, object: 'crown', action: 'tilt', label: '身份' }] }] };
  const output = drawing(delayed);
  assert.equal(output.calls.find(c => c.layer === 1 && c.method === 'path').args.at(-1).at, 5);
  assert.equal(output.at(5)[0].rot, 0);
  assert(output.at(5 + 1 / 24)[0].rot > 0, 'Motion must begin without an extra 1.3 second delay.');
  const marked = drawing({ ...delayed, scenes: [{ ...delayed.scenes[0], beats: [{ cue: 1, object: 'certificate', action: 'check', label: '确认' }] }] });
  assert.equal(marked.calls.find(c => c.method === 'check').args.at(-1).at, 5);
});

test('one subject is centered and paired narration has a clear lead without biasing a comparison', () => {
  const scene = { first: 0, last: 1, kind: 'focus', title: '阅读', beats: [{ cue: 0, object: 'book', action: 'draw', label: '书籍' }] };
  assert.equal(drawing({ ...plan, scenes: [scene] }).at(4)[0].x, 960);
  scene.beats.push({ cue: 1, object: 'pen', action: 'draw', label: '书写' });
  const paired = drawing({ ...plan, scenes: [scene] }).at(8);
  assert(paired[0].s > paired[1].s);
  assert(paired[0].x < 960 && paired[1].x > 960);
  const compared = drawing({ ...plan, scenes: [{ ...scene, kind: 'compare' }] }).at(8);
  assert.equal(compared[0].s, compared[1].s);
  scene.beats.push({ cue: 1, object: 'crown', action: 'draw', label: '象征' });
  const sequence = drawing({ ...plan, scenes: [{ ...scene, kind: 'route' }] }).at(8);
  assert(sequence[0].s > sequence[1].s && sequence[0].s > sequence[2].s);
});

test('a truncated preview has the exact full-scene drawing even when later beats fall outside it', () => {
  const input = { ...plan, scenes: [{ first: 0, last: 1, kind: 'focus', title: '阅读', beats: [{ cue: 0, object: 'book', action: 'sway', label: '书籍' }, { cue: 1, object: 'pen', action: 'draw', label: '书写' }] }] };
  const full = drawing(input), preview = drawing(input, 0, { start: 0, end: 3 });
  assert.deepEqual(preview.calls, full.calls);
  assert.deepEqual(preview.at(2), full.at(2));
  assert(Math.abs(full.at(8)[0].rot) < 0.001, 'A semantic sway settles instead of looping for the whole scene.');
  const silent = buildEpisode({ ...plan, cues: plan.cues.map(c => ({ ...c, start: c.start + 30, end: c.end + 30 })), duration: 45 }, { start: 0, end: 30 });
  const context = { TIMING: {} };
  vm.runInNewContext(silent.script, context);
  assert.equal(context.TIMING.layout().pages[0].cues.length, 0);
  assert.equal(context.TIMING.layout().duration, 30);
});

test('object storyboards compile the reference composition and exact subtitle anchors', () => {
  const illustrated = {
    ...plan,
    scenes: [{ first: 0, last: 1, kind: 'collapse', title: '身份变化', beats: [
      { cue: 0, object: 'crown', action: 'tilt', label: '皇冠' },
      { cue: 1, object: 'certificate', action: 'check', label: '公民' },
    ] }],
  };
  const result = buildEpisode(illustrated);
  const context = { TIMING: {} };
  vm.runInNewContext(result.script, context);
  const page = context.TIMING.layout().pages[0];
  assert.equal(context.SCRIPT.META.lang, 'zh');
  assert.equal(context.SCRIPT.META.fadeDelay, 0);
  assert(context.SCRIPT.META.extras.includes('illustrations.js'));
  assert.equal(page.kind, 'collapse');
  assert.equal(page.beats[1].cue, 1);
  assert.equal(page.cues[1].id, 1);
  assert.equal(page.cues[1].start, 5);
  assert.match(result.visuals, /case 'crown'/);
  assert.match(result.visuals, /font-size:44px/);
  assert.match(result.visuals, /P\.move/);
});

test('text-card plans and unknown objects cannot silently render as same-style animation', () => {
  assert.throws(() => buildEpisode({ ...plan, scenes: [{ first: 0, last: 1, kind: 'book', title: '旧图解', labels: ['一本书'] }] }), /重新生成分镜/);
  const scene = { first: 0, last: 1, kind: 'focus', title: '场景', beats: [{ cue: 0, object: '../../evil.js', action: 'draw', label: '物体' }] };
  assert.throws(() => buildEpisode({ ...plan, scenes: [scene] }), /object/i);
  scene.beats[0].object = 'book';
  scene.beats[0].cue = 2;
  assert.throws(() => buildEpisode({ ...plan, scenes: [scene] }), /cue/i);
});

test('every catalog object has the reference drawing and every action is finite and seek-independent', () => {
  const catalog = JSON.parse(fs.readFileSync(new URL('../src-tauri/video/visual-catalog.json', import.meta.url), 'utf8'));
  const source = fs.readFileSync(new URL('../src-tauri/video/illustrations.js', import.meta.url), 'utf8');
  const normalized = source.replaceAll('\r\n', '\n');
  const geometry = normalized.slice(normalized.indexOf('  function sketch('), normalized.indexOf('  function draw('));
  assert.equal(createHash('sha256').update(geometry).digest('hex'), '96a6bc4e09c4440682d68e97f417b1357cf38c7a0a327482084d8f590b36fe9b', 'Keep all reference object geometry, not approximate replacements.');
  assert.deepEqual(new Set([...source.matchAll(/case '([^']+)'/g)].map(match => match[1])), new Set(Object.keys(catalog.objects)));
  assert.equal(Object.keys(catalog.objects).length, 85);
  for (const object of Object.keys(catalog.objects)) {
    for (const action of Object.keys(catalog.actions)) {
      const transforms = [], live = [], calls = [];
      const page = { id: 'p01', index: 0, sourceIndex: 0, title: '手绘物体', kind: 'focus', start: 0, end: 12, drawDuration: 12, cues: [{ id: 0, start: 0 }], beats: [{ cue: 0, object, action, label: '物体', at: 0 }] };
      const builder = () => new Proxy({ page, c: () => 0, layer: () => builder(), move: (_, fn) => transforms.push(fn), liveRough: fn => live.push(fn) }, {
        get(target, key) { return key in target ? target[key] : (...args) => { calls.push([key, ...args]); }; },
      });
      const context = { SCRIPT: { META: { title: '原书' } }, PV: { K: (t, at, duration) => Math.max(0, Math.min(1, (t - at) / duration)), col: () => '#FBF8F1' }, window: {}, document: { createElement: () => ({}), head: { appendChild() {} } } };
      vm.runInNewContext(source, context);
      context.window.EPISODE.draw(builder());
      const drawAt = t => [...transforms.map(fn => fn(t)), ...live.map(fn => fn(t, { line: (...args) => args }))];
      const first = JSON.stringify(drawAt(5));
      drawAt(11); drawAt(0);
      assert.equal(JSON.stringify(drawAt(5)), first, `${object}/${action} must be a pure function of time`);
      const finite = value => {
        if (typeof value === 'number') assert(Number.isFinite(value), `${object}/${action}: invalid geometry`);
        else if (typeof value === 'string') assert.doesNotMatch(value, /NaN|Infinity/);
        else if (value && typeof value === 'object') Object.values(value).forEach(finite);
      };
      finite([calls, drawAt(0), drawAt(5), drawAt(11)]);
      assert(calls.some(([method]) => ['path', 'circle', 'box', 'ellipse', 'line'].includes(method)), `${object} must draw a real object`);
    }
  }
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

function failedStageEpisode(t, extraInput = {}) {
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
  fs.writeFileSync(input, JSON.stringify({ action: 'render', engineRoot: engine, audioPath: 'fixture.mp3', plan, ...extraInput }));
  const script = fileURLToPath(new URL('../src-tauri/video/render.mjs', import.meta.url));
  const result = spawnSync(process.execPath, [script, input, path.join(directory, 'output.json')], { encoding: 'utf8', windowsHide: true, timeout: 4000 });
  const html = fs.readFileSync(path.join(directory, 'episode/index.html'), 'utf8');
  return { result, html, modules, engine, script: fs.readFileSync(path.join(directory, 'episode/script.js'), 'utf8'), progress: JSON.parse(fs.readFileSync(path.join(directory, 'progress.json'), 'utf8')) };
}

test('the render entry point routes a preview through the same compiler and reports preview progress', t => {
  const { script, progress } = failedStageEpisode(t, { action: 'preview', window: { start: 5, end: 8 }, signature: 'fixture' });
  const context = { TIMING: {} };
  vm.runInNewContext(script, context);
  assert.equal(context.TIMING.layout().duration, 3);
  assert.equal(context.TIMING.layout().pages[0].cues[0].id, 1);
  assert.equal(progress.stage, 'previewing');
});

test('stage initialization errors exit promptly and browser dependencies use canonical file URLs', t => {
  const { result, html, modules, engine } = failedStageEpisode(t);
  assert.equal(result.status, 1, `The failed worker must exit rather than wait on browser handles: ${result.stderr}`);
  assert.match(result.stderr, /fixture stage initialization failed/);
  assert(html.includes(`content="${pathToFileURL(modules + path.sep).href.replaceAll('&', '&amp;')}"`));
  assert(html.includes(`src="${pathToFileURL(path.join(engine, 'boot.js')).href.replaceAll('&', '&amp;')}"`));
});

test('episode readiness waits for the initial paint before checks or frame capture', async t => {
  const { html } = failedStageEpisode(t);
  const readyScript = /<script>([\s\S]*?)<\/script>/.exec(html)?.[1];
  assert.ok(readyScript, 'The generated episode must wait for paint after the engine boot script.');
  assert(html.indexOf('<script>') > html.indexOf('boot.js'));
  const frames = [];
  const engine = Promise.withResolvers();
  const window = { __ready: engine.promise };
  vm.runInNewContext(readyScript, { window, requestAnimationFrame: callback => frames.push(callback) });
  let ready = false;
  window.__ready.then(() => { ready = true; });
  await Promise.resolve();
  assert.equal(frames.length, 0, 'Paint must wait for the engine, including its fonts.');
  engine.resolve(12);
  await Promise.resolve();
  assert.equal(ready, false);
  assert.equal(frames.length, 1);
  frames.shift()();
  await Promise.resolve();
  assert.equal(ready, false, 'The first animation callback runs before paint.');
  assert.equal(frames.length, 1);
  frames.shift()();
  assert.equal(await window.__ready, 12, 'Preserve the duration returned by the engine.');
  assert.equal(frames.length, 0);

  const failedEngine = Promise.withResolvers();
  const failedWindow = { __ready: failedEngine.promise };
  vm.runInNewContext(readyScript, { window: failedWindow, requestAnimationFrame: () => assert.fail('Do not capture a failed stage.') });
  const error = new Error('engine could not load its fonts');
  failedEngine.reject(error);
  await assert.rejects(failedWindow.__ready, cause => cause === error);
});
