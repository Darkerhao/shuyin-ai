import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { createHash } from 'node:crypto';
import { spawn } from 'node:child_process';
import { createInterface } from 'node:readline';

// All executable drawing code is owned here. Model output only supplies JSON data.
export function buildEpisode(plan) {
  const pages = plan.scenes.map((scene, index) => ({
    id: `p${String(index + 1).padStart(4, '0')}`, title: scene.title, scene,
    start: index === 0 ? 0 : plan.cues[scene.first].start,
    end: index + 1 === plan.scenes.length ? plan.duration : plan.cues[plan.scenes[index + 1].first].start,
    cues: plan.cues.slice(scene.first, scene.last + 1).map(c => ({ t: c.text, start: c.start, end: c.end })),
  }));
  const data = { META: { title: plan.title, name: 'film', lang: 'en', style: 'study', extras: ['timeline.js'], fadeOut: 0, pageMotion: false }, PAGES: pages, duration: plan.duration };
  const script = `(function(root) {
root.SCRIPT = ${JSON.stringify(data)};
root.applyVideoTimeline = function() {
  root.TIMING.layout = function() {
    return { duration: root.SCRIPT.duration, gap: 0,
      pages: root.SCRIPT.PAGES.map((p, index) => ({ ...p, index, speechEnd: p.cues.at(-1).end,
        cues: p.cues.map(c => ({ text: c.t, sub: c.t, start: c.start, end: c.end, dur: c.end-c.start, hold: 0 })) })) };
  };
};
if (root.TIMING) root.applyVideoTimeline();
})(typeof window === 'undefined' ? globalThis : window);`;
  return { script, pages };
}

const drawing = `
const scene = SCRIPT.PAGES.find(p => p.id === ID).scene;
ENGINE.visual(ID, P => {
  P.text(scene.title, { x: 960, y: 215, w: 1600, align: 'c', size: 72, weight: 700, at: 0, cps: 18 });
  P.underline(760, 320, 400, { color: 'orange', at: 0.3 });
  const count = scene.labels.length;
  const when = i => P.c(Math.min(i, scene.last - scene.first), 0.2);
  if (scene.kind === 'book') {
    P.box(630, 390, 660, 370, { color: 'ink', fill: 'blueF', at: 0.2 });
    P.line(695, 390, 695, 760, { color: 'blue', at: 0.5 });
    P.text(scene.labels[0], { x: 990, y: 490, w: 530, align: 'c', size: 54, at: when(0), cps: 8 });
    P.text(scene.labels.slice(1).join(' / '), { x: 990, y: 640, w: 530, align: 'c', size: 34, color: 'soft', at: when(1) });
  } else if (scene.kind === 'quote') {
    P.line(300, 420, 300, 760, { color: 'orange', sw: 8, at: 0.3 });
    scene.labels.forEach((text, i) => P.text(text, { x: 1030, y: 440 + i * 95, w: 1240, align: 'c', size: 60, at: when(i), cps: 10 }));
  } else {
    const step = 1500 / count;
    scene.labels.forEach((text, i) => {
      const x = 210 + step * (i + 0.5), at = when(i);
      if (scene.kind === 'timeline') {
        P.line(210, 580, 1710, 580, { color: 'soft', at: 0.2 });
        P.circle(x, 580, 44, { color: 'orange', fill: 'orangeF', at });
      } else {
        P.box(x - step * 0.43, 420, step * 0.86, 260, { color: i % 2 ? 'orange' : 'blue', fill: i % 2 ? 'orangeF' : 'blueF', at });
        if (i && scene.kind === 'flow') P.arrow(x - step * 0.56, 550, x - step * 0.45, 550, { color: 'ink', at });
      }
      P.text(text, { x, y: scene.kind === 'timeline' ? 650 : 505, w: step * 0.8, align: 'c', size: 44, at, cps: 10 });
      P.text(String(i + 1).padStart(2, '0'), { x, y: 365, w: 100, align: 'c', size: 30, color: 'muted', at });
    });
  }
});`;

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

function run(executable, args, cwd, onLine) {
  return new Promise((resolve, reject) => {
    const child = spawn(executable, args, { cwd, stdio: onLine ? ['ignore', 'pipe', 'inherit'] : 'inherit', windowsHide: true });
    if (onLine) {
      child.stdout.on('data', chunk => process.stdout.write(chunk));
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
  const progressFile = path.join(path.dirname(inputFile), 'progress.json');
  const report = (phase, counts = {}) => {
    const temporary = progressFile + '.tmp';
    fs.writeFileSync(temporary, JSON.stringify({ stage: 'rendering', phase, ...counts }));
    fs.renameSync(temporary, progressFile);
  };
  report('checking_layout');
  if (Math.abs(metadata.duration - plan.duration) > 0.15) throw new Error('The audio duration changed. Create a new video task.');
  const key = createHash('sha256').update(JSON.stringify(plan)).update(engine).update(fs.readFileSync(fileURLToPath(import.meta.url))).digest('hex').slice(0, 24);
  const episode = input.cacheRoot ? path.join(path.resolve(input.cacheRoot), key) : path.join(path.dirname(inputFile), 'episode');
  const fresh = !fs.existsSync(episode);
  if (fresh) fs.mkdirSync(path.join(episode, 'pages'), { recursive: true });
  const { linkProject } = await import(pathToFileURL(path.join(engine, 'tools/project.mjs')));
  if (fresh) linkProject(episode);
  else if (fs.realpathSync(path.join(episode, 'engine')) !== fs.realpathSync(engine)) throw new Error('The render cache belongs to a different engine.');
  const result = buildEpisode(plan);
  fs.writeFileSync(path.join(episode, 'script.js'), result.script);
  fs.writeFileSync(path.join(episode, 'timeline.js'), 'window.applyVideoTimeline();');
  // Chromium cannot read some Windows file URLs through deeply nested junctions.
  const modulesUrl = pathToFileURL(fs.realpathSync(path.join(episode, 'engine_modules')) + path.sep).href.replaceAll('&', '&amp;');
  const bootUrl = pathToFileURL(path.join(engine, 'boot.js')).href.replaceAll('&', '&amp;');
  fs.writeFileSync(path.join(episode, 'index.html'), `<!doctype html><html lang="zh-CN"><head><meta charset="utf-8"><meta name="engine-modules" content="${modulesUrl}"><script src="script.js"></script><script src="${bootUrl}"></script></head><body></body></html>`);
  for (const page of result.pages) fs.writeFileSync(path.join(episode, 'pages', `${page.id}.js`), `{ const ID = ${JSON.stringify(page.id)}; ${drawing} }`);
  process.chdir(episode);
  const { openStage, loadEpisode, closeStage } = await import(pathToFileURL(path.join(engine, 'tools/lib.mjs')));
  const { lintPages } = await import(pathToFileURL(path.join(engine, 'tools/checks.mjs')));
  const stage = await openStage('render');
  try {
    const { L, META } = loadEpisode();
    L.meta = META;
    const issues = await lintPages(stage.page, { L, pages: L.pages, determinism: true });
    fs.writeFileSync(path.join(path.dirname(inputFile), 'layout-check.json'), JSON.stringify(issues));
    const errors = issues.filter(issue => issue.level === 'error');
    if (errors.length) throw new Error(`Layout check failed: ${errors.slice(0, 3).map(issue => issue.msg).join('; ')}`);
  } finally { await closeStage(stage.browser); }
  const renderArgs = [path.join(engine, 'tools/render.mjs'), '--fps', '24', '--workers', '2', '--name', 'film'];
  let planJson = '';
  await run(process.execPath, [...renderArgs, '--plan'], episode, line => { if (line.startsWith('PLAN ')) planJson = line.slice(5); });
  const { segments } = JSON.parse(planJson);
  const startedAt = Date.now();
  const updateFrames = () => report('rendering_frames', measureRenderProgress(segments, path.join(episode, 'out/chunks'), startedAt));
  updateFrames();
  const clock = setInterval(updateFrames, 1000);
  try { await run(process.execPath, [...renderArgs, '--skip-subs'], episode); }
  finally { clearInterval(clock); }
  report('rendering_subtitles');
  await run(process.execPath, [...renderArgs, '--skip-video'], episode);
  const video = path.join(path.dirname(inputFile), 'video.mp4');
  // Render without the engine's voice track: its mixer normalizes speech. Keep the user's original track instead.
  report('muxing_audio');
  await run(FFMPEG, ['-nostdin', '-n', '-loglevel', 'error', '-protocol_whitelist', 'file,pipe', '-i', path.join(episode, 'out/film-with-subtitles.mp4'), '-protocol_whitelist', 'file,pipe', '-i', input.audioPath, '-map', '0:v:0', '-map', '1:a:0', '-c:v', 'copy', '-c:a', 'aac', '-b:a', '192k', '-movflags', '+faststart', video], episode);
  report('checking_video');
  const output = probe(video);
  if (output.width !== 1920 || output.height !== 1080 || Math.abs(output.duration - plan.duration) > 0.2) throw new Error('Rendered video metadata does not match the audio.');
  fs.writeFileSync(outputFile, JSON.stringify(output), { flag: 'wx' });
}

if (process.argv[1] && import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href) {
  main(process.argv[2], process.argv[3]).catch(error => {
    console.error(error.message);
    // A failed openStage can leave browser handles alive. Exit so the host closes the process job.
    process.exit(1);
  });
}
