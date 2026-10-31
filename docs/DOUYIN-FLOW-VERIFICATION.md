# 智能辅助流程改版实施与本地验证

## 启动误报与发布入口等待修复（2026-11-01）

用户截图为官方创作者中心已显示，但应用立即提示“当前不是抖音创作者中心，已停止页面操作”。实际调用链为 `start_douyin_task` → `open_douyin_window` → `drive` → `inspect_douyin_window` → `evaluate`。窗口创建只发起导航，不等待页面加载；首次读取可能仍是 `about:blank`，原 URL 校验立即中止，会话不会等到原有首页自动导航逻辑。Rust 回归先用该初始地址复现同一句错误；页面夹具同时复现文档仍在加载却被当作可读上传页。

修复仅区分加载与非法来源：只读检查对初始空白页/未加载文档有限等待 30 秒，保留停止与关闭检查，执行脚本和原生写入的官方来源限制不放宽。账号确认后继续使用唯一的固定官方上传地址，菜单折叠不影响导航，不新增菜单点击或兜底路径。导航完成需要上传控件就绪或可识别的编辑页、登录、草稿、弹窗等状态，不再以“不是首页”判定成功；直接进入上传页同样等待控件，不提前预占视频发送状态。

修改文件：`src-tauri/src/douyin.rs`、`src-tauri/src/douyin_flow.rs`、`src-tauri/src/douyin_page.js`、`tests/douyin-editor.test.ts`、`docs/TECHNICAL-DESIGN.md`、`docs/DEVELOPMENT-PLAN.md`、本记录。无新增依赖、权限、数据库或迁移，SQLite 保持 v13；归类 fix/patch，未改版本号、提交或发布。

### 实际验证

Rust 命令先执行 `. ./.tools/env.ps1`；全量 Rust 测试的 `TEMP`、`TMP`、`SQLITE_TMPDIR` 指向仓库 `output/douyin-startup/rust-temp`。

| 命令 | 结果 |
| --- | --- |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked douyin::tests::new_window_waits -- --nocapture`（修复前） | 失败，实际返回截图同文案的非官方来源错误，预期为等待 |
| `npx vitest run tests/douyin-editor.test.ts`（修复前） | 19 项通过、1 项失败，复现文档加载中没有等待 |
| `npx vitest run tests/douyin-editor.test.ts tests/douyin.test.ts tests/publishing.test.ts` | 48 项通过 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked douyin:: -- --nocapture` | 12 项通过 |
| `npm run lint` | 通过 |
| `npm test` | Vitest 225 项通过；发布/音频脚本 78 项通过、1 项 macOS 跳过；队列 18 项通过、1 项失败 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked` | 130 项通过、1 项既有系统凭据测试忽略 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | 通过 |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets -- -D warnings` | 通过 |
| `npx tauri build --debug --no-bundle --config output/douyin-startup/tauri-test.json` | 隔离桌面构建通过，包含 `npm run build`、TypeScript 检查及 Vite 构建；保留约 516 kB 分块提示 |
| `node output/douyin-startup/native.cjs` | 5 项隔离原生场景通过，结果为 `output/douyin-startup/native-result.json` |
| `git diff --check` | 通过 |

整体测试失败仍位于未修改的 `tests/codex-queue.test.mjs:118`：`a missing predecessor report reruns that task and its completed successors` 预期执行 `one,two,one,two`，实际 `one,two`。未删除测试或修改队列逻辑，不能将 `npm test` 写为全部通过。

### 验收与边界

隔离构建使用 `com.shuyin.douyin-startup-20261101`，新建测试数据库及工作区，不读取正式浏览器会话。测试进程临时启用本地 CDP 和阻塞代理，保证首次官方导航停留空白页；随后用本地 HTML 拦截官方页面地址，不连接真实抖音账号，不上传到远程服务，产品未增加调试端口配置。

- 初始空白页不报非官方来源：原生窗口在阻塞导航下保持 running，通过。
- 有限等待和停止：加载超时提示独立原因，重试后可在等待中停止，通过。
- 自动进入发布视频：折叠菜单首页经人工账号确认后只导航一次固定上传地址，通过。
- 等待上传控件：上传页 1.8 秒后才创建文件输入框，宿主等待后选择仓库的测试成片且仅一次，通过；不代表真实平台上传与处理完成。
- 草稿及任务保护：保留官方页面模拟草稿和弹窗，不再次导航，SQLite 任务仍 ready、没有发布记录，通过。

尚未替换用户正在运行的正式安装版，也未执行本轮 NSIS 打包、真实账号登录后的首页 DOM 兼容性、真实上传/封面/合集或最终发布验收。下一步在包含此修复的正式构建中复验用户截图场景；账号、验证码及最终发布仍由用户确认。

## 合集确认前恢复视频预览（2026-10-29）

用户截图在「确认本次合集」时报“网页视频身份无法与当前会话核对”。调用链为 `DouyinVerification.vue` → `continue_douyin_task` 的 Collection 决定 → `verify_video`，报错发生在保存、选择合集之前。

当前代码原先直接拼接页面内所有 video 的 currentSrc/src 作为标识；播放器卸载会读到空字符串，封面弹窗播放器也会被计入。本地 DOM 夹具模拟切到封面预览后卸载播放器：原代码读到空标识，恢复预览的断言失败；追加弹窗视频时也因标识多拼一个地址而失败。初次 `npx vitest run tests/douyin-editor.test.ts` 为 14 项通过、2 项失败。用户暂不便确认真实页面所处页签，因此这些证据证明代码缺陷，不能单凭截图断定这次实际发生了哪种页面变化。

修复统一在既有视频、处理完成、文案、封面、合集决定和最终核对时恢复视频页签，有限等待播放器挂载及就绪，再做原有严格身份比较；已激活页签不重点击，弹窗视频不参与标识，有可见弹窗时禁止切换。未就绪与身份不一致分别提示，不缓存旧视频标识替代当前读取，不重传视频、改写原素材或自动点击最终发布。

修改 `src-tauri/src/douyin_editor.js`、`src-tauri/src/douyin_flow.rs`、`tests/douyin-editor.test.ts` 及技术设计、本记录。无依赖、数据库迁移或版本号变化，SQLite 保持 v13，属于 fix/patch。回归覆盖恢复同一视频、不同视频保持可区分、弹窗播放器排除、弹窗禁止操作、延迟挂载和已激活页签不重复点击。

Rust 先加载 `. ./.tools/env.ps1`，TEMP/TMP/SQLITE_TMPDIR 指向仓库 `output/douyin-video-test-temp`。实际验证：

| 命令 | 最终结果 |
| --- | --- |
| `npx vitest run tests/douyin-editor.test.ts tests/douyin.test.ts tests/publishing.test.ts` | 3 个文件、45 项通过 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked douyin:: -- --nocapture` | 10 项通过 |
| `npm run lint` | 通过 |
| `npm run build` | 类型检查及生产构建通过；保留约 516 kB 分块提示 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | 通过 |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets -- -D warnings` | 通过 |
| `git diff --check` | 通过 |

首次 Rust 测试因沙箱拒绝 Cargo 构建锁而未执行，首次前端构建在类型检查通过后因沙箱拒绝清理 dist/assets 失败；经工具审批在沙箱外运行原命令通过。本轮未替换运行中的应用、操作正式数据库、执行桌面安装包构建或真实抖音账号验收；下一步在包含修复的桌面构建中复验选择合集与最终检查。

## 2026-10-27 实施记录

日期：2026-10-27。

本轮按 `DOUYIN-PUBLISHING-PLAN.md` 的智能辅助流程设计完成代码实施。用户明确选择「先完成代码与本地验证，真实账号验收稍后进行」。本记录中的自动化测试、DOM 夹具和浏览器内存 IPC 结果均不能代替本轮真实登录、原生上传、合集及安装版验收。未上传真实素材、操作真实草稿或代替用户发布；未提交、发布或修改正式版本号。

## 实现内容

- 一个「开始抖音辅助发布」入口先保存当前修改、复用现有检查并按需准备任务包，再启动宿主流程；不替用户选择素材、文案来源或勾选人工确认项。
- Rust 维护唯一会话和串行操作锁。前端先订阅事件再回读快照，按任务 ID 和运行 ID 过滤，显示连接、视频、内容、封面与合集、最终检查五个步骤。
- 宿主等待登录并导航已知的非编辑页面；账号、已有视频、不同文案、封面和合集等决策就地处理。未知页面、弹窗或旧草稿保留，等待人工处理。
- 文件发送前占用当前任务；超时结果不明时不自动重发。已核对的文字、视频身份和逐比例封面预览用于恢复；人工改动触发重新确认，并取消对应字段的旧完成标记。
- 可播放预览只证明预览可用。确认旧视频属于任务后仍需平台完成依据，缺少可核验信号时要求明确人工确认上传和处理完成。
- 合集从可见选项读取并回读；重名、失效、未加载或已有不同选择均不静默替换。人工采用结果标注为「人工确认」。账号变化时清除账号许可、选项和未确认的人工合集输入。
- 停止先设置取消标志，等待宿主正在执行的操作结束后才应答；关闭窗口清除页面证明。活动和等待期间锁定本地编辑、任务切换及导航。
- 保留人工结果记录事务。只有记录成功后才释放匹配任务，并启动用户明确选择的下一篇；失败保持原任务和记录表单。下一篇启动参数在执行前消费，刷新不重放。
- 删除被替代的上传/继续命令和前端编排；页面检查仅作为 Rust 内部读取方法，不再开放旧检查 IPC。未增加依赖或第二套发布队列。

## 修改文件

| 范围 | 文件 |
| --- | --- |
| 界面和类型 | `src/modules/publishing/PublishingPage.vue`、`DouyinVerification.vue`、`src/services/douyin.ts`、`src/types/publishing.ts`、`src/style.css` |
| 宿主及页面适配 | `src-tauri/src/douyin.rs`、新增 `douyin_flow.rs`、`douyin_page.js`、`douyin_editor.js` |
| IPC 与权限 | `src-tauri/src/lib.rs`、`publish_commands.rs`、`src-tauri/permissions/publishing.toml` |
| 数据与迁移 | `src-tauri/src/database.rs`、`database/publishing.rs`、新增 `src-tauri/migrations/0011_douyin_collection.sql` |
| 回归 | `tests/douyin.test.ts`、`douyin-editor.test.ts`、`publishing.test.ts`、`src-tauri/src/database/workflow_tests.rs`、宿主模块内测试 |
| 文档 | `docs/PRD.md`、`TECHNICAL-DESIGN.md`、`DATABASE.md`、`AI-RULES.md`、`DEVELOPMENT-PLAN.md`、`DOUYIN-PUBLISHING-PLAN.md`、`USER-GUIDE.md`、本文件 |

任务开始时已有的开发计划和抖音方案设计内容保留，增加实施状态及本记录入口。生成的浏览器临时日志和诊断测试副本已清理；截图保留在本地忽略目录 `output/playwright/douyin-flow-review.png`。

## 数据库与兼容性

schema v10 → v11 只增加可空 JSON 字段 `publish_tasks.douyin_collection`，不重建或改写旧任务包。空字段表示未选择；对象中 `name: null` 表示明确不加入合集。

任务意图和可选账号默认在同一事务保存。账号默认复用 `app_settings`，必须绑定稳定公开账号 ID；账号不同不套用。合集不进入素材 manifest，改变合集不使已校验任务包失效。已发布记录拒绝修改合集。

当前留存官方 DOM 只有昵称和头像线索，适配器明确返回 `id: null`，**未启用跨重启账号识别和默认合集复用**。头像指纹只用于当前窗口的变化检查，不能证明持久账号身份；空头像不构成身份依据。数据库默认能力已实现并测试，启用仍需真实页面稳定公开标识证据。

回归覆盖 v10 升级保留旧数据、迁移冲突事务回滚、重启、合集与原包快照独立、数据库备份恢复、旧版本备份升级、完整目录备份恢复和项目删除级联。所有数据库验证使用临时夹具；未迁移用户正式数据库。

## 实际验证命令与结果

Rust 命令先执行 `. ./.tools/env.ps1`，并将本次进程的 `TEMP`、`TMP`、`SQLITE_TMPDIR` 指向仓库的 `output/flow-test-temp`。

| 命令 | 结果 |
| --- | --- |
| `cargo check --manifest-path src-tauri/Cargo.toml --locked` | 通过 |
| `npx vitest run` | 最终 12 个文件、139 项通过 |
| `npm run lint` | 通过，零警告规则保持启用 |
| `npm run build` | 通过，包含 `vue-tsc --noEmit` 和 Vite 生产构建 |
| `npm run test:rust` | 最终 90 项通过、1 项系统凭据测试按原设置忽略；输出保留于 `output/douyin-flow-rust-tests.log` |
| `npm run check:rust` | fmt 与 Clippy `--all-targets -- -D warnings` 通过 |
| `npm test` | 未全绿：当次 Vitest 138 项通过；发布脚本 30 项通过、1 项 macOS 专属测试跳过；队列测试 18 项通过、1 项失败 |
| `node --test --test-name-pattern="a missing predecessor report" tests/codex-queue.test.mjs` | 单独复现同一队列夹具失败；正常权限下也复现 |
| `git diff --check` | 通过 |

部分沙箱内 Rust 构建或 Vite 清理旧输出遇到文件权限错误；随后在正常权限下重跑，前端构建和 Rust 检查通过。未关闭检查或更改构建配置。

队列失败位于未改动的 `tests/codex-queue.test.mjs:118`：删除前序 `report.md` 后，预期重跑两项任务，实际仍跳过。在临时诊断副本中，`rmSync` 返回后立即 `existsSync` 仍为 true，目录仍包含该文件；改用 `unlinkSync` 的对照中，文件确实消失且该用例通过。当前环境是 Windows / Node v24.11.0；底层删除行为未继续归因。本轮未改动队列脚本或其测试，不能把诊断副本通过写成正式 `npm test` 通过。

浏览器本地验证使用模拟 IPC：等待状态锁定标题编辑，进入待检查状态后允许人工记录；1440、1120、800、360px 均无横向溢出，已查看截图。最初模拟器复用可变对象导致过期显示，改用独立页面并返回克隆对象后通过；未将该模拟器问题归为应用缺陷。该验证未调用原生文件上传或真实平台账号。

## 验收对应及尚未覆盖范围

| 设计验收项 | 本轮证据与状态 |
| --- | --- |
| 保存、检查、包准备后一次开始 | 前端组件回归通过，包括保存/打包失败不启动且保留修改 |
| 登录等待、自动定位、账号确认 | 宿主流程和 DOM 夹具已实现；本轮真实登录及跳转待验 |
| 视频上传及平台处理 | 保留原生接口并区分阶段；处理信号不能证明时人工确认；本轮真实成片上传待验 |
| 文案和双封面恢复、人工修改 | 本地回读夹具及会话验证通过；原生网页恢复和裁切回读待验 |
| 合集选择、重名、失效、账号隔离 | DOM/组件/数据库夹具通过；真实选项加载和采用结果待验；持久默认尚未启用 |
| 停止、旧运行与关闭窗口 | Rust 操作屏障、过期决策、证明清理和前端锁定回归通过；实机关闭与断网待验 |
| 人工记录与下一篇 | 前端及数据库回归通过，记录失败不启动下一篇；未对真实任务写入模拟成功记录 |
| 迁移、原素材和备份兼容 | 临时数据库和文件回归通过；正式数据与安装升级未操作 |

未运行本轮 `npm run desktop:build`、安装包升级、macOS/Linux 实机验收，也未验证真实平台的章节、语义话题控件、声明或最终发布。这些设置仍由用户在官方页检查；应用不自动生成章节或点击最终发布。

## 下一步

按用户安排，在隔离 Windows 安装/开发环境中，由用户扫码并指定允许上传的成片，补验登录落点、稳定公开账号标识、视频上传/处理、双封面、真实合集选择与回读、停止/关闭/断网恢复。先确认稳定账号标识再启用持久默认；随后验证安装升级和备份恢复。最终发布始终由用户操作。
