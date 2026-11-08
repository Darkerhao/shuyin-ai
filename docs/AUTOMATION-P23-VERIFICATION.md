# 自动制作 P2/P3 实施与本地验收（2026-11-08）

Git交付补充：用户随后授权先拉取dev、中文提交并推送，再备份/拉取main后cherry-pick同一提交并推送。dev已拉取上游v0.54.0，仅版本清单变化，37个实现/测试/文档改动保持。下文“不提交、不推送、0.53.2”描述实施与验证时基线；Git交付按当前授权执行，实际结果见分支历史和交付回复，不提前宣称自动打包发布成功。

**P2/P3代码已实施，工程与隔离Windows制作链通过；三本真实书籍、真实账号网页准备的完整零中途操作验收尚未执行。P4正式自动提交尚未交付。** 本文对应[主计划](BOOK-TO-DOUYIN-AUTOMATION-PLAN.md)A06–A17，不以本机Provider、测试声音或本地发布任务替代正式平台验证。

基线dev/18f86ca，应用0.53.2，开始时保留已有P2/P3未提交实现。本轮无新依赖、权限、版本号调整、提交、推送或安装升级。SQLite由v22升级v23。

## 实现与修改范围

- `src-tauri/src/automation.rs`、`database/automation.rs`：唯一顺序改为字幕→校核→三尺寸背景/导出→同任务分镜/渲染→网页准备，独立内容与渲染摘要、准确最终版本和素材关联。
- `ai/content_check.rs`、`prompts/content-check-v1.md`：显式verified/reason/changes，无法核实暂停；仅修订必要发布/封面字段，沿用原revision与完整发布校验。无变化复用初稿，有变化追加based_on，实际音频指导与目标不变。删除无调用的transcript-publishing-v1旧Prompt。
- `video.rs`、`video_commands.rs`、`database/video.rs`：复用同一转写/校对实现增加Transcript检查点，保存draft/transcript_ready；封面后继续同一任务分镜，已完成校对不再调用。自动任务活动时拒绝手动启动越过阶段，停止后可继续手动任务。
- `migrations/0023_automation_media.sql`、`database.rs`及AI/音频/图片关联：第三尺寸任务/封面与render_plan_sha256、workflowVersion=2；暂停并保留旧双封面流程及上传标记。
- `AutomationSetup.vue`、`AutomationTasks.vue`、`services/automation.ts`、Writing/Covers/Video页面与类型：三图片授权、真实阶段/失败阶段、目标与实际差值、三张成品及准确文案/音频/视频/发布入口。CoversPage复用原CoverThumbnail只读查看指定版本，保留手工草稿。
- `tests/automation.test.ts`、`tests/covers.test.ts`、`tests/ai.test.ts`、Rust automation/content_check回归，以及产品/技术/数据/AI规则/用户手册/计划与验收文档。

v23只追加两个可空外键和一个摘要，不改写既有业务记录。旧活动流程暂停增加代次，原输入及子任务、上传/交接标记保留，不自动补第三次付费图片调用。旧备份在副本迁移，源文件不改写；回退程序需使用升级前备份。

## 实际验证命令

工作目录为仓库根；Rust命令先执行`. ./.tools/env.ps1`。Node视频使用现有`D:/MyWork/software/shuyin-video-node/v24.22.2/node.exe`；未安装或替换全局工具。

| 命令 | 结果 |
| --- | --- |
| `npm run lint` | 通过，最终UI改动后复验通过 |
| `npm run build` | TypeScript检查及Vite生产构建通过 |
| `npx vitest run tests/automation.test.ts tests/covers.test.ts tests/ai.test.ts tests/video.test.ts tests/video-progress.test.ts tests/publishing.test.ts` | 6文件239项通过 |
| `npx vitest run tests/automation.test.ts tests/douyin.test.ts tests/douyin-editor.test.ts tests/publish-queue.test.ts` | 4文件59项通过；与上行有重叠，不累计为独立总数 |
| `npm run test:rust` | 260通过、2项既有忽略；原生凭据与显式Node进程专项没有运行 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked automation` | 最终内部命名清理后32项通过，含迁移/旧备份/回滚及三尺寸失败续作 |
| `npm run check:rust` | fmt及Clippy `--all-targets -- -D warnings`通过，最终代码复验通过 |
| `D:/MyWork/software/shuyin-video-node/v24.22.2/node.exe --test tests/video-render.test.mjs` | 39项通过 |
| `npx tauri build --debug --no-bundle --config output/playwright/automation-p23-20261108/tauri.json` | 最终隔离Windows构建通过，含TypeScript/Vite；未打安装包 |
| `node output/playwright/automation-p23-20261108/native.mjs` | 最终构建完整复跑通过，退出0；见下节真实IPC/Canvas/渲染证据 |
| `node output/playwright/automation-p23-20261108/reopen.mjs` | 最终构建重开，精确ID/文件/版本保留，未知上传不重发，模型请求0，3档布局通过 |
| `python output/douyin-automation-p4/refresh.py` | 9份官方文档实时读取成功；接入结论见[P4复核](AUTOMATION-P4-VERIFICATION.md) |
| `git diff --check` | 通过 |
| Python Markdown编码/冲突标记/本地链接检查 | 180个本地链接通过 |

测试过程中遇到的问题：恢复用例只暂停音频不满足原备份规则，改为先停止跟进；原生种子少填ai_tasks.input_json被非空约束拒绝并回滚，补齐后执行；测试脚本请求window.close被现有权限拒绝，未扩权，后续沿用已有destroy权限结束隔离窗口。系统Node24.11首次跑视频测试33通过/6失败，已确认被Windows版本检查提前拒绝，用现有24.22.2后39通过。没有修改测试断言或弱化产品限制。

## 隔离Windows证据

文件与脚本在忽略目录`output/playwright/automation-p23-20261108/`。独立identifier为`com.shuyin.automation-p23-20261108`，SQLite按Tauri规则保存在C盘独立应用目录；WebView、工程、原声/字幕及全部产物在D盘output。正式安装进程、正式数据库和原始媒体未改动。

从准备好的前置版本/音频/SRT开始，实际执行统一Provider与原执行器：

1. 6秒本地测试声音和一条现成SRT，真实探测时长、字幕校对1次，进入同任务字幕检查点。此处未运行ASR或NotebookLM。
2. 内容校核1次，只改标题；追加1个based_on版本。最终提示词与前置音频指导逐字段一致。
3. 三次独立图片请求，Provider尺寸依既有适配器为1536×1024、1024×1536、1536×1024；Canvas输出分别1440×1080、1080×1440、1920×1080，文件头/登记版本逐个核对。生成过程未进入封面页。
4. 只有三个封面导出均成功后才请求1次分镜。同一个视频任务经历字幕、分镜、渲染三次运行；实际生成6秒1920×1080带音轨MP4与SRT，音频/原书摘要保持不变。
5. 只建1条发布任务，准确绑定最终文案、原音频、正式成片及横竖封面；禁止保存、未来北京时间01:11、人工确认字段false，未写published。无真实账号，未进行网页实际上传/填写。
6. 精确封面链接显示目标文件；1440/1120/800宽度无横向溢出。已查看成品截图和PNG中文排版，测试背景为本机生成渐变，不代表外部图片模型质量。
7. 最终构建重开，全部child IDs、三份PNG、两个文案版本及同一个成功视频保留；继续原发布阶段提示会话丢失、核对原草稿，Provider请求0，发布任务仍只有1条。没有以重启为由重新生成或上传。

`native-result.json`、`reopen-result.json`记录本次请求数、阶段、ID、尺寸和结果；原生截图及三张PNG可直接复核。初始脚本的失败记录保留，最终结果不覆盖当时遇到的问题说明。

## 验收对应与剩余边界

| 验收 | 当前结果 |
| --- | --- |
| A06 校核保留实际指导 | 通过本地解析/事务回归及真实IPC；无法确定、非法音频改写、字幕变更均拒绝 |
| A07 目标与实际时长 | 目标/实际及差值已展示；实际超15分钟沿用暂停检查；真实长音频质量未验 |
| A08 三尺寸 | 三次实际本机Provider请求、三个准确PNG与版本通过；外部AI效果待验 |
| A09 局部失败 | 第三张背景失败只重试该任务；导出失败复用成功背景，前两尺寸保留，数据库回归通过 |
| A10 同一视频任务 | 字幕检查点→三封面→分镜→渲染顺序及单任务三次运行通过；现成SRT不证明ASR效果 |
| A11 摘要一致性 | 字幕变化拒绝旧校核、仅分镜变化不影响文案；渲染冻结后分镜改变拒绝，回归通过 |
| A12 停止与草稿 | 重复启动/迟到结果/停止回归与精确封面不改手工草稿通过 |
| A13 恢复 | 新流程重开/旧备份、精准ID保留、未知上传不重发通过；真实远端未知结果待验 |
| A14 旧数据 | v19/v20/v21/v22升级、旧授权暂停、标记保留、备份源不变及迁移失败回滚通过 |
| A15 精确材料 | 本机实际发布任务绑定正确，16:9仅本地归档，01:11/禁止保存通过 |
| A16 网页边界 | 沿用网页自检/最终人工发布，相关回归通过；新流程真实账号awaiting_publish尚未验收 |
| A17 全自动提交 | 未通过；[P4所需资料](AUTOMATION-P4-VERIFICATION.md)尚缺，不记为完成 |

未执行三本不同体裁真实书籍整链、外部付费模型、NotebookLM生成、真实抖音上传/最终投稿、长片全量质检、安装升级及macOS/Linux实机。本机产物和状态测试不能代替这些验收。npm用户配置提示和Vite约817kB主包警告保留，不为本任务修改全局配置或构建阈值。

下一步优先用有权使用的三本书验证真实P2/P3整链，记录次数、时间、产物ID和零中途操作；并补齐P4的应用能力证明与正式契约，再实施真实提交和结果归档。
