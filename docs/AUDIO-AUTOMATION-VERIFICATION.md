# Gemini Notebook 自动音频验收（2026-10-28）

## 当前状态

Windows + Chrome 自动音频的代码与本地自动化验证已完成。真实 Google 登录后的页面操作与一次完整音频生成仍未验收；本地协议夹具不能证明实际页面控件、账号能力、下载目标或成品可播放。

P0 真实验收须先由用户指定试验书籍资料，并确认使用一次音频生成额度。当前浏览器检查连接未能初始化，本轮没有打开网站，也没有创建笔记本、登录、上传资料、生成或下载。

## 本地验证

| 命令 | 结果 |
| --- | --- |
| `npm test` | Vitest 178 项通过；发布与打包测试 58 项通过、1 项 macOS 专用检查跳过；队列测试 18 项通过、1 项失败。 |
| `npm run test:queue` | 同一队列用例再次失败：`a missing predecessor report reruns that task and its completed successors`。当前工作区未修改该队列脚本或测试，本轮不在音频范围内修复。 |
| `npm run lint` | 通过。 |
| `npm run build -- --outDir output/audio-build-continue` | TypeScript 检查与隔离目录下的 Vite 生产构建通过。 |
| `npm run build` | TypeScript 检查与标准 `dist` 生产构建通过；首次受限执行因清理 `dist/assets` 收到 EPERM，重跑成功。 |
| `. ./.tools/env.ps1; npm run test:rust` | 109 项通过，1 项既有原生凭据库测试按默认设置忽略。 |
| `. ./.tools/env.ps1; npm run check:rust` | Rust 格式检查与 Clippy 通过。 |
| `cargo fmt --manifest-path src-tauri/browser-bridge/Cargo.toml --check` | 通过。 |
| `cargo clippy --manifest-path src-tauri/browser-bridge/Cargo.toml --locked --all-targets -- -D warnings` | 通过。 |
| `cargo test --manifest-path src-tauri/browser-bridge/Cargo.toml --locked` | 3 项通过。 |

首次受限 Rust 测试无法打开项目现有 `src-tauri/target/debug/.cargo-build-lock`；使用项目本机工具链重跑通过。扩展协议、页面适配和打包测试由 `npm test` 的发布/打包阶段执行，58 项通过、1 项 macOS 专用检查跳过。

## 尚待 P0 真实验收

在隔离的 Windows 安装环境中，用用户指定的书籍资料和专用 Chrome 账号逐项验证：

1. 用户可在正常 Chrome 窗口完成人工 Google 登录，扩展能识别登录状态。
2. 任务专属 Notebook 创建成功，文本或用户选定的 PDF/EPUB 来源上传并显示就绪。
3. 完整音频指导及格式、语言、网站提供的时长档位在真实页面写入并回读一致。
4. 用户确认后仅提交一次真实生成；长时间等待、应用/Chrome 重开后只读恢复原任务，不重复提交。
5. 下载动作绑定到该 Audio Overview 的确切下载项；最终文件真实格式有效、可播放，并登记到正确书籍。
6. Native Messaging helper 在安装版可启动，扩展/宿主注册与安装说明符合目标环境。

正式扩展商店分发、正式签名安装升级/卸载、macOS/Linux 自动化以及正式用户数据库迁移未在本轮验收。正式扩展分发不等同于本地开发验证。

## 变更与数据

本轮新增此验收记录，并更新 `task_plan.md`、`progress.md` 与 `findings.md`；没有修改业务源码或数据库。仓库已有 schema v13 迁移与回归测试，但没有对正式用户数据库执行迁移。工作区原有未提交改动全部保留；未提交、推送或发布。
