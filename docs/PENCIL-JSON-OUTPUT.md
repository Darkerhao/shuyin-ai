# 精绘画稿 JSON 报错修复

日期：2026-11-18。影响类型为 `fix` / patch。修复模型返回不完整画稿 JSON 的生成入口，不改变已有任务的数据结构。

## 取证与根因

正式程序为 `D:/书引 AI/shuyin-ai.exe`，版本 0.69.2。通过该进程已打开的数据库文件句柄读取一致性快照，在隔离副本中检查，未写入正式书库。

截图显示已保存 10 镜。取证时，任务 `ddb60a65e189cb3ce05b747ed99620a2` 已由用户续作到 run13，保留 369 条字幕、730.13 秒原声和 11 镜；第 12 镜“面子盖过理智”（字幕 115—126）再次失败。该次模型调用完整结束，耗时 109477ms、输出 4306 token，随后发生画稿解析错误。

历史模型诊断不保存响应正文，不能还原截图那次输出中的准确坏字符。本轮使用当前选中的 91Token / gpt-6-astra，按真实 `drawing_input` 和旧 v2 Prompt 重放当前失败镜，复现相同的“精绘画稿不是合法JSON”。完整响应 15099 字节，15 层画稿之后追加了：

```text
  ],
  "z_end_turn_review": "
```

模型服务已发出正常结束信号，但额外字段的字符串和根对象尚未闭合。生产 `parse_artwork` 稳定拒绝该回复；serde 给出 `unknown field z_end_turn_review`，纯 JSON 解析同时报告字符串意外结束。原实现把字段和语法错误都丢弃成同一条笼统提示。

生成请求此前只在 Prompt 中要求 JSON，没有向服务传递结构约束。这是本轮修复的入口：请求严格结构化输出，保留宿主校验及原保存检查点。

## 实现与范围

- `src-tauri/src/ai.rs`：`TextRequest` 增加可选 `response_format`，统一 Provider 仅在调用方明确提供时发送。普通文本、模型连接测试和原有其他请求仍省略该参数。
- `src-tauri/src/ai_commands.rs`：公共请求初值为 `None`，不增加用户设置或模型配置字段。
- `src-tauri/src/video/pencil.rs`：逐镜生成使用 v3 Prompt 和同一份 `json_schema` / `strict=true` 响应格式；解析失败保留 serde 字段及行列详情。
- `src-tauri/src/video_commands.rs`：精绘单镜修订使用同一份响应格式。
- `prompts/video-pencil-draw-v3.md` 与 `video-pencil-draw-v3.schema.json`：替代 v2 活动绘图 Prompt。根对象只允许 `design/layers`，图层及笔画、阴影、文字、头像均声明必需字段并禁止额外字段。无头像的图层返回 `portrait:null`，沿用已有 `Option<Portrait>` 解析，保存时仍省略空头像。
- `src-tauri/src/ai/tests.rs` 与 `src-tauri/src/video/pencil/tests.rs`：回归覆盖真实 HTTP 请求中的 Schema、初次和恢复画稿的约束一致性、字段及语法错误详情、非法 JSON 不保存及有效画稿续作。

保留原字幕覆盖、人物唯一引用、有限坐标、区域闭合及展开资源预算校验。未增加 JSON 修补、额外重试、Provider 分支、依赖或权限。自动流程继续沿用原有限重试策略。所用模型服务须支持严格 `json_schema`；本轮已实测当前 91Token 配置。

SQLite 仍为 v25，无表、列或迁移变化。正式任务、原声、已保存画稿和冻结头像未改写。任务恢复时生成缺失画稿，既有检查点继续使用原数据。

## 真实重放与保存验收

同一模型、同一镜头改用 v3 Prompt 和严格响应格式后，完整返回 21066 字节；语法、Schema、生产 `parse_artwork` 和四个人物的完整唯一引用均通过。

| 重放 | 总耗时 | 服务返回输入 token | 服务返回输出 token | 结果 |
| --- | ---: | ---: | ---: | --- |
| 原 v2 请求 | 98140ms | 7832 | 6711 | 额外审阅字段及未闭合 JSON，拒绝保存 |
| v3 严格结构请求 | 147218ms | 25248 | 9575 | 完整画稿，结构与业务校验通过 |

用正式数据库快照进行真实 `claim_video_job → pencil::generate → save_video_progress → end_video_job` 验证：采用捕获的有效第 12 镜后，下一镜用可控断线结束；重开隔离数据库，保留 12 镜。逐字比较确认原 11 镜、全部字幕、冻结头像与原剩余待办正确保留，没有重新规划。

同一份保存结果通过 JS 严格绘制编译，展开为 21 层、643 笔、4 个头像；包含原 11 镜的连续片段经 `buildEpisode` 编译为 12 页。此为画稿与引擎编译验收，不代表整段长片的实际画质或播放验收。

诊断证据位于忽略目录 `output/pencil-json-20261118/`，包括数据库快照、两份原始画稿、请求、服务用量、Schema 校验及绘制编译结果。密钥仅由系统凭据库读取后在内存中传给原 Provider，未输出或写入证据。临时模型重放和快照测试的 `include!` 已移除，常规测试不会调用这些服务。

## 实际工程验证

Rust 命令先加载 `. ./.tools/env.ps1`。真实模型重放移除该工具链脚本注入的进程代理，使请求环境与正式应用一致。

- 三个新增回归在修复前各失败，修复后各通过。
- `npm run lint`：通过。
- `npm run typecheck`：通过。
- `npm run check:rust`：fmt 与所有目标 Clippy 通过，警告按错误处理。
- `cargo test --manifest-path src-tauri/Cargo.toml --locked --lib -- --quiet`：351 通过、3 项既有忽略、0 失败。
- `npx vitest run tests/video.test.ts tests/video-storyboard.test.ts tests/video-progress.test.ts tests/video-scene-revision.test.ts`：4 个文件、63 项通过。
- `node --test tests/pencil-drawing.test.mjs`：6 项通过。
- `python output/pencil-json-20261118/verify-schema.py`：Schema 自身、实际响应及拒绝额外字段通过；使用本机已有 jsonschema 4.26.0，不引入应用依赖。
- `node output/pencil-json-20261118/verify-renderer.mjs`：12 页片段编译及旧字幕、画稿、头像保留通过。
- 临时 `pencil_json_remote_replay`：同模型真实 v3 请求通过。
- 临时 `pencil_json_structured_captured_replay`、`pencil_json_real_task_resumes_in_an_isolated_database`、`pencil_json_captured_replay`：各通过；原非法回复继续被拒绝。

构建前执行 `git pull --ff-only`，只快进上游 v0.69.2 的五个版本文件，功能代码相同；原有文档工作与本轮修复保留，没有手动升版、提交或推送。

## Windows 本地修复包

实际构建命令：

```powershell
. ./.tools/env.ps1
npm run tauri -- build --bundles nsis --no-sign --ci --config D:/MyWork/project/JIAZI-PROJECT/shuyin-ai/output/pencil-json-20261118/tauri.local.conf.json
```

前端 TypeScript / Vite 生产构建、Rust release 构建及 NSIS 封装均通过。仅本地构建配置关闭更新包签名产物，未修改正式应用配置；沿用上游 0.69.2 版本号，无覆盖安装或远程发布。构建保留已有 npm 本机配置和 Vite 大分块提示。

交付 `output/pencil-json-20261118/书引AI-分镜JSON修复-0.69.2-setup.exe`，8921635 字节。SHA-256：`9a09af2db488c614f005f2518e43e3da26fa245821c38245c1b6091a89324295`。

`python output/pencil-json-20261118/verify-build.py` 通过：核对 EXE 为 0.69.2，包含严格 `pencil_artwork` Schema、v3 Prompt 及详细解析错误。记录与哈希位于同目录 `build-verification.json` 和 `.exe.sha256` 文件。

最后只读复查正式任务仍为 run13 / failed，保留 369 条字幕和 11 镜，第 12 镜仍待新构建续作。本地重放结果未写回正式任务。先退出书引 AI，再安装本地修复包，重新打开原任务并点击“继续准备字幕与分镜”。

## 运行边界

本轮验证了当前失败的第 12 镜和隔离数据库恢复，未在正式任务上生成后续镜头，也未进行整段长片、正式覆盖安装或新安装程序的桌面人工验收。使用包含修复的构建后，在原任务点击“继续准备字幕与分镜”，复用当前保存成果继续。

接口依据：[OpenAI 当前 Structured Outputs 文档](https://developers.openai.com/api/docs/guides/structured-outputs)。文档要求严格 Schema 的对象声明必需字段并设置 `additionalProperties:false`；可空类型表达无头像，宿主继续处理拒绝、未完整响应及 Schema 未表达的业务规则。serde 结论基于本机锁定版本 serde_json 1.0.154 的 `from_str` 与错误分类实现。
