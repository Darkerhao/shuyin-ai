# shadcn-vue 全页面接入验收

日期：2026-10-25。

## 全页面补齐结果

已完成全部页面及共用交互控件的迁移。截图中的封面预览比例、对齐方式、文字位置均改为 shadcn-vue Select。沿用现有暖白、灰阶、墨绿主题和业务布局。

| 页面 / 模块 | 当前使用的组件 |
| --- | --- |
| 书架、书籍新建 / 编辑 | Button、Input、Textarea、Badge、Select、Collapsible |
| 内容创作 | Button、Textarea、Select、Tabs、Collapsible |
| 封面工坊 | Button、Input、Textarea、Badge、Select、Slider、Tabs、Collapsible |
| 发布准备 | Button、Input、Textarea、Badge、Select、Checkbox、Collapsible |
| 模型设置 | Button、Input、Badge、Select、Checkbox、Collapsible |
| 数据与备份、技术验证 | Button、Textarea、Badge（按实际控件使用） |
| 素材管理、公共反馈 | Button、Badge（按实际控件使用） |

保留原有提交、取消、未保存保护、失败保稿、数据备份确认、素材移除确认和半自动发布流程。删除手写 EditorTabs 及已替代的原生控件样式。Tabs 和折叠区保留内部内容；颜色与日期时间选择仍使用 Input 所承载的浏览器原生能力。

Select、Checkbox、Slider 显式绑定禁用条件。历史选择由保存版本控制，用户取消放弃修改时保持原选项和草稿；空素材仍提交 `null`，字号和画布尺寸仍提交数字。

浏览器发现并修复了动态任务标签滞后：当前 Reka SelectItemText 注册选项时缓存文字，同一任务 ID 的标题/状态改变后触发器仍可能显示旧文字。发布任务与视频状态改用 SelectValue 插槽直接读取业务数据。回归先复现失败，再验证修改后标题、状态与文件可用性立即更新；没有修改依赖或增加缓存状态。

## 本轮修改文件

- `src/modules/{writing,covers,publishing,settings,verification}/` 下的页面，`src/modules/projects/ProjectAssets.vue` 和 `ProjectPage.vue`：统一组件、事件绑定、禁用条件和动态标签。
- `src/components/ui/{checkbox,slider,tabs,collapsible}/`：按官方 registry 导入实际使用的组件，沿用现有 Reka / VueUse 和 AppIcon，无新增 npm 依赖。
- `src/components/ui/select/Select.vue`：保留业务值的泛型类型；`textarea/Textarea.vue`：保留 rows 和纵向调整大小；`OperationFeedback.vue`：统一关闭按钮。
- 删除 `src/components/EditorTabs.vue`；`src/style.css` 清理原生按钮、输入、Select、旧状态标签与手写页签样式，保留页面布局。
- `tests/ai.test.ts`、`tests/covers.test.ts`、`tests/publishing.test.ts`、`tests/ui.ts`：真实 DOM / 键盘交互与业务回归。Select 测试操作浮层选项，不通过组件 emit 跳过交互；ResizeObserver 仅在 jsdom 测试中提供空实现，布局另由真实浏览器检查。
- `docs/PRD.md`、`docs/TECHNICAL-DESIGN.md`、`docs/DEVELOPMENT-PLAN.md`、本文：同步全页面范围和验证证据。

## 本轮数据库与版本

schema v7 不变，无迁移、Rust、IPC、权限、实际数据或凭据变更。本轮补齐没有新增依赖，未修改版本号，也未执行 Git 暂存、提交或发布；已有首批接入改动保留。

## 本轮实际命令与结果

| 命令 | 结果 |
| --- | --- |
| `npm run lint` | 通过，零警告 |
| `npm run build` | TypeScript 及 Vite 生产构建通过 |
| `npm test` | 当时 72 项前端测试通过；发布脚本 27 通过、1 个 macOS 专属检查跳过 |
| `npx vitest run tests/publishing.test.ts -t 'updates the selected task label'` | 修复前明确复现任务标签仍显示旧标题 / 待发布状态 |
| `npx vitest run` | 修复动态标签并新增回归后，最终 73/73 通过，覆盖全部 6 个前端测试文件 |
| `git diff --check` | 通过 |

发布脚本的源文件和依赖在补充标签回归后没有变化，因此未重复执行发布脚本测试；最终前端全量测试、lint 和构建已重跑。

浏览器使用已有本地 Vite 开发服务及 Playwright CLI，所有业务数据由 `output/playwright/setup-all-pages.js` 注入的内存 IPC 提供，不访问正式 SQLite、不调用真实模型、不写入实际素材或发布任务包。

实际命令：

```powershell
npx --yes @playwright/cli -s=shuyin-all-ui open http://127.0.0.1:1420 --browser chrome
npx --yes @playwright/cli -s=shuyin-all-ui run-code --filename=output/playwright/setup-all-pages.js
npx --yes @playwright/cli -s=shuyin-all-ui run-code --filename=output/playwright/audit-all-pages.js
npx --yes @playwright/cli -s=shuyin-all-ui run-code --filename=output/playwright/check-cover-controls.js
npx --yes @playwright/cli -s=shuyin-all-ui run-code --filename=output/playwright/check-model-controls.js
npx --yes @playwright/cli -s=shuyin-all-ui run-code --filename=output/playwright/check-publish-controls.js
npx --yes @playwright/cli -s=shuyin-all-ui console error
```

- 9 个路由 × 1440 / 1120 / 800 / 360px，共 36 组：无横向溢出、无遗留可见原生控件、无未命名 Select，页面运行错误为 0。
- 封面：实际键盘选择居中后焦点回到触发器；切换靠下、字号 96、横版后保存，IPC 数值与 Canvas 的 1440×1080 尺寸一致。
- 模型：点击停用后，默认勾选立即清除且禁用，再保存成功。
- 发布：清空横版封面后确认状态撤销，保存接收到 `landscapeCoverId=null`；任务触发器立即显示修改后的标题与草稿状态。
- 控制台 Errors 0 / Warnings 0。Chromium 的密码表单 autocomplete 建议信息属于 verbose，不是运行错误。
- 布局数据保存在 `output/playwright/all-ui-audit.json`；交互结果在 `cover-controls-result.log`、`model-controls-result.log`、`publish-controls-result.log`；截图为 `all-ui-*.png`，已检查主要编辑页、模型页及窄屏发布页。截图为示例数据。

## 本轮验收与剩余边界

- [x] 所有页面使用统一组件，截图中的三处原生下拉已替换。
- [x] 封面模板 / 排版 / 数值 / 空背景 / 双比例导出原有逻辑回归通过。
- [x] 文案编辑、复制、切换标签、失败保稿、快捷保存和历史切换保护通过。
- [x] 模型复选框联动、发布素材变更后撤销确认、动态标签刷新、保存中禁用通过。
- [x] 书籍、素材、目录备份、技术验证原有回归继续通过。
- [x] 无新增数据库迁移、依赖或权限；旧手写 Tabs 与控件 CSS 已移除。
- [x] Windows Tauri 桌面补充：下拉浮层、滑块、日期时间、目录 / 素材 / 背景图片选择与取消已核对；详见下方桌面记录。
- [ ] 原生颜色弹层的鼠标选色、备份文件选择器：本轮未完成，原因及已覆盖范围见下方。
- [ ] 安装包更新、macOS / Linux 实机：本轮未执行。

组件来源：官方文档 `https://www.shadcn-vue.com/docs/components`，源码 `https://github.com/unovue/shadcn-vue/tree/dev/apps/v4/registry/new-york-v4/ui`；同时检查了本项目安装的 Reka 2.11.0 Select / Checkbox / Slider / Tabs / Collapsible 实现。

## Windows WebView2 与原生对话框补充核对（2026-10-25）

本次使用当前工作区源码重新编译 Tauri 开发运行版（`--release` 为编译优化模式，不代表安装包验收）。Windows 11 23H2 / 10.0.22631 x64，WebView2 149.0.4022.80，Rust Tauri 2.12.5，Dialog JS / Rust 均为 2.8.3。复用本仓库 Vite 1420，独立窗口为 1120×900；Playwright 通过 CDP 19341 连接真实 WebView2，Computer Use 操作 Windows 原生窗口。未注入内存 IPC，也未调用外部模型。

独立 identifier 为 `com.shuyin.desktop-ui-20261025`。SQLite / WebView 数据位于 C 盘该 identifier 的 AppData 目录；测试素材、工作目录、导出、备份及证据在 D 盘 `output/playwright/desktop-ui/`。本轮未连接正式 identifier 的 IPC。无业务源码、依赖、Tauri 权限、数据库结构或版本号改动；schema 保持 v7，测试只写入独立验证库。已有暂存和未暂存修改保留。

核对实际调用链 `src/services/{projects,covers,verification}.ts`、`src-tauri/src/lib.rs`、`src-tauri/capabilities/main.json` 与本地锁定的 Dialog 2.8.3 源码：选择接口统一使用 `open`，只有主窗口 `dialog:allow-open` 权限；插件设置 Windows 父窗口，取消返回空结果，由现有调用方保留原值。文档参照 Tauri 官方 Dialog 与 Microsoft WebView2 调试文档，不据浏览器模拟结果推定原生行为。

| 核对项 | 实际结果 |
| --- | --- |
| 原生目录选择 | 选择包含中文及空格的测试目录，真实 `workspace_settings` 返回正确路径；再次取消后设置不变 |
| 原生素材选择 | 显示视频 / 图片 / 字幕 / 文档过滤器；选中 `验证 视频.mp4` 后真实 SQLite 记录关联，源文件保留 |
| 重新定位取消 | 点击原生取消，窗口关闭，素材 ID、路径及元数据不变 |
| 原生背景图片选择 | 图片过滤器仅显示 PNG / JPEG / WebP；选择 `背景 图片.png` 后画布显示背景，再次取消保留背景、草稿和两条素材记录 |
| WebView2 Select | 对齐用方向键和 Enter 改为居中，焦点返回 `cover-align`；文字位置改为靠下，预览改为 3:4，Canvas 实际变为 1080×1440 |
| WebView2 Slider | 方向键按 0.05 调整；End 到 0.85，Home 到 0；最终 0.05 随配置保存，数据库仍为数值 |
| 颜色输入 | 原生颜色弹层显示正常；通过真实 color input 填入 `#e8dcc0` 后保存，SQLite 配色一致。弹层内鼠标选色未完成，不计为通过 |
| 原生日期时间 | Alt+ArrowDown 打开日历；左方向键和 Enter 将 10 月 25 日改为 24 日，支持完整日期时间输入；取消确认后任务仍为 ready，发布记录为 0 |
| 发布控件 | 选择视频和双比例封面，勾选两项确认，生成真实任务包；任务选择器显示更新后的标题和待发布状态 |
| 保存与文件闭环 | 1 条配置版本、2 条导出版本；PNG 文件头确认 1440×1080 / 1080×1440，任务包两张封面与源 PNG 字节相同；任务包不复制视频 |
| 数据完整性 | 独立库及新建备份均为 schema v7，`integrity_check=ok`、外键错误为 0；均为 1 本书、2 条素材、3 条封面、1 条任务、0 条发布记录 |
| 错误与源文件 | 页面异常及 console error 均为 0；测试源图片和视频与原夹具逐字节一致 |

本次实际执行：

| 命令 / 操作 | 结果 |
| --- | --- |
| `. ./.tools/env.ps1` 后 `npm run tauri -- info` | Windows、MSVC、Rust、WebView2 环境识别通过 |
| `npm run desktop:dev -- --release --config output/playwright/desktop-ui/tauri-test.json --no-watch` | 当前源码编译通过，独立 WebView2 窗口启动成功 |
| `npm run lint` | 通过，零警告 |
| `npm run build` | TypeScript 检查与 Vite 生产构建通过 |
| `npm test` | 前端 73/73；发布脚本 27 通过、1 个 macOS 专属检查跳过 |
| Node REPL：Playwright `chromium.connectOverCDP` 与 Computer Use `sky` | 完成上表控件及原生选择核对；结果在 `evidence.json`，桌面输入操作记录在本聊天工具日志中 |
| `python output/playwright/desktop-ui/verify_files.py` | 只读数据库 / 备份 / PNG / 源素材 / 任务包比对通过；结果在 `filesystem.json` |
| `git diff --check` | 通过 |

证据位于 `output/playwright/desktop-ui/`：`tauri-test.json`、`evidence.json`、`filesystem.json`、`verify_files.py`、`cover-style.png`、`cover-final.png`。目录受 Git 忽略，不包含模型密钥。

收尾已按可执行文件绝对路径核对并停止本轮独立进程，测试数据和证据保留；原有开发窗口与正式安装窗口保持运行。`Stop-Process` 导致 Tauri 启动命令最终报告 `0xffffffff`，这是主动结束验证进程的结果，不是编译或业务运行失败。

剩余边界：Computer Use 点击颜色弹层时返回 `point ... is over msedgewebview2.exe "Chrome Legacy Window", not target window shuyin-ai.exe`；激活并重新截图后重试仍被拦截。随后窗口截图黑屏，重新激活返回 `failed to activate captured window`，因此停止原生输入，没有将工具故障记为应用缺陷或验收通过。备份已通过真实 IPC 创建并只读验证，原生备份文件选择器本轮未操作。下一步恢复可交互的桌面窗口后补测鼠标选色和备份选择 / 取消；技术验证页专用视频选择器、重启恢复、安装 / 升级、缺少 WebView2 的环境与其他操作系统仍不属于本次证据范围。

---

以下保留首批接入当时的记录与边界，当前完成范围以上文为准。

## 首批范围与结果（历史记录）

已完成应用侧栏、顶栏、书架与书籍表单的首批迁移。采用暖白背景、灰阶文字、墨绿主操作，移除书架大宣传横幅，统一筛选、搜索、排序和书籍卡片。

本轮没有重新实现内容创作、封面工坊、模型设置和发布准备的业务流程。这些页面使用新的应用壳层和共享主题，原有控件保留；其组件迁移作为后续切片。

## 修改文件

- `package.json`、`package-lock.json`：锁定 UI / Tailwind 依赖；VueUse 与 Reka 使用同一版本。
- `components.json`、`vite.config.ts`、`tsconfig.json`：组件路径、Tailwind 插件、`@/` 别名和 VueUse 所需全局类型。
- `src/components/ui/{button,input,textarea,badge,select}/`、`src/components/ui/LICENSE`：官方组件源码与许可证；只保留本切片需要的组件，复用已有 SVG 图标。
- `src/lib/utils.ts`：统一合并 Tailwind 类名。
- `src/App.vue`：侧栏、顶栏和页面容器。
- `src/modules/projects/ProjectsPage.vue`：书架、筛选、搜索、Select 排序和状态展示。
- `src/modules/projects/ProjectPage.vue`：书籍表单改用 Input、Textarea、Button。
- `src/style.css`：主题变量、CSS 分层、旧样式清理、正文辅助字号调整；保持现有 SVG 图标行内布局，避免 Tailwind Preflight 改变旧编辑页标题排版。
- `tests/projects.test.ts`：使用真实 Select DOM 和键盘事件验证排序，保留原有业务回归。
- `.gitignore`：忽略 Playwright 本地运行产物。
- `docs/PRD.md`、`docs/TECHNICAL-DESIGN.md`、`docs/DEVELOPMENT-PLAN.md`、本文：同步范围、实现和验收记录。

## 数据库与版本

数据库 schema v7 保持不变，没有迁移、Rust、IPC、权限或数据文件修改。浏览器验收数据只存在于测试页面内存，不访问正式 SQLite，不调用真实模型服务。

本轮属于样式与内部重构，默认 patch；未手动修改版本、提交代码或发布安装包。

## 实际命令与结果

| 命令 | 结果 |
| --- | --- |
| `npx vitest run tests/projects.test.ts` | 11/11 通过，包含搜索、筛选、键盘排序、新建、失败保稿、删除确认和备份确认 |
| `npm run lint` | 通过，零警告；没有关闭现有规则 |
| `npm test` | 65 项前端测试通过；发布脚本测试 27 通过、1 跳过，跳过项为 macOS ZIP 包结构的专属检查 |
| `npm run build` | TypeScript 检查及 Vite 生产构建通过，没有启用 skipLibCheck |
| `npm ls @vueuse/core` | 根依赖和 Reka 共用 14.4.0，无重复版本 |
| `git diff --check` | 通过 |

接入时发现并解决：初始化 CLI 的间接依赖要求比本机 Node 更新，改为官方源码导入；VueUse 15 与 Reka 14.x 重复并引入本项目不具备的 Temporal 类型，统一为 14.4.0；补充现有 VueUse 声明需要的 Web Bluetooth 类型；旧原生 select 测试改为真正操作新的下拉框，没有通过组件 emit 绕过交互。

## 浏览器验收

使用 `npm run dev` 和 Playwright CLI。先确认无桌面桥接的浏览器预览禁用本地操作，再通过 `output/playwright/setup-preview.js` 注入四本示例书籍和受限内存 IPC，检查实际 DOM 与样式。

实际运行的主要命令：

```powershell
npx --yes @playwright/cli -s=shuyin-ui open http://127.0.0.1:1420 --browser chrome
npx --yes @playwright/cli -s=shuyin-ui run-code --filename=output/playwright/setup-preview.js
npx --yes @playwright/cli -s=shuyin-ui snapshot
npx --yes @playwright/cli -s=shuyin-ui press ArrowDown
npx --yes @playwright/cli -s=shuyin-ui press Enter
npx --yes @playwright/cli -s=shuyin-ui resize 800 650
npx --yes @playwright/cli -s=shuyin-ui resize 360 800
npx --yes @playwright/cli -s=shuyin-ui screenshot --filename=output/playwright/shadcn-library-1440.png
npx --yes @playwright/cli -s=shuyin-ui console error
```

- 1440px：书架三列、状态标签、筛选和搜索区正常；空白预览与有数据状态分别检查。
- 800×650：书架无横向溢出，筛选栏自然换行。
- 360×800：侧栏收缩为图标，书架无横向溢出，数据与备份入口仍可用。
- 排序：打开下拉框后用方向键和 Enter 选择书名排序；焦点返回 `book-sort`；页面内存记录只含一次 `list_projects` 调用。
- 书籍表单：实际填写书名、作者、内容并点击保存；内存 IPC 接收保存，路由进入对应内容创作页。
- 1120px：检查内容创作、封面工坊、模型设置和发布准备，未出现横向溢出；封面 Canvas 正常渲染，没有页面错误；控制台 Errors 0 / Warnings 0。
- 截图位于 `output/playwright/shadcn-*.png`，已进行视觉检查。截图展示示例数据，不代表正式数据库内容。

## 验收标准对应

- [x] shadcn-vue 真实组件参与页面渲染，而非只添加依赖。
- [x] 书架搜索、筛选、计数、排序和刷新保留；排序具备键盘操作与焦点返回。
- [x] 新建书籍、保存失败保稿、未保存离页保护与删除确认回归通过。
- [x] 浏览器预览不直接访问本地数据库。
- [x] 清理被替代的侧栏和书架样式，新旧样式互不覆盖控件。
- [x] lint、类型检查、前端测试、生产构建完成。
- [ ] Windows Tauri 人工验收、安装包更新、macOS / Linux 运行验证：本轮未执行。

## 后续

先在 Windows 桌面开发版确认首批样式和操作，再按同一套组件依次迁移内容创作、封面工坊、模型设置和发布准备；迁移每页时保留其未保存保护和失败保稿语义。
