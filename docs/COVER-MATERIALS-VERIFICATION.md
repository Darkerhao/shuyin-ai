# 封面素材生成验收（2026-10-24）

## 实现范围

一键生成在原发布标题、发布内容、音频提示词之外，增加封面主标题、副标题及横版 4:3 / 竖版 3:4 两份图片提示词。以书籍资料和本期重点为输入，强调具体看点、同主题下不同构图、文字留白和无字背景。结果可编辑、复制、追加版本，并带入封面工坊。

新封面自动带入主副标题，已有封面由用户主动应用。结果页直达使用当前选中的 writingVersion，有修改先保存；保存失败保留原页。内置 AI 背景沿用现有方形输出与双向裁切；外部工具可分别使用两种构图提示词。未新增依赖或图片协议。

## 修改文件

- `prompts/publishing-v2.md`：唯一启用的新版文本生成 Prompt；旧 Prompt 仅保留历史记录。
- `src-tauri/src/ai.rs`、`src-tauri/src/ai/publishing.rs`：结果字段、历史默认值、AI 输出完整性/长度/比例校验。
- `src/types/ai.ts`、`src/modules/writing/WritingPage.vue`：七项编辑、复制、保存与当前版本直达。
- `src/modules/covers/CoversPage.vue`：主副标题应用、横竖提示词复制、AI 描述带入及读取失败重试。
- `src/App.vue`、`src/modules/projects/ProjectPage.vue`、`src/modules/projects/ProjectsPage.vue`：入口说明。
- `tests/ai.test.ts`、`tests/covers.test.ts`、`tests/publishing.test.ts`：生成、编辑、兼容、复制与封面集成回归。
- `src-tauri/src/database/m2_tests.rs`、`src-tauri/src/database/m4_tests.rs`：旧 JSON 和新字段的保存、备份、恢复与重启验证。
- `README.md`、`docs/PRD.md`、`docs/TECHNICAL-DESIGN.md`、`docs/DATABASE.md`、`docs/AI-RULES.md`、`docs/DEVELOPMENT-PLAN.md`、`docs/USER-GUIDE.md` 及本文：范围、契约、使用与验证同步。

## 数据库

schema 仍为 v7，无表结构变更和 SQL 迁移。四项封面字符串随 `writing_versions.content_json` 保存。旧记录缺少字段时读为空值，原 JSON 不改写；人工编辑仍允许缺少封面内容。自动测试确认旧 JSON 经恢复后保持原字节，新字段与中文换行经备份、恢复和重启完整保留。

## 实际命令与结果

PowerShell 中 Rust 命令先执行 `. ./.tools/env.ps1`，使用本机现有 Rust 环境。

| 命令 | 结果 |
| --- | --- |
| `npx vitest run tests/ai.test.ts tests/covers.test.ts tests/publishing.test.ts` | 43 项通过 |
| `npm run typecheck` | 通过 |
| `npm test` | Vitest 62 项通过；发布脚本 27 项通过，1 项 macOS 测试按平台跳过 |
| `npm run lint` | 最终通过；首次 8 条 Vue 模板换行警告已按规则修复，未关闭规则 |
| `npm run build` | TypeScript 检查及 Vite 构建通过 |
| `npm run test:rust` | 最终 53 项通过，1 项需单独触发的系统凭据测试忽略；新增测试首次缺少 params 宏导入，补齐后全量重跑通过 |
| `npm run check:rust` | rustfmt 和 Clippy 全目标、零警告检查通过 |
| `npm run tauri -- build --no-bundle` | Windows release 可执行文件构建通过；没有构建安装包或升级正式安装 |
| `git diff --check` | 通过 |

可执行文件：`src-tauri/target/release/shuyin-ai.exe`。保留现有 0.6.0 版本声明，本次属于新增用户能力，后续发布使用 feat/minor。

## 浏览器验证

使用 playwright 技能、可见 Chromium 和当前 Vite 页面，IPC 为隔离模拟数据；不访问正式数据库、不调用外部模型。运行：

```powershell
npm run dev -- --port 1437
npx --yes --package @playwright/cli playwright-cli -s=cover-materials open about:blank --headed
npx --yes --package @playwright/cli playwright-cli -s=cover-materials run-code --filename output/playwright/cover-materials/setup.cjs
npx --yes --package @playwright/cli playwright-cli -s=cover-materials snapshot
```

根据快照执行复制横版提示词、编辑主标题、保存并制作封面、选择竖版提示词并复制、切换竖版预览；随后运行断言：

```powershell
npx --yes --package @playwright/cli playwright-cli -s=cover-materials run-code --filename output/playwright/cover-materials/verify.cjs
```

确认保存先于跳转、URL 指向新版本、封面带入已修改标题、系统剪贴板与竖版提示词一致、封面保持未保存、Canvas 为1080×1440，900px窗口无横向溢出。截图已人工查看，文字及控件无截断。控制台仅有开发页 favicon.ico 404，无业务异常。

截图、夹具和 CLI 快照保留于忽略目录 `output/playwright/cover-materials/`，截图是示例资料的页面效果，不是 AI 质量证明。

## 验收对应与剩余范围

- 四项封面内容随同一文本任务生成：结构、字段必填、上限、错误比例及同文拒绝有 Rust 回归；真实模型的书籍忠实度和吸引力尚待人工试用。
- 可编辑、复制、保存、切换历史：前端自动测试通过，浏览器验证中文剪贴板与保存后跳转。
- 减少重复输入：新封面自动带标题、当前版本直达、已有封面手动应用有回归。
- 失败不丢内容：请求失败、生成中编辑、取消、保存失败、文案读取失败后重试有回归。
- 旧数据与备份保留：真实临时 SQLite 自动测试通过，无正式库改动。
- 本轮未执行外部文本/图片模型实测、Windows 安装升级或 macOS/Linux 实机验收；内置图片服务不直接生成两种原始比例。

下一步用一本补充过原书资料的书，在桌面开发版运行一键生成，审核主副标题与两种构图是否贴合本期主题；确认质量后再打包并升级安装。回退旧应用时使用升级前数据库备份，因为旧程序不理解新 JSON 字段。
