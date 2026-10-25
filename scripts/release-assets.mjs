import { execFileSync } from 'node:child_process'
import { createHash } from 'node:crypto'
import { log } from 'node:console'
import { chmodSync, copyFileSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'
import process from 'node:process'
import { pathToFileURL } from 'node:url'

const formats = {
  'windows-x64': ['.exe', '.exe.sig', '.zip'],
  'macos-arm64': ['.dmg', '.zip', '.app.tar.gz', '.app.tar.gz.sig'],
  'macos-x64': ['.dmg', '.zip', '.app.tar.gz', '.app.tar.gz.sig'],
  'linux-x64': ['.deb', '.rpm', '.AppImage', '.AppImage.sig', '.tar.gz'],
}

const updaterPackages = {
  'windows-x86_64': 'windows-x64.exe',
  'darwin-aarch64': 'macos-arm64.app.tar.gz',
  'darwin-x86_64': 'macos-x64.app.tar.gz',
  'linux-x86_64': 'linux-x64.AppImage',
}

export function releaseNotes(cwd, commit = 'HEAD') {
  const git = (...args) => execFileSync('git', args, { cwd, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] }).trim()
  const tags = git('tag', '--merged', `${commit}^`, '--sort=-version:refname').split('\n')
  const previous = tags.find(tag => /^v\d+\.\d+\.\d+$/.test(tag))
  const subjects = git('log', '--reverse', '--no-merges', '--format=%s', previous ? `${previous}..${commit}` : commit)
    .split('\n').filter(subject => subject && !subject.startsWith('chore(release):'))
  return subjects.map(subject => `- ${subject.replace(/^[a-z]+(?:\([^)]*\))?!?:\s*/i, '')}`).join('\n') || '本次发布更新了应用版本。'
}

export function assetNames(version, platform) {
  if (!/^\d+\.\d+\.\d+$/.test(version)) throw new Error('无效的产物版本号。')
  if (platform && !formats[platform]) throw new Error(`未知平台：${platform}`)
  return (platform ? [platform] : Object.keys(formats))
    .flatMap(key => formats[key].map(extension => `shuyin-ai-${version}-${key}${extension}`)).sort()
}

function singleBundle(directory, extension) {
  const matches = readdirSync(directory).filter(name => name.endsWith(extension))
  if (matches.length !== 1) throw new Error(`${directory} 必须有且仅有一个 ${extension} 产物。`)
  return join(directory, matches[0])
}

function nonEmpty(file) {
  if (!statSync(file).isFile() || statSync(file).size === 0) throw new Error(`产物为空或不是文件：${file}`)
}

export function collectAssets(cwd, platform, target, destination) {
  const version = JSON.parse(readFileSync(join(cwd, 'package.json'), 'utf8')).version
  const names = assetNames(version, platform)
  const root = join(cwd, 'src-tauri/target', target, 'release')
  const output = resolve(destination)
  mkdirSync(output, { recursive: true })
  const prefix = `shuyin-ai-${version}-${platform}`
  const copy = (subdir, extension) => {
    const source = singleBundle(join(root, 'bundle', subdir), extension)
    copyFileSync(source, join(output, `${prefix}${extension}`))
    return source
  }
  const copySigned = (subdir, extension) => {
    const source = copy(subdir, extension)
    copyFileSync(`${source}.sig`, join(output, `${prefix}${extension}.sig`))
  }
  if (platform === 'windows-x64') {
    copySigned('nsis', '.exe')
    const staging = mkdtempSync(join(tmpdir(), 'shuyin-zip-'))
    try {
      copyFileSync(join(root, 'shuyin-ai.exe'), join(staging, 'shuyin-ai.exe'))
      writeFileSync(join(staging, 'README.txt'), '书引 AI：解压后运行 shuyin-ai.exe。\n需要 Microsoft Edge WebView2 Runtime。\n数据保存在系统用户数据目录，不是随目录移动的数据便携版。\n')
      // Git Bash may put GNU tar first; Windows' bundled bsdtar supports ZIP creation.
      execFileSync(join(process.env.SystemRoot, 'System32/tar.exe'),
        ['-a', '-cf', join(output, `${prefix}.zip`), '-C', staging, 'shuyin-ai.exe', 'README.txt'])
    } finally {
      rmSync(staging, { recursive: true, force: true })
    }
  } else if (platform.startsWith('macos-')) {
    copy('dmg', '.dmg')
    copySigned('macos', '.app.tar.gz')
    // ditto preserves the app bundle layout, symbolic links and executable permissions.
    execFileSync('ditto', ['-c', '-k', '--sequesterRsrc', '--keepParent',
      singleBundle(join(root, 'bundle/macos'), '.app'), join(output, `${prefix}.zip`)])
  } else {
    copy('deb', '.deb')
    copy('rpm', '.rpm')
    copySigned('appimage', '.AppImage')
    chmodSync(join(output, `${prefix}.AppImage`), 0o755)
    // Archive the bundled AppImage, not a raw executable missing its shared libraries.
    // GNU tar treats a colon in an archive path (such as C:) as a remote host.
    execFileSync('tar', ['-czf', `${prefix}.tar.gz`, `${prefix}.AppImage`], { cwd: output })
  }
  for (const name of names) nonEmpty(join(output, name))
  return names
}

export function verifyAssets(directory, version, notes) {
  if (!notes?.trim()) throw new Error('发布必须提供更新说明。')
  const packages = assetNames(version)
  const actual = readdirSync(directory).filter(name => !['SHA256SUMS.txt', 'latest.json'].includes(name)).sort()
  if (JSON.stringify(actual) !== JSON.stringify(packages)) throw new Error('发布产物不完整、版本不一致或包含额外文件。')
  for (const name of packages) nonEmpty(join(directory, name))
  const platforms = Object.fromEntries(Object.entries(updaterPackages).map(([target, suffix]) => {
    const filename = `shuyin-ai-${version}-${suffix}`
    const signature = readFileSync(join(directory, `${filename}.sig`), 'utf8').trim()
    if (!signature) throw new Error(`更新签名为空：${filename}`)
    return [target, { signature, url: `https://github.com/Darkerhao/shuyin-ai/releases/download/v${version}/${filename}` }]
  }))
  writeFileSync(join(directory, 'latest.json'), `${JSON.stringify({ version, notes, platforms }, null, 2)}\n`)
  const expected = [...packages, 'latest.json']
  const checksums = expected.map(name => {
    const file = join(directory, name)
    nonEmpty(file)
    return `${createHash('sha256').update(readFileSync(file)).digest('hex')}  ${name}\n`
  }).join('')
  writeFileSync(join(directory, 'SHA256SUMS.txt'), checksums)
  return [...expected, 'SHA256SUMS.txt']
}

export function publishAssets(directory, version, commit, notes, run = (args) => execFileSync('gh', args, { encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] })) {
  const files = verifyAssets(directory, version, notes)
  if (!/^[a-f0-9]{40}$/.test(commit)) throw new Error('无效的发布提交。')
  const tag = `v${version}`
  const existing = JSON.parse(run(['release', 'list', '--limit', '1000', '--json', 'tagName,isDraft']))
    .find(release => release.tagName === tag)
  // Never overwrite assets of an already published version.
  if (existing && !existing.isDraft) return { tag, alreadyPublished: true }
  const notesDirectory = mkdtempSync(join(tmpdir(), 'shuyin-release-notes-'))
  const notesFile = join(notesDirectory, 'notes.md')
  try {
    writeFileSync(notesFile, notes)
    if (!existing) run(['release', 'create', tag, '--verify-tag', '--target', commit, '--draft', '--title', `书引 AI ${tag}`, '--notes-file', notesFile])
    else run(['release', 'edit', tag, '--notes-file', notesFile])
  } finally { rmSync(notesDirectory, { recursive: true, force: true }) }
  run(['release', 'upload', tag, ...files.map(name => resolve(directory, name)), '--clobber'])
  run(['release', 'edit', tag, '--draft=false', '--latest'])
  return { tag }
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const [command, ...args] = process.argv.slice(2)
  let result
  if (command === 'collect') result = collectAssets(process.cwd(), ...args)
  else if (command === 'verify') result = verifyAssets(...args, releaseNotes(process.cwd()))
  else if (command === 'publish') result = publishAssets(...args, releaseNotes(process.cwd(), args[2]))
  else if (command === 'notes') { log(releaseNotes(process.cwd())); process.exit(0) }
  else throw new Error('用法：release-assets.mjs collect <平台> <target> <目录> | verify <目录> <版本> | publish <目录> <版本> <提交>')
  log(JSON.stringify(result))
}
