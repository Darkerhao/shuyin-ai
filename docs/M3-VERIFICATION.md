# M3 封面工坊验证记录

日期：2026-10-23。用户本轮明确重新启用原 M3；保留三项文案生成，不实施 M4。类型为兼容新增功能 `feat`，发布影响 minor；版本号仍由现有发布流水线处理，未提交或发布。

当前结论（2026-10-24）：发布作品的图片封面固定为横版 4:3 和竖版 3:4，一次导出两张；可选 AI 背景及独立图片模型已完成本机契约服务的 Windows 开发版与新版安装升级、重启和备份恢复验证。最新证据见文末「AI 封面安装升级与 v6 数据续验」。此前四种尺寸与旧安装记录保留为历史证据；外部图片服务仍待实测。

## M3 初轮实现内容（历史记录）

- 书籍资料与结果页的「制作封面」进入封面工坊，无需配置图片 API。
- 两个本地模板，主标题/副标题/品牌文字，系统宋体/黑体，字号、颜色、左对齐/居中、上/中/下位置和背景蒙层。
- 纯色或本地 PNG/JPEG/WebP 图片；复用素材关联，图片缺失时明确提示并可重新选择或重新读取。
- 唯一 Canvas 渲染器完成实时预览和 PNG。中文按字素换行并缩小以适配安全区域，无法容纳时阻止导出；支持四种固定尺寸。
- 每次保存/导出追加版本。SQLite 保存完整编辑配置；PNG 以唯一文件名写入项目 covers 目录，失败保留草稿与旧文件，数据库事务失败清理本次新文件。
- 历史切换和重启恢复，路由/窗口未保存提示；无新增依赖。

## 修改文件

- 前端新增：`src/types/cover.ts`、`src/services/covers.ts`、`src/services/coverRenderer.ts`、`src/modules/covers/CoversPage.vue`。
- 前端接入：`src/app/router/index.ts`、`src/modules/projects/ProjectPage.vue`、`src/modules/writing/WritingPage.vue`、`src/services/projects.ts`、`src/style.css`。
- Rust 新增：`src-tauri/src/cover_commands.rs`、`src-tauri/src/database/covers.rs`、`src-tauri/permissions/covers.toml`。
- Rust 接入：`src-tauri/src/lib.rs`、`src-tauri/build.rs`、`src-tauri/capabilities/main.json`、`src-tauri/src/commands.rs`、`src-tauri/src/database/assets.rs`。
- 数据与回归：`src-tauri/migrations/0005_covers.sql`、`src-tauri/src/database.rs`、`src-tauri/src/database/m2_tests.rs`、`src-tauri/src/database/m3_tests.rs`、`tests/covers.test.ts`、`tests/coverRenderer.test.ts`。
- 文档：README、PRD、TECHNICAL-DESIGN、DATABASE、AI-RULES、DEVELOPMENT-PLAN、本记录及原有三个规划文件。

## 数据库变更

schema v4→v5，新增 cover_versions（id、project_id、version_no、config_json、exported_path、created_at）；原七张表与旧数据保留。版本按项目递增，删除项目只清理数据库关联，不删源图与 PNG。备份支持 v2–v5，旧备份在内存升级，源备份不改写。

## 实际命令与结果

Rust 命令前加载 `. ./.tools/env.ps1`，该文件为本机工具链配置，未入 Git。所有命令在仓库根目录执行；output 为 Git 忽略目录。

| 命令 | 实际结果 |
| --- | --- |
| `npm run typecheck` | 通过 |
| `npm run lint` | 通过，零警告 |
| `npx vitest run` | 5 个文件、37 项通过 |
| `npm run test:release` | 27 项通过，1 个 macOS 专用用例在 Windows 跳过 |
| `npm run test:rust` | 32 项通过，1 个需显式执行的系统凭据测试默认跳过；本轮未变更凭据模块 |
| `npm run check:rust` | fmt 和 Clippy 全目标 `-D warnings` 通过 |
| `npm run build` | 类型检查及 Vite 生产构建通过 |
| `python output/playwright/m3/prepare.py` | 建立独立 v4 测试库，记录旧行及正式库哈希 |
| `npm run dev` | 首次失败：1420 端口 EACCES；netsh 实测属于系统排除范围 1331–1430 |
| `npm run dev -- --port 15173` | Vite 启动成功 |
| `npm run desktop:dev -- --config output/playwright/m3/tauri-test.json --no-watch` | 隔离 Windows WebView2 开发版启动；测试配置使用 15173，启用本机 CDP 9226 |
| `node output/playwright/m3/runtime.cjs` | 通过；真实 UI、IPC、文件、7 个版本、四种尺寸、背景失效/恢复、历史切换与备份 |
| `npm run desktop:build` | 通过；Windows release 和 NSIS 安装包生成 |
| 停止开发进程后，`& ./src-tauri/target/debug/shuyin-ai.exe`（CDP 环境同上） | 完全重启同一隔离产品 |
| `node output/playwright/m3/restart.cjs` | 通过；7 个版本逐字段一致，最新配置恢复，实际导出目录冲突保留草稿，v5 备份恢复一致 |
| `python output/playwright/m3/inspect.py` | 通过；schema=5，完整性/外键正常，旧行保留；6 个 PNG 全量解码且尺寸准确，正式库 SHA-256 不变 |
| `git diff --check` | 通过 |

安装包：`src-tauri/target/release/bundle/nsis/书引 AI_0.4.0_x64-setup.exe`。当前源码清单仍为 0.4.0，此构建不代表已发布的新版本。

SHA-256：`BEF95B61711AA787556E9CE05E2F1CB8046A0F04BB3D48E51B2695E3CCCEBE8F`。

隔离产品 identifier：`com.shuyin.m3-verification`。本机测试数据库及项目默认目录在 C 盘 AppData；验收脚本、复制出的 PNG 和安装包在 D 盘。正式数据库哈希与实施前一致。

## 验收对应

| M3 标准 | 证据 |
| --- | --- |
| 不配置图片 API 可导出 | 隔离产品没有模型配置，UI 成功导出纯色与本地图片背景 |
| 中文正常且不越界 | 审阅纸页竖版与深林横版 PNG、工作台截图；四种尺寸用真实 Canvas TextMetrics 检查长中文/中英混排的每行边界，全部在画布内 |
| 输出尺寸正确 | 实际 PNG 解码；1080×1440、1080×1920、1920×1080、1080×1080 全部覆盖 |
| 历史不覆盖 | 7 个追加版本，v1 逐字段不变；不同文件名，历史切换回读；Rust 覆盖事务失败后文件与旧版本不变 |
| 重启恢复编辑配置 | 完全停止进程后重启，最新表单及全部7个版本与快照一致 |
| 背景和导出失败保留编辑 | 实际移走/放回背景文件，缺失时禁导出且可保存配置；目录冲突导出失败后表单文字与旧版本保留 |
| 迁移与备份 | v4→v5 实际启动迁移，单测涵盖冲突回滚和 v2/v3/v4 旧备份；v5 真实 IPC 备份恢复 |

## 检查中发现并处理的问题

- 初轮类型检查：嵌套函数中 Canvas 上下文空值收窄失效，改为捕获已收窄变量的箭头函数。模板标签拆到 style 之外，消除未使用字段。
- 字号输入先校验整数范围，防止非法大数进入逐步缩小循环。
- v5 引入后，3 个旧测试仍断言 schema=4/较新版本=5；更新至5/6，保留原数据与失败回滚断言。最终32项通过。
- 首次桌面脚本执行时页面位于模型设置，查找「新建项目」超时；脚本按当前 DOM 先进入「我的书籍」，随后完整执行通过。
- 1420 为系统保留端口，隔离配置切换15173；未修改系统端口策略或正式项目开发配置。

## 边界与下一步

- 可选图片生成 Provider 未接入；本地封面验收已完成。
- 初轮背景文件通过素材关联 IPC 登记；后续已补齐安装版原生文件窗口选择与取消，见下节。
- 后续已补齐独立 Windows 安装与两次升级；正式产品尚未替换。macOS/Linux 的字体、导出与原生安装未实测。
- 系统字体和关联的源图不随数据库备份；跨机器字体或外部源图变化可能改变重新渲染结果，历史 PNG 不会被本应用覆盖。没有持久化尚未保存的草稿。
- 文件与 SQLite 不共享事务。正常失败已覆盖补偿；在写入文件和提交事务之间强杀进程可能留下未登记的新 PNG，旧内容保留。
- 下一步建议使用一本真实书在目标发布页面确认封面比例和视觉效果；跨平台正式分发前补对应平台实机验收。M4 仍待用户另行确认。

## 后续：Windows 安装升级与原生选图（2026-10-23）

### 范围与结果

独立产品 `Shuyin M3 Install Verification`，identifier `com.shuyin.m3-install-verification`，二进制 `shuyin-m3-install-verification.exe`。安装和书籍工作目录均位于 D 盘的 `output/playwright/m3-install/`；SQLite 与 WebView 数据仍使用 C 盘 AppData 的独立产品目录。

基线来自 `git archive HEAD`，提交 `77bd9f6e8cf539870a94384ade899875dac34e1d`，内容为三项生成/schema v4。临时配置版本 0.4.0→0.5.0→0.5.1 仅用于安装测试，不修改正式版本清单或代表真实发布。

- 首次0.4.0安装退出0，安装版生产来源为 tauri.localhost。在UI中创建书籍；经实际IPC保存发布版本、模型配置、取消任务、图片关联与v4备份。模型为回环占位服务，无凭据、无远程模型请求。
- 0.4.0→0.5.0安装升级退出0，schema v4→v5。生成封面前通过IPC逐字段比较原书籍、文案、模型、任务与素材。SQLite对照保留原七表数据；封面生成后项目updated_at按业务更新，其余旧字段一致，完整性和外键正常。
- 0.5.0安装版实际导出纯色和图片背景封面。原生「选择封面背景图」窗口成功选择含中文和空格的本地路径，返回到表单并解码预览；再次打开并取消，当前文字、背景ID和素材记录一致，随后导出新增v3。
- 0.5.0→0.5.1安装升级退出0，八张表逐行逐字段与升级前完全一致，3个封面版本、3张PNG、关联素材与发布结果保留。三个PNG全量解码，均为1080×1440，哈希在升级前后相同；源图哈希不变。四种尺寸覆盖仍以上文初轮实测为准。
- 使用窗口正常退出（Alt+F4），确认安装进程已结束，再从安装目录启动。最新编辑配置、历史切换、背景读取及八表快照一致。
- 安装版调用restore_backup恢复升级前v5备份，自动备份当前库；全部封面、八张表和PNG文件再次一致。
- 正式产品注册仍为0.1.0，正式安装路径、程序SHA-256与正式数据库SHA-256均与本次开始时一致；未操作已有M2隔离安装。

### 本轮实际命令

所有临时脚本和证据位于 Git 忽略的 `output/playwright/m3-install/`。Rust构建前加载 `. ./.tools/env.ps1`；基线构建将 CARGO_TARGET_DIR 指向主仓库 src-tauri/target 复用缓存。

| 命令 | 实际结果 |
| --- | --- |
| `python output/playwright/m3-install/prepare.py` | 归档基线、准备三份独立构建配置及测试图片、记录正式产品快照 |
| `npm run desktop:build -- --config D:/MyWork/project/JIAZI-PROJECT/shuyin-ai/output/playwright/m3-install/tauri-0.4.0.json`（baseline-source目录） | 通过；基线生产构建和NSIS |
| `npm run desktop:build -- --config output/playwright/m3-install/tauri-0.5.0.json` | 通过；M3生产构建和NSIS |
| `npm run desktop:build -- --config output/playwright/m3-install/tauri-0.5.1.json` | 通过；M3再次升级用生产构建和NSIS |
| `./output/playwright/m3-install/install.ps1 -Version 0.4.0` | 初次安装退出0，注册版本与安装二进制核验通过 |
| `node output/playwright/m3-install/baseline.cjs` | 安装版UI/IPC准备旧数据和备份通过 |
| `python output/playwright/m3-install/inspect.py baseline` | v4、七表、完整性、外键和正式产品保护核验通过 |
| `./output/playwright/m3-install/install.ps1 -Version 0.5.0` | 升级退出0，注册0.5.0，二进制哈希变更 |
| `python output/playwright/m3-install/inspect.py upgrade` | v5、旧数据保留、PNG解码与正式产品保护核验通过 |
| `node output/playwright/m3-install/after-upgrade.cjs` | 安装版旧内容保留，两个封面导出通过 |
| Computer Use `sky.set_value` / `sky.click` 与 Playwright 已渲染表单核对 | 原生窗口选择中文空格路径、取消保留、导出v3通过；结果见native-dialog.json |
| `node output/playwright/m3-install/patch.cjs before` | 保存完整IPC快照和v5备份 |
| `python output/playwright/m3-install/inspect.py prepatch` | 保存八表、三张PNG及源图哈希 |
| `./output/playwright/m3-install/install.ps1 -Version 0.5.1` | 升级退出0，注册0.5.1，二进制哈希变更 |
| `node output/playwright/m3-install/patch.cjs after` | IPC逐字段一致、最新封面与历史切换通过 |
| `python output/playwright/m3-install/inspect.py patch` | 八表完全一致、三张PNG内容保留、正式产品保护通过 |
| `sky.press_key`（Alt+F4），确认进程退出后 `Start-Process` 从安装目录重新启动 | 正常退出及重启通过，PID改变 |
| `node output/playwright/m3-install/patch.cjs restart` | 重启后配置、历史与IPC状态一致 |
| 安装版真实IPC `restore_backup` + `list_cover_versions` 对照（Node单次命令执行） | v5备份恢复、3个封面版本一致，恢复前自动备份已保留 |
| `python output/playwright/m3-install/inspect.py restart` | 正常重启及备份恢复后，八表、PNG与正式产品检查通过 |
| `git diff --check` | 通过 |

三次构建均实际执行TypeScript检查、Vite生产构建和Rust release编译。此次没有业务代码、依赖或新迁移修改，未重复执行初轮已通过的单元测试/lint/Clippy；这些历史命令结果与本次安装结果分开记录。

独立NSIS包均在 `src-tauri/target/release/bundle/nsis/`：

| 测试版本 | 安装包SHA-256 |
| --- | --- |
| 0.4.0 | `619A406C45908831DEAAD74ED1C607C52EA425AB888F28AFA9E70FABB8C64869` |
| 0.5.0 | `A5BED9F0FB901516DF236185682EDDE3C9C76DBDF95412669C02A0074AFBA164` |
| 0.5.1 | `3890796318E3682009B74279AEE25035FDB59BE3173CCF12BD088E336E867931` |

### 本次脚本调整及剩余边界

- Python默认GBK读取UTF-8快照失败，显式指定UTF-8后通过。Windows规范化路径含扩展前缀，验收脚本比较路径前统一规范化；不改变应用路径或重写旧数据。
- 原生模态窗口按元素点击被工具命中检查拒绝；刷新观察后使用模态截图坐标点击，实际选择和取消成功。不将失败点击和等待超时记为业务通过。
- 三个封面为功能验收素材，已审阅真实PNG中文与排版；尚未在目标发布平台确认裁切效果。可选图片API、macOS/Linux仍未实施/实测；M4不在本轮范围。
- 本轮仅更新本验收报告、开发计划、README与规划记录，数据库沿用上轮v5。无新增业务缺陷被本轮验证确认。

## 发布图片封面比例修正（2026-10-23）

用户明确需要发布作品时上传的两张图片封面。当前新制作仅提供横版 4:3（1440×1080）与竖版 3:4（1080×1440），页面名为「发布图片封面」，按钮为「导出两张封面」。此前四种尺寸的实现及安装验收属于历史记录。

### 实现、影响范围与数据

- 一份文字、背景和样式草稿，切换比例进行预览；点击导出后分别排版两张 PNG，保留导出前所选预览比例，成功时显示两条路径。
- 两张均渲染完成后才写文件。复用已有保存命令，每张各追加一个历史版本；第二张保存失败时保留第一张、成功路径和未保存草稿，明确显示已导出 1/2 张及错误。
- 文件名包含像素尺寸、版本和唯一 ID。旧版本及原 PNG 保留；恢复旧尺寸继续编辑时，横向转为 4:3，竖向及方形转为 3:4，并提示原尺寸。
- 本轮业务修改：`src/types/cover.ts`、`src/modules/covers/CoversPage.vue`、`src-tauri/src/database/covers.rs`；回归修改：`tests/covers.test.ts`、`tests/coverRenderer.test.ts`、`src-tauri/src/database/m3_tests.rs`。同步 README、五份设计文档及三个规划记录。
- 数据库仍为 schema v5，仅收敛新保存尺寸校验，不新增迁移、表、依赖或版本清单修改。原三项生成保留；未修改正式用户数据，未提交或发布。

### 本轮实际命令与结果

以下结果属于比例修正后的当前源码；Rust 命令前加载 `. ./.tools/env.ps1`。

| 命令 | 结果 |
| --- | --- |
| `npm test` | 前端 40 项通过；发布脚本 27 项通过，1 项 macOS 专用用例跳过 |
| `npm run lint` | 通过 |
| `npm run build` | TypeScript 检查及 Vite 生产构建通过 |
| `npm run test:rust` | 33 项通过，1 项原生凭据显式测试默认忽略 |
| `npm run check:rust` | fmt、Clippy 全目标 `-D warnings` 通过 |
| `$env:TAURI_CONFIG = Get-Content output/playwright/cover-ratios/tauri-test.json -Raw` 后执行 `cargo rustc --manifest-path src-tauri/Cargo.toml --locked --bin shuyin-ai -- -o 'D:/MyWork/project/JIAZI-PROJECT/shuyin-ai/output/playwright/cover-ratios/ratio-runtime.exe'` | 独立验证二进制编译通过；有两个自定义输出命名相关警告 |
| `npm run dev -- --port 15174` | 独立 Vite 启动成功，供验证程序使用 |
| `node output/playwright/cover-ratios/runtime.cjs` | 真实 Windows Tauri/WebView2 UI、IPC、SQLite、文件闭环通过 |
| `python output/playwright/cover-ratios/inspect.py` | 四张 PNG 全量解码且尺寸准确，数据库 schema v5、完整性和外键检查通过 |
| `git diff --check` | 通过 |

首次 fmt 检查要求将尺寸 `matches!` 收为一行，按格式修正后复验通过。新回归覆盖两种比例、第二张渲染失败时无文件写入、第二张保存失败时保留首张与草稿，以及三种旧尺寸恢复不改写历史。

### 桌面验收对应

隔离产品 identifier 为 `com.shuyin.cover-ratios-verification`，项目及图片在 D 盘 `output/playwright/cover-ratios/`，独立 SQLite/WebView 数据在 C 盘 AppData。使用唯一 CDP 端口 19337，未复用正式用户数据库。

| 验收标准 | 实测证据 |
| --- | --- |
| 发布上传的两张图片 | 页面仅有两个预览比例；一次导出横版和竖版，两条路径与两个历史版本一致 |
| 尺寸与中文排版正确 | 纯色和本地图片背景各导出一对，共四张 PNG；解码尺寸准确，审阅实际图片中文正常；长中文/英文文本边界均合法，横版 14 行、竖版 17 行 |
| 历史保留及页面恢复 | 四个版本追加保存，第一对内容和原文件不变；切换历史及页面重新加载后配置恢复 |
| 失败不清空草稿 | 前端回归确认渲染失败不开始保存、第二张写入失败保留首张/路径/草稿及离开保护 |
| 真实运行无页面异常 | `runtime.json` 的 `passed=true`，`errors=[]` |

证据：`runtime.cjs`、`runtime.json`、`inspect.py`、`cover-4x3.png`、`cover-3x4.png`、`background-4x3.png`、`background-3x4.png` 及两张工作台截图，均位于上述忽略目录。

本轮未重新打包或更新已安装应用，未去目标平台实际上传，macOS/Linux 未实测。下一步在新版封面页点击「导出两张封面」，将两张 PNG 分别上传至发布页对应位置；正式分发前补最新安装包验收。

## 图片模型保存、AI 背景导出及异常保护续验（2026-10-24）

### 范围与结果

保留已有全部未提交实现，验证上轮新增的图片 Provider、模型用途、v6 迁移与生成入口。本轮未发现需要修复的业务缺陷，仅完善验收脚本/证据并同步实现文档；没有新增迁移、依赖、版本号修改或 Git 提交。

本轮复用独立 identifier `com.shuyin.ai-cover-verification`，重新从当前源码编译 Windows Tauri 开发版。SQLite/WebView 数据位于 C 盘标准 AppData，工作目录、程序和证据在 D 盘 `output/playwright/ai-cover/`。Vite 15175、WebView2 CDP 19338、本机图片契约服务 19339；没有使用正式数据库或外部图片服务。契约服务返回可控的 1024×1024 PNG，用于验证真实 Rust HTTP、原生 IPC、SQLite、Canvas 与文件闭环。

### 验收对应

| 验收标准 | 实际结果 |
| --- | --- |
| 图片模型可保存并恢复 | UI保存图片用途、服务地址、模型ID、默认设置及临时测试密钥；文本和图片默认模型相互独立；重载及正常关闭重启后逐字段一致，系统凭据 available，输入框回空 |
| 模型保存失败保护 | 更换已绑定密钥的Base URL但留空密钥，真实IPC拒绝；原模型/凭据保留，页面输入不清空；恢复地址后可保存 |
| 未配置图片模型仍可本地使用 | 新项目先在无图片模型时导出两张纯色背景PNG；主动展开AI区域后提示无图片模型 |
| AI背景只替换背景 | 实际请求使用配置的模型ID、单张base64 PNG、PNG格式及授权头；记录仅含授权头是否存在。生成图片哈希与响应一致，主副标题、字体、颜色、比例等配置不变；生成不追加封面历史 |
| 双比例导出与旧文件保护 | AI背景通过同一Canvas导出1440×1080和1080×1440，每张追加历史；原文件哈希不变；9张最终历史PNG逐张全量解码并核对尺寸，中文视觉正常 |
| 服务异常不丢稿 | 503、401、429、非JSON、非PNG、无效base64、仅返回URL，共7类响应：草稿/背景/封面历史/素材不变，任务failed并有对应错误码，页面不暴露上游错误正文 |
| 取消与迟到响应保护 | 真实延迟请求期间离页被阻止；取消后等待上游延迟结束，任务cancelled，无新增素材/封面版本，草稿与背景不变 |
| 背景目录不可写 | 暂时将隔离项目covers目录移走并在同路径放置普通文件，生成返回storage错误；finally恢复原目录后素材可读、旧文件哈希一致 |
| 第二张导出失败 | 在隔离库中临时设置仅阻止竖版新增的触发器；首张保存成功且页面显示1/2和路径，第二张错误后保留草稿及首张。finally删除触发器后重新导出成功 |
| 重启与凭据持久化 | 通过CloseMainWindow正常退出PID34016，再启动PID10872；模型、历史和背景逐字段一致。重启后真实图片请求仍含授权头，可再次生成并导出 |
| 备份恢复与正式数据保护 | v6备份恢复后模型、背景素材和封面历史一致；schema6、integrity_check=ok、外键检查为空、临时触发器不存在。数据库及备份未包含随机测试密钥，正式数据库SHA-256与上轮快照相同 |

当前验收项目有9条封面版本、2张AI背景和11条图片任务（2成功、8失败、1取消），与数据库逐项一致。10类异常检查由7类服务响应、取消迟到、背景写入、第二张导出失败组成。此前脚本调试建立的隔离项目保留，不计入当前项目结果。

### 本轮实际命令

Rust命令前加载 `. ./.tools/env.ps1`；所有命令均在项目根目录运行。

| 命令/操作 | 结果 |
| --- | --- |
| `npm test` | 前端47项通过；发布脚本27项通过、1项macOS专用跳过，退出0 |
| `npm run lint` | 通过，0警告 |
| `npm run build` | `vue-tsc --noEmit`与Vite生产构建通过 |
| `npm run test:rust` | 41通过、1项原生凭据显式测试默认忽略；本轮系统凭据另由真实UI保存/重启验证 |
| `npm run check:rust` | fmt和Clippy全目标`-D warnings`通过 |
| `$env:TAURI_CONFIG = Get-Content output/playwright/ai-cover/tauri-test.json -Raw` 后执行 `cargo rustc --manifest-path src-tauri/Cargo.toml --locked --bin shuyin-ai -- -o D:/MyWork/project/JIAZI-PROJECT/shuyin-ai/output/playwright/ai-cover/ai-cover-runtime.exe` | 当前源码隔离二进制编译通过；两个自定义输出文件名相关警告，与上一轮同类验证一致 |
| `node node_modules/vite/bin/vite.js --host 127.0.0.1 --port 15175 --strictPort` | 独立Vite服务启动成功（Start-Process隐藏后台运行） |
| `node output/playwright/ai-cover/server.cjs` | 本机图片契约服务启动成功；支持成功/错误/延迟夹具，未记录密钥值 |
| `node output/playwright/ai-cover/runtime.cjs` | 最终退出0，初轮7条封面版本、1张背景、10类异常保护通过，pageerror为空 |
| 核对测试进程路径后调用 `CloseMainWindow()`/`WaitForExit(15000)`，再 `Start-Process` 同一隔离EXE | 正常退出/重启通过，进程ID变化，记录于restart.json |
| `node output/playwright/ai-cover/runtime.cjs restart` | 退出0，凭据保留及生成、两张追加导出、备份恢复通过；最终9版本、2背景，pageerror为空 |
| `python output/playwright/ai-cover/inspect.py` | 退出0，9张PNG、2张背景、SQLite/v6备份/正式库哈希检查全部通过 |
| `git diff --check` | 通过 |

Rust回归实际覆盖v5→v6旧模型/凭据引用/封面保留、迁移回滚、v5备份恢复，以及短超时契约测试；桌面未专门等待180秒触发超时。现有原生凭据显式测试仍保持默认忽略，不将其计为单测通过。

### 证据、脚本修正与剩余边界

- 证据均在忽略目录 `output/playwright/ai-cover/`：`runtime.cjs`、`runtime.json`、`inspect.py`、`inspect.json`、`restart.json`、`requests.json`、`ai-workbench.png`、`export-contact-sheet.png`及页面ARIA快照。`requests.json`包含本轮调试请求，最终项目结果以runtime.json及inspect.json为准。
- 初次断言将仅选图片时出现的蒙层控件当成样式变化；补齐默认值后通过。再次运行时未保存离页保护阻止建项目；明确重载测试页面后继续。Windows扩展路径前缀导致字符串路径比较失败，按规范化路径检查后通过；临时阻断目录时素材状态应为missing，恢复目录后应回available，修正此断言后通过。这些失败均属于验收脚本，没有修改业务实现或弱化业务校验。
- 本机服务证明保存/网络/导出/异常保护闭环，不能证明外部模型可用性、生成质量或服务账单；本轮未调用远程图片服务，也未补当前AI源码的NSIS安装升级、macOS/Linux或目标平台上传验收。
- 收尾已正常关闭本轮隔离桌面程序，核对进程命令后停止本轮Vite及契约服务；关闭后再次运行inspect.py通过。隔离数据、凭据和证据保留，未改动正式安装或正式数据库。
- 下一步使用用户确认的兼容图片服务做一次真实生成与双图导出；正式分发前补当前源码安装包升级验收。数据库备份仍不包含图片二进制，原图片与导出目录须另行保管。

## AI 封面安装升级与 v6 数据续验（2026-10-24）

### 范围与结果

本轮使用既有独立产品 `com.shuyin.m3-install-verification` 完成当前源码的 Windows 安装升级续验，未触碰正式安装 `com.shuyin.desktop`。基线为 0.5.1/schema v5；测试安装包通过临时配置声明为 0.6.0，不修改仓库正式版本清单。

- 安装后注册版本和程序文件版本均为 0.6.0；应用首次启动将 SQLite 从 v5 迁至 v6。升级前 8 张表、4 条封面、2 个素材及其文件全部保留；旧模型补为 `model_kind=text`，旧任务补为 `asset_id=NULL`。
- 在安装版 UI 中新增并设为默认的图片模型，临时密钥只进入系统凭据库；数据库和备份不包含密钥，重载后密钥输入为空。
- 封面页真实调用本机 `/v1/images/generations` 契约服务，请求包含配置模型、1024×1024、PNG 和授权头；仅替换背景，生成后不自动追加封面历史。
- 一次导出追加横版 1440×1080 与竖版 1080×1440 两条版本。最终 6 条封面版本均存在对应文件；6 张 PNG 全量解码且尺寸与配置一致，素材由 2 增至 3，成功图片任务已关联新素材。
- 正常关闭并重启后模型、系统凭据、封面、素材、任务和既有发布版本保持；恢复本轮 v6 备份后再次逐字段一致，再重启仍为 schema v6。
- 正式产品 EXE、SQLite（含 WAL/SHM，如存在）及卸载注册信息在验收前后哈希/值一致。

### 实际命令与结果

仓库根目录执行；Rust/Tauri 命令前加载 `. ./.tools/env.ps1`，测试文件均位于 Git 忽略的 `output/playwright/ai-cover-install/`。

| 命令/操作 | 实际结果 |
| --- | --- |
| `npm run desktop:build -- --config output/playwright/ai-cover-install/tauri-0.6.0.json` | 类型检查、Vite 生产构建、Rust release 和 NSIS 构建通过 |
| 静默启动 0.6.0 NSIS 安装包并启动独立 EXE | 注册版本和安装文件版本为 0.6.0，应用启动成功 |
| `python output/playwright/ai-cover-install/inspect.py migrated` | v5→v6 通过；8 表、旧行与文件保留，完整性和外键通过 |
| `node output/playwright/ai-cover-install/model.cjs` | 安装版 UI 保存图片模型通过；凭据状态 available，数据库不含密钥 |
| `node output/playwright/ai-cover-install/generate.cjs` | 背景生成和双图导出实际完成；首轮脚本误按版本升序取记录，尺寸断言退出 1，业务写入未回滚 |
| `node output/playwright/ai-cover-install/capture.cjs` | 修正为新到旧排序后通过；6 封面、3 素材、成功任务及两尺寸文件成立 |
| `python output/playwright/ai-cover-install/inspect.py postpatch` | schema 6、6 张 PNG 全量解码、任务素材关联、正式产品不变均通过 |
| `./output/playwright/ai-cover-install/install.ps1 -Action restart -Version 0.6.0` | `CloseMainWindow` 正常退出，进程 ID 由 39100 变为 38872 |
| `node output/playwright/ai-cover-install/restart.cjs` | 重启逐字段一致，系统凭据 available，v6 备份恢复一致，页面错误为空 |
| 再次正常关闭/启动后执行 `python output/playwright/ai-cover-install/inspect.py final` | schema 6、6 封面、3 素材、完整性和外键通过，正式产品不变 |

安装包：`src-tauri/target/release/bundle/nsis/Shuyin M3 Install Verification_0.6.0_x64-setup.exe`，SHA-256 `CCC1560742474B44E140DE35DEFD6816B59BA90074B05C6021893046D70A75F5`。安装后 EXE 被 Tauri 打包后处理，不能用其哈希直接等同 target 中未封装二进制；验收改为核对安装文件版本和运行行为。

证据：`before.json`、`database-postpatch.json`、`final.json`、`restart.json`、`requests.json`、`generated.png`、`restart.png`、`model.cjs`、`generate.cjs`、`capture.cjs`、`restart.cjs` 与 `inspect.py`。首轮 `install.ps1` 在安装成功后因错误的二进制直接哈希假设退出 1，未启动应用；随后核对注册版本并按正确目标继续，数据快照和正式产品未受影响。

### 剩余边界

本轮未调用外部图片服务，画面来自可控本机响应，因此仍不能证明第三方服务的模型可用性、画质或计费。未替换正式安装，未去目标发布平台上传，macOS/Linux 实机尚未验证。数据库备份不包含背景和封面 PNG，文件目录仍需独立保管。
## UGQ.ai 外部图片服务实测（2026-10-24）

### 服务与结果

使用独立安装 `com.shuyin.m3-install-verification`，Base URL 为 `https://ugq.ai/v1`，模型为 `gpt-image-2`，接口为 `POST /v1/images/generations`。服务标价 `$0.20/次`，当前分组 `Gpt-image2-2.5-Adobe 0.75x`，两次实际扣费均为 `$0.15`，合计 `$0.30`。

- 首次请求未显式设置 `response_format`，New API 默认返回 URL；应用按设计判定 `invalid_response`，未新增素材或封面版本。随后从 UGQ 绘图日志取回该次已生成的 512×512 JPEG，仅用于定位兼容性。
- 根因修复后请求显式发送 `response_format=b64_json`，并移除上游文档未要求的 `output_format`。图片 Provider 现完整解码并接受静态 PNG、JPEG、WebP，保存时使用真实扩展名。
- 第二次请求成功返回 1024×1024 PNG，文件 1,472,681 字节，耗时约 74 秒；任务状态 `succeeded`，新素材已落盘，生成后不自动追加封面历史。
- 从该背景导出横版 1440×1080（v7）和竖版 1080×1440（v8）两张 PNG；两张均全量解码通过，中文文字、主体与蒙层视觉正常。
- 首次响应的 URL-only 路径验证了错误脱敏和原内容保护；401、429、重定向、非 JSON 等 HTTP/响应异常仍由本机契约测试覆盖，未在外部服务上制造额外失败请求。
- 第二次成功后的自动化脚本曾错误读取不存在的 `task.assetId` 字段而退出；数据库已成功落库，因此没有重试模型，改为按新素材和成功任务继续导出。

### 实际命令与结果

| 命令/操作 | 实际结果 |
| --- | --- |
| `npm run test:rust` | 41 通过、1 项原生凭据测试默认忽略 |
| `npm run check:rust` | fmt 与 Clippy 全目标 `-D warnings` 通过 |
| `npm test` | 前端 47 项、发布脚本 27 项通过，1 项 macOS 专用跳过 |
| `npm run lint`、`npm run build` | 通过，0 警告；类型检查与 Vite 生产构建通过 |
| `npm run desktop:build -- --config output/playwright/ai-cover-install/tauri-0.6.1.json` | 隔离 0.6.1 NSIS 构建通过 |
| `./output/playwright/ai-cover-install/install.ps1 -Action upgrade -Version 0.6.1` | 0.6.0→0.6.1 升级退出 0，模型与凭据保留 |
| `node output/playwright/ai-cover-install/external-preflight.cjs` | 真实调用前备份、模型和凭据状态通过 |
| UGQ 真实生成与导出脚本 | 1024×1024 PNG 成功；v7/v8 双图导出通过；页面错误为空 |
| UGQ 绘图/用量日志 | 第二次调用成功，`$0.15`，模型 `gpt-image-2` |

证据位于 Git 忽略目录 `output/playwright/ai-cover-install/`，包括 `external-before.json`、`external-second-state.json`、`external-success.json`、背景 PNG、两张封面和备份文件。

### 剩余边界

外部服务目前只实测 UGQ.ai 的 `gpt-image-2`；其他供应商、认证失败和额度耗尽的外部响应尚未逐项实测。本轮验证的是隔离安装，正式安装升级与发布产物验收另行执行。
