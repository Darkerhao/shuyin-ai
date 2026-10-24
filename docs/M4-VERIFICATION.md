# M4 发布准备验收（2026-10-24）

## 范围

用户确认启动条件后要求继续实施 M4。实现书籍内的发布准备、发布草稿、双比例封面/视频选择、检查清单、完整任务包、中文复制、固定抖音入口和人工结果记录。M5 全面试用、真实平台上传和自动发布不在本轮范围。影响级别为 `feat` / minor，正式版本由既有发布流程处理。

## 实现与验收目标

| 验收项 | 实现 |
| --- | --- |
| 一键完整任务包 | 先保存草稿，检查资料及确认项，唯一新目录写入 8 个文件，再标记 ready |
| 视频引用正确 | 本项目 video 素材；保留原路径、不复制视频；路径变化必须重建任务包 |
| 缺失视频不 ready | 保存允许草稿，生成阻止；查询就绪任务重新检查文件并退回草稿 |
| 标题和封面确认提示 | UI 显式勾选，修改撤销确认；Rust 清单阻止不完整任务生成 |
| 中文复制 | 分别复制标题、内容、标签及一键复制全部，换行保留，失败有提示 |
| 人工结果记录 | 确认对话框；记录、任务 published、项目状态在同一事务提交 |
| 失败保留源素材 | 只清理本次未完成包；旧包、视频及封面保留，写入失败保留草稿 |

## 数据库

新增 v7 `publish_tasks` / `publish_records`，不重写旧业务表；源媒体和旧包均不删除。备份恢复接受 v2–v7，源备份保持原样，恢复前自动备份当前库。记录为任务文字快照与素材引用；详情见 DATABASE 第 13 节。

## 实际验证命令与结果

根目录执行。Rust/Tauri 命令先加载 `. ./.tools/env.ps1`；测试脚本和截图位于忽略目录 `output/playwright/m4/`。

| 命令 | 实际结果 |
| --- | --- |
| `npm run lint -- --fix` | 通过；统一新增 Vue 模板格式，未关闭规则 |
| `npm run build` | TypeScript 与 Vite 生产构建通过 |
| `npx vitest run` | 最终 6 个测试文件、55 项通过，其中 M4 新增 8 项 |
| `npm run lint` | 最终源码通过，0 警告 |
| `npm test` | 收尾修正前前端 54 项、发布脚本 27 项通过，1 项 macOS 专用跳过；确认期间锁定修正后前端全量 55 项另行通过 |
| `npm run test:rust` | 48 项通过，其中 M4 新增 7 项；1 项原生凭据显式测试默认忽略 |
| `npm run check:rust` | fmt 与 Clippy 全目标 `-D warnings` 通过 |
| `npm run desktop:build -- --config output/playwright/m4/tauri.json` | Windows release 与 NSIS 构建通过 |
| `python output/playwright/m4/inspect.py seed` | 从既有 v6 验收备份创建独立测试库，原备份哈希不变 |
| `./output/playwright/m4/install.ps1 -Action install` | 安装及启动成功，存在有效 MainWindowHandle 与「书引 AI · M4 验证」窗口 |
| `python output/playwright/m4/inspect.py migrated` | v6→v7；原 8 表全部旧行/字段保持，新两表为空，完整性与外键通过 |
| `node output/playwright/m4/runtime.cjs create` | 真实安装 UI 新建书籍、保存三项文案、本地双图导出、确认项校验、8 文件任务包及系统剪贴板逐字一致通过 |
| `node output/playwright/m4/runtime.cjs negative`，修正脚本后 `negative-resume` | 视频缺失→draft、重新定位→重建、目录写入失败保留草稿和源文件、打开文件/入口、取消确认和确认保存通过 |
| `./output/playwright/m4/install.ps1 -Action restart` | 正常退出并重启，进程由 3324 变为 51756 |
| `node output/playwright/m4/runtime.cjs restart` | 发布任务及记录逐字段一致，原视频哈希不变；v7 备份恢复及界面重载通过 |
| `python output/playwright/m4/inspect.py restored` | schema 7、10 张表、完整性/外键及正式数据库前后哈希通过 |
| `node output/playwright/m4/confirm-lock.cjs`，修正定位后 `resume` | 最终安装版填写结果时锁定任务编辑、素材关联、任务切换、刷新及重复确认；取消恢复编辑，随后恢复基线数据通过 |
| `python output/playwright/m4/inspect.py final` | 最终 v7 完整性、外键、旧备份及正式数据库哈希通过 |
| `git diff --check` | 通过 |

发布脚本回归耗时约 216 秒，退出 0；全程使用临时本地仓库，不向项目远端发布。最后一处仅修改确认期间的 UI 禁用状态，未改发布脚本，复跑全部前端测试及 lint。

## Windows 桌面证据

- 独立产品 identifier 为 `com.shuyin.m4-verification`，安装到 `output/playwright/m4/installed`；正式安装和正式数据库未替换。测试包沿用当前源码版本 0.5.1，不代表已发布的新版本。
- 升级基线保留 1 项目、4 素材、8 封面、3 模型、4 AI 任务和 1 创作版本；新增业务测试只在新建书籍及独立目录操作。最终为 2 项目、5 素材、10 封面、2 创作版本、1 发布任务、1 发布记录。
- 视频关联/重新定位使用真实 Tauri IPC 与独立视频副本，未重复执行原生文件选择对话框。复用素材组件的选择/取消/失败行为有既有单测与 M1 桌面验收；本轮复跑前端回归通过。
- 任务包包含 8 文件，两张封面为安装 UI 实际 Canvas 导出；中文文字和换行经系统 `Get-Clipboard -Raw` 比较一致。源视频 SHA-256 在移动/失败/发布记录/恢复后保持一致。
- 打开命令通过后检查实际系统窗口：文件资源管理器显示任务目录，GeePlayer 显示「重新定位 视频.mp4」，WPS 图片分别显示两张封面，Chrome 显示「抖音创作者中心」。未登录、上传或点击最终发布。
- 人工确认验证使用测试记录，备注明确「M4 本地确认流程验收；未向真实平台发布。」；不把测试确认表述为平台实际发布成功。
- 证据：`before.json`、`inspect-migrated.json`、`create.json`、`negative.json`、`restart.json`、`inspect-restored.json`、`ready.png`、`write-failure.png`、`published.png`、`restart.png` 与 ARIA 快照。运行期间页面错误为空。
- 最终安装包 `src-tauri/target/release/bundle/nsis/Shuyin M4 Verification_0.5.1_x64-setup.exe`，SHA-256 `8EAB202F2E5E474493D6D591B1E2B7409FC3613AF95F331C818180CFF576D464`。收尾 UI 修正后重新构建并安装至同一独立目录，已保存发布记录保持，确认保护见 `confirm-lock.json` / `confirm-lock.png`。

## 失败记录与修正

- 最终回归补充“填写发布记录时禁止继续编辑任务”，先复现失败，再锁定任务/素材表单、任务切换、刷新和重复确认按钮。取消恢复编辑，记录输入保持可编辑；新增测试通过。

- 旧“拒绝未来数据库”测试固定使用版本 7，本轮 v7 正式启用后将测试改为当前迁移数加一；保持拒绝未知版本与保留原库的断言。
- 前端新任务保存测试改为显式触发表单 submit，符合 jsdom 下未挂载到文档的表单行为。
- 桌面脚本首次定位视频时相对路径多回退一级，尚未创建业务测试项目；修正后创建流程通过。
- 异常脚本在 Windows 扩展路径上调用 `realpathSync` 失败，改为 `realpathSync.native` 后从已有检查点继续；先前视频丢失/重新定位已通过，不重复移动素材。目录变更均验证位于独立工作目录，退出时恢复。
- 重启后首次 CDP 连接早于 WebView2 启动而拒绝连接；服务就绪后重新执行重启验收通过，没有重新创建发布数据。
- 最终确认保护脚本最初用可见角色定位折叠素材区内的按钮，定位超时；改为包含隐藏元素后从原检查点续验，通过真实禁用状态断言，未修改业务逻辑。

## 修改文件

- 前端新增：`src/modules/publishing/PublishingPage.vue`、`src/services/publishing.ts`、`src/types/publishing.ts`；新增回归 `tests/publishing.test.ts`。
- 前端集成：`src/app/router/index.ts`、书籍/文案/封面页面增加入口，`src/modules/projects/ProjectAssets.vue` 增加素材变更通知供发布页刷新。
- 后端新增：`src-tauri/src/database/publishing.rs`、`src-tauri/src/publish_commands.rs`、`src-tauri/src/database/m4_tests.rs`、`src-tauri/migrations/0007_publish_tasks.sql`、`src-tauri/permissions/publishing.toml`。
- 后端集成：`src-tauri/src/database.rs`、`src-tauri/src/lib.rs`、`src-tauri/build.rs`、`src-tauri/capabilities/main.json`。
- 文档：README、PRD、TECHNICAL-DESIGN、DATABASE、AI-RULES、DEVELOPMENT-PLAN 与本报告。`output/playwright/m4/` 的脚本、截图、数据及安装包仅为本地验证产物。

## 结论与边界

M4 七项核心验收均通过；源码、数据库及业务文档已更新，无新增依赖。当前仅抖音半自动准备，发布结果来自用户确认；正式安装升级、macOS/Linux 实机和 M5 三本真实书全流程试用尚未执行。

任务包以外部视频路径为引用，不是完整媒体备份。检查使用路径、大小、修改时间与文字内容，不对大型视频计算全量校验和。断电/强制结束可能保留本次未登记目录，旧包和源素材不受影响。下一步按 M5 使用真实书籍及视频，在目标平台检查上传预览和最终人工发布结果。
