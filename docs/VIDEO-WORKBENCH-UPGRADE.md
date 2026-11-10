# 视频制作工作台升级

日期：2026-11-10。范围：现有视频制作页面及其进度、字幕、分镜展示。

## 设计与取舍

沿用书引现有深浅主题，以实际任务为中心组织页面。深色底色#060911、面板#10172a、正文#f1f5f9、视频紫#c084fc、试听冰青#00e5ff、交付青绿#00ff9d；浅色由现有同义变量映射。使用现有本地Inter与系统中文字体，正文14px、说明至少12px，等宽字体仅用于时间码与数值。

```text
视频制作 / 书名                  打开书籍目录
五步导航
制作设置（含运行环境；已有任务默认收起，新项目展开）
当前制作任务                    历史任务选择
真实进度 / 已保存成果 / 停止或继续
字幕校对 | 分镜精修 | 试片与成片（就地定位）
逐句字幕与原声对照
分镜精修与观看提醒
试片播放器 / 起点与时长 / 分镜缩略图
保存校对与确认生成完整视频
```

内容与控件左对齐，以实际视频预览作为视觉重点。压缩重复框线和大号状态卡，保留真实流程状态及必要操作；不采用新增Tab来隐藏编辑器，避免试听、编辑位置和草稿生命周期改变。分镜与字幕保留现有内部滚动和分页，窄屏改为单列。

已有任务的创建表单可以折叠；收起时卸载其播放器，停止隐藏音频，选择值仍由原页面状态保存。分区定位只滚动与聚焦，不保存、不启动任务。创建、字幕校对、单镜候选、AI插画、试片、完整渲染、草稿和用于发布继续沿用现有服务。

## 验收标准

- 新项目直接展示制作设置；已有任务优先展示当前进度，设置可随时展开且选择不丢失。
- 字幕、分镜、试片按实际操作顺序排列；键盘定位后焦点落在目标区，浏览不改变业务内容。
- 进度来自原任务；配置路径不表示环境通过，试片不表示正式成片，完成视频不表示已发布。
- 编辑自动暂存、切任务、单镜采用、取消、生成、只读成片与精确发布入口保持可用。
- 深浅主题、360/800/1120/1440px布局无横向溢出；减少动态效果时关闭非必要过渡。
- 相关单元测试、lint、TypeScript与生产构建通过；浏览器隔离验收与真实桌面、引擎、安装包验收分别记录。

## 实施与验证

已完成本页改造。修改文件：

- `src/modules/video/VideoPage.vue`：制作设置折叠、编辑顺序与定位、试片空态、生成操作位置与页面样式。
- `src/modules/video/VideoJobProgress.vue`：紧凑阶段、横排成果摘要、成片前置。
- `src/modules/video/VideoSubtitleEditor.vue`、`VideoSceneEditor.vue`：正文/说明字号、对照与编辑卡片层级。
- `tests/video.test.ts`：新增设置折叠保留、空任务展开、分区定位保留编辑3项行为回归；现有流程测试按新入口展开设置。
- `docs/PRD.md`、`TECHNICAL-DESIGN.md`、`DEVELOPMENT-PLAN.md`及本文同步现行规则。

无SQL迁移、依赖、权限或版本号变更；本轮按style/patch评估，未提交、推送或发布。

### 实际命令与结果

```powershell
npx vitest run tests/video.test.ts tests/video-progress.test.ts tests/video-subtitles.test.ts tests/video-storyboard.test.ts tests/video-scene-revision.test.ts tests/video-image.test.ts tests/video-environment.test.ts tests/audio-player.test.ts tests/drafts.test.ts tests/ui-refinement.test.ts --maxWorkers=2
npm run lint
npm run build
git diff --check
```

10个文件90项测试通过，lint通过，build包含TypeScript检查与Vite生产构建通过，diff空白检查通过。之前8文件76项聚焦检查也通过。构建保留既有大于500kB分包提示；npm保留现有python/msvs_version用户配置提示，没有关闭规则。

浏览器使用现有ego lite 0.6.2.7，CLI绝对路径调用；独立TaskSpace 9，验收完毕已关闭。开发服务命令为`npm run dev -- --port 14359 --strictPort`，只访问独立内存IPC夹具，未连接正式数据库或外部服务。脚本与截图位于忽略目录`output/playwright/video-workbench-20261110-1540/`。

```powershell
& 'C:/Program Files/Citro Labs/ego lite/Application/0.6.2.7/ego-browser.exe' nodejs -e "await import('file:///D:/MyWork/project/JIAZI-PROJECT/shuyin-ai/output/playwright/video-workbench-20261110-1540/round-09.mjs')"
& 'C:/Program Files/Citro Labs/ego lite/Application/0.6.2.7/ego-browser.exe' nodejs -e "await import('file:///D:/MyWork/project/JIAZI-PROJECT/shuyin-ai/output/playwright/video-workbench-20261110-1540/round-28.mjs')"
```

round-09通过10种任务状态（待确认、转写、模型加载、分镜、渲染、失败、停止、完成、已有试片、空项目）×2主题×4宽度的80组布局。round-28在最终字体调整后再验证8组双主题宽度及800×600，共9组；页面与主要编辑容器无横向溢出，最终未知IPC命令为空。结果为`layouts.json`、`interactions-final.json`。

交互证据分布在round-13/15/16/22/25/26/27：实际音频播放后收起停止、音频选择保留、字幕及分镜修改和自动暂存、三分区聚焦、试片缩略图定位、编辑后隐藏旧试片、试片保存当前校对且不启动整片、完整渲染入口、停止当前任务、成片只读与精确发布链接、Enter定位与减少动态效果均验证。生成/停止部分使用ego的页面DOM入口触发并核对IPC调用，不代表调用真实引擎或模型。

已查看深浅主题1440px与浅色360px截图。后续编辑区截图因ego窗口最小化未完成；真实视频连续播放尝试未完成验证，不记为通过。Windows WebView2、系统缩放、真实引擎/长片/外部模型、新安装包与升级未执行；下一步在桌面版用真实书籍检查字幕、分镜和试片。

### 验证中的修正与限制

实际截图发现原运行环境折叠容器仍占padding空白，已在关闭态清除，同时将运行环境并入制作设置。保留已有检查逻辑和busy锁。

首次PATH未找到ego，随后按用户提供位置使用现有程序，未安装浏览器。临时CDP注入跨连接失效后改用独立HTML入口；测试夹具直接structuredClone响应式参数报错，核实后按真实IPC做JSON序列化，忽略目录旧资源缓存通过运行时夹具适配解决。相同URL导航保留当前任务时，后续用独立case参数重新加载。部分指针自动滚动和宽泛选择器失败后，依据当前页面定位并使用明确选择器或DOM入口；这些是测试工具/夹具问题，没有据此修改产品业务代码。

验收标准在上述本地代码与隔离浏览器范围满足；桌面与真实制作边界保留待验收，未发现本次新增的确定业务问题。

组件依据：已读取锁内Reka UI 2.11.0的CollapsibleRoot/CollapsibleContent源码与[当前公开文档](https://reka-ui.com/docs/components/collapsible)，沿用defaultOpen、unmountOnHide及open插槽；设置保持挂载，仅按展开状态卸载播放器，不新增组件库。

## 追加独立评审与回归（2026-11-10）

评审基线为`2e2aa0ac898e13e15d7fb78a048864695244fb32`，范围为本轮未提交差异及本文。Standards与Spec分别由独立只读评审完成，修复后分别进行只读复审；不包含无关历史问题或未发生的远程发布。

### Standards

初评1项P2：外层制作设置默认卸载整个子树，覆盖了内层运行环境原先保持挂载的行为。已有字幕任务中手动选择“从原声新转写字幕”并检查后，折叠再展开会重新初始化为“使用已有 SRT / 已保存字幕”，检查报告丢失，再次检查会跳过Python/语音模型要求。

新增`retains the chosen environment check purpose and report when settings are collapsed`回归先在原实现失败：预期“从原声新转写字幕”，实际“使用已有 SRT / 已保存字幕”。最小修复保留设置子树，使用现有Collapsible的open插槽仅控制AudioPlayer挂载；关闭内容区清除padding/border，不增加检查状态副本。回归覆盖用途、失败报告、折叠无额外检查/保存及再次检查仍要求转写环境。

Standards独立复审确认根因关闭，剩余确定问题0项。

### Spec

初评确定问题0项，确认编辑顺序、草稿、生成/停止、只读成片与精确发布入口符合本轮范围。修复后独立复审确认设置保留和隐藏播放器停止均满足需求，无新增确定问题。真实桌面、引擎与视觉验收限制继续单独列明。

### 本轮实际验证

| 命令/证据 | 结果 |
| --- | --- |
| `npx vitest run tests/video.test.ts -t 'retains the chosen environment' --maxWorkers=2`（修复前） | 新回归按预期失败，确认用途被重置 |
| `npx vitest run tests/video.test.ts tests/video-environment.test.ts tests/audio-player.test.ts --maxWorkers=2`（首次修复后） | 42通过、1失败；新环境回归通过，旧测试对hidden枚举的精确断言不适用于jsdom，改为验证实际不可见和无播放器 |
| `npx vitest run --maxWorkers=2`（最终） | 34文件479项通过；修复前全量478项亦通过，说明此前缺少环境用途保留覆盖 |
| `npm run lint` | 通过，0警告 |
| `npm run build` | TypeScript及Vite生产构建通过，保留既有大分包与npm配置提示 |
| `git diff --check` | 通过 |

ego复验使用TaskSpace 12及`npm run dev -- --port 14360 --strictPort`，脚本目录`output/playwright/video-review-20261110/`。实际执行：

```powershell
& 'C:/Program Files/Citro Labs/ego lite/Application/0.6.2.7/ego-browser.exe' nodejs -e "await import('file:///D:/MyWork/project/JIAZI-PROJECT/shuyin-ai/output/playwright/video-review-20261110/round-04.mjs')"
& 'C:/Program Files/Citro Labs/ego lite/Application/0.6.2.7/ego-browser.exe' nodejs -e "await import('file:///D:/MyWork/project/JIAZI-PROJECT/shuyin-ai/output/playwright/video-review-20261110/round-05.mjs')"
```

round-04以隔离IPC返回明确的Python依赖错误报告，实际选择用途并检查，折叠后内容高度0、音频节点移除、报告节点仍存在；展开后用途与报告逐字相同，检查次数保持1。round-05通过2主题×2展开状态×4宽度共16组布局，字幕实际计算字号14px，无横向溢出，页面错误与未知IPC均0。证据为`environment-retained.json`及`review-browser.json`。初始页面等待与自动滚动失败后读取实际页面、明确定位再操作，没有改变产品校验逻辑。

ego编辑区截图仍因窗口最小化未完成；未操作用户真实数据、外部模型或平台发布。未执行Rust、真实引擎、连续视频播放、Windows WebView2、安装包或升级验收。验证结束已关闭本次TaskSpace。

本轮修复仅追加VideoPage、video测试与相关文档变动，数据库、依赖、权限及版本号不变；未提交或发布。验收对应：环境选择/报告保留有先失败后通过的回归和浏览器读回；隐藏播放器、零折叠空白与响应式均复验。下一步在Windows桌面版完成实际书籍的视听验收。
