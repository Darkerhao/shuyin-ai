# 浅色风格实现与验收

日期：2026-11-05。参考：`D:/谷歌浏览器下载/工作账号下载/ai (2)-浅色风格.html` 的浅色模式。

## 方案与验收标准

原页面和组件共用一套主题变量，不建立第二套布局或业务状态。浅灰网格背景 `#F8FAFC`、白色卡片 `#FFFFFF`、分隔线 `#E2E8F0`、正文 `#0F172A`、青绿强调 `#0F6B63`、蓝色辅助 `#0369A1`；琥珀与紫色分别标识封面和视频模块。相较参考稿，加深小字号强调文字以提高可读性，白字主按钮使用深青绿至蓝色渐变。

保留本地Inter/JetBrains Mono、系统中文字体、原页面间距和组件。浅色使用柔和投影、白色半透明面板和36px网格；深色延续原科技工作台。

验收要求：右上角可用鼠标及键盘切换；首次默认深色、后续恢复本机选择；各页面、菜单、确认框、通知和播放器默认配色一致；存储失败保留原主题并反馈；切换不重建编辑器、不触发业务保存、不丢输入或音频播放位置；封面Canvas内容不变；宽窄窗口无横向溢出，通知不挡按钮，减少动态效果可用。

## 实现与修改文件

- `src/composables/useTheme.ts`、`src/main.ts`：唯一状态、挂载前恢复、切换及存储失败提示。
- `src/App.vue`、`src/components/AppIcon.vue`：顶栏按钮、太阳/月亮图标、可访问名称和错误反馈。
- `src/style.css`：共享深浅颜色、阴影、品牌、背景、渐变按钮、原生控件主题和响应式入口。
- `src/components/{AppNotifications,AudioPlayer,BookCard}.vue`：通知主题和顶部间距、默认播放器主题与OLED隔离、书封和界面状态颜色分离。
- `src/modules/projects/ProjectsPage.vue`、`writing/WritingPage.vue`、`covers/CoversPage.vue`、`publishing/{PublishQueuePage,PublishingPage}.vue`、`settings/{AboutPage,ModelDiagnostics,ModelsPage,SettingsPage}.vue`、`video/{VideoJobProgress,VideoPage,VideoSceneEditor,VideoSubtitleEditor}.vue`：替换局部深色硬编码；媒体自身保留独立色彩。
- `tests/theme.test.ts`、`tests/ui-refinement.test.ts`：恢复、非法值、读写失败、重试、未保存输入与通知联动。
- PRD、技术设计、数据库说明、开发计划、使用手册、本文及截图。

无数据库结构变更，schema17不变；无新增依赖、IPC或权限。主题保存到当前应用WebView的 `localStorage.shuyin.theme`，不进入SQLite或资料备份。清除WebView数据后可重新选择，业务数据仍以SQLite为准。

新增兼容的用户主题功能，发布影响为 `feat` / minor；版本号仍为0.50.0，不手工升版。按用户要求，以 `feat(ui): 新增浅色主题并支持明暗风格切换` 本地提交代码、测试、文档和截图，不推送或发布。

## 实际验证命令与结果

| 命令 | 结果 |
| --- | --- |
| `npm run lint` | 通过，ESLint 0警告 |
| `npm run build` | TypeScript及Vite生产构建通过，857 modules |
| `npx vitest run tests/theme.test.ts tests/ui-refinement.test.ts tests/notifications.test.ts tests/confirmation.test.ts tests/audio-player.test.ts tests/updates.test.ts --maxWorkers=2` | 6文件45项通过 |
| `npm run preview -- --port 18616 --strictPort` | 生产预览启动成功 |
| 下述Playwright CLI脚本 | 154组布局、9组主要交互、6组补充检查通过 |
| `git -c core.safecrlf=false diff --check` | 通过 |

浏览器使用已安装的Playwright CLI、Chromium、真实生产Vue页面及隔离内存Tauri IPC。示例书籍、模型、任务、封面和8秒音频均为验收数据，不连接真实模型、不操作正式数据库。通知由模拟更新检查失败触发，不代表真实更新服务失败。

```powershell
node D:/DevCache/npm-cache/_npx/31e32ef8478fbf80/node_modules/@playwright/cli/playwright-cli.js -s=light-theme run-code --filename output/playwright/light-theme/setup.cjs
node D:/DevCache/npm-cache/_npx/31e32ef8478fbf80/node_modules/@playwright/cli/playwright-cli.js -s=light-theme run-code --filename output/playwright/light-theme/layout.cjs --raw
node D:/DevCache/npm-cache/_npx/31e32ef8478fbf80/node_modules/@playwright/cli/playwright-cli.js -s=light-theme goto 'http://127.0.0.1:18616/#/projects?dark-check'
node D:/DevCache/npm-cache/_npx/31e32ef8478fbf80/node_modules/@playwright/cli/playwright-cli.js -s=light-theme run-code --filename output/playwright/light-theme/layout.cjs --raw
node D:/DevCache/npm-cache/_npx/31e32ef8478fbf80/node_modules/@playwright/cli/playwright-cli.js -s=light-theme run-code --filename output/playwright/light-theme/actions.cjs --raw
node D:/DevCache/npm-cache/_npx/31e32ef8478fbf80/node_modules/@playwright/cli/playwright-cli.js -s=light-theme run-code --filename output/playwright/light-theme/details.cjs --raw
```

| 验收项 | 证据 |
| --- | --- |
| 双主题布局 | 11页面×7宽度×2主题=154组；宽度1920/1440/1366/1120/800/560/360px，高度900px；页面和内容区无横向溢出，按钮可见，页面异常0 |
| 恢复与失败 | 浏览器明暗各刷新恢复；单测覆盖无值、未知值、读取失败、写入失败保留旧值及重试 |
| 未保存内容 | 输入标题后键盘切换，输入节点、文字、焦点保留，未调用业务保存 |
| 浮层 | 浅色下拉、右键菜单和确认框为浅底深字；取消删除后示例书籍数量不变 |
| 通知 | 跟随主题；桌面与360px窄窗口均从80px处显示，不遮挡64px高顶栏里的切换按钮 |
| 媒体 | 切换前后封面Canvas数据一致；播放中切换不暂停或重置；默认播放器跟随主题，OLED保持黑底白字 |
| 对比度 | 8组正文/状态颜色在浅灰基底及15%同色背景均≥4.5:1，最低分别5.67:1和4.56:1；这是语义颜色检查，不代表全应用WCAG审计 |
| 减少动态 | `prefers-reduced-motion: reduce` 下主题按钮过渡0秒 |

检查发现通知默认24px顶部间距遮挡主题按钮，改为80px后通过。青绿和琥珀文字在着色底上的对比度不足，已加深并调整视频完成/错误底色。首轮音频夹具路径错误，路由到本地测试WAV后播放及波形检查通过。截图等待滚动合成完成，避免将瞬间离屏玻璃面板缺块作为最终结果。

构建仍有已有的大于500kB分块提示，npm仍提示用户级python/msvs_version配置弃用；不影响检查通过，未调整打包策略或系统配置。

## 截图、边界与下一步

- [浅色书架](assets/light-theme/overview.png)
- [浅色创作](assets/light-theme/writing.png)
- [窄窗口](assets/light-theme/narrow.png)

其他截图、脚本及报告保存在忽略目录 `output/playwright/light-theme/`，报告为 `light-layout.json`、`dark-layout.json`、`actions-result.json`、`details-result.json`。未运行Rust检查（无Rust变更）、Windows原生WebView2人工验收、安装器构建或升级。本轮前端验收标准已完成；下一步在包含此源码的Windows桌面版核对系统缩放、关闭重开后的主题恢复和原生文件选择窗口。
