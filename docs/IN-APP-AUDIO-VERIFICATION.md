# 应用内自动音频实施与验收

## 添加来源与上传文件修复（2026-10-30，源码 0.24.0）

用户确认当前网站的文件流程为「来源面板顶部添加来源 → 弹窗内上传文件」。现场截图显示笔记本未命名、0 个来源；之前只显示 `UiError` 的原始触发原因不能仅凭截图还原。本轮针对确认的入口流程、命名阻塞和错误丢失进行修改，不将模拟页面证据等同于真实 Google 服务验收。

### 实现与取舍

- 保留创建意图、先登记笔记本 URL、旧任务明确确认和精确身份核对；移除自动改名及猜测创建弹窗的分支。标题由网站或用户管理，旧命名阶段可核对原 URL 后进入上传。
- 创建按钮只点击一次，已有创建标记时等待跳转，由原有阶段超时终止等待，不重复新建。
- 依据来源标题限定面板，点击顶部添加入口；同面板空态同名入口不再制造歧义，多个不能区分的面板仍停止。
- 必须等待来源弹窗并点击其中唯一的上传按钮，仅选择该弹窗内唯一的文件输入框；不在弹窗缺失时使用页面其他文件输入框。资料文本仍走「复制的文字」。
- `runStep` 捕获异步异常并回传错误代码与完整消息，Rust 仍经 `audio_step_failed` 处理，保留添加来源/提交结果不明时的只读核对规则。

修改文件：`src-tauri/src/audio_page.js`、`src-tauri/src/audio.rs`（含 Rust 回归）、`tests/audio-page.test.mjs`；同步 `docs/PRD.md`、`docs/TECHNICAL-DESIGN.md`、`docs/DATABASE.md`、`docs/USER-GUIDE.md`、`docs/DEVELOPMENT-PLAN.md` 与本报告。无新依赖、权限、表、列或迁移，SQLite 保持 v13；按 fix/patch 处理，未手动升版、提交、发布或替换安装版。

### 实际验证

| 命令 | 结果 |
| --- | --- |
| `node --test --test-name-pattern='recorded notebook proceeds\|missing notebook title\|file upload follows\|file upload never skips\|ambiguous upload controls\|multiple unidentified' tests/audio-page.test.mjs` | 修改实现前 6 项失败，覆盖命名阻塞、来源入口、跳步上传和错误回传 |
| `node --test --test-name-pattern='waits for navigation' tests/audio-page.test.mjs` | 修复前失败，实测会再次点击新建 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked audio::tests::page_errors_preserve_the_code_and_complete_message -- --nocapture` | 修改宿主解析前失败：结构化错误被当作成功结果 |
| `node --test tests/audio-page.test.mjs tests/audio-package.test.mjs` | 最终 41 项通过，其中页面 37、打包权限 4 |
| `npx vitest run tests/ai.test.ts` | 65 项通过 |
| `npx vitest run` | 14 文件、223 项通过 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked audio:: -- --nocapture` | 26 项通过 |
| `npm run test:rust` | 127 项通过，1 项既有原生凭据测试忽略，含数据库迁移与恢复 |
| `npm run lint`、`npm run typecheck`、`npm run check:rust` | 通过；check:rust 包含 fmt 与全目标 Clippy |
| `npm run tauri -- build --debug --no-bundle --config output/playwright/audio-upload/tauri.json` | 最终独立标识桌面构建通过，包含 TypeScript/Vite 构建；保留约 516 kB 分块提示 |
| `node output/playwright/audio-upload/run-native.cjs` | 两条隔离真实 WebView2 / Tauri IPC / SQLite 链路通过，网站和音频内容为本地模拟 |
| `git diff --check` | 通过 |

Rust 命令加载 `. ./.tools/env.ps1`，TEMP/TMP/SQLITE_TMPDIR 指向 `output/audio-upload-test-temp`；首次 Rust 测试因沙箱无法打开既有 Cargo 锁而未执行，经工具审批后重跑。最终桌面二进制已校验包含当前完整页面脚本与独立应用标识，不使用旧构建冒充验证。

### 原生证据与边界

复用既有 native 验证脚本，隔离夹具包含无标签的未命名标题、来源面板两处添加按钮、来源弹窗和无关文件输入框。文件任务点击顶部添加入口及上传按钮各 1 次，空态入口点击 0 次，原生选择读回中文测试文件完整内容；资料文本任务点击添加入口 1 次且未点击上传文件。两条任务均只提交 1 次模拟生成、原生下载 844 字节 blob WAV，字节一致并各登记唯一音频素材。标题保持未命名，无关文件输入框保持空。报告与截图位于 `output/playwright/audio-upload/`，已查看文件任务主窗口截图，显示归档完成和 1 个音频。

测试书籍和文件位于仓库 output，桌面框架的独立 SQLite/WebView 资料位于 `%LOCALAPPDATA%/com.shuyin.audio-upload-verification-20261030`（C 盘）；未接触正式任务、Cookie 或账号资料，没有上传用户真实原书。未运行整体 `npm test` 或队列测试，未构建正式安装包；真实 Google EPUB 上传、生成/下载、旧任务升级恢复与 macOS/Linux 仍未验收。下一步构建正式安装版后，在原任务中继续并核对实际网站流程。

## 历史验收

日期：2026-10-29。平台：Windows x64 / WebView2。源码版本字段：0.21.0；SQLite：v13。

## 笔记本身份登记与恢复修复（2026-10-29，源码 0.22.3）

已复现：旧页面脚本在命名前清除创建标记，命名失败时既没有保存 URL，也失去重试依据；继续/重启会转 `verify_remote`，宿主因 URL 为空停止。新增页面回归先失败（期望 `notebook_identified`，实际为 `page_changed`）；新增 Rust 回归在旧实现因未接受 `notebook_identified` 失败。修复将地址登记放在命名前，保留命名阶段和创建标记至回读成功；地址已登记时命名失败或重启可继续命名。

旧任务无地址时，「继续跟进」刷新窗口快照，通过现有 `ConfirmAction` 显示当前 URL，用户确认后宿主只读检查精确窗口/URL、文档代次、0 来源和无音频结果，再在原行保存地址并恢复命名。已有 URL、提交记录、成品、过期运行或另一活动任务均不能被重新关联。停止也改用同一页面内确认组件，取消/Escape 不调用任务控制。无新依赖、权限、表、列或迁移，SQLite 保持 v13，按 fix/patch 处理，不手动升版本。

| 实际命令 | 结果 |
| --- | --- |
| `node --test tests/audio-page.test.mjs tests/audio-package.test.mjs` | 33 项通过；包括命名前登记、命名失败保留标记、空白/非空笔记本判断及既有不重复生成回归 |
| `npx vitest run tests/ai.test.ts` | 最终 65 项通过；关联确认、取消/Escape、原任务停止再继续及既有业务回归 |
| `npx vitest run` | 最终 219 项通过，14 个测试文件 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked audio:: -- --nocapture` | 24 项通过；命名失败/重启、旧任务原行恢复、旧运行拒绝、地址替换拒绝、已提交任务拒绝及原迁移/归档回归 |
| `npm run lint` | 最终通过 |
| `npm run typecheck` | 通过；最终桌面构建再次包含该检查 |
| `npm run check:rust` | fmt / 全目标 Clippy 通过 |
| `npm run tauri -- build --debug --no-bundle --config output/playwright/in-app-audio/tauri.json` | 最终独立标识桌面程序构建通过；包含 `npm run build`，保留约516 kB分块提示 |
| `node output/playwright/audio-identity/run-native.cjs` | 最终通过：真实 Tauri IPC / WebView2 / SQLite，提前保存身份、命名失败后原页恢复、新建仅1次、真实页面内取消/确认按钮、拒绝非空笔记本及变化网址、同一任务恢复，`submittedAt=null` |
| `git diff --check` | 通过 |

Rust 命令先加载 `. ./.tools/env.ps1`，TEMP/TMP/SQLITE_TMPDIR 指向 `output/audio-identity-test-temp`。Cargo 构建锁在普通沙箱中不可写，经工具审批运行相同回归与构建；没有更改权限配置。

桌面使用 `com.shuyin.audio-in-app-verification-20261029` 隔离标识及本地网站夹具，未改正式安装或用户数据库。首次夹具更新旧任务时发生 SQLite 写锁竞争，为测试连接设置 busy_timeout 后继续。桌面还证实 Tauri Dialog 2.8.3 的 `src/init-iife.js` 将 `window.confirm` 覆写为异步函数，而主窗口未授予 message 权限；最终复用页面内确认，不增加原生消息框权限，也没有模拟最终确认按钮。结果与截图在 `output/playwright/audio-identity/{result.json,confirm-notebook.png,recovered-card.png}`。

验收对应：原笔记本身份不再依赖命名成功、命名失败及重启可恢复、旧无地址任务有明确人工恢复路径、取消不改任务、来源/生成状态不明不重复提交均已本地覆盖。真实 Google 控件、上传/生成/下载、原生外部账号、正式安装升级及 macOS/Linux 未在本轮验收；未提交、推送或发布。下一步将修复打包进正式版本后，在当前旧任务中核对网址并继续。

## 网站刷新与故障恢复补充（2026-10-29，源码 0.22.2）

截图中的 `Error: Not Found` 已通过实际页面脚本复现：修复前 `node --test --test-name-pattern='website 404' tests/audio-page.test.mjs` 失败，返回 waiting 而非 page_changed。原实现没有错误页检测，查看网站只显示原窗口，已停止任务没有继续入口，停止状态还展示持续等待时间。

最小修复新增任务卡“刷新网站”和“重新打开”，复用原窗口及登录 profile；已记录原笔记本则使用原 URL，尚未创建则使用首页。刷新正常页调用 Tauri 2.12.5 reload；空白、浏览器错误、地区限制和显式重开调用 navigate。依据本机锁定版本 `tauri-2.12.5/src/webview/{webview_window.rs,mod.rs}` 的源码/API 核对，未增加依赖或权限。活动任务刷新先暂停、递增 run_id 并中断旧动作，准备好后手动继续；failed/cancelled 可恢复原行，不能抢占另一活动任务。adding_source/submitting 不明时只读核对，原始输入、来源、笔记本和成品保留。SQLite 仍为 v13，无迁移。

页面动作前检测明确的 404、403、429、500/502/503/504 标题、常见浏览器网络错误码、地区限制及可识别的人机验证，保留任务并显示恢复方法；普通未知页面仍使用既有有限等待。停止/暂停/失败不再显示持续等待。地区和账号限制仍需用户按网站实际提示处理，不承诺刷新解除限制或覆盖全部外部网站错误。

| 实际命令 | 结果 |
| --- | --- |
| `npm run lint` | 通过 |
| `npm run build` | TypeScript 与 Vite 通过；普通沙箱首次因 dist 清理 EPERM 失败，本机权限重跑通过；保留约 515 kB 分块提示 |
| `npx vitest run tests/ai.test.ts` | 59 通过 |
| `node --test tests/audio-page.test.mjs tests/audio-package.test.mjs` | 31 通过 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked audio:: -- --nocapture` | 21 通过 |
| `npm test` | Vitest 213 通过；发布/资源/页面 64 通过、1 平台跳过；队列 18 通过、1 失败，整条命令退出 1 |
| `node --test tests/codex-queue.test.mjs` | 本机权限复跑同样 18 通过、1 失败：missing predecessor report；没有改动队列文件 |
| `node --test --test-name-pattern='missing predecessor report' output/playwright/audio-recovery/baseline/tests/codex-queue.test.mjs` | 对 git HEAD 的队列脚本和测试副本复现同一失败，确认是基线问题 |
| `npm run test:rust` | 122 通过，1 既有凭据测试忽略 |
| `npm run check:rust` | 最终 fmt / 全目标 Clippy 通过；初次检查发现新增测试格式差异，格式修正后重跑通过 |
| `npm run tauri -- build --debug --no-bundle --config output/playwright/in-app-audio/tauri.json` | 独立标识桌面程序构建通过，包含最终 TypeScript/Vite 构建 |
| `node output/playwright/audio-recovery/run-native.cjs` | 真实 Tauri IPC / WebView2 / SQLite 回归通过：404 后提示、UI 真实刷新、暂停与继续、记录原笔记本、登录页刷新保留、重开原页、停止后恢复且 submittedAt 仍为 null，800px 无横向溢出 |
| `git diff --check` | 通过 |

Rust 命令先执行 `. ./.tools/env.ps1`。桌面回归仅启动独立标识 `com.shuyin.audio-in-app-verification-20261029`，使用本地夹具拦截网站请求；运行完关闭测试进程，未替换 `D:/书引 AI` 正式安装与用户数据库。第一版夹具的文档跳转中断评估，改为实际 Notebook 使用的单页路由；刷新断言等待异步 DOM 导航完成。最终原生结果 `realReload/errorDetected/stoppedTaskRecovered/loginPreserved=true`、`submittedAt=null`、`overflow=false`。截图在 `output/playwright/audio-recovery/{error-card,recovery-card,recovery-narrow}.png`。

本轮修复归类 fix/patch，不手工升版本。Google 真实账号、真实来源上传/生成下载、正式安装升级、macOS/Linux 仍未验收；未提交、推送或发布。

## 查看网站黑屏修复补充（2026-10-29，源码 0.22.1）

实际入口请求返回 `302 notebook.google.com → notebooklm.google/?location=unsupported → 301 notebook.google/?location=unsupported`，最终 200 页面标题为 Gemini Notebook。旧宿主只放行 .google.com 域名，取消重定向后停在初始 about:blank；同一任务还没有 notebook_url 时，查看网站/继续只显示旧窗口，无法恢复。调度也会把首次导航尚未提交的空白页误报为非官方网站。

最小修复：宿主、网页身份和数据库地址验证补齐 notebook.google、notebooklm.google 两个精确 HTTPS 域名；保留端口、凭据、笔记本 ID 和相似域名拒绝校验。同任务空白窗口重新导航，初始空白页在既有阶段超时内等待；Google 登录页保留。location=unsupported 显示地区不支持并保留任务，不提交生成。SQLite v13 无迁移，无新增依赖，发布类型 fix/patch，未手工递增版本。

本轮实际验证命令：

| 命令 | 结果 |
| --- | --- |
| `node --test tests/audio-page.test.mjs tests/audio-package.test.mjs` | 27 通过；新增官方域名间笔记本身份回归及相似域名/其他笔记本拒绝 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked audio:: -- --nocapture` | 17 通过；覆盖新旧四域名的保存、重开数据库及非笔记本地址拒绝 |
| `npm run lint` | 通过 |
| `npm run check:rust` | fmt / 全目标 Clippy 通过 |
| `npm run tauri -- build --debug --no-bundle --config output/playwright/in-app-audio/tauri.json` | 独立标识桌面程序构建通过，beforeBuildCommand 完成 TypeScript 和前端构建；保留既有 513.42 kB 分块提示 |
| `node output/audio-blank-fix/run-native.cjs` | 真实 WebView2 / Tauri IPC 的本地夹具回归通过；输出 initial native URL: about:blank、errors: []、loginPreserved: true、jobPaused: true |
| `git diff --check` | 通过 |

Rust 命令先加载 `. ./.tools/env.ps1`，临时目录指向 D 盘 output/audio-blank-fix。首次普通沙箱 Rust 命令因 Cargo 缓存权限失败，前端构建也因 dist 清理 EPERM 失败；获准的本机命令重跑通过。

修复前 `node output/audio-blank-fix/navigation.cjs` 已实际失败并记录查看网站仍为 about:blank。首版 HTTP 跳转夹具发现 Playwright 不拦截重定向的后续地址，不能隔离外网；最终原生回归使用本地 HTML 跳转逐个驱动真实 NavigationStarting，仅对测试进程使用临时阻塞代理固定初始空白状态。补充断言地区限制进入 waiting_user、submittedAt 仍为 null，查看同任务不覆盖登录页。截图 `output/audio-blank-fix/redirect-restored.png` 为明确标注的本地夹具。

实际网络重定向通过 Python urllib 的 RedirectHandler 只读跟踪，使用本机已有代理，未改变系统代理。回归脚本只连接 `com.shuyin.audio-in-app-verification-20261029` 的独立数据库，退出后关闭测试进程；正式安装和数据未替换，未上传原书或执行真实生成。真实 Google 账号、地区可用性和音频全流程仍需验收。

## 1. 交付结论

已按用户要求，以应用管理的专用网站窗口替换 Chrome 扩展和 Native Messaging helper。点击「开始生成音频」后，由 Rust 跟进网页阶段，自动添加来源、填写完整指导、提交生成、接收具体下载对象并登记音频素材。保留「查看网站」、暂停、继续、停止及手动导入。

**本地代码和隔离模拟网页闭环已验证；真实 Google 登录、网站控件和生成下载未验收。** 模拟音频和模拟笔记本均明确标注，不是 Google 服务结果。正在运行的正式安装没有被替换；本轮没有提交、推送或发布。

单一方案、状态流程、恢复规则和官方依据见 [设计文档](NOTEBOOKLM-IN-APP-AUDIO-DESIGN.md)。

## 2. 实现与文件范围

| 范围 | 主要文件与结果 |
| --- | --- |
| 网站宿主与调度 | `src-tauri/src/audio.rs` 管理专用窗口、串行执行、任务/运行/文档关联及有限等待；`audio_commands.rs` 提供主工作台入口 |
| 页面动作 | `src-tauri/src/audio_page.js` 复用一份中英文语义控件适配，完整回读指导，不明确时等待人工核对 |
| 上传和下载 | `webview_host.rs` 从抖音宿主整理共用；`audio_download.rs` 使用 WebView2 下载开始、完成、中断与取消事件 |
| 数据和归档 | `src-tauri/src/database/audio.rs` 保留冻结输入，制作核验后的原书副本，下载完成后校验格式、大小和 SHA-256，事务登记素材 |
| 前端 | `WritingPage.vue`、`services/audio.ts`、`types/audio.ts` 删除扩展连接门槛和安装按钮，显示当前完整网址与实际任务阶段 |
| 旧链路清理 | 删除 `browser-extension/` 的源码、`src-tauri/browser-bridge/` 工程源码、桥接构建脚本及旧扩展测试；根忽略规则继续排除 Rust 构建缓存 |
| 打包 | Tauri 配置、NSIS hooks、发布 workflow、ZIP 收集和测试删除 helper/扩展资源；升级清理仅处理精确对应本产品的旧宿主注册和指定旧文件 |
| 文档 | 同步 PRD、技术设计、数据库说明、AI 规则、使用手册、开发计划、README 和规划记录；旧音频方案保留历史标识 |

已有书籍来源绑定改动继续保留。`windows-core = 0.62.2` 由既有间接锁定改为显式声明，没有增加库版本；移除旧桥接所需的 Registry/FileSystem 直接 feature 声明。

## 3. 数据库与恢复

本次替换仍使用 **v13**，没有新表、列或迁移，也没有修改历史迁移。旧成功记录、已归档音频及发布引用保留。

- 旧 `awaiting_extension` 任务继续时转为 `opening_site`，无需扩展。
- 应用重启后，尚未开始的任务等待继续；已开始或提交状态不明的任务先核对原 Notebook，不重复生成。
- 暂停/停止使旧运行失效，旧下载回调不能写入新运行。
- 完整缓存有大小和摘要记录时可只重试归档；部分文件不作为成功结果。
- `.audio-jobs` 缓存当前保留，不进入完整备份；自动缓存清理未交付。

## 4. 实际命令与结果

Rust 命令先执行 `. ./.tools/env.ps1`。表中阶段区分前段验证与最终回归，不把未执行的检查写成通过。

| 命令 | 实际结果 |
| --- | --- |
| `npx vitest run` | 本轮前段：204 项通过 |
| `npm run lint` | 本轮前段：通过 |
| `npm run build` | 前段单独运行及最终打包的 beforeBuildCommand 均通过，包含类型检查；保留 513.42 kB 的既有分块提示 |
| `npm run test:release` | 本轮前段：54 项通过，1 项 macOS 平台测试跳过；随后补充页面用例并执行下列聚焦回归 |
| `node --test tests/audio-page.test.mjs tests/audio-package.test.mjs tests/release-assets.test.mjs` | 收尾聚焦回归：38 项通过，1 项平台测试跳过；包含 21 项页面用例和 4 项打包用例 |
| `npm run check:rust` | 收尾回归：fmt 和 Clippy 通过 |
| `npm run test:rust` | 收尾回归：117 项通过，1 项既有原生凭据测试按原配置忽略 |
| `npm run desktop:build -- --debug --bundles nsis --config output/playwright/in-app-audio/tauri.json` | 清理诊断输出后的最终隔离调试 NSIS 构建通过，产物 7,028,075 字节 |
| `python output/playwright/in-app-audio/audit.py` | 最终二进制完成四个组合并关闭后，v13、完整性、外键、四项成功记录、文件大小和 SHA-256 均核对通过 |
| `git diff --check` | 通过；另核对四份当前文档的 16 个本地 Markdown 链接，目标均存在 |

`npm test` 还包含独立的 Codex 队列测试。此前已经重复定位到未修改的 `tests/codex-queue.test.mjs` 缺少前序报告重跑用例失败，详见开发计划中的书籍来源绑定记录；本次没有修改队列实现，也没有将整个 `npm test` 宣称为全绿。

普通沙箱运行曾拒绝 Rust 格式化、构建缓存和 Vite 输出写入；使用获准的本机开发命令完成相应检查。停止隔离测试进程也需要同一执行权限；未停止正式安装实例。

最终调试包：`src-tauri/target/debug/bundle/nsis/Shuyin Audio Verification_0.21.0_x64-setup.exe`。SHA-256：`E67772D62FA6AAA698BF002C9986F507FD2F6C7CF57378662B86FFF941021BBA`。只生成隔离标识测试包，没有安装、签名发布或替换正式程序。

已实际检查生成的 `src-tauri/target/debug/nsis/x64/installer.nsi`：含旧资源清理 hook 及安装/卸载宏，没有打包 helper/扩展的 `File` 条目。旧注册和文件的清理行为由脚本检查覆盖，未操作正式注册表或用户 Chrome 资料。

## 5. Windows 原生隔离闭环

隔离标识：`com.shuyin.audio-in-app-verification-20261029`，数据库位于该标识独立的 AppData 目录。测试使用真实 Tauri 命令、Rust 调度、WebView2、原生文件输入和下载完成回调；网页内容与音频数据来自明确标注的模拟夹具。

命令：

```powershell
$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = '--remote-debugging-port=0'
# 启动采用独立配置编译的测试应用；调试端口只用于此隔离验收。
$env:SHUYIN_AUDIO_TEST_DOWNLOAD = 'blob'
node output/playwright/in-app-audio/native.cjs
$env:SHUYIN_AUDIO_TEST_DOWNLOAD = 'https'
node output/playwright/in-app-audio/native.cjs
```

HTTPS 测试使用临时自签名本机服务，测试会话接受该证书；没有安装系统证书，也没有在产品代码加入证书例外。产品没有开放远程调试端口。

| 来源 / 下载方式 | 实际结果 |
| --- | --- |
| 资料文本 / blob | 创建笔记本、添加文字、填写指导、一次生成、下载与素材登记通过 |
| 本书原书文件 / blob | 原生选择副本，文件名和中文内容一致；一次生成、下载与素材登记通过 |
| 资料文本 / HTTPS | 真实本机 HTTPS 响应触发原生下载，下载与素材登记通过 |
| 本书原书文件 / HTTPS | 原生文件输入、一次生成、HTTPS 下载与素材登记通过 |

每个任务均断言：生成计数恰为 1、完整指导一致、恰有一个音频素材、落盘字节与合成 WAV 的 844 字节完全一致。文件来源还核对上传文本与原文件一致。任务依次复用同一个网站窗口，验证切换任务无需重建同名 WebView。清理诊断输出并完成最终打包后，已重新启动最终二进制，四个组合再次通过。

最后通过仅面向隔离主窗口的原生销毁命令关闭测试应用，并确认测试进程 150716 退出、正式进程 142132 仍位于 `D:/书引 AI/shuyin-ai.exe`。关闭后只读检查 SQLite：`user_version=13`、`integrity_check=ok`、外键错误为 0，四个最终任务的成功状态、唯一音频素材、文件大小和摘要持久化一致。

本机证据位于忽略目录，不作为仓库分发资源：

- `output/playwright/in-app-audio/native.cjs`、`fixture.html`：隔离验收脚本与明确标注的模拟网页。
- `native-report.json`：首轮 blob 文本与文件成功记录。
- `native-report-blob.json`、`native-report-https.json`：最终二进制的 blob/HTTPS 文本与文件成功记录。
- `audit.py`、`audit-report.json`：最终四个组合的只读数据库、文件大小和摘要核对。
- `text-https-main.png`、`asset-https-main.png`：真实主工作台任务完成与音频列表；已查看文件来源截图，布局和操作可见。
- `text-site.png`、`asset-site.png`、`text-https-site.png`、`asset-https-site.png`：明确标注的模拟网站结果。

## 6. 验证中定位并修复的问题

| 实际现象 | 根因与最小处理 |
| --- | --- |
| 创建后无法解析网页观察 | JS 返回 `notebookUrl`，Rust 枚举字段未使用 camelCase；统一序列化契约并加回归 |
| 未进入自定义入口就填写 | 无匹配标签时，标题输入框被选作指导；显式标签必须语义匹配 |
| 第二个任务打不开网站 | 销毁后立即创建相同标签发生竞争；改为复用唯一窗口并更新任务绑定 |
| 下载尚未归档就重复处理 | 调度先执行了缓存归档重试；改为先等待当前具体下载与归档 worker |
| 下载导航使自身结果失效 | 下载也触发导航事件；仅已绑定精确 URL 允许在有效期内保持文档代次 |
| 原生下载中断 | 传给 WebView2 的目标包含 Windows 扩展路径前缀；上传、下载统一复用现有 `paths::shell_path`，中文文件成功闭环 |
| 模拟 HTTPS 不触发原生下载 | 直接拦截并返回文件的夹具未触发下载事件；改为真实本机 HTTPS 文件响应后通过，没有为该测试改产品下载逻辑 |

隔离连接还显式恢复浏览器默认下载处理，避免测试端接管下载替代原生宿主。临时诊断输出已从产品源码移除。

## 7. 验收对应与待完成项

| 用户要求 | 对应情况 |
| --- | --- |
| 应用内部打开网站 | 已实现专用应用窗口，真实 Windows WebView2 已运行 |
| 不安装扩展或 helper | 已移除连接门槛、工程源码、随包资源与构建依赖，打包配置回归通过 |
| 自动找到入口执行任务 | 已实现并以模拟中英文控件及原生桌面流程验证；真实 Google 控件待验收 |
| 成品回到本书 | 模拟 blob/HTTPS 成品校验、落盘、素材事务和列表展示通过 |
| 保留控制和失败恢复 | 页面/数据库回归覆盖暂停、失效运行、未知提交不重发、只读核对、缓存归档和旧任务继续 |
| 文档与实现一致 | 当前设计、用户说明、数据库边界和本报告同步 |

发布前仍须验证 Google 官方允许的登录路径、真实中英文页面、实际文件类型和完整提示词、一次真实生成、服务侧 HTTPS/blob 下载及长时间隐藏窗口。嵌入式登录支持不能从模拟或抖音登录经验推断；Google 拒绝时保留任务并使用既有手动导入，不绕过限制。

NSIS 构建不等于安装升级/卸载验收；本轮没有修改用户正式安装或旧 Chrome 用户资料，也未实跑 macOS/Linux 编译与桌面流程。这两个系统保留手动音频导入。下一步先在隔离新版中完成一次真实 Google 账号验收，再决定分发；不以本报告代替真实服务验收。
