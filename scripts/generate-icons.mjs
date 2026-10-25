import { spawnSync } from 'node:child_process'
import { log } from 'node:console'
import { copyFileSync, mkdirSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath, URL } from 'node:url'
import { createRequire } from 'node:module'
import process from 'node:process'

const root = fileURLToPath(new URL('../', import.meta.url))
const require = createRequire(import.meta.url)
const cli = join(dirname(require.resolve('@tauri-apps/cli/package.json')), 'tauri.js')
const source = join(root, 'assets', 'app-icon.svg')
const generated = join(root, 'output', 'brand-icons')
const result = spawnSync(process.execPath, [cli, 'icon', source, '--output', generated], { cwd: root, stdio: 'inherit' })
if (result.error) throw result.error
if (result.status !== 0) process.exit(result.status ?? 1)

const icons = join(root, 'src-tauri', 'icons')
mkdirSync(icons, { recursive: true })
for (const file of ['32x32.png', '128x128.png', '128x128@2x.png', 'icon.ico', 'icon.icns']) {
  copyFileSync(join(generated, file), join(icons, file))
}
copyFileSync(source, join(root, 'public', 'favicon.svg'))
log('Updated desktop icons and favicon from assets/app-icon.svg.')
