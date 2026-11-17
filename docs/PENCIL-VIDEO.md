# 铅笔素描逐笔视频

日期：2026-11-17。当前实现是用户明确的“像铅笔一笔一笔画出来”的素描画风。

## 实现与范围

在现有本地双画风工作流上，把 ink 入口统一为「铅笔素描」，保留「书卷手绘」。素描使用白纸、石墨灰细线、内部纹理与明暗排线；轮廓先画，细节和排线后画，笔尖跟随当前笔画。画面依照原声音频的真实字幕切换，原声和字幕时间码不改写。

沿用 `theme=ink/classic`、原分镜 JSON、视频任务、单镜修订、试片与成片；不增加第三种画风键或数据库迁移。新视频不生成图片，也不从图片提取轮廓。已保存的旧图片水墨仍显示为「旧版水墨画」，保留读取与素材，不继续补图。自动制作所选图片模型仍仅用于封面背景。

## 绘制方式

- [pencil-style.js](../src-tauri/video/pencil-style.js) 提供书页、书卷、人物衣褶、枝干松针与山石的本地轮廓、内部细节和排线；其余主体继续使用唯一的受信任绘图目录。
- [illustrations.js](../src-tauri/video/illustrations.js) 对其他目录主体使用单色细笔和 hachure 排线，移除前序主体/环境墨块。书卷手绘的原几何与配色保留。
- 在构建阶段，对原 Frametell `draw` 动作按真实 SVG 路径长度分配 `at/dur`，一个笔画结束后才画下一个。同 cue 的多个主体共享下一个 cue 或镜尾之前的窗口，后续 cue 主体不会提前出现。主体结束后留出短暂停留。
- 笔尖复用原 `P.tick`、动作 `tip()` 和 HUD 逆坐标变换，不新增计时器、动画状态或像素累计层。原引擎负责每帧显现、重复取帧与回退。
- 素描关闭引擎整页的 `will-change` 提示。当前整页不做运动，路径自己绘制；整页合成缓存原本会因全片与试片的页数不同产生边缘色差，去掉该提示后相同源时间像素一致。
- [pencil-look.json](../src-tauri/video/pencil-look.json) 是唯一素描色板与线条配置。[分镜规则](../prompts/video-pencil-style-v1.md) 同时用于整批和单镜；手动/自动从原任务 theme 冻结读取。
- [render.mjs](../src-tauri/video/render.mjs) 与 [video_commands.rs](../src-tauri/src/video_commands.rs) 的缓存/预览签名与宿主嵌入包含同一 pencil 源文件。短试片保留完整镜头的 `drawDuration`，不因截短而加速画笔。

未修改外部 Frametell 安装或依赖版本。本轮依据实际快照的 `runtime/core.js`、`runtime/engine.js`、`runtime/look.js` 与 `styles/study/style.js` 实现；将来引擎升级须同时验证动作和笔尖接口。

## 验收标准

| 编号 | 标准 | 验证 |
| --- | --- | --- |
| P01 | 白纸、石墨灰细线，轮廓→内部细节→排线 | 专属与普通目录回归、真实样片抽帧 |
| P02 | 单个笔画逐渐增长，笔尖沿当前路径移动 | 注册动作时序与真实浏览器笔尖/进度检查 |
| P03 | 不提前画后续字幕主体，不更改音频或 cue | 同 cue/未来 cue 回归及源时间检查 |
| P04 | 试片与整片同源时间完全一致，重复/回退确定 | 真实引擎相同源时间 PNG 和连续/回退核对 |
| P05 | 素描与书卷手绘独立保留，新视频零逐镜图片请求 | 两种画风及旧图像读取回归、本地双样片 |
| P06 | 手动/自动/单镜、保存恢复仍沿用同一任务 | Vue/Rust相关回归和后端资源嵌入检查 |
| P07 | 本地输出可解码，原声/字幕正确，工程检查通过 | 实际命令、媒体检查和可见画面核对 |

## 本轮验证记录

验证已完成，P01—P07在本地受控分镜、接口回归及真实渲染范围内通过。本轮使用本地SAPI验收旁白的4条实际字幕和3个受控镜头，没有调用付费文本/图片模型。产物位于 `output/pencil-sketch-20261117/`，基线快照保留在 `before/` 与 `before.json`。

### 可观看产物

- [最终素描样片](../output/pencil-sketch-20261117/final/pencil/video.mp4)：容器17.90秒，视频17.833333秒/428帧，1920×1080、24fps、H.264/yuv420p、AAC双声道48kHz；-16.6 LUFS、-1.9 dBTP。
- [同源素描试片](../output/pencil-sketch-20261117/final/preview/video.mp4)：容器4.013秒，视频4秒/96帧，从原声音频4.3062585034秒开始。4秒是验证夹具，界面原有30/60秒选项保留。
- [书卷手绘对照](../output/pencil-sketch-20261117/classic/video.mp4)：同一音频和分镜的原配色/原几何样片，容器17.90秒。
- [素描六格图](../output/pencil-sketch-20261117/final/pencil/contact.png)、[局部笔尖帧](../output/pencil-sketch-20261117/final/pencil/partial-stroke.png)及书卷对照图已查看；轮廓生长、衣褶排线、山石细节与笔尖可见，没有遮住字幕。

### 实际命令与结果

从仓库根目录执行。`node` 使用 `D:/MyWork/software/shuyin-video-node/v24.22.2/node.exe`；媒体及Node音量回归使用 `D:/MyWork/software/ffmpeg/current/bin/ffmpeg.exe`；Rust先执行 `. ./.tools/env.ps1`，临时目录定向本轮 `test-temp`。

| 实际命令 | 结果 |
| --- | --- |
| `node node_modules/vitest/vitest.mjs run tests/video.test.ts tests/automation.test.ts tests/video-progress.test.ts tests/video-scene-revision.test.ts tests/video-storyboard.test.ts --maxWorkers=2` | 5文件、79项通过；手动/自动、新版、单镜及恢复回归 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked --lib -- --quiet` | 332通过、2项原有忽略 |
| `node --test tests/video-render.test.mjs tests/video-audio.test.mjs` | 设置`SHUYIN_TEST_FFMPEG`，53通过、0跳过；绘制时序、未来cue、预览时钟、93物体与原声音量回归 |
| `npm run lint` | 最终代码通过 |
| `npm run build` | TypeScript与生产构建通过；原有大分块和本机npm配置提示保留 |
| `npm run check:rust` | 最终fmt与all-targets Clippy通过，警告视为错误 |
| `node src-tauri/video/render.mjs output/pencil-sketch-20261117/final/pencil/input.json output/pencil-sketch-20261117/final/pencil/result.json` | 最终素描整片、字幕与原声合成成功，layout-check为空 |
| `node src-tauri/video/render.mjs output/pencil-sketch-20261117/final/preview/input.json output/pencil-sketch-20261117/final/preview/result.json` | 同源4秒试片成功，layout-check为空 |
| `node src-tauri/video/render.mjs output/pencil-sketch-20261117/classic/input.json output/pencil-sketch-20261117/classic/result.json` | 书卷手绘样片成功，layout-check为空 |
| `ego-browser nodejs -e "await import('file:///D:/MyWork/project/JIAZI-PROJECT/shuyin-ai/output/pencil-sketch-20261117/browser-final-native.mjs')"` | 3个同源时间PNG相同、4组前后跳转相同；笔尖误差约0.000028px，Engine与颜色错误0 |
| `python C:/Users/haozi/.agents/skills/ffmpeg-skill/scripts/probe.py output/pencil-sketch-20261117/final/pencil/video.mp4 --json` | 最终整片尺寸、帧率、帧数与双声道通过；同命令另探测最终试片 |
| `python C:/Users/haozi/.agents/skills/ffmpeg-skill/scripts/check.py output/pencil-sketch-20261117/final/pencil/video.mp4 --platform custom --content --lufs -16 --tp -1.5 --json` | 10 PASS、1 WARN、0失败；WARN为无独立软字幕轨，画面内中文字幕已查看 |
| `python C:/Users/haozi/.agents/skills/ffmpeg-skill/scripts/look.py output/pencil-sketch-20261117/final/pencil/video.mp4 --tiles 3x2 -o output/pencil-sketch-20261117/final/pencil/contact.png --json-brief` | 最终六格图生成并查看 |
| `node output/pencil-sketch-20261117/verify-evidence.mjs`、`git diff --check` | 渲染源文件/编译脚本一致；同源/回退证据、12份Markdown本地链接及空白检查通过 |

最终CSS修正之后重新渲染素描整片/试片，运行浏览器精确对照、lint及Rust工程检查。Vue/Node/Rust单测与构建在该CSS修正之前已通过；后续仅关闭素描整页合成提示并更新旧任务错误文案，没有重复运行无关测试。

### 根因修复与现场边界

初次Ego截图为系统DPR1.5，取帧时再切到DPR1会引起边缘重采样。最终验证与真实Frametell `openStage` 一致：从初始化到截图始终1920×1080、DPR1。

固定DPR下，试片与成片的cam/hud DOM、笔尖和opacity一致，但原引擎整页`will-change:opacity,transform`合成缓存会使不同页数的边缘色值相差最多2。保留multiply、仅改will-change为auto，PNG即相同；仅改multiply为normal不能解决。项目素描CSS直接关闭该提示，不改第三方源码、不增加另一套状态、不放宽精确检查。当前笔画20%/50%/80%时dashoffset为102.9/64.3/25.7，证实线条逐步增长。

Node几何摘要首轮因公共选项入口加入`paint`而失败；检查原85种参考物体路径保留、仅公共画笔入口改变后更新摘要，最终93物体及全部动作回归通过。浏览器空间7已关闭，本轮24318静态验证服务已停止；未操作正式应用或数据库。

### 修改文件

本轮新增 `src-tauri/video/pencil-style.js`、`pencil-look.json`、`prompts/video-pencil-style-v1.md`、本文，替换前序尚未提交的ink-style/ink-look/ink-style Prompt。修改 `src-tauri/video/{render.mjs,illustrations.js}`、`src-tauri/src/{video.rs,video_commands.rs}`、`src/types/video.ts`、`src/modules/video/{VideoPage.vue,VideoJobProgress.vue,VideoSceneEditor.vue}`、`src/components/AutomationSetup.vue`、`tests/video-render.test.mjs`，同步五份主文档、主题/历史水墨计划和原三个规划文件。其他前序工作区改动保留。

## 数据库、边界与下一步

schema25，无表、列、设置、迁移、依赖或权限变更。已有视频文件保留，工作区源码变化不会更新当前安装版；本轮没有提交、推送或替换正式安装。

样片使用本地验收旁白，展示受信任绘图目录的逐笔效果，不能据此声称真实模型能为所有书籍设计准确镜头或画出任意艺术素描。真实文本服务、原生 Tauri 完整创作、长片性能、安装版与 macOS/Linux 尚需对应验收。下一步用一段真实书籍原声，在含本功能的构建中核对分镜叙事、画面密度和观看节奏。
