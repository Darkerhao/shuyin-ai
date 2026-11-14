# 手动生成音频后的原任务衔接

日期：2026-11-14。基线：`8d547c6`。影响：fix / patch，不手动升版本。

## 问题与根因

用户在 NotebookLM 手动完成音频后，应用任务仍停在“等待来源处理”，本地音频成品为 0。

`audio_page.js::executeStep` 原先只按本地阶段执行：`source_processing` 只检查来源，`configuring_audio` 直接填写配置，`submitting` 直接点击生成。因此用户在网页手动推进后，本地准备阶段不会识别已有音频，还可能再次配置或提交。`database/audio.rs::observe_audio_job` 也拒绝准备阶段直接登记生成完成。

用同一真实测试笔记本只读复现：原脚本的 `detectGenerationState()` 返回 `complete`，但 `runStep(source_processing)` 返回 `source_ready`。新增数据库回归在原任务超时、继续后登记已完成音频，失败为“当前阶段不能登记音频生成完成”。

## 最小修复与验收标准

- 在等待来源、配置音频和提交之前，核对任务已记录的笔记本，并复用原音频状态识别。已有生成中音频则等待，已有成品则下载，明确失败则保留任务提示处理。
- Rust 的同一观察入口允许这三个准备阶段接续生成中/已完成状态，继续原下载、音频校验、素材事务和完成收尾。
- 未发现音频时继续原流程；不同笔记本仍拒绝。暂停/待处理须明确继续，旧运行编号的迟到结果仍拒绝，不恢复用户主动暂停的全局自动制作。
- 保留原任务、来源快照、文案版本与成品引用，不新建任务，不重新生成音频。无数据库结构、迁移、权限或依赖变更。

本卡片属于自动音频任务，完成条件是下载校验并归档为本地音频。只有从“自动制作并准备发布”启动、且全局流程处于运行中的任务，才由原自动执行器在音频成功后进入字幕阶段；单独“开始生成音频”不会自动启动整条视频制作链。

## 实际验证

| 命令 | 结果 |
| --- | --- |
| `node --test --test-name-pattern='source processing follows manually' tests/audio-page.test.mjs`（修复前） | 1 项失败：返回 `source_ready`，预期 `generation_complete` |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked manual_audio_recovers_preparation -- --nocapture`（修复前） | 1 项失败：准备阶段拒绝生成完成观察 |
| `node --test tests/audio-page.test.mjs tests/audio-package.test.mjs` | 73 项通过 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked audio -- --nocapture` | 50 项通过，251 项未被筛选选中；包括超时恢复、迟到结果拒绝及原任务归档 |
| `npx vitest run tests/automation.test.ts tests/chapters.test.ts --maxWorkers=2` | 2 文件、18 项通过 |
| `npm run lint` | 通过 |
| `npm run check:rust` | fmt、Clippy 全目标通过 |
| `npm run tauri -- build --no-bundle` | TypeScript、Vite、Windows release 程序构建通过 |
| `git diff --check` | 通过 |

Rust 命令先执行 `. ./.tools/env.ps1`。浏览器采用 ego-browser，在已有的独立公开测试笔记本只读验证，命令形式为 `ego-browser nodejs -e "await import('file:///D:/MyWork/project/JIAZI-PROJECT/shuyin-ai/output/manual-audio-recovery/round-03.mjs')"`。修复后三个阶段均返回 `generation_complete`，点击次数为 0。前后结果在忽略目录 `output/manual-audio-recovery/live-before.json` 与 `live-after.json`。

用户补充下载菜单截图后，通过同目录 `round-07.mjs` 调用实际 `captureDownloadTarget()`，成功点击当前成品的“更多 → 下载”，返回 `download_target` 和 HTTPS 地址；临时拦截网页下载动作，未执行原生文件下载。结果摘要在 `live-download-menu.json`。现有菜单下载路径无需改动，原问题发生于准备阶段尚未进入下载。

构建程序另存为 `output/manual-audio-recovery/shuyin-ai.exe`，SHA-256 为 `F479EC3DDCE9F8D44AD9218B1A76E74180DC9DDBB324325E5BD081A82356F87E`。保留现有 npm 用户配置警告与 Vite 大于 500 kB 分块提示，没有构建错误。实现验证时未构建安装器、替换安装程序、提交或推送。

## 范围与边界

修改页面执行脚本、数据库观察规则及相应 Node/Rust 回归，技术设计和开发记录同步更新。上述验收标准在页面回归、真实测试网页只读识别和隔离数据库归档中通过。未修改用户数据、替换正在运行的安装版或操作用户原章节音频；未执行原生网站下载、字幕、视频和发布完整链路。源码修复不会热更新当前进程，使用包含修复的程序后，沿用原任务继续跟进。
