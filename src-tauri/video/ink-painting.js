/* Deterministic detail reveal sampled from each frozen painting; source images remain intact. */
(function () {
  const ready = [];
  const clamp = value => Math.max(0, Math.min(1, value));
  const smooth = value => { const u = clamp(value); return u * u * (3 - 2 * u); };
  function frame(time, at, duration, motion) {
    const progress = smooth((time - at) / Math.max(1, duration - at));
    const drawing = clamp((time - at) / Math.min(7, Math.max(1.2, (duration - at) * .72)));
    const scale = motion === 'push_in' ? 1.02 + .045 * progress : motion === 'pull_out' ? 1.065 - .045 * progress : motion.startsWith('pan_') ? 1.065 : 1;
    const x = motion === 'pan_left' ? 28 - 56 * progress : motion === 'pan_right' ? -28 + 56 * progress : 0;
    return { drawing, scale, x, opacity: smooth((time - at) / .35) };
  }

  function mount(P) {
    const pg = P.page, image = pg.image, sourceUrl = image.src.startsWith('ink-image:') ? SCRIPT.INK_IMAGES[image.src.slice(10)] : '';
    if (!sourceUrl) throw new Error(`Missing frozen ink image for ${image.assetId}.`);
    const art = P.img(sourceUrl, 0, 0, 1920, 1080, {
      fit: image.fit, pos: `${image.x * 100}% ${image.y * 100}%`, r: 0, shadow: false, border: false, at: image.at, pop: 0,
    });
    art.el.style.visibility = 'visible';
    const imageLayer = art.el.querySelector('img'), source = imageLayer;
    imageLayer.style.opacity = '0';
    const canvas = document.createElement('canvas');
    canvas.width = 960; canvas.height = 540;
    Object.assign(canvas.style, { position: 'absolute', inset: '0', width: '1920px', height: '1080px', pointerEvents: 'none' });
    P.html.appendChild(canvas);
    const context = canvas.getContext('2d', { willReadFrequently: true });
    const inkCanvas = document.createElement('canvas');
    inkCanvas.width = canvas.width; inkCanvas.height = canvas.height;
    const inkContext = inkCanvas.getContext('2d', { willReadFrequently: true });
    let paths, colors, last = -1, lineCount = 0;
    function reset() {
      context.clearRect(0, 0, canvas.width, canvas.height);
      inkContext.clearRect(0, 0, inkCanvas.width, inkCanvas.height);
      lineCount = 0;
    }

    ready.push(source.decode().then(() => {
      const analysis = document.createElement('canvas'); analysis.width = canvas.width; analysis.height = canvas.height;
      const ac = analysis.getContext('2d', { willReadFrequently: true });
      const scale = image.fit === 'contain'
        ? Math.min(960 / source.naturalWidth, 540 / source.naturalHeight)
        : Math.max(960 / source.naturalWidth, 540 / source.naturalHeight);
      const width = source.naturalWidth * scale, height = source.naturalHeight * scale;
      ac.fillStyle = '#f4f0e5'; ac.fillRect(0, 0, 960, 540);
      ac.drawImage(source, (960 - width) * image.x, (540 - height) * image.y, width, height);
      const pixels = ac.getImageData(0, 0, 960, 540).data, gray = new Float32Array(960 * 540), records = [];
      for (let i = 0; i < gray.length; i++) gray[i] = pixels[i * 4] * .299 + pixels[i * 4 + 1] * .587 + pixels[i * 4 + 2] * .114;
      for (let py = 3; py < 537; py += 4) for (let px = 3; px < 957; px += 4) {
        const i = py * 960 + px, gx = gray[i + 1] - gray[i - 1], gy = gray[i + 960] - gray[i - 960], magnitude = Math.hypot(gx, gy);
        if (magnitude < 13 || gray[i] > 236) continue;
        const length = 2 + Math.min(5, magnitude / 18);
        records.push({ x: px, y: py, dx: -gy / magnitude * length, dy: gx / magnitude * length, tone: Math.round(gray[i]), order: Math.hypot((px - 480) / 1.5, py - 300) + (px % 19) * .25 });
      }
      records.sort((a, b) => a.order - b.order);
      paths = new Int16Array(records.length * 4); colors = new Uint8Array(records.length);
      records.forEach((point, index) => {
        const offset = index * 4;
        paths[offset] = point.x * 4; paths[offset + 1] = point.y * 4;
        paths[offset + 2] = Math.round(point.dx * 8); paths[offset + 3] = Math.round(point.dy * 8);
        colors[index] = Math.round(255 - point.tone);
      });
      ac.clearRect(0, 0, canvas.width, canvas.height); analysis.width = 0; analysis.height = 0;
      P.tick(draw);
    }));

    function draw(time) {
      if (!paths) return;
      const f = frame(time, image.at, pg.drawDuration, pg.painting.motion);
      if (time < last) reset();
      const count = Math.floor(clamp(f.drawing / .72) * colors.length);
      inkContext.lineWidth = .44; inkContext.lineCap = 'round'; inkContext.lineJoin = 'round';
      while (lineCount < count) {
        const index = lineCount++, offset = index * 4, x = paths[offset] / 4, y = paths[offset + 1] / 4;
        const dx = paths[offset + 2] / 8, dy = paths[offset + 3] / 8;
        inkContext.strokeStyle = `rgba(40,43,35,${.18 + colors[index] / 450})`;
        inkContext.beginPath(); inkContext.moveTo(x - dx, y - dy); inkContext.lineTo(x + dx, y + dy); inkContext.stroke();
      }
      context.save(); context.clearRect(0, 0, canvas.width, canvas.height);
      context.translate(480 + f.x, 270); context.scale(f.scale, f.scale); context.translate(-480, -270);
      context.drawImage(inkCanvas, 0, 0);
      context.restore();
      imageLayer.style.opacity = (.2 + f.drawing * .8).toFixed(3);
      imageLayer.style.transformOrigin = '50% 50%';
      imageLayer.style.transform = `translateX(${(f.x * 2).toFixed(1)}px) scale(${f.scale.toFixed(4)})`;
      last = time;
    }

    P.hud.text(pg.title, { x: 104, y: 56, size: 34, color: 'ink', at: -.5, cps: 25, pen: false });
    const credit = Array.from(image.source);
    P.hud.text((credit.length > 42 ? credit.slice(0, 41).join('') + '…' : image.source) + ' · ' + String(pg.sourceIndex + 1).padStart(2, '0'), { x: 1816, y: 62, w: 1040, align: 'r', size: 26, color: 'muted', at: -1, cps: 80, pen: false, raw: true });
  }
  window.INK_PAINTING = { ready, mount, frame };
})();
