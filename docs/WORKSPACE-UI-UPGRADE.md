# 工作台页面与交互升级验收

## 首次打开焦点边框修复（2026-10-29）

根因是路由监听立即执行，并在首次路由解析后再次把焦点移入整个 `main`，触发已有的 2px 深绿色焦点轮廓。`App.vue` 现在通过 Vue Router 的 `START_LOCATION` 区分初始导航，仅在已有页面之间发生路径变化时转移焦点；页面标题仍在首次加载更新，查询参数变化保留原控件焦点，跳到主要内容及键盘焦点提示继续有效。

修改文件为 `src/App.vue`、`tests/projects.test.ts` 和本报告。无样式、数据库、迁移、依赖或版本号变更，按 fix/patch 处理。

实际验证：

- `npx vitest run tests/projects.test.ts -t 'initial focus|search focus|skip link|exact workflow step'`：修改前 3 项首次焦点回归失败、3 项通过、22 项跳过，确认覆盖本次问题。
- `npx vitest run tests/projects.test.ts`：修复后 28 项通过，覆盖待解析的初始重定向、直接打开其他页面、挂载前已解析路由、查询更新、跳过导航和后续切页。
- `npm run lint`：通过，0 警告。
- `npm run build`：含 TypeScript 检查；初次因沙箱拒绝清理 `dist/assets` 失败，获准在受限环境外重跑后通过。保留已有主包超过 500 kB 的提示。
- `npm run dev -- --port 1420` + Playwright MCP：分别在独立新页面打开 `/`、`/#/projects`、`/#/settings/models`，主内容区均未获得焦点，计算样式 `outline-style: none`；Tab/Enter 跳过导航与键盘切页保留可见焦点，鼠标切页不显示外框。新页面用于区分真正启动与同文档 hash 导航，浏览器预览未连接真实数据库。
- `git diff --check`：通过。

首次打开不出现整页焦点框、后续键盘操作可用及搜索不丢焦点的验收均已通过。未构建或更新桌面安装包，未执行 Windows WebView2 人工验收；已安装版本需后续打包更新后生效。

日期：2026-10-28。

本文前半部分保留上轮书架与导航升级验收，其他页面的后续实现见文末“续轮页面升级”。

## 设计与实现

使用当前项目已安装的 ui-ux-pro-max、frontend-design，并读取 Vercel 官方最新 Web Interface Guidelines 审查本次改动。设计检索的营销页结构不适用桌面工作台，因此采用现有品牌、原生系统字体和工作流程作为页面依据。

配色：工作区 `#f5f6f4`、正文 `#202d28`、白色内容面板、主操作 `#285b46`、侧栏 `#173e33`、选中导航 `#d7e8ca`。书架标题使用系统宋体，控制与正文使用系统无衬线；不加载远程字体。大屏保留侧栏、工具栏、书架层级，小窗口收缩侧栏并允许筛选和页签换行，低高度隐藏辅助文案并允许侧栏滚动。

验收对应：

| 目标 | 实现与证据 |
| --- | --- |
| 连续创作 | 从现有项目推导最近更新、未归档且未发布的书籍；入口直达内容创作 |
| 查找与返回 | q/status/sort/view 查询参数作为唯一界面状态来源；搜索、筛选、排序、网格/列表可随历史返回恢复；非法枚举回默认 |
| 搜索与刷新分工 | 搜索实时过滤；Enter 不刷新数据库；刷新按钮保留失败前列表；Escape/清空按钮清除搜索 |
| 完整导航 | 发布任务独立主导航；书籍资料、内容创作、封面工坊、发布准备四个入口；精确步骤高亮，不冒充完成进度 |
| 键盘与反馈 | 实际页面标题、跳转后主要内容焦点、可见焦点、可访问按钮名称与按下状态；参数变化不抢焦点 |
| 空态闭环 | 发布任务空态可查看草稿、返回书籍；搜索无结果可直接清空；保留现有任务处理和最终人工发布边界 |
| 共享样式 | 面板、流程导航、页签、输入悬停、按钮显式过渡属性、粗指针触控尺寸、减少动态效果偏好 |

## 修改范围

- `src/App.vue`、`src/app/router/index.ts`：导航、页面标题、焦点与滚动。
- `src/components/ProjectNavigation.vue`、`src/components/AppIcon.vue`：四步流程及网格/列表图标。
- `src/components/ui/button/index.ts`、`src/style.css`：共享样式、显式过渡、布局与响应式。
- `src/modules/projects/ProjectsPage.vue`：最近创作、URL 状态、双视图、独立刷新。
- `src/modules/publishing/PublishQueuePage.vue`：搜索与可操作空态。
- `tests/projects.test.ts`、`tests/publish-queue.test.ts`：状态恢复、非法参数、精确导航、焦点与空态操作回归。
- `docs/PRD.md`、`docs/TECHNICAL-DESIGN.md`、`docs/DEVELOPMENT-PLAN.md` 与本报告：同步实际实现。

无数据库变更、SQL 迁移、Rust 修改、新依赖、手工版本号变更或提交。

## 命令与结果

| 实际执行 | 结果 |
| --- | --- |
| `npm run lint` | 通过，0 警告 |
| `npm run build` | 通过，含 vue-tsc 类型检查及 Vite 生产构建；JS 主包约 504 kB，存在超过 500 kB 的构建提示 |
| `npx vitest run` | 13 个测试文件、186 项通过 |
| `npm run test` | 前端 185 项通过（追加发布空态回归前），发布/音频脚本完成并进入队列测试；整体因队列测试失败退出，不能标记全绿 |
| `npm run test:queue` | 18 通过、1 失败；缺少前置报告时应重跑任务及后续项，实际只保留原两次执行。位置 `tests/codex-queue.test.mjs:118` |
| `git diff --check` | 通过 |

初次格式化和构建受沙箱写入限制，获准执行权限后成功完成。另一次临时禁用 core.fsmonitor 的发布脚本重试在等待期间中止，未将该次记为通过；未改动 Git 配置。Playwright CLI 获取受网络限制，改用现有 Playwright MCP 完成浏览器检查。

## 浏览器验证

通过 `npm run dev -- --port 1420` 启动本地页面，使用隔离内存 IPC 数据；不连接真实数据库、Google、抖音或模型。测试模拟包含窗口事件、自动草稿、音频任务快照及模型/更新状态；最初模拟缺失事件 API 导致错误提示，补齐模拟后重跑如下结果。

- 11 路由：书架、新书、书籍资料、内容创作、封面工坊、发布准备、发布任务、模型设置、数据备份、关于更新、技术验证。
- 1440、1120、800、360 px 共 44 组：无横向溢出、页面异常或错误浮层；最终加测各宽度的列表视图，共 48 组，无溢出或错误浮层。
- 实际点击：搜索余华、切换列表、打开书籍、确认四步骤及唯一高亮、返回书架恢复条件、Escape 清空、发布空态切换草稿均通过。
- 切页焦点进入 main；搜索/视图的查询更新不重新创建页面；减少动态效果媒体偏好已启用检查；360 px 列表无横向溢出。
- 人工查看了 1440 px 书架和 360 px 列表截图，修复 Tailwind 工具层覆盖列表布局及窄窗口最近创作区域挤压；最终读取计算样式确认列表真实使用两列布局，而不只检查 class。书架专用样式置于工具层之后，无重复实现。截图：`output/playwright/workspace-upgrade-1440.png`、`workspace-upgrade-list.png`、`workspace-upgrade-360.png`。这些文件位于本地忽略目录。

按最新 Web Interface Guidelines 审查本次改动：图标按钮有名称，表单有标签，新增搜索提供 name/type/autocomplete，流程入口使用真实链接，当前态不单靠颜色，焦点与减少动态效果可见，按钮不再使用 transition-all。该检查不等同于全站 WCAG 认证或屏幕阅读器人工验收。

## 剩余与下一步

本报告记录书架、发布任务及共享导航/样式这一轮的验收，不代表所有页面已逐页完成升级。完整剩余项见 [全页面升级缺口审查](UI-UPGRADE-AUDIT.md)。

本轮页面与导航验收已满足。队列脚本失败未修改，需要独立定位。未运行 Rust 检查、数据库迁移测试、Windows WebView2 人工桌面验收、安装包构建、真实模型调用或账号发布。建议下一步在 Windows 桌面版核对文件选择、离页保存和发布窗口切换；构建主包体积可作为独立性能任务评估。

## 续轮页面升级（2026-10-28）

沿用现有墨绿、灰白面板、系统字体与统一控件，以保护操作、返回恢复和主次层级为本轮验收目标；没有新增依赖或重复实现已有自动草稿、生成取消、裁切和发布步骤。

| 验收标准 | 实现 / 结果 |
| --- | --- |
| 取消不删除，键盘可进入和返回 | 新增共享 `ConfirmAction.vue`，个人样式及书籍/模型/素材删除、恢复、更新安装统一焦点与 Escape；样式取消时删除命令调用为 0 |
| 资料正常关闭有保护 | `useWindowCloseGuard.ts` 监听浏览器卸载与 Tauri 关闭；未保存确认取消及忙碌时阻止关闭。单测覆盖异步决定、监听清理与晚注册；真实关窗待验 |
| 刷新 / 返回定位当前任务 | 任务切换、新建和保存同步 `task`；刷新任务二仍显示任务二。`useQueryState.ts` 提取书架已有模式，复用于页签和队列筛选；单测覆盖返回及非法参数 |
| 音频入口及后续操作正确 | 发布页链接直达音频页签；时长摘要可展开，音频页进入时展开；音频页下一步进入发布准备 |
| 预览与编辑易达且无遮挡 | 大窗口吸顶避开顶栏并限制可用高度；窄窗口就地切换编辑 / 预览，预览标题避开切换栏；保留当前页签与编辑内容，底部操作保持吸底 |
| 缺失项可以补齐 | 已保存检查项提供字段焦点，封面/音频可直达制作页，素材可一键展开管理 |
| 备份及模型反馈明确 | 模型按读取 / 保存 / 连接测试 / 删除显示状态并记住所选模型；备份中保护离页与关闭，当前会话保留最近结果，完整备份主入口与高级数据库维护分层 |
| 素材查找与统一排版 | 名称 / 路径搜索、类型及文件状态筛选、清空空态；数据备份、关于与技术验证统一页头；封面字号和颜色提示改用共享语义色与可读字号 |

修改文件：

- 共享：`src/components/ConfirmAction.vue`、`src/composables/useQueryState.ts`、`src/composables/useWindowCloseGuard.ts`、`src/composables/useOperation.ts`、`src/style.css`。
- 书籍与创作：`ProjectsPage.vue`、`ProjectPage.vue`、`ProjectAssets.vue`、`WritingPage.vue`、`CoversPage.vue`。
- 发布：`PublishingPage.vue`、`PublishQueuePage.vue`。
- 设置：`ModelsPage.vue`、`SettingsPage.vue`、`workspaceOperation.ts`、`AboutPage.vue`；技术验证：`VerificationPage.vue`。
- 回归：`tests/projects.test.ts`、`ai.test.ts`、`covers.test.ts`、`publishing.test.ts`、`publish-queue.test.ts`、新增 `window-close.test.ts`。
- 文档：本报告、`UI-UPGRADE-AUDIT.md`、`TECHNICAL-DESIGN.md`。

实际验证：`npm run lint` 0 警告；`npm run typecheck` 通过；页面及窗口专项 153 项通过，完整 Vitest 199 项通过；`npm run build` 通过（保留主包超过 500 kB 提示）；`npm test` 首次整体因既有队列 1 项失败而退出，发布/音频脚本 65 项通过、1 项按平台跳过。具体复现与隔离浏览器证据见审查报告顶部。

获准执行权限后再次运行完整 `npm test`，仍为相同队列用例失败（18 通过 / 1 失败），前端与发布/音频脚本结果不变。最终 `npm run build` 通过，JS 主包 511.82 kB；`git diff --check` 通过。

无数据库变更、迁移脚本、Rust 修改、依赖或版本号调整，未提交。剩余是实际 Windows 关闭及原生文件/备份恢复验收、独立队列问题和可选素材页内预览；备份结果只保留于当前应用会话，不承诺重启恢复操作记录。
