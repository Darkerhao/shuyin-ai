import assert from 'node:assert/strict'
import { spawn } from 'node:child_process'
import { existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs'
import { rm } from 'node:fs/promises'
import { dirname, join, resolve } from 'node:path'
import process from 'node:process'
import { setTimeout as delay } from 'node:timers/promises'
import { fileURLToPath, URL } from 'node:url'
import test from 'node:test'
import { runQueue } from '../scripts/codex-queue.mjs'

const temporaryRoot = resolve('output/codex-queue-tests')
const runner = fileURLToPath(new URL('../scripts/codex-queue.mjs', import.meta.url))

function fixture(t) {
  mkdirSync(temporaryRoot, { recursive: true })
  const root = mkdtempSync(join(temporaryRoot, '中文 空格-'))
  t.after(async () => {
    const child = join(root, 'child.pid')
    if (existsSync(child)) {
      try { process.kill(Number(readFileSync(child, 'utf8'))) } catch (cause) { if (cause.code !== 'ESRCH') throw cause }
    }
    await rm(root, { recursive: true, force: true, maxRetries: 5, retryDelay: 100 })
  })
  const fake = join(root, 'fake-codex.mjs')
  writeFileSync(fake, `
import { appendFileSync, existsSync, readFileSync, writeFileSync } from 'node:fs'
import { spawn } from 'node:child_process'
import { join } from 'node:path'
import process from 'node:process'
const args = process.argv.slice(2)
let prompt = ''
for await (const part of process.stdin) prompt += part.toString('utf8')
const title = prompt.match(/^当前任务：(\\S+)/m)[1]
const report = args[args.indexOf('--output-last-message') + 1]
const trace = join(process.cwd(), 'trace.jsonl')
appendFileSync(trace, JSON.stringify({ title, args, prompt, pid: process.pid, previous: existsSync('previous.txt') }) + '\\n')
console.log(JSON.stringify({ type: 'thread.started', thread_id: 'session-' + process.pid }))
if (prompt.includes('[inherited]')) {
  const child = spawn(process.execPath, ['-e', 'setTimeout(() => {}, 60_000)'], { stdio: ['ignore', 'inherit', 'ignore'] })
  writeFileSync('child.pid', String(child.pid))
  child.unref()
}
if (prompt.includes('[wait]')) {
  const child = spawn(process.execPath, ['-e', 'setTimeout(() => {}, 60_000)'], { stdio: 'ignore' })
  writeFileSync('child.pid', String(child.pid))
  await new Promise(accept => setTimeout(accept, 60_000))
}
if (prompt.includes('[exit]')) process.exit(7)
if (prompt.includes('[missing]')) process.exit(0)
if (prompt.includes('[malformed]')) { writeFileSync(report, '{}'); process.exit(0) }
writeFileSync('previous.txt', title)
writeFileSync(report, JSON.stringify({
  status: prompt.includes('[blocked]') ? 'blocked' : 'completed', summary: '中文结果 ' + title,
  files: ['previous.txt'], database: '无变更', checks: ['fixture 验证通过'], risks: [], acceptance: ['已完成本项'], nextSteps: [],
}))
console.log(JSON.stringify({ type: prompt.includes('[turn-failed]') ? 'turn.failed' : 'turn.completed' }))
`)
  const config = {
    cwd: root, outputDir: join(root, '日志'), command: [process.execPath, fake],
    tasks: [
      { id: 'one', title: '第一项', prompt: '完成第一项，中文 / $() / ` / " 都保持原样。' },
      { id: 'two', title: '第二项', prompt: '完成第二项。' },
    ],
  }
  const trace = () => existsSync(join(root, 'trace.jsonl')) ? readFileSync(join(root, 'trace.jsonl'), 'utf8').trim().split('\n').map(line => JSON.parse(line)) : []
  const state = () => JSON.parse(readFileSync(join(config.outputDir, 'state.json'), 'utf8'))
  return { root, config, trace, state }
}

test('sequential fresh sessions share files, preserve Chinese, and produce reports for acceptance', async t => {
  const { config, trace, state } = fixture(t)
  await runQueue(config)
  const calls = trace()
  assert.deepEqual(calls.map(call => call.title), ['one', 'two'])
  assert.equal(calls[0].previous, false)
  assert.equal(calls[1].previous, true)
  assert.notEqual(state().tasks.one.threadId, state().tasks.two.threadId)
  assert.ok(calls[0].prompt.includes(config.tasks[0].prompt))
  assert.ok(calls[1].prompt.includes('report.md'))
  for (const call of calls) {
    assert.equal(call.args[0], 'exec')
    assert.equal(call.args.at(-1), '-')
    assert.equal(call.args[call.args.indexOf('--sandbox') + 1], 'workspace-write')
    assert.ok(call.args.includes('approval_policy="never"'))
    assert.ok(!call.args.includes('resume') && !call.args.includes('--ephemeral'))
    const schema = JSON.parse(readFileSync(call.args[call.args.indexOf('--output-schema') + 1], 'utf8'))
    assert.ok(schema.required.includes('status'))
  }
  assert.equal(state().tasks.two.status, 'completed')
  const first = join(config.outputDir, state().tasks.one.directory)
  assert.match(readFileSync(join(first, 'report.md'), 'utf8'), /待用户验收[\s\S]*中文结果 one/)
  assert.match(readFileSync(join(first, 'events.jsonl'), 'utf8'), /thread.started/)
  assert.ok(existsSync(join(first, 'stderr.log')))
  assert.ok(existsSync(join(first, 'prompt.md')))
  assert.match(readFileSync(join(config.outputDir, 'SUMMARY.md'), 'utf8'), /completed/)
  assert.equal(existsSync(join(config.outputDir, 'queue.lock')), false)
})

test('restart skips completed tasks and appending a task only runs the appended task', async t => {
  const { config, trace } = fixture(t)
  await runQueue(config)
  await runQueue(config)
  assert.equal(trace().length, 2)
  config.tasks.push({ id: 'three', title: '第三项', prompt: '追加任务' })
  await runQueue(config)
  assert.deepEqual(trace().map(call => call.title), ['one', 'two', 'three'])
})

test('changing an earlier task invalidates its completed dependent tasks', async t => {
  const { config, trace } = fixture(t)
  await runQueue(config)
  config.tasks[0].prompt += '\n新的验收标准'
  await runQueue(config)
  assert.deepEqual(trace().map(call => call.title), ['one', 'two', 'one', 'two'])
})

test('a missing predecessor report reruns that task and its completed successors', async t => {
  const { config, trace, state } = fixture(t)
  await runQueue(config)
  rmSync(join(config.outputDir, state().tasks.one.directory, 'report.md'))
  await runQueue(config)
  assert.deepEqual(trace().map(call => call.title), ['one', 'two', 'one', 'two'])
})

for (const [marker, expected] of [
  ['[exit]', /退出码：7/], ['[missing]', /ENOENT/], ['[malformed]', /报告字段无效/],
  ['[blocked]', /任务受阻/], ['[turn-failed]', /Codex 执行失败/],
]) {
  test(`stops on ${marker}, records the failure, and does not start the next task`, async t => {
    const { config, trace, state } = fixture(t)
    config.tasks[1].prompt += marker
    config.tasks.push({ id: 'three', title: '第三项', prompt: '不应执行' })
    await assert.rejects(runQueue(config), expected)
    assert.deepEqual(trace().map(call => call.title), ['one', 'two'])
    assert.equal(state().tasks.one.status, 'completed')
    assert.equal(state().tasks.two.status, marker === '[blocked]' ? 'blocked' : 'failed')
    assert.ok(state().tasks.two.error)
    assert.equal(state().tasks.three, undefined)
    assert.equal(existsSync(join(config.outputDir, 'queue.lock')), false)
  })
}

test('a failed task is retried in a new session while the completed predecessor stays skipped', async t => {
  const { config, trace, state } = fixture(t)
  config.tasks[1].prompt += '[exit]'
  await assert.rejects(runQueue(config), /退出码/)
  const failedDirectory = state().tasks.two.directory
  config.tasks[1].prompt = '完成第二项。'
  await runQueue(config)
  assert.deepEqual(trace().map(call => call.title), ['one', 'two', 'two'])
  assert.notEqual(state().tasks.two.directory, failedDirectory)
  assert.ok(existsSync(join(config.outputDir, failedDirectory, 'events.jsonl')))
})

test('dry run starts no process and writes no state, and disabled tasks are skipped', async t => {
  const { config, trace } = fixture(t)
  config.tasks[1].enabled = false
  await runQueue(config, { dryRun: true })
  assert.equal(existsSync(config.outputDir), false)
  assert.deepEqual(trace(), [])
  await runQueue(config)
  assert.deepEqual(trace().map(call => call.title), ['one'])
})

test('duplicate IDs and empty enabled queues fail before starting a session', async t => {
  const { config, trace } = fixture(t)
  config.tasks[1].id = 'one'
  await assert.rejects(runQueue(config), /重复/)
  config.tasks[0].enabled = false
  config.tasks[1].enabled = false
  await assert.rejects(runQueue(config), /没有启用/)
  assert.deepEqual(trace(), [])
})

test('existing lock prevents a second runner from executing tasks', async t => {
  const { config, trace } = fixture(t)
  mkdirSync(config.outputDir)
  writeFileSync(join(config.outputDir, 'queue.lock'), 'another runner')
  await assert.rejects(runQueue(config), /已锁定/)
  assert.deepEqual(trace(), [])
  assert.equal(readFileSync(join(config.outputDir, 'queue.lock'), 'utf8'), 'another runner')
})

test('timeout stops the active session, records failure, and does not run the next task', async t => {
  const { root, config, trace, state } = fixture(t)
  config.tasks[0].prompt += '[wait]'
  config.timeoutMinutes = 0.01
  await assert.rejects(runQueue(config), /分钟时限/)
  assert.deepEqual(trace().map(call => call.title), ['one'])
  assert.equal(state().tasks.one.status, 'failed')
  const pid = Number(readFileSync(join(root, 'child.pid'), 'utf8'))
  assert.throws(() => process.kill(pid, 0), { code: 'ESRCH' })
})

test('SIGINT stops the active process tree and prevents subsequent tasks', async t => {
  const { root, config, trace, state } = fixture(t)
  config.tasks[0].prompt += '[wait]'
  const stopped = assert.rejects(runQueue(config), /用户中断/)
  for (let attempt = 0; attempt < 100 && !existsSync(join(root, 'child.pid')); attempt++) await delay(20)
  assert.ok(existsSync(join(root, 'child.pid')))
  process.emit('SIGINT')
  await stopped
  assert.deepEqual(trace().map(call => call.title), ['one'])
  assert.equal(state().tasks.one.status, 'failed')
  assert.equal(existsSync(join(config.outputDir, 'queue.lock')), false)
  assert.throws(() => process.kill(Number(readFileSync(join(root, 'child.pid'), 'utf8')), 0), { code: 'ESRCH' })
})

test('corrupt persisted state reports an error and releases the queue lock', async t => {
  const { config, trace } = fixture(t)
  mkdirSync(config.outputDir)
  writeFileSync(join(config.outputDir, 'state.json'), '{')
  await assert.rejects(runQueue(config), SyntaxError)
  assert.deepEqual(trace(), [])
  assert.equal(existsSync(join(config.outputDir, 'queue.lock')), false)
})

test('finished CLI does not wait for stdout inherited by background MCP processes', { timeout: 5000 }, async t => {
  const { config, state } = fixture(t)
  config.tasks = [config.tasks[0]]
  config.tasks[0].prompt += '[inherited]'
  await runQueue(config)
  assert.equal(state().tasks.one.status, 'completed')
})

test('missing CLI is recorded as failure and releases the lock', async t => {
  const { root, config, state } = fixture(t)
  config.command = [join(root, 'does-not-exist')]
  await assert.rejects(runQueue(config), /ENOENT/)
  assert.equal(state().tasks.one.status, 'failed')
  assert.equal(existsSync(join(config.outputDir, 'queue.lock')), false)
})

test('CLI entry resolves relative config paths and --dry-run creates no logs', async t => {
  const { root, config, trace } = fixture(t)
  const configFile = join(root, '任务.mjs')
  config.cwd = '.'
  config.outputDir = '记录'
  writeFileSync(configFile, `export default ${JSON.stringify(config)}`)
  const result = await new Promise((accept, reject) => {
    const child = spawn(process.execPath, [runner, '--config', configFile, '--dry-run'], { cwd: dirname(runner), stdio: 'pipe' })
    let stdout = '', stderr = ''
    child.stdout.on('data', chunk => { stdout += chunk })
    child.stderr.on('data', chunk => { stderr += chunk })
    child.once('error', reject)
    child.once('close', code => accept({ code, stdout, stderr }))
  })
  assert.equal(result.code, 0, result.stderr)
  assert.ok(result.stdout.includes(root))
  assert.deepEqual(trace(), [])
  assert.equal(existsSync(join(root, '记录')), false)
  assert.equal(readdirSync(root).some(name => name.endsWith('.lock')), false)
})
