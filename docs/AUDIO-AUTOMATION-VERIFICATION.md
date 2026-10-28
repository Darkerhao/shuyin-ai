# Gemini Notebook 自动音频验收（2026-10-28）

## 评审修复记录（2026-10-28）

针对 `HEAD~5..HEAD`（`5e09500` 之后至 `6a2e314`）的三项评审问题：

- Windows 桥接协议、帧处理及请求处理按平台编译；数据库桥接/归档接口与辅助函数使用 `cfg(any(target_os = "windows", test))`，跨平台保留数据库回归。共享命令所需的错误持久化接口继续跨平台编译，不屏蔽 Clippy 警告或调整依赖来绕过检查。
- 注册清理从 `NSIS_HOOK_PREUNINSTALL` 移到 `NSIS_HOOK_POSTUNINSTALL`，继续跳过原地更新。实际生成的 `src-tauri/target/debug/nsis/x64/installer.nsi` 在第809行执行运行中应用检查，第905行调用卸载完成钩子；检查取消路径会 `Abort`，因此不会执行清理。已生成本地 NSIS 调试包，未运行安装器或操作正式注册表。
- 删除任务级“来源已校验”缓存及可跳过哈希校验的参数。Rust 在首块和末块核对原文件 SHA-256，扩展始终使用冻结大小/摘要，并校验完整组装字节后才触发上传。原文件同大小改写或中途改写后恢复均不能将混合内容上传；原生快照错误的文字提示正常传递给用户。现有错误上报流程将任务转为 `waiting_user`。

修复前新增回归已失败：末块仍读出同大小改写后的内容；扩展接受中途改写后恢复的混合分块以及被替换的冻结指纹；卸载清理仍位于确认前。修复后相关回归通过，网页文件输入事件在校验失败时未触发。

| 本轮实际命令 | 结果 |
| --- | --- |
| `node --test tests/release-assets.test.mjs tests/audio-package.test.mjs tests/audio-extension.test.mjs` | 44 项通过，1 项 macOS 专用检查跳过。 |
| `node --test --test-timeout=180000 tests/release.test.mjs` | 18 项通过；完整版本发布夹具耗时约291秒，单项均未超时。 |
| `npm run lint` | 通过。 |
| `. ./.tools/env.ps1; npm run check:rust` | Windows 上 fmt 与 Clippy `--all-targets -- -D warnings` 通过。 |
| `. ./.tools/env.ps1; npm run test:rust` | 109 项通过，1 项既有原生凭据测试忽略。 |
| `. ./.tools/env.ps1; npm run desktop:build -- --debug --bundles nsis --config output/audio-review-tauri.json` | TypeScript、Vite、Windows helper 与应用构建、NSIS 打包通过。临时配置仅关闭本地调试包的更新签名产物，不改正式配置。 |

Rust fmt/检查首次受限执行遇到已有文件或 Cargo 构建锁访问拒绝，随后在本机环境重跑通过；桌面构建使用同一本机环境。`npm run test:release` 首次受限运行长时间无完整结果后中止，上表拆分执行的四个测试文件合计62项通过、1项跳过。Rust 与版本发布夹具使用仓库 `output/audio-review-test-temp` 作为临时目录。本机仅安装 Windows Rust 目标；只读检查 Ubuntu WSL 后确认缺少 Rust 与 GTK/WebKit 开发依赖，未安装新工具链，未执行 Linux 或 macOS 检查。对应平台仍须由 CI 实跑确认。本轮未执行真实 Google 登录、上传、生成、下载，也未进行取消卸载的人工安装验收。

本轮仅修改音频桥接、来源分块读取、扩展校验、卸载钩子、对应测试和设计/验收文档；schema v13 不变，没有迁移或修改正式用户数据库，没有新增依赖、提交、推送或发布。下一步复跑四平台 CI，再以隔离安装版完成取消卸载检查及用户指定资料的一次真实 Google P0 链路。

修改文件：`src-tauri/src/audio.rs`、`src-tauri/src/database/audio.rs`、`browser-extension/content-core.js`、`src-tauri/windows/installer-hooks.nsh`、`src-tauri/src/database/audio_tests.rs`、`tests/audio-extension.test.mjs`、`tests/audio-package.test.mjs`、`docs/TECHNICAL-DESIGN.md`、`docs/DEVELOPMENT-PLAN.md`、本文件。本地未跟踪构建配置 `output/audio-review-tauri.json` 仅用于关闭调试包更新签名产物。

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
