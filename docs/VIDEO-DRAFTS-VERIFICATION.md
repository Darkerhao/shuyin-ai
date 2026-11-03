# 视频自动草稿与字幕对照验收（2026-11-03）

## 实现与范围

- 视频任务独立暂存未完成SRT、分镜及编辑位置。复用`useEditorDraft`的400ms防抖、串行写入、关闭监听；失败提示并支持重试，离页及任务切换等待写入。恢复最近编辑任务，显式任务链接优先。
- 正式保存前排空草稿队列，执行期间暂停暂存；正式保存/新运行在原SQL更新中清除旧草稿，迟到的旧代次写入被拒绝。读取另一个任务失败时不展示上一个任务的编辑缓冲。
- 原文与校对文逐句对照、只看修改项、编辑/恢复单句、原声时间窗试听；共用同一SRT字符串，每页最多20句。保留全文编辑和正式校验，单句操作不改编号、时间码或其他句子。
- 新任务校对前冻结原文；旧任务读取已登记、属于本任务最早运行的原始SRT，Windows规范化路径一致比较。缺失明确提示，不以已校对文本代替。试听绑定任务音频；句尾暂停、缓冲不提前截断，卸载停止播放。

本轮仅完成“视频自动草稿＋字幕对照试听”。运行环境检查、单镜生成、全局续作、缓存管理、模型诊断及CI/Node版本统一未实施。属于feat/minor；未手动改版本、提交或发布。原声、旧成片与正式用户数据库未改动。

## 修改文件

- 页面与交互：`src/modules/video/VideoPage.vue`、`VideoSubtitleEditor.vue`、`subtitles.ts`。
- 共用草稿/接口/类型：`src/composables/useEditorDraft.ts`、`src/services/{drafts,video}.ts`、`src/types/video.ts`。
- 数据与IPC：`src-tauri/src/database.rs`、`database/video.rs`、`database/video/editor.rs`、`video_commands.rs`、`lib.rs`、`permissions/video.toml`。
- 迁移：`src-tauri/migrations/0016_video_editor.sql`。
- 测试：`tests/video.test.ts`、`tests/video-subtitles.test.ts`及Rust editor模块内测试。
- 文档：PRD、技术设计、数据库、AI规则、使用手册、开发计划及本记录。

## 数据库

v15→v16仅给video_jobs增加可空`editor_draft_json`、`original_cues_json`。原字段与行不重写；迁移沿用事务，快照备份/恢复保留新列。自动草稿不修改正式plan_json、任务状态或更新时间。新原文快照只写一次；草稿最大4MiB，basedOn必须匹配run_id。

## 实际执行

Rust命令先执行`. ./.tools/env.ps1`。各项均为本地运行，未调用付费模型。

| 命令 | 结果 |
| --- | --- |
| `npx vitest run` | 最终全量320项通过（21个文件）。 |
| `npx vitest run tests/video.test.ts tests/video-subtitles.test.ts tests/drafts.test.ts` | 30项通过，覆盖新增功能与原视频流程。 |
| `npm run lint` | 通过；窄屏文字调整引入的两处缩进警告已修正并重跑。 |
| `npm run build` | TypeScript和前端生产构建通过；保留约610kB主包的既有体积提示。 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked` | 187项通过，2项原有原生凭据/进程树专项测试忽略。 |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets -- -D warnings` | 通过。 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | 通过。 |
| `git diff --check` | 通过。 |

首次新增回归暴露并修正了Windows扩展路径比较、编辑光标定位及失败提示展示问题；首次Rust编译的类型推断/测试结构字段、迁移夹具的必填列也已修正。HTML textarea的DOM value自动规范化换行，换行保留测试检查模型值及字符串替换函数，而非误把DOM的LF当作存储改写。

## 浏览器证据

使用Playwright CLI和本机Edge，入口：`output/playwright/video-editor/fixture.html`。`serve.mjs`只扫描当前夹具，避免历史output页面影响Vite依赖扫描。通过Tauri官方mockIPC运行真实Vue页面，草稿在夹具localStorage模拟持久化；SQLite持久化由上面的Rust重开/备份测试验证。音频为隔离合成WAV，不是用户真实原声。

主要命令：

```powershell
node output/playwright/video-editor/serve.mjs
npx --yes --package @playwright/cli playwright-cli -s=video-editor open http://127.0.0.1:1947/output/playwright/video-editor/fixture.html --browser msedge
npx --yes --package @playwright/cli playwright-cli -s=video-editor reload
```

随后通过CLI click/fill/run-code/screenshot实际检查：

- 60句中筛选出3句；编辑第1句后刷新，文字、onlyChanges及index恢复，未调用正式保存。
- 第1句原声时间窗0—1.8秒，实际播放器在1.843166秒暂停；播放缓冲逻辑另由单元测试覆盖。
- 单句恢复后修改项由3句变为2句，第21句仍保留修改，正式保存调用数为0，无未处理IPC。
- 1440/1120/800/390/360px：页面宽度均未超过窗口，字幕列表scrollWidth分别等于clientWidth（1086/798/478/252/222px）。先前截图发现的窄屏时间码截断已通过独立换行修正，并复查列表内部溢出。
- 截图已人工查看：`output/playwright/video-editor/desktop.png`、`mobile.png`。浏览器仅记录夹具favicon.ico的404，未发现页面脚本异常。

## 验收对应与剩余边界

| 验收标准 | 证据 |
| --- | --- |
| 校对后重开可恢复 | SQLite重开/备份恢复测试＋浏览器刷新恢复；包含未完成SRT、分镜、筛选和位置。 |
| 不同任务不串稿 | 慢写入后切换、独立存储、读取失败屏蔽旧缓冲回归通过。 |
| 快速定位修改句 | 修改项筛选、20句分页、原文与校对文对照通过。 |
| 对应原声试听及撤回 | 任务音频绑定、起止位置、缓冲/失败/卸载，以及单句恢复保留其他内容通过。 |
| 正式保存仍独立 | 自动暂存不改计划；正式保存失败保稿、成功清旧稿；原试片/渲染确认回归通过。 |

未执行新的安装包构建、真实Windows WebView2关闭/强制结束进程重启验收、真实长音频听审或macOS/Linux实机验证。强制退出前仍在防抖/写入中的最后输入不保证恢复；旧任务原始SRT缺失时不能提供原文对照，但保留校对编辑和可用原声试听。高版本数据库回退须用升级前备份。

下一步：用隔离Windows桌面构建打开一条已有长音频任务，修改后等待“草稿已自动暂存”，结束并重开应用，核对字幕、镜头、光标和实际原声试听；随后进入运行环境一键检查切片。
