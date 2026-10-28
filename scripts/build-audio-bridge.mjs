import { execFileSync } from 'node:child_process'
import { log } from 'node:console'
import { copyFileSync, mkdirSync, readFileSync, statSync } from 'node:fs'
import { join } from 'node:path'
import process from 'node:process'
import { fileURLToPath, pathToFileURL, URL } from 'node:url'

const project = fileURLToPath(new URL('..', import.meta.url))

export function parseArgs(argv) {
  const debug = argv.includes('--debug')
  const release = argv.includes('--release') || !debug
  const targetIndex = argv.indexOf('--target')
  const inlineTarget = argv.find(value => value.startsWith('--target='))
  const target = targetIndex >= 0 ? argv[targetIndex + 1] : inlineTarget?.slice('--target='.length)
  return { debug, release: debug ? false : release, target }
}

export function defaultTarget() {
  if (process.env.TAURI_ENV_TARGET_TRIPLE) return process.env.TAURI_ENV_TARGET_TRIPLE
  if (process.env.BUILD_TARGET) return process.env.BUILD_TARGET
  if (process.platform === 'darwin') return process.arch === 'arm64' ? 'aarch64-apple-darwin' : 'x86_64-apple-darwin'
  if (process.platform === 'linux') return 'x86_64-unknown-linux-gnu'
  if (process.arch === 'arm64') return 'aarch64-pc-windows-msvc'
  return 'x86_64-pc-windows-msvc'
}

export function sidecarName(target) {
  return `browser-bridge-${target}.exe`
}

export function buildAudioBridge({ debug = false, target = defaultTarget(), cwd = project, run = execFileSync } = {}) {
  if (!target.includes('windows')) return { skipped: true, target }
  const profile = debug ? 'debug' : 'release'
  const manifest = join(cwd, 'src-tauri/browser-bridge/Cargo.toml')
  const args = ['build', '--manifest-path', manifest, '--locked', '--target', target]
  if (!debug) args.push('--release')
  run(process.env.CARGO || 'cargo', args, { cwd, stdio: 'inherit' })
  const source = join(cwd, 'src-tauri/browser-bridge/target', target, profile, 'browser-bridge.exe')
  const binaries = join(cwd, 'src-tauri/binaries')
  mkdirSync(binaries, { recursive: true })
  const destination = join(binaries, sidecarName(target))
  copyFileSync(source, destination)
  if (!statSync(destination).isFile() || statSync(destination).size === 0) {
    throw new Error(`Native Messaging helper 产物为空：${destination}`)
  }
  const signature = readFileSync(destination).subarray(0, 2).toString('ascii')
  if (signature !== 'MZ') throw new Error(`Native Messaging helper 不是有效的 Windows PE：${destination}`)
  return { skipped: false, target, profile, destination }
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const options = parseArgs(process.argv.slice(2))
  const result = buildAudioBridge({ debug: options.debug, target: options.target || defaultTarget() })
  log(JSON.stringify(result))
}