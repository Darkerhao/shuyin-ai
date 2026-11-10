# 内容创作与作品交付台升级

日期：2026-11-10

## 范围与设计

本轮完成内容创作与发布准备，视频制作留到下一轮。参考用户提供的 [React Bits Pro](https://pro.reactbits.dev/) 公开 [Editor](https://pro.reactbits.dev/docs/app-ui/editor) 和 [Agent Activity](https://pro.reactbits.dev/docs/app-ui/agent-activity) 展示：采用正文与建议分区、紧凑真实阶段条；复用现有 Vue、AppIcon 和 CoverThumbnail，不引入 React 或其他依赖。

沿用现有主题变量：深蓝底 `#060911`、面板 `#10172a`、正文 `#f1f5f9`、冰青创作 `#00e5ff`、青绿发布 `#00ff9d`、琥珀封面 `#fbbf24`；浅色使用现有同义变量。字体继续为本地 Inter、系统中文字体与 JetBrains Mono，正文恢复常规无衬线，等宽字体用于计数和次要信息。标题20px、正文15px/1.9行高、辅助信息12px。

内容页以完整编辑面板为主体，宽窗口右侧展示候选，1199px及以下改为单列；按项与历史使用统一双栏对照，窄屏逐项纵向排列。发布页左侧文案与复制操作，右侧原声/成片和双封面；封面等高展示、保持图片比例，发布自检与实际结果记录独立成区。保持左对齐，避免把每个编辑字段拆成同等权重卡片。

候选只在面板出现时做180ms短过渡，不逐字模拟生成；辅助发布进入review时轻量呈现准备完成，减少动态效果时关闭。Vue Transition采用现有3.5.44实现的具名CSS类，不增加动画库。

## 实现与边界

| 区域 | 实现 | 保留的业务约束 |
| --- | --- | --- |
| 主编辑区 | 标题、正文、标签组成连续编辑面板；说明后置；显式显示当前稿来源与历史入口 | 原换行、长度、自动暂存、保存快捷键继续有效 |
| 生成候选 | 等待返回、正在接收、生成完成；已有未取用AI版本可进入对照 | 从原任务与版本列表派生，不增加持久化状态；片段不写历史；失败/停止不覆盖当前稿 |
| 按项调整 | 紧凑选择项、当前/候选双栏、候选下方采用操作 | 保留原保存后生成、完整校验、明确采用与放弃逻辑 |
| 历史 | 来源类型、时间、当前编辑来源；查看对照与整版/单项取用分开 | 浏览不写稿，单项取用后仍需保存 |
| 作品素材 | 原声试听、所选成片预览、真实视频元信息、横竖封面展示与放大 | 复用openAsset、videoAssetMetadata与CoverThumbnail；缺失文件不显示就绪 |
| 只读任务 | 显式禁用所有编辑控件；已发布作品仍能复制文字和放大封面 | 不因预览入口落在禁用fieldset中而无法查看；不解除任务编辑锁 |
| 自检与辅助发布 | SVG图标和文字状态、缺失项入口、真实已核对/进行中/待确认阶段 | completed数组决定完成标记；review仍为人工最终检查，打开网页不会记录发布成功 |
| 实际发布记录 | 独立结果区，保留原确认对话框 | 只有原人工确认记录流程可以表示已发布 |

数据库仍schema23，无迁移、Rust代码、Tauri权限、依赖或版本号变更。按`style`/patch评估，不提交、不推送、不打包发布。

修改文件：`src/modules/writing/WritingPage.vue`、`src/modules/publishing/PublishingPage.vue`、`src/modules/publishing/DouyinVerification.vue`、`src/style.css`；调整`tests/publishing.test.ts`，新增`tests/browser/creator-delivery.mjs`，同步PRD、技术设计与开发计划。

## 实际验证

- `npm run typecheck`：通过。
- `npx vitest run tests/ai.test.ts tests/publishing.test.ts tests/douyin.test.ts tests/generation.test.ts tests/drafts.test.ts`：首次161通过、1失败。原测试绑定外层fieldset结构，修改为逐一验证10个真实编辑控件禁用，并验证已发布任务仍可复制及放大；未弱化只读要求。
- `npx vitest run tests/publishing.test.ts`：40项通过。
- `npx vitest run tests/ai.test.ts tests/publishing.test.ts tests/douyin.test.ts tests/generation.test.ts tests/drafts.test.ts tests/theme.test.ts tests/ui-refinement.test.ts --maxWorkers=2`：7个文件176项通过。
- `npm run lint`：通过。
- `npm run build`：类型检查和生产构建通过，仍有既有单分包超过500kB的提示。
- `npm run dev -- --port 1422`：启动浏览器验收服务。
- Playwright MCP `browser_run_code_unsafe(filename: "output/playwright/creator-delivery-runner.js")`：56组布局与9类交互通过，pageerror为0、未知IPC为0。runner从浏览器脚本移除`export default`生成，未改变验收逻辑。
- `git diff --check`：通过。

浏览器使用隔离内存IPC、真实Vue/Canvas/Reka组件。覆盖深浅主题、1440/1120/800/390四档窗口、普通编辑/流式候选/历史/按项对照/交付/官方检查/缺失素材七类状态。验证当前稿保留、只读候选、历史单项取用、放弃按项候选、所选成片预览调用、封面Esc关闭与焦点返回、最终人工发布边界、减少动态效果及缺失素材阻止发布准备。

首次浏览器验收发现按项候选完成后，单列网格的自动最小宽度受历史下拉文本撑开；已将创作网格固定为`minmax(0, 1fr)`并限制历史面板最小宽度，56组复验通过。夹具命令名和任务浅响应更新也已修正；这些夹具问题不作为产品缺陷。

## 验收对应与下一步

- [x] 主编辑区层级清楚，当前稿与生成候选独立展示。
- [x] 等待/接收/完成来自实际任务状态与内容，停止/失败保持原稿。
- [x] 按项调整与历史复用现有明确取用逻辑，来源与时间可见。
- [x] 文案、原声、成片与横竖封面集中在交付视图，封面可放大。
- [x] 自检、官方页面检查和实际发布记录明确区分。
- [x] 深浅主题与窄屏适配，减少动态效果有效。
- [ ] Windows WebView2实机、系统缩放、外部真实模型和真实抖音账号人工验收。

截图位于本地忽略目录`output/playwright/delivery-*.png`，分别保留两种主题；参考图为`reactbits-editor-reference.png`与`reactbits-activity-reference.png`。未运行Rust、SQL迁移、视频渲染和安装包检查，本轮未修改这些执行层。建议先在桌面版用实际书籍确认两页阅读及素材查看，再按原顺序推进视频制作页。

## 追加评审与修复（2026-11-10）

按用户要求对本轮未提交改动进行独立Standards/Spec双轴评审，固定基线为`e22c18325ef97f2e0663b3efda5f6ebe34c9d645`，包括新增浏览器脚本与本文档。没有新提交或外部发布。

### Standards

初评1项P2：封面主、副标题仍有绝对定位的字数计数，但本轮删除了短字段的窄屏右留白。浏览器实际测得右留白16px、计数所需64.4px，长文本编辑进入计数区域。只为`coverTitle`/`coverSubtitle`增加`pr-20`，不改变业务数据和其他字段。独立代码复审已通过，剩余确定问题0项。

### Spec

初评1项P2：同一封面标题问题违反窄屏可读性要求，原56组检查未切换封面素材页签。浏览器脚本新增两种主题、四档窗口的封面字段检查，直接对比输入框右留白与计数占位。修复前断言失败，修复后通过；最终覆盖扩展为64组。

独立Spec代码复审确认封面留白与mixed来源说明均符合实际行为，剩余确定问题0项。Standards与Spec各自初评1项P2（同一问题），两轴修复后均通过；主线程补充的来源误标也已修复并回归。

### 主线程补充问题

另发现1项P2来源误标：`database/ai.rs::save_writing`对手工修改AI版本也写入`sourceType=mixed`，不能据此声称运行了「按项调整」。新回归先复现手工保存被显示成按项调整，随后将统一显示文案改为「AI 生成后修改」，同时验证当前稿来源和历史来源，不新增状态、字段或迁移。

本次修复文件为`WritingPage.vue`、`tests/ai.test.ts`、`tests/browser/creator-delivery.mjs`以及验收/技术设计/开发计划文档。原双页改造的其他文件保留。

### 最终实际验证

| 命令或调用 | 结果 |
| --- | --- |
| `npx vitest run --maxWorkers=2` | 初评前34文件474项通过；修复后34文件475项全部通过 |
| `npx vitest run tests/ai.test.ts -t 'describes a manually edited AI version' --maxWorkers=2` | 修复前按预期失败，证实来源误标；后续修正新增测试的历史按钮选择器，最终全量覆盖通过 |
| Playwright `browser_run_code_unsafe(filename: "output/playwright/creator-delivery-runner.js")` | 新增留白检查修复前失败；修复后64组布局、10类交互全部通过，pageerror与未知IPC均为0 |
| `npm run lint` | 修复后通过 |
| `npm run build` | 修复后TypeScript及生产构建通过，保留既有单分包超过500kB提示 |
| `git diff --check` | 通过 |

无数据库变更、新依赖、权限或版本号变更。上述浏览器仍是隔离内存IPC验收，不表示Windows WebView2、真实模型、真实抖音账号或安装包通过。下一步建议在桌面版用真实书籍确认阅读与素材预览，再继续视频制作页。
