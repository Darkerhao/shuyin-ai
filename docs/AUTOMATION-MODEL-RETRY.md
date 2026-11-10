# 自动制作模型重试与切换（2026-11-10）

## 行为与验收标准

- 自动制作从用户所选文本、图片模型开始；视频页打开已有任务时恢复该任务模型，缺失或停用时不静默改用默认模型。
- 模型首次失败后，使用同一模型最多再重试3次，分别等待2、4、8秒；仍失败后自动切换其他已启用的同类模型，无需用户逐次选择。备用模型按默认优先、名称、ID确定顺序，每个备用模型也最多首次请求加3次重试。
- 已耗尽的模型在本轮流程中不再选回；切换成功后的模型用于后续同类步骤。全部候选用尽则暂停并保留最后错误，用户明确重试才重置本轮重试记录。
- 重试覆盖目标时长、创作方案、字幕模型校对、内容校核的模型请求、封面背景及分镜；视频从已保存字幕校对/分镜检查点继续，封面只重做未完成的当前尺寸。
- 存储失败、内容校核无法证实、音频制作/上传、字幕本地转写、封面本地排版、视频渲染和发布失败仍按原流程暂停处理。取消不触发模型重试，暂停/停止期间不发起下一次尝试。
- 页面显示当前模型、重试次数或自动切换说明；启动授权说明包含可能增加的模型调用费用，最终发布仍由用户完成。

## 实现与数据

复用自动流程唯一调度器、原AI/视频任务和Provider接口，不增加网络重试层、依赖、权限或SQL迁移。单次请求仍为300秒；2/4/8秒是请求失败后的等待，不是请求超时。

`automation_runs.input_json.modelRetry`保存是否启用、当前模型的重试次数和已耗尽模型ID。阶段成功重置重试计数，但保留已耗尽名单。自动切换在同一SQLite事务内同步流程模型快照、当前AI任务provider/model/input或视频provider，并重新标记当前失败任务为待执行。已成功版本、字幕、音频、分镜与图片不清空；请求诊断仍记录每次实际调用。

旧JSON缺少该字段时按未启用读取；用户继续或重试旧任务后启用当前规则。应用重开仍先暂停，重试计数随备份和重启保留；继续已失败子任务只恢复待执行，不重置次数，明确“重试当前阶段”才重新开始一轮。普通手动生成不启用自动切换。

视频任务原先仅保存错误文字；模型调用及模型输出校验边界统一使用“模型生成失败：”标识，调度器只对这一类视频失败重试。文件、数据库、渲染、停止等错误不加此标识。旧失败任务通过原继续入口重新执行后应用新的错误分类。

## 根因

`VideoPage.load`原来无条件选择全局默认模型，未恢复`video_jobs.provider_id`，续作再把该默认ID传入`continue_video_job`。新增回归测试在修复前确实出现“期待任务所选模型，实际默认模型”。自动流程自身原已保存选择，但首次错误直接进入`waiting_user`，无有限重试或自动切换。资料页自动制作设置在重挂载时同样只展示默认模型，现跟随活动任务的实际选择。

## 验证边界

本轮使用本地组件测试和隔离SQLite任务测试验证模型选择、失败次数、切换、暂停/取消/过期保护和已保存进度。Rust命令先执行`. ./.tools/env.ps1`加载现有工具链。

| 实际命令 | 最终结果 |
| --- | --- |
| `npx vitest run tests/video.test.ts tests/automation.test.ts tests/automation-tasks.test.ts` | 48项通过 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked automation_` | 38项通过，含兼容迁移/备份及6项新增恢复测试 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked video_commands::tests` | 5项通过，含字幕批次失败后继续 |
| `npm run typecheck` | 通过 |
| `npm run lint` | 通过 |
| `npm run build` | 通过，保留Vite产物体积提醒 |
| `npm run check:rust` | fmt和Clippy全部target通过，警告作为错误 |
| `git diff --check` | 通过 |

根因复现命令`npx vitest run tests/video.test.ts -t 'restores the task model|requires an explicit replacement'`在修复前2项均失败；相同用例已包含在最终48项通过结果中。新增Rust测试初次因测试中将默认模型停用而失败，修正为合法非默认停用配置后通过。npm提示已有python/msvs_version用户配置将在未来版本失效，本轮未修改全局配置。

验收标准的模型保留、3次重试后自动切换、同类/启用筛选、候选用尽停止、部分分镜及原内容快照保留、重启计数保留、旧JSON兼容和状态回显均有本地测试覆盖。未调用外部付费模型，未上传真实音频/视频，未执行桌面全链、抖音最终发布或新安装包验收。所有候选均不可用时只能暂停，有限重试不保证外部服务一定成功。

## 修改文件与下一步

- 调度与恢复：`src-tauri/src/automation.rs`、`src-tauri/src/database/automation.rs`。
- 视频模型错误分类：`src-tauri/src/video.rs`、`src-tauri/src/video_commands.rs`。
- 任务模型回显与状态：`src/modules/video/VideoPage.vue`、`src/components/AutomationSetup.vue`、`src/components/AutomationTasks.vue`、`src/services/automation.ts`。
- 回归：`src-tauri/src/database/automation_tests.rs`、`tests/video.test.ts`、`tests/automation.test.ts`、`tests/automation-tasks.test.ts`。
- 文档：本文件以及PRD、TECHNICAL-DESIGN、DATABASE、AI-RULES、DEVELOPMENT-PLAN、USER-GUIDE。

下一步建议构建Windows安装包，在实际失败任务上验收模型故障→重试→自动切换→沿用原分镜进度。用户已要求先同步远端，再提交推送dev并cherry-pick推送main；Git推送与流水线触发不代表安装包发布或正在运行的安装版已更新。
