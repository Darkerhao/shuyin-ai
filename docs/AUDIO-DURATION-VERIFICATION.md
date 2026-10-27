# 音频目标时长验收（2026-10-27）

本轮交付：生成音频提示词前可采用AI基于书籍资料/体量的推荐，也可自定分钟；同一目标贯穿全量、音频按项调整、页面摘要、总时长、预计口播字数、分段安排、开场与结尾。属于feat/minor，未手动升版本、提交或发布。

## 实现和修改文件

- UI与数据边界：`src/modules/writing/WritingPage.vue`、`src/services/ai.ts`、`src/types/ai.ts`、`src/types/project.ts`。新增分钟输入、快捷按钮、首次/再次推荐、原因说明、失败与取消反馈；保存使用单独字段命令，避免回写整份项目。
- 退出保存：`src/composables/useEditorDraft.ts` 支持异步退出准备，目标时长先保存，再暂存文案；保存失败阻止关闭。原封面调用继续使用同步检查。
- 相关文字：`src/types/cover.ts` 的新封面默认副标题改为「读懂一本书」；`src/modules/publishing/PublishingPage.vue` 的话题占位去掉固定十分钟。已有封面与历史保持原值。
- AI与持久化：`src-tauri/src/ai/audio.rs`、`ai/publishing.rs`、`ai/revision.rs`、`ai.rs`、`ai_commands.rs`、`database/ai.rs`、`database/projects.rs`。复用统一文本Provider、系统凭据、超时、取消、任务与版本流程。
- IPC登记：`src-tauri/build.rs`、`src-tauri/src/lib.rs`、`src-tauri/permissions/ai.toml`，仅新增受限的 `set_audio_duration`。
- 模板：`prompts/audio-duration-v1.md`、`audio-template-v2.md`、`publishing-v4.md`、`publishing-revision-v3.md`。既有版本保留为历史；当前入口只使用新版本。
- 迁移：`src-tauri/migrations/0012_audio_duration.sql`、`src-tauri/src/database.rs`。
- 回归：`tests/ai.test.ts`、`tests/drafts.test.ts`、Rust AI模块内测试、`database/m2_tests.rs`、`m3_tests.rs`、`m4_tests.rs`。旧v5夹具改用历史schema直接写入，避免用当前项目API构造旧库。
- 文档：PRD、TECHNICAL-DESIGN、DATABASE、AI-RULES、DEVELOPMENT-PLAN、USER-GUIDE与本记录。

## 数据库边界

schema升级至v12，仅新增可空 `book_projects.audio_duration_source`（ai/manual），复用 `target_duration_seconds`；NULL表示尚未设置。迁移不改旧目标秒数、书籍资料、更新时间或历史提示词。推荐成功时目标/source与任务成功状态事务提交；无新增文案版本。生成任务创建时冻结时长，不随项目后续修改漂移。

验证使用独立临时数据库，未操作用户正式库。数据库备份支持v2—v12，测试覆盖手动时长及来源随备份/重启恢复、旧备份在内存升级、v11数据保留和迁移冲突回滚。回退旧程序需使用升级前备份。

## 实际执行命令与结果

- `npx vitest run`：最终12个文件、165项通过。包括自动推荐、手动覆盖、非法输入、失败/取消保留、按项参数与其他字段保留、异步退出准备等待。
- `npx vitest run tests/ai.test.ts`：新增时长交互后36项通过。
- `npm run lint`：通过；最终测试文件格式修正后复查。
- `npm run build`：通过，包含 `vue-tsc --noEmit` 与Vite生产构建。首次沙箱运行清理dist被EPERM阻止，项目内授权重跑成功。
- `. ./.tools/env.ps1` 后 `cargo test --manifest-path src-tauri/Cargo.toml --locked`：97通过、1项系统凭据原生测试按原设置忽略。测试进程TEMP、TMP、SQLITE_TMPDIR指向项目 `output/audio-duration-temp`，不写正式业务数据库。
- `. ./.tools/env.ps1` 后 `npm run check:rust`：fmt和Clippy（all-targets、deny warnings）通过。
- `npm test`：前端当时164通过；发布脚本30通过、1个macOS专属检查跳过；队列测试18通过、1失败，整体失败。之后新增退出回归，独立完整Vitest为165通过。
- `npm run test:queue`：单独重跑仍18通过、1失败；失败为 `a missing predecessor report reruns that task and its completed successors`（`tests/codex-queue.test.mjs:118`），期待缺失报告时重跑one/two，实际未重跑。`scripts/codex-queue.mjs` 与该测试均无本轮修改，不扩大本次业务范围修复；不将完整npm test写为通过。
- `git diff --check`：通过。

## 浏览器验证

Playwright使用本机Vite与隔离的内存IPC，模拟书籍《围城》及模型响应；不连接真实模型、不写正式数据库。验证首次推荐20分钟并保留旧提示词，手动改为35分钟后先调用 `set_audio_duration(2100)`，再以 `targetDurationSeconds:2100` 生成，音频页显示35分钟。

1440、1120、800、360px四种宽度均无横向溢出；截图在本机 `output/playwright/audio-duration-1440.png`。初始测试夹具缺少Tauri事件注销模拟，热更新产生测试环境错误；补齐事件mock并重新加载后，控制台错误/警告均为0。浏览器结果只证明界面与IPC参数，真实模板与数据库逻辑由Rust测试覆盖。

## 验收对应与下一步

- 推荐依据真实输入、资料不足标注估计：版本化Prompt及界面说明已实现，真实外部模型效果待验。
- 推荐后可任意修改及重新推荐：前端回归与浏览器交互通过；整数范围1—1440分钟。
- 总时长、字数、开场、五段安排与结尾同步：Rust覆盖1/5/15/30/60/1440分钟，五段加开场结尾之和精确等于目标；30分钟按项调整不改其他字段。
- 历史保留、失败/取消不覆盖：前后端回归、迁移及事务测试通过。
- 保存、重启和备份闭环：Rust临时库及退出准备回归通过，未执行真实Windows人工关闭/重启或安装包验收。
- 下一步：在桌面开发版选一本短书和一本长书，用实际模型核对推荐依据、修改后生成内容与外部工具实际成品时长；独立处理上述队列测试失败。

目标时长和每分钟220字均为给外部音频工具的指导，应用不生成音频，不能保证外部成品精确等时长。用户仍需在外部工具选择原书来源并核对成品。
