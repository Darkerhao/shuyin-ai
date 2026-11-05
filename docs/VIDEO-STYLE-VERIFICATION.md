# 视频制作设计稿实现与验收

日期：2026-11-05。参考：用户提供的 `D:/谷歌浏览器下载/工作账号下载/ai (1).html`。

## 方案与验收标准

在当前Vue视频模块上重排布局，继续复用统一深色主题、现有组件与本地字体。色彩为背景 `#060911`、面板 `#0F1626`、边界 `#1E293B`、紫色 `#C084FC`、冰青 `#00E5FF`、青绿 `#00FF9D`。主标题使用Inter/系统中文字体，字段说明与时间/进度使用JetBrains Mono/系统中文字体。内容左对齐，24px区块间距，16px面板圆角。

```text
返回封面工坊                       打开书籍目录
视频制作                         当前真实环境提示
书籍资料 → 内容创作 → 封面工坊 → 视频制作 → 发布准备
┌ 原声输入：音频 / 模型，字幕来源，原声播放器，通栏生成按钮 ┐
└─────────────────────────────────────────────────────┘
环境配置（按需要展开）
任务与成片：历史选择、真实进度 / 失败 / 完成、成片操作
字幕校对：暂存状态、原声播放器、原文 / 校对文、保存操作
原声试片（保留现有业务入口）
观看提醒
分镜精修：双列卡片、物体/场景设置、完整视频确认
```

按原稿调整区域和强调色，取消当前任务面板的衬线标题、浅色视觉残留及堆叠操作。原型中的仿真按钮、固定计数/百分比、假波形、假文件地址和“引擎就绪”不作为业务结论；配置只能说明已填写，真正环境检查继续使用现有检查结果。原稿没有覆盖的试片、单镜候选与图片编辑继续使用实际组件。

验收包含：运行（可量化与不可量化）、失败、停止、待确认、成功和空任务；生成/停止/继续/校对保存/草稿恢复/试听/试片/发布入口；1440、1366、1120、800、390、360px布局、键盘焦点及减少动态效果。

## 数据与版本影响

前序代码已按用户要求提交为 `1b59a1a`。本轮视频改动属于样式/内部结构调整（style/refactor，patch），不修改版本号、数据库schema17、Rust/IPC、Prompt、模型调用、依赖、媒体文件及发布权限。

## 修改文件

- `src/modules/video/VideoPage.vue`：独立区域、返回栏、环境配置、主操作、局部播放器视觉及窄窗布局。
- `src/modules/video/VideoJobProgress.vue`：真实状态色、五步卡片、计数/预览、集中错误详情、操作及媒体插槽；移除旧环形装饰和衬线标题样式。
- `src/modules/video/VideoSubtitleEditor.vue`：筛选/跟随状态、原文对照、校对输入、试听与分页视觉。
- `src/modules/video/VideoSceneEditor.vue`：试听/重生成同行、标题、构图/场景/景别、按需图片与物体设置的双列卡片。
- `src/App.vue`、`src/style.css`：同一视频路由的侧栏/顶部入口和共享紫色token，现有其他页面沿用原主题。
- `tests/video.test.ts`：滚动验证改为定位新的整个任务工作区，继续断言显式确认、先保存计划、再渲染及滚动到任务。
- PRD、技术设计、开发计划、用户指南与现有规划记录同步；截图在 `docs/assets/video-style/`。

对四个视频组件的脚本与提交基线逐段比较，除两个组件的图标导入外，业务脚本完全相同。没有改写生成、轮询、时间码、草稿、媒体读取或单镜候选逻辑。

## 实际验证

### 代码检查

| 实际命令 | 结果 |
| --- | --- |
| `npm run lint` | 最终通过，ESLint 0 警告 |
| `npm run typecheck` | 通过 |
| `npm run build` | 最终 TypeScript / Vite 生产构建通过，856 modules |
| 下方视频/播放器 Vitest 命令 | 7文件68项通过 |
| 下方导航/波形 Vitest 命令 | 2文件11项通过 |
| `git diff --check` | 通过 |

```powershell
npx vitest run tests/video.test.ts tests/video-progress.test.ts tests/video-subtitles.test.ts tests/video-scene-revision.test.ts tests/video-image.test.ts tests/video-environment.test.ts tests/audio-player.test.ts --maxWorkers=2
npx vitest run tests/ui-refinement.test.ts tests/audio-waveform.test.ts --maxWorkers=2
```

提交前基线另执行 `npx vitest run`：388项通过，`updates.test.ts` 一项超出默认5秒；随即单独执行 `npx vitest run tests/updates.test.ts`，11项全部通过。没有调整超时时间或跳过测试。本轮首轮视频回归发现滚动测试仍给旧面板设置spy，已定位为DOM重排后的选择器变化，更新到真实ref所在任务容器后68项通过，断言未弱化。

### 生产预览与浏览器

复用本机已运行的Vite生产预览 `http://127.0.0.1:14329`，加载本轮最终构建。浏览器使用独立内存IPC夹具，调用真实Vue组件、现有表单/弹窗、AudioPlayer及原声波形；播放仓库已有的隔离音频和25秒视频样本。模型、任务、路径与状态来自夹具，不读写正式SQLite或调用收费模型。

实际执行（CLI结果另存JSON并解析检查错误）：

```powershell
node D:/DevCache/npm-cache/_npx/31e32ef8478fbf80/node_modules/@playwright/cli/playwright-cli.js -s=video-style-reference run-code --filename output/playwright/video-style/reference-shots.cjs
node D:/DevCache/npm-cache/_npx/31e32ef8478fbf80/node_modules/@playwright/cli/playwright-cli.js -s=video-style-current run-code --filename output/playwright/video-style/setup.cjs --json
node D:/DevCache/npm-cache/_npx/31e32ef8478fbf80/node_modules/@playwright/cli/playwright-cli.js -s=video-style-current run-code --filename output/playwright/video-style/check-interactions.cjs --json
node D:/DevCache/npm-cache/_npx/31e32ef8478fbf80/node_modules/@playwright/cli/playwright-cli.js -s=video-style-current run-code --filename output/playwright/video-style/check-layout.cjs --json
node D:/DevCache/npm-cache/_npx/31e32ef8478fbf80/node_modules/@playwright/cli/playwright-cli.js -s=video-style-current run-code --filename output/playwright/video-style/check-start.cjs --json
node D:/DevCache/npm-cache/_npx/31e32ef8478fbf80/node_modules/@playwright/cli/playwright-cli.js -s=video-style-current run-code --filename output/playwright/video-style/final-shots.cjs --json
```

| 验收内容 | 结果 |
| --- | --- |
| 1440/1366/1120/800/390/360px × 待确认、模型加载、转写、分镜、渲染、失败、停止、成功、空任务 | 54组通过，页面及主要区域无横向溢出，页面异常0 |
| 原稿截图对照 | 已逐区查看顶部、运行/失败/成功、字幕和分镜；保留真实媒体、时间、文本长度带来的差异 |
| 初始加载与导航 | 不自动启动/写草稿；本书目录、封面返回和视频当前步骤正确 |
| 环境配置 | 展开/检查、取消选路保留原值、改路径使检查失效、还原配置通过 |
| 字幕校对 | 修改自动暂存、仅看修改、单句还原、SRT时间码保留、正式保存不自动渲染通过 |
| 分镜 | 编辑标题、展开物体设置、历史任务切换与草稿恢复、生成候选后放弃保留原稿通过 |
| 原声 | 真实本地样本逐句/分镜播放与停止、同一时间单份声音通过 |
| 制作 | 原声/模型/字幕参数准确带入新任务，滚动定位；试片与整片区别、显式确认、先保存后渲染、停止/继续通过 |
| 成片 | 本地视频实播到0.31秒后暂停，readyState=4，媒体错误为空；字幕导出和精确发布任务链接通过，已完成任务不开放编辑 |
| 窄窗与键盘 | 360px下拉键盘打开/关闭及焦点返回、单镜弹窗边界通过；减少动态效果时无运行动画 |

交互主脚本8组与新任务启动补验1组均通过。生产结果见忽略目录 `output/playwright/video-style/{layout-production,interactions-production,start-production,final-shots}.json`。

脚本调试中两次失败均已查明并修正：测试IPC直接structuredClone了Vue代理参数，改为真实IPC对应的JSON序列化；同一含hash的URL未触发新文档，夹具复位改为显式reload。初次测试样本的分镜枚举也按当前catalog修正为route。上述修改仅在测试夹具，没有为其修改产品逻辑。

### 截图

- [顶部与原声输入](assets/video-style/overview.png)
- [制作中](assets/video-style/running.png)
- [失败详情](assets/video-style/failed.png)
- [字幕校对](assets/video-style/subtitles.png)
- [分镜精修](assets/video-style/scenes.png)
- [成片与真实样本播放](assets/video-style/completed.png)

## 未覆盖范围与下一步

本轮没有新SQL迁移，也没有执行Rust/数据库迁移测试、Tauri原生桌面人工验收、原生文件选择器、安装升级、真实ASR/远程模型/整片渲染、平台发布或跨平台实机验收。浏览器的任务操作验证的是前端状态与IPC参数，不能视为真实引擎成功。构建仍有既有大于500kB分块提示，npm仍提示已有python/msvs_version用户配置。

本轮样式与上述本地验收已完成，未发现本轮范围内尚未解决的代码问题。下一步建议在Windows桌面打开一个已有任务确认实际缩放及原生媒体外观。用户随后明确要求生成中文提交信息并提交，本次以 `style(video): 按设计稿重构视频制作工作台样式` 归档代码、测试、文档和截图；仅本地提交，不推送。本次提交前重新检查Git差异与工作区，沿用上述已执行验证，不重复运行未变更代码的测试和构建。
