import assert from 'node:assert/strict'
import { existsSync, readFileSync } from 'node:fs'
import { join } from 'node:path'
import test from 'node:test'

const root = join(import.meta.dirname, '..')
const read = path => readFileSync(join(root, path), 'utf8')
const json = path => JSON.parse(read(path))

test('desktop development and build need only the application', () => {
  const tauri = json('src-tauri/tauri.conf.json')
  const pkg = json('package.json')
  assert.equal(tauri.build.beforeDevCommand, 'npm run dev')
  assert.equal(tauri.build.beforeBuildCommand, 'npm run build')
  assert.equal(pkg.scripts['audio:bridge:dev'], undefined)
  assert.equal(pkg.scripts['audio:bridge:release'], undefined)
  assert.match(pkg.scripts['test:release'], /tests\/audio-page\.test\.mjs/)
})

test('Windows bundles have no sidecar or extension resources', () => {
  for (const path of ['src-tauri/tauri.conf.json', 'src-tauri/tauri.windows.conf.json']) {
    const config = json(path)
    assert.equal(config.bundle.externalBin, undefined)
    assert.equal(config.bundle.resources, undefined)
  }
  assert.equal(json('src-tauri/tauri.windows.conf.json').bundle.windows.nsis.installerHooks, 'windows/installer-hooks.nsh')
  assert.equal(existsSync(join(root, 'scripts/build-audio-bridge.mjs')), false)
  assert.equal(existsSync(join(root, 'browser-extension/manifest.json')), false)
  const workflow = read('.github/workflows/release.yml')
  assert.doesNotMatch(workflow, /cargo .*browser-bridge|build-audio-bridge/)
  assert.match(workflow, /tests\/audio-page\.test\.mjs/)
})

test('remote audio page has no application command permissions', () => {
  const capability = json('src-tauri/capabilities/main.json')
  assert.deepEqual(capability.windows, ['main'])
  assert.equal(capability.remote, undefined)
  const permissions = read('src-tauri/permissions/audio.toml')
  assert.doesNotMatch(permissions, /open_audio_extension_setup/)
})

test('upgrade and uninstall remove only exact owned legacy registrations', () => {
  const hook = read('src-tauri/windows/installer-hooks.nsh')
  assert.match(hook, /!macro NSIS_HOOK_POSTINSTALL/)
  assert.match(hook, /!macro NSIS_HOOK_POSTUNINSTALL/)
  assert.match(hook, /ReadRegStr/)
  assert.match(hook, /\$LOCALAPPDATA\\com.shuyin.desktop\\audio-bridge\\hosts/)
  assert.match(hook, /DeleteRegKey \/ifempty HKCU/)
  assert.doesNotMatch(hook, /RMDir \/r|Chrome\\User Data/)
})
