import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';
import { spawnSync } from 'node:child_process';
import { fileURLToPath, pathToFileURL } from 'node:url';

export function assertNodeRuntime(version = process.versions.node, platform = process.platform) {
  const [major, minor, patch] = version.split('.').map(Number);
  // nodejs/node#61108 fixes native rmSync crashes on Windows non-ASCII paths.
  if (platform === 'win32' && (major === 23 || (major === 24 && (minor < 13 || (minor === 13 && patch === 0))) || (major === 25 && minor < 4))) {
    throw new Error(`Node.js ${version} 存在 Windows 中文路径处理缺陷，会导致视频进程崩溃。请在「视频运行环境」的「Node 程序」中选择 Node.js 24.13.1 或更新的 24.x LTS（推荐 24.22.2），保存后重试；已保存字幕和分镜会保留。`);
  }
}

export function enginePackage(engine) {
  const pkg = JSON.parse(fs.readFileSync(path.join(engine, 'package.json'), 'utf8'));
  if (pkg.name !== '@frametell/engine' || pkg.version !== '0.7.3') throw new Error('请选择 Frametell 0.7.3 的 engine 目录。');
  return pkg;
}

export async function inspectRuntime(engine, directory) {
  const items = [];
  const record = async (id, action) => {
    try { items.push({ id, status: 'ok', ...await action() }); }
    catch (error) { items.push({ id, status: 'error', detail: String(error.message), path: '', version: '' }); }
  };
  await record('node', () => {
    assertNodeRuntime();
    if (Number(process.versions.node.split('.')[0]) < 22) throw new Error(`Node.js ${process.versions.node} 过旧，请选择受支持的 Node 24 LTS。`);
    return { version: process.versions.node, path: process.execPath, detail: '版本和中文路径兼容性检查通过。' };
  });
  let bins;
  await record('engine', async () => {
    const pkg = enginePackage(engine);
    for (const file of ['boot.js', 'runtime/engine.js', 'tools/render.mjs', 'tools/project.mjs', 'template/index.html']) {
      if (!fs.statSync(path.join(engine, file)).isFile()) throw new Error(`引擎文件缺失：${file}`);
    }
    bins = await import(pathToFileURL(path.join(engine, 'tools/bins.mjs')));
    return { version: pkg.version, path: engine, detail: '引擎版本和必要文件可读取。' };
  });
  await record('ffmpeg', () => {
    if (!bins) throw new Error('请先修复引擎目录，再检查引擎使用的 FFmpeg。');
    // Match render.mjs: a bare command that the renderer cannot use is not reported as ready.
    if (!fs.existsSync(bins.FFMPEG)) throw new Error('引擎未找到 FFmpeg 文件，请在独立引擎中安装依赖或修正 FFMPEG_PATH。');
    const result = spawnSync(bins.FFMPEG, ['-version'], { encoding: 'utf8', windowsHide: true, timeout: 5000, maxBuffer: 1024 * 1024 });
    if (result.error || result.status !== 0) throw new Error('FFmpeg 无法启动，请重新安装引擎的 ffmpeg-static 依赖。');
    return { version: result.stdout.split('\n')[0].trim(), path: bins.FFMPEG, detail: '引擎实际使用的 FFmpeg 已启动。' };
  });
  let browser;
  await record('browser', async () => {
    if (!bins) throw new Error('请先修复引擎目录，再检查渲染浏览器。');
    const executablePath = bins.findChrome();
    if (!executablePath || !fs.existsSync(executablePath)) throw new Error('未找到可用 Chrome/Edge，请安装浏览器或修正 CHROME_PATH。');
    const { chromium } = createRequire(path.join(engine, 'package.json'))('playwright-core');
    browser = await chromium.launch({ executablePath, headless: true, timeout: 10000 });
    return { version: browser.version(), path: executablePath, detail: '已通过引擎的 Playwright 启动浏览器。' };
  });
  try {
    await record('fonts', async () => {
      if (!browser) throw new Error('浏览器未就绪，修复后才能验证字体加载。');
      const { engineModulesDir } = await import(pathToFileURL(path.join(engine, 'tools/project.mjs')));
      const modules = engineModulesDir();
      const styles = ['lxgw-wenkai-webfont/lxgwwenkai-regular.css', 'lxgw-wenkai-webfont/lxgwwenkai-bold.css', '@fontsource/jetbrains-mono/400.css', '@fontsource/jetbrains-mono/600.css', '@fontsource/caveat/500.css', '@fontsource/caveat/700.css', '@fontsource/fredoka/500.css', '@fontsource/fredoka/600.css'];
      for (const style of styles) if (!fs.existsSync(path.join(modules, style))) throw new Error(`字体样式缺失：${style}；请重新安装引擎依赖。`);
      const file = path.join(directory, 'fonts.html');
      fs.writeFileSync(file, '<meta charset="utf-8">' + styles.map(s => `<link rel="stylesheet" href="${pathToFileURL(path.join(modules, s)).href}">`).join(''), { flag: 'wx' });
      const page = await browser.newPage();
      await page.goto(pathToFileURL(file).href, { timeout: 10000 });
      const loaded = await page.evaluate(async () => {
        const samples = [['400 20px "LXGW WenKai"', '中文书籍'], ['700 20px "LXGW WenKai"', '中文书籍'], ['400 20px "JetBrains Mono"', '123'], ['600 20px "JetBrains Mono"', '123'], ['500 20px "Caveat"', 'Book'], ['700 20px "Caveat"', 'Book'], ['500 20px "Fredoka"', '123'], ['600 20px "Fredoka"', '123']];
        return Promise.all(samples.map(async ([font, text]) => (await document.fonts.load(font, text)).length > 0));
      });
      if (!loaded.every(Boolean)) throw new Error('引擎字体未完整加载，请重新安装字体依赖。');
      return { version: '', path: modules, detail: '霞鹜文楷、Caveat、JetBrains Mono、Fredoka 已在浏览器加载。' };
    });
  } finally { await browser?.close(); }
  return items;
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    if (process.argv[4]) {
      const deadline = Date.now() + 10000;
      while (!fs.existsSync(process.argv[4])) {
        if (Date.now() > deadline) throw new Error('Host process gate timed out.');
        await new Promise(resolve => setTimeout(resolve, 50));
      }
    }
    const input = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
    const result = await inspectRuntime(input.engineRoot, path.dirname(process.argv[3]));
    fs.writeFileSync(process.argv[3], JSON.stringify(result), { flag: 'wx' });
  } catch (error) { console.error(error.message); process.exitCode = 1; }
}
