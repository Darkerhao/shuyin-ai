import assert from 'node:assert/strict'
import { execFileSync } from 'node:child_process'
import { copyFileSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'
import { fileURLToPath, URL } from 'node:url'
import test from 'node:test'
import { exportPrepared, finalize, nextVersion, prepare, releaseType, restorePrepared, updateVersions, versionFiles } from '../scripts/release.mjs'

const project = fileURLToPath(new URL('..', import.meta.url))

function git(cwd, ...args) {
  return execFileSync('git', args, { cwd, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] }).trim()
}

function identity(cwd) {
  git(cwd, 'config', 'user.name', 'Release test')
  git(cwd, 'config', 'user.email', 'release-test@example.invalid')
  git(cwd, 'config', 'commit.gpgsign', 'false')
}

function commitFile(cwd, file, content, message) {
  writeFileSync(join(cwd, file), content)
  git(cwd, 'add', '--', file)
  git(cwd, 'commit', '-m', message)
  return git(cwd, 'rev-parse', 'HEAD')
}

function fixture(t, { message = 'fix: improve probe', initialTag = true } = {}) {
  const root = mkdtempSync(join(tmpdir(), 'shuyin-release-'))
  t.after(() => rmSync(root, { recursive: true, force: true }))
  const remote = join(root, 'remote.git')
  const seed = join(root, 'seed')
  const runner = join(root, 'runner')
  mkdirSync(seed)
  git(root, 'init', '--bare', '--initial-branch=main', remote)
  git(seed, 'init', '--initial-branch=main')
  identity(seed)
  for (const file of versionFiles) {
    mkdirSync(dirname(join(seed, file)), { recursive: true })
    copyFileSync(join(project, file), join(seed, file))
  }
  const current = JSON.parse(readFileSync(join(seed, 'package.json'), 'utf8')).version
  updateVersions(seed, current, '0.1.0')
  writeFileSync(join(seed, 'probe.txt'), 'initial\n')
  git(seed, 'add', '.')
  git(seed, 'commit', '-m', 'chore: initial application')
  if (initialTag) git(seed, 'tag', 'v0.1.0')
  git(seed, 'branch', 'dev')
  const merge = commitFile(seed, 'probe.txt', 'merged change\n', message)
  // A normal dev -> main PR leaves the same merged source on dev.
  git(seed, 'branch', '-f', 'dev', merge)
  git(seed, 'remote', 'add', 'origin', remote)
  git(seed, 'push', 'origin', 'main', 'dev', '--tags')
  git(root, 'clone', remote, runner)
  identity(runner)
  const event = { ref: 'refs/heads/main', deleted: false, after: merge }
  return { root, remote, seed, runner, event }
}

function remoteRefs(remote) {
  return git(remote, 'show-ref')
}

function versions(cwd, ref) {
  const read = file => git(cwd, 'show', `${ref}:${file}`)
  const lock = JSON.parse(read('package-lock.json'))
  return [
    JSON.parse(read('package.json')).version,
    lock.version,
    lock.packages[''].version,
    JSON.parse(read('src-tauri/tauri.conf.json')).version,
    read('src-tauri/Cargo.toml').match(/\[package\][\s\S]*?version = "([^"]+)"/)[1],
    read('src-tauri/Cargo.lock').match(/name = "shuyin-ai"\r?\nversion = "([^"]+)"/)[1],
  ]
}

test('selects the highest semantic change from titles, headers and breaking footers', () => {
  assert.equal(releaseType(['fix: repair', 'feat(ui): projects']), 'minor')
  assert.equal(releaseType(['feat: project', 'refactor(api)!: remove command']), 'major')
  assert.equal(releaseType(['fix: storage\n\nBREAKING CHANGE: new file format']), 'major')
  assert.equal(releaseType(['fix: storage\n\nBREAKING-CHANGE: new format']), 'major')
  assert.equal(releaseType(['Merge pull request #1\n\nfeat: projects']), 'minor')
  assert.equal(releaseType(['docs: update', '修复中文显示']), 'patch')
  assert.equal(releaseType(['docs: explain feat: and feat! syntax']), 'patch')
})

test('release size follows user impact rather than the number of commits', () => {
  const fixes = Array.from({ length: 100 }, (_, index) => `fix: repair issue ${index}`)
  assert.equal(nextVersion('1.4.8', releaseType([...fixes, 'refactor: reorganize services'])), '1.4.9')
  assert.equal(nextVersion('1.4.8', releaseType([...fixes, 'feat: cover export', 'feat: project search'])), '1.5.0')
  assert.equal(nextVersion('1.4.8', releaseType(['fix!: remove incompatible legacy format'])), '2.0.0')
})

test('examples and plans in a commit body cannot promote a maintenance release', () => {
  assert.equal(releaseType(['docs: explain commits\n\nfeat!: this is an example header']), 'patch')
  assert.equal(releaseType(['chore: record future work\n\nfeat: project search is planned']), 'patch')
  assert.equal(releaseType(['docs: show an example\n\n```text\nBREAKING CHANGE: example only\n```']), 'patch')
  assert.equal(releaseType(['docs: show an example\n\n~~~text\nBREAKING-CHANGE: example only\n~~~']), 'patch')
  assert.equal(releaseType(['fix: repair probe\n\nBREAKING CHANGE:']), 'patch')
  assert.equal(releaseType(['Merge pull request #1 from branch\n\nfeat!: remove old import format']), 'major')
})

test('bumps and resets the correct version segments, including pre-1.0 major changes', () => {
  assert.equal(nextVersion('0.9.9', 'major'), '1.0.0')
  assert.equal(nextVersion('1.9.9', 'minor'), '1.10.0')
  assert.equal(nextVersion('1.9.9', 'patch'), '1.9.10')
  assert.throws(() => nextVersion('1.0.0-beta.1', 'patch'), /正式语义版本/)
  assert.throws(() => nextVersion('01.0.0', 'patch'), /正式语义版本/)
  assert.throws(() => nextVersion('1.0.0', 'unknown'), /未知版本类型/)
})

test('first release needs no tag; preparing the version never writes to the remote', t => {
  const { runner, remote, event } = fixture(t, { message: 'feat: projects', initialTag: false })
  const before = remoteRefs(remote)
  const result = prepare(runner, event)
  assert.equal(result.version, '0.2.0')
  assert.deepEqual(versions(runner, 'HEAD'), Array(6).fill('0.2.0'))
  assert.equal(remoteRefs(remote), before)
  assert.equal(git(runner, 'status', '--porcelain'), '')
  const diff = git(runner, 'diff', '--numstat', 'HEAD^', 'HEAD').split('\n')
  assert.equal(diff.length, 5)
  assert.ok(diff.every(line => /^(1\t1|2\t2)\t/.test(line)), 'only version lines change')
})

test('rejects dev, tags, branch deletion and PR events before changing files', t => {
  const { runner, event } = fixture(t)
  for (const invalid of [
    { ...event, ref: 'refs/heads/dev' },
    { ...event, ref: 'refs/tags/v0.1.0' },
    { ...event, deleted: true },
    { ...event, after: '0'.repeat(40) },
    { action: 'closed', pull_request: { merged: true } },
  ]) assert.throws(() => prepare(runner, invalid), /仅允许 main 分支的代码推送/)
  assert.equal(git(runner, 'status', '--porcelain'), '')
})

test('validates every manifest before modifying any of them', t => {
  const { runner } = fixture(t)
  const cargo = join(runner, 'src-tauri/Cargo.toml')
  writeFileSync(cargo, readFileSync(cargo, 'utf8').replace('version = "0.1.0"', 'version = "9.0.0"'))
  const before = versionFiles.map(file => readFileSync(join(runner, file), 'utf8'))
  assert.throws(() => updateVersions(runner, '0.1.0', '0.2.0'), /不一致/)
  assert.deepEqual(versionFiles.map(file => readFileSync(join(runner, file), 'utf8')), before)
})

test('publishes main, tag and cherry-picked dev together while retaining dev changes', t => {
  const { seed, runner, remote, event } = fixture(t, { message: 'feat: projects' })
  git(seed, 'checkout', 'dev')
  const development = commitFile(seed, 'dev-only.txt', 'unreleased work\n', 'feat: next development')
  git(seed, 'push', 'origin', 'dev')
  const result = prepare(runner, event)
  const pushed = finalize(runner, result.commit, result.version)
  assert.equal(git(remote, 'rev-parse', 'main'), result.commit)
  assert.equal(git(remote, 'rev-parse', result.tag), result.commit)
  assert.equal(git(remote, 'rev-parse', 'dev^'), development)
  assert.deepEqual(versions(remote, 'main'), versions(remote, 'dev'))
  assert.equal(git(remote, 'show', 'dev:dev-only.txt'), 'unreleased work')
  assert.match(git(remote, 'log', '-1', '--format=%B', 'dev'), new RegExp(`cherry picked from commit ${result.commit}`))
  assert.notEqual(pushed.devCommit, result.commit)
  assert.throws(() => git(remote, 'show', 'main:dev-only.txt'))
})

test('reruns and commits already covered by a published tag do not increment again', t => {
  const { runner, remote, event } = fixture(t)
  const result = prepare(runner, event)
  finalize(runner, result.commit, result.version)
  git(runner, 'checkout', '--detach', 'origin/main')
  const before = remoteRefs(remote)
  assert.deepEqual(prepare(runner, event), { skip: true })
  assert.equal(remoteRefs(remote), before)
})

test('a second dev-to-main merge and version sync excludes the previous breaking change', t => {
  const { runner, seed, remote, event } = fixture(t, { message: 'feat!: breaking release' })
  const first = prepare(runner, event)
  assert.equal(first.version, '1.0.0')
  finalize(runner, first.commit, first.version)
  git(seed, 'pull', '--ff-only', 'origin', 'main')
  git(seed, 'checkout', 'dev')
  git(seed, 'pull', '--ff-only', 'origin', 'dev')
  commitFile(seed, 'probe.txt', 'fixed after release\n', 'fix: persistence')
  git(seed, 'push', 'origin', 'dev')
  git(seed, 'checkout', 'main')
  git(seed, 'merge', '--no-ff', 'dev', '-m', 'Merge pull request #2\n\nfix: persistence')
  const merge = git(seed, 'rev-parse', 'HEAD')
  git(seed, 'push', 'origin', 'main')
  git(runner, 'fetch', 'origin')
  git(runner, 'checkout', '--detach', 'origin/main')
  const next = prepare(runner, { ...event, after: merge })
  assert.equal(next.version, '1.0.1')
  finalize(runner, next.commit, next.version)
  assert.deepEqual(versions(remote, 'main'), Array(6).fill('1.0.1'))
  assert.deepEqual(versions(remote, 'dev'), Array(6).fill('1.0.1'))
  assert.equal(git(remote, 'show', 'main:probe.txt'), 'fixed after release')
  assert.equal(git(remote, 'show', 'dev:probe.txt'), 'fixed after release')
})

test('main advancing during a build prevents publishing the stale version', t => {
  const { seed, runner, remote, event } = fixture(t)
  const result = prepare(runner, event)
  commitFile(seed, 'probe.txt', 'newer main\n', 'fix: another merge')
  git(seed, 'push', 'origin', 'main')
  const before = remoteRefs(remote)
  assert.throws(() => finalize(runner, result.commit, result.version), /构建期间 main 已前进/)
  assert.equal(remoteRefs(remote), before)
})

test('a cherry-pick conflict preserves both remote branches and creates no tag', t => {
  const { seed, runner, remote, event } = fixture(t)
  git(seed, 'checkout', 'dev')
  updateVersions(seed, '0.1.0', '0.1.9')
  git(seed, 'commit', '-am', 'chore: conflicting manual version')
  git(seed, 'push', 'origin', 'dev')
  const result = prepare(runner, event)
  const before = remoteRefs(remote)
  assert.throws(() => finalize(runner, result.commit, result.version), /cherry-pick 到 dev 失败/)
  assert.equal(remoteRefs(remote), before)
  assert.equal(git(runner, 'status', '--porcelain'), '')
})

test('a server rejection of dev atomically rejects main and the version tag as well', t => {
  const { runner, remote, event } = fixture(t)
  const result = prepare(runner, event)
  writeFileSync(join(remote, 'hooks/update'), '#!/bin/sh\nif [ "$1" = "refs/heads/dev" ]; then\n  exit 1\nfi\n', { mode: 0o755 })
  const before = remoteRefs(remote)
  assert.throws(() => finalize(runner, result.commit, result.version), /atomic|rejected/)
  assert.equal(remoteRefs(remote), before)
})

test('an existing next tag is never overwritten', t => {
  const { seed, runner, remote, event } = fixture(t)
  git(seed, 'tag', 'v0.1.1')
  git(seed, 'push', 'origin', 'v0.1.1')
  const before = remoteRefs(remote)
  assert.throws(() => prepare(runner, event), /已存在/)
  assert.equal(remoteRefs(remote), before)
})

test('another runner restores the exact unpushed version commit from a bundle', t => {
  const { root, runner, remote, event } = fixture(t)
  const before = remoteRefs(remote)
  const result = prepare(runner, event)
  const bundle = join(root, 'release.bundle')
  exportPrepared(runner, bundle, result.commit)
  const builder = join(root, 'builder')
  git(root, 'clone', remote, builder)
  assert.throws(() => restorePrepared(builder, bundle, result.base), /预期版本提交不一致/)
  restorePrepared(builder, bundle, result.commit)
  assert.equal(git(builder, 'rev-parse', 'HEAD'), result.commit)
  assert.deepEqual(versions(builder, 'HEAD'), Array(6).fill(result.version))
  assert.equal(git(builder, 'status', '--porcelain'), '')
  assert.equal(remoteRefs(remote), before)
})

test('retrying sync after a successful atomic push does not create another commit', t => {
  const { runner, remote, event } = fixture(t)
  const result = prepare(runner, event)
  finalize(runner, result.commit, result.version)
  git(runner, 'checkout', '--detach', result.commit)
  const before = remoteRefs(remote)
  assert.equal(finalize(runner, result.commit, result.version).alreadySynced, true)
  assert.equal(remoteRefs(remote), before)
})

test('local cherry-pick pushed to main prepares a release without any PR payload', t => {
  const { seed, runner, event } = fixture(t)
  git(seed, 'checkout', 'dev')
  const source = commitFile(seed, 'feature.txt', 'cross-platform packaging\n', 'feat: platform packages')
  git(seed, 'push', 'origin', 'dev')
  git(seed, 'checkout', 'main')
  git(seed, 'cherry-pick', '-x', source)
  const after = git(seed, 'rev-parse', 'HEAD')
  git(seed, 'push', 'origin', 'main')
  git(runner, 'pull', '--ff-only')
  const result = prepare(runner, { ...event, after })
  assert.equal(result.version, '0.2.0')
  assert.equal(result.base, after)
})

test('a push of the generated version commit never starts another release', t => {
  const { runner, event, remote } = fixture(t)
  const result = prepare(runner, event)
  finalize(runner, result.commit, result.version)
  git(runner, 'checkout', '--detach', result.commit)
  const before = remoteRefs(remote)
  assert.deepEqual(prepare(runner, { ...event, after: result.commit }), { skip: true })
  assert.equal(remoteRefs(remote), before)
})
