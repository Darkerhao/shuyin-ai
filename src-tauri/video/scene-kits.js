/* Trusted adapters for the external engine's native theme kits. All motion uses page-local source time. */
(function () {
  const { K } = PV;
  const ready = [];
  const sprites = new Map();
  const webgl = new Set(['cosmos', 'diorama', 'exploded', 'globe', 'molecule']);
  const native = object => /^(pixel_|comic_|space_|mini_|product_|world_|micro_)/.test(object);
  const safe = text => String(text).replace(/[&<>"']/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[c]);
  const xs = count => count === 1 ? [960] : count === 2 ? [590, 1330] : [420, 960, 1500];

  function label(P, beat, x, y = 790, width = 440) {
    const H = P.hud || P;
    H.text(beat.label, { x, y, w: width, align: 'c', size: 36, color: 'ink', at: beat.at, cps: 14, raw: true });
    if (beat.action === 'check') H.check(x + width / 2 - 10, y + 12, 46, { color: 'green', at: beat.at, dur: .5 });
    if (beat.action === 'cross') H.cross(x + width / 2 - 10, y + 12, 46, { color: 'red', at: beat.at, dur: .5 });
  }

  function motion(beat, t) {
    const k = K(t, beat.at, 1.5);
    return { x: beat.action === 'slide' ? -40 * (1 - k) : 0,
      y: beat.action === 'tilt' ? 25 * k : 0,
      rot: beat.action === 'tilt' ? 12 * k : beat.action === 'sway' ? 5 * Math.sin(Math.PI * k) : 0 };
  }

  function paperObject(P, sketch, beat, x, y, scale) {
    const Q = sketch(P, beat.object, beat.at, 'ink');
    P.move(Q, t => { const m = motion(beat, t); return { ...m, x: x + m.x, y: y + m.y, s: scale }; });
  }

  // The existing trusted geometry becomes a low-resolution sprite, rather than a second drawing catalog.
  function sprite(P, sketch, object) {
    if (sprites.has(object)) return sprites.get(object);
    const firstAct = P.sh.acts.length;
    const Q = sketch(P, object, -1000, 'ink');
    const bounds = Q.svg.getBBox(), svg = Q.svg.cloneNode(true);
    Q.wrap.remove();
    P.sh.acts.splice(firstAct);
    svg.setAttribute('xmlns', 'http://www.w3.org/2000/svg');
    svg.setAttribute('viewBox', `${bounds.x - 10} ${bounds.y - 10} ${bounds.width + 20} ${bounds.height + 20}`);
    svg.setAttribute('width', '48'); svg.setAttribute('height', '48');
    svg.removeAttribute('style');
    for (const node of svg.querySelectorAll('*')) {
      node.removeAttribute('filter'); node.style.strokeDasharray = ''; node.style.strokeDashoffset = '';
      node.style.visibility = ''; node.style.opacity = '';
    }
    const image = new Image();
    image.src = 'data:image/svg+xml;charset=utf-8,' + encodeURIComponent(new XMLSerializer().serializeToString(svg));
    ready.push(image.decode());
    sprites.set(object, image);
    return image;
  }

  function pixel(P, sketch) {
    const beats = P.page.beats, positions = xs(beats.length).map(x => x / 6);
    const images = new Map(beats.filter(b => !native(b.object) && !['coins', 'key'].includes(b.object)).map(b => [b.object, sprite(P, sketch, b.object)]));
    P.pixel((g, t) => {
      g.sky('night'); g.mountains(111, { par: .2 }); g.hills(129, { par: .45 }); g.ground(147, 'grass');
      beats.forEach((beat, i) => {
        if (t < beat.at) return;
        const m = motion(beat, t), x = positions[i] + m.x / 6, y = 137 + m.y / 6;
        g.ctx.save(); g.ctx.translate(Math.round(x), Math.round(y)); g.ctx.rotate(m.rot * Math.PI / 180);
        if (beat.object === 'pixel_robot') g.hero(0, 0, { t: Math.max(0, t - beat.at), scale: 3, pose: beat.action === 'slide' ? 'walk' : beat.action === 'check' ? 'win' : beat.action === 'cross' || beat.action === 'tilt' ? 'hurt' : 'idle' });
        else if (beat.object === 'pixel_bug') g.bug(0, 0, { t: Math.max(0, t - beat.at), scale: 4 });
        else if (beat.object === 'pixel_chest') g.chestOpen(0, 0, K(t, beat.at, 1.5), { scale: 3 });
        else if (beat.object === 'pixel_star') g.sprite('star', 0, 0, { scale: 4 });
        else if (beat.object === 'coins') g.coin(0, 0, { t: Math.max(0, t - beat.at), scale: 4 });
        else if (beat.object === 'key') g.sprite('key', 0, 0, { scale: 4 });
        else { const image = images.get(beat.object); if (image.complete && image.naturalWidth) g.ctx.drawImage(image, -28, -56, 56, 56); }
        g.ctx.restore();
      });
    });
    beats.forEach((beat, i) => label(P, beat, positions[i] * 6, 815));
  }

  function manga(P, sketch) {
    const beats = P.page.beats;
    const layout = beats.length === 1 ? 'full' : beats.length === 2 ? (P.page.kind === 'compare' ? 'slash' : '2') : (P.page.sourceIndex % 2 ? '2+1' : '1+2');
    const rects = P.grid(layout, { x: 140, y: 205, w: 1640, h: 605 });
    const props = { book: 'book', clock: 'clock', calendar: 'calendar', bed: 'bed', cup: 'coffee' };
    beats.forEach((beat, i) => {
      const R = rects[i], A = P.panel(R, { at: beat.at, tone: 'light' });
      const x = R.cx, y = R.cy - 12, size = Math.min(R.w * .7, R.h * .72);
      const subject = A.layer();
      if (beat.object.startsWith('comic_')) {
        subject.chara(x, y - size * .12, size, { at: beat.at, mood: beat.action === 'check' ? 'happy' : beat.action === 'cross' || beat.action === 'tilt' ? 'despair' : 'calm', hair: beat.object === 'comic_mentor' ? 'neat' : 'spiky', outfit: 'suit', glasses: beat.object === 'comic_mentor' });
      } else if (props[beat.object]) subject.prop(props[beat.object], x, y, size, { at: beat.at });
      else paperObject(A, sketch, beat, x, y, size / 350);
      if (native(beat.object) || props[beat.object]) P.move(subject, t => motion(beat, t), [x, y]);
      label(P, beat, x, 827, Math.min(440, R.w));
    });
  }

  function pop(P, sketch) {
    const beats = P.page.beats, rects = P.grid(beats.length === 1 ? 'full' : String(beats.length), { x: 140, y: 205, w: 1640, h: 605 });
    beats.forEach((beat, i) => {
      const R = rects[i], x = R.x + R.w / 2, y = R.y + R.h * .52;
      const A = P.panel(R.x, R.y, R.w, R.h, { at: beat.at, from: 'pop', dots: { color: i % 2 ? 'red' : 'blue', ramp: .15 } });
      paperObject(A, sketch, beat, x, y, Math.min(R.w, R.h) * .7 / 350);
      label(P, beat, x, 827, Math.min(R.w, 440));
    });
  }

  function cosmos(P, beats) {
    const S = P.space({ dist: beats.length === 1 ? 7 : 11, drift: .3, seed: 4242 });
    beats.forEach((beat, i) => {
      const body = S.body(beat.object.slice(6), { r: 1.05, pos: [(i - (beats.length - 1) / 2) * 3.4, 0, 0], at: beat.at });
      S.label(body, safe(beat.label), { at: beat.at, sub: '示意 · 非等比例', side: i === 0 ? 'l' : 'r' });
      if (P.page.framing === 'detail' && beats.length === 1) S.cameraTo(body, 5.5, beat.at, 3);
    });
  }

  function diorama(P, beats) {
    // Shift the kit's own entrance clock so a later cue is never treated as pre-existing scenery.
    const D = P.diorama({ at: -1, size: [24, 16], seed: 7, sky: 'day' });
    D.terrain({ hills: .1, trees: 0 });
    beats.forEach((beat, i) => {
      const area = [2 + i * (20 / beats.length), 4, 18 / beats.length, 8];
      if (beat.object === 'mini_city' || beat.object === 'mini_houses') D.city({ area, kind: beat.object === 'mini_houses' ? 'houses' : 'mixed', density: .7, at: beat.at });
      else if (beat.object === 'mini_factory') D.landmark('factory', [area[0] + area[2] / 2, 8], { at: beat.at, s: 1.7 });
      else D.zone(area, beat.object === 'mini_farm' ? 'farm' : 'forest', { at: beat.at });
      D.label([area[0] + area[2] / 2, 8], safe(beat.label), { at: beat.at, sub: '场景示意' });
    });
  }

  function exploded(P, beats) {
    beats.forEach((beat, i) => {
      const A = P.layer(), next = beats[i + 1]?.at ?? P.page.drawDuration;
      const duration = Math.max(.2, next - beat.at);
      const S = A.studio({ model: beat.object.slice(8), at: beat.at, box: [430, 210, 1060, 590] });
      S.turn(beat.at, 24, Math.min(3, duration));
      if (duration >= 4) { S.explode(beat.at + duration * .25, { dur: Math.min(1.2, duration * .2) }); S.assemble(beat.at + duration * .75, { dur: Math.min(.75, duration * .15) }); }
      P.tick(t => { A.wrap.style.visibility = t >= beat.at && t < next ? 'visible' : 'hidden'; });
      const caption = (P.hud || P).layer();
      label(caption, beat, 960, 815);
      P.tick(t => { caption.wrap.style.visibility = t >= beat.at && t < next ? 'visible' : 'hidden'; });
    });
  }

  function globe(P, beats) {
    P.globe({ at: beats[0].at, cx: 960, cy: 495, r: 300, spin: .7 });
    beats.forEach(beat => label(P, beat, 960, 815));
  }

  function molecule(P, beats) {
    const M = P.micro({ mood: 'teal', scale: false, motes: 100, drift: .3 });
    beats.forEach((beat, i) => {
      const x = xs(beats.length)[i], o = { x, y: 510, r: beats.length === 1 ? 210 : 135, at: beat.at, pop: false, float: .25 };
      const object = beat.object === 'micro_dna' ? M.helix({ ...o, length: 750 })
        : beat.object === 'micro_cell' ? M.cell(o)
          : M.molecule({ micro_water: 'H2O', micro_oxygen: 'O2', micro_co2: 'CO2' }[beat.object], o);
      M.label(object, safe(beat.label), { at: beat.at, sub: '结构示意' });
    });
  }

  function draw(P, sketch) {
    const page = P.page, theme = page.theme;
    if (page.image || page.painting) return false;
    if (theme !== 'pixel' && theme !== 'manga' && theme !== 'pop' && !(webgl.has(theme) && page.beats.some(b => native(b.object)))) return false;
    if (theme === 'pixel') pixel(P, sketch);
    else if (theme === 'manga') manga(P, sketch);
    else if (theme === 'pop') pop(P, sketch);
    else {
      const beats = page.beats.filter(b => native(b.object));
      const glyphs = document.createElement('span');
      glyphs.hidden = true;
      glyphs.textContent = beats.map(beat => beat.label).join('') + '示意非等比例结构场景';
      P.html.appendChild(glyphs);
      // The engine discovers fonts in a first build; allocate GPU scenes only when the page is actually rendered.
      let built = false;
      P.tick(t => {
        if (built) return;
        built = true;
        const from = P.sh.acts.length;
        ({ cosmos, diorama, exploded, globe, molecule })[theme](P, beats);
        for (const act of P.sh.acts.slice(from)) PV.applyAct(act, t);
      });
      const ordinary = page.beats.filter(b => !native(b.object));
      ordinary.forEach((beat, i) => { paperObject(P, sketch, beat, 350 + i * 1200, 680, .55); label(P, beat, 350 + i * 1200, 815); });
      for (const [i, beat] of beats.entries()) if (beat.action === 'check' || beat.action === 'cross') label(P, { ...beat, label: '' }, xs(beats.length)[i], 820);
    }
    const H = P.hud || P;
    H.text(page.title, { x: 140, y: 90, w: 1600, size: 52, color: 'ink', at: -.5, cps: 18, raw: true });
    return true;
  }
  window.SCENE_KITS = { draw, ready };
})();
