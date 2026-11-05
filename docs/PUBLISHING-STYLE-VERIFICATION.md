# 发布准备设计稿实现与验收

日期：2026-11-05。参考：用户提供的 `D:/谷歌浏览器下载/工作账号下载/ai (2)-发布准备.html`。

## 方案与验收标准

在现有发布准备页内重排布局，复用统一主题、共享控件及本地字体。背景 `#060911`，玻璃面板使用现有 `--panel-surface`，边界 `#1E293B`，青绿 `#00FF9D`、冰青 `#00E5FF`、琥珀 `#FBBF24` 分别强调发布操作、来源文案与封面。标题采用Inter/系统中文字体，控件和元信息采用JetBrains Mono/系统中文字体；面板24px内边距、16px圆角、区块24px间距。

```text
返回我的书籍                         发布任务 / 交付提示
准备好，分享你的作品                   Step 5 · 交付
书籍资料 → 内容创作 → 封面工坊 → 视频制作 → 发布准备
本书发布任务：选择、真实状态、新建与刷新
┌ 发布文字                     ┬ 音频、成片与封面          ┐
│ 文案来源、标题、内容和标签    │ 原声、成片、路径及双封面  │
└──────────────────────────────┴──────────────────────────┘
复制发布文字：标题 / 内容 / 标签 / 全部
发布前自检：当前摘要、真实双封面、明确确认操作
辅助发布进度（有会话、操作或错误时显示）
已保存任务检查、任务包、文件入口、人工发布结果记录
```

按设计稿呈现分区、间距、色彩、控件和封面比例；任务、字数、文件路径和发布成功均来自当前业务数据。参考稿的固定记录、演示按钮和外部图片不写入用户数据。初始素材管理收起，点击原按钮打开；辅助发布保持原订阅，不因空闲时隐藏而漏掉恢复会话或错误。

验收标准：宽窗口双栏、窄窗口单栏；文案/素材/菜单不横向溢出；真实双封面与元信息可核对；复制、保存、来源替换、素材管理、检查定位、导出及辅助发布仍可使用；缺失素材或文件读取失败保留当前任务；已发布任务只读，记录须明确确认；键盘关闭菜单后焦点返回，减少动态效果有效。

## 修改范围与版本影响

- `src/modules/publishing/PublishingPage.vue`：页面布局、计数、真实检查摘要、封面预览及局部样式；继续使用原任务、草稿和确认方法。
- `src/modules/publishing/DouyinVerification.vue`：内嵌空闲时隐藏，恢复会话、开始、失败均保留显示。
- `src/App.vue`、`src/style.css`：补齐同一本书的发布准备导航；调整顶部快捷导航断点，清理原重复隐藏规则。
- `tests/publishing.test.ts`、`tests/douyin.test.ts`：新增实时字数、未保存与空检查提示、隐藏后恢复会话及读取错误回归。
- PRD、技术设计、开发计划和使用手册同步；截图在 `docs/assets/publishing-style/`。

发布影响为 `style(publishing)` / patch。无数据库表、列、索引或迁移，schema17不变；未修改Rust、Provider、Prompt、依赖、权限或版本号。原始音视频不复制、不改写；任务包和人工发布记录沿用原接口。

## 实际验证

| 实际命令 | 结果 |
| --- | --- |
| `npm run lint` | 通过，ESLint 0警告 |
| `npm run build` | TypeScript检查及Vite生产构建通过，856 modules |
| `npx vitest run tests/publishing.test.ts tests/publish-queue.test.ts tests/douyin.test.ts tests/ui-refinement.test.ts tests/confirmation.test.ts --maxWorkers=2` | 5文件64项全部通过 |
| `npm run dev -- --port 18572 --strictPort` | 启动设计稿与源码预览 |
| `npm run preview -- --port 18573 --strictPort` | 启动本轮生产构建预览 |
| 下方浏览器脚本 | 布局与交互检查通过，页面异常0 |
| `git diff --check` | 提交前检查无空白错误 |

浏览器使用本地Chromium与独立内存Tauri IPC夹具，封面由本地Canvas生成；验证真实Vue页面及组件，所有文件、任务和记录调用均仅在内存模拟，不接触正式SQLite或抖音账号。命令如下：

```powershell
node D:/DevCache/npm-cache/_npx/31e32ef8478fbf80/node_modules/@playwright/cli/playwright-cli.js -s=publish-style run-code --filename output/playwright/publishing-style/setup.cjs --raw
node D:/DevCache/npm-cache/_npx/31e32ef8478fbf80/node_modules/@playwright/cli/playwright-cli.js -s=publish-style run-code --filename output/playwright/publishing-style/check-layout.cjs --raw
node D:/DevCache/npm-cache/_npx/31e32ef8478fbf80/node_modules/@playwright/cli/playwright-cli.js -s=publish-style run-code --filename output/playwright/publishing-style/check-actions.cjs --raw
```

| 验收内容 | 实际结果 |
| --- | --- |
| 待发布、已发布、空素材、缺失音视频、封面读取失败 × 1440、1366、1280、1120、800、375、900px | 35组通过；页面、工作区和顶栏无横向溢出 |
| 任务长选项与键盘 | 菜单位于窗口内；Escape关闭后焦点返回任务选择器 |
| 字数、文字编辑与保存 | 实时计数、未保存提示、当前草稿和保存后的任务名称同步通过 |
| 复制与素材入口 | 全部中文及换行复制准确；素材管理展开、收起、再次打开通过 |
| 导出与本地入口 | 明确确认后调用原任务包接口；试听、定位、任务包和平台入口参数正确；不自动记录发布 |
| 辅助发布 | 空闲时隐藏，明确开始后显示；启动一次；单测另覆盖恢复等待会话及读取错误 |
| 发布记录 | 取消不写入，确认保存后切为已发布只读；375px弹窗保持窗口内且可关闭 |
| 缺失与失败 | 缺失素材禁用开始并定位补齐字段；封面不可读时阻止保存/导出并保留当前任务 |
| 减少动态效果 | 按钮过渡为0秒 |

初轮布局检查发现1120px时新增顶部入口撑宽工作区，已按设计稿的大屏导航断点修复，并增加1280px与工作区/顶栏边界断言。没有放宽断言或屏蔽错误。生产构建保留既有大于500kB的分块提示，未为样式任务调整打包策略。

## 截图与人工验收边界

- [已发布任务完整页面](assets/publishing-style/overview.png)
- [窄窗口页面](assets/publishing-style/mobile.png)

详细脚本和浏览器报告保存在忽略目录 `output/playwright/publishing-style/`；最终生产构建对应 `layout-production.json`（35组布局）与 `actions-production.json`（7组操作），页面异常均为0。未执行Windows安装包构建、原生WebView2实机验收、真实文件打开/上传、抖音最终发布或正式数据库操作，浏览器中的模拟记录不代表真实作品已发布。无未解决的本次前端检查失败。

下一步在包含本次提交的Windows桌面应用中打开一本现有书籍，核对系统缩放、真实封面及素材路径，按原流程检查并亲自发布。
