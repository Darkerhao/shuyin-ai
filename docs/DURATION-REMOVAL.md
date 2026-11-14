# 取消推荐时长与创作时长预算（2026-11-14）

## 当前需求与验收标准

用户不再需要“推荐时长”，创作不应受目标分钟数限制；网站自动操作固定选择“默认”时长。

1. 整书与章节均移除推荐按钮、分钟输入、快捷分钟数、目标展示及自动推荐请求。
2. 新建、保存、全量生成、按项调整和启动音频均不需要目标时长；自动流程从完整创作方案开始。
3. 新提示词不计算目标分钟数、预计口播字数、开场/结尾时间和分段预算，按提供内容的信息量自然展开。保留来源真实性、固定口播、范围校验及原有内容结构。
4. Notebook 音频配置固定选择“默认”，不依据旧目标选择更短或更长。缺少对应控件时报告页面错误，不猜测其他选项。
5. 既有创作版本、原书资料、素材文件和来源关联保持；实际音视频继续使用完整时间轴，字幕与媒体校验保持。

## 实现与影响范围

- `WritingPage.vue` 删除推荐和保存时长状态、离页保存链、自动推荐、时长面板与相关生成禁用条件；保留草稿暂存、取消及候选采用流程。历史章节音频取用只比较内容来源，不因旧目标不同而阻止取用。
- `ProjectChapters.vue`、书籍卡片、自动制作设置/任务展示及全局任务列表同步删除目标入口；清理专属样式与已停用语音提醒资源。
- `GenerationInput`、`GenerationResult`、文本执行器及数据库生成事务移除推荐分支和目标参数；删除 `set_audio_duration` Command、权限和推荐解析模块。旧推荐任务不能再发送模型请求。
- 自动制作直接创建 `writing` 阶段；旧流程停在 `duration` 时，继续/重试均不重新排队推荐，执行器在同一事务内推进到 `writing` 并结束仍为 pending/running 的旧推荐。已有终态及历史子任务 ID 保留供核查。
- 音频快照与网站执行输入不再携带目标时长；音频启动和后置内容校核继续核对实际使用的版本、指导、来源摘要及完整字幕。
- 新 Prompt 版本为 `publishing-v9`、`publishing-revision-v8`、`audio-template-v5`。历史 Prompt 文件及已保存文本保留；用户重新生成或调整音频指导时才采用新规则。
- `audio_page.js` 在配置格式和语言后固定选择默认时长，再填写完整指导；配置步骤不提交生成。

## 数据库与兼容性

仍为 schema v25，无迁移、无新依赖和权限扩张。书籍/章节表旧目标列与来源列保留为历史元数据，不参与创作，也不再通过资料保存接口编辑。由于旧书籍表目标列非空，新书写入原默认占位值 600；该值不进入新任务、Prompt 或网站操作。章节新记录目标为 NULL。备份和恢复继续保存旧列及旧版本。

新音频快照不写目标字段；读取含旧目标的音频快照仍能恢复原来源与提示词。旧自动流程的目标字段不再进入运行模型，其实际指导和媒体成果不被改写。原始视频及音频文件没有被裁切、变速、重生成或删除。

本轮为现有行为清理与修正，按 `fix` / patch 判断，不手工升版，版本由既有发布流水线管理。

## 首次实现验证记录

测试使用本地隔离数据和页面夹具，不调用真实收费模型或点击真实网站生成/发布。Rust 命令执行前均运行 `. ./.tools/env.ps1`。

| 实际命令 | 结果 |
| --- | --- |
| `npm run typecheck` | 通过 |
| `npm run lint` | 通过，0 个 ESLint 警告 |
| `npx vitest run tests/ai.test.ts tests/chapters.test.ts tests/automation.test.ts tests/automation-tasks.test.ts --maxWorkers=2` | 4 个文件，120 项通过 |
| `npx vitest run --maxWorkers=2` | 37 个文件，535 项通过 |
| `npx vitest run tests/ai.test.ts --maxWorkers=2` | 最后补验：95 项通过，包含同一章节来源、不同旧目标仍可取用历史音频指导 |
| `node --test tests/audio-page.test.mjs` | 48 项通过；两种音频格式均选择默认，不点击更短，且不提交生成 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked --lib` | 296 项通过，2 项原有环境测试 ignored |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets -- -D warnings` | 通过 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | 通过 |
| `npm run build` | TypeScript 与 Vite 构建通过；仍有主 JS chunk 超过 500 kB 的体积提示 |
| `git diff --check` | 通过 |

2 项 ignored 分别为原生系统凭据往返测试和要求 `SHUYIN_TEST_NODE` 的视频子进程终止测试，本轮未启用。首次实现时未运行完整 `npm test` 的视频渲染、发布打包及队列脚本；后续独立评审已补跑，见下方记录。未执行真实桌面/Notebook 登录会话验收或重新生成安装包。

初次 Rust 测试有两项断言仍要求生成“10分钟 / 30分钟”，根据新需求改为验证不含该预算，后续完整 Rust 测试通过；没有关闭类型、lint 或测试规则。差异复查确认删除的是推荐功能自身的实现、专属样式/资源和旧行为测试，来源隔离、取消/事务、历史恢复及实际长音频校验仍在测试范围内。

验收标准 1—3、5 已通过前端/后端回归测试；标准 4 已通过网站 DOM 夹具测试，真实网站状态仍需桌面验收。

## 独立评审与补测（2026-11-14）

固定基线为 `9d0db0627cb32f81eab84c10ba5386acacad0626`，评审当前工作区差异及三份新增 Prompt，未创建提交。通过 `code-review` 的两个独立只读代理分别检查工程规范（Standards）与需求覆盖（Spec）。

两路首轮各发现 1 项 P2，为同一个问题：旧自动流程停在 duration，应用恢复后原推荐为 failed；普通继续或显式重试仍会把它重置成 pending，随后执行器跳过推荐，使该任务永久残留在全局“生成中”，并阻止模型删除或用途切换。

先增加两个回归测试，运行 `cargo test --manifest-path src-tauri/Cargo.toml --locked retired_duration_ -- --nocapture`，修复前 **0 通过、2 失败**。随后在 `database/automation.rs` 修复：

- `generation_task()` 不再将退役推荐当作可恢复生成任务。
- 显式重试先校验数据库中当前代次和状态，遇 duration 不重新排队。
- `set_automation_stage()` 用同一事务推进 writing 并取消旧 pending/running 推荐；成功、失败和取消的历史终态及任务 ID 保留。

相同命令修复后 **2 通过、0 失败**。回归覆盖普通继续/显式重试与五种旧推荐状态，验证新创作保存后没有遗留活动推荐；另以数据库触发器模拟取消写入失败，验证阶段变更一起回滚，过期运行编号也不能取消任务。

复评结果分开记录：Standards 首轮1项P2，修复后0项未解决、0项额外代码异味判断；Spec 首轮1项P2，修复后0项未解决、无新增需求偏离。两个代理均基于当前代码和测试只读复核，实际命令由主线程执行。

完整 npm 测试使用已安装的 `D:/MyWork/software/shuyin-video-node/v24.22.2`，仅置于当前测试进程 PATH 首位，并设置 `VITEST_MAX_WORKERS=2`。默认 Node 24.11.0 首次执行视频测试为33通过/7失败（现有入口明确拒绝此版本），队列18通过/1失败；不修改测试或业务代码，切换到已有兼容版本后完整 `npm test` 通过：前端535、视频40、发布/音频85、队列19项，另1项macOS专属检查跳过。

| 本轮最终命令 | 结果 |
| --- | --- |
| `npm test` | 前端535、视频40、发布/音频85、队列19通过；1项macOS专属检查跳过 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked retired_duration_ -- --nocapture` | 2项回归从失败变为通过 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked --lib` | 298通过，0失败；原有2项环境测试ignored |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets -- -D warnings` | 通过 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | 通过 |
| `npm run lint` | 通过 |
| `npm run build` | TypeScript与Vite构建通过；仍有主JS chunk超过500kB的提示 |
| `git diff --check` | 通过 |

本轮日志保存于 `.tools/duration-review-*.log`。没有执行真实收费模型、真实网站生成/发布或安装包构建。

本轮修复仅涉及 `src-tauri/src/database/automation.rs`、`src-tauri/src/database/automation_tests.rs` 和本记录及开发计划；没有数据库结构、依赖或权限变更。

## 使用边界

网站的“默认”由外部服务决定实际产出长度，应用不承诺无限长度或精确时长。旧保存指导中的分钟文字保留；重新生成/调整音频指导后才获得无时长预算的新版本。桌面安装包需后续构建并安装才会包含本次源码修改。
