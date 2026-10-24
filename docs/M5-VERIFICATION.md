# M5 测试、打包与试用验收（2026-10-24）

## 1. 范围与结论

用户要求开始 M5，并明确同意使用《论语》《西游记》《朝花夕拾》和验收视频。本轮完成当前三项生成、双比例封面、半自动发布准备的整体测试、Windows 隔离安装升级和用户手册。七项 M5 核心验收通过；验证范围内未发现未解决的高优先级数据丢失问题。

真实文本生成使用已有 BMAPI 配置的 `gpt-6-astra` 模型，密钥通过操作系统凭据在隔离验收产品中使用，没有输出密钥。可控异常使用本机契约服务，与外部模型成功明确区分。视频为用户同意的本地验收片段副本，不代表这三本书的正式成片。

没有执行真实平台上传或最终发布。首轮仅使用隔离安装；用户随后要求继续任务，已补充正式 Windows 安装 0.1.0→0.5.1 升级及 ZIP 启动，见第 8 节。macOS/Linux 实机、音频工具生成及三本书的最终成片留待对应环境与实际创作验收。M5 不重新启用历史大纲/完整讲稿或运营统计。

## 2. 实现及根因修复

发现：`database/publishing.rs::package_valid` 原来只比较任务包中封面文件的大小。将 PNG 的一个字节改坏、保持长度不变后，任务仍为 ready，并可进入发布成功记录流程。

先增加 `changed_package_cover_with_same_size_cannot_remain_ready`，实际得到 `left: ready / right: draft` 的失败。修正为复用现有 15 MB 上限读取，对两张任务包封面分别与源 PNG 比较内容。损坏时退回草稿，阻止成功确认，重新生成新任务包后恢复就绪；旧包和原素材保留。

新增回归还覆盖：

- v2–v7 每个受支持备份版本的恢复、原备份字节不变、原时间戳及缺失素材引用保留、完整性/外键检查，以及使用恢复前安全备份撤销恢复。
- 三本书的文案版本、失败任务、封面历史、完整任务包、素材内容和模型凭据可用状态在删除项目、备份恢复及重新打开后的保留。

没有新增依赖、Provider、状态层或持久化校验字段。该批修改属于 `fix` / patch，版本由既有发布流程递增；本地未提交、推送或发布。

## 3. 数据库与文件

schema 保持 v7，没有新迁移。隔离数据库从现有 v6 验收备份复制，并加入独立文本模型配置用于本轮试用；首次启动后比对基线 8 张表的全部旧行与字段，均保持。v6 原备份和正式数据库的哈希不变。

最终隔离库为：4 本书（1 本基线 + 3 本试用）、7 素材、7 文案版本、14 封面版本、7 AI 任务、4 模型、3 就绪发布任务、0 发布记录。人工发布记录测试明确备注“未向真实平台发布”，随后恢复到三本书就绪的基线；含该测试记录的恢复前安全备份保留。

视频移动/重新定位、任务包写入失败和恢复后，三个视频副本 SHA-256 均与试用前一致。源视频未删除或改写，任务包不复制视频。数据库备份不包含媒体与任务包，需要另行保管。

## 4. 实际验证命令与结果

命令在仓库根目录运行。Rust/Tauri 命令先执行 `. ./.tools/env.ps1`。`output/playwright/m5/` 为本地忽略目录，包含运行脚本、数据、日志和截图，不提交外部服务凭据或试用产物。

| 命令 | 实际结果 |
| --- | --- |
| `npm test` | 前端 6 文件、55 项通过；发布脚本首次 26 通过、1 失败、1 macOS 专用跳过，失败为本地临时仓库 Git fetch，见下文 |
| `node --test --test-name-pattern='main advancing' tests/release.test.mjs` | 首次失败的同一用例单独重跑通过 |
| `node --import ./output/playwright/m5/trace-git.mjs --test tests/release.test.mjs tests/release-assets.test.mjs` | 发布脚本完整复跑 27 通过、0 失败、1 macOS 专用跳过，约 197 秒；诊断加载器只在本地临时仓库 fetch 失败时输出 stderr，未修改断言 |
| `npm run lint` | 通过，0 警告 |
| `npm run build` | 两次隔离桌面构建均实际执行该前置命令，TypeScript 与 Vite 生产构建通过 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked changed_package_cover_with_same_size` | 修改前复现失败，修复后通过 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml` | 新增 Rust 测试及修正格式化完成 |
| `npm run test:rust` | 最终 51 项通过，1 项原生凭据测试默认忽略；新增 3 项回归 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked native_system_credential_roundtrip -- --ignored` | 单独执行系统凭据写入/读取/删除，1 项通过 |
| `npm run check:rust` | fmt 与全目标 Clippy `-D warnings` 通过 |
| `npm run desktop:build -- --config output/playwright/m5/tauri-baseline.json` | Windows release 与独立 NSIS 0.5.0 验收包构建通过 |
| `npm run desktop:build -- --config output/playwright/m5/tauri.json` | Windows release 与独立 NSIS 0.5.1 验收包构建通过 |
| `npm run desktop:build` | 最终默认配置的前端、Rust release 和 Windows NSIS 构建通过，生成 `src-tauri/target/release/bundle/nsis/书引 AI_0.5.1_x64-setup.exe`；仅构建，未安装到正式目录 |
| `python output/playwright/m5/inspect.py seed` | 从 v6 备份创建新隔离数据库；不替换原库；文本凭据仅复制到隔离系统凭据条目 |
| `./output/playwright/m5/install.ps1 -Version 0.5.0` | 安装并启动，PID 49660，存在非零窗口句柄和「书引 AI · M5 试用」窗口 |
| `python output/playwright/m5/inspect.py migrated` | v6→v7；旧表逐字段一致，完整性与外键通过，正式库及原备份哈希不变 |
| `node output/playwright/m5/runtime.cjs books` | 三本书全部通过外部文本生成、编辑、复制、历史切换、双封面、完整任务包与视频路径检查 |
| `./output/playwright/m5/install.ps1 -Version 0.5.1` | 正常关闭 0.5.0 后同目录升级并启动，PID 46412；没有卸载或重建数据库 |
| `node output/playwright/m5/runtime.cjs restart` | 升级及后续恢复/重启后，三本书版本、封面、就绪任务逐字段一致，视频哈希和系统凭据可用状态一致 |
| `node output/playwright/m5/negative.cjs` | 从检查点修正脚本等待后，15 项异常/取消/素材/任务包/人工记录/恢复检查通过，页面错误数组为空 |
| `python -X utf8 output/playwright/m5/inspect.py security` | 扫描实际密钥的 UTF-8/UTF-16 表示，187 个文件未检出；包括数据库、备份、日志、任务包和 Git 跟踪文件；只输出结果与数量 |
| `./output/playwright/m5/install.ps1 -Action restart -Version 0.5.1` | 正常关闭后重启，PID 46412→5576；随后再次执行数据一致性检查通过 |
| `python -X utf8 output/playwright/m5/inspect.py final`、`closed` | 最终及退出后数据库完整性、外键、正式库和旧备份哈希通过 |
| `./output/playwright/m5/install.ps1 -Action stop` | 正常退出，确认隔离进程结束；随后关闭本轮本机契约服务 |
| `git diff --check` | 通过 |

安装版本 0.5.0→0.5.1 是同一当前实现使用配置覆盖构建的安装器升级试验，用于验证版本升级与保留数据，不冒充已发布旧版本到新版本的升级。旧 schema 升级另由 v6 基线和 v2–v7 回归验证。

最终独立验收包：`src-tauri/target/release/bundle/nsis/Shuyin M5 Verification_0.5.1_x64-setup.exe`，SHA-256 `B98C793E2D760749A6984E08A9DAD3F1A947BDB71668ACFFD86C94DCE2BFCDC4`。安装目录 `output/playwright/m5/installed`，identifier `com.shuyin.m5-verification`。

## 5. 三本书试用与视觉结果

| 书目及重点 | 实际生成标题 | 结果 |
| --- | --- | --- |
| 《论语》·《学而》学习、朋友、自省、言行 | 《论语·学而》：学习，也是一种日常功夫 | 两文案版本、两封面、8 文件任务包，就绪 |
| 《西游记》·第一回石猴出世到拜师得名 | 《西游记》第一回：美猴王为何离开花果山 | 两文案版本、两封面、8 文件任务包，就绪 |
| 《朝花夕拾》·百草园、美女蛇、捕鸟、三味书屋 | 百草园到三味书屋：鲁迅笔下的童年 | 两文案版本、两封面、8 文件任务包，就绪 |

三份音频提示词各有五个不同主题，固定开场/结尾和 8–10 分钟规则保留。与输入重点复核，未将《西游记》后续取经情节放进第一回，未将回忆散文描述为虚构小说；没有据此声称所有模型事实已获独立学术校验。

试用素材依据的原文入口：

- https://zh.wikisource.org/wiki/論語/學而第一
- https://zh.wikisource.org/wiki/西遊記/第001回
- https://zh.wikisource.org/wiki/從百草園到三味書屋

六张 PNG 均来自安装版 Canvas 导出，PNG 头的实际像素与 1440×1080 / 1080×1440 一致；包内图与原图 SHA-256 一致。已查看 `six-covers.jpg` 联系表，中文主标题、副标题和品牌显示正常、位于安全区域。长中文/换行溢出由既有 renderer 回归覆盖，不冒充本轮所有字体、模板和背景均逐项人工检查。

视频关联及重新定位通过真实受限 Tauri IPC 执行；本轮未重复原生选择对话框的鼠标选择/取消。原生对话框历史证据见 M1/M3，当前选择组件相关前端测试已复跑。备份恢复通过真实 SQLite/IPC 执行；没有测试平台上传与发布。

主要证据：`books-result.json`、`before-upgrade.json`、`upgrade-result.json`、`negative-result.json`、`inspect-migrated.json`、`inspect-security.json`、`inspect-final.json`、`inspect-closed.json`、每本书的 `*-writing.png` / `*-cover.png` / `*-ready.png`、`six-covers.jpg`、`model-failures.png`、`missing-video.png`、`corrupt-package.png`、`write-failure.png`、`manual-record.png`、`restored.png`。

## 6. 首次失败与修正记录

- 新封面完整性回归先真实失败，再修复根因；没有删除测试、放宽断言或增加另一套就绪状态。
- 新恢复矩阵初稿使用 `usize` 调用 SQLite 参数，编译不通过；按实际 rusqlite 支持类型改为 `u32` 后全部 Rust 测试与 Clippy 通过。
- 首次发布脚本全量中的临时本地仓库 fetch 失败，错误原因未在原断言输出中保留。单例及带 stderr 诊断的完整复跑通过，未更改发布逻辑，也不猜测失败根因；下次再现可用同一诊断脚本取证。
- 桌面超时脚本先用 15 秒等待错误，并在原请求仍运行时尝试选择被禁用的模型；修正为等待实际请求终态及 135 秒错误窗口，保留原检查点后确认真实 120 秒超时。没有缩短生产超时来通过测试。
- 取消后脚本用 hash 跳转离开未保存页面，被离页保护阻止；改为先将验证输入恢复到已保存内容，再通过实际链接继续文件验证，未弱化应用的未保存保护。

## 7. 修改文件、验收对应与下一步

业务修复：`src-tauri/src/database/publishing.rs`。

回归：`src-tauri/src/database/m1_tests.rs`、`src-tauri/src/database/m4_tests.rs`。

文档：新增 `docs/USER-GUIDE.md` 与本报告，更新 README、PRD、TECHNICAL-DESIGN、DATABASE、AI-RULES、DEVELOPMENT-PLAN，移除当前范围中失效的 M5 暂停说明。工作区未暂存、提交或推送。

| M5 验收标准 | 对应证据 |
| --- | --- |
| 核心测试通过 | 前端 55、Rust 51、原生凭据 1、发布回归最终 27 通过；平台专用跳过如上 |
| 失败有明确提示 | 8 类文本异常、取消、缺失视频、损坏包、目录写入失败、无效备份 |
| 模型失败不丢草稿 | 三个编辑字段逐字一致；旧版本逐字段一致；失败响应未新增版本 |
| Windows 安装包启动 | 两个独立 NSIS 包实际安装、可见窗口、正常退出与重启 |
| 备份可恢复 | v2–v7 回归、v7 安装版恢复及重启、恢复前安全备份保留 |
| 3 个真实项目完成发布准备 | 三个外部模型结果、6 张 PNG、3 套各 8 文件的就绪任务包 |
| 无已知高优先级数据丢失问题 | 原视频、旧备份及正式数据保留；本轮发现的问题已修复并有回归 |

下一步按使用手册换成实际书籍成片，在平台自行验证上传预览并完成最终发布。需要发布新版本时按 `fix` / patch 进入现有发布流程；其他平台实机仍需分别验收。本轮不执行远端发布。

## 8. 继续任务：正式安装升级与本地交付（2026-10-24）

用户要求继续后，核对实际安装及数据：正式程序位于 `D:\MyWork\software\shuyin-ai`，注册版本和文件版本均为 0.1.0；正式数据库为 schema v1，只有一条 M0 文字记录，没有书籍、模型或待处理业务数据。

先在 `output/playwright/m5-formal/rollback/` 保存原程序、卸载程序、原 0.1.0 安装包、原数据库文件及 SQLite 一致性快照，并核对字节哈希。随后将上轮已构建的默认 0.5.1 NSIS 安装到原目录；这是实际原有 0.1.0 程序升级，不是更换验收包版本号。

升级后注册表与文件版本均为 0.5.1，数据库迁移到 v7。原文字和 `2026-10-23T03:37:15.828Z` 写入时间逐字段保持；其余九张业务表为空，没有把隔离试用书籍或模型凭据写入正式数据。正式应用的书籍、模型设置、目录与备份页面均可读取，未出现页面错误。实际创建并恢复 v7 备份后旧记录不变，恢复前安全备份保留。

正常退出并重启后再次比对通过。ZIP 解压后的程序也实际启动，读取同一份正式数据库，旧记录及时间一致；验证后正常退出 ZIP 程序，再以移除远程调试参数的普通方式启动正式安装供使用。

| 本轮实际命令 | 结果 |
| --- | --- |
| `python -X utf8 output/playwright/m5-formal/verify.py backup` | 原 v1 数据库一致性及原程序/安装包回退副本通过 |
| `./output/playwright/m5-formal/install.ps1 -Action upgrade` | 原目录 0.1.0→0.5.1 升级，启动成功；进程 39688，有有效窗口句柄 |
| `python -X utf8 output/playwright/m5-formal/verify.py upgraded` | v1→v7、原行和时间、数据库完整性/外键、回退材料哈希通过 |
| `node output/playwright/m5-formal/smoke.cjs installed` | 正式库读取、创建/恢复备份及三个页面检查通过，页面错误 0 |
| `./output/playwright/m5-formal/install.ps1 -Action restart` | 正常退出并重启，进程 39688→21448 |
| `node output/playwright/m5-formal/smoke.cjs restart` | 重启后正式库原记录、空业务表和页面状态通过 |
| `node output/playwright/m5-formal/delivery.mjs` | 复用现有 `collectAssets`，将明确选定的当前构建整理成 EXE、ZIP、使用手册、说明及校验清单；未更改产物筛选规则 |
| `./output/playwright/m5-formal/zip.ps1` | ZIP 解压成功，二进制 SHA-256 与已构建程序一致，进程 51304 启动 |
| `node output/playwright/m5-formal/smoke.cjs zip` | ZIP 版读取同一正式数据库，旧记录及页面检查通过 |
| `python -X utf8 output/playwright/m5-formal/verify.py restarted`、`zip`、`closed` | 重启、ZIP 使用和关闭后的完整性/外键、旧记录及回退材料哈希均通过 |
| `Get-FileHash -Algorithm SHA256`（逐项对照交付清单） | EXE、ZIP、同步后的使用手册及交付说明哈希全部匹配 |
| `git diff --check` | 通过 |

原生文件选择使用 Computer Use 技能实测：对话框能够打开，按 Escape 取消后应用显示“已取消选择，保留原路径”，页面仍可使用。本轮选中具体文件未完成：工具对原生弹窗的控件缓存和坐标定位不一致，出现 `Cannot set a value for an element that is not settable`、`element ... is not available in cached app state`，未据此修改应用，也没有把选择动作标记为成功。具体路径选择的既有 M1/M3 历史证据及 M5 IPC 路径验证仍保持原范围。

本地交付目录：`output/delivery/m5-windows-0.5.1/`，包含：

- `shuyin-ai-0.5.1-windows-x64.exe`
- `shuyin-ai-0.5.1-windows-x64.zip`
- `使用手册.md`
- `交付说明.txt`
- `SHA256SUMS.txt`

这是包含 M5 修改的本地验收构建，不是 GitHub 已发布的新版本。安装包 SHA-256 为 `BDC62C8F359A3296F3937D12B8DD56CD2BD0904B60A12CAC3B3ECE7A9D5FA04F`；ZIP 中程序 SHA-256 为 `2CF9EC94333766B09C9B0C47208B5954344576AE86523D22572E22EFFDD5E900`。正式发布仍由流水线增加版本。

本轮仅更新验收及交付文档，不改业务代码或数据库迁移，所以未重复运行上轮已通过的完整单元测试和构建。测试脚本、回退副本和安装产物均在忽略目录；工作区仍未暂存、提交或推送。没有发现新增应用缺陷。

最终正式应用普通启动进程为 42128，有有效窗口句柄，已激活到「我的书籍」页面；验收用 19348 调试端口未监听。

回退材料仅保管，未执行降级。若需要回退，先关闭程序并另外备份升级后的数据，再配套使用原 0.1.0 程序和 v1 数据库；不要让旧程序直接打开 v7 数据库，v1 也不能从当前应用的 v2–v7 备份恢复入口导入。
