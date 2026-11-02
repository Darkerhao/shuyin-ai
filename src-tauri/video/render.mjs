import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { createHash } from 'node:crypto';
import { spawn } from 'node:child_process';
import { createInterface } from 'node:readline';

const catalog = JSON.parse(fs.readFileSync(new URL('./visual-catalog.json', import.meta.url), 'utf8'));
const visuals = fs.readFileSync(new URL('./illustrations.js', import.meta.url), 'utf8');

export function assertNodeRuntime(version = process.versions.node, platform = process.platform) {
  const [major, minor, patch] = version.split('.').map(Number);
  // nodejs/node#61108 fixes native rmSync crashes on Windows non-ASCII paths.
  if (platform === 'win32' && (major === 23 || (major === 24 && (minor < 13 || (minor === 13 && patch === 0))) || (major === 25 && minor < 4))) {
    throw new Error(`Node.js ${version} 存在 Windows 中文路径处理缺陷，会导致视频进程崩溃。请在「视频运行环境」的「Node 程序」中选择 Node.js 24.13.1 或更新的 24.x LTS（推荐 24.22.2），保存后重试；已保存字幕和分镜会保留。`);
  }
}

// The model selects trusted drawings and subtitle anchors; it never supplies code.
export function buildEpisode(plan, range, images = {}) {
  const shortText = (value, limit) => typeof value === 'string' && value.trim() && [...value].length <= limit && !/[\p{Cc}{}*`]/u.test(value);
  let next = 0;
  for (const scene of plan.scenes) {
    const { environment = 'plain', framing = 'reference' } = scene;
    if (Object.keys(scene).some(key => !['first', 'last', 'title', 'kind', 'labels', 'beats', 'environment', 'framing', 'image'].includes(key))) throw new Error('Unknown scene field.');
    if (!Object.hasOwn(catalog.environments, environment)) throw new Error('Unknown scene environment.');
    if (!Object.hasOwn(catalog.framings, framing)) throw new Error('Unknown scene framing.');
    if (!scene.beats?.length && !scene.image) throw new Error('旧版文字图解分镜：请保留字幕并重新生成分镜，升级为同款手绘动画。');
    if (!Object.hasOwn(catalog.kinds, scene.kind)) throw new Error('Unknown scene kind.');
    if (!shortText(scene.title, catalog.limits.titleChars) || scene.labels?.length) throw new Error('Invalid scene title or legacy labels.');
    if (!Number.isInteger(scene.first) || scene.first !== next || !Number.isInteger(scene.last) || scene.last < scene.first || scene.last >= plan.cues.length || scene.last - scene.first >= catalog.limits.maxSceneCues) throw new Error('Invalid scene cue coverage.');
    if ((scene.beats?.length ?? 0) > catalog.limits.maxBeats) throw new Error('A scene supports at most three objects.');
    if (scene.image) {
      const im = scene.image, frozen = images[im.assetId];
      const text = (v, n) => typeof v === 'string' && v.trim() && [...v].length <= n && !/[\p{Cc}]/u.test(v.replaceAll('\n', ''));
      if (Object.keys(im).some(k => !['assetId', 'source', 'rights', 'creator', 'fit', 'x', 'y', 'sha256'].includes(k))
        || !text(im.assetId, 128) || !text(im.source, 1000) || !text(im.rights, 1000) || (im.creator && !text(im.creator, 200))
        || !['contain', 'cover'].includes(im.fit) || !Number.isFinite(im.x) || !Number.isFinite(im.y) || im.x < 0 || im.x > 1 || im.y < 0 || im.y > 1
        || scene.beats?.length || !frozen || !/^[a-f0-9]{64}$/.test(im.sha256) || im.sha256 !== frozen.sha256
        || !new RegExp('^assets/' + im.sha256 + '\\.(png|jpg|webp)$').test(frozen.src)) throw new Error('Invalid or missing frozen scene image.');
    }
    let previous = scene.first;
    for (const beat of scene.beats ?? []) {
      if (!Object.hasOwn(catalog.objects, beat.object)) throw new Error('Unknown drawing object.');
      if (!Object.hasOwn(catalog.actions, beat.action ?? 'draw')) throw new Error('Unknown drawing action.');
      if (!shortText(beat.label, catalog.limits.labelChars)) throw new Error('Invalid object label.');
      if (!Number.isInteger(beat.cue) || beat.cue < previous || beat.cue > scene.last) throw new Error('Invalid object cue anchor.');
      previous = beat.cue;
    }
    next = scene.last + 1;
  }
  if (!plan.scenes.length || next !== plan.cues.length) throw new Error('Incomplete scene cue coverage.');
  if (range && (!Number.isFinite(range.start) || !Number.isFinite(range.end) || range.start < 0 || range.end <= range.start || range.end > plan.duration || range.end - range.start > 60 || !plan.scenes.some((s, i) => (i === 0 ? 0 : plan.cues[s.first].start) === range.start))) throw new Error('Invalid preview window: choose a scene boundary and at most 60 seconds.');
  const from = range?.start ?? 0, to = range?.end ?? plan.duration;
  const pages = plan.scenes.map((scene, index) => {
    const start = index === 0 ? 0 : plan.cues[scene.first].start;
    const end = index + 1 === plan.scenes.length ? plan.duration : plan.cues[plan.scenes[index + 1].first].start;
    return {
      id: `p${String(index + 1).padStart(4, '0')}`, index, title: scene.title, kind: scene.kind,
      environment: scene.environment ?? 'plain', framing: scene.framing ?? 'reference',
      // Keep the full drawing clock even when the preview ends before this scene does.
      drawDuration: end - start, ...(scene.image ? { image: { ...scene.image, src: images[scene.image.assetId].src, at: plan.cues[scene.first].start - start } } : {}), beats: (scene.beats ?? []).map(b => ({ ...b, at: plan.cues[b.cue].start - start })),
      start: start - from, end: Math.min(end, to) - from,
      cues: plan.cues.slice(scene.first, scene.last + 1).map((c, offset) => ({ id: scene.first + offset, t: c.text, start: c.start - from, end: (range ? Math.min(c.end, to) : c.end) - from })).filter(c => c.start < to - from && c.end > 0),
    };
  }).filter(p => p.start >= 0 && p.start < to - from).map((p, index) => ({ ...p, sourceIndex: p.index, index }));
  const data = { META: { title: plan.title, name: 'film', lang: 'zh', style: 'study', extras: ['timeline.js', 'illustrations.js'], fadeDelay: 0, fadeOut: 0, pageMotion: false }, PAGES: pages, duration: to - from };
  const script = `(function(root) {
root.SCRIPT = ${JSON.stringify(data)};
root.applyVideoTimeline = function() {
  root.TIMING.layout = function() {
    return { duration: root.SCRIPT.duration, gap: 0,
      pages: root.SCRIPT.PAGES.map(p => ({ ...p, speechEnd: p.cues.at(-1)?.end ?? p.end,
        cues: p.cues.map(c => ({ id: c.id, text: c.t, sub: c.t, start: c.start, end: c.end, dur: c.end-c.start, hold: 0 })) })) };
  };
};
if (root.TIMING) root.applyVideoTimeline();
})(typeof window === 'undefined' ? globalThis : window);`;
  return { script, pages, visuals };
}

// Only the host resolves asset IDs to bounded, frozen local files. No path comes from the plan.
export function freezeImages(plan, files, episode) {
  const images = {};
  for (const scene of plan.scenes) {
    if (!scene.image || Object.hasOwn(images, scene.image.assetId)) continue;
    const image = scene.image, file = files?.[image.assetId];
    if (!file || !path.isAbsolute(file.path ?? '') || !['png', 'jpg', 'webp'].includes(file.extension)) throw new Error('Missing frozen scene image.');
    const input = fs.openSync(file.path, 'r');
    let bytes;
    try {
      if (fs.fstatSync(input).size > 15 * 1024 * 1024) throw new Error('Scene image exceeds 15 MB.');
      bytes = fs.readFileSync(input);
    } finally { fs.closeSync(input); }
    const hash = createHash('sha256').update(bytes).digest('hex');
    if (hash !== image.sha256 || hash !== file.sha256) throw new Error('Scene image changed after confirmation.');
    const src = `assets/${hash}.${file.extension}`;
    fs.mkdirSync(path.join(episode, 'assets'), { recursive: true });
    fs.writeFileSync(path.join(episode, src), bytes);
    images[image.assetId] = { src, sha256: hash };
  }
  return images;
}

// FFmpeg seals mdat and appends moov when a fragment finishes. The engine only
// renames fragments after a whole worker group finishes, so size alone is not progress.
function finalizedFragment(file, startedAt) {
  let descriptor;
  try {
    descriptor = fs.openSync(file, 'r');
    const metadata = fs.fstatSync(descriptor);
    if (metadata.mtimeMs < startedAt) return false;
    const length = metadata.size;
    const header = Buffer.alloc(16);
    for (let offset = 0; offset + 8 <= length;) {
      if (fs.readSync(descriptor, header, 0, 8, offset) !== 8) return false;
      let size = header.readUInt32BE(0);
      if (size === 1) {
        if (fs.readSync(descriptor, header, 8, 8, offset + 8) !== 8) return false;
        size = Number(header.readBigUInt64BE(8));
        if (size < 16) return false;
      }
      if (size < 8 || offset + size > length) return false;
      if (header.toString('ascii', 4, 8) === 'moov') return true;
      offset += size;
    }
    return false;
  } catch { return false; }
  finally { if (descriptor !== undefined) fs.closeSync(descriptor); }
}

export function measureRenderProgress(segments, directory, startedAt) {
  let completed = 0, total = 0;
  for (const segment of segments) {
    const frames = segment.b - segment.a;
    total += frames;
    const file = path.join(directory, segment.file);
    if (fs.existsSync(file) || finalizedFragment(file + '.part.mp4', startedAt)) completed += frames;
  }
  return { completed, total };
}

export function measureSubtitleProgress(directory, total, startedAt) {
  try {
    const completed = fs.readdirSync(directory).filter(file => /^cue-\d+\.png$/.test(file) && fs.statSync(path.join(directory, file)).mtimeMs >= startedAt).length;
    return { completed: Math.min(completed, total), total };
  } catch (error) {
    if (error.code !== 'ENOENT') throw error;
    return { completed: 0, total };
  }
}

export function encodingProgress(total, report) {
  let completed = 0, speed;
  return line => {
    const [key, raw] = line.split('=');
    const value = raw?.trim();
    if (key === 'out_time_us' && Number.isFinite(Number(value))) completed = Math.min(total, Math.max(completed, Number(value) / 1e6));
    if (key === 'speed') {
      const measured = /^(\d+(?:\.\d+)?)x$/.exec(value ?? '');
      speed = measured && Number(measured[1]) > 0 ? Number(measured[1]) : undefined;
    }
    if (key === 'progress') report({ completed, total, unit: 'seconds', speed });
  };
}

export function writeProgress(file, progress) {
  const temporary = file + '.tmp';
  fs.writeFileSync(temporary, JSON.stringify(progress));
  try { fs.renameSync(temporary, file); }
  catch (error) {
    // A Windows sharing conflict may hold the previous snapshot; let the next report replace it.
    if (process.platform !== 'win32' || !['EPERM', 'EBUSY'].includes(error.code)) throw error;
  }
}

function run(executable, args, cwd, onLine) {
  return new Promise((resolve, reject) => {
    const child = spawn(executable, args, { cwd, stdio: onLine ? ['ignore', 'pipe', 'inherit'] : 'inherit', windowsHide: true });
    if (onLine) {
      createInterface({ input: child.stdout }).on('line', onLine);
    }
    child.on('error', reject);
    child.on('close', code => code === 0 ? resolve() : reject(new Error(`Render process exited ${code}`)));
  });
}

async function main(inputFile, outputFile) {
  inputFile = path.resolve(inputFile);
  outputFile = path.resolve(outputFile);
  if (process.argv[4]) {
    const deadline = Date.now() + 10000;
    while (!fs.existsSync(process.argv[4])) {
      if (Date.now() > deadline) throw new Error('Host process gate timed out.');
      await new Promise(resolve => setTimeout(resolve, 50));
    }
  }
  assertNodeRuntime();
  const input = JSON.parse(fs.readFileSync(inputFile, 'utf8'));
  const engine = path.resolve(input.engineRoot);
  const pkg = JSON.parse(fs.readFileSync(path.join(engine, 'package.json'), 'utf8'));
  if (pkg.name !== '@frametell/engine' || pkg.version !== '0.7.3') throw new Error('Select the tested Frametell engine 0.7.3 directory (whiteboard-teacher-video/engine).');
  const { FFMPEG, probe, findChrome } = await import(pathToFileURL(path.join(engine, 'tools/bins.mjs')));
  if (!fs.existsSync(FFMPEG) || !findChrome()) throw new Error('FFmpeg or Chrome/Edge is missing from the independent engine installation.');
  const metadata = probe(input.audioPath);
  if (!(metadata.duration > 0 && metadata.duration <= 7200)) throw new Error('Audio must be readable and at most two hours.');
  if (input.action === 'probe') {
    fs.writeFileSync(outputFile, JSON.stringify({ duration: metadata.duration, engineVersion: pkg.version }), { flag: 'wx' });
    return;
  }
  const plan = input.plan;
  const range = input.action === 'preview' ? input.window : undefined;
  if (input.action === 'preview' && (!range || typeof input.signature !== 'string')) throw new Error('A preview requires its source window and snapshot signature.');
  const duration = range ? range.end - range.start : plan.duration;
  const progressFile = path.join(path.dirname(inputFile), 'progress.json');
  const report = (phase, counts = {}) => {
    writeProgress(progressFile, { stage: range ? 'previewing' : 'rendering', phase, updatedAt: Date.now(), ...counts });
  };
  const encode = async (phase, args) => {
    report(phase, { completed: 0, total: duration, unit: 'seconds' });
    await run(FFMPEG, ['-progress', 'pipe:1', '-stats_period', '1', '-nostats', ...args], episode, encodingProgress(duration, counts => report(phase, counts)));
    report(phase, { completed: duration, total: duration, unit: 'seconds' });
  };
  report('checking_layout');
  if (Math.abs(metadata.duration - plan.duration) > 0.15) throw new Error('The audio duration changed. Create a new video task.');
  const key = createHash('sha256').update(JSON.stringify({ plan, range })).update(engine).update(fs.readFileSync(fileURLToPath(import.meta.url))).update(visuals).update(JSON.stringify(catalog)).digest('hex').slice(0, 24);
  const episode = input.cacheRoot ? path.join(path.resolve(input.cacheRoot), key) : path.join(path.dirname(inputFile), 'episode');
  const fresh = !fs.existsSync(episode);
  if (fresh) fs.mkdirSync(path.join(episode, 'pages'), { recursive: true });
  const { linkProject } = await import(pathToFileURL(path.join(engine, 'tools/project.mjs')));
  if (fresh) linkProject(episode);
  else if (fs.realpathSync(path.join(episode, 'engine')) !== fs.realpathSync(engine)) throw new Error('The render cache belongs to a different engine.');
  const images = freezeImages(plan, input.images, episode);
  const result = buildEpisode(plan, range, images);
  fs.writeFileSync(path.join(episode, 'script.js'), result.script);
  fs.writeFileSync(path.join(episode, 'timeline.js'), 'window.applyVideoTimeline();');
  fs.writeFileSync(path.join(episode, 'illustrations.js'), result.visuals);
  // Chromium cannot read some Windows file URLs through deeply nested junctions.
  const modulesUrl = pathToFileURL(fs.realpathSync(path.join(episode, 'engine_modules')) + path.sep).href.replaceAll('&', '&amp;');
  const bootUrl = pathToFileURL(path.join(engine, 'boot.js')).href.replaceAll('&', '&amp;');
  // Engine readiness covers DOM/fonts, not paint. All capture workers must await the initial paint too.
  fs.writeFileSync(path.join(episode, 'index.html'), `<!doctype html><html lang="zh-CN"><head><meta charset="utf-8"><meta name="engine-modules" content="${modulesUrl}"><script src="script.js"></script><script src="${bootUrl}"></script>
<script>window.__ready = window.__ready.then(async duration => {
  await Promise.all([...new Set(SCRIPT.PAGES.filter(p => p.image).map(p => p.image.src))].map(async src => {
    const image = new Image(); image.src = src;
    try { await image.decode(); } catch { throw new Error('Scene image failed to decode: ' + src); }
    if (!image.naturalWidth || !image.naturalHeight) throw new Error('Scene image is empty: ' + src);
  }));
  return new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(() => resolve(duration))));
});</script>
</head><body></body></html>`);
  for (const page of result.pages) fs.writeFileSync(path.join(episode, 'pages', `${page.id}.js`), `ENGINE.visual(${JSON.stringify(page.id)}, P => EPISODE.draw(P));`);
  process.chdir(episode);
  const { openStage, loadEpisode, closeStage } = await import(pathToFileURL(path.join(engine, 'tools/lib.mjs')));
  const { lintPages } = await import(pathToFileURL(path.join(engine, 'tools/checks.mjs')));
  const stage = await openStage('render');
  const thumbnailScenes = [];
  try {
    const { L, META } = loadEpisode();
    L.meta = META;
    const issues = await lintPages(stage.page, { L, pages: L.pages, determinism: true });
    fs.writeFileSync(path.join(path.dirname(inputFile), 'layout-check.json'), JSON.stringify(issues));
    const errors = issues.filter(issue => issue.level === 'error');
    if (errors.length) throw new Error(`Layout check failed: ${errors.slice(0, 3).map(issue => issue.msg).join('; ')}`);
    // The engine otherwise inserts a future "cover" frame at t=0. Use the actual first frame.
    await stage.page.evaluate(() => globalThis.ENGINE.renderAt(0));
    const cover = `cover-${key.slice(0, 12)}.png`;
    await stage.page.screenshot({ path: path.join(episode, cover) });
    fs.writeFileSync(path.join(episode, 'cover.json'), JSON.stringify({ image: cover }));
    if (range) {
      report('rendering_thumbnails');
      for (const page of result.pages.slice(0, 12)) {
        await stage.page.evaluate(time => globalThis.ENGINE.renderAt(time), Math.max(page.start, page.end - 1 / 24));
        await stage.page.screenshot({ path: path.join(path.dirname(inputFile), `scene-${page.sourceIndex}.png`) });
        thumbnailScenes.push(page.sourceIndex);
      }
    }
  } finally { await closeStage(stage.browser); }
  const renderArgs = [path.join(engine, 'tools/render.mjs'), '--fps', '24', '--workers', '2', '--name', 'film'];
  let planJson = '';
  await run(process.execPath, [...renderArgs, '--plan'], episode, line => {
    if (line.startsWith('PLAN ')) planJson = line.slice(5);
    else console.log(line);
  });
  const { segments } = JSON.parse(planJson);
  const startedAt = Date.now();
  const updateFrames = () => report('rendering_frames', { ...measureRenderProgress(segments, path.join(episode, 'out/chunks'), startedAt), unit: 'frames' });
  updateFrames();
  const clock = setInterval(updateFrames, 1000);
  try { await run(process.execPath, [...renderArgs, '--skip-subs'], episode); }
  finally { clearInterval(clock); }
  const subtitleDirectory = path.join(episode, 'out/subs');
  const cueCount = result.pages.reduce((total, page) => total + page.cues.length, 0);
  const subtitleStartedAt = Date.now();
  report('preparing_subtitles', { completed: 0, total: cueCount, unit: 'cues' });
  const subtitleClock = setInterval(() => report('preparing_subtitles', { ...measureSubtitleProgress(subtitleDirectory, cueCount, subtitleStartedAt), unit: 'cues' }), 1000);
  // Reuse the engine's exact subtitle images; own the encode invocation to read FFmpeg's progress stream.
  try { await run(process.execPath, [path.join(engine, 'tools/subs.mjs'), '0', String(duration)], episode); }
  finally { clearInterval(subtitleClock); }
  const subtitled = path.join(episode, 'out/film-有字幕.mp4');
  await encode('rendering_subtitles', ['-nostdin', '-y', '-loglevel', 'error', '-i', path.join(episode, 'out/film-无字幕.mp4'), '-f', 'concat', '-safe', '0', '-i', path.join(subtitleDirectory, 'list.txt'),
    '-filter_complex', '[1:v]fps=24,format=rgba[s];[0:v][s]overlay=0:0:format=auto:shortest=1,format=yuv420p[v]',
    '-map', '[v]', '-map', '0:a?', '-c:a', 'copy', '-c:v', 'libx264', '-preset', 'veryfast', '-crf', '16', '-r', '24', '-movflags', '+faststart', subtitled]);
  const video = path.join(path.dirname(inputFile), 'video.mp4');
  // Render without the engine's voice track: its mixer normalizes speech. Keep the user's original track instead.
  await encode('muxing_audio', ['-nostdin', '-n', '-loglevel', 'error', '-protocol_whitelist', 'file,pipe', '-i', subtitled, ...(range ? ['-ss', String(range.start), '-t', String(duration)] : []), '-protocol_whitelist', 'file,pipe', '-i', input.audioPath, '-map', '0:v:0', '-map', '1:a:0', '-c:v', 'copy', '-c:a', 'aac', '-b:a', '192k', '-movflags', '+faststart', video]);
  report('checking_video');
  const output = probe(video);
  if (output.width !== 1920 || output.height !== 1080 || Math.abs(output.duration - duration) > 0.2) throw new Error('Rendered video metadata does not match the audio.');
  if (range) output.preview = { ...range, signature: input.signature, scenes: thumbnailScenes };
  fs.writeFileSync(outputFile, JSON.stringify(output), { flag: 'wx' });
}

if (process.argv[1] && import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href) {
  main(process.argv[2], process.argv[3]).catch(error => {
    console.error(error.message);
    // A failed openStage can leave browser handles alive. Exit so the host closes the process job.
    process.exit(1);
  });
}
