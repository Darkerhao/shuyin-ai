# 创作流程代码优化（2026-11-14）

## 目标与范围

基于 dev / `6d813df` 的当前实现，梳理资料 → 创作方案 → 音频 → 字幕校核 → 三尺寸封面 → 分镜/成片 → 发布准备，以及手动工作台、章节隔离和失败续作。优先收敛已有调用链，不增加执行器、缓存、依赖或数据库迁移。

## 已确认的改进点

1. `workflow_snapshot` 每三秒将全部历史转换为 Rust 对象，再用 BTreeMap 和排序选出每个创作范围的最近任务。改为 SQL 按书籍/章节选出最近任务，并保留所有运行中任务；沿用原优先级、草稿时间及归档过滤。
2. `project_progress` 只核对 4:3 / 3:4，将两张导出视为封面完成，漏掉当前工坊支持的 16:9。复用现有三尺寸流程定义，核对三张实际文件和对应草稿。发布平台本身仍只要求横竖两张，不改变发布任务契约。
3. 自动内容校核、渲染和发布检查在同一次校验中重复读取并解析相同视频计划、来源版本。直接返回并复用当前数据库连接内已校验的数据；保持来源、字幕摘要、原声、模型结果及渲染摘要检查。
4. `automation_snapshot` 的三个界面消费者只使用未结束流程，原查询却每次加载最近 30 条流程的完整冻结输入。改用 `active_automation_runs` 仅查询未结束流程，去掉与活动查询无关的历史数量限制；历史继续通过原按 ID 读取接口访问。

## 验收标准

- A1：每个整书/章节范围仅一个续作入口，运行任务优先，全部运行任务仍可见；相同时间使用原任务 ID 排序，归档项目不展示。
- A2：三张封面全部存在才显示 3/3；16:9 的未保存修改、缺失文件不能显示完成；整书和章节结果隔离。
- A3：自动制作仍复用原音频、字幕、封面和视频；字幕变化及渲染计划变化继续被拒绝，暂停、取消、有限重试与原发布交接通过回归。
- A4：在隔离数据库比较优化前后快照结果与查询返回行数；运行适用的 Rust/前端回归、lint、类型检查和构建。
- A5：自动任务快照排除已完成/已停止历史，运行、暂停、待处理和等待发布继续展示；历史数据和按 ID 查询不受影响。

## 验证记录

### 根因复现与回归

- `cargo test --manifest-path src-tauri/Cargo.toml --locked database::workflow::tests -- --nocapture`：先复现三个封面断言失败，旧结果为 2/2、预期为 2/3 或 3/3。首次另有一个新增夹具违反视频单运行约束，已改用允许同时存在的文本/图片任务。修复后本组 8 项通过。
- `cargo test --manifest-path src-tauri/Cargo.toml --locked automation_snapshot_only_loads_active_runs_without_losing_history -- --nocapture`：先复现返回 30 条而非 1 条活动流程；修复后包含于全量 Rust 回归并通过。40 条历史记录仍可按 ID 查询，取消活动流程后快照为空。
- 已有自动流程回归验证字幕改变拒绝、分镜摘要变化拒绝、三张背景各调用一次、局部导出失败续作、同一视频任务恢复、暂停与取消、模型重试和切换、账号/合集及发布交接。未删除或放宽原断言。

### 隔离查询实验

实际执行 `python output/workflow-optimization/benchmark.py`，从 `git show HEAD:src-tauri/src/database/workflow.rs` 提取旧 SQL，与工作区最终 SQL 对比；用当前仓库迁移创建内存 SQLite，填入 20 本书、40 个整书/章节范围、20,000 条历史和 80 条活动任务。

| 指标 | 优化前 | 优化后 |
| --- | ---: | ---: |
| SQL 返回行数 | 20,080 | 80 |
| 续作入口 | 40 | 40 |
| 运行任务 | 80 | 80 |

完整结果及排序逐项相等，返回行数减少约 99.6%。这是 Python SQLite 3.45.3 的隔离查询和行处理实验，不能作为桌面端延迟或整段制作速度的保证；用时受同时运行的测试影响，不用它宣称固定加速倍数。脚本及报告在 Git 忽略目录 `output/workflow-optimization/`。

Rust 实际使用锁定 `rusqlite 0.40.2` / `libsqlite3-sys 0.38.2` 自带 SQLite 3.53.2。已核对该版本 `query_map`、`Row::get` 源码/内嵌文档及 SQLite 窗口函数实现，并查阅 [SQLite 官方窗口函数说明](https://www.sqlite.org/windowfunctions.html)。未变更依赖。

### 全量检查

Rust 命令先执行 `. ./.tools/env.ps1`。前端单测限定两个 worker。视频/发布/队列测试仅在当前命令的 PATH 前置 `D:/MyWork/software/shuyin-video-node/v24.22.2`；视频集成用 `SHUYIN_TEST_FFMPEG=D:/MyWork/project/JIAZI-PROJECT/whiteboard-teacher-video/node_modules/ffmpeg-static/ffmpeg.exe`，实际版本 6.1.1。

| 实际命令 | 结果 |
| --- | --- |
| `npx vitest run --maxWorkers=2` | 37 个文件、536 项通过 |
| `npm run test:rust` | 302 项通过，2 项原有忽略 |
| `npm run lint` | 通过 |
| `npm run build` | TypeScript 检查和 Vite 生产构建通过 |
| `npm run check:rust` | fmt / 全目标 Clippy（`-D warnings`）通过 |
| `npm run test:video` | 使用上述 Node / FFmpeg 后，46 项通过、0 跳过 |
| `npm run test:release` | 106 项通过、1 项 macOS 专属检查跳过；仅使用隔离测试仓库，无真实远端发布 |
| `npm run test:queue` | 19 项通过 |
| `git diff --check` | 通过 |

默认终端 Node 24.11.0 的首轮视频测试被项目原有的 Windows 版本保护拒绝；依据当前运行时检查和项目已安装工具切换至 24.22.2 后重跑通过，未修改版本保护或机器全局配置。npm 输出本机旧 `python` / `msvs_version` 设置警告，Vite 保留入口包大于 500 KB 的提示；本轮未改这些配置。

### 独立评审与验收

- Standards：0 项可操作发现；确认无新增缓存/状态/依赖、无查询或读取副作用，重复读取已收敛。
- Spec：0 项可操作发现；确认 A1—A5 对应实现、原排序和来源范围、三尺寸状态、历史保留及校验边界。
- A1：运行优先、同时间 ID 排序、每范围一个入口、全部运行任务与归档过滤由数据库回归和查询结果比对通过。
- A2：整书/章节 3/3、16:9 草稿变化和文件缺失由回归通过；发布任务的双封面要求不变。
- A3：本轮全量 Rust / 前端 / 视频回归通过，覆盖原流程恢复与产物校验。
- A4：隔离 SQL 实验、前端/Rust/视频/发布/队列回归、lint、类型、构建、fmt/Clippy 均通过。
- A5：活动查询及取消后空快照通过，历史仍可读取；暂停、待处理、等待发布均保留在原状态筛选中。

正式数据库、真实模型/网站账号和安装版验收未执行，本地回归不替代这些场景。未新增 SQL 迁移、IPC 命令、依赖或权限，未提交、推送或升级版本。下一步建议在包含本轮改动的 Windows 桌面版检查整书/章节续作入口和三尺寸状态。

## 修改文件

- `src-tauri/src/automation.rs`：快照命令调用活动流程查询。
- `src-tauri/src/database/automation.rs`：活动查询及校核数据复用。
- `src-tauri/src/database/automation_tests.rs`：活动查询回归及原调用更新。
- `src-tauri/src/database/workflow.rs`：SQL 续作筛选、三尺寸完成判断及对应回归。
- `docs/PRD.md`、`docs/TECHNICAL-DESIGN.md`、`docs/DATABASE.md`、`docs/DEVELOPMENT-PLAN.md`：同步当前行为与验证入口。
- `docs/WORKFLOW-OPTIMIZATION.md`：本次方案、验收、评审和验证记录。
