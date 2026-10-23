import { execFileSync } from 'node:child_process'
import { createHash } from 'node:crypto'
import { log } from 'node:console'
import { chmodSync, copyFileSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'
import process from 'node:process'
import { pathToFileURL } from 'node:url'

const formats = {
  'windows-x64': ['.exe', '.zip'],
  'macos-arm64': ['.dmg', '.zip'],
  'macos-x64': ['.dmg', '.zip'],
  'linux-x64': ['.deb', '.rpm', '.AppImage', '.tar.gz'],
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
  const copy = (subdir, extension) => copyFileSync(singleBundle(join(root, 'bundle', subdir), extension), join(output, `${prefix}${extension}`))
  if (platform === 'windows-x64') {
    copy('nsis', '.exe')
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
    // ditto preserves the app bundle layout, symbolic links and executable permissions.
    execFileSync('ditto', ['-c', '-k', '--sequesterRsrc', '--keepParent',
      singleBundle(join(root, 'bundle/macos'), '.app'), join(output, `${prefix}.zip`)])
  } else {
    copy('deb', '.deb')
    copy('rpm', '.rpm')
    copy('appimage', '.AppImage')
    chmodSync(join(output, `${prefix}.AppImage`), 0o755)
    // Archive the bundled AppImage, not a raw executable missing its shared libraries.
    // GNU tar treats a colon in an archive path (such as C:) as a remote host.
    execFileSync('tar', ['-czf', `${prefix}.tar.gz`, `${prefix}.AppImage`], { cwd: output })
  }
  for (const name of names) nonEmpty(join(output, name))
  return names
}

export function verifyAssets(directory, version) {
  const expected = assetNames(version)
  const actual = readdirSync(directory).filter(name => name !== 'SHA256SUMS.txt').sort()
  if (JSON.stringify(actual) !== JSON.stringify(expected)) throw new Error('发布产物不完整、版本不一致或包含额外文件。')
  const checksums = expected.map(name => {
    const file = join(directory, name)
    nonEmpty(file)
    return `${createHash('sha256').update(readFileSync(file)).digest('hex')}  ${name}\n`
  }).join('')
  writeFileSync(join(directory, 'SHA256SUMS.txt'), checksums)
  return [...expected, 'SHA256SUMS.txt']
}

export function publishAssets(directory, version, commit, run = (args) => execFileSync('gh', args, { encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] })) {
  const files = verifyAssets(directory, version)
  if (!/^[a-f0-9]{40}$/.test(commit)) throw new Error('无效的发布提交。')
  const tag = `v${version}`
  const existing = JSON.parse(run(['release', 'list', '--limit', '1000', '--json', 'tagName,isDraft']))
    .find(release => release.tagName === tag)
  // Never overwrite assets of an already published version.
  if (existing && !existing.isDraft) return { tag, alreadyPublished: true }
  if (!existing) run(['release', 'create', tag, '--verify-tag', '--target', commit, '--draft', '--title', `书引 AI ${tag}`, '--generate-notes'])
  run(['release', 'upload', tag, ...files.map(name => resolve(directory, name)), '--clobber'])
  run(['release', 'edit', tag, '--draft=false', '--latest'])
  return { tag }
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const [command, ...args] = process.argv.slice(2)
  let result
  if (command === 'collect') result = collectAssets(process.cwd(), ...args)
  else if (command === 'verify') result = verifyAssets(...args)
  else if (command === 'publish') result = publishAssets(...args)
  else throw new Error('用法：release-assets.mjs collect <平台> <target> <目录> | verify <目录> <版本> | publish <目录> <版本> <提交>')
  log(JSON.stringify(result))
}
