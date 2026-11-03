# 剩余三项创作体验验收（2026-11-03）

## 范围与结果

本轮完成准确续作/全局运行提示、缓存与备份管理、模型调用诊断。开始时工作区干净，版本0.44.0；先前的视频自动草稿、逐句字幕对照试听、环境检查和单镜重新生成已在基线。无新依赖，无版本修改、提交或发布。

数据库schema16→17，新增model_calls表；备份时间使用既有app_settings。升级保留旧任务、草稿、素材和版本，回退旧程序需使用升级前备份。续作与缓存占用为派生数据，不增加持久化流程状态。

## 验收对应

| 验收目标 | 实际证据 |
| --- | --- |
| 继续创作打开正确任务，运行状态优先 | Rust运行任务优先回归；前端精确路由测试；真实WebView2从书架/全局提示打开指定视频、音频、发布任务 |
| 缓存清理可预览、取消，只删除允许项 | 前端确认/取消与过期错误回归；Rust运行中拒绝、过期令牌、登记文件/run输出保留、Windows外部链接目标保留；隔离原生实际删除并读回 |
| 清理后可继续原任务 | 实际删除72个文件，释放17,417,262字节（约16.6MiB）；旧试片不可用；同一计划重新生成并播放1920×1080、20秒试片，登记素材数不变 |
| 备份时间只在成功后更新，并在重启后保留 | Rust数据库备份成功/失败/重开；隔离完整备份成功、故意失败不更新时间、强制退出后回读；7天提醒前端测试 |
| 每次模型请求有阶段、耗时和真实用量 | Rust本机HTTP/SSE契约测试；原生SSE返回输入12/输出3/总15；无usage时NULL；HTTP524记录timeout；取消记录cancelled |
| 重启、迁移及恢复不伪造完成状态 | v16→v17回归；数据库备份恢复将running记录置interrupted，时长保持NULL；原生重启后4条模型记录和完整备份记录保留 |
| 正常编辑与使用仍可用 | 前端全量335项、Rust全量200项、lint/类型/构建/Clippy通过；设置页1440/1120/800宽度无页面水平溢出、无脚本错误 |

## 实际执行命令

Rust命令先加载`. ./.tools/env.ps1`。以下为本轮实际结果，历史验收数量不计入本表。

| 命令 | 结果 |
| --- | --- |
| `npx vitest run --maxWorkers=2` | 24个测试文件，335项通过 |
| `npm run lint` | 通过 |
| `npm run typecheck` | 通过 |
| `npm run tauri -- build --debug --no-bundle --config output/playwright/creator-completion/tauri.verify.conf.json` | 包含`npm run build`的类型/Vite构建及Windows Tauri编译通过；未生成安装包 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked --lib` | 200项通过，2项原有专项默认忽略 |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets -- -D warnings` | 通过 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | 通过 |
| `node output/playwright/creator-completion/native.mjs initial` | 真实路由/IPC/模型契约/清理/备份/响应式布局通过 |
| `node output/playwright/creator-completion/native.mjs restart` | 强制重启后模型记录与备份记录读回通过 |
| `node output/playwright/creator-completion/cache-roundtrip.mjs` | 清理、旧试片失效、原计划重生成及播放通过 |
| `git diff --check` | 通过；CRLF→LF提示不属于空白错误 |

现有Vite主包约630.46kB，仍有单包体积提示。本轮未改视频Node脚本、Python转写或发布/队列脚本，没有重跑这些专项，也未运行组合命令`npm test`，不宣称其全链通过。

## 隔离Windows桌面证据

使用独立Tauri identifier `com.shuyin.completion-verify`、真实WebView2、嵌入式生产前端、真实IPC/SQLite和本机HTTP契约服务，没有mockIPC。隔离数据库按Tauri规则位于`C:\Users\haozi\AppData\Local\com.shuyin.completion-verify\shuyin.sqlite3`；工程、音频副本、缓存、备份和截图在D盘本仓库的忽略目录`output/playwright/creator-completion/`。正式安装、用户数据库和原媒体未改写，未清理正式用户缓存。

证据文件：

- `native-initial.json`：三类精确路由、SSE含/不含usage、网关错误、取消、不泄漏原始内容、运行中/过期清理拒绝、确认清理、完整备份和三档布局。
- `native-restart.json`：强制退出后重新打开，读回4条调用记录和最近成功完整备份。
- `cache-roundtrip.json`：释放字节数、72个文件、旧试片失效、重生成成功、samePlan/noNewAssets及媒体尺寸/时长。
- `final-evidence.json`：7份本轮文档/新增段落中的9个本地链接核对，隔离库schema17、integrity=ok、外键错误0、调用记录4条、running调用0条，完整备份记录仍在。
- `diagnostics-1440.png`、`diagnostics-1120.png`、`diagnostics-800.png`：实际设置页截图；1440和800已人工查看。
- `app.ps1`：按精确测试程序路径和PID启动、强制结束、正常关闭；验收后隔离窗口已关闭，18562/18563不再监听。未结束正式应用。

脚本与媒体属于本机证据，不纳入Git。可维护回归位于`tests/maintenance.test.ts`、`tests/ai.test.ts`、`tests/projects.test.ts`及Rust的workflow/storage/diagnostics/ai测试模块。

## 实现取舍与边界

- 续作按已有活动排序，不记录最后一次纯浏览；单镜/插画AI任务只关联书籍，回到该书视频页后沿用最近编辑任务选择。等待登录/人工确认可以续作，但不冒充运行中；不会自动触发远程动作。
- 可清理范围限定已知任务cache和preview目录，全部run目录保留，不能把“可释放”当作所有可再生文件的总量。保留空目录和引擎junction是原渲染器再次使用缓存目录所必需，实际清理往返已验证。扫描期间会持有数据库连接锁，大目录可能让其他数据库操作等待。
- 最近备份时间只说明当时备份成功，不持续检查备份是否后来被移动/删除；恢复快照会恢复其较早的记录。提醒仅出现在设置页，不是自动备份计划。
- 首响应为HTTP响应头时刻，总耗时包含Provider解析/图片校验；不测首正文token、不精确拆分供应商排队时间。缺省用量为未知，不估算计费；响应完成与业务采用分开。记录仅显示最近200条，没有新增历史清理功能。
- 本机契约服务验证接口与状态，不证明真实供应商支持usage、计费准确性或模型质量。没有调用外部付费模型，没有验收NSIS安装升级、macOS/Linux实机或正式账号发布。

使用方式见[使用手册](USER-GUIDE.md)，数据格式见[数据库](DATABASE.md)，实现与官方流式usage依据见[技术设计](TECHNICAL-DESIGN.md)。建议下一步做安装升级及真实服务试用；开发分支CI和Node版本统一单独处理。
