# 单镜重新生成验收（2026-11-03）

## 实现与影响范围

每张分镜卡增加“重新生成这一镜”，提供本次文本模型、修改要求、当前/候选对照、采用、放弃和停止。对照展示标题、构图、环境、景别、物体、动作、标签及触发字幕；这是候选内容预览，实际画面仍通过原声试片验证。弹窗正文滚动，底部操作按钮始终可见。

生成使用当前编辑快照，包括其他镜头尚未正式保存的修改。模型只收到目标镜、邻镜及附近字幕，不收到完整音频、图片或图片路径/来源备注。生成结果限定一镜，first/last不变；采用只赋值scenes[index]，原字幕和其他镜头保持。图片镜头保留原image及裁切/来源，不允许模型换图或叠加手绘物体。

复用TextProvider、流式接收、300秒上限、ActiveTasks和cancel_generation。任务状态复用ai_tasks；准备/结束核对run_id和正式计划摘要，取消/过期结果不能进入候选。候选生成不写video_jobs，明确采用后进入原自动草稿，正式保存及试片继续复用。关闭未采用候选即放弃，修改要求/模型后旧候选失效。

无新依赖、数据库表/列/迁移或新渲染路线，schema16。新增主窗口start_video_scene_revision、execute_video_scene_revision两条命令；AI用途video_scene_revision与Prompt版本video-scene-revision-v1。影响为feat/minor，本轮未手动改版本、提交或发布。

修改文件：

- `src/modules/video/VideoSceneRevision.vue`、`VideoSceneEditor.vue`、`VideoPage.vue`。
- `src/services/video.ts`、`src/types/video.ts`。
- `src-tauri/src/video/revision.rs`、`video.rs`、`database/video/revision.rs`、`database/video.rs`、`video_commands.rs`、`lib.rs`、`permissions/video.toml`。
- `prompts/video-scene-revision-v1.md`。
- `tests/video-scene-revision.test.ts`、`tests/video.test.ts`及两个Rust revision模块内测试。
- PRD、技术设计、数据库、AI规则、使用手册、开发计划及本记录。

## 实际验证命令

Rust命令先执行`. ./.tools/env.ps1`。未修改测试超时、断言或检查规则。

| 命令 | 结果 |
| --- | --- |
| `npx vitest run tests/video-scene-revision.test.ts` | 4项通过：请求/模型选择、预览与采用、放弃、失败重试、取消后迟到结果保护。 |
| `npx vitest run` | 首次328项通过，原更新页动态加载测试超过既定5秒上限；没有改动该测试。 |
| `npx vitest run --maxWorkers=2` | 降低并行后全量329项通过，23个文件；保留原5秒测试上限。 |
| `npm run lint` | 通过。 |
| `npm run typecheck` | 通过。 |
| `npm run build`（Tauri构建的前置命令） | 类型检查与生产前端构建通过；主包约622kB，仍有既有体积提示。 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked video` | 51项通过，1项原进程树专项忽略。 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked` | 194项通过，2项原有原生凭据/进程树专项忽略。 |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets -- -D warnings` | 通过。 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | 通过。 |
| `npm run tauri -- build --debug --no-bundle --config output/playwright/video-scene/tauri.verify.conf.json` | 两次独立标识调试EXE构建成功，第二次包含固定操作区调整；不是NSIS安装包。 |
| `node output/playwright/video-scene/native.mjs initial` | 真实桌面候选、放弃、无效响应、取消、采用及自动草稿检查通过。 |
| `node output/playwright/video-scene/native.mjs restart` | 强退重开恢复、正式保存、四种宽度、原声试片生成与播放通过。 |
| `git diff --check` | 通过。 |

首次实现检查发现并修正了Rust异步闭包所有权、摘要类型格式化及测试模块位置问题。前端测试改用项目既有真实浮层测试辅助工具，避免将Reka内部Teleport一并替换为空stub；过期快照测试通过组件更新事件模拟外部变化，不尝试向已禁用的控件输入。以上不是通过弱化校验规避失败。

## 验收标准与证据

| 标准 | 已执行证据 |
| --- | --- |
| 只修改选中的一镜 | 前端采用第二镜后，第一镜未保存修改保留，第三镜和SRT完全相同；Rust拒绝额外镜头、改范围、非法物体、错误首锚点与额外字段。 |
| 预览后才采用 | 原生界面显示当前/候选并排；生成完成和放弃都未改变正式计划，未采用时原镜头仍在。 |
| 支持模型与要求调整 | 单次请求使用选中的模型、目标字幕及相邻编辑上下文；修改要求/模型后候选失效。 |
| 失败及停止保稿 | 原生契约服务返回两镜被拒绝；延迟请求通过原取消命令终止，原稿保留；前端单测还覆盖取消之后成功响应迟到。 |
| 防止过期结果覆盖 | 后端测试覆盖正式计划变化、新run_id及取消状态，前端覆盖编辑快照变化。 |
| 图片镜头保留 | Rust回归验证image、裁切及来源原样保留，不发送素材ID/路径/来源备注，也不允许模型新增物体。未另跑图片镜头的原生UI生成。 |
| 暂存与正式保存闭环 | 原生采用仅写自动草稿；强制终止精确测试进程后重开，目标镜与其他未保存修改恢复。正式保存后逐镜比较，只有原人工修改和采用镜变化，全部cue保持。 |
| 实际渲染仍可用 | 使用正式保存的新计划运行原试片入口，生成1920×1080、20秒原声试片并在WebView2播放；请求上限30秒，源音频约20秒，按原逻辑取剩余长度。 |
| 界面适配 | 最终原生1440/1120/800/390px下弹窗及正文无内部横向溢出，采用按钮完全位于弹窗内；截图已查看。 |

原生验证使用独立identifier `com.shuyin.scene-verify`、本机HTTP契约模型（127.0.0.1:18543）与复制的20秒音频。通过Playwright CDP连接真实WebView2、使用真实Tauri IPC和SQLite，无mockIPC、外部付费模型或正式数据库操作。

启动/结束使用`output/playwright/video-scene/app.ps1 Start/Force/Close`，每次核验精确EXE/PID；强退仅作用于测试进程。D盘output保存构建、工作区、音频副本与截图；SQLite按现有Tauri路径规则放在C盘独立应用目录。初次截图中采用按钮位于滚动区下方，已改为固定操作区并在重建后的原生窗口复查。

证据文件：`output/playwright/video-scene/native-initial.json`、`native-restart.json`、`context.json`、`candidate-{1440,1120,800,390}.png`。本次5次模型请求均为本机契约请求，其中一项非法输出和一项主动取消；未据此判断真实外部模型的画面质量、耗时或费用。

## 未覆盖与下一步

未执行NSIS安装升级、外部真实模型质量验收、长片全量渲染或macOS/Linux实机验证；本轮验证的是隔离Windows调试构建。未采用候选不做重启恢复，已采用内容按现有自动草稿恢复；强制退出前未完成的最后一次写入仍不能保证恢复。

下一步在包含本次功能的新构建中，用实际文本模型挑选一个不满意的镜头，核对候选后采用并试听试片，确认真实创作效果。其余全局续作、缓存管理和模型诊断仍作为独立后续切片。
