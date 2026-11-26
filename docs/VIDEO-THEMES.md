# 视频主题

## 当前规则：42 种引擎画面风格（2026-11-26）

手动视频制作与自动制作均提供 42 项选择，默认书卷手绘。选择在创建时保存；历史任务不随下拉框变化，已有成片可用原音频、原字幕与新风格制作独立新版。

### 接入方式与边界

- 直接使用用户现有 `whiteboard-teacher-video/engine/styles`，不复制引擎，不增加依赖。视频运行环境继续配置该项目的 `engine` 目录；实际 Frametell 0.7.3 安装需含完整 42 套风格及字体资源。
- `src-tauri/video/themes.json` 为前后端与渲染器唯一新建目录。原 `classic` 对应引擎 `study`，另外 41 个键与引擎目录一致；因此没有重复的 `study` 选项。
- 引擎 `style.js` 与 `style.css` 提供配色、笔触、字体、背景及滤镜。后续同日已接入 pixel、manga、pop 和五套三维原生 kit，复用同一分镜与字幕链路，详见[专属场景](VIDEO-NATIVE-SCENES.md)。`free` 沿用引擎基础外观，没有单片自定义外观编辑器。
- 新 `ink-wash`（水墨宣纸）为本地引擎风格；历史 `ink` 显示“水墨宣纸（历史主题）”，保留原逐镜画作与恢复授权，不接受新建。新风格均不追加逐镜图片费用；自动制作原三尺寸封面生成仍按原规则执行。
- 试片和成片使用同一编译器及 `META.style`。原声、字幕时间码、画面元素的时间锚点、响度和最终人工发布流程沿用现有实现。
- 所选风格源文件进入 Node 缓存键；缺少风格文件时明确说明路径。环境检查校验全部 42 套风格的 JS/CSS，拒绝将缺失资源静默显示为书卷主题。
- 无数据库结构/迁移、权限或版本号变更；主题复用已有 JSON 字段。新增能力按 `feat`/minor 判断，发布版本由现有流水线处理。

### 风格目录

完整目录以 [themes.json](../src-tauri/video/themes.json) 为准，包括书卷手绘、白板马克笔、黑板粉笔、暖色插画、科技深色、水墨宣纸、水彩绘画、中国剪纸、像素游戏、黑白漫画、黏土动画、工程蓝图、宇宙星空、复古地图等 42 套。

### 本次验证

以下为基础42风格接入时的记录，后续原生组件迁移的范围和新验证见[专属场景](VIDEO-NATIVE-SCENES.md)。

以下命令从书引 AI 仓库根目录执行。Node 使用 `D:/MyWork/software/shuyin-video-node/v24.22.2/node.exe`；Rust 命令前执行 `. ./.tools/env.ps1`；媒体命令将 `D:/MyWork/software/ffmpeg/current/bin` 加入 PATH。样片、截图和隔离浏览器夹具仅位于忽略目录 `output/video-styles-20261126/`。

| 实际命令 | 结果 |
| --- | --- |
| `node node_modules/vitest/vitest.mjs run tests/video.test.ts tests/automation.test.ts tests/automation-tasks.test.ts --maxWorkers=2` | 最终文案调整后复跑：3 文件、67 项通过。覆盖两个完整风格目录、所选风格提交、历史水墨标签和恢复 |
| `$env:SHUYIN_TEST_FFMPEG='D:/MyWork/software/ffmpeg/current/bin/ffmpeg.exe'; npm run test:video` | 52 项通过、0 跳过；覆盖 42 套风格的完整/试片编译、原时间轴、未知路径拒绝、旧水墨和实际音轨合成 |
| `npm run lint` | 通过 |
| `npm run build` | 类型检查及生产构建通过；保留本机 npm 配置和既有 Vite 大分块提示 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked --lib -- --quiet` | 335 通过、2 项原有忽略、0 失败；覆盖全部风格的新建与持久化、自动冻结、历史逐镜水墨继续 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | 通过 |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets -- -D warnings` | 通过 |
| `node output/video-styles-20261126/prepare.mjs` | 核对实际引擎风格目录与共享目录逐项相等，共 42 项；生成隔离夹具 |
| `ego-browser nodejs -e "await import('file:///D:/MyWork/project/JIAZI-PROJECT/shuyin-ai/output/video-styles-20261126/round-02.mjs')"` | 实际 Chromium 加载全部 42 套引擎风格，逐项核对 STYLE.id、6 秒时钟、字幕、路径元素、资源响应及字体状态：42/42，无缺失资源、无字体错误 |
| `ego-browser nodejs -e "await import('file:///D:/MyWork/project/JIAZI-PROJECT/shuyin-ai/output/video-styles-20261126/round-07.mjs')"` | 两个入口均有完整 42 项，选中自动水墨与手动矢量；1440/900 宽度和深浅色 4 组合无横溢；手动创建准确提交 vector，无逐镜图片模型，未知 IPC 为 0 |
| `node src-tauri/video/render.mjs output/video-styles-20261126/preview-watercolor/input.json output/video-styles-20261126/preview-watercolor/result.json` | 真实水彩试片：1920×1080、24fps、72 帧、视频 3 秒，音轨/容器 3.008 秒；布局检查 `[]` |
| `node src-tauri/video/render.mjs output/video-styles-20261126/render-tech-dark/input.json output/video-styles-20261126/render-tech-dark/result.json` | 真实科技深色成片：1920×1080、24fps、144 帧、6 秒；布局检查 `[]` |
| `python C:/Users/haozi/.agents/skills/ffmpeg-skill/scripts/probe.py output/video-styles-20261126/render-tech-dark/video.mp4 --json-brief`，对水彩试片执行相同 probe | 两份均 H.264、AAC 双声道、48000Hz，时长/尺寸符合输入 |
| `python C:/Users/haozi/.agents/skills/ffmpeg-skill/scripts/look.py output/video-styles-20261126/render-tech-dark/video.mp4 --tiles 3x2 --width 1200 --json-brief`，对水彩试片执行相同 look | 两份联系表生成并实际查看；逐笔绘制、两种底色和字幕可见，3 秒试片保留本镜原绘制速度 |
| `python C:/Users/haozi/.agents/skills/ffmpeg-skill/scripts/check.py output/video-styles-20261126/render-tech-dark/video.mp4 --platform custom --json-brief` | 5 项 PASS，0 失败，1 个预期提醒：采用烧录字幕，无独立软字幕轨；工具 `ok=true`、`verified=false`，不将提醒忽略为全通过 |
| `git diff --check` | 通过 |

首轮视频 Node 套件在两个样片同时编码时出现原有 4 秒子进程夹具超时（status=null），停止编码负载后单项 0.6 秒通过，完整含音频复跑 52/52；未加长超时或弱化断言。浏览器首轮 Vite 导航等待 15 秒超时，但页面已提交，后续在同页确认完成加载；长列表 End 后焦点异步变更，按实际选项状态完成选择，未改产品键盘逻辑。最终 UI 夹具选择和提交均已核验。

浏览器页面为真实组件加隔离 IPC，渲染样片为本机真实引擎与 FFmpeg，音轨为合成测试音；不接正式书库，不调用付费模型，不宣称真实书籍内容、原生安装版完整链路或 42 套主题全部长片编码已验收。未制作安装包、替换正式桌面程序、提交、推送或发布。建议下一步在开发版用真实短音频试听，再按现有流程打包发布。

## 历史规则：水墨宣纸新建下线（2026-11-26，已由上述范围更新）

- 手动视频制作和自动制作只提供「书卷手绘」，默认 `classic`；新视频不再选择水墨画作模型或追加逐镜水墨图片请求。自动制作的图片模型仍用于三尺寸封面。
- 使用原音频、原字幕制作新版也使用书卷手绘；新建 API 拒绝水墨输入，自动新建拒绝逐镜水墨标志。
- 历史 `ink` 任务标注「水墨宣纸（历史主题）」，原声、字幕、画作、成片、任务冻结输入和原任务续作保留。已有自动流程仍按原冻结主题创建内部视频任务。
- 不增加新主题、不改数据库结构、不清理用户素材、不手动升版本。

### 本次修改范围

前端：`src/types/video.ts`、`src/services/video.ts`、`src/modules/video/VideoPage.vue`、`src/components/AutomationSetup.vue`。后端：`src-tauri/src/database/video.rs`、`src-tauri/src/database/automation.rs`。回归：`tests/video.test.ts`、`tests/automation.test.ts`、`src-tauri/src/database/video/ink.rs`、`src-tauri/src/database/automation_tests.rs`。本页、`INK-VIDEO.md` 及五份主文档同步记录当前规则，原历史实现和验收记录保留。

### 本次验收（2026-11-26）

所有命令从仓库根目录执行，Node 使用 `D:/MyWork/software/shuyin-video-node/v24.22.2`，Rust 先执行 `. ./.tools/env.ps1`。

| 实际命令 | 结果 |
| --- | --- |
| `node node_modules/vitest/vitest.mjs run tests/video.test.ts tests/automation.test.ts tests/automation-tasks.test.ts --maxWorkers=2` | 3 文件、67 项通过；覆盖两个入口、无图片模型创建、旧水墨成片制作经典新版、历史自动任务结束后的默认值、原逐镜续作 |
| `npm run lint` | 通过；首次发现测试中已不再使用的导入，删除后重跑通过 |
| `npm run build` | 类型检查和生产构建通过；保留原 npm 本机配置与 Vite 大分块提示 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked --lib -- --quiet` | 334 通过、2 项原有忽略、0 失败；覆盖新任务门禁、经典默认值和历史水墨恢复 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | 通过 |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets -- -D warnings` | 通过 |
| `ego-browser nodejs -e "await import('file:///D:/MyWork/project/JIAZI-PROJECT/shuyin-ai/output/theme-removal-20261126/round-04.mjs')"` | 两个下拉仅含书卷手绘；1440/900 宽度与深浅主题 4 组合无横溢；没有水墨模型控件；无图片模型的手动创建仅发送 classic，未识别 IPC 为 0 |
| `git diff --check` | 通过 |

浏览器为真实 Chromium 加隔离 IPC 夹具，不连接正式书库、不调用付费模型、不渲染真实成片。首轮夹具因直接导入另一份路由模块导致注入失败，改为经 Vite 统一解析模块后通过，产品代码未为此调整。证据在忽略目录 `output/theme-removal-20261126/`，截图已查看，浏览器空间和临时 Vite 服务已关闭。未制作安装包、未替换正式桌面应用、未执行真实模型/发布或跨平台验收。后续新主题确定后另行实施。

以下为原双主题的历史设计和验证，涉及新建主题选项与默认值的描述已由上述规则取代。

## 历史双主题设计（2026-11-16）

## 1. 需求与范围

视频制作提供两种主题，由用户在创建任务时选择：

- `书卷手绘`：保留原有配色、线稿、环境和物体几何。
- `水墨宣纸`：依照实际字幕创建详细艺术分镜，为每镜用所选图片模型生成一张完整水墨画，并沿原声时间轴进行细线显影、墨色层次与轻运镜。

新任务默认 `水墨宣纸` 并启用逐镜画作；用户须选图片模型，调用张数等于分镜数量，可能产生模型费用。`书卷手绘` 不调用逐镜图片模型。旧计划缺 `imageProviderId` 时保持旧分镜和渲染，不自动升级或发起付费图片请求。可用原字幕制作新版切换风格。

逐镜水墨实现与验收见[水墨视频闭环](INK-VIDEO.md)。入口见 [VideoPage.vue](../src/modules/video/VideoPage.vue)、[render.mjs](../src-tauri/video/render.mjs)、[ink-painting.js](../src-tauri/video/ink-painting.js)、[illustrations.js](../src-tauri/video/illustrations.js)。

## 2. 数据与兼容

- `VideoPlan.theme` 为可选 JSON 字段，值为 `classic` 或 `ink`。
- `VideoPlan.imageProviderId` 标识新逐镜水墨任务；`Scene.painting` 存储画面描述、有限运镜和可恢复图片 taskId，`Scene.image` 冻结生成图及内容摘要。
- 旧 `plan_json` 缺少该字段时反序列化为 `classic`，并在重新保存经典主题时继续省略该字段。
- 新建任务的 `StartVideoInput.theme` 缺省为 `ink`；手动页面显式传入选中的主题。
- 自动制作 `selection.detailedInk` 采用显式可选值，旧 JSON 缺字段默认 false；创建视频时冻结图片模型 ID。
- 描述、图片 taskId 和图像资产复用 plan_json、ai_tasks、assets；无新 SQLite 结构/迁移，schema 仍为 v25。
- 预览签名、渲染缓存键和自动流程的计划摘要继续包含完整 plan，因此主题变化会使旧试片失效。
- 已完成的成片不重写；旧任务只读保持原主题。
- 已移除铅笔素描后，历史 `plan_json` 中的 `pencil`、`Scene.artwork` 和 `Beat.drawing` 在读取时剥离，对应镜头降级为旧版文字图解并要求重新生成；数据库原 JSON 不改写，正式新分镜和模型输出仍拒绝未知字段。旧版 `sceneEdits` 草稿不再套用，仅恢复字幕与编辑位置。

## 3. 渲染实现

`render.mjs` 将主题写入每个页面的 `theme` 和 `META.theme`：

- 经典主题走原 `environment`、`sketch` 和颜色表；原 85 个参考物体的归一化源码摘要仍由 `test:video` 校验。
- 旧水墨计划沿用原低饱和矢量分镜；新逐镜画作独立生成图片，不把调色滤镜当作完整水墨画。
- `video_ink` 子任务沿用既有图像 Provider，画作在单一 SQLite 事务中登记素材并回写 plan；失败后重试同一个镜头，重启后成功画作不重做。
- `ink-painting.js` 从同一冻结图片中逐时显影笔画，并按确定性运镜状态渲染，预览/成片共用 cue 时钟。
- 字幕仍由引擎固定层负责；水墨主题使用暖纸色半透明底和更薄的边角，经典主题字幕样式不变。
- 仅画面描述变化时当前镜头图片失效；字幕或时间码变化时清空水墨分镜，按新内容重建。原声、字幕时间码、HUD 字幕层、响度与发布流程不变。

## 4. 页面交互

手动视频页要求选择图片模型并说明按镜费用；分镜区展示描述、画作完成状态及运镜，用户修改描述后只重做本镜。未完成画作时不提供试片或成片。自动制作开始授权说明调用费用随镜头数变化；旧自动流程不追加图像请求。

## 5. 历史双主题验证（2026-11-15）

工作目录为仓库根目录。Node 使用 `D:/MyWork/software/shuyin-video-node/v24.22.2/node.exe`，媒体工具使用本机 `D:/MyWork/software/ffmpeg/current/bin`。

| 实际命令 | 结果 |
| --- | --- |
| `npm run test:video` | 47 项中 41 通过、6 项既有音频集成测试跳过、0 失败 |
| `npx vitest run` | 38 文件 548 项通过 |
| `npx vitest run tests/video.test.ts` | 最终按钮文案调整后 36 项通过 |
| `npx vitest run tests/automation.test.ts tests/automation-tasks.test.ts tests/video.test.ts` | 3 文件 62 项通过 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked video` | 64 项通过、1 项既有进程树专项忽略 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked automation` | 52 项通过 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked --lib -- --quiet` | 320 项通过、2 项既有忽略 |
| `npm run lint` | 通过，未关闭规则 |
| `npm run build` | 类型检查与 Vite 构建通过；保留既有大于 500 kB 分块提醒 |
| `cargo fmt --check` / `cargo clippy --locked --all-targets -- -D warnings` | 通过 |
| `git diff --check` | 通过 |
| 真实 Frametell 0.7.3 隔离样片 | 同一 12 秒静音夹具分别以 `ink`、`classic` 渲染；两份均 1920×1080、24fps、288 帧、12.0 秒、H.264 + AAC 双声道 |
| `probe.py` + `check.py --platform custom --no-loudness` | 两份 probe 通过；check 0 失败，2 个预期提醒（夹具音轨静音、无独立软字幕轨） |
| `look.py --tiles 3x2` / `--at 8` | 两份联系表及 8 秒帧已查看；水墨主题为暖纸、低饱和墨色和淡墨层，经典主题保留蓝色水纹、橙色进度线等原配色 |
| `layout-check.json` | 两份均为空数组，无布局错误 |

样片在忽略目录 `output/theme-verify/`：`video.mp4` 为水墨，`classic/video.mp4` 为经典；联系表分别为 `video_sheet.png` 与 `classic/video_sheet.png`。样片使用静音 WAV 和两条测试字幕，仅验证渲染与界面链路，不代表真实内容质量。

## 新逐镜水墨验证

当前实现命令、真实引擎样片及未覆盖边界见[水墨视频闭环](INK-VIDEO.md)。本页以下 2026-11-15 的调色样片仅作兼容/历史记录，不证明新模型画质。

## 6. 未验证边界（原双主题记录）

- 未用真实书籍音频、真实长片和实际模型生成内容审听水墨主题；只用 12 秒夹具验证编码与画面差异。
- 未运行完整 `npm test` 的音频、发布和队列套件；本轮运行的是与改动相关的视频 Node、视频页面、Rust video、lint 和类型检查。
- 未制作安装包、未替换正式安装版、未做 macOS/Linux 实机验收。
- 水墨主题是现有矢量分镜的视觉主题，不是每镜 AI 水墨插画；后者需要另立图片生成/素材审核切片。

## 7. 原双主题历史结论

以下结论针对 2026-11-15 的调色/矢量水墨实现，不代表新逐镜水墨画作已通过。当前闭环、测试命令和质量边界见[水墨视频实现与验证](INK-VIDEO.md)。

上述遗留的“后续再增加每镜插画”建议已被本轮完整实现取代；实际第三方模型画质及安装版运行仍须按 `INK-VIDEO.md` 的边界验收。
