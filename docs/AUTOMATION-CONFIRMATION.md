# 自动发布准备的一次确认与账号复用

日期：2026-11-15。

## 问题与根因

自动制作开始时已经确认账号和合集，进入发布阶段仍会出现「使用这个账号继续」。原实现将任务目标账号持久化到 `automation_runs.input_json.account`，但只在 `DouyinPublisher.confirmed` 内存中保留窗口确认；窗口关闭、重新登录或应用重启会清空这份确认。`Session::use_account` 即使核对到同一目标身份，也只接受内存确认，因此又进入 account 等待。原会话复用和自动执行器继续路径还会原样返回这个等待，不能自行重新核对。

发布准备页的后端检查清单已允许冻结自动材料通过，前端仍无条件显示「确认发布标题和两张封面」，造成额外的重复确认印象。

## 当前行为与影响范围

- 自动制作开始区和未编辑的自动发布材料区提供「自动沿用启动时确认的账号，不重复确认」，默认开启，本机记忆，适用于当前和后续自动任务。
- 开启后，以官方当前可读身份匹配本条任务冻结账号，沿用启动确认和本次合集。重开窗口、重新登录或重启恢复后，身份一致即可继续；昵称相同不足以通过。关闭后保留原窗口内一次确认模式。
- 等待账号的原会话在明确继续时重新核对，自动执行器继续原任务；不另建会话调度器或上传实现，不清空已发视频的记录。
- 未修改的自动材料显示已校验及「继续辅助发布」，不补写人工确认；手动编辑后恢复原核对。
- 普通手动发布、账号变化/无法读取、草稿、合集歧义、发送结果不明等仍走原处理。已开始上传但会话丢失时继续拒绝自动重发。最终发布、声明和实际发布记录由用户完成。

数据库仍为 v25，仅增加 `app_settings.automation_account_reuse` 布尔文本设置，无新表、列、迁移或依赖。账号来源、发布任务及人工确认字段保持原模型。此次没有操作用户正式数据库、替换正在运行的应用或发布真实作品。

## 验证

修复前实际运行以下测试，均复现对应问题：

- `cargo test --manifest-path src-tauri/Cargo.toml --locked --lib automatic_account_confirmation_survives_a_reopened_window`：同账号、没有内存确认时失败。
- `npx vitest run tests/publishing.test.ts -t 'does not ask to reconfirm materials'`：已自动校验材料仍显示人工确认提示，断言失败。

修复后的实际验证：

| 命令 | 结果 |
| --- | --- |
| `npm run lint` | 通过 |
| `npm run typecheck`、`npm run build` | 通过，生产构建有既有的 500 kB 分块提示 |
| `node node_modules/vitest/vitest.mjs run --maxWorkers=2` | 38 个文件、546 项通过 |
| `npm run test:video` | 46 项通过，包含真实 FFmpeg 检查 |
| `npm run test:release` | 107 项通过、1 项 macOS 平台检查按原设置跳过 |
| `npm run test:queue` | 19 项通过 |
| `npm run check:rust` | fmt / Clippy 全目标通过 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked --lib -- --quiet` | 315 项通过、2 项按原设置忽略 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked --lib account` | 8 项通过，覆盖相同身份复用、不同/未知身份拦截、关闭模式及设置恢复 |
| `git diff --check` | 通过 |

前端最终全量命令与 Node 脚本使用本机已有 `D:/MyWork/software/shuyin-video-node/v24.22.2/node.exe`；视频测试设置 `SHUYIN_TEST_FFMPEG=D:/MyWork/software/ffmpeg/current/bin/ffmpeg.exe`。Rust 命令先加载 `. ./.tools/env.ps1`。Rust/Node 媒体测试的临时目录为仓库 `output/account-confirmation-test-temp`，仅包含测试数据。

最初的 `npm test` 默认并行运行中曾出现旧更新界面等测试的 5 秒超时，单独复核更新测试 16 项通过；不修改断言或超时，最终通过限制 Vitest worker 数量完成全部测试。系统默认 Node 24.11.0 还被已有 Windows 中文路径保护拒绝，改用上述已安装的固定版本后，视频 46 项全部通过。没有修改用户系统 Node 配置。

使用 `ego-browser` 与真实源码组件进行隔离页面验收：开关默认开启、鼠标关闭、刷新后保留、键盘重新开启、未修改材料直接继续、编辑标题后恢复核对、还原后恢复自动校验，以及深浅两主题和 900px 窗宽均通过；仅发起一次模拟辅助准备，没有写入人工材料确认或发布成功记录，页面错误为 0。截图位于 `output/account-confirmation-review/{setup-dark,publishing-dark,publishing-light}.png`，本次浏览器空间与预览服务已结束。夹具不连接真实抖音账号，不代替原生桌面端到端验收。

两个既有 Rust 忽略项分别是原生系统凭据回环和需显式 `SHUYIN_TEST_NODE` 的视频进程取消验证，本次未改动其设置。

## 验收对应

| 标准 | 证据 |
| --- | --- |
| 开始确认后不再重复询问相同账号 | 修复前失败回归转绿，重开窗口后从任务冻结身份接续 |
| 开关记忆并适用于后续任务 | SQLite 关闭/开启、重启及备份恢复测试；页面刷新复用 |
| 旧的账号等待可继续 | 原会话重新核对测试，视频发送标志保持 |
| 自动材料不再要求重复确认 | 发布页续作与导出测试；修改后恢复核对测试 |
| 不把另一个同名账号当作原账号 | 身份变更及未知指纹回归拦截，普通手动任务仍走原确认 |
| 最终发布仍由用户完成 | 页面停在待人工发布，模拟发布成功写入为 0 |

## 修改文件与交付边界

- 界面与服务：`src/components/AutomationAccountPreference.vue`、`src/components/AutomationSetup.vue`、`src/modules/publishing/PublishingPage.vue`、`src/services/automation.ts`。
- 后端与命令权限：`src-tauri/src/douyin_flow.rs`、`src-tauri/src/automation.rs`、`src-tauri/src/database/automation.rs`、`src-tauri/src/lib.rs`、`src-tauri/build.rs`、`src-tauri/permissions/automation.toml`。
- 回归：`tests/automation-account-preference.test.ts`、`tests/automation.test.ts`、`tests/publishing.test.ts`、`src-tauri/src/database/automation_tests.rs`，以及原 `douyin_flow.rs` 内测试。
- 文档：本文件及 `docs/{PRD,TECHNICAL-DESIGN,DATABASE,AI-RULES,DEVELOPMENT-PLAN}.md`。

本轮本地验证未打包、安装或替换正在使用的桌面程序，也未执行真实账号上传、发布及跨平台验收。发布影响为新增可记忆的账号复用能力（`feat` / minor），版本交由原发布流水线处理。更新到包含修复的构建后，原账号等待且原发布会话仍在的任务可开启开关再点击「继续流程」；已经上传但会话丢失的任务仍需先核对原草稿，避免重复上传。
