# 章节精讲独立评审与补测

日期：2026-11-13。固定基线：`b55ec9cef0926ab2b6d9c407bedefcfb8b5b8b2c`。用户要求在章节功能完成后进行评审测试。本报告覆盖本轮独立评审、可复现问题、最小修复及新执行的验证，上一轮实施记录保留于[功能验收](CHAPTER-WORKFLOW-VERIFICATION.md)。

## 范围和方法

评审命令为`git diff b55ec9cef0926ab2b6d9c407bedefcfb8b5b8b2c --`，并读取`git ls-files --others --exclude-standard`中的新文件。开始时共89份已跟踪修改与13份新文件，没有遗漏尚未提交的SQL、Prompt、章节模块和测试。

采用code-review两条独立只读评审：Standards依据AGENTS、AI规则和项目最少实现原则；Spec依据用户最新澄清、[章节设计](CHAPTER-VIDEO-DESIGN.md)C01—C16及PRD。主线程对发现补充行为回归，先验证失败，再修复，并请两轴复核最终增量。评审代理读取测试日志不算自行重跑测试。

独立评审阶段未拉取、提交、推送或合并远端代码。用户随后授权Git交付，该步骤在评审完成后单独执行；实际SHA与推送状态以Git交付回执为准。没有操作正式数据库、真实模型、NotebookLM或发布账号。

## Standards

最终：**通过，0项明确违规，0项需要处理的代码异味**。

最终复核确认五项修复均直接修改原调用链：准确媒体查询复用原文件校验；章节入口归并到现有路由和草稿守卫；手工音频仅更新目标、保留原文快照；单镜任务保存原章节；历史音频兼容性使用派生状态、不增加持久化状态或重复历史机制。未新增依赖或第二套执行器。

Standards检查发现的媒体预览和单镜归属问题纳入下方功能问题列表，避免被“规范通过”掩盖。最终需求符合性单独评估。

## Spec

最终：**通过，初审、补充评审与主线程核对共5项P2，全部修复，剩余0项**。

| 问题 | 原触发和影响 | 最小修复及对应规格 |
| --- | --- | --- |
| P2 章节音视频无法通过应用播放器预览 | `media_asset_path`从`list_assets(project, None)`寻找素材。该列表仅含整书音视频，已登记的章节音视频被过滤，返回“只能预览本书已登记的音频或视频” | [assets.rs](../src-tauri/src/database/assets.rs)按书籍与精确素材ID查询audio/video，再使用原路径、类型及存在性校验；[video_commands.rs](../src-tauri/src/video_commands.rs)沿用单文件协议授权。符合C11完整章节音视频能力 |
| P2 收起章节输入丢失最后编辑 | `chaptersOpen`直接卸载编辑器，400ms未触发时没有保存；暂存失败也可收起。路由守卫没有机会执行 | [ProjectPage.vue](../src/modules/projects/ProjectPage.vue)删除局部折叠状态，按钮与选择器统一进入`?chapter=new`，离开按原暂存守卫保存或阻止导航。符合C03失败保稿 |
| P2 手动改音频指导后仍用旧目标 | 5分钟版本→章节改15分钟→手动改指导并保存，新版本完整继承旧来源快照的300秒；创建音频时实际仍为300秒 | [database/ai.rs](../src-tauri/src/database/ai.rs)手工保存时比较audioPrompt，实际改变才把本章当前目标写入新快照。正文、章节名仍继承旧版本；只改标题继续300秒。符合C05/C09来源及目标一致 |
| P2 单镜调整任务记入整书 | 章节视频调用`start_video_scene_revision`创建ai_tasks时漏写chapter_id，本章任务列表为空，全局任务回到整书范围 | [video/revision.rs](../src-tauri/src/database/video/revision.rs)原INSERT补写job.chapter_id，章节历史与恢复入口保留原归属。符合C11 |
| P2 历史单项音频取用混合不同正文 | 同章正文A生成v1→改正文B生成v2→当前v2单项取用v1音频提示词，basedOn仍为v2，形成指导A加正文B | [WritingPage.vue](../src/modules/writing/WritingPage.vue)在按钮及操作函数核对来源快照与目标；不一致时引导切换完整版本。同来源、同目标仍允许单项取用，不影响其他字段取用。符合C09“旧任务/版本继续原快照” |

未将C16真实模型/平台整链未验收当作新代码缺陷，也未把本地测试宣称为C16通过。最终发布仍由用户在官方页面完成。

## 失败与修复证据

本轮新增5项前端测试、3项Rust测试，并补充单镜恢复路由断言，均保留原有测试。核心行为通过实际组件或Command调用的数据库接口验证。

| 首次运行 | 结果 |
| --- | --- |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked chapter_review_` | 媒体与手工时长2项失败：章节预览被拒绝；实际300秒、期望900秒 |
| `npx vitest run tests/projects.test.ts -t 'retains the last keystrokes' --maxWorkers=1` | 失败，关闭章节输入后saveDraft调用0次 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked chapter_review_scene_revision` | 失败，本章AI任务列表0条、期望1条 |
| `npx vitest run tests/ai.test.ts -t 'does not mix a historical chapter' --maxWorkers=1` | 失败，当前指导被旧版单项提示词替换，来源仍为新版 |

修复后`npx vitest run tests/ai.test.ts tests/projects.test.ts tests/chapters.test.ts --maxWorkers=2`为145项通过；`cargo test --manifest-path src-tauri/Cargo.toml --locked chapter_review_`为3项通过。覆盖同章/整书媒体、跨书拒绝、缺文件和目录拒绝、未到400ms离开、暂存失败阻止离开、手工目标和原文保留、单镜历史与全局归属、历史不兼容取用及同来源继续可用。

日志在`output/chapter-review-20261113/`：`rust-red.log`、`collapse-red.log`、`scene-red.log`、`history-red.log`、`focused-ui.log`、`rust-green-final.log`。媒体代码首次等价移到数据库接口以便测试，失败回归确认其原行为后才改变查询。

## 本轮完整验证

| 实际命令 | 结果 |
| --- | --- |
| `npx vitest run --maxWorkers=2` | 37文件、543通过、0失败 |
| `npm run test:rust` | 298通过、0失败、2项原有环境测试忽略 |
| `npm run check:rust` | rustfmt/Clippy全部target通过 |
| `npm run test:video` | 40通过、0失败 |
| `npm run test:release` | 80通过、0失败、1项macOS专属检查跳过 |
| `npm run test:queue` | 19通过、0失败 |
| `npm run lint` | 通过，0错误0警告 |
| `npm run build` | TypeScript和Vite生产构建通过，仍有原分块大于500KB的体积提示 |
| `python -X utf8 output/chapter-review-20261113/check_docs.py` | 6份文档168个本地链接通过 |
| `git diff --check` | 通过 |

日志分别为`vitest-full.log`、`rust-full.log`、`rust-check.log`、`video-node.log`、`release-node.log`、`queue-node.log`、`lint.log`、`build.log`。Node测试使用本机已有24.22.2；Rust先加载`. ./.tools/env.ps1`。TEMP/TMP/SQLITE_TMPDIR指向仓库`output/chapter-test-temp`，未改系统配置。两项原有Rust忽略为原生凭据存储验证和需SHUYIN_TEST_NODE的子进程终止测试。

首次lint的2项Vue换行警告已按规则修正，未关闭规则。测试夹具中的未登记路由警告不影响业务断言，实际应用路由未被替换。

## 文件、数据库与验收边界

本轮生产代码修改为[ProjectPage.vue](../src/modules/projects/ProjectPage.vue)、[WritingPage.vue](../src/modules/writing/WritingPage.vue)、[assets.rs](../src-tauri/src/database/assets.rs)、[database/ai.rs](../src-tauri/src/database/ai.rs)、[video/revision.rs](../src-tauri/src/database/video/revision.rs)、[video_commands.rs](../src-tauri/src/video_commands.rs)。测试在projects、ai、chapters与Rust章节/单镜测试中，同步更新章节设计、技术设计、开发计划、使用说明和验收记录。

schema仍为上一轮25，无新增迁移、依赖、权限或版本变更。完整Rust重跑包含旧数据库迁移、精确结构校验、备份/恢复及原整书工作流。

本轮未运行浏览器/Windows桌面人工播放、真实NotebookLM/模型/抖音、新安装包或macOS/Linux实机。上轮浏览器记录仅作背景，不冒充本轮复验。代码与本地验收已覆盖C03/C05/C09/C11修复及相关完整回归；C16保留待真实环境验收。

下一步建议在新版Windows应用用两个真实章节检查精讲质量和完整制作链，重点试听章节原声、播放成片、核对历史版本与发布素材，再由用户最终发布。
