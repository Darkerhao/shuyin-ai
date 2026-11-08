# 自动制作 P1 验收记录

日期：2026-11-08。范围：前置目标时长、完整创作方案及实际音频输入绑定。基线dev/db5cda3，应用0.53.2不手工升版；新增SQLite v22。

Git交付补充：用户随后授权先拉取dev、中文提交并推送，再备份和拉取main、cherry-pick本次提交并推送。下方“不提交或推送”指实现验收阶段；实际交付结果以提交历史、远端读回和本次最终回复为准，推送不等于安装包发布完成。

## 1. 实现与验收对应

| 要求 | 实际实现与证据 |
| --- | --- |
| 未配置先推荐，已配置不覆盖 | 新流程从duration或writing启动；15分钟保持900秒；未确定目标为null，推荐范围1—15分钟，超限输出事务回滚 |
| 完整方案先于音频 | 统一execute_writing_task保存writing_versions后，执行器才能进入audio；音频引用真实版本ID与流程冻结秒数 |
| 请求输入一致 | 书名、作者、简介、资料及模型配置在启动时冻结；真实隔离调用中，推荐13分钟后完整创作请求含780秒，音频指导与保存版本逐字符一致 |
| 成果可见、手工稿保留 | 全局任务提供精确版本链接；进入内容页不再重复推荐；已有草稿只打开历史对照，收到方案不覆盖当前编辑 |
| 失败及恢复 | 事务绑定保证重复启动/重复建子任务不产生多份结果；暂停保存已发出请求的合法结果而不推进；取消拒绝迟到结果，重启不自动重新请求 |
| 旧任务与后续流程 | v22将旧后置文案转到content_check_task_id，旧流程暂停；音频、原指导、素材及上传/交接标记保留。当前双封面和网页发布准备继续使用后置字幕文案 |

当前完整路径为：目标时长 → 完整创作 → 音频 → 原字幕/分镜 → 原字幕发布文案 → 双封面 → 渲染 → 网页准备。P2的字幕检查点、只修订必要字段、第三尺寸与后置分镜顺序未实施；P4正式自动提交未实施。不能把本记录解释为全部计划完成。

## 2. 修改范围与数据库

- Rust：`automation.rs`、`database/automation.rs`、`database/ai.rs`、`database/audio.rs`、`ai_commands.rs`及迁移注册；共用原Provider与音频创建路径，移除不再调用的固定自动指导构造函数和Prompt文件。
- 数据库：`0022_automation_creation.sql`新增duration_task_id/content_check_task_id；输入JSON增加流程格式、冻结资料和推荐说明；旧活动记录暂停，历史关联保留。精确备份校验自动覆盖新迁移，无降低校验。
- 前端：AutomationSetup、AutomationTasks、automation service、WritingPage，沿用既有组件、历史和草稿状态。
- Prompt：新增audio-duration-v2，按调用方可选范围推荐；手动生成不传范围时保持原允许范围。
- 测试：自动流程Rust、tests/automation.test.ts与tests/ai.test.ts；产品、技术、数据、AI规则、使用手册、开发计划和主计划同步。
- 无新依赖、Tauri权限、正式版本变更、提交或推送。旧数据库升级后不能用旧应用读取，回退使用升级前备份；旧任务需通过原详情继续或停止后新建，不自动补收费生成。

## 3. 实际执行的命令

Rust命令先执行`. ./.tools/env.ps1`。

| 命令 | 结果 |
| --- | --- |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked automation_frontloaded`（旧代码） | 3项真实失败，复现直接开始音频、900秒变600秒及超限未拒绝 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked database::automation_tests` | 当时24项通过，覆盖来源/时长/取消/恢复/备份/后置任务等 |
| `npm run test:rust` | 251通过，2项既有专项忽略，0失败 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked automation_v2` | 全量后新增迁移复核：3项通过，其中2项为本次追加的v21转存/标记保留与v22失败回滚测试 |
| `npm run check:rust` | 最终Rust fmt与Clippy通过，包含追加迁移测试的all-targets检查 |
| `npm run lint` | 最终通过；首轮新增RouterLink的2个格式警告已修复 |
| `npm run typecheck`、`npm run build` | 通过；最终build再次执行vue-tsc和Vite，保留已有大分块提示与npm用户配置提示 |
| `npx vitest run tests/automation.test.ts tests/ai.test.ts tests/generation.test.ts` | 最终3文件106项通过 |
| `cargo rustc --manifest-path src-tauri/Cargo.toml --locked --bin shuyin-ai -- -o D:/MyWork/project/JIAZI-PROJECT/shuyin-ai/output/playwright/automation-p1-20261108/shuyin-p1.exe` | 使用隔离TAURI_CONFIG成功构建；存在输出文件名/-o既有编译提示，未替换用户运行程序 |

`git diff --check`通过；PowerShell here-string执行`python -`检查29个变更路径、162个本地文档链接、编码/冲突、未改版本、空暂存区及真实原生证据通过。未运行全量前端/Node发布脚本测试，改动集中于上述已验证调用链；没有把未运行项记为通过。

## 4. 隔离 Windows 原生验证

隔离identifier为`com.shuyin.automation-p1-20261108`，工作资料在项目忽略目录`output/playwright/automation-p1-20261108`，测试数据库由Tauri保存于该identifier的本机应用数据目录。正式安装、用户原开发进程与业务数据库未操作。

使用项目已有Playwright Core连接隔离WebView2，实跑Tauri IPC、SQLite和统一HTTP Provider。为避免登录/付费前置，隔离库只预置一条暂停的duration起点及虚构账号快照，调用原control_automation继续；这是执行器验证，不代表真实账号首次启动验收。NotebookLM请求由测试路由拦截为静态说明页，未执行真实来源上传、音频生成或下载。

实际命令：

```text
node output/playwright/automation-p1-20261108/native.mjs
node output/playwright/automation-p1-20261108/verify-existing.mjs
```

首个脚本完成1次时长推荐、1次完整创作及音频任务创建后，因Python默认GBK输出导致中文断言失败；产品数据库中的文本正确。续验脚本改为JSON ASCII输出、使用真实model_calls.stage字段，沿用同一成果，不重新生成，验证通过。

已确认：

1. 时长模型收到启动时资料及1—15分钟边界，返回13分钟并持久保存。
2. 完整创作模型实际收到780秒，生成三比例提示词和完整音频指导，保存唯一文案版本。
3. 音频任务的版本ID、完整指导、780秒和原书SHA-256与本次结果一致。
4. 内容页精确打开本次版本；恢复手工草稿时保留手写标题，并展示准确历史对照。
5. 刷新没有增加模型请求，model_calls中duration/writing各1条；原书摘要未变。
6. 1280、1120、800宽度无横向溢出，页面异常为0；已检查实际截图。

已执行的原生结果与截图为`native-result.json`、`native-creation.png`和`native-draft.png`。脚本首次CDP连接早于端口就绪，后续核对隔离进程并连接，没有重启或终止用户程序。

最终前端小调整后的独立二进制已重新构建成功。最后启动该隔离程序并执行`reopen.mjs`的命令被执行策略拒绝（`blocked by policy`，未提供更具体原因），命令未执行，没有`reopen-result.json`或最终重开截图，也没有改用其他入口重试。数据库重开/备份恢复由自动化测试覆盖；最终二进制的桌面重启复验仍未完成。上一轮隔离进程已按精确PID及路径核对后关闭，正式用户进程未操作。

## 5. 未覆盖范围与下一步

本机模型契约与音频任务创建通过，不等于真实NotebookLM整书生成、实际时长达标、外部模型质量或抖音投稿成功。本轮未做真实远端付费调用、音频产物听审、长视频制作、正式平台上传/最终发布、新安装包升级以及macOS/Linux实机验收。

P1本机验收完成后，下一步推进P2：同一视频任务的字幕检查点、基于实际音频校核已有材料、三尺寸AI背景与导出；继续保持精确版本绑定和局部失败恢复。
