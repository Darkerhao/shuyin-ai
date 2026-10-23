# 三项生成验收记录

日期：2026-10-23。当前需求只包含发布标题、发布内容、音频生成提示词。

## 实现与取舍

- 主导航收敛为“我的书籍”“模型设置”。录入书名即可开始，作者和书籍内容/创作重点选填，保存后直接进入结果页。
- 一次模型请求生成一套结果：一个标题、一段发布内容及书籍专属音频指导。标题最多30字符，发布内容最多1000字符；音频指导严格五个部分，解析、校验后才保存。
- Rust使用独立版本化模板组合音频提示词，保留甲子主持、系列开场和关注结尾，默认8—10分钟、简体中文、通俗幽默故事感。历史书、小说和非虚构按各自内容安排正文，不套用同一组历史人物和事件。
- 三项均可编辑、分别复制；修改追加为新版本，历史折叠显示。失败、取消及生成中的修改不覆盖已有草稿，保存期间禁用编辑，离开未保存页面时提示。
- 不生成完整口播稿或音频文件；提示词供用户复制到外部音频工具，并在该工具选择原书来源。大纲、话题候选、封面、素材和发布任务包退出当前主流程。
- 沿用原Provider、系统凭据、SQLite与受限IPC，无新增依赖。保留旧数据类型及必要底层能力以读取历史和恢复旧备份，不新增第二套生成路径。

## 修改文件

| 范围 | 文件 |
| --- | --- |
| 页面 | `src/App.vue`、`src/modules/projects/ProjectPage.vue`、`src/modules/projects/ProjectsPage.vue`、`src/modules/writing/WritingPage.vue`、`src/modules/settings/ModelsPage.vue`、`src/style.css` |
| 前端边界 | `src/services/ai.ts`、`src/types/ai.ts`、`src/types/project.ts` |
| 生成与校验 | `src-tauri/src/ai.rs`、`src-tauri/src/ai/publishing.rs`、`src-tauri/src/ai_commands.rs` |
| 提示词 | `prompts/publishing-v1.md`、`prompts/audio-template-v1.md` |
| 数据与迁移 | `src-tauri/migrations/0004_publishing.sql`、`src-tauri/src/database.rs`、`src-tauri/src/database/ai.rs`、`src-tauri/src/database/backup.rs` |
| 回归 | `tests/ai.test.ts`、`tests/projects.test.ts`、`src-tauri/src/database/m2_tests.rs`，以及生成模块内的Rust测试 |
| 文档 | `README.md`、`docs/PRD.md`、`docs/AI-RULES.md`、`docs/DATABASE.md`、`docs/TECHNICAL-DESIGN.md`、`docs/DEVELOPMENT-PLAN.md`、本报告、`task_plan.md`、`findings.md`、`progress.md` |

## 数据库变更与保留证据

schema从v3升级为v4，新增 `kind='publishing'`；一次事务内重建相互引用的 `writing_versions` 和 `ai_tasks` 并复制旧行，保留任务指向旧版本的外键。旧五类结果不转换、不删除，模型和凭据引用不变。新结果以 `format='publishing'` 保存 `title`、`description`、`audioPrompt`。

备份恢复支持v2/v3/v4；旧备份先在内存升级，不修改源备份，恢复前仍备份当前数据库。

真实隔离安装升级后逐字段比较旧行，全部保留：

| 表 | 升级前旧行保留数 |
| --- | ---: |
| `ai_tasks` | 5 |
| `app_settings` | 1 |
| `assets` | 1 |
| `book_projects` | 1 |
| `m0_probe` | 1 |
| `model_providers` | 1 |
| `writing_versions` | 7 |

升级后 `PRAGMA user_version=4`，完整性检查为ok，外键检查无异常。证据位于本地忽略目录 `output/playwright/publishing/`：`before-v4.sqlite3`、`before-v4.json`、`migration-evidence.json`。

正式应用数据库 `%LOCALAPPDATA%\com.shuyin.desktop\shuyin.sqlite3` 的验收前后SHA-256一致：`0C26E86C0520B71899E82816FC769F34E7D9D0FEA4B2F32672F4BBEBFB5EC825`。本轮未替换正式安装或修改正式数据库。

## 实际执行的命令与结果

命令工作目录均为仓库根目录；Rust相关命令前加载 `. ./.tools/env.ps1`。

| 命令 | 实际结果 |
| --- | --- |
| `npm run lint` | 通过，零警告 |
| `npm run typecheck` | 通过 |
| `npm test` | 前端24项通过；发布脚本27项通过、1项macOS专用测试跳过 |
| `npm run test:rust` | 27项通过，1项原生凭据显式测试默认跳过 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml` | 通过 |
| `npm run check:rust` | fmt检查及Clippy通过 |
| `npm run desktop:build -- --config output/playwright/publishing/tauri-test.json` | TypeScript、Vite、Rust release和Windows NSIS构建通过 |
| `node output/playwright/publishing/runtime.cjs` | 退出0，两本书的真实桌面生成、复制、编辑与历史验收通过 |
| `Get-FileHash -LiteralPath "$env:LOCALAPPDATA\com.shuyin.desktop\shuyin.sqlite3" -Algorithm SHA256` | 与升级前记录一致 |
| `git diff --check` | 通过 |

Rust新增回归覆盖固定模板、五段结构/长度拒绝、仅书名生成、v3→v4旧版本和任务引用保留、v3/v4备份恢复以及迁移失败回滚。前端覆盖三项界面、复制、生成中编辑保留、失败/取消保留草稿及未保存修改保护。

首轮lint发现的全局 `navigator` 引用已改为 `globalThis.navigator`。第一次fmt曾遇Windows文件映射占用（os error 1224），之后重跑fmt及Rust检查通过；没有通过关闭检查绕过问题。本轮默认跳过的原生凭据测试未记为通过；安装版的真实请求证明既有凭据可继续使用，未回读或记录密钥内容。

## Windows安装升级

沿用此前已授权的独立验收产品：

- 产品名：`Shuyin M2 Install Verification`。
- identifier：`com.shuyin.m2-install-verification`。
- 安装目录：`output/playwright/m2-install/installed/`，位于D盘。
- 实际NSIS升级：0.4.1→0.4.2，安装退出码0，安装注册版本核对为0.4.2。
- 新安装包：`src-tauri/target/release/bundle/nsis/Shuyin M2 Install Verification_0.4.2_x64-setup.exe`。
- 安装包SHA-256：`7038DCB0A4978D0DA59189C186E3CEDCA1B7D1A8299F2EE874E54E32D74DD50D`。

0.4.2是隔离配置的临时验证版本，不是正式发布版本。安装信息见 `output/playwright/publishing/install.json`。隔离数据库和WebView仍使用Windows的C盘用户AppData目录，凭据由Windows系统存储管理。

## 两本书的真实服务验收

沿用用户指定的HTTPS服务、模型ID `gpt-6-astra` 及用户此前在应用保存的凭据。每本仅填书名和作者，不填笔记或创作资料，各生成一次。模型ID只作为供应商配置，不推断上游身份。

| 书籍 | 实际生成标题 | 音频提示词 |
| --- | --- | --- |
| 《明朝那些事儿》 | 《明朝那些事儿》：皇位背后的人间账本 | 1536字符；朱元璋建明、皇位/靖难、土木堡与于谦、张居正改革、阅读价值 |
| 《白鹿原》 | 《白鹿原》：一座祠堂背后的时代风暴 | 1573字符；土地与祠堂、白嘉轩与鹿子霖、年轻人的选择、田小娥困境、作品叙事与阅读价值 |

实际通过：

1. 打包安装版的WebView→IPC→Provider→SQLite完整调用链；主导航只有两项，结果页只有三项文本。
2. 标题/内容长度、音频固定主持与时长、五段正文及当前书名校验；两书指导不同，《白鹿原》没有混入朱元璋。
3. 每本三个复制按钮均实际写入Windows剪贴板，再读回与初稿比较；中文及换行保留。
4. 修改发布内容保存v2，v1逐字段保持原样；界面切换v1/v2显示一致。每本只有一条成功生成任务。
5. 已审阅两张结果页截图及完整音频指导，排版正常，五段指导与各书体裁相符。

原始证据：`output/playwright/publishing/runtime-evidence.json`、`明朝那些事儿.png`、`白鹿原.png`。两本各三份初稿已导出至 `D:\书引 AI\output\三项生成验收\`；安装版保留的v2在描述末尾另加一句文字，仅用于验证编辑保存。

## 验收对应与剩余边界

- 三项一次生成、编辑、复制、保存、历史切换：通过。
- 无需预先写笔记或生成大纲：通过；仅书名路径由Rust测试覆盖，真实模型验收填了书名和作者、资料为空。
- 固定系列模板与随书变化的五段指导：通过，两种体裁均实测。
- v4旧数据、任务外键及凭据引用保留：迁移测试和真实安装升级均通过。
- 失败/取消/编辑保护及旧备份恢复：自动化通过；本轮未重复上次的全部原生对话框和跨进程凭据专用测试。
- 外部工具的实际音频生成、8—10分钟成品时长和朗读发音未验收，当前证据只覆盖提示词。没有用户个人笔记，不宣称完成个人笔记或全部事实的质量验收。
- macOS/Linux安装与凭据、本轮正式产品安装升级、GitHub发布未执行；代码未提交或发布。

下一步直接用导出的提示词在音频工具选择对应原书来源，试听一集即可检验实际节奏和时长。若正式发布本轮新增音频提示词能力，按兼容功能增强使用 `feat` 并升一次minor；正式版本交由现有发布流水线处理，不手改为测试版本。
