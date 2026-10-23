# M1 项目与素材管理验收记录

日期：2026-10-23。

## 当前结论

M1 项目与素材管理核心验收完成：功能代码、自动化测试、真实 Tauri 业务流程、进程重启持久化、原生选择/取消、默认播放器实际播放、隔离 Windows 安装与安装版本升级均已通过。正式发布及跨平台场景仍按 M5 另行验收。

本轮安装验证使用独立测试产品；未覆盖正式安装或正式数据库。完整安装版本测试使用相同 M1 业务源码、两份临时配置，测试产品版本为 0.1.0 → 0.1.1；这不是正式发布，也不等同于真实历史 M0 二进制到 M1 的完整升级矩阵。

## 实现与验收对应

| 验收项 | 实际证据 | 结果与边界 |
|---|---|---|
| 创建、编辑、查询、归档 | 前端行为测试、Rust SQLite 测试、真实 WebView2 页面创建/归档/搜索/删除确认；安装版创建中文项目 | 通过 |
| 重启后数据不丢失 | 独立开发实例关闭重开，逐字段与时间戳比较；安装升级后再次核对 | 通过 |
| 删除项目保留源视频 | Rust 测试及真实项目删除后源文件字节一致 | 通过 |
| 失效路径检测与重新关联 | 移动视频后 missing → 同素材 ID 重新定位 → available；真实 IPC 与回归测试 | 通过；安装版原生选择也已完成 |
| 顺序迁移与重复启动 | v1 → v2 保留 M0 原文字及时间；多次重开；冲突迁移回滚；高版本拒绝；安装版预置 v1 实测 | 通过 |
| 工作目录管理 | 新目录只影响新项目；重定位只更新引用；实际打开对应资源管理器窗口 | 通过 |
| 备份恢复 | 真实数据库备份/恢复、恢复前自动备份；损坏及结构不匹配拒绝；源文件不变 | 通过；仅数据库，不含外部素材文件 |
| Windows 安装/安装版本升级 | 独立 NSIS 安装和 0.1.0 → 0.1.1 升级退出码均为 0，生产页面正常，升级前后数据逐字段一致 | 隔离测试通过；正式产品和跨平台升级未覆盖 |
| 原生素材选择/取消 | 安装版原生对话框选中中文/空格路径视频并保存；再次打开后 Esc 取消，素材字段逐项一致 | 通过 |
| 视频打开与播放 | 安装版打开测试素材，GeePlayer 显示对应视频标题、绿色画面、02:00 时长及前进的播放进度 | 通过 |

## 实现与修改文件

- 前端：`src/types/project.ts`、`src/services/projects.ts`、`src/composables/useOperation.ts`、`src/modules/projects/{ProjectsPage,ProjectPage,ProjectAssets}.vue`、`src/modules/settings/SettingsPage.vue`、路由、App、样式及 index.html 标题。
- Rust：`src-tauri/src/commands.rs`、`lib.rs`、`database.rs`、`database/{projects,assets,backup,m1_tests}.rs`；命令注册、capability 和 permission。
- 数据：`src-tauri/migrations/0002_projects_assets.sql`；Cargo 启用已有 rusqlite 的 backup feature，无新依赖包。
- 验证：`tests/projects.test.ts`、Rust 迁移/文件/备份/路径回归。
- 文档：README、DATABASE、TECHNICAL-DESIGN、DEVELOPMENT-PLAN、本报告及 task_plan/progress/findings。

## 数据库与文件边界

新增版本 2 的 `book_projects`、`assets`、`app_settings`，保留 `m0_probe`。启用外键、路径唯一性、状态和时长校验；每版本迁移在事务内完成。素材存在状态实时检查；删除关联、项目或更新目录引用均不删除源媒体文件。

基础备份使用 SQLite VACUUM INTO 创建新文件；恢复校验版本、完整性、schema 和外键，自动备份当前数据库后再通过 SQLite backup 替换内容。M1 仅接受同结构 v2 备份；原视频、图片及工作目录内容需要另行保管。

## 已执行自动化与构建命令

Rust 命令前先 `. ./.tools/env.ps1`。以下为本次 M1 累计结果；未修改业务源码的收尾轮次没有重复执行全部测试。

| 命令 | 实际结果 |
|---|---|
| `npm run lint` | 通过，零警告 |
| `npm run typecheck` | 通过 |
| `npm test` | 前端 15/15；发布回归 27 通过、1 macOS 专属测试跳过 |
| `npm run build` | 类型检查、Vite 生产构建通过 |
| `npm run test:rust` | Windows 13/13 |
| `npm run check:rust` | fmt --check、Clippy all-targets -D warnings 通过 |
| `npm run desktop:build -- --config src-tauri/tauri.windows.conf.json` | 正式配置 release 编译及 NSIS 构建通过 |
| `git diff --check` | 通过；环境有全局 ignore 读取权限与换行提示 |

正式配置安装包仍为 `src-tauri/target/release/bundle/nsis/书引 AI_0.1.0_x64-setup.exe`，2,806,248 字节。

SHA-256：`AE47E3F3BCC77C27DC3FC9926F2D5A3BA8150A65CD43B2D6DA2E6DEDD395A0A4`。隔离安装测试未覆盖或改写此安装包。正式源码版本保持 0.1.0；本次功能按 feat / minor 语义由现有发布流水线处理。未提交、未发布 Git。

## 真实运行与安装验证

业务链路证据位于忽略目录 `output/m1/`：

- `node output/m1/runtime.cjs`：真实 WebView2 UI 与 Tauri IPC 完成项目、素材、备份恢复流程；页面错误列表为空。
- `node output/m1/restart.cjs`：应用进程关闭重开后项目、原时间戳、素材 ID/路径、可用状态和设置一致。
- `node output/m1/check-current.cjs`、`node output/m1/open.cjs`：原素材引用保留，目录和视频打开命令成功；实际资源管理器窗口已观察到。

安装与升级证据位于 `output/m1/install-verification/`：

| 命令/检查 | 结果 |
|---|---|
| `npm run desktop:build -- --config output/m1/install-verification/tauri-baseline.json` | 独立产品 0.1.0 安装包构建通过 |
| `& ./output/m1/install-verification/install.ps1` | 用户明确授权后执行；创建全新 v1 测试库，NSIS 安装退出 0，专用注册版本 0.1.0 |
| 启动已安装的测试 EXE | 用户明确授权后执行；实际来自独立安装目录 |
| `node output/m1/install-verification/before-upgrade.cjs` | 生产来源 tauri.localhost；M0 原文字与时间保留；页面创建项目和关联视频通过 |
| `npm run desktop:build -- --config output/m1/install-verification/tauri-upgrade.json` | 同源码的独立产品 0.1.1 安装包构建通过 |
| `& ./output/m1/install-verification/upgrade.ps1` | 用户明确授权关闭新测试实例并升级；退出 0，专用注册版本 0.1.1，安装 EXE 哈希更新 |
| `node output/m1/install-verification/after-upgrade.cjs` | 项目、笔记、原时间戳、素材、工作目录和 M0 记录与升级前逐字段相同 |
| Python SQLite 只读检查 | user_version=2；integrity_check=ok；foreign_key_check 无异常 |

独立产品名 `Shuyin M1 Install Verification`，标识 `com.shuyin.m1-install-verification`，可执行文件名 `shuyin-m1-install-verification.exe`。安装目录在仓库 `output/m1/install-verification/installed`，数据位于系统标准 `%LOCALAPPDATA%/com.shuyin.m1-install-verification`。测试产品的注册表、快捷方式和数据保留用于继续人工验收；未卸载或操作正式产品。

0.1.0 安装 EXE 哈希：`4B3DD079C539F49C8E043E285D500114E9216892F147DE181A08B90CCA9590DB`。

0.1.1 安装 EXE 哈希：`30765EF998F01D2ECB604333EB2FCAB5DC5E0CC4A77DCCEFA5D8E9AA9B17A395`。

结构化证据：`install-result.json`、`before-upgrade.json`、`upgrade-result.json`、`after-upgrade.json`。界面截图：`installed-before-upgrade.png`、`installed-after-upgrade.png`；不入 Git。

## 原生选择、取消和播放最终验收

权限环境更新后沿用已有授权完成，未修改业务代码、默认播放器或正式数据库。

- 原生 Windows 对话框输入项目内 `output/verification/本地 视频.mp4`，回车确认；安装版显示「素材已关联，源文件保持原位」。
- `node output/m1/install-verification/check-native-selection.cjs`：通过；素材列表为两项，新增中文/空格路径素材为 available。
- `node output/m1/install-verification/picker-open.cjs` 再次打开原生对话框，Computer Use 在确认该对话框后发送 Esc。
- `node output/m1/install-verification/check-native-cancellation.cjs`：通过；按钮恢复启用，取消前后全部素材 ID、路径和元数据逐字段一致。
- 初次使用一秒样本未及时观察到画面，因此不将当时播放器其他播放列表内容作为本项目证据；随后生成时长足够的独立样本。
- FFmpeg 初次未出现在 PATH；只在当前命令追加已有 D 盘 FFmpeg 目录后，使用 ffmpeg-skill `background.py --duration 120 --width 320 --height 180 --fps 25 --color 0x355e43 --fast -o "output/m1/install-verification/M1 播放验收.mp4" --json-brief` 成功，输出验证为 120 秒、320×180、25fps、H.264。
- `node output/m1/install-verification/open-playback-sample.cjs`：在隔离安装版关联新样本，点击对应素材的「打开文件」。
- Computer Use 只读检查 GeePlayer：标题为 `M1 播放验收.mp4`，画面为与样本一致的绿色，时间轴总长 02:00，进度从 00:11 到 00:13，证明文件实际播放。后续暂停请求遇到窗口已最小化，未执行；保留播放器当前状态。

新增证据在忽略目录：`native-selected.json`、`native-cancelled.json`、`native-selected.png`、`playback-evidence.json`。此轮仅补测试样本和验收文档，无业务代码、依赖或数据库结构变更，不重复运行未受影响的完整测试。`git diff --check` 通过。

## 后续边界

M1 无未解决的核心验收阻塞。测试安装及样本保留便于复核，没有自动卸载或删除。正式产品原位升级、真实历史版本组合、macOS/Linux、签名及缺少 WebView2 的环境属于后续发布验收。本轮未提交、未发布 Git。

下一步可进入 M2 AI 创作：模型 Provider、配置与安全凭据、大纲/讲稿/文案、版本管理及失败保留草稿。

## 参考

实现依据为本机锁定版本的 rusqlite 0.40.2 源码、Tauri/Dialog 实现与生成的 NSIS 脚本；业务结论以上述实际代码和运行结果为准。
