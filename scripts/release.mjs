import { execFileSync } from 'node:child_process'
import { log } from 'node:console'
import { appendFileSync, readFileSync, writeFileSync } from 'node:fs'
import process from 'node:process'
import { pathToFileURL } from 'node:url'

export const versionFiles = [
  'package.json',
  'package-lock.json',
  'src-tauri/tauri.conf.json',
  'src-tauri/Cargo.toml',
  'src-tauri/Cargo.lock',
]

function git(cwd, ...args) {
  return execFileSync('git', args, { cwd, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] }).trim()
}

function requireClean(cwd) {
  if (git(cwd, 'status', '--porcelain')) throw new Error('工作区必须干净，不能把其他改动带入版本提交。')
}

function fetchBranches(cwd) {
  git(cwd, 'fetch', '--tags', 'origin',
    'refs/heads/main:refs/remotes/origin/main', 'refs/heads/dev:refs/remotes/origin/dev')
}

function isAncestor(cwd, ancestor, descendant) {
  // Validate both refs first so a missing ref is not mistaken for a negative result.
  git(cwd, 'rev-parse', '--verify', ancestor)
  git(cwd, 'rev-parse', '--verify', descendant)
  try {
    git(cwd, 'merge-base', '--is-ancestor', ancestor, descendant)
    return true
  } catch (error) {
    if (error.status === 1) return false
    throw error
  }
}

export function releaseType(messages) {
  let type = 'patch'
  for (const message of messages) {
    const text = message.trim().replace(/\r\n/g, '\n')
    const lines = text.split('\n')
    // GitHub merge commits put the PR title below the generated merge header.
    const title = /^Merge pull request #\d+\b/.test(lines[0])
      ? lines.slice(1).find(line => line.trim()) ?? ''
      : lines[0]
    const prose = text.replace(/^ {0,3}(`{3,}|~{3,})[^\n]*\n[\s\S]*?^ {0,3}\1[ \t]*(?:\n|$)/gm, '')
    if (/^[a-z][\w-]*(?:\([^\r\n)]+\))?!:[ \t]+\S/i.test(title) ||
        /^BREAKING[ -]CHANGE:[ \t]+\S/m.test(prose)) return 'major'
    if (/^feat(?:\([^\r\n)]+\))?:[ \t]+\S/i.test(title)) type = 'minor'
  }
  return type
}

export function nextVersion(version, type) {
  if (!/^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/.test(version)) {
    throw new Error(`仅支持正式语义版本号，当前为 ${version}`)
  }
  const [major, minor, patch] = version.split('.').map(Number)
  if (type === 'major') return `${major + 1}.0.0`
  if (type === 'minor') return `${major}.${minor + 1}.0`
  if (type === 'patch') return `${major}.${minor}.${patch + 1}`
  throw new Error(`未知版本类型：${type}`)
}

// Keep the original formatting and dependency locks; only replace app version fields.
export function updateVersions(cwd, current, next) {
  const fields = [
    [versionFiles[0], /("version"\s*:\s*")([^"]+)(")/],
    [versionFiles[1], /("version"\s*:\s*")([^"]+)(")/,
      /(""\s*:\s*\{[\s\S]*?"version"\s*:\s*")([^"]+)(")/],
    [versionFiles[2], /("version"\s*:\s*")([^"]+)(")/],
    [versionFiles[3], /(\[package\][\s\S]*?\nversion\s*=\s*")([^"]+)(")/],
    [versionFiles[4], /(\[\[package\]\]\r?\nname = "shuyin-ai"\r?\nversion = ")([^"]+)(")/],
  ]
  const changes = fields.map(([file, ...patterns]) => {
    let content = readFileSync(`${cwd}/${file}`, 'utf8')
    for (const pattern of patterns) {
      if (content.match(pattern)?.[2] !== current) throw new Error(`${file} 的应用版本与 ${current} 不一致。`)
      content = content.replace(pattern, (_, prefix, _version, suffix) => `${prefix}${next}${suffix}`)
    }
    return [file, content]
  })
  // Validate every file before writing any of them.
  for (const [file, content] of changes) writeFileSync(`${cwd}/${file}`, content)
}

export function prepare(cwd, event) {
  if (event.ref !== 'refs/heads/main' || event.deleted === true ||
      !/^[a-f0-9]{40}$/.test(event.after) || /^0+$/.test(event.after)) {
    throw new Error('仅允许 main 分支的代码推送生成版本。')
  }
  requireClean(cwd)
  fetchBranches(cwd)
  const base = git(cwd, 'rev-parse', 'HEAD')
  if (base !== git(cwd, 'rev-parse', 'origin/main')) throw new Error('main 已前进，请重新运行流水线。')
  if (!isAncestor(cwd, event.after, base)) throw new Error('推送提交不在 main 历史中。')

  const current = JSON.parse(readFileSync(`${cwd}/package.json`, 'utf8')).version
  const previousTag = `v${current}`
  const hasTag = git(cwd, 'tag', '--list', previousTag) === previousTag
  if (hasTag && !isAncestor(cwd, previousTag, base)) throw new Error(`${previousTag} 不在 main 历史中。`)
  // Covers reruns, queued older pushes and the generated version commit itself.
  if (hasTag && isAncestor(cwd, event.after, previousTag)) return { skip: true }

  const messages = git(cwd, 'log', '--format=%B%x00', hasTag ? `${previousTag}..HEAD` : 'HEAD')
    .split('\0').map(message => message.trim()).filter(message => !message.startsWith('chore(release): v'))
  const type = releaseType(messages)
  const version = nextVersion(current, type)
  const tag = `v${version}`
  if (git(cwd, 'tag', '--list', tag)) throw new Error(`${tag} 已存在，停止重复发布。`)
  updateVersions(cwd, current, version)
  git(cwd, 'add', '--', ...versionFiles)
  git(cwd, 'commit', '-m', `chore(release): ${tag}\n\nRelease-Source: ${event.after}`)
  return { skip: false, base, version, tag, type, commit: git(cwd, 'rev-parse', 'HEAD') }
}

export function exportPrepared(cwd, bundle, commit) {
  if (git(cwd, 'rev-parse', 'HEAD') !== commit) throw new Error('只能导出刚生成的版本提交。')
  git(cwd, 'bundle', 'create', bundle, 'HEAD^..HEAD')
}

export function restorePrepared(cwd, bundle, commit) {
  if (!/^[a-f0-9]{40}$/.test(commit)) throw new Error('无效的版本提交。')
  requireClean(cwd)
  git(cwd, 'bundle', 'verify', bundle)
  git(cwd, 'fetch', bundle, 'HEAD')
  if (git(cwd, 'rev-parse', 'FETCH_HEAD') !== commit) throw new Error('构建快照与预期版本提交不一致。')
  git(cwd, 'checkout', '--detach', commit)
  return { commit }
}

export function finalize(cwd, commit, version) {
  if (!/^[a-f0-9]{40}$/.test(commit)) throw new Error('无效的版本提交。')
  nextVersion(version, 'patch') // Validate before using the version as a Git ref.
  requireClean(cwd)
  if (git(cwd, 'rev-parse', 'HEAD') !== commit) throw new Error('当前代码与构建时的版本提交不一致。')
  const changed = git(cwd, 'diff-tree', '--no-commit-id', '--name-only', '-r', commit).split('\n').sort()
  if (JSON.stringify(changed) !== JSON.stringify([...versionFiles].sort())) {
    throw new Error('版本提交必须且只能修改五个版本文件。')
  }
  updateVersions(cwd, version, version)
  fetchBranches(cwd)
  const tag = `v${version}`
  if (git(cwd, 'tag', '--list', tag)) {
    const synced = git(cwd, 'log', 'origin/dev', '--format=%H', '--fixed-strings',
      `--grep=(cherry picked from commit ${commit})`)
    if (git(cwd, 'rev-parse', `${tag}^{commit}`) === commit &&
        isAncestor(cwd, commit, 'origin/main') && synced) return { commit, tag, alreadySynced: true }
    throw new Error(`${tag} 已存在且与本次同步不一致，未推送任何分支。`)
  }
  if (git(cwd, 'rev-parse', 'origin/main') !== git(cwd, 'rev-parse', `${commit}^`)) {
    throw new Error('构建期间 main 已前进；未推送版本，请重新运行流水线以构建最新 main。')
  }
  git(cwd, 'checkout', '--detach', 'origin/dev')
  try {
    git(cwd, 'cherry-pick', '-x', commit)
  } catch (error) {
    git(cwd, 'cherry-pick', '--abort')
    throw new Error('版本提交 cherry-pick 到 dev 失败；远端未变更。请解决版本文件冲突后重新运行。', { cause: error })
  }
  updateVersions(cwd, version, version)
  const devCommit = git(cwd, 'rev-parse', 'HEAD')
  git(cwd, 'tag', tag, commit)
  // A protected branch, concurrent push or hook rejection must leave ALL remote refs unchanged.
  git(cwd, 'push', '--atomic', 'origin',
    `${commit}:refs/heads/main`, `${devCommit}:refs/heads/dev`, `refs/tags/${tag}`)
  return { commit, devCommit, tag }
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const cwd = process.cwd()
  let result
  if (process.argv[2] === 'prepare') {
    if (process.env.GITHUB_EVENT_NAME !== 'push') throw new Error('版本准备只能由 main 推送事件运行。')
    result = prepare(cwd, JSON.parse(readFileSync(process.env.GITHUB_EVENT_PATH, 'utf8')))
    if (!result.skip && process.env.RELEASE_BUNDLE) exportPrepared(cwd, process.env.RELEASE_BUNDLE, result.commit)
  } else if (process.argv[2] === 'restore') {
    result = restorePrepared(cwd, process.env.RELEASE_BUNDLE, process.env.RELEASE_COMMIT)
  } else if (process.argv[2] === 'finalize') {
    result = finalize(cwd, process.env.RELEASE_COMMIT, process.env.RELEASE_VERSION)
  } else {
    throw new Error('用法：node scripts/release.mjs prepare|restore|finalize')
  }
  log(JSON.stringify(result))
  if (process.env.GITHUB_OUTPUT) {
    appendFileSync(process.env.GITHUB_OUTPUT, Object.entries(result).map(([key, value]) => `${key}=${value}\n`).join(''))
  }
}
