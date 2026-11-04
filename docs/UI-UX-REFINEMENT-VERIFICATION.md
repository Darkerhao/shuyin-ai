# 视觉与创作交互优化实施与验收

规划日期：2026-11-03。实施与验收：2026-11-03至2026-11-04。

本报告记录上一轮[优化方案](UI-UX-REFINEMENT-PLAN.md)中P0/P1/P2的实现及当时执行的检查，不能据此认定整体视觉已验收完成。2026-11-04用户截图指出书架菜单透明叠字；重新复现确认浮层外壳的scoped样式没有命中，原菜单检查只验证打开、关闭和焦点，漏检了展开后的外观。此前“P0–P2已完成实现和本地验证”的结论范围过大，现予更正。本次根因、修复、开发/构建页面检查与截图见[书架菜单复核](BOOK-MENU-VERIFICATION.md)。

下文保留上一轮实际检查记录及其边界。没有提交、发布、更新现有安装版或改动正式用户数据。P3逐字候选预览按原方案保持按需后续状态，首轮已完善实际等待与停止反馈。

## 最新补充：视觉规范与Windows验收（2026-11-04）

视频进度区已清理旧深色文字/渐变及持续流光，复用现有浅色主题；58项相关回归、lint与Release NSIS构建通过。本轮重新执行60组浏览器布局和交互，在Windows设置实际切换100%/125%/150%并读取真实DPR，9页×3档共27组无横向溢出或页面异常，最后恢复原150%。独立安装、同版覆盖安装/重启及原生另存摘要一致通过。**原生跨窗口文件拖入手势仍未取得通过结果**，正式安装旧版本升级、跨平台与真实外部服务仍未验证；P3未实施。完整命令、证据、限制及下一步见[补充验收](WINDOWS-VISUAL-ACCEPTANCE.md)。下文为此前记录，不作为本轮重跑结果。

## 评审问题修复（2026-11-04）

本节记录未提交代码评审发现的三个P2问题及后续修复，原有验收记录继续保留。修复前正式回归已复现三个失败，修复后新增五个场景随最终前端全量检查通过：

- 文案当前来源不是最新历史时，通过原`editor_drafts`保留内容和`based_on`，无需正文先变成dirty。生成备选后离页再进入仍恢复原稿；主动采用最新版本后按原规则清除草稿，不额外创建历史。恢复与正式来源一致时保持“已保存为版本”，不冒充未保存修改。
- 字幕切句时只处理播放器当前确已暂停的pause事件，迟到的旧事件不清空新句子的结束边界。回归模拟实际paused状态和异步pause/play事件，第二句在原时间窗结束后停止，SRT保持不变。
- 辅助发布在核验前等待当前成片元信息查询完成。回归分别覆盖有效成片、缺音轨和读取失败：有效时只启动一次，后续查询参数变化不重放；无效或失败不启动、不记录已发布，保留文字与素材并解除等待锁定。不增加查询、自动重试或第二套任务状态。

本次实际检查：

| 命令 | 结果 |
| --- | --- |
| `npx vitest run --reporter=json --outputFile=output/review-fixes/frontend-final.json` | 363项通过，更新页既有动态导入用例在默认并发下超过原5000ms限制，1项失败；与修复前评审所见相同 |
| `npx vitest run --maxWorkers=2 --reporter=json --outputFile=output/review-fixes/frontend-limited-workers.json` | 364项全部通过，含上述五个新增场景；原5000ms限制保持不变，仅本次命令限制并发 |
| `npm run lint` | 通过 |
| `npm run build` | 通过，包含TypeScript检查；保留既有大于500kB块大小提示 |
| `git diff --check`、`git diff --check HEAD --` | 通过 |

修改三个业务组件`WritingPage.vue`、`VideoSubtitleEditor.vue`、`PublishingPage.vue`及对应三个测试文件，同步技术设计和本报告。未修改共享草稿队列、数据库结构、权限、依赖或测试超时配置；schema仍为v17，复用已有草稿字段。修复影响为fix/patch，不手动升版本。保留原有暂存内容，本次修复没有暂存、提交或发布。

本轮未执行Windows桌面、真实账号、安装包或跨平台验收，也未重跑与本次前端修复无关的Rust及视频引擎测试。下一步用当前开发构建核对快速切句、备选生成后正常重开，以及辅助发布下一篇接续。

## 1. 实现内容与验收对应

| 改造项 | 实际结果与证据 |
| --- | --- |
| 色彩与内容面 | 浅色侧栏`#F7F7F5`、工作区`#F5F5F3`、白色内容面，墨绿只强调主操作与选中项。重复深侧栏、面板、导航规则清理；浏览器计算样式确认普通面板边框为0px |
| 表单与排版 | 共用40px输入/主按钮、10px控件圆角、13px标签、6px标签间距、24px字段节奏；模型页移除重复控件材质和外框覆盖。使用本机字体，无网络字体或新依赖 |
| 书架质感 | 原封面结构增加局部书脊渐变、纸页边缘和柔和投影。继续操作在悬停/聚焦时显示，无悬停设备保留；主链接与更多按钮并列，避免嵌套按钮 |
| 五项导航 | 单行名称、图标与成果标记，说明通过可聚焦Tooltip查看；Ctrl/Cmd+1～5调用原路由。模态交互、输入法组合和重复按键不误跳，未保存与忙碌守卫继续有效 |
| 成果状态 | 从真实草稿、版本、封面PNG校验、音视频元信息和任务查询；运行中的任务优先，不以人工书籍状态推断完成。读取失败显示待同步，当前编辑提示不冒充已保存 |
| 版本与备选 | 首次仍可一键生成；已有内容显示生成备选，追加历史并保留当前稿。历史选择只预览，字段对照与取用进入原草稿，明确采用整套时仍有保护。Windows实际Provider SSE与HTTP524验证新旧版本保留 |
| 复制与保存 | 共用800ms按钮成功态及稳定宽度，连续复制重新计时；失败有手工复制入口。草稿暂存与正式版本保存分开，Ctrl/Cmd+S没有新修改时不重复创建版本 |
| 生成掌控 | 显示实际等待秒数、5分钟上限与原停止入口；完整结束和原业务校验后才交付，不显示未验证片段、不自动重试 |
| 封面画布 | 同一渲染器返回真实文字命中范围，双击/显式画布按钮编辑当前尺寸。Enter完成、Escape仅撤销本次文字；其他字段与尺寸保留。预设即时生效，160ms显示过渡尊重减少动态效果 |
| 视频联动 | 单句、连续原声及分镜试听共用原播放器与原时间码。播放高亮按索引联动，跟随只滚动内部列表；长字幕输入时不换走正在编辑的页、不抢焦点，不高频写入播放位置 |
| 发布自检 | 一张卡显示真实字数、原声、视频尺寸/时长/音轨和两张发布封面；已通过的保存检查折叠，待处理项可就近补齐。一次明确操作核对当前组合，仍复用原确认字段和任务包校验；不自动点击最终发布或写发布记录 |
| 侧栏 | 提示可收起，设置提供恢复入口，显隐随SQLite重启保留；本地根目录可见，书籍内可打开本书目录 |
| 文件拖入 | 主窗口书架使用Tauri文件拖放事件，单文件带入原添加表单；TXT/Markdown/EPUB/PDF/DOCX只关联来源，保存复用原事务，取消无写入。多文件或不支持格式明确提示，不自动生成/解析原书 |
| 上下文菜单 | More、右键、Shift+F10复用一套Reka菜单，Escape返回发起位置。重命名进入原资料字段，目录沿用原服务，成片另存校验原格式且拒绝覆盖，删除保留原件与工作目录 |

## 2. 视觉复核

最终浏览器检查覆盖12个页面状态：书架、新增书籍、资料、创作三个页签、封面、视频、发布准备、模型、数据备份、关于。1440×900、1366×768、1120×900、800×1000、360×900共60组均无整页横向溢出，普通内容面外框0px，页面异常0。

查看了书架、创作、封面、视频、自检和模型的实际截图，补查窄窗口书架/封面；Windows最终截图也复核。截图使用隔离示例或验收项目，不是正式用户作品。

辅助灰由原建议`#64748B`微调为`#5D6D82`，对白底约5.28:1，对共享最深的浅灰面约4.66:1。输入边界`#7D8995`对共享浅色面最低约3.15:1，品牌色对白底约7.85:1。棕色书架封面略加深为`#876648`，文字继承纸页白，对比度约5.02:1。保留可见焦点、错误语义和减少动态效果；这不是全站屏幕阅读器或WCAG认证报告。

## 3. 实际执行的命令

前端基线`npx vitest run`为338项通过。后续因新交互、测试夹具和实际视觉发现逐步修正，以下为最终检查及独立脚本结果。

| 命令 | 实际结果 |
| --- | --- |
| `npx vitest run --reporter=json --outputFile=output/playwright/ui-refinement/frontend-final.json` | 359项通过，0失败；涵盖原338项与新增交互检查 |
| `npm run lint` | 通过，0错误/警告；未关闭规则 |
| `npm run typecheck` | 通过；最终桌面构建中又执行同一TypeScript检查 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked database::workflow::tests` | 6项通过，覆盖实际成果、运行优先、偏好重启、缺失输出、旧有效颜色、另存不覆盖 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked` | 最终206项通过，2项原有专项忽略，0失败 |
| `npm run check:rust` | fmt及Clippy全部目标通过，未放宽警告 |
| `npm run test:video` | 使用独立Node24.22.2，39项通过 |
| `npm run test:release` | 使用独立Node24.22.2，78项通过，1项macOS检查按平台跳过 |
| `npm run test:queue` | 使用独立Node24.22.2，19项通过 |
| `npm run desktop:build -- --debug --no-bundle --config output/playwright/ui-refinement/tauri.verify.conf.json` | Windows隔离debug应用构建通过，包含`npm run build`、TypeScript、Vite和Rust编译；不是NSIS安装包或发布产物 |
| `git diff --check` | 文档和源码收尾检查通过 |

Rust命令加载`. ./.tools/env.ps1`，本轮临时目录指向`output/playwright/ui-refinement/rust-temp`。Node脚本在当前命令进程PATH前置`D:/MyWork/software/shuyin-video-node/v24.22.2`，临时目录指向本轮输出；没有替换全局Node或用户配置。

组合`npm test`最初在系统Node24.11.0下退出1：前端357项通过，视频阶段有6个入口夹具在既有Windows运行时保护处提前退出，未创建episode文件。确认当前render入口主动拒绝有中文路径缺陷的Node版本后，使用已安装24.22.2分别重跑视频、发布、队列全部通过；不将首次组合命令记为通过，也没有绕过保护。最后追加长字幕跟随与已保存封面安静恢复检查，最终前端为359项。

最终生产构建仍有单块大于500kB提示，npm仍提示既有用户配置项。没有调整构建提示阈值或用户npm配置。

## 4. 浏览器实际流程

使用现有Playwright CLI与本地18590服务，夹具只模拟Tauri IPC。实际运行：

```powershell
node D:/DevCache/npm-cache/_npx/31e32ef8478fbf80/node_modules/@playwright/cli/playwright-cli.js -s=ui-refinement run-code --filename output/playwright/ui-refinement/setup.cjs
node D:/DevCache/npm-cache/_npx/31e32ef8478fbf80/node_modules/@playwright/cli/playwright-cli.js -s=ui-refinement run-code --filename output/playwright/ui-refinement/verify.cjs --json
```

7组交互检查通过：

1. 60组页面/窗口与面板边框检查。
2. 卡片显隐、More、Shift+F10、Escape焦点返回和取消删除。
3. Ctrl+1～5五个真实路由与唯一当前步骤。
4. 版本预览/取用、精确复制、按钮宽度稳定、显式保存。
5. 双击画布文字、表单同步、Enter完成、Escape撤销。
6. 实际静音音频播放、字幕/分镜定位、启用跟随后保留输入焦点。
7. 一张自检、两张封面、一次明确核对、生成任务包不记录发布。

输出在忽略目录`output/playwright/ui-refinement/`：`browser-final.json`、各页面1440px截图、`library-360.png`和`covers-360.png`。模拟投放和文件选择不能代替操作系统手势或对话框验收。

## 5. Windows隔离运行

应用标识为`com.shuyin.ui-refinement-verify-20261104`，使用独立数据库，关闭自动更新。测试媒体、工作目录和WebView缓存位于D盘本轮输出；Tauri独立SQLite按默认规则位于该标识的本机LocalAppData目录。未读取或改写正式`com.shuyin.desktop`库，未使用真实账号、密钥或付费模型。

实际运行了：

```powershell
python -B output/playwright/ui-refinement/seed-native.py
./output/playwright/ui-refinement/app.ps1 Start
D:/MyWork/software/shuyin-video-node/v24.22.2/node.exe output/playwright/ui-refinement/native.mjs
./output/playwright/ui-refinement/app.ps1 Close
./output/playwright/ui-refinement/app.ps1 Start
D:/MyWork/software/shuyin-video-node/v24.22.2/node.exe output/playwright/ui-refinement/native.mjs restart
D:/MyWork/software/shuyin-video-node/v24.22.2/node.exe output/playwright/ui-refinement/native-ui-final.mjs
```

本机CDP验证连接使用`NO_PROXY=127.0.0.1,localhost`；Windows应用通过独立路径及进程ID核验后正常关闭，没有停止用户的正式应用。

实际证明：

- 来源路径预填进入原新增表单，真实`save_project`保存书籍及文档关联；作者留空，原文件摘要不变。
- 本机契约服务通过统一Provider返回SSE，一次备选保留当前稿，单字段取用后保存v3；HTTP524保留三份历史。此结果只验证协议与业务行为，不代表真实模型质量。
- 画布行内编辑后真实导出1440×1080、1080×1440、1920×1080三张PNG，核对文件头尺寸。
- 实际1080×1920媒体元信息读取成功；另存字节相同，重复目标明确拒绝，原件不被覆盖。
- asset协议原声播放、按句/按镜定位、焦点保留及人工字幕修改暂存通过，不改变原声或时间码。
- 自检一次明确核对后真实保存、验证封面、生成ready任务包；没有发布记录，音视频摘要不变。
- 正常关闭/重启后，文案版本、三个封面导出、视频草稿、ready包和侧栏显隐均保留。
- 最终构建补查原生More/Shift+F10菜单、Ctrl+1～5、五档窗口、自检面板0px外框、360px画布行内输入边界；页面异常0。

证据：`native-verification.json`、`native-restart.json`、`native-ui-final.json`、`native-library-final-1440.png`和`native-preflight-final-1440.png`。隔离数据库最终完整性、外键及schema核对在本轮输出记录。

收尾只关闭本次隔离应用、浏览器与预览服务，18590/18592/18594无监听。正常关闭后的只读核对为schema17、integrity=ok、外键问题0、发布记录0、运行视频0，三份原件摘要未变；`native-integrity-final.json`保存结果。

## 6. 修改范围

| 文件组 | 修改文件与职责 |
| --- | --- |
| 应用与主题 | `src/App.vue`、`src/style.css`：浅色壳层、目录/提示显隐、主窗口快捷键与拖放、唯一共享布局 |
| 共享界面 | `BookCard.vue`、`CopyButton.vue`、`ProjectNavigation.vue`、`AppIcon.vue`及基础Button/Input/Textarea/SelectTrigger：书架/菜单、复制反馈、轻导航与控件材质 |
| 前端服务 | `services/interface.ts`、`services/projects.ts`、`services/workflow.ts`、`services/coverRenderer.ts`；`useInterfacePreferences`、`useCopyFeedback`、`lib/scroll.ts`：受控接口、共享状态/反馈、同源排版与内部滚动 |
| 创作页面 | `ProjectsPage.vue`、`ProjectPage.vue`、`WritingPage.vue`、`CoversPage.vue`：复用卡片、来源预填、版本对照、当前尺寸行内编辑 |
| 视频与发布 | `VideoPage.vue`、`VideoSubtitleEditor.vue`、`VideoSceneEditor.vue`、`PublishingPage.vue`、`DouyinVerification.vue`：原声联动和统一自检，原辅助流程保留 |
| 设置 | `ModelsPage.vue`、`SettingsPage.vue`：收敛旧视觉覆盖、侧栏提示恢复入口 |
| Rust与权限 | `commands.rs`、`database/workflow.rs`、`database/assets.rs`、`lib.rs`、`permissions/projects.toml`、`capabilities/main.json`：五项受控能力及保存选择器权限 |
| 回归 | `ai`、`covers`、`projects`、`publishing`、`video-subtitles`原测试；新增`copy-feedback`、`ui-refinement`：保护、取消、失败保留、真实版本取用和键盘行为 |
| 工程与文档 | `vite.config.ts`限定入口扫描；产品、技术、数据库、AI规则、手册、开发计划、方案和本报告同步，原三份规划文件继续保留历史 |

## 7. 数据库、权限与版本

数据库仍为v17，没有新增迁移或业务表；`sidebar_prompt_visible`复用app_settings，原备份/恢复路径自然保留。完成摘要和播放位置没有新增持久化状态。

新增`project_progress`、`interface_preferences`、`set_sidebar_prompt`、`video_asset_metadata`、`export_video_asset`，授权限定于原主窗口。元信息和另存复用按书籍/素材/类型校验，另存只创建新文件；没有任意SQL、Shell或文件删除接口。新增dialog:allow-save，没有扩大远程页面权限。

本批包含版本对照、行内编辑、拖入和菜单等兼容用户功能，发布声明应为feat/minor。版本仍为0.45.0，不手工递增、不提交、不推送。

## 8. 过程修正与未覆盖范围

CSS字符串切片曾匹配更早的sticky规则，导致共享规则重复；通过源码与计算样式定位后清理，核心规则只保留一处。测试夹具补齐新元信息/文字范围与matchMedia，原版本切换/确认断言改为新流程；未删除业务验证。一次更新页测试并行超时，聚焦与最终全量均通过，未放宽超时。新增SQL夹具遗漏必填模型字段已修正。工具链临时代理导致一次本机CDP连接502，绕过本机代理后最终连接通过；不改生产网络逻辑。原生验证脚本补齐页面读取/控件就绪等待，避免在原有忙碌守卫生效时提前发快捷键；没有弱化产品守卫。

书籍资料页补齐当前编辑状态传入导航。已保存封面工作区的普通恢复不再反复弹出成功提示；真正恢复未保存修改仍明确提示，并有独立回归。截图中的键盘焦点轮廓按原有无障碍规则保留，展示截图使用鼠标进入安静浏览状态。

尚未人工验证操作系统文件拖放手势、原生另存对话框、100%/125%/150%系统缩放、新安装/升级、macOS/Linux实机、真实外部AI效果和账号辅助发布。本轮实际WindowsWebView2、IPC与隔离文件/数据库结果不替代这些边界。P3逐字预览、PDF/EPUB正文解析和新的文件树均未纳入本轮实现。

下一步在包含本次改动的安装包中核对原生拖入/保存对话框与实际系统缩放，再按原流程进行真实创作。最终发布继续由用户操作，安装或发布不在本次执行范围内。
