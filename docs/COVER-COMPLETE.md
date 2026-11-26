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

## 整图提示词冲突修复（2026-11-26）

用户反馈选择整图后仍没有标题、副标题。实际调用链为工坊 `coverText` / 自动制作 `CoverStep.image_input` → `start_cover_generation` → `prepare_image_generation` → `ImageProvider`。两条入口已有文字传递和独立整图提示词，并非只增加了界面选项；但全量创作 `publishing-v11`、按项调整 `publishing-revision-v10` 及自动内容校核 `content-check-v4` 仍要求画面描述包含「只生成无字背景」「主副标题由后期排版添加」。这些描述原样出现在整图要求之后，导致同一个图片请求包含互相冲突的要求。

这是源码及最终请求组装层面确认的缺陷。本次未取得用户这张图片对应的最新远端请求或模型输出，不能仅根据请求文本断言它是该样张漏字的唯一原因，也不能用测试通过代替真实模型的中文质量验收。

修复保持原有流程，未增加模型调用、依赖、执行器或本地补字路径：

- 新的 `publishing-v12`、`publishing-revision-v11`、`content-check-v5` 仅描述视觉主题、构图和风格；是否绘制文字由实际图片模式决定。字段及返回格式保持兼容，历史模板保留版本记录。
- `cover-complete-v2` 将完整封面要求放在书籍及视觉参考之后，最后单独列出必须逐字绘制的 `title/subtitle/brand`；主标题及非空可选文案必须出现，空字段省略。旧画面描述只作为视觉参考，不批量改写旧数据或删除用户描述。
- 待处理封面真正发送时记录实际提示词版本。普通背景继续使用 `cover-background-v2`，视频插画维持原提示词及组装方式；已生成图片和封面不会自动重做。

修改文件：

- 提示词：新增 `prompts/{publishing-v12,publishing-revision-v11,content-check-v5,cover-complete-v2}.md`。
- 业务：`src-tauri/src/ai.rs`、`src-tauri/src/ai/{revision,content_check}.rs`、`src-tauri/src/database/image_generation.rs`。
- 测试：`src-tauri/src/ai/tests.rs`、`src-tauri/src/database/{m3_tests,automation_tests}.rs`。
- 文档：`docs/{PRD,TECHNICAL-DESIGN,DATABASE,AI-RULES,DEVELOPMENT-PLAN,COVER-COMPLETE}.md`。

数据库 schema 仍为 v25，无 SQL 迁移。沿用原任务、版本、输入和素材结构，仅发送待处理请求时更新其提示词版本；不修改正式书库中的历史作品。

回归验收：

1. 全量创作、按项调整与内容校核的当前模板不再强制无字背景。
2. 历史描述在三种比例整图中先作为参考出现，之后明确完整封面要求及精确文案清单。
3. 整书与章节自动制作均使用校核后的主副标题和品牌，保留有限重试与已完成导出。
4. 空副标题、旧输入、待处理任务提示词版本、普通背景、保存恢复和整图直接导出继续通过。
5. 安装版更新及真实图片模型中文样张仍待单独验收；修复按 `fix` / patch 处理，不手动改版本、提交、推送或替换运行中的应用。

先增加两条回归再修复：`cargo test --manifest-path src-tauri/Cargo.toml --locked cover_ -- --nocapture` 首次为 15 通过、2 失败，分别暴露上游强制禁字和旧参考位于最终要求之后；修复后该组 17 通过。随后扩展自动制作夹具，验证历史禁字描述、整书/章节及三种比例的最终文案清单。

实际验证命令与最终结果见下表；Rust 命令均先执行 `. ./.tools/env.ps1`。

| 命令 | 结果 |
| --- | --- |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked` | 342 通过，2 项原有手动环境验证忽略 |
| `npx vitest run tests/covers.test.ts tests/coverRenderer.test.ts tests/generation.test.ts tests/automation.test.ts tests/automation-tasks.test.ts --maxWorkers=2` | 5 文件、116 项通过 |
| `npm run lint` | 通过 |
| `npm run build` | TypeScript 与 Vite 构建通过，保留已有大于 500 kB 分包提示 |
| `npm run check:rust` | rustfmt / Clippy（所有 targets、警告视为错误）通过 |
| `git diff --check` | 通过 |

构建另有本机 npm 旧 `python` / `msvs_version` 配置提示，未修改用户全局配置。未运行付费图片请求、安装包构建、桌面人工验收或真实发布流程。验收 1—4 在源码与隔离夹具范围通过；验收 5 的真实图片质量仍保留待验收边界。下一步将修复纳入桌面构建，用当前标题、副标题及品牌实际生成一张并核对中文，再确认三比例最终画面。
