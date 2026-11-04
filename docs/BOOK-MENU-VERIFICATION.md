# 书架菜单视觉问题复核

日期：2026-11-04。

## 结论与范围

用户截图中的菜单透明叠字问题可稳定复现，已在当前源码中修复；按钮菜单改为贴合更多按钮，右键菜单按点击位置展开。开发服务和最终生产构建页面均通过本页记录的检查。本次不重新宣称整套P0–P2已完成视觉验收，也没有更新安装版。

上一轮报告的菜单检查只验证了打开、关闭与焦点，未检查展开后的背景、边界与排版，因此没有发现实际视觉缺陷。已在原验收报告开头更正完成结论。

## 根因与修复

- 原真实DOM的菜单外壳有`book-context-menu`类，但没有当前组件的`data-v-*`属性；其子菜单项有该属性。编译后的`.book-context-menu[data-v-*]`无法命中外壳，实际背景为`rgba(0, 0, 0, 0)`、边框/圆角/内边距为0、阴影为none。主题`--card`实际为白色，排除了变量缺失；正文实际使用Microsoft YaHei，无字体修复。
- 原ContextMenuTrigger包住整张article，直接打开受控菜单时按整张卡片定位，More按钮没有成为锚点。
- 使用现有Reka UI 2.11.0的DropdownMenu，不新增依赖。原来的菜单实现已替换，只保留一套项目操作；More通过真实DropdownMenuTrigger定位，右键通过公开`reference`提供点击坐标，统一碰撞避让12px。
- Content使用`as-child`，将真实菜单外壳渲染为本组件的div，让scoped样式实际命中。菜单宽256px，按窗口限制宽高，白底、1px边界、10px圆角、6px内边距与柔和投影；沿用主题色和原有操作服务。
- 保留原More按钮ID供删除确认返回焦点，显式关联菜单的`aria-labelledby`；删除使用垃圾桶图标。搜索占位缩短为“书名、作者或 ISBN”，完整可访问名称与搜索范围不变。

已核对[Reka Dropdown Menu文档](https://reka-ui.com/docs/components/dropdown-menu)、[Vue Teleport文档](https://vuejs.org/api/built-in-components.html#teleport)及本地当前版本ContextMenuTrigger、DropdownMenuTrigger/Content、MenuContent和PopperContent源码。定位使用公开API，没有合成点击事件或手写第二套菜单键盘逻辑。

## 实际检查

| 命令或操作 | 结果 |
| --- | --- |
| `npm run lint` | 通过，0错误/警告，未关闭规则 |
| `npx vitest run tests/ui-refinement.test.ts tests/projects.test.ts` | 2个测试文件，39项通过 |
| `npm run build` | TypeScript与Vite生产构建通过 |
| Playwright CLI执行`tests/browser/book-menu.mjs`的函数，开发服务18590 | 五档窗口、菜单外观及真实交互通过，页面异常0 |
| 同一浏览器回归，最终dist预览服务18591 | 同样通过，证明生产打包后的作用域样式也生效 |
| 最终dist页面补查排序Select、导航Tooltip、删除确认 | 背景不透明、边框及圆角实际生效 |
| Computer Use查看当前Windows开发运行版（1420） | 真实14本书架中展开《文城》菜单，白底/边界/位置正常；实际成片项加载、Shift+F10与Escape开关通过 |
| `git diff --check` | 通过 |

浏览器尺寸为1440×900、1366×768、1120×900、800×1000与360×900。逐项断言白底、边界、圆角、阴影、内边距、按钮右对齐、上下避让、可视范围及无内部横向溢出。空间不足时向上展开是正常避让，不强制让菜单伸出窗口。

交互覆盖More、Shift+F10、独立菜单键、Enter/Space/ArrowDown、Home与方向键导航、Escape返回原发起焦点、右键位置及边缘避让、目录命令对应书籍、取消删除、点击外部关闭、列表视图、长书名、无成片时禁用项和菜单可访问名称。Tauri IPC使用内存夹具，Vue组件、Reka UI、计算样式及构建产物均为真实实现。

浏览器回归先在旧实现运行，确实以`Transparent menu at 1440`失败；修复后在开发与生产页面通过。Playwright CLI的JSON结果以`isError`表示脚本失败，检查JSON结果，不以CLI退出0代替通过。

实际CLI位置与复跑方式：

```powershell
$menuCli = 'D:/DevCache/npm-cache/_npx/31e32ef8478fbf80/node_modules/@playwright/cli/playwright-cli.js'
# 先启动 npm run dev -- --port 18590，或 npm run preview -- --port 18591。
node $menuCli -s=menu-built open http://127.0.0.1:18591
# 将标准ES模块导出的函数转换为CLI接受的函数表达式，测试逻辑仍以仓库文件为准。
(Get-Content -LiteralPath tests/browser/book-menu.mjs -Raw).Replace('export default ', '') |
  Set-Content -LiteralPath output/playwright/menu-repair/run-regression.cjs -Encoding utf8
node $menuCli -s=menu-built run-code --filename output/playwright/menu-repair/run-regression.cjs --json
```

证据保存在忽略目录`output/playwright/menu-repair/`：

- `before.json`、`regression-before.json`：旧实现透明菜单的实际失败结果；`before.png`：修复前截图。
- `regression-after.json`、`regression-built.json`：开发/生产页面的五档计算样式及交互结果。
- `after-1440.png`、`after-1366.png`、`after-360.png`：本次逐张查看的展开菜单截图。
- `after-detail.png`：修复后局部；`after-built.png`：生产构建、与修复前同一示例数据的截图，已查看。
- `adjacent.json`：Select、Tooltip与删除确认的实际计算样式。

## 文件、数据与边界

修改`src/components/BookCard.vue`、`src/components/AppIcon.vue`、`src/modules/projects/ProjectsPage.vue`；新增可复跑的`tests/browser/book-menu.mjs`。技术设计、开发计划与原验收报告同步本次修复和结论更正。

数据库仍为v17，无迁移、权限、依赖或业务服务改动；没有提交、推送或发布，保留此前已暂存的其他工作。构建仍有既有的500kB块大小提示，npm仍提示既有python/msvs_version用户配置；未调整阈值或用户配置。

本轮额外使用Computer Use查看用户正在运行的Windows开发版，窗口标题为“书引 AI · 读书创作工作台”，实际WebView来源为`http://127.0.0.1:1420/#/projects`。先恢复已最小化窗口，再从真实14本书架打开《文城》的More菜单；逐张查看实际截图，确认白色底板、边框、阴影及按钮下方定位。成片列表读取完成后显示“导出成片”。实际执行Escape关闭、Shift+F10重新打开及Escape关闭，最终保留原书架数据与视图。没有通过真实菜单执行目录、导出、重命名或删除动作。桌面截图由Computer Use工具直接显示在会话中，未另存含用户数据的截图；本页本地PNG为隔离示例。

没有改变或逐档验证Windows系统缩放，没有执行安装/升级或真实文件对话框验收。浏览器的五档CSS窗口不代表Windows 125%/150%系统缩放；前一轮Windows记录也不作为本轮新验证结果。下一步补充安装版与系统缩放核对，整体视觉仍须逐页面验收，不以本次局部修复替代。
