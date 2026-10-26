# M6 应用内抖音辅助发布验收

日期：2026-10-26。Windows x64 核心闭环通过；未执行最终发布，未提交或发布代码。M7 待发布队列是后续独立切片，不包含在本报告的 M6 证据中。

## 实现与影响文件

- `src/modules/publishing/DouyinVerification.vue`、`PublishingPage.vue`、`src/services/douyin.ts`：官方窗口、检查、人工核对账号、上传填写、步骤、停止接管及活动期间本地编辑/导航锁定。
- `src-tauri/src/douyin.rs`、`douyin_page.js`、`douyin_editor.js`、`douyin_webview.rs`：单活动快照、官方来源限制、原生文件选择、有限页面动作、上传等待及回读；没有最终发布动作。
- `database/publishing.rs` 复用已保存任务及包校验生成快照；`lib.rs`、`build.rs`、`permissions/publishing.toml` 登记主窗口命令。`Cargo.toml/lock` 直接声明锁文件已有 webview2-com 0.39.1，没有新增 npm 依赖。
- 前端 `douyin.test.ts`、`douyin-editor.test.ts`、`publishing.test.ts` 与 Rust `douyin.rs`、`database/m4_tests.rs` 覆盖核心保护。产品、技术、数据、使用说明和开发计划同步当前行为。

数据库仍为 schema v9，无 SQL 迁移。浏览器会话和操作进度只在专用 WebView/内存中保留；上传和填写不改为 published，发布记录仍由原有人工确认事务保存。

## 验收环境与隔离

开发验证使用独立 identifier `com.shuyin.m6-verification-20261026`。用户后来对该官方草稿添加了合集和章节，本轮保留原进程与网页，不关闭、不放弃、不重传。

安装验证使用 `com.shuyin.m6-install-verification`，安装到 `D:\MyWork\software\shuyin-m6-verification`，工作目录位于仓库 `output/playwright/m6/install-workspace`。SQLite 与专用 WebView2 数据按系统约定保存在该独立 identifier 的用户 AppData；未覆盖正式产品数据。0.14.1、0.14.2、0.14.3 均为 output 临时配置中的测试版本，正式清单仍为0.14.0。

测试进程通过环境变量临时启用 WebView2 动态 CDP 端口用于观察；产品代码通过宿主 API 上传，不依赖调试端口。不读取 Cookie、密码或授权存储，不将账号资料及签名图片 URL 写入本报告。

## 真实页面及原生命令证据

| 验收项 | 实际结果 |
| --- | --- |
| 扫码与账号 | 用户在独立安装版官方窗口完成扫码并明确确认账号；升级重开后登录态保留 |
| 原始视频 | 引用用户确认的《高性价比人生指南》MP4，265922746字节，官方预览约697.621秒；不复制到任务包、不改写源文件 |
| 页面技术试验 | 中文标题/简介及1440×1080、1080×1440封面在真实官方页回读，暂存恢复后保留 |
| 原生继续填写 | 安装版从已上传但未填写的编辑器完成中文文案、双封面和回读；官方显示封面检测通过 |
| 修正后的完整原生链 | 0.14.3安装版从空白上传页，经UI触发上传、等待处理、文案、双封面到最终回读，一次调用连续通过；结果见本地install-upload-result.json和events日志 |
| 重复请求 | 在活动操作期间直接重复调用命令，被“正在操作/已有上传任务”拒绝 |
| 已有草稿与内容 | 重开上传页保留草稿时，原生命令拒绝重传；继续编辑后原文案和双封面一致；已有封面拒绝再次自动填写 |
| 网络 | 仅对验收WebView模拟断网，检查/编辑读取明确报网络不可用，恢复联网后可检查；没有主动断开系统网络 |
| 权限 | 远程官方网页调用workspace_settings，实测被Tauri ACL拒绝；业务能力只授予main |
| 关闭及重开 | 官方草稿暂存后销毁隔离主窗口，发布窗口及隔离进程一起退出；升级后任务、包、文字和双封面保留 |
| 数据恢复 | 隔离SQLite备份真实恢复，任务逐字段相同；12张表与升级前备份逐行比较一致，schema9/完整性/外键检查通过 |
| 发布状态 | 就绪任务保持ready，publish_records为0，没有点击最终发布；首页同名已有作品不作为本轮发布证据 |
| 正式数据保护 | 正式数据库及源视频SHA-256与before.json基线一致，旧开发草稿原进程保留 |

只重置了安装版由验收流程自动生成、且已保存证据的测试草稿以复验整链；没有触碰用户开发版的人工草稿。

## 实际发现与最小修复

1. 简介恢复后不再具有空内容的placeholder属性。定位改用稳定的Slate编辑属性和实际编辑器类，不依赖空值状态。
2. 切换封面/标题预览会卸载video。需要时切回视频预览；预览页签含提示图标，不能用叶子节点文字查找。
3. 左侧文案区先于右侧预览面板挂载。首次上传实测过早查找页签导致停止；将预览页签加入editorReady判定，并补独立回归，0.14.3整链复验通过。
4. 双封面在同一弹窗依次上传后一次完成，避免仅保存横图时的竖图提示；现有不同封面不覆盖。
5. 官方隐藏充值弹窗仍有ClientRects。可见判断同时核验visibility，隐藏弹窗不误阻塞，可见弹窗仍停止。
6. 文件节点定位成功后、发送文件前绑定活动快照，避免超时后误认为尚未上传而重传。

验收脚本曾等待SPA导航的load事件超时，而路径实际已改变；改为检查目标路径。销毁主窗口时CDP连接中断，通过隔离进程消失核实关闭结果。以上没有通过放宽业务断言或弱化安全边界绕过。

## 验证命令

Rust命令先执行 `. ./.tools/env.ps1`，测试进程的TEMP/TMP/SQLITE_TMPDIR指向仓库output。

- `cargo check --manifest-path src-tauri/Cargo.toml --locked --offline`：通过。
- `cargo test --manifest-path src-tauri/Cargo.toml --locked --offline`：初轮74通过，1项既有原生凭据测试忽略；后续总数随M7回归另行记录。
- `cargo clippy --manifest-path src-tauri/Cargo.toml --locked --offline --all-targets -- -D warnings`、`cargo fmt --manifest-path src-tauri/Cargo.toml --check`：通过。
- `npm run lint`、`npm run typecheck`、`npm run build`：通过。
- `npm test`：初轮前端121通过，发布脚本30通过、1项macOS专用跳过。补齐回归后全量Vitest123通过，后续新增的两个预览/挂载回归聚焦10通过；最终完整前端总数见后续收尾记录。
- `npm run desktop:build -- --config output/playwright/m6/tauri-install.json`：独立Windows NSIS构建通过；先后测试0.14.1→0.14.2→0.14.3，同一D盘独立目录安装/升级退出码均0。
- `node output/playwright/m6/install-prepare.cjs`：首次只初始化独立书籍、素材和ready任务，已完成，不重复运行。
- `node output/playwright/m6/install-reopen.cjs`，以及`install-verify.cjs`的navigate/retained/upload/continue/recover-offline/save-close：按阶段实际执行；最终0.14.3完整上传、保护和恢复验证通过。
- Python sqlite3/哈希核验：12表数据保留，完整性ok、无外键错误；正式库和源视频哈希不变。

原始日志、临时配置、SQLite比较、截图和脚本位于Git忽略的`output/playwright/m6/`，不提交包含个人资料的网页快照。`.playwright-mcp/`是原有未跟踪工作，保留。

## 未覆盖范围与下一步

收尾更新：连同M7回归后，全量Vitest为131通过，Rust为76通过/1项既有凭据测试忽略，lint、含TypeScript的构建、fmt/Clippy和0.14.4隔离安装升级通过。M7本地模拟记录及重启证据见`M7-VERIFICATION.md`，不能将模拟记录当作本报告的真实平台发布证据。

核心验收对应M6.1真实技术试验、M6.2单任务原生闭环、M6.3重复/保稿/网络/安装恢复；停止标志、任务变化及页面缺项等保护有本地DOM、组件和Rust回归。

未执行真实账号自然过期、30分钟上传超时、长期断网、大范围平台页面升级、macOS/Linux原生上传、真实最终发布和正式安装升级；模拟网络不能代替这些实况。平台未提供网页结构兼容承诺，遇到变化停止并允许用户在官方页接管。继续M7时复用现有任务与人工记录，逐篇处理，不引入第二套状态或自动最终发布。
