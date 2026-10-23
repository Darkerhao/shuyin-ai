# M2 文本创作验证记录

日期：2026-10-23。范围：文本 Provider、模型配置、系统凭据、生成、编辑和历史版本基础闭环，以及后续真实服务与隔离 Windows 安装升级。未提交或发布，不修改正式用户数据库。

## 实现和入口

- 文本模型：新增/编辑/删除、启用/停用、默认模型与连接测试；Base URL 与模型名可配置。
- 密钥通过 keyring 存入系统凭据库，WebView 无读取 API；数据库、任务和备份不含密钥。
- 项目详情进入 AI 创作，支持大纲、讲稿、标题、描述、话题；列表生成数量 1–20，可引用已保存大纲/讲稿。
- AI 结果严格校验后追加版本；人工编辑和确认也追加版本，可切换历史。失败和取消不替换编辑草稿，生成期间继续编辑也保留。
- 请求有超时、取消、固定脱敏错误和中断恢复；结果与任务成功状态事务提交。
- v3 迁移保留 v2 项目数据，支持 v2/v3 备份恢复。

## 验证命令与结果

本轮属于兼容新增用户功能，变更类型为 `feat`，发布影响为 minor；没有手工修改版本号，后续由现有发布流水线处理。

在仓库根目录执行 Rust 命令前加载 `. ./.tools/env.ps1`，仅为本机环境，不入 Git。

| 命令 | 实际结果 |
| --- | --- |
| `npm run lint` | 通过，零警告 |
| `npm run typecheck` | 通过 |
| `npm test` | Vitest 24 通过；发布脚本 27 通过、1 个 macOS 专用用例在 Windows 跳过 |
| `npm run test:rust` | 23 通过；默认跳过 1 个需要系统凭据库的显式实测 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked native_system_credential_roundtrip -- --ignored --exact ai::tests::native_system_credential_roundtrip` | 显式运行上述实测，1 通过；隔离条目写入/读回/删除通过 |
| `npm run check:rust` | fmt 与 Clippy `-D warnings` 通过 |
| `npm run build` | 类型检查和 Vite 生产构建通过 |
| `npm run desktop:dev -- --config output/playwright/m2/tauri-test.json --no-watch` | 隔离产品启动通过，真实 WebView2 与 IPC 可用；复用本仓库已启动的 Vite 服务 |
| `node output/playwright/m2/runtime.cjs` | 完整桌面业务链路通过，9 次本机服务请求全部携带从系统凭据库读取的测试凭据 |
| `node output/playwright/m2/prepare-restart.cjs` | 创建 pending 任务以核验重启恢复 |
| `node output/playwright/m2/restart.cjs` | 完全停止并重启隔离进程后验证通过；版本逐字段一致，凭据仍可用，pending 转 interrupted，实际 M2 备份恢复通过；最后清理隔离模型及测试凭据 |
| `npm run desktop:build` | 最终改动后重新构建通过；Windows release 与 NSIS 成功，安装包 4,513,050 字节 |

临时桌面脚本和证据位于被 Git 忽略的 `output/playwright/m2/`，自动化回归在 `tests/ai.test.ts`、`src-tauri/src/ai/tests.rs`、`src-tauri/src/database/m2_tests.rs`。测试服务只返回预设 JSON，不把这些结果视作真实远程模型调用。

最后补齐编辑保护后再次执行 `npm run lint`、`npm run typecheck`、`npx vitest run`（24 通过）、`npm run test:rust`（23 通过）、`npm run check:rust`、`npm run desktop:build` 与 `git diff --check`，均通过。安装包位置：`src-tauri/target/release/bundle/nsis/书引 AI_0.3.0_x64-setup.exe`。

安装包 SHA-256：`452B95AF3C87F577A77B68F81D17F005D72A705FB6CFB623CFF20B2D41044899`。

## 修改文件

- Rust 与权限：`src-tauri/Cargo.toml`、`Cargo.lock`、`build.rs`、`capabilities/main.json`、`permissions/ai.toml`、`src/lib.rs`、`src/ai.rs`、`src/ai_commands.rs`、`src/credentials.rs`。
- 数据：`src-tauri/migrations/0003_ai_writing.sql`、`src-tauri/src/database.rs`、`src-tauri/src/database/ai.rs`、`src-tauri/src/database/backup.rs`。
- 前端：`src/App.vue`、`src/app/router/index.ts`、`src/types/ai.ts`、`src/services/ai.ts`、`src/services/verification.ts`、`src/modules/settings/ModelsPage.vue`、`src/modules/settings/SettingsPage.vue`、`src/modules/writing/WritingPage.vue`、`src/modules/projects/ProjectPage.vue`、`src/style.css`。
- Prompt 与测试：`prompts/writing-v1.md`、`tests/ai.test.ts`、`src-tauri/src/ai/tests.rs`、`src-tauri/src/database/m2_tests.rs`。
- 文档：`README.md`、`docs/DATABASE.md`、`docs/TECHNICAL-DESIGN.md`、`docs/DEVELOPMENT-PLAN.md`、本报告；在原有 `task_plan.md`、`findings.md`、`progress.md` 中追加本轮进度，保留已有内容。

## 验收对应

| 标准 | 证据与边界 |
| --- | --- |
| 可调用已配置模型 | Rust HTTP 适配器通过本机契约测试；后续安装版在用户指定 HTTPS 服务及模型 ID 上完成真实五类生成 |
| 模型可配置、切换不改业务组件 | 模型设置页面与统一 TextProvider/IPC 契约；启用、默认与删除回归通过 |
| 密钥系统存储、不写日志/普通配置 | Windows 原生凭据实测、重启读取、数据库/备份字节扫描；401 回显测试凭据被固定本地错误替换 |
| 异常响应与超时 | Rust 契约测试覆盖 401、429、503、重定向、无效 JSON、空文本、截断和超时 |
| 结构校验后才入库 | 类型、空值、数量、长度校验回归；桌面非 JSON 输出生成失败，原版本数不变 |
| 失败不丢草稿 | 组件测试及桌面异常响应/认证失败/取消后草稿保留；生成期间手动编辑也由组件回归覆盖 |
| 保存、确认、切换历史 | 桌面大纲 v1/v2/v3、人为修订来源、确认标记与历史选择通过；五类内容均生成落库 |
| 重启、迁移与事务一致性 | v2→v3 保留数据、冲突回滚、版本重启逐字段比较、取消后拒绝迟到结果、任务成功写入失败时版本一起回滚 |
| 备份恢复 | v2 原备份不改写、内存升级、v3 真实 IPC 恢复、恢复前备份保留、运行任务阻止恢复 |

## 已定位的问题与修正

- 初始 HTTP 契约测试收到代理 502；已核对环境代理与实际请求，回环模型服务现在明确绕过代理，远程服务保留环境代理支持。
- 第一次桌面调用返回 command not found；处理器已注册，但 build.rs 的命令清单和 capability 未授权。已按现有最小权限体系补齐，后续完整桌面验证通过。
- 首次重启后立即连接 CDP 尚未就绪，等待 WebView 启动后重新验证通过；未把连接拒绝记为业务失败。

## 尚未覆盖与使用边界

- 已在用户指定 HTTPS 服务与 gpt-6-astra 模型 ID 验证模型权限、五类 JSON 输出及公开原文基准质量；不据此推断服务上游模型身份或其他模型兼容性。用户目前没有个人笔记，个人创作试用仍待进行。
- macOS Keychain 与 Linux Secret Service 为已接入的原生后端，本轮只在 Windows 实跑。Linux 需要已解锁的桌面 Secret Service。
- 后续已实际执行独立 Windows 安装与两次升级，见下节。正式安装与正式数据库保留；macOS/Linux 安装升级仍未验证。
- 草稿保存在当前编辑页面；保存的版本持久化。路由/刷新有未保存提示，崩溃或强制结束进程不会恢复尚未保存的编辑。
- 当前支持文本 Chat Completions，要求 finish_reason=stop 和严格 JSON；只支持 Responses、多模态输出或非标准响应的服务需要新增适配器。
- 本轮没有实现单分段重生成、封面文案、收藏/选择/复制和 Markdown 导出等完整 PRD 增强；封面工坊与发布中心按后续里程碑实施。
- SQLite 与系统凭据不共享事务；更换密钥采用新条目与失败补偿。数据库恢复到旧备份、迁移机器或手动删除凭据后可能需重新输入密钥；不会退回明文存储。

## 后续：真实服务、公开原文基准与安装升级

### 范围与材料

- 用户指定 Base URL 为 `https://bmapi.020212.xyz/v1`，模型 ID 为 `gpt-6-astra`。用户在安装版 UI 直接输入密钥并保存到 Windows 凭据管理器；验收脚本只检查 `credentialStatus=available`，未读取密钥输入、系统凭据内容或其他应用配置。
- 用户指定 `D:\书引 AI\output`，检查为空后用户明确暂无读书笔记。因此准备《论语·学而》1.1–1.4 公开原文、阅读提纲、明确标为虚构的现代类比与待核实项，来源记录在该目录的 `论语学而-公开原文验收资料.md`；同时提供 `最小读书笔记模板.md`。
- 这是公开原文基准，不是用户个人笔记或 3 个真实项目试用。修订稿的确认勾选用于验收编辑/确认链路，不代表用户最终发布审核。

### 实际生成质量

真实服务共调用 7 次：首次连接测试、五类生成、升级后再次连接；没有自动重试或为质量比较额外重复生成。

| 内容 | 请求耗时 | 复核结论 |
| --- | --- | --- |
| 大纲 | 60.171 秒 | 具备主题、观点、概念、五段结构、现实联系及待核实项；正确区分孔子、有子和曾子；不将虚构类比包装成原书故事 |
| 讲稿 | 28.003 秒 | 开场、分段、结尾与非口播备注完整；初稿口播部分约 478 个中文字符，修订稿约 487 个；接近两分钟的字数估算，未用实际口播计时验收 |
| 标题 | 7.024 秒 | 严格 5 项，均与讲稿一致，无夸张结果承诺 |
| 描述 | 15.722 秒 | 严格 3 项，准确概括内容，区分现代类比与原文 |
| 话题 | 4.945 秒 | 严格 8 项，围绕论语、经典阅读、学习、自省，无无关热点 |

大纲中将“对所传授的内容”修订为“对所学的内容”，保留“传不习”释义待核实；讲稿只调整开场和结尾的口语措辞。两类均追加 v2、保留 AI v1 及模型/Prompt 来源，实际切换 v1/v2 并比较编辑内容。结果共 7 个不可变版本、5 条成功任务；5 类响应全部由现有 Rust 解析/校验后才入库，没有调整业务代码或 Prompt 来绕过校验。

样本资料短且约束明确，本次质量表现不能外推到长篇笔记、小说、现代非虚构或个人表达。最终选题、注本释义和实际口播仍需用户审核。初稿、验收修订稿及验收说明保存在用户指定输出目录。

### Windows 安装升级

- 测试产品：`Shuyin M2 Install Verification`；identifier `com.shuyin.m2-install-verification`；二进制 `shuyin-m2-install-verification.exe`。
- 安装目录：`output/playwright/m2-install/installed/`，位于 D 盘。SQLite 与 WebView 的产品数据仍由 Tauri 放在 C 盘用户 AppData，系统凭据由 Windows 管理；没有迁移或删除正式用户数据。
- 基线源码来自 `git archive HEAD`，提交 `3093f35481a194c2f2084325991bb2884c70cc1e`，实际为 M1/schema v2。当前 M2 未提交实现使用同一隔离产品升级到 schema v3。临时版本号 0.3.0→0.4.0→0.4.1 仅用于安装测试，不修改正式版本号，不代表正式发布。
- 首次 NSIS 安装退出 0；安装版 UI 创建项目，保存公开原文笔记、工作目录与实际 Markdown 素材关联，并通过 IPC 创建 v2 备份。
- M1→M2 的 0.3.0→0.4.0 升级退出 0；原四表逐行逐字段一致，保留项目、笔记、ID、时间戳、素材引用和设置；v3 三张 AI 表可用，旧 v2 备份原数据仍可读。
- M2 的 0.4.0→0.4.1 升级退出 0；全部七表逐行逐字段一致，7 个版本、5 条成功任务及凭据引用保留。安装版重新启动后凭据仍为 available，实际 HTTPS 连接测试再次成功（3.594 秒），讲稿 v2 在 UI 正确读回。
- 两次迁移检查均 `integrity_check=ok`、外键检查无异常；关联原文文件 SHA-256 保留。正式数据库 SHA-256、正式安装注册版本/路径和原正式安装包 SHA-256 前后完全一致。
- 原正式安装包仍为本报告上文的 `452B95AF...44899`；独立测试安装包哈希见被忽略的 `baseline-install.json` 与 `upgrade-*.json`。保留隔离安装版与凭据供用户继续试用，没有把密钥复制到正式产品。

### 本次实际执行命令

下列脚本与原始证据位于 Git 忽略的 `output/playwright/m2-install/`，不进入应用业务代码。构建前加载 `. ./.tools/env.ps1`；基线构建另将 `CARGO_TARGET_DIR` 指向仓库 `src-tauri/target` 以复用依赖。

| 命令 | 结果 |
| --- | --- |
| `npm run desktop:build -- --config D:/MyWork/project/JIAZI-PROJECT/shuyin-ai/output/playwright/m2-install/tauri-baseline.json`（归档的 baseline-source 目录） | 通过，M1 独立 NSIS；包含 TypeScript、Vite 与 Rust release 构建 |
| `npm run desktop:build -- --config output/playwright/m2-install/tauri-upgrade.json` | 通过，M2 0.4.0 独立 NSIS |
| `npm run desktop:build -- --config output/playwright/m2-install/tauri-patch.json` | 通过，M2 0.4.1 独立 NSIS |
| `Start-Process -FilePath <M1独立安装包> -ArgumentList '/S',"/D=<隔离安装目录>" -WindowStyle Hidden -PassThru -Wait` | 实际退出 0；安装注册版本 0.3.0，启动生产来源 tauri.localhost |
| `node output/playwright/m2-install/baseline.cjs` | 通过；首次因测试目录未创建失败，创建目录后完成资料和素材准备 |
| `python output/playwright/m2-install/inspect-db.py baseline` | 通过，schema v2，完整性及外键正常 |
| `./output/playwright/m2-install/upgrade.ps1 -Version 0.4.0` | 通过，NSIS 退出 0，注册版本及二进制哈希变更 |
| `node output/playwright/m2-install/after-upgrade.cjs` | 通过，旧数据一致，M2 命令可用，预填服务/模型 |
| `python output/playwright/m2-install/inspect-db.py upgrade` | 通过，schema v3，原四表数据及源文件哈希保留 |
| `node output/playwright/m2-install/connect.cjs` | 通过，用户保存凭据后真实远程连接约 4 秒 |
| `node output/playwright/m2-install/generate.cjs outline`（另依次执行 script / titles / descriptions / hashtags） | 五类均通过；首次 outline 脚本标题写错，在请求前超时，根据实际 DOM 修正后生成成功 |
| `node output/playwright/m2-install/review.cjs outline`（另执行 script） | 两类修订、确认及历史切换通过 |
| `node output/playwright/m2-install/patch.cjs before` | 通过，保存完整 IPC 状态及 M2 升级前备份 |
| `python output/playwright/m2-install/inspect-db.py prepatch` | 通过，七表快照完整 |
| `./output/playwright/m2-install/upgrade.ps1 -Version 0.4.1` | 通过，NSIS 退出 0，注册版本 0.4.1 |
| `node output/playwright/m2-install/patch.cjs after` | 通过，IPC 状态逐字段一致，凭据与实际远程连接成功 |
| `python output/playwright/m2-install/inspect-db.py postpatch` | 通过，全部七表与源文件哈希完全一致 |
| PowerShell `Get-FileHash` / 注册表前后比较；Python 只读检查旧备份与口播字数 | 通过，正式产品状态未变，原 v2 备份数据保留 |

本次没有业务源码、依赖或新迁移改动，未重复执行初轮 lint、前后端单元测试及 Clippy；上文这些命令的结果属于初轮实施。三次安装包构建均实际重跑类型检查与生产构建。最终文档更新后执行 `git diff --check` 通过。

下一步：使用最小笔记模板记录一本读过的书，以个人观点与实际口播补验；跨平台凭据、断网和锁定系统凭据的实机行为继续保留未覆盖边界。
