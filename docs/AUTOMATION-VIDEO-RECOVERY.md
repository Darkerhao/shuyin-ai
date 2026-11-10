# 自动视频局部失败恢复（2026-11-10）

## 问题与根因

视频页「继续准备字幕与分镜」调用`continue_video_job`，通过`claim_video_job`进入`claim_bound_video_job`。原实现发现视频属于任意未结束的v2自动流程就拒绝，连paused、waiting_user也不例外，因此显示“视频由自动流程管理，请在全局任务中继续，或停止流程后手动制作”。单项失败保留原流程是正确行为，错误在于把保留关联与禁止局部恢复绑定在一起。

新增回归先在原实现运行：`cargo test --manifest-path src-tauri/Cargo.toml --locked automation_video_can_recover_locally_without_cancelling_the_flow`，结果1失败，返回与截图完全相同的拦截文字。测试使用隔离SQLite和原视频领取入口，不使用正式书籍数据或外部模型。

## 实现与验收标准

- 已暂停或待处理流程，当前处于字幕或分镜阶段：允许视频页继续原任务，不取消流程或创建新任务。
- 当前字幕阶段只执行Transcript，完成后保存transcript_ready；分镜阶段执行Prepare，保留已保存字幕及部分分镜，从原检查点继续。
- 全局流程保持paused，完成局部修复后点击全局「继续」再推进；不因局部操作自动启动下游调用或发布。
- 本次选中的文本模型同步到流程选择与配置快照，显式局部续作重置本轮模型重试记录；后续全局继续使用新模型。
- 原流程运行中、等待最终发布、当前在其他阶段、错配工作类型、无效模型、视频重复领取均拒绝，保持原状态。
- 视频领取和关联流程更新共用事务；流程运行代次递增，使旧调度器不能推进。取消/完成后的手动制作继续沿用原行为。
- 原内容校核、三尺寸封面及渲染快照验证继续有效。修改已校核字幕仍会使原校核失效，本次不增加自动重做上游材料的逻辑。

## 修改文件与数据

`src-tauri/src/database/automation.rs`集中读取视频所属活动流程；`src-tauri/src/database/video.rs`处理局部恢复准入与事务；`src-tauri/src/video_commands.rs`识别字幕检查点；`src-tauri/src/database/automation_tests.rs`增加恢复与拦截回归，并扩展三封面→分镜→渲染测试覆盖单项恢复后的接续。

同步PRD、TECHNICAL-DESIGN、DATABASE、AI-RULES、DEVELOPMENT-PLAN及USER-GUIDE。数据库保持v23，无新依赖、权限、命令或第二套任务状态。属于fix/patch，不手工改版本号。

## 验证

Rust命令先执行`. ./.tools/env.ps1`加载仓库现有工具链。

| 实际命令 | 结果 |
| --- | --- |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked automation_` | 40项通过，包含局部恢复和冲突保护新增回归，以及三封面保留后接续渲染的数据库验证 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked video_` | 28项通过、1项按原设置忽略；忽略项为需要SHUYIN_TEST_NODE的进程终止测试 |
| `npx vitest run tests/video.test.ts tests/automation.test.ts tests/automation-tasks.test.ts` | 3文件48项通过 |
| `npm run check:rust` | Rust fmt及所有target的Clippy通过，警告作为错误 |
| `git diff --check` | 通过 |

验收标准的局部续作、部分字幕/分镜保留、模型同步、重复执行/错阶段拒绝、字幕检查点及保留三封面后接续渲染均有本地测试覆盖。未新增或修改前端代码，本轮未另跑前端lint、typecheck或Vite构建。npm提示本机已有python/msvs_version配置将于未来版本失效，未修改全局环境。

未执行真实付费模型、Windows桌面交互、完整渲染、安装包升级或发布验证。下一步使用包含修复的桌面构建，在原失败任务上核对：直接继续→原字幕及分镜保留→全局继续推进。源码修改不会更新当前运行的安装版。

## 独立评审与补充测试（2026-11-11）

评审基线为`def90da7b506cc88d292c06e3df3b4bc2959e337`，范围是本次未提交改动及本文档。按工程规范与需求符合性两个维度分别只读评审，结果如下：

- Standards：0项确定问题。事务、阶段配对、代次保护、原任务复用及文档边界符合工程规范，没有引入重复状态或另一套执行流程。
- Spec：0项确定问题。暂停/待处理局部续作、成果保留、模型同步、字幕检查点与全局继续的调用链符合本轮约定；没有扩大到直接发布或跳过内容校核。

补充`automation_video_local_recovery_rolls_back_when_flow_update_fails`：在隔离SQLite中用临时trigger注入流程更新失败，确认事务中此前领取的视频任务也回滚，流程与视频快照保持原样。新增测试先通过，但Clippy指出`.err().expect()`写法，已改为`expect_err()`并重新验证。本轮未新增业务代码或数据库结构变更。

| 本轮实际命令 | 结果 |
| --- | --- |
| `npm run test:rust` | 全量271项通过、2项按原设置忽略；此时尚未加入上述新回滚测试 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked automation_video_local_recovery_rolls_back_when_flow_update_fails` | 新增回滚测试1项通过 |
| 设置本进程`SHUYIN_TEST_NODE=(Get-Command node).Source`后，`cargo test --manifest-path src-tauri/Cargo.toml --locked cancelling_video_worker_terminates_its_descendants -- --ignored` | 原忽略项单独执行通过，停止worker时其子进程也退出 |
| `npx vitest run tests/video.test.ts tests/automation.test.ts tests/automation-tasks.test.ts` | 48项通过 |
| `npm run lint` | 通过，0警告 |
| `npm run build` | TypeScript检查与Vite构建通过；保留已有大分块提醒 |
| `npm run check:rust` | 修正新测试写法后，fmt及Clippy通过 |
| `git diff --check` | 通过 |

评审与测试覆盖代码和本地状态闭环，不代替Windows桌面命令到worker、真实模型和成片的整链验收；这些范围仍待使用新构建验证。未提交、推送或发布。
