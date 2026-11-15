# 音频内容疑点导致自动制作暂停修复（2026-11-15）

## 问题与根因

用户截图停在「字幕与校核」，错误包含章节外童年叙述、人物心理推断等意见，并显示「无法完成内容校核」「原稿和音频已保留」。这些是模型的内容判断，未在本轮人工核实。

当前真实调用链：

1. `prompts/content-check-v2.md` 末段要求章节实际音频无法确认、明显越界或补造其他章节时返回 `verified=false`。
2. `ai/content_check.rs::apply` 将 false 转成错误，不保存成功校核结果。
3. `automation.rs::check_content` 将该错误作为数据库/校验错误返回；原 `run_task` 将子任务标记失败。
4. `automation.rs::drive` 进入 `waiting_user`，后续封面、分镜、成片都停止。此类校验错误不会进入自动模型重试，因而提示出现即等待处理。

问题是把音频质量判断和发布材料校对混成了同一个阻断条件。根因由明确提示词、真实错误分支和失败回归共同确认，无需猜测网络、音频生成或播放器原因。

## 本次处理

- 新校核提示词 `content-check-v3`：`verified` 仅表示最终发布/封面字段与有效讲述对应；音频范围、无来源细节、过度心理推断等写入 `warnings`，不因这些音频疑点要求暂停。
- 优先保留或修订为来源与实际讲述共同支持的发布材料，不把音频里的不确定断言写成确定事实，不把文案核对说成音频事实已全部证实。
- 提醒最多5条，每条1—500字，尽量标明字幕时间段，供用户发布前试听。字幕有效讲述不足、无法形成一致发布材料、原素材/字幕/摘要校验失败或非法输出仍可报错。
- 原声、字幕、时间码、音频指导及已有历史保留，不裁切、不重新配音、不重新生成音频。只按原流程修订必要发布/封面字段。
- 提醒复用原 `ai_tasks.preview_json`，从成功校核结果派生任务卡字段 `contentCheckWarnings`；不复制到会被阶段切换清空的 `error`，不新增状态、队列或设置。
- 原任务可在新构建中点击「继续流程」，通过现有恢复入口重新校核；发送前记录当前实际使用的 Prompt 版本，沿用冻结输入、同一个校核任务、音频和视频ID。
- 原最终人工发布边界保留。提醒显示至活动流程交接发布，用户仍可主动暂停、试听、修改或停止。

## 修改文件与数据库

产品代码：`src-tauri/src/ai/content_check.rs`、`src-tauri/src/database/automation.rs`、`src/services/automation.ts`、`src/components/AutomationTasks.vue`。

提示词：新增 `prompts/content-check-v3.md`；旧版本文件仅作历史追溯。

回归：`src-tauri/src/database/automation_tests.rs`、`tests/automation-tasks.test.ts`，并同步 `tests/automation.test.ts`、`tests/ai.test.ts` 的任务快照夹具。

文档：PRD、技术设计、数据库、AI规则、开发计划、使用手册及本文。

仍为 schema v25；无表、列、索引、SQL迁移、依赖或权限变化，未改写正式书库。修复按 `fix/patch` 处理。上述实现与验证阶段未手动升版本、提交、推送、发布或替换运行中的安装版；用户随后授权先拉取dev并中文提交推送，再拉取main并cherry-pick推送，Git交付结果以实际分支历史为准。

## 已执行验证

工作目录为仓库根。Rust命令先执行 `. ./.tools/env.ps1`；测试进程的 `TEMP`、`TMP`、`SQLITE_TMPDIR` 指向 `output/content-check-warning/test-temp`。

| 命令 | 实际结果 |
| --- | --- |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked --lib content_check_warnings -- --nocapture`（修复前） | 1项失败，真实 `finish_content_check_task` 返回「内容校核必须返回结论和必要修改项」，捕获提醒被当作失败的路径 |
| `npx vitest run tests/automation-tasks.test.ts`（修复前） | 7通过、1失败，缺少音频提醒展示 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked --lib content_check -- --nocapture`（首次修复后） | 9项通过；之后补充旧任务恢复和非法提醒回归，纳入下行全量 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked --lib` | 311通过、2项原有忽略（原生凭据、显式Node子进程专项），0失败 |
| `npx vitest run tests/automation-tasks.test.ts tests/automation.test.ts tests/ai.test.ts` | 最终3文件119项通过 |
| `npm run lint` | 通过 |
| `npm run build` | TypeScript检查与Vite生产构建通过；保留现有大于500kB产物提示 |
| `npm run check:rust` | fmt及Clippy `--all-targets -- -D warnings`通过 |
| `git diff --check` | 通过 |
| `npm run dev -- --port 1486 --strictPort` | 仅为隔离组件验收启动本地Vite服务 |
| `ego-browser nodejs -e "await import('file:///D:/MyWork/project/JIAZI-PROJECT/shuyin-ai/output/content-check-warning/browser-round-01.mjs')"` | 真实浏览器加载当前Vue任务卡组件 |
| `ego-browser nodejs -e "await import('file:///D:/MyWork/project/JIAZI-PROJECT/shuyin-ai/output/content-check-warning/browser-round-02.mjs')"` | 深浅主题×1440/800宽度共4组无横向溢出，运行中操作可用；交接发布后提醒仍在且不谎称继续制作 |

浏览器证据位于忽略目录 `output/content-check-warning/`：`ui.html`、`browser-result.json` 与5张界面截图。已查看运行中的深色1440截图和等待发布的浅色800截图。本次浏览器任务空间和1486端口测试服务已关闭。使用独立测试页面及本地状态夹具，未打开或操作用户正式任务/官方发布网页，不代表真实模型、NotebookLM或安装版全链验收。

## 验收对应

| 验收标准 | 结果 |
| --- | --- |
| 音频疑点不阻断已有合法发布材料 | 通过解析/真实数据库事务回归；整书和章节均进入封面阶段 |
| 提醒不在阶段切换或重开后丢失 | 通过同任务推进、SQLite重开及恢复回归 |
| 已因旧规则失败的任务可复用恢复 | 通过原 `verified=false` 错误 → 待处理 → 原任务恢复 → 当前Prompt → 带提醒成功的回归，音频/视频/冻结输入不变 |
| 三尺寸封面及同一视频继续 | 原三封面局部失败/恢复/冻结渲染回归加入章节提醒，仍通过 |
| 不能借提醒绕过材料和来源校验 | 非法字段、改写音频指导、非法提醒、无有效讲述的false、字幕变化和停止后迟到结果沿用拒绝，相关回归通过 |
| 用户能看见提醒并控制流程 | 119项相关前端回归及隔离真实浏览器验证通过；原暂停和最终人工发布操作保留 |

## 生效与剩余边界

需运行包含本次修复的新桌面构建后生效。当前安装版未替换，截图中的真实任务尚未恢复；无新安装包，未调用外部模型、NotebookLM或真实抖音上传。回归使用受控模型结果，不能据此宣称真实服务一定正确区分所有内容疑点，也不能认定截图列举的内容已证实错误。

下一步将修复随桌面版本交付后，在原任务点击「继续流程」并检查：原音频/字幕复用、疑点保留为提醒、封面继续生成；发布前按提醒时间段试听。该继续操作会重新调用文本校核，沿用既有限次重试规则。
