import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { mkdirSync, mkdtempSync, readFileSync, rmSync, statSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import process from 'node:process'
import test from 'node:test'
import { buildAudioBridge, parseArgs, sidecarName } from '../scripts/build-audio-bridge.mjs'

const project = process.cwd()
const windowsHosts = [
  'com.shuyin.desktop.audio',
  'com.shuyin.desktop.audio.dev',
]

function readJson(path) {
  return JSON.parse(readFileSync(join(project, path), 'utf8'))
}

function fixture(t) {
  const directory = mkdtempSync(join(tmpdir(), 'shuyin-audio-package-'))
  t.after(() => rmSync(directory, { recursive: true, force: true }))
  return directory
}

test('desktop commands prepare the helper before Vite and Tauri builds', () => {
  const tauri = readJson('src-tauri/tauri.conf.json')
  const packageJson = readJson('package.json')
  assert.equal(tauri.build.beforeDevCommand, 'npm run audio:bridge:dev && npm run dev')
  assert.equal(tauri.build.beforeBuildCommand, 'npm run build && npm run audio:bridge:release')
  assert.equal(packageJson.scripts['audio:bridge:dev'], 'node scripts/build-audio-bridge.mjs --debug')
  assert.equal(packageJson.scripts['audio:bridge:release'], 'node scripts/build-audio-bridge.mjs --release')
  assert.match(packageJson.scripts['test:release'], /tests\/audio-package\.test\.mjs/)
  assert.match(packageJson.scripts['test:release'], /tests\/audio-extension\.test\.mjs/)
})

test('Windows helper is prepared before application Rust checks', () => {
  const workflow = readFileSync(join(project, '.github/workflows/release.yml'), 'utf8')
  const helperStep = workflow.indexOf('- name: Test and prepare Windows Native Messaging helper')
  const rustChecksStep = workflow.indexOf('- name: Rust checks and database tests on each platform')
  assert.notEqual(helperStep, -1)
  assert.notEqual(rustChecksStep, -1)
  assert.ok(helperStep < rustChecksStep)
})

test('Native Messaging sidecar and extension resources are Windows-only', () => {
  const base = readJson('src-tauri/tauri.conf.json')
  const windows = readJson('src-tauri/tauri.windows.conf.json')
  assert.equal(base.bundle.externalBin, undefined)
  assert.deepEqual(windows.bundle.targets, ['nsis'])
  assert.deepEqual(windows.bundle.externalBin, ['binaries/browser-bridge'])
  assert.equal(windows.bundle.resources['../browser-extension/'], 'browser-extension/')
  assert.equal(windows.bundle.windows.nsis.installerHooks, 'windows/installer-hooks.nsh')
  assert.ok(statSync(join(project, 'browser-extension/manifest.json')).isFile())
  assert.ok(statSync(join(project, 'browser-extension/background.js')).isFile())
})

test('extension key, fixed ID and native host names stay aligned', () => {
  const manifest = readJson('browser-extension/manifest.json')
  const identity = readJson('browser-extension/identity.json')
  const digest = createHash('sha256').update(manifest.key, 'base64').digest('hex').slice(0, 32)
  const extensionId = [...digest].map(character => String.fromCharCode(97 + Number.parseInt(character, 16))).join('')
  assert.equal(identity.extensionId, extensionId)
  assert.equal(identity.publicKey, manifest.key)
  const background = readFileSync(join(project, 'browser-extension/background.js'), 'utf8')
  for (const host of windowsHosts) assert.ok(background.includes(host))
})

test('uninstall hook preserves updates and removes only this product host registration', () => {
  const hook = readFileSync(join(project, 'src-tauri/windows/installer-hooks.nsh'), 'utf8')
  assert.match(hook, /!macro NSIS_HOOK_POSTUNINSTALL/)
  assert.doesNotMatch(hook, /!macro NSIS_HOOK_PREUNINSTALL/)
  assert.match(hook, /\$UpdateMode <> 1/)
  for (const host of windowsHosts) assert.ok(hook.includes(`NativeMessagingHosts\\${host}`))
  assert.doesNotMatch(hook, /DeleteRegKey HKCU "Software\\Google\\Chrome\\NativeMessagingHosts"\s*$/m)
})

test('helper build arguments and sidecar names stay target-specific', t => {
  assert.deepEqual(parseArgs(['--debug', '--target', 'x86_64-pc-windows-msvc']), {
    debug: true,
    release: false,
    target: 'x86_64-pc-windows-msvc',
  })
  assert.deepEqual(parseArgs(['--target=aarch64-pc-windows-msvc']), {
    debug: false,
    release: true,
    target: 'aarch64-pc-windows-msvc',
  })
  assert.equal(sidecarName('x86_64-pc-windows-msvc'), 'browser-bridge-x86_64-pc-windows-msvc.exe')

  const directory = fixture(t)
  const target = 'x86_64-pc-windows-msvc'
  const outputDirectory = join(directory, 'src-tauri/browser-bridge/target', target, 'release')
  mkdirSync(outputDirectory, { recursive: true })
  writeFileSync(join(outputDirectory, 'browser-bridge.exe'), 'MZ test helper')
  const calls = []
  const result = buildAudioBridge({
    cwd: directory,
    target,
    run: (command, args, options) => calls.push({ command, args, options }),
  })
  assert.deepEqual(calls[0].args, [
    'build',
    '--manifest-path',
    join(directory, 'src-tauri/browser-bridge/Cargo.toml'),
    '--locked',
    '--target',
    target,
    '--release',
  ])
  assert.equal(result.profile, 'release')
  assert.equal(readFileSync(result.destination, 'utf8'), 'MZ test helper')

  const debugDirectory = join(directory, 'src-tauri/browser-bridge/target', target, 'debug')
  mkdirSync(debugDirectory, { recursive: true })
  writeFileSync(join(debugDirectory, 'browser-bridge.exe'), 'MZ debug helper')
  calls.length = 0
  const debugResult = buildAudioBridge({
    cwd: directory,
    target,
    debug: true,
    run: (command, args, options) => calls.push({ command, args, options }),
  })
  assert.ok(!calls[0].args.includes('--release'))
  assert.equal(debugResult.profile, 'debug')
  assert.equal(readFileSync(debugResult.destination, 'utf8'), 'MZ debug helper')
})

test('non-Windows targets are a no-op and never invoke cargo', () => {
  let called = false
  const result = buildAudioBridge({
    target: 'aarch64-apple-darwin',
    run: () => { called = true },
  })
  assert.deepEqual(result, { skipped: true, target: 'aarch64-apple-darwin' })
  assert.equal(called, false)
})
