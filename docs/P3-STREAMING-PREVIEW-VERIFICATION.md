# P3 流式候选预览实施与验收

日期：2026-11-04。

## 结果与范围

P3已实现：内容创作全量生成与按项调整，在同一次模型请求中逐步显示已收到的正文；预览与当前稿分离。完整结束、结构/业务校验及原有保存成功后，才交付正式版本或可采用的按项候选。停止、断流、非法输出清除临时预览，保留当前稿、草稿与历史。

覆盖发布标题/内容、音频指导及五项封面文字。音频指导预览只展示模型生成的引入、部分标题与指导正文；固定开场、结尾和制作参数仍在完整校验后由原模板生成。配色对象不作为原始JSON预览，时长推荐、模型连接测试及视频分镜不新增逐字界面。

## 实现与取舍

- 统一Provider保留原`TextProvider.generate`接口，按需挂接当前请求的正文观察回调；SSE只接收`delta.content`，忽略推理片段。约80ms合并一次预览，完整结束时补最后一次；继续使用原300秒请求总上限、结束校验、取消与无自动重试规则。兼容服务若一次返回完整JSON，继续完成后交付，不伪造逐字动画。
- `ai/preview.rs`只有一套轻量只读字符串提取，处理未闭合字符串、引号/换行转义、Unicode及代理对，按业务字段投影。按项只投影选中字段，音频嵌套指导转为可读文字；不补写或修复正式JSON，不替代原结构验证。
- `execute_generation`接收可选`JavaScriptChannelId`，按调用方WebView创建本任务专属Channel；只发送解码后的文字字段，不广播全局事件。现有不订阅预览的调用继续复用同一执行入口。
- 创作页复用原字段名称与主题，显示有高度上限的只读候选区，明确“尚未校验或保存”。正文以普通文本渲染，无HTML执行；未按片段反复播报屏幕阅读器，也不自动滚动或抢焦点。
- 请求结束清空回调；取消成功和页面卸载解除当前预览接收，旧任务的迟到片段无法更新新预览。Channel随Rust请求结束释放回调，不新增全局监听或第二套AI任务状态。当前稿仍由原自动草稿管理。

已核对已安装Tauri 2.12.5的`ipc/channel.rs`及前端API 2.12.4的`core.js/core.d.ts`，使用公开`JavaScriptChannelId.channel_on`、`Channel.onmessage`和结束释放机制。参考入口：[Tauri Channels](https://v2.tauri.app/develop/calling-frontend/#channels)。无新增依赖或能力权限。

## 实际验证

| 命令 | 结果 |
| --- | --- |
| `npx vitest run --maxWorkers=2 --reporter=json --outputFile=output/playwright/p3-preview/frontend-all.json` | 367项全部通过；含完成前可见、当前稿保留、停止/断流、迟到片段隔离、卸载和纯文本渲染场景 |
| `npm run typecheck` | 通过；最终桌面构建又执行同一检查 |
| `npm run lint` | 通过，未关闭规则 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked ai::` | 33通过，1项既有系统专项忽略；覆盖正文提前到达、推理隔离、Unicode与选中字段，并保留原断流/超时/心跳回归 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked` | 最终210通过，2项既有专项忽略，0失败 |
| `npm run check:rust` | fmt及Clippy全目标通过，`-D warnings`保持 |
| `npm run desktop:build -- --no-bundle --config output/playwright/p3-preview/tauri.verify.conf.json` | Release应用构建通过，包含TypeScript、Vite和Rust；不是安装包或发布产物 |
| `node output/playwright/p3-preview/native.mjs` | Windows实际WebView2、IPC、Provider SSE与SQLite的6组流程通过，页面异常0 |
| `git diff --check` | 通过 |

Rust命令先加载`. ./.tools/env.ps1`，临时目录指向本轮D盘输出。前端测试只限制命令并发，未放宽超时或删除断言。构建保留既有500kB块大小提示与npm用户配置提示；本轮不重跑未受影响的视频引擎及发布脚本测试。

首个前端回归在实现前以“找不到生成中的候选预览”失败，实现后通过。开发中修正测试的按钮名称、Rust测试字符串界定及Vue换行规则。可选`Channel<T>`不能直接作为Tauri的Option参数，编译明确拒绝后按当前公开API改为`Option<JavaScriptChannelId>`；最终编译、Clippy及真实IPC均通过，没有为此增加自定义反序列化或关闭检查。

## Windows实际流程

使用独立标识`com.shuyin.p3-preview-20261104`、新建schema17测试库及D盘工作目录，关闭自动更新。应用主体、测试脚本和WebView缓存均位于`output/playwright/p3-preview/`，Tauri独立SQLite按默认规则位于C盘LocalAppData。未打开或改写正式`com.shuyin.desktop`库、真实凭据或用户作品，未替换已安装版本。

本机HTTP服务通过真实统一Provider返回分段SSE：先发推理片段与未闭合正文，再由测试显式释放剩余正文或断开连接。没有模拟Tauri IPC、版本保存或业务校验。

1. **取消**：正文已出现，但历史仍只有原版本、自动草稿为空；取消后任务记为cancelled，临时预览消失，原稿不变。
2. **断流**：收到部分正文后EOF且无完整结束；任务记为failed/incomplete，没有新增版本。
3. **结构错误**：SSE正常结束但缺少必要业务字段；任务记为failed/invalid_content，没有新增版本。
4. **全量成功**：完整响应前历史不变，正文已可见；此时继续修改当前稿，仅当前修改进入原自动草稿，没有候选片段。完整校验后追加一个备选，当前稿及生成过程中输入的修改保留。
5. **按项调整**：只显示选中标题；完整结果先成为原有候选，明确“采用并保存新版本”后才追加历史。
6. **音频指导**：嵌套引入/部分指导显示为正文，没有JSON键或推理内容；完整结果放弃后不改变正式内容。

首次及最终构建分别复核六组流程，每轮恰好6次HTTP请求，没有重试。本书最终4份文案版本分别来自初始稿、完整备选、按项调整前原有规则保存当前修改、明确采用的标题候选；不包含部分流式输出。四档CSS宽度1366/1120/800/360无整页横向溢出；查看了1366和360截图。此处是WebView中的CSS窗口检查，不冒充新的Windows系统缩放验收。

证据保存在忽略目录：`frontend-all.json`、`rust-ai-final.log`、`rust-all-final.log`、`rust-check-final.log`、`native-build-final.log`、`native-verification.json`及`live-1366/1120/800/360.png`。`integrity-final.json`记录独立程序退出后的schema17、integrity=ok、外键问题0；本书4份历史、恢复草稿0、失败/取消任务候选0、运行任务0。

## 修改文件、数据库与版本

| 文件 | 职责 |
| --- | --- |
| `src-tauri/src/ai.rs`、`ai/stream.rs` | 同一Provider的可选正文回调、预览节奏与最终补发 |
| `src-tauri/src/ai/preview.rs`、`ai/revision.rs` | 唯一只读文字投影、复用现有字段键；含解析回归 |
| `src-tauri/src/ai_commands.rs` | 当前调用方的任务专属Channel接入，原执行/取消/保存路径保留 |
| `src/services/ai.ts`、`src/types/ai.ts` | 类型化预览、Channel与请求结束清理 |
| `src/modules/writing/WritingPage.vue` | 临时候选区、回调解除、迟到片段隔离 |
| `tests/ai.test.ts`、`src-tauri/src/ai/tests.rs` | 当前稿、失败、取消、纯文本与真实SSE行为回归 |
| 产品、技术、AI规则、数据库、使用手册、开发计划及原方案/验收文档 | 同步能力与验收边界 |

无SQL迁移、新表/字段、偏好配置、权限、Prompt或模型协议变化。部分预览只在当前界面保留，不写`editor_drafts`、`writing_versions`或`ai_tasks.preview_json`；完整按项候选继续沿用原任务输出，采用才追加历史。schema仍为17。

这是新增兼容用户能力，应按feat/minor判断发布影响；版本保持当前0.46.0，不手动递增、暂存、提交、推送或发布。

## 未覆盖范围与下一步

未调用真实外部模型或付费服务，不能由本机协议结果保证供应商分段速度与内容质量；需要供应商实际返回正文片段才会逐步出现。未更新正式安装版，也未执行macOS/Linux实机或长时间人工创作验收。此前Windows跨窗口文件拖入手势仍保持待人工验证，与本轮P3完成分开记录。

下一步在包含本次改动的构建中使用已配置模型完成一次实际创作，核对候选可读性与等待体验；发布仍沿用原人工核对流程。
