# 封面字号、配色与默认样式验收（2026-10-27）

## 实现与使用

- 一键生成同步推荐主标题字号/颜色、副标题字号/独立颜色、点缀色、底色与图片蒙层，沿用统一文本 Provider。
- 旧版本可在「按项调整」只选择「封面字号与配色」，先对比后采用；其他文字保持原样。
- 新封面自动带入推荐；已有封面在「排版样式」主动应用。五套默认样式为黑金焦点、赤焰醒目、蓝调纪实、深林、纸页，卡片预览后点击应用，仍可逐项调整或保存为个人样式。
- 文字和无字背景继续分开生成。预览与三种比例导出共用 Canvas；长文字自动换行与缩小，不保证推荐字号原样适用于任意长文案。

## 修改文件

- `src/types/cover.ts`、`src/types/ai.ts`：独立副标题色、单一内置样式配置和 AI 推荐类型。
- `src/components/CoverDesignPreview.vue`、`src/modules/covers/CoversPage.vue`、`src/modules/writing/WritingPage.vue`：推荐展示、卡片应用、按项调整对比、旧配置兼容；补齐内容页既有 Badge 导入。
- `src/services/coverRenderer.ts`：副标题使用独立颜色，旧配置沿用原文字色。
- `src-tauri/src/ai.rs`、`src-tauri/src/ai/publishing.rs`、`src-tauri/src/ai/revision.rs`：推荐对象解析/校验和选中字段合并；采用 `Option<Box<CoverDesign>>` 避免扩大历史内容枚举。
- `src-tauri/src/database/covers.rs`：副标题色校验、个人样式保存和新增模板 ID。
- `prompts/publishing-v3.md`、`prompts/publishing-revision-v2.md`：启用的新 Prompt 版本；旧 Prompt 仅保留历史，不再被运行代码引用。
- 前端 `tests/ai.test.ts`、`tests/covers.test.ts`、`tests/coverRenderer.test.ts` 和 Rust `database/m2_tests.rs`、`m4_tests.rs`、`workflow_tests.rs`：自动回归、兼容、保存/恢复验证。
- 同步 PRD、技术设计、数据库、AI 规则、开发计划和使用手册。

## 数据库

保持 schema v11，无表结构或 SQL 迁移；新增字段随现有 JSON 快照保存。旧文案缺少 coverDesign 时无推荐，旧封面/草稿/个人样式缺少 subtitleColor 时使用其原 textColor，历史原始 JSON 不改写。新增推荐随文案备份恢复及重启保留；副标题色随个人样式备份恢复保留。没有读取或修改正式业务库，没有新增依赖、IPC、权限或密钥操作。

## 实际命令与结果

Rust 命令先运行 `. ./.tools/env.ps1`。Rust target 构建锁和 Vite 旧 dist 清理最初被沙箱拒绝，随后使用正常本机权限重跑；没有关闭规则或修改依赖。

| 命令 | 结果 |
| --- | --- |
| `npx vitest run tests/covers.test.ts tests/coverRenderer.test.ts tests/ai.test.ts` | 当时 75 项通过；后续增加按项设计采用用例，由全量测试覆盖 |
| `npm run typecheck` | 通过 |
| `npm run lint` | 通过，零警告 |
| `npm run build` | TypeScript 和 Vite 构建通过；未构建桌面安装包 |
| `npm run check:rust` | rustfmt / Clippy 全目标通过，零警告；首次 Clippy 体积检查已通过 Box 修正 |
| `npm run test:rust` | 92 项通过，1 项系统凭据测试按原规则忽略 |
| `npm test` | 前端 153 项通过；发布脚本 30 项通过、1 项 macOS 按平台跳过；队列测试 18 项通过、1 项失败 |
| `npm run test:queue` | 正常本机权限下单独复核，仍为18项通过、1项失败：缺失前驱报告时没有重跑已完成任务及后继任务；未改动本轮范围之外的队列脚本/测试 |
| `git diff --check` | 通过 |

完整测试未全部通过，不能将此批改动报告为全量绿灯。队列失败来自 `tests/codex-queue.test.mjs:118`，原有队列文件未修改；日志为忽略目录 `output/cover-styles-tests-final.log`、`output/cover-styles-queue-final.log`。

## 浏览器与验收对应

按 Playwright 技能验证，使用 `npm run dev -- --port 1441 --strictPort` 及 Playwright MCP 的 `browser_navigate`、`browser_run_code_unsafe`、`browser_snapshot`、`browser_take_screenshot`、`browser_console_messages`。CLI 缓存检查不可用后复用 MCP，不安装新依赖。浏览器内注入隔离 Tauri IPC 夹具，不访问正式数据库、不调用外部模型。

- 推荐：新封面显示148/54 px、主副标题色、点缀色、底色及75%蒙层，自动带入主副标题；已有封面主动应用、旧版本单项推荐采用由前端/Rust回归验证。
- 默认样式：依次点击全部五张卡片，选中状态、字号和副标题色同步，主副标题不变，真实 Canvas 无排版错误。
- 独立颜色：应用 AI 推荐后把副标题改为 `#a8e5ff`，主标题仍为 `#ffe45c`，点缀仍为 `#ff7547`。点击导出后，真实 Canvas 生成 PNG；模拟 IPC 接收到1440×1080、1080×1440、1920×1080三份配置且三色分别保留。
- 响应式：1440、900、390 px 窗口均无页面水平溢出。控制台检查为零错误、零警告。
- 视觉：查看 `output/playwright/cover-styles/workbench-1440.png`，大标题、独立副标题和配色卡可读；封面局部效果为同目录 `canvas-preview.png`。图片仅为界面夹具示意，不作为外部 AI 质量证明。

## 剩余边界与下一步

本轮未进行真实文本/图片服务质量实测、桌面安装版导出写文件、安装升级、macOS/Linux实机验收；不提交、不升版本、不发布。本轮新增用户能力，后续发布应归类 feat/minor。

下一步在桌面开发版选择一本有书籍资料的真实项目，单项生成排版推荐，核对背景上标题与副标题的实际清晰度，再保存并导出三种比例。发布前另行处理现有队列测试失败。
