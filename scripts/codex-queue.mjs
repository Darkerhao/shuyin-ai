import { spawn, spawnSync } from 'node:child_process'
import { log, error } from 'node:console'
import { createHash, randomUUID } from 'node:crypto'
import { closeSync, existsSync, mkdirSync, openSync, readFileSync, unlinkSync, writeFileSync, writeSync } from 'node:fs'
import { rename } from 'node:fs/promises'
import { delimiter, dirname, join, resolve } from 'node:path'
import process from 'node:process'
import { createInterface } from 'node:readline'
import { clearTimeout, setTimeout } from 'node:timers'
import { setTimeout as delay } from 'node:timers/promises'
import { fileURLToPath, pathToFileURL, URL } from 'node:url'
import { parseArgs } from 'node:util'

const instructions = `你正在执行串行任务队列中的一项任务，这是独立的新会话。
先阅读项目 AGENTS.md 和当前真实代码，保留所有已有改动。共用工作目录中的前序成果，不恢复或覆盖无关修改。
只完成当前任务：实施、运行适用的验证、更新相关文档，不以方案或计划代替实现。
不得自行运行任务队列、启动后续任务、提交、推送或发布。不得输出密钥。
需要人工操作或缺少必要信息时，报告 blocked 和准确原因；不要假装已完成。
报告中的 checks 只写实际执行的命令及结果；未运行的检查写入 risks。
只有当前目标和可自动验证的验收标准全部达成才报告 completed，最终人工验收由用户负责。`

const reportSchema = {
  type: 'object',
  additionalProperties: false,
  properties: {
    status: { type: 'string', enum: ['completed', 'blocked'] },
    summary: { type: 'string' },
    files: { type: 'array', items: { type: 'string' } },
    database: { type: 'string' },
    checks: { type: 'array', items: { type: 'string' } },
    risks: { type: 'array', items: { type: 'string' } },
    acceptance: { type: 'array', items: { type: 'string' } },
    nextSteps: { type: 'array', items: { type: 'string' } },
  },
  required: ['status', 'summary', 'files', 'database', 'checks', 'risks', 'acceptance', 'nextSteps'],
}

async function saveJson(file, value) {
  writeFileSync(`${file}.tmp`, `${JSON.stringify(value, null, 2)}\n`)
  // Windows 本机回归中原子替换出现短暂 EPERM；只对此类占用有限等待，其他错误直接上报。
  for (let attempt = 0; ; attempt++) {
    try { await rename(`${file}.tmp`, file); return } catch (cause) {
      if (!['EPERM', 'EBUSY'].includes(cause.code) || attempt === 3) throw cause
      await delay(50 * (attempt + 1))
    }
  }
}

// Windows 的 npm codex.cmd 不能直接交给 spawn；启动同一安装的 JS 入口，避免 shell 转义。
function resolveCommand(command) {
  if (command[0] !== 'codex' || process.platform !== 'win32') return command
  for (const directory of (process.env.PATH ?? '').split(delimiter)) {
    const root = directory.replace(/^"|"$/g, '')
    if (!root) continue
    const exe = join(root, 'codex.exe')
    if (existsSync(exe)) return [exe, ...command.slice(1)]
    const entry = join(root, 'node_modules/@openai/codex/bin/codex.js')
    if (existsSync(join(root, 'codex.cmd')) && existsSync(entry)) {
      return [process.execPath, entry, ...command.slice(1)]
    }
  }
  throw new Error('找不到 Codex CLI。请确认 codex --version 可运行，或在配置中指定 command 数组。')
}

function reportMarkdown(task, report) {
  const sections = { files: '修改文件', checks: '实际验证', risks: '未解决问题与风险', acceptance: '验收标准', nextSteps: '下一步建议' }
  return `# ${task.title}\n\n${report.status === 'completed' ? '执行完成，待用户验收' : '任务受阻'}\n\n${report.summary}\n\n## 数据库变更\n\n${report.database}\n\n`
    + Object.entries(sections).map(([key, title]) => `## ${title}\n\n${report[key].map(value => `- ${value}`).join('\n') || '无'}\n`).join('\n')
}

function readReport(file) {
  const report = JSON.parse(readFileSync(file, 'utf8'))
  for (const key of reportSchema.required) {
    if (reportSchema.properties[key].type === 'array'
      ? !Array.isArray(report[key]) || report[key].some(value => typeof value !== 'string')
      : typeof report[key] !== 'string') throw new Error(`任务报告字段无效：${key}`)
  }
  if (!['completed', 'blocked'].includes(report.status)) throw new Error('任务报告 status 无效。')
  return report
}

async function execute(command, args, cwd, prompt, directory, timeoutMinutes) {
  const events = openSync(join(directory, 'events.jsonl'), 'w')
  const stderr = openSync(join(directory, 'stderr.log'), 'w')
  let child, lines, timer, stopReason, threadId, turnFailed
  const stop = reason => {
    stopReason ??= reason
    if (!child?.pid || child.exitCode !== null || child.signalCode !== null) return
    if (process.platform === 'win32') {
      const result = spawnSync('taskkill.exe', ['/PID', String(child.pid), '/T', '/F'], { windowsHide: true, stdio: 'ignore' })
      if (result.status !== 0) child.kill()
    } else {
      try { process.kill(-child.pid, 'SIGKILL') } catch (cause) { if (cause.code !== 'ESRCH') throw cause }
    }
  }
  const interrupt = () => stop('用户中断')
  const terminate = () => stop('进程终止')
  process.on('SIGINT', interrupt)
  process.on('SIGTERM', terminate)
  try {
    child = spawn(command[0], [...command.slice(1), ...args], {
      cwd, stdio: ['pipe', 'pipe', stderr], windowsHide: true, detached: process.platform !== 'win32',
    })
    lines = createInterface({ input: child.stdout })
    lines.on('line', line => {
      writeSync(events, `${line}\n`)
      let event
      try { event = JSON.parse(line) } catch { return }
      if (event.type === 'thread.started') { threadId = event.thread_id; log(`  新会话：${threadId}`) }
      if (event.type === 'turn.failed') turnFailed = true
      if (event.type === 'item.completed' && event.item?.type === 'agent_message') log(event.item.text)
      if (event.type === 'item.started' && event.item?.type === 'command_execution') log(`  执行：${event.item.command}`)
    })
    timer = setTimeout(() => stop(`超过 ${timeoutMinutes} 分钟时限`), timeoutMinutes * 60_000)
    // 大段中文提示经 UTF-8 stdin 传递，不进入 shell 或命令行参数。
    child.stdin.on('error', cause => { if (cause.code !== 'EPIPE') error(cause.message) })
    child.stdin.end(prompt, 'utf8')
    const exitCode = await new Promise((accept, reject) => {
      child.once('error', reject)
      // npm 启动器退出后，MCP 子进程可能仍持有 stdout；等待进程退出，避免误等管道关闭。
      child.once('exit', code => accept(code))
    })
    if (stopReason) throw new Error(stopReason)
    if (exitCode !== 0 || turnFailed) throw new Error(`Codex 执行失败，退出码：${exitCode}；详情见 stderr.log 和 events.jsonl。`)
    if (!threadId) throw new Error('未收到新会话 ID；请检查 events.jsonl 和 CLI 版本。')
    return threadId
  } finally {
    clearTimeout(timer)
    lines?.close()
    child?.stdout.destroy()
    child?.stdin.destroy()
    process.off('SIGINT', interrupt)
    process.off('SIGTERM', terminate)
    closeSync(events)
    closeSync(stderr)
  }
}

export async function runQueue(config, { baseDir = process.cwd(), dryRun = false } = {}) {
  const cwd = resolve(baseDir, config.cwd ?? '.')
  const output = resolve(baseDir, config.outputDir ?? 'output/codex-queue')
  const sandbox = config.sandbox ?? 'workspace-write'
  const timeoutMinutes = config.timeoutMinutes ?? 120
  const tasks = config.tasks?.filter(task => task.enabled !== false)
  if (!tasks?.length) throw new Error('没有启用的任务：请填写 codex-tasks.mjs，将任务 enabled 改为 true。')
  if (!['read-only', 'workspace-write', 'danger-full-access'].includes(sandbox)) throw new Error('sandbox 值无效。')
  if (!Number.isFinite(timeoutMinutes) || timeoutMinutes <= 0 || timeoutMinutes * 60_000 > 2_147_483_647) throw new Error('timeoutMinutes 必须是有效的正数。')
  if (!existsSync(cwd)) throw new Error(`工作目录不存在：${cwd}`)
  const ids = new Set()
  for (const task of tasks) {
    if (typeof task.id !== 'string' || !/^[a-zA-Z0-9][a-zA-Z0-9_-]{0,79}$/.test(task.id) || ids.has(task.id)) throw new Error(`任务 id 无效或重复：${task.id}`)
    if (typeof task.title !== 'string' || !task.title.trim() || typeof task.prompt !== 'string' || !task.prompt.trim()) throw new Error(`任务 ${task.id} 缺少 title 或 prompt。`)
    ids.add(task.id)
  }
  const command = config.command ?? ['codex']
  if (!Array.isArray(command) || !command.length || command.some(value => typeof value !== 'string' || !value)) throw new Error('command 必须是非空字符串数组。')
  const executable = dryRun ? undefined : resolveCommand(command)
  const lockFile = join(output, 'queue.lock')
  let lock
  if (!dryRun) {
    mkdirSync(output, { recursive: true })
    try { lock = openSync(lockFile, 'wx') } catch (cause) {
      if (cause.code === 'EEXIST') throw new Error(`队列已锁定：${lockFile}。确认原进程及其 Codex 子进程已停止后，才能删除此锁。`, { cause })
      throw cause
    }
  }
  try {
    const stateFile = join(output, 'state.json')
    const state = existsSync(stateFile) ? JSON.parse(readFileSync(stateFile, 'utf8')) : { tasks: {} }
    let fingerprint = JSON.stringify({ cwd, sandbox, model: config.model, command, instructions, reportSchema, extra: config.instructions })
    let predecessorsDone = true
    const queue = tasks.map(task => {
      fingerprint = createHash('sha256').update(fingerprint + JSON.stringify(task)).digest('hex')
      const previous = state.tasks[task.id]
      const done = predecessorsDone && previous?.status === 'completed' && previous.fingerprint === fingerprint && existsSync(join(output, previous.directory, 'report.md'))
      predecessorsDone = Boolean(done)
      return { ...task, fingerprint, done }
    })
    log(`项目：${cwd}\n验收汇总：${join(output, 'SUMMARY.md')}`)
    for (const [index, task] of queue.entries()) log(`${index + 1}. ${task.id} ${task.title} — ${task.done ? '已完成，跳过' : '待执行，新会话'}`)
    if (dryRun) return
    const save = async () => {
      await saveJson(stateFile, state)
      const rows = queue.map(task => {
        const record = state.tasks[task.id]
        const current = record?.fingerprint === task.fingerprint ? record : undefined
        const report = current?.directory ? `[报告目录](./${current.directory}/)` : ''
        return `| ${task.id} | ${task.title.replace(/[|\r\n]/g, ' ')} | ${current?.status ?? 'pending'} | ${report} |`
      })
      writeFileSync(join(output, 'SUMMARY.md'), `# Codex 任务验收汇总\n\n项目：${cwd}\n\ncompleted 表示执行结束且模型报告完成，最终仍待用户验收；failed / blocked 后队列停止。\n\n| ID | 任务 | 执行状态 | 记录 |\n| --- | --- | --- | --- |\n${rows.join('\n')}\n`)
    }
    writeSync(lock, JSON.stringify({ pid: process.pid, cwd, startedAt: new Date().toISOString() }))
    const schemaFile = join(output, 'report.schema.json')
    await saveJson(schemaFile, reportSchema)
    await save()
    for (const task of queue) {
      if (task.done) continue
      const directory = join(task.id, `${new Date().toISOString().replace(/[:.]/g, '-')}-${randomUUID().slice(0, 8)}`)
      const absoluteDirectory = join(output, directory)
      mkdirSync(absoluteDirectory, { recursive: true })
      const record = state.tasks[task.id] = { fingerprint: task.fingerprint, status: 'running', directory: directory.replaceAll('\\', '/'), startedAt: new Date().toISOString() }
      const preceding = queue.slice(0, queue.indexOf(task)).map(previous => join(output, state.tasks[previous.id].directory, 'report.md'))
      const prompt = `${instructions}\n\n${config.instructions ?? ''}\n\n前序任务报告（需要时读取）：\n${preceding.join('\n') || '无'}\n\n当前任务：${task.id} ${task.title}\n${task.prompt}\n\n最终报告必须只输出符合以下 JSON Schema 的 JSON 对象，不加代码块；所有列表字段均为字符串数组，无内容时写 []，不得遗漏字段：\n${JSON.stringify(reportSchema)}\n`
      writeFileSync(join(absoluteDirectory, 'prompt.md'), prompt)
      await save()
      log(`\n开始：${task.id} ${task.title}`)
      const reportFile = join(absoluteDirectory, 'report.json')
      const args = ['exec', '-C', cwd, '--sandbox', sandbox, '-c', 'approval_policy="never"', '--json', '--color', 'never', '--output-schema', schemaFile, '--output-last-message', reportFile]
      if (config.model) args.push('--model', config.model)
      args.push('-')
      try {
        record.threadId = await execute(executable, args, cwd, prompt, absoluteDirectory, timeoutMinutes)
        const report = readReport(reportFile)
        writeFileSync(join(absoluteDirectory, 'report.md'), reportMarkdown(task, report))
        record.status = report.status
        if (report.status === 'blocked') throw new Error(`任务受阻：${report.summary}`)
      } catch (cause) {
        if (record.status !== 'blocked') record.status = 'failed'
        record.error = cause.message
        throw cause
      } finally {
        record.finishedAt = new Date().toISOString()
        await save()
      }
    }
    log('\n所有任务执行结束，请查看 SUMMARY.md 和各任务 report.md 进行验收。')
  } finally {
    if (lock !== undefined) {
      closeSync(lock)
      unlinkSync(lockFile)
    }
  }
}

if (process.argv[1] && pathToFileURL(resolve(process.argv[1])).href === import.meta.url) {
  try {
    const { values } = parseArgs({ options: { config: { type: 'string' }, 'dry-run': { type: 'boolean' }, help: { type: 'boolean' } } })
    if (values.help) {
      log('用法：node scripts/codex-queue.mjs [--config 任务配置.mjs] [--dry-run]\n默认配置：项目根目录 codex-tasks.mjs。失败即停，再次执行跳过已完成任务。')
    } else {
      const configFile = values.config ? resolve(values.config) : fileURLToPath(new URL('../codex-tasks.mjs', import.meta.url))
      const config = (await import(pathToFileURL(configFile).href)).default
      await runQueue(config, { baseDir: dirname(configFile), dryRun: values['dry-run'] })
    }
  } catch (cause) {
    error(`队列停止：${cause.message}`)
    process.exitCode = 1
  }
}
