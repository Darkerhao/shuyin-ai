import test from 'node:test';
import assert from 'node:assert/strict';
import '../src-tauri/video/pencil-drawing.js';
import { buildEpisode } from '../src-tauri/video/render.mjs';
const api = globalThis.SHUYIN_PENCIL_DRAWING;
const stroke = (x = 20, w = 1.2) => ({ p: [[x, 20], [x + 20, 50], [x, 80]], w, a: .6 });
const head = { mask: [[[0, 0], [100, 0], [100, 100], [0, 100], [0, 0]]], strokes: [stroke()], shades: [] };
const cast = [{ id: 'reader', name: '读者', description: '方巾、长脸、窄深领边', heads: { front: head, side: head } }];
const layer = (cue = 0, portrait) => ({ cue, name: '主体', mask: [], strokes: [stroke(300)], shades: [], texts: [], ...(portrait ? { portrait } : {}) });
const placement = (x = 300) => ({ characterId: 'reader', view: 'front', x, y: 150, scale: 1, flip: false });
const art = layers => ({ design: '按原声逐步画出主体与关系', layers });
const cues = [{ index: 0, start: 0 }, { index: 1, start: 2 }, { index: 2, start: 4 }];
test('frozen character geometry survives new poses and placements without a second model head', () => {
    const before = globalThis.structuredClone(cast);
    const a = api.compile(art([layer(0, placement(300))]), cast, cues, 0, 6, 0, 2);
    const b = api.compile(art([layer(0, placement(600))]), cast, cues, 0, 6, 0, 2);
    const one = a.layers.find(l => l.portrait), two = b.layers.find(l => l.portrait);
    assert.deepEqual(one.strokes[0].points, two.strokes[0].points.map(([x, y]) => [x - 300, y]));
    assert.deepEqual(cast, before);
});
test('crowded short cue keeps visible contour durations and groups excess detail instead of racing a pen', () => {
    const l = layer();
    l.strokes = Array.from({ length: 200 }, (_, i) => stroke(20 + i));
    const drawing = api.compile(art([l, layer(1)]), [], cues, 0, 4, 0, 1);
    const strokes = drawing.layers[0].strokes, primary = strokes.filter(s => s.primary), detail = strokes.filter(s => !s.primary);
    assert(primary.length > 0 && primary.length < strokes.length);
    assert(primary.every(s => s.duration >= .08 && s.at + s.duration <= 2));
    assert(detail.every(s => s.duration >= .45 && s.at >= 0 && s.at + s.duration <= 2));
    assert.equal(new Set(detail.map(s => s.at)).size, 1);
    assert.equal(drawing.layers[1].at, 2);
});
test('unknown character, unsafe paths, open masks and invalid cue clocks are rejected', () => {
    const bad = art([layer(0, { ...placement(), characterId: 'other' })]);
    assert.throws(() => api.compile(bad, cast, cues, 0, 6, 0, 2), /portrait/);
    const open = art([layer()]);
    open.layers[0].mask = [[[0, 0], [1, 1]]];
    assert.throws(() => api.validate(open, [], 0, 2), /close/);
    const script = art([layer()]);
    script.layers[0].strokes[0].p = 'alert(1)';
    assert.throws(() => api.validate(script, [], 0, 2), /coordinates/);
    assert.throws(() => api.compile(art([layer(0), layer(2)]), [], cues.slice(0, 2), 0, 6, 0, 2), /clock/);
});
test('adoption validation rejects expanded shadow overload and duplicate frozen character references', () => {
    const l = layer();
    l.shades = Array.from({ length: 60 }, () => ({ p: [[0, 0], [1600, 0], [1600, 900], [0, 900], [0, 0]], angle: 65, gap: 2.5, w: .7, a: .2 }));
    assert.throws(() => api.validate(art([l]), [], 0, 2), /Expanded pencil resource/);
    assert.throws(() => api.validate(art([layer(0, placement()), layer(1, placement(600))]), cast, 0, 2), /portrait/);
});
test('episode preview retains full artwork clock and frozen cast, including cues beyond the preview end', () => {
    const plan = { title: '验证', audioSha256: 'a'.repeat(64), theme: 'ink', duration: 8, cues: [{ start: 0, end: 2, text: '开场' }, { start: 3, end: 5, text: '后文' }, { start: 6, end: 8, text: '结束' }], pencil: { characters: cast, pending: [] }, scenes: [{ first: 0, last: 2, title: '精绘', kind: 'focus', beats: [], artwork: art([layer(0, placement()), layer(2)]) }] };
    const whole = buildEpisode(plan), preview = buildEpisode(plan, { start: 0, end: 4 });
    assert.equal(preview.pages[0].drawDuration, 8);
    assert.deepEqual(preview.pages[0].pencilCues, whole.pages[0].pencilCues);
    assert.deepEqual(preview.pages[0].characters, cast);
    assert(preview.visuals.includes('SHUYIN_PENCIL_DRAWING'));
});
test('numeric renderer is deterministic for forward and backward seeks and frozen portraits are not generated anew', () => {
    const compiled = api.compile(art([layer(0, placement()), layer(1)]), cast, cues, 0, 6, 0, 2);
    const capture = time => {
        const commands = [];
        const ctx = new Proxy({}, { get: (_, name) => (...args) => commands.push([name, ...args]), set: (_, name, value) => { commands.push(['set', name, value]); return true; } });
        api.render({ width: 1600, height: 900, getContext: () => ctx }, compiled, time);
        return commands;
    };
    const expected = capture(1.3);
    capture(5.8);
    assert.deepEqual(capture(1.3), expected);
    assert(capture(4).some(c => c[0] === 'stroke'));
});
