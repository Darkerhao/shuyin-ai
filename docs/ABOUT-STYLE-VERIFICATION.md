# 关于与更新设计稿实现与验收

日期：2026-11-05。参考：用户提供的 `D:/谷歌浏览器下载/工作账号下载/ai (2)-关于与更新.html`，其中 `view-about` 为本次实现依据。

## 方案与验收标准

在原 `AboutPage.vue` 内完成样式重构，复用现有应用框架、共享控件和唯一更新状态。内容限宽960px居中，卡片间距24px、圆角16px，版本卡内边距28px、其余卡片24px。使用背景 `#060911`、玻璃面板 `--panel-surface`、边框 `#1E293B`、青绿 `#00FF9D`、冰青 `#00E5FF`、次要文字 `#94A3B8`，沿用本地 Inter、JetBrains Mono 与系统中文字体。

```text
应用与版本
关于书引 AI / 创作说明
┌ 品牌符号、名称、桌面标记、真实版本       检查更新 ┐
│ 自动检查并下载更新开关 / 安装前确认说明          │
└───────────────────────────────────────────────┘
┌ 应用更新通道                         真实状态 ┐
│ 安装方式说明 / 上次检查时间 / 支持平台         │
│ 按状态展示下载进度、重试、安装或重启操作       │
└──────────────────────────────────────────────┘
┌ 更新内容                      对应版本的说明 ┐
│ 终端式日志 / 彩色提交类型 / 缺失说明提示       │
└──────────────────────────────────────────────┘
```

验收要求：对齐设计稿的三卡布局、品牌图形、开关、间距、色彩和日志层级；真实版本、时间和进度不写死；更新设置失败保留原值；下载总量未知时不虚构百分比；安装须明确确认，取消不安装；窄窗口、长说明和确认弹窗无横向溢出，键盘操作和减少动态效果可用。

参考稿的 `DESKTOP PRO` 和 `Core Engine Stable` 分别改为描述性的 `DESKTOP`、桌面创作工作台，避免宣称产品存在未实现的Pro版本或未经检测的稳定状态。平台标签补全项目支持的macOS Intel，不表示当前设备检测结果。示例版本、时间、更新日志和模拟成功脚本不进入生产业务；现有应用框架继续使用已实现的导航。没有新增依赖或在线字体。

## 修改文件与数据范围

- `src/modules/settings/AboutPage.vue`：三卡结构、局部样式、滑动开关、真实状态展示、日志文本分段与标签；移除旧Checkbox和重复日志展示分支。
- `tests/updates.test.ts`：补充日志标签与文本安全、空说明、开关保存失败/忙碌、未知下载总量、浏览器预览禁用回归，共16项更新相关测试；共用页面挂载入口。
- `docs/PRD.md`、`docs/TECHNICAL-DESIGN.md`、`docs/DEVELOPMENT-PLAN.md`、`docs/USER-GUIDE.md`及本文：同步需求、实现、验收与使用说明。
- `docs/assets/about-style/overview.png`、`docs/assets/about-style/narrow.png`：最终生产页面的宽/窄窗口截图。

版本影响为 `style(about)` / patch。没有新增数据库表、列、索引或迁移，schema17及版本号0.50.0不变。没有修改Rust、更新服务、签名校验、全局通知、原安装确认流程、权限或发布配置。设置仍通过现有 `setAutomatic` 保存，不维护第二份开关状态；日志只使用Vue文本插值，绝不执行远端HTML。

## 实际命令及结果

| 实际命令 | 结果 |
| --- | --- |
| `npm run lint` | 通过，ESLint 0警告 |
| `npm run build` | TypeScript检查与Vite生产构建通过，856 modules |
| `npx vitest run tests/updates.test.ts tests/notifications.test.ts tests/confirmation.test.ts --maxWorkers=2` | 3文件24项全部通过 |
| `npm run preview -- --port 18606 --strictPort` | 启动生产构建预览，供以下浏览器检查 |
| 以下Playwright CLI脚本 | 77组布局与11类操作检查通过，页面异常0 |
| `git diff --check` | 通过，无空白错误 |

浏览器使用本地Chromium、真实生产Vue页面及独立内存Tauri IPC模拟，检查/下载/安装/重启和设置命令均在内存响应，不连接真实更新服务、不操作正式SQLite、不安装任何更新。执行命令：

```powershell
node D:/DevCache/npm-cache/_npx/31e32ef8478fbf80/node_modules/@playwright/cli/playwright-cli.js -s=about-style run-code --filename output/playwright/about-style/setup.cjs
node D:/DevCache/npm-cache/_npx/31e32ef8478fbf80/node_modules/@playwright/cli/playwright-cli.js -s=about-style run-code --filename output/playwright/about-style/check-layout.cjs --raw
node D:/DevCache/npm-cache/_npx/31e32ef8478fbf80/node_modules/@playwright/cli/playwright-cli.js -s=about-style run-code --filename output/playwright/about-style/check-actions.cjs --raw
node D:/DevCache/npm-cache/_npx/31e32ef8478fbf80/node_modules/@playwright/cli/playwright-cli.js -s=about-style run-code --filename output/playwright/about-style/capture.cjs --raw
```

| 验收内容 | 实际结果 |
| --- | --- |
| 已是最新、手动模式、检查中、下载中、未知下载总量、待安装、检查失败、设置读取失败、空说明、超长说明、浏览器预览 | 各在1440×900、1366×768、1120×800、800×900、560×900、375×900、900×500核对，共77组通过；三张卡片、工作区和顶栏无横向溢出 |
| 开关键盘/标签 | 空格切换、点击文字标签切换通过；保存中禁用，保存失败保留原值，重试后更新 |
| 手动检查与下载 | 手动检查不自动下载；网络失败可重试；签名错误模拟保留重试入口，重试后进入待安装 |
| 下载反馈 | 已知总量显示42%；未知总量无aria-valuenow，显示已下载42.0 MB |
| 更新日志 | 当前/候选版本正确，缺失和纯空白说明有提示，长行换行；脚本文本原样展示，不产生脚本节点 |
| 安装确认 | 375px确认弹窗位于视口内，初始聚焦取消，Escape关闭且焦点返回；取消不安装，明确确认后才调用安装 |
| 安装与重启状态 | 安装中禁用设置、阻止离页；重启失败后再次确认重启，不重复安装 |
| 设置读取失败 | 开关禁用，重新读取后恢复操作 |
| 减少动态效果 | 检查图标animation为none，开关过渡为0秒 |

浏览器检查发现空字符串说明被分成一条空日志，已在分段前判断空内容，新增空字符串和纯空白两项回归后全部通过。初轮布局探针误将进度条被裁切的动画部分计为溢出，改为测量祖先裁切后的可见边界，仍保留页面与工作区scrollWidth检查。最终截图采用实际视口捕获，避免长页截图中玻璃面板的离屏合成缺块，窄窗口日志也已滚动到可见区域目视核对。

生产构建保留既有的大于500kB分块提示；npm提示用户级python/msvs_version配置将弃用，均不影响本轮检查通过。未为样式任务调整打包策略或系统配置。

## 截图、边界与下一步

- [宽窗口页面](assets/about-style/overview.png)
- [窄窗口页面](assets/about-style/narrow.png)

截图中的0.50.0当前版本、0.50.1候选版本和更新说明来自隔离测试数据，仅用于验收界面，不表示已经发布或安装0.50.1。完整脚本、参考稿截图与报告保存在忽略目录 `output/playwright/about-style/`；最终报告为 `layout-result.json`、`actions-result.json`。

本轮验收标准在上述前端与模拟交互范围内均满足，没有未解决的本次检查失败。未执行Windows原生WebView2人工验收、真实远端下载/签名校验、安装器构建或实际安装重启。下一步在包含本次提交的Windows桌面应用核对系统缩放与真实版本展示，并按原确认流程验收更新。
