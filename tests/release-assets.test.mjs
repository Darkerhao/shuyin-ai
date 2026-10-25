import assert from 'node:assert/strict'
import { execFileSync } from 'node:child_process'
import { createHash } from 'node:crypto'
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import process from 'node:process'
import test from 'node:test'
import { URL } from 'node:url'
import { assetNames, collectAssets, publishAssets, verifyAssets, releaseNotes } from '../scripts/release-assets.mjs'

const notes = '- 新增应用内更新\n- 修复封面导出'

function fixture(t) {
  const directory = mkdtempSync(join(tmpdir(), 'shuyin-assets-'))
  t.after(() => rmSync(directory, { recursive: true, force: true }))
  return directory
}

function completeAssets(t) {
  const directory = fixture(t)
  for (const name of assetNames('1.2.3')) writeFileSync(join(directory, name), `test bytes: ${name}`)
  return directory
}

function buildFixture(t, bundles) {
  const directory = fixture(t)
  const root = join(directory, 'src-tauri/target/test-target/release')
  mkdirSync(root, { recursive: true })
  writeFileSync(join(directory, 'package.json'), JSON.stringify({ version: '1.2.3' }))
  for (const [subdir, filename] of bundles) {
    mkdirSync(join(root, 'bundle', subdir), { recursive: true })
    writeFileSync(join(root, 'bundle', subdir, filename), `test package: ${filename}`)
  }
  return { directory, root, output: join(directory, 'release assets') }
}

test('release requires installers, archives and updater signatures for four platforms', () => {
  assert.deepEqual(assetNames('1.2.3'), [
    'shuyin-ai-1.2.3-linux-x64.AppImage', 'shuyin-ai-1.2.3-linux-x64.AppImage.sig', 'shuyin-ai-1.2.3-linux-x64.deb',
    'shuyin-ai-1.2.3-linux-x64.rpm', 'shuyin-ai-1.2.3-linux-x64.tar.gz',
    'shuyin-ai-1.2.3-macos-arm64.app.tar.gz', 'shuyin-ai-1.2.3-macos-arm64.app.tar.gz.sig',
    'shuyin-ai-1.2.3-macos-arm64.dmg', 'shuyin-ai-1.2.3-macos-arm64.zip',
    'shuyin-ai-1.2.3-macos-x64.app.tar.gz', 'shuyin-ai-1.2.3-macos-x64.app.tar.gz.sig',
    'shuyin-ai-1.2.3-macos-x64.dmg', 'shuyin-ai-1.2.3-macos-x64.zip',
    'shuyin-ai-1.2.3-windows-x64.exe', 'shuyin-ai-1.2.3-windows-x64.exe.sig', 'shuyin-ai-1.2.3-windows-x64.zip',
  ])
})

test('checksums cover every installer and are stable when verification is retried', t => {
  const directory = completeAssets(t)
  assert.equal(verifyAssets(directory, '1.2.3', notes).length, 18)
  const first = readFileSync(join(directory, 'SHA256SUMS.txt'), 'utf8')
  const lines = first.trim().split('\n')
  assert.equal(lines.length, 17)
  for (const line of lines) {
    const [checksum, filename] = line.split('  ')
    assert.equal(checksum, createHash('sha256').update(readFileSync(join(directory, filename))).digest('hex'))
  }
  verifyAssets(directory, '1.2.3', notes)
  assert.equal(readFileSync(join(directory, 'SHA256SUMS.txt'), 'utf8'), first)
})

test('missing platform, empty artifact and unexpected files block publication', t => {
  const directory = completeAssets(t)
  const file = join(directory, assetNames('1.2.3')[0])
  rmSync(file)
  assert.throws(() => verifyAssets(directory, '1.2.3', notes), /不完整/)
  writeFileSync(file, '')
  assert.throws(() => verifyAssets(directory, '1.2.3', notes), /为空/)
  writeFileSync(file, 'restored')
  writeFileSync(join(directory, 'release.bundle'), 'unexpected source archive')
  assert.throws(() => verifyAssets(directory, '1.2.3', notes), /额外文件/)
  assert.throws(() => verifyAssets(directory, '9.9.9', notes), /版本不一致/)
})

test('Windows ZIP contains the executable and runtime instructions', { skip: process.platform !== 'win32' }, t => {
  const { directory, root, output } = buildFixture(t, [['nsis', '书引 AI_1.2.3_x64-setup.exe'], ['nsis', '书引 AI_1.2.3_x64-setup.exe.sig']])
  writeFileSync(join(root, 'shuyin-ai.exe'), 'test application binary')
  assert.deepEqual(collectAssets(directory, 'windows-x64', 'test-target', output), assetNames('1.2.3', 'windows-x64'))
  const archive = join(output, 'shuyin-ai-1.2.3-windows-x64.zip')
  assert.equal(readFileSync(archive).subarray(0, 2).toString(), 'PK')
  const contents = execFileSync(join(process.env.SystemRoot, 'System32/tar.exe'), ['-tf', archive], { encoding: 'utf8' })
  assert.match(contents, /shuyin-ai.exe/)
  assert.match(contents, /README.txt/)
})

test('Linux TAR.GZ contains the AppImage instead of an unbundled binary', t => {
  const { directory, output } = buildFixture(t, [
    ['deb', 'Shuyin AI_1.2.3_amd64.deb'], ['rpm', 'Shuyin AI-1.2.3-1.x86_64.rpm'],
    ['appimage', 'Shuyin AI_1.2.3_amd64.AppImage'], ['appimage', 'Shuyin AI_1.2.3_amd64.AppImage.sig'],
  ])
  collectAssets(directory, 'linux-x64', 'test-target', output)
  const archive = 'shuyin-ai-1.2.3-linux-x64.tar.gz'
  const contents = execFileSync('tar', ['-tzf', archive], { cwd: output, encoding: 'utf8' }).trim()
  assert.equal(contents, 'shuyin-ai-1.2.3-linux-x64.AppImage')
  assert.deepEqual(execFileSync('tar', ['-xOf', archive, contents], { cwd: output }), readFileSync(join(output, contents)))
})

test('macOS ZIP preserves the application bundle', { skip: process.platform !== 'darwin' }, t => {
  const { directory, root, output } = buildFixture(t, [['dmg', '书引 AI_1.2.3_aarch64.dmg'], ['macos', '书引 AI.app.tar.gz'], ['macos', '书引 AI.app.tar.gz.sig']])
  const executable = join(root, 'bundle/macos/书引 AI.app/Contents/MacOS')
  mkdirSync(executable, { recursive: true })
  writeFileSync(join(executable, 'shuyin-ai'), 'test executable', { mode: 0o755 })
  collectAssets(directory, 'macos-arm64', 'test-target', output)
  const contents = execFileSync('unzip', ['-l', join(output, 'shuyin-ai-1.2.3-macos-arm64.zip')], { encoding: 'utf8' })
  assert.match(contents, /Contents\/MacOS\/shuyin-ai/)
})

test('ambiguous installer selection fails instead of uploading an older build', t => {
  const { directory, root, output } = buildFixture(t, [['nsis', 'old.exe']])
  writeFileSync(join(root, 'bundle/nsis/new.exe'), 'new')
  assert.throws(() => collectAssets(directory, 'windows-x64', 'test-target', output), /有且仅有一个/)
})

test('an unrelated signature cannot be paired with the selected Windows installer', t => {
  const { directory, output } = buildFixture(t, [['nsis', 'new.exe'], ['nsis', 'old.exe.sig']])
  assert.throws(() => collectAssets(directory, 'windows-x64', 'test-target', output), /new\.exe\.sig/)
})

test('release stays draft until all assets are uploaded', t => {
  const directory = completeAssets(t)
  const calls = []
  publishAssets(directory, '1.2.3', 'a'.repeat(40), notes, args => {
    calls.push(args)
    if (args[1] === 'create') assert.equal(readFileSync(args.at(-1), 'utf8'), notes)
    return args[1] === 'list' ? '[]' : ''
  })
  assert.deepEqual(calls.map(args => args[1]), ['list', 'create', 'upload', 'edit'])
  assert.ok(calls[1].includes('--draft'))
  assert.ok(calls[1].includes('--verify-tag'))
  assert.equal(calls[2].filter(arg => arg.startsWith(directory)).length, 18)
  assert.ok(calls[3].includes('--draft=false'))
})

test('upload failure leaves a draft and retry resumes without creating another release', t => {
  const directory = completeAssets(t)
  const calls = []
  assert.throws(() => publishAssets(directory, '1.2.3', 'a'.repeat(40), notes, args => {
    calls.push(args[1])
    if (args[1] === 'upload') throw new Error('upload unavailable')
    return args[1] === 'list' ? '[]' : ''
  }), /upload unavailable/)
  assert.ok(!calls.includes('edit'))
  calls.length = 0
  publishAssets(directory, '1.2.3', 'a'.repeat(40), notes, args => {
    calls.push(args[1])
    return args[1] === 'list' ? '[{"tagName":"v1.2.3","isDraft":true}]' : ''
  })
  assert.deepEqual(calls, ['list', 'edit', 'upload', 'edit'])
})

test('existing public release assets are never replaced on retry', t => {
  const directory = completeAssets(t)
  const calls = []
  const result = publishAssets(directory, '1.2.3', 'a'.repeat(40), notes, args => {
    calls.push(args[1])
    return '[{"tagName":"v1.2.3","isDraft":false}]'
  })
  assert.equal(result.alreadyPublished, true)
  assert.deepEqual(calls, ['list'])
})

test('updater manifest maps native targets to signed packages and includes release notes', t => {
  const directory = completeAssets(t)
  verifyAssets(directory, '1.2.3', notes)
  const manifest = JSON.parse(readFileSync(join(directory, 'latest.json'), 'utf8'))
  assert.equal(manifest.version, '1.2.3')
  assert.equal(manifest.notes, notes)
  assert.deepEqual(Object.keys(manifest.platforms).sort(), ['darwin-aarch64', 'darwin-x86_64', 'linux-x86_64', 'windows-x86_64'])
  for (const entry of Object.values(manifest.platforms)) {
    const filename = new URL(entry.url).pathname.split('/').at(-1)
    assert.equal(entry.signature, readFileSync(join(directory, `${filename}.sig`), 'utf8'))
    assert.match(entry.url, /\/releases\/download\/v1\.2\.3\//)
  }
  writeFileSync(join(directory, 'shuyin-ai-1.2.3-windows-x64.exe.sig'), '  \n')
  assert.throws(() => verifyAssets(directory, '1.2.3', notes), /签名为空/)
  assert.throws(() => verifyAssets(directory, '1.2.3', ''), /更新说明/)
})

test('release notes include only changes since the previous release, preserving Chinese and Markdown', t => {
  const directory = fixture(t)
  const git = (...args) => execFileSync('git', args, { cwd: directory, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] })
  git('init')
  git('config', 'user.name', 'Release test')
  git('config', 'user.email', 'release-test@example.invalid')
  git('config', 'commit.gpgsign', 'false')
  git('commit', '--allow-empty', '-m', 'chore: old changes')
  git('tag', 'v1.0.0')
  git('commit', '--allow-empty', '-m', 'feat(updates): 自动更新与 **更新说明**')
  git('commit', '--allow-empty', '-m', 'fix: 修复下载')
  git('commit', '--allow-empty', '-m', 'chore(release): v1.1.0')
  assert.equal(releaseNotes(directory), '- 自动更新与 **更新说明**\n- 修复下载')
})
