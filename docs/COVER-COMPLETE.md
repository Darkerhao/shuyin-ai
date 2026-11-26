# AI 整图封面

封面工坊 → 背景图片 → AI 封面生成 → AI 整图封面。沿用当前文字，可调整主标题、副标题和品牌文字后生成当前尺寸。空副标题或品牌不绘制；品牌为栏目名称等文字标识，暂不传 Logo 图片。

生成方式只决定下一次请求，不立即改变当前封面。候选显示后核对中文并点击「采用到该尺寸」，图片与本次冻结文案一起应用；失败、取消、不采用时保留原图。修改文字不会修改已经生成的图片，需要重新生成。

采用的整图不叠加本地文字、蒙层或装饰线，不提供会裁掉文字的缩放拖动；预览、缩略图和 PNG 导出共用同一渲染函数。4:3、3:4、16:9 分别生成，整图不直接共用到其他比例。已有本地整图可选择后勾选「图片已包含文字，直接作为完整封面」，比例不匹配会阻止导出。切换到普通背景或取消此选项恢复本地排字。

实现沿用 `start_cover_generation`、ImageProvider、素材与版本保存流程。请求增加可选 `coverText`，缺省生成无字背景；完整封面使用独立版本提示词 `cover-complete-v1`。`textMode` 存于现有配置 JSON，缺省 `overlay`，无需数据库迁移。工坊初次实现未接入自动制作；同日追加的自动流程集成见下文。旧版程序不支持新配置字段，回退需使用更新前备份。

验收标准：

1. 请求包含当前文案与目标比例，候选保持请求快照。
2. 整图采用后不二次绘制文字、蒙层或裁切，导出保持原画面。
3. 旧配置、普通背景与三种尺寸继续可用。
4. 保存、历史、草稿恢复和跨页面操作保留模式；失败不替换旧图。
5. 说明模型中文可能有误，并保留人工核对、采用步骤。

主要修改文件：

- `src/modules/covers/CoversPage.vue`、新增 `src/components/CoverTextFields.vue`：模式入口、共用文字输入、候选采用、整图控件及图片加载状态。
- `src/types/cover.ts`、`src/services/{covers,generation,coverRenderer}.ts`：配置恢复、请求快照、统一渲染。
- `src-tauri/src/database/{covers,image_generation}.rs`、新增 `prompts/cover-complete-v1.md`：参数校验、提示词、素材命名及持久化。
- `src-tauri/src/automation.rs`、`src-tauri/src/database/{automation.rs,video/ink.rs}`：初次工坊改动显式保持背景模式；同日自动制作接入见下文。
- `tests/{covers,coverRenderer,generation}.test.ts`、`src-tauri/src/database/{m3_tests,automation_tests}.rs`：回归验证与既有输入夹具适配。
- `docs/{PRD,TECHNICAL-DESIGN,DATABASE,AI-RULES,DEVELOPMENT-PLAN,COVER-COMPLETE}.md`：需求、设计、数据边界与本记录。

本地验证（2026-11-26）：

| 命令 | 结果 |
| --- | --- |
| `npm run typecheck` | 通过 |
| `npm run lint` | 通过 |
| `npm run build` | 通过，含 TypeScript；保留现有大于 500 kB 分包提示 |
| `npx vitest run tests/covers.test.ts tests/coverRenderer.test.ts tests/generation.test.ts --maxWorkers=2` | 82 项通过 |
| `npm run check:rust`（先 `. ./.tools/env.ps1`） | fmt / Clippy 通过 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked database::m3_tests` | 18 项通过，含整图请求、尺寸拒绝、保存及数据库重开 |
| `git diff --check` | 通过 |

浏览器命令使用 `ego-browser nodejs -e "await import('file:///D:/MyWork/project/JIAZI-PROJECT/shuyin-ai/output/cover-complete-20261126/round-09.mjs')"`，以及同目录 `round-10.mjs`、`round-11.mjs`、`round-12.mjs`。隔离内存 IPC 与本地画布夹具验证：生成请求含三组文字；采用后原图、预览与导出 PNG 像素完全一致；整图切换、保存全部尺寸、离开返回恢复通过；深浅色页面检查及 1366 / 1024 / 800 宽度无横向溢出。报告为 `output/cover-complete-20261126/browser-report.json`，截图同目录。初次夹具对 Vue 代理使用 structuredClone 导致测试请求失败，改为与 IPC 一致的 JSON 序列化后通过；不是实际服务故障。

验收标准 1—5 在源码、隔离数据库及浏览器夹具范围通过。真实付费图片模型、中文生成质量和已安装桌面版尚未验收；不能据此声称点击率提升。下一步在包含改动的桌面构建中用实际图片模型生成一张当前尺寸整图，核对文字和构图后导出。

## 自动制作接入（2026-11-26）

书籍资料的「自动制作并准备发布」增加「封面生成方式」，可选「AI 背景 + 本地排字」（默认）与「AI 整图封面」。选择冻结到本次任务，未完成期间锁定；整书和章节共用该设置。整图按校核后的主标题、副标题及现有品牌文字「甲子 · 读书」，独立生成 4:3、3:4、16:9 三张图片并自动采用、直接导出。发布前由用户检查中文和构图。该选项按任务保存，不是跨任务全局偏好。

复用 `StartAutomationInput.cover_text_mode` / `CoverTextMode`、`CoverStep`、ImageProvider 和封面渲染器。旧任务缺省为 `overlay`，重启及有限重试保留选择，已生成图片只重试本地导出。无新增依赖、权限或数据库迁移，schema 保持 v25；功能类型为 `feat`，未手动升版本、提交或发布。

本次修改文件：

- `src/components/{AutomationSetup,AutomationTasks}.vue`、`src/services/automation.ts`：设置、任务显示、模式请求及说明。
- `src-tauri/src/automation.rs`、`src-tauri/src/database/automation.rs`：复用三尺寸请求构造和保存的模式，导出使用相同模式。
- `tests/automation.test.ts`、`src-tauri/src/database/automation_tests.rs`：整书/章节、校核后的文案、三种比例、旧输入、非法模式、任务幂等、重启、失败续作和不重复生成回归。
- `docs/{PRD,TECHNICAL-DESIGN,DATABASE,AI-RULES,DEVELOPMENT-PLAN,COVER-COMPLETE}.md`：同步需求、规则、持久化及验收记录。

实际验证：

| 命令 | 结果 |
| --- | --- |
| `npx vitest run tests/automation.test.ts tests/automation-tasks.test.ts tests/coverRenderer.test.ts tests/generation.test.ts --maxWorkers=2` | 55 项通过 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked database::automation_tests`（先 `. ./.tools/env.ps1`） | 54 项通过 |
| `npm run lint` | 通过 |
| `npm run build` | TypeScript 与 Vite 通过，保留已有大于 500 kB 分包提示 |
| `npm run check:rust`（先 `. ./.tools/env.ps1`） | fmt / Clippy 通过 |
| `git diff --check` | 通过 |

Rust 重启回归首次为 53 通过 / 1 失败：测试直接重开数据库后对仍为 running 的任务调用 resume，未执行应用启动的 `recover_automation`。补齐真实恢复步骤后 54 项通过，未修改业务恢复规则。

浏览器使用 `npm run dev -- --port 18597` 和 `ego-browser nodejs -e "await import('file:///D:/MyWork/project/JIAZI-PROJECT/shuyin-ai/output/automation-cover-20261126/round-02.mjs')"`，随后依次运行同目录 `round-03.mjs`、`round-04.mjs`、`round-05.mjs`。隔离内存 IPC 验证真实组件下拉选择、章节启动请求的 `coverTextMode=embedded`、组件重挂恢复并锁定该选项、旧任务默认背景；深浅色及 1366/1024/800 三宽度无横向溢出，页面错误为空。布局与交互证据分别为该目录 `layout-report.json`、`interaction-report.json`，截图为 `dark.png`、`light.png`。首次导航等待 load 超时，但页面已加载完成，在同一页面继续检查通过。

上述验收标准在源码测试、隔离数据库及浏览器组件夹具范围通过；未调用真实付费图片模型，未打安装包或更新已安装桌面版，未执行真实 NotebookLM / 抖音全链路。下一步在包含改动的桌面构建中选择整图模式，用一次实际创作检查三张封面中的文字及最终发布预览。
