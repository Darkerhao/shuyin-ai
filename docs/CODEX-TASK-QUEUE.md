# Codex 串行任务队列

填写任务清单，一次启动；每项任务新开 Codex CLI 会话，按顺序执行，最后由用户统一验收。脚本复用 Node.js 和已安装的 Codex CLI，不新增依赖，不操作应用数据库，也不会自动提交、推送或发布。

## 1. 填写清单

编辑项目根目录 `codex-tasks.mjs`。三个占位任务默认 `enabled: false`，防止执行尚未填写的要求。每项写好后改为 `true`，需要更多任务就往 `tasks` 数组追加：

```js
{
  id: 'project-search',
  title: '改进书籍搜索',
  enabled: true,
  prompt: `
目标：在现有书籍搜索中支持作者。
范围：复用当前搜索调用链，不新增搜索状态或依赖。
验收：书名及作者都可命中；清空搜索可恢复列表；新增相关回归。
完成后运行适用的检查，记录实际结果并更新相关文档。
`,
},
```

示例仅说明写法，不是本次已授权或已实现的产品需求。`id` 必须唯一，使用英文字母、数字、连字符或下划线。`prompt` 使用反引号支持多行；要在内容中写反引号或 `${...}` 时，按 JavaScript 模板字符串规则转义。

公共要求写在 `instructions`。`cwd`、`outputDir` 的相对路径以配置文件目录为起点。`model` 不填则沿用本机 Codex 配置，不替换用户模型或凭据。

## 2. 运行

先在 PowerShell 确认 `node --version` 和 `codex --version` 能运行，并确认 Codex 已完成登录或供应商配置；脚本沿用现有认证，不保存密钥。然后在项目根目录执行：

```powershell
npm run codex:queue -- --dry-run
npm run codex:queue
```

预览只检查配置、显示执行顺序和已完成状态，不启动模型、不创建日志。清单没有启用项时明确报错。

也可直接执行或指定另一份配置：

```powershell
node scripts/codex-queue.mjs --config .\my-tasks.mjs --dry-run
node scripts/codex-queue.mjs --config .\my-tasks.mjs
```

每项都调用一次 `codex exec`，不调用 `resume` 或 `fork`，也不启用 `--ephemeral`。因此聊天上下文独立，项目文件保留；后续任务提示包含前序报告路径，需要时可读取。CLI 会话 ID 记录在任务状态和事件日志中。不通过桌面界面自动打开聊天窗口，也不保证桌面侧栏实时展示 CLI 会话；以报告与 ID 为准。

队列只执行启动时读取到的清单一遍，不无限重复、不监听文件。运行期间追加的任务需要下一次启动执行；完成后可继续追加任务再启动同一命令。保持终端及电脑运行，系统休眠、网络/额度和模型服务可用性仍会影响执行。

## 3. 权限与停止

默认 `sandbox: 'workspace-write'`，并显式设置 `approval_policy="never"`：自动执行允许的操作，超出沙箱权限的操作不等待人工审批。需要权限或人工操作而无法完成的任务应报告 `blocked`，队列停止。`read-only` 适合纯分析任务；确需整机权限时，可由用户明确将配置改为 `danger-full-access`，该模式允许任务在项目外执行操作。脚本不会自动提升权限，也不使用绕过沙箱参数。

每项默认超时 120 分钟，可通过 `timeoutMinutes` 修改。按 Ctrl+C 停止当前任务及其子进程，并停止队列；超时也会终止进程树。未完成任务保留为失败状态，不启动下一项。

同一输出目录通过 `queue.lock` 防止重复运行。如果电脑断电或进程被强制结束留下锁，先确认原脚本和其 Codex 子进程确实已停止，再删除这个特定锁文件并重跑。不要同时在同一个项目工作目录运行多份队列，即使使用不同输出目录；也避免与其他修改同一文件的 Codex 会话同时执行。

## 4. 续跑与依赖

- 成功条件：Codex 正常退出、返回新会话 ID、报告格式有效且 `status` 为 `completed`。模型自报完成只代表进入人工验收，不等于用户验收通过。
- 进程失败、报告缺失/损坏、模型报告 `blocked` 或超时都会停止后续任务。
- 再次运行相同命令，跳过内容及公共设置未变的已完成项，从首个未完成项开启新会话。失败不自动无限重试。
- 修改前序任务、调整顺序、删除/禁用前序任务，或改变项目、模型、公共要求、沙箱、命令时，相关项和后续项重新执行；前序项的报告丢失也会重新执行后续项。
- 仅在末尾追加任务，已完成项保持跳过。修改 `timeoutMinutes` 不会重跑成功项。
- 每次尝试都有独立目录，旧日志不会被覆盖。状态不跟踪任意外部文件改动；若验收后要返工，在原任务中明确补充返工要求，或追加一个修复任务。
- 不创建 Git 提交或自动回滚；重新执行会面对当前文件，包括上一次未完成的修改。任务应根据当前代码增量修复。

## 5. 验收记录

默认目录已由仓库的 `output/` 忽略规则排除，不进入 Git：

```text
output/codex-queue/
  SUMMARY.md                 当前清单的验收入口
  state.json                 状态、会话 ID、开始/结束时间
  report.schema.json         统一最终报告格式
  <任务 ID>/<本次尝试>/
    prompt.md                本次实际提示
    events.jsonl             Codex JSONL 事件流
    stderr.log               CLI 诊断信息
    report.json              结构化最终报告（生成成功时）
    report.md                可读验收报告（生成成功时）
```

每份报告列出实现摘要、修改文件、数据库变化、实际验证命令与结果、风险、验收标准对应情况和下一步。先查看 `SUMMARY.md`，再逐项核对代码、测试结果和需要人工操作的界面。网络错误或模型输出不符合 schema 时查看 `stderr.log` / `events.jsonl`；日志包含提示和工具结果，勿在任务中填写密钥或直接对外分享敏感日志。

## 6. 验证与来源

本次维护级工具变更按 `chore` 处理，不手动提升产品版本。入口：`npm run test:queue`；已纳入 `npm test`。不涉及 Vue 页面、Rust、SQLite 迁移或桌面权限。

当前实际 CLI 为 `codex-cli 0.163.0`，参数以本机 `codex exec --help` 及官方文档为依据：

- https://developers.openai.com/codex/noninteractive/
- https://developers.openai.com/codex/cli/reference/

请勿把使用模拟 CLI 的回归结果当成真实模型调用验证；真实会话检查与自动化结果在交付时分别说明。

## 7. 本次实现与验收（2026-10-26）

修改文件：`codex-tasks.mjs`、`scripts/codex-queue.mjs`、`tests/codex-queue.test.mjs`、`package.json`、`README.md` 和本文。无新增依赖、SQLite 变更或迁移，不手动改版本，不提交或发布。

已实际执行：

- `codex --version`：`codex-cli 0.163.0`；读取 `codex exec --help` 核对参数。
- `node --check scripts/codex-queue.mjs`：通过。
- `npm run test:queue`：19 项通过，覆盖顺序、新进程、共享文件、中文及空格路径、续跑/追加/前序失效、失败/受阻/报告缺失或损坏、超时和 SIGINT 终止进程树、重复启动锁、损坏状态清锁，以及子进程继承 stdout 时不阻塞队列。
- `npm test`：最终全量通过；131 项 Vitest、30 项发布脚本、19 项队列回归通过，1 项既有 macOS 专属检查在 Windows 跳过。
- `npm run lint`：通过。
- `npm run build`：通过，包含 `vue-tsc --noEmit`。
- `git diff --check`：通过。
- `node scripts/codex-queue.mjs --config output/codex-queue-smoke/tasks.mjs`：隔离 Git 目录中的两个真实只读 Codex 调用均完成，中文可读，第二项读取了第一项报告；真实会话 ID 分别为 `01a17671-539b-7763-a403-8ac61d1a6117` 和 `01a17673-1194-77e2-ab20-e8153ec154ea`。
- 对同一隔离配置执行 `--dry-run` 和再次正常启动：两项都显示已完成并跳过，没有重复调用模型。

真实调用最初暴露两处问题，已修复并复验：仅提供 `--output-schema` 不足以让当前服务稳定遵循字段，现同时将同一 schema 明确放入提示并在本地严格验证；Windows npm 启动器退出后可能有 MCP 进程继承 stdout，现等待进程 exit 并关闭本端流，不等待所有继承管道关闭。Windows 子进程夹具的清理改为等待异步文件删除，避免测试清理阻塞句柄释放。

验收对应：按序执行、新会话、前序成果衔接、自动续跑、最终报告及失败停止均已有自动化或真实调用证据。当前产品业务需求没有填进占位清单，也未自动执行。macOS/Linux 未实机运行；Rust、安装包及产品界面未因本次脚本变更另行验收。下一步填写清单、预览后正式启动，最终按报告逐项人工验收。
