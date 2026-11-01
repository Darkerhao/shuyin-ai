# 各尺寸封面独立编辑验收记录

日期：2026-11-01。发布影响：feat / minor；未修改版本号、提交或发布。

## 实现与取舍

- 4:3、3:4、16:9 各有独立文字、排版、背景、裁切与画面描述。当前表单直接编辑选中配置，不再使用一份配置改变宽高模拟三份封面。
- 可单独生成背景、设置本地图片、导出当前尺寸，也可保存全部配置或导出三张。预览缩略图可切换编辑。
- 共用背景或完整封面均先确认；应用后没有持续联动。共用背景保留各尺寸文字/排版/提示词；完整共用复制整套配置，保持各自画布尺寸。
- 一键生成及按项调整增加独立的 16:9 提示词。旧文案可只补该项，不重写其他内容。
- 保留单一渲染器、Provider、版本和自动暂存队列。完整工作区在正式保存后仍保留，避免单张历史无法恢复其他尺寸配置或当前选择。
- GPT Image 兼容接口请求横向 1536x1024 / 纵向 1024x1536，再本地裁切为准确 PNG 尺寸。不增加供应商特例、自动降级或额外收费重试。

## 修改文件

- 前端：`src/modules/covers/CoversPage.vue`、`src/modules/writing/WritingPage.vue`、`src/services/covers.ts`、`src/types/cover.ts`、`src/types/ai.ts`、`src/composables/useEditorDraft.ts`。
- Rust：`src-tauri/src/ai.rs`、`src-tauri/src/ai/images.rs`、`src-tauri/src/ai/publishing.rs`、`src-tauri/src/ai/revision.rs`、`src-tauri/src/cover_commands.rs`、`src-tauri/src/database/covers.rs`、`src-tauri/src/database/image_generation.rs`。
- Prompt：`prompts/publishing-v5.md`、`prompts/publishing-revision-v4.md`、`prompts/cover-background-v2.md`。旧版本文件仅保留作历史，新调用不再引用旧入口。
- 测试：`tests/covers.test.ts`、`tests/ai.test.ts`、`src-tauri/src/ai/tests.rs`、`src-tauri/src/database/m2_tests.rs`、`src-tauri/src/database/m3_tests.rs`、`src-tauri/src/database/m4_tests.rs`；按项调整与全量输出校验测试同时位于对应 Rust 模块。
- 文档：`docs/PRD.md`、`docs/TECHNICAL-DESIGN.md`、`docs/DATABASE.md`、`docs/AI-RULES.md`、`docs/DEVELOPMENT-PLAN.md`、`docs/USER-GUIDE.md`、本文。

## 数据库

无 SQL 表/列或 schema 版本变更，仍为 v13。已有 JSON 增加 `coverPromptWidescreen`、`backgroundPrompt`，cover 草稿增加 schemaVersion=2 的多尺寸工作区。新版兼容读取旧文案、旧封面和旧单份草稿；不承诺旧应用能读取新 JSON。背景文件仍按素材 ID 引用，原视频、历史版本、导出文件不被重写。

## 实际命令与结果

| 命令 | 结果 |
| --- | --- |
| `npx vitest run tests/covers.test.ts -t 'edits and exports only\|copies a cover to other'`（实现前） | 2 项预期失败：旧尺寸切换串改，缺少共用操作。 |
| `npx vitest run tests/covers.test.ts tests/ai.test.ts tests/coverRenderer.test.ts` | 第一轮 120 项通过；后续增加独立背景/恢复/按项补充覆盖。 |
| `npx vitest run`（最终） | 15 文件、250 项通过。 |
| `npm run typecheck` | 通过；最终 build 再次执行 vue-tsc。 |
| `npm run lint`（最终） | 通过。 |
| `npm run build`（最终） | 通过；Vite 仍提示单包超过 500 kB，未调整阈值或引入依赖。 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml` | 已执行格式化。 |
| `npm run test:rust` | 137 通过、1 项既有原生凭据测试忽略；main/doc tests 无失败。包含旧迁移、备份恢复、多尺寸配置/草稿重启、图片请求方向、目标尺寸校验和 16:9 按项补充。 |
| `npm run check:rust` | fmt / Clippy all-targets 通过。 |
| `npm test` | 未整体通过：当时前端 249 项通过，发布/音频脚本 78 通过、1 项 macOS 检查跳过；队列 18 通过、1 失败。最终新增的前端用例随后纳入 250 项全量通过结果。 |
| `npm run test:queue`（独立复跑） | 18 通过、同一项失败，详见下节。 |
| `git diff --exit-code HEAD -- scripts/codex-queue.mjs tests/codex-queue.test.mjs` | 退出码 0，失败模块与 HEAD 一致，本次未修改。 |
| `git diff --check` | 通过；存在工作区 CRLF 转 LF 的提示。 |

Rust 命令先加载 `. ./.tools/env.ps1`。测试进程 `TEMP`、`TMP`、`SQLITE_TMPDIR` 指向 `output/cover-workspace-temp`，没有移动或清理用户系统目录。

日志：`output/cover-vitest-final.log`、`output/cover-lint.log`、`output/cover-build.log`、`output/cover-rust-tests.log`、`output/cover-rust-check.log`、`output/cover-full-tests.log`、`output/cover-queue-retry.log`。

## 浏览器运行验证

使用 Playwright CLI、隔离浏览器和内存 IPC / localStorage 夹具，不访问正式 SQLite、真实图片服务或用户素材。最终运行针对最新 `npm run build` 产物，通过 `npm run preview -- --port 5190 --strictPort` 启动。

实际执行：

```powershell
npx --yes --package @playwright/cli playwright-cli -s=cover-release-check open about:blank
npx --yes --package @playwright/cli playwright-cli -s=cover-release-check run-code --filename output/playwright/cover-independent-setup.js
npx --yes --package @playwright/cli playwright-cli -s=cover-release-check snapshot
npx --yes --package @playwright/cli playwright-cli -s=cover-release-check run-code --filename output/playwright/cover-independent-verify.js
npx --yes --package @playwright/cli playwright-cli -s=cover-release-check eval '() => window.__coverVerification'
```

结果：

- 分别编辑三个标题，切换不串改；单独导出 16:9，再批量导出；刷新后分别恢复三个标题与当前选择。
- 实际 Canvas PNG 的 IHDR 为 1920×1080、1440×1080、1080×1440、1920×1080，字节数分别 91969、81060、80396、91969。这里只生成浏览器 PNG 数据，文件保存 IPC 是夹具。
- 16:9 对应提示词可见。点击「将当前封面应用到全部尺寸」并通过 `dialog-accept` 确认后，回读三份标题一致、activeSize=2。
- 1440×1000、980×768、390×844 下 document scrollWidth 均等于 clientWidth；390px 下可切换实时预览。已查看桌面、窄屏编辑和窄屏预览截图。
- 最终浏览器 pageErrors 为空，console error 为 0。开发服务器热更新期间曾暴露夹具缺少事件清理方法和缓存旧模块的问题；补齐夹具后改用最新生产构建完成上述验证，不将此前过程计为最终验收。

证据：`output/playwright/cover-independent-evidence.log`、`output/playwright/cover-shared-evidence.log`、`output/playwright/cover-independent-1440.png`、`output/playwright/cover-independent-980.png`、`output/playwright/cover-independent-390.png`、`output/playwright/cover-independent-mobile-preview.png`。

## 验收对应

| 标准 | 证据 / 状态 |
| --- | --- |
| 三尺寸独立设置与生成 | 前端回归验证独立文字、描述、背景、裁切及生成目标，Rust 验证任务冻结尺寸和 Provider 请求；真实远程生成待验收。 |
| 16:9 独立提示词 | 新全量 Prompt/必填与长度比例校验、旧文案单项预览采用、复制及页面展示通过。 |
| 共用背景或封面 | 确认/取消、不持续串改、背景共用保留文字/提示词的测试通过；浏览器确认后回读通过。 |
| 单张和全部导出 | 独立背景与当前单张输出测试、实际浏览器 PNG 尺寸检查通过。 |
| 草稿与历史保留 | 多尺寸恢复、旧数据默认值、SQLite 重启及备份恢复通过；正式安装版待验收。 |
| 失败不丢稿 | 生成失败/取消、背景缺失、排版失败和部分写入失败回归通过；不删除其他尺寸编辑。 |

## 未解决问题与下一步

1. 与本次封面修改无关的队列测试 `tests/codex-queue.test.mjs:118` 持续失败：删除前置任务 report 后，预期重跑 one/two，实际仍仅有首次 one/two。未修改队列模块，也未弱化测试；应独立定位后再要求完整 `npm test` 全绿。
2. 未调用真实付费图片/文本服务；需在实际供应商核验横竖图片尺寸支持，尤其本次用户使用的图片模型。服务不支持时明确报错，原封面保留。
3. 未构建或升级桌面安装包，未操作正式业务库，未验收 macOS/Linux。下一步在 Windows 桌面版使用一本测试书完成各尺寸生成、拖动裁切、正常关闭重开及真实 PNG 文件导出，再打包发布。
