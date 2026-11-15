# 整书与章节的本期介绍（2026-11-15）

## 根因与确认方案

《三国演义》第五回任务的v5和冻结提示词均含完整回目，下载与归档摘要一致；已有原始转写在约16秒提到第五回，但省略回目并改写引入。旧音频模板仅将引子、欢迎语、收束和告别列为定稿，遗漏本期介绍。

统一由程序根据本次书名和可选章节标题生成报题首句，AI的 `introduction` 只写一两句本期看点。整书为“这一期，我们一起读《书名》。”，章节为“这一期，我们一起读《书名》中的‘完整章节标题’。”。章节标题作为整体使用，兼容章、回、节、篇、序言、无编号和外文标题，不猜编号、拆分目录或补造名称。

完整本期介绍列为定稿口播，主讲人在欢迎语后连续朗读。全量、音频按项、生成中预览与最终提示词共用组装；已保存指导、旧音频和执行中任务保留原冻结输入。利用已有内容校核的 `warnings` 提醒开场介绍疑似遗漏，不按提示词补字幕、不因提醒阻断制作或自动收费重生成。

## 验收标准与测试入口

- A1：`PublishingDraft::into_content` 的整书介绍包含本次真实书名，章节包含本次书名及完整任意格式标题，模型仅提供看点时仍完整报题。
- A2：`revision::revise` 的音频调整使用同一组装，其他字段调整原样保留音频；章节改名后的历史修订仍使用原来源快照。
- A3：生成中 `preview::texts` 与合法成稿使用同一音频渲染，完整响应的音频预览与保存内容完全一致；部分响应仍标为生成中且不可采用。
- A4：本期介绍作为定稿写入新模板和字幕首尾参考；章节名不能被省略、重编号或用片段代替，字幕仍只校对实际说出的内容。
- A5：已有音频内容校核对整书/章节的开场介绍疑似遗漏返回非阻断提醒；ASR同音、标点和数字读法差异不能直接判定漏读。
- A6：旧提示词、旧任务和新任务来源冻结保持，结构、数据库、依赖和自动化状态无需变更。

## 实现与影响

- `publishing-v11` / `publishing-revision-v10` 将 `introduction` 收窄为本期看点。`AudioDraft::into_prompt` 保留完整结构验证，仅渲染部分提为 `render` 与流式预览共用；最终业务结构仍只有原 `audioPrompt` 字符串。
- `audio-template-v7` 将本期介绍标题改为“开场介绍（逐字朗读）”，利用已有字幕提取器“开场”前缀读取。程序保留书名和完整章节标题，同一主讲人连续读完开场三段；定稿段数从四段修正为五段。
- 预览入口接收与最终组装相同的 `PreparedGeneration.book_title/chapter_title`，完整响应的音频预览与保存值一致。未完整结果仍使用页面原有“尚未校验或保存”提示，不能采用。
- `content-check-v4` 在原后置校核增加开场介绍检查，以冻结实际指导为准，考虑标点、数字读法和ASR同音差异；返回原 `warnings`，不增加确定性的字符串硬匹配或新的模型调用。
- 无依赖、schema、权限、自动状态、用户设置或发布版本号变更。旧模板仅保留历史追溯，新运行入口不再引用旧版本。本次业务影响为 `fix/patch`。

修改文件：`src-tauri/src/ai.rs`、`ai/{publishing,preview,revision,content_check,tests}.rs`、`ai_commands.rs`、`database/{chapter_tests,automation_tests}.rs`；新增 `prompts/{audio-template-v7,publishing-v11,publishing-revision-v10,content-check-v4}.md`；同步 `docs/{PRD,TECHNICAL-DESIGN,DATABASE,AI-RULES,DEVELOPMENT-PLAN}.md` 与本文。最终差异由主线程复核，未进行独立代理评审。

## 验证记录

Rust命令先运行 `. ./.tools/env.ps1`。

1. `cargo test --manifest-path src-tauri/Cargo.toml --locked --lib audio_introduction_announces -- --nocapture`：修复前1失败，确认旧模板没有定稿介绍与程序固定报题；修复后在发布生成12项回归中通过。
2. `cargo test --manifest-path src-tauri/Cargo.toml --locked --lib audio_previews_match_saved -- --nocapture`：修复前1失败，旧预览只包含模型片段、没有欢迎语与程序报题；改用共享渲染后通过。

| 实际命令 | 结果 |
| --- | --- |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked --lib ai:: -- --nocapture` | 最终45通过、1项原有忽略 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked --lib --quiet` | 318通过、2项原有忽略 |
| `npx vitest run tests/ai.test.ts tests/video.test.ts tests/chapters.test.ts tests/generation.test.ts tests/automation.test.ts tests/automation-tasks.test.ts` | 6个文件、165项通过 |
| `npm run lint` | 通过 |
| `npm run build` | TypeScript检查与Vite生产构建通过 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | 通过 |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets -- -D warnings` | 通过 |
| `git diff --check` | 通过 |

构建仍有既有约870kB主分块提示，npm仍提示本机旧 `python/msvs_version` 配置。

A1覆盖整书、完整三国回目、普通编号章节、序言、无编号篇名、层级与外文标题；A2/A3覆盖全量/音频按项与完整/部分预览，数据库测试从生成保存、来源改名、音频修订采用直到新音频任务，确认使用原章节快照、历史内容不变且音频冻结等于采用提示词。A4通过原字幕解析器提取完整开场，原校对Prompt明确禁止按参考补台词。A5以整书漏书名/章节漏名称的模拟模型回复核对原提醒可继续后续阶段并跨重开保留；本地测试不能证明真实模型一定识别遗漏。A6由全量Rust旧数据/迁移/冻结回归和本次差异核对。

未执行完整 `npm test`、真实付费文本/NotebookLM生成、人工试听或安装包构建；未替换 `D:\书引 AI\shuyin-ai.exe`、操作正式书库、提交或推送。实际音轨是否遵从新提示词需生成后试听，不将本地模型夹具视为实站成功。

## 生效方式

运行包含本次修改的新桌面构建，对目标整书或章节选择“按项调整 → 音频生成提示词 → 生成预览 → 采用并保存新版本”，再新建音频任务。新书或新章节直接生成方案即使用新模板。已经归档的音频不会因指导变化被改写。
