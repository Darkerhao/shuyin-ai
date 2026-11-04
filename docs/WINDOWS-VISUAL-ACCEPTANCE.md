# 视觉规范收尾与 Windows 验收

日期：2026-11-04。

## 结论

已修正视频进度区的浅色主题对比度与持续流光问题，完成本轮浏览器布局、Windows 三档真实系统缩放、隔离安装、同版覆盖安装/重启及原生另存验证。**原生跨窗口文件拖入手势仍待人工核对，不能将整套 Windows 验收标记为全部通过。** P3 逐字候选预览仍保持按需后续状态。

本报告是对 [原方案](UI-UX-REFINEMENT-PLAN.md)和[原验收记录](UI-UX-REFINEMENT-VERIFICATION.md)的补充。保留此前菜单问题的更正，不用局部修复或测试数量代替整体验收。

## 根因与最小修改

真实浏览器复现中，视频渲染区仍用旧深色配色的 `--render-ink/light/muted`，底色却引用已变浅的 `--sidebar`，并叠加局部深绿渐变，导致左侧进度、速度和步骤文字难以阅读。进度条伪元素仍循环执行 `render-scan`，与方案第12节不符。

- `VideoJobProgress.vue`：删除整套旧渲染颜色和流光伪元素，直接使用现有主题变量；使用浅灰实面、墨绿进度与清楚的正文/辅助文字，去掉装饰光晕，辅助说明调整为12px。
- `Progress.vue`：真实进度宽度过渡统一为180ms；进度环同为180ms。
- 保留真实百分比、四阶段、耗时、速度及15秒无反馈说明。不可量化任务的等待指示仍保留，减少动态效果时关闭动画；没有为样式增加状态、依赖或任务流程。

复现脚本修复前明确失败：`持续装饰流光仍在运行：render-scan-b84516d5`。修复后五档窗口全部通过，实际渲染区辅助文字最低对比度为4.662:1，背景无渐变、装饰动画为none；四个真实进度阶段、失败停止显示与减少动态效果检查通过。

## 验收对应

| 项目 | 本轮结果 |
| --- | --- |
| 全局视觉与窗口布局 | 12个页面状态 × 5档CSS窗口，共60组无整页横向溢出；普通 `.panel` 外框0px，页面异常0。覆盖书架、新建、资料、创作三页签、封面、视频、发布、模型、备份与关于 |
| 导航、书架与菜单 | 浏览器实际点击/键盘回归通过；安装版实际展开菜单白底、边界和定位正常，读取真实登记成片后可导出 |
| 文案、复制与保存 | 本轮浏览器重新验证备选保留当前稿、历史取用、精确复制、按钮宽度稳定及显式保存 |
| 封面与视频联动 | 本轮浏览器重新验证画布双击/Enter/Escape、同一播放器定位与跟随不抢焦点；安装版对应页面和既有草稿可恢复 |
| 发布自检 | 浏览器实际核对双封面、一次明确确认、生成任务包不自动记录发布；安装版读取原ready任务正常。未操作真实账号 |
| 动效、字体与对比 | 实际渲染区计算样式、五档窗口、减少动态效果与截图复核通过；不将此表述为全站WCAG认证 |
| Windows系统缩放 | 在Windows“系统→屏幕”实际选择100%、125%、150%；WebView2读取的devicePixelRatio分别为1、1.25、1.5。每档9页共27组无整页横向溢出/页面异常。最后恢复原150% |
| 原生成片另存 | Computer Use通过书架菜单打开实际“另存成片”对话框，输入本轮D盘路径并保存；导出9652字节，SHA-256与原件一致 |
| 安装与重启 | Release NSIS隔离安装及同版 `/UPDATE` 覆盖安装均退出0，安装目录位于D盘；正常关闭、重开后书籍、三份文案版本、三张封面、视频草稿、ready任务和侧栏偏好一致 |
| 原生文件拖入 | **待人工验证**。跨显示器尝试被工具以 `point ... is outside window bounds` 拒绝；同屏重叠窗口尝试中来源窗口被激活，未得到应用的drop结果，书籍数仍为1。没有将浏览器夹具或来源路径预填当作真实拖入通过 |

Windows缩放不是浏览器viewport或DPR模拟。主显示器实际分辨率2520×1680保持不变；只修改三档缩放并恢复。查看了安装版书架/菜单/另存实际窗口，以及封面、创作、发布、模型、视频和备份截图；焦点轮廓按既有键盘规则保留。

## 实际命令与证据

```powershell
npx vitest run tests/video-progress.test.ts tests/ui-refinement.test.ts tests/projects.test.ts tests/confirmation.test.ts --maxWorkers=2
npm run lint
. ./.tools/env.ps1
npm run desktop:build -- --config output/playwright/visual-completion/tauri.verify.conf.json
node output/playwright/visual-completion/native-scale.mjs 150
node output/playwright/visual-completion/native-scale.mjs 100
node output/playwright/visual-completion/native-scale.mjs 125
node output/playwright/visual-completion/native-lifecycle.mjs
node output/playwright/visual-completion/native-lifecycle.mjs verify
git diff --check
```

结果：4个测试文件58项通过；lint通过；NSIS构建通过，包含TypeScript和Vite生产构建。没有代码变更需要新增SQL迁移或重跑无关Rust单元测试；Rust编译已在Release构建中实际执行。

Playwright CLI使用既有 `D:/DevCache/npm-cache/_npx/31e32ef8478fbf80/node_modules/@playwright/cli/playwright-cli.js`，本轮会话 `visual-completion` / `visual-pages`：

```powershell
$cli = 'D:/DevCache/npm-cache/_npx/31e32ef8478fbf80/node_modules/@playwright/cli/playwright-cli.js'
node $cli -s=visual-completion run-code --filename output/playwright/visual-completion/progress-check.cjs --json
node $cli -s=visual-completion run-code --filename output/playwright/visual-completion/progress-final.cjs --json
node $cli -s=visual-pages run-code --filename output/playwright/visual-completion/setup.cjs --json
node $cli -s=visual-pages run-code --filename output/playwright/visual-completion/verify.cjs --json
```

检查CLI JSON中的 `isError` 和业务断言，不以进程退出0代替通过。`progress-check.cjs`在修复后将截图名称从before改为after；原失败证据另存，没有覆盖。完整输出位于忽略目录 `output/playwright/visual-completion/`：

- `progress-before.json/png`、`progress-after.json/png`、`progress-final.json/png`：真实组件的失败复现、修复与五档窗口/对比度/减少动态效果检查。
- `browser-pages.json`与页面PNG：60组布局和7组交互回归。
- `native-scale-100/125/150.json`与对应9页PNG：真实Windows缩放及安装版页面。
- `native-save.json`、`Windows另存成片.mp4`：原生另存及摘要一致。
- `desktop-release-build.log`、`Shuyin-Visual-Acceptance-setup.exe`：Release NSIS构建与隔离交付件。
- `lifecycle-baseline.json`、`lifecycle-after.json`：同版覆盖安装与重启前后业务记录比对。
- `integrity-final.json`：正常关闭后schema17、integrity=ok、外键问题0、书籍1本、发布记录0、运行视频0，三份原件摘要未变。

首次debug构建失败：用户正在运行的 `target/debug/shuyin-ai.exe` 被占用，Rust返回os error 5。保留该进程，改用既有release缓存构建成功；未把失败记为通过。隔离种子初次停用模型违反“默认模型须启用”约束，仅在测试库同时清除默认标记后完成。未改变生产约束。构建保留既有500kB块大小提示与npm用户配置提示。

## 文件、数据与安装边界

业务代码仅修改上述两个Vue组件；同步方案、产品、技术设计、开发计划及验收文档。无新增依赖、权限、迁移、Prompt或版本变更，schema仍为17，影响为style/fix（patch）。未暂存、提交、推送或发布。

测试安装名称为 `Shuyin Visual Acceptance`，identifier为 `com.shuyin.visual-acceptance-20261104`；不是正式的 `com.shuyin.desktop`。本轮从先前隔离验收库创建独立SQLite快照，关闭更新、停用本机契约模型；没有使用正式库、真实凭据、外部模型或账号。NSIS沿用当前平台安装逻辑及既有旧音频桥清理hook，安装目标仅为D盘本轮输出；未替换用户正式安装目录。SQLite按Tauri规则位于C盘LocalAppData，安装注册/快捷方式使用Windows标准位置，WebView缓存和主体文件位于D盘。

本轮验证的是独立新安装、同版本覆盖安装和正常重启，不是正式安装版从旧版本升级、签名/自动更新发布或卸载验收。macOS/Linux、真实外部AI效果及真实账号辅助发布未运行。本轮测试应用正常关闭，测试安装与证据保留；原用户运行中的开发应用没有停止。

## 剩余验收与P3门槛

唯一仍缺的本轮Windows原生手势是：在隔离测试版“我的书籍”页面，从资源管理器拖入 `drop-source/Windows拖入验收.md`，核对书名/来源后取消，确认没有新书；再拖入并保存，确认只创建一本且原件不变。多文件/不支持格式的服务与前端保护已有自动化覆盖，但不代替手势。

本报告记录时建议先补齐该人工结果，再判断是否需要P3。用户后续明确要求开始P3，现已按[P3验收](P3-STREAMING-PREVIEW-VERIFICATION.md)实施；原生拖入手势仍保持待人工验证，不补记为通过。
