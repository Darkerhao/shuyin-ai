# 书籍原声 MG 视频接入方案

日期：2026-11-01。状态：用户确认后已实施 Windows 首版，并完成隔离短片链路验证。具体源码落点、运行环境和验收范围见 [实施与验收](MG-VIDEO-VERIFICATION.md)。本文保留原始设计动机及后续完整长片/发行验收要求。

同日画面迭代已将首版五类文字图解替换为参考成片同款85物体/13构图/6动作；使用video-storyboard-v2和唯一绘图目录，不保留旧模板渲染路径。旧任务明确保留字幕升级、已完成作品新建版本，具体实现见技术设计和验收记录。

2026-11-02 P0已将生成Prompt替换为v3并完善主次与显式字幕触发；增加30/60秒同源试片、缩略图、受控编辑和观看提醒。仍使用同一渲染器/任务与原声，预览结束等待确认，不登记正式成片。以下首版设计保留为历史，当前边界以 [P0实施与验收](VIDEO-DIRECTOR-VERIFICATION.md) 为准；场景、扩库、图片/AI插画尚未接入。

## 1. 结论与交付目标

可以在书引内完成「选择本书音频 → 识别字幕 → 生成分镜 → 确认并渲染 → 播放成片 → 用于发布」。不再要求用户必须经过剪映，但保留已有外部成片导入能力。

推荐复用 whiteboard-teacher-video 的独立本地渲染引擎，而不是合并另一个项目。书引负责业务流程、字幕、分镜数据、任务和素材；引擎负责绘制和编码。原书音频是唯一配音来源，不调用 Edge TTS，不重新生成声音，不自动改变语速、删减内容或加入背景音乐。

首版限定 Windows、横屏 1920×1080、24 fps、书卷风、单个视频任务。这是最小闭环，不同时扩展竖屏、五套风格、时间线编辑器或云端渲染。

成品为 H.264/AAC MP4 和独立 UTF-8 SRT。音频内容与时间保持，兼容 MP4 所需的 AAC 转码不等于音频文件字节不变。字幕烧录入画面；SRT 可另存和校对，播放器不再重复叠加同一份字幕。

## 2. 接入前的代码核对

| 能力 | 实际依据 | 结论 |
| --- | --- | --- |
| 选择本书音频 | `src/modules/writing/WritingPage.vue` 的 audioAssets、importAudio；`src/services/projects.ts` | 可复用已生成或手动导入的音频素材 |
| 字幕/视频登记 | `src/types/project.ts`、`src-tauri/src/database/assets.rs` | 已支持 audio、video、subtitle，包括 SRT/ASS/VTT，无需再建素材系统 |
| 音频转写 | `src-tauri/src/ai.rs` 的 TextProvider/OpenAiCompatible | 当前没有音频转写 Provider；音频指导不是逐字稿 |
| 成片发布 | `src-tauri/src/database/publishing.rs` 的 video_path/final_video_ready | 可复用视频选择与检查；应去掉界面「必须剪映导出」的表述 |
| 应用内播放 | `src-tauri/tauri.conf.json`、现有前端源码 | 没有本地视频播放器；需要受限媒体读取与 CSP 配套，不只增加 video 标签 |
| MG 引擎 | 外部项目 `engine/tools/render.mjs`、`tools/lib.mjs`、`runtime/timing.js` | 依赖 Node、Chrome/Edge、FFmpeg、字体；按工程目录渲染，不是现成的书籍视频 API |
| 原声时间轴 | `D:/films/视频风格实验/08-人生由我/script.js`、`extras/原声时间轴.js`、`build_project.py` | 有原声/SRT工程范例，但字幕来自已有文件，分镜和绘图是逐片创作，不是通用自动导演 |

这些是前置评估时的能力与缺口。后续新增能力以实施报告为准，不以旧影片制作记录代替本次验证。

## 3. 唯一推荐的生成链路

```text
本书 audio asset
  -> 本地 TranscriptionProvider（faster-whisper）
  -> 带音频时间码的字幕 + SRT
  -> 当前 TextProvider 生成结构化分镜
  -> 用户检查字幕和分镜，确认制作
  -> 固定组件模板编译为引擎工程
  -> 独立 Frametell 进程渲染、叠字幕、合入原音频
  -> 校验 MP4/SRT -> 事务登记素材和成功状态
  -> 应用内播放器 -> 用户明确选为发布成片
```

### 3.1 字幕以音频为准

- 新增一个本地 TranscriptionProvider 实现，使用 faster-whisper 的 CPU/int8 模式和多语言模型，开启 word_timestamps。首版不要求 CUDA，也不增加第二套云转写协议。
- 当前锁定 faster-whisper 1.2.1、av 16.1.0 及本地 small 模型；已有20秒原声验证，但长片性能及中文专名准确率仍需试听验收，不根据其他语言跑分承诺本机速度。
- 通过独立 setup-video.ps1 一次性安装依赖/下载模型，应用只选择运行路径，不自动执行安装脚本。目录由用户选择，优先D盘；转写要求完整本地模型，缺失tokenizer直接报错，不触发隐式下载。
- 从识别结果分句并导出 `HH:MM:SS,mmm` SRT，校验非空文本、有限且递增的起止时间、无重叠和不超出音频时长。自然静音可以没有字幕，不能强行补满时间轴。
- 书名、作者可作识别上下文，但不能用音频生成提示词或摘要捏造台词。人名、多音字、外语和低置信度位置交给用户试听校对，不承诺自动转写零错误。
- 校对另存版本，不覆盖原始识别结果。改文字但未改音频时保留时间码；不因润色字幕偷偷替换原声含义。
- 已有正确 SRT 可复用素材输入，进入同一校验/分镜/渲染链；这是输入来源差异，不另建制片实现。

### 3.2 分镜是数据，不是任意代码

复用当前文本模型配置、安全凭据与 TextProvider，新增独立版本化分镜 Prompt。单次请求沿用当前 300 秒上限；长音频按字幕片段有界拆分并保存已完成结果，不用一次请求输出几百镜，也不把 300 秒作为整个视频任务的时限。

模型只返回受约束 JSON：引用的字幕 ID、镜头主题、组件类型、短标签、相对字幕的动作时刻。绝对起止时间由程序根据字幕计算，模型不得重写原声时间轴。

当前分镜使用目录中的13种构图、85种物体和6种动作；每镜1—3个物体，标签最多8字，cue是原声字幕的全局索引。程序验证枚举、文字长度、引用范围及有序锚点，再由参考读书成片的可信绘图生成引擎页。不让模型输出可直接执行的 JavaScript、shell、路径或任意网络地址；不把 Node vm 当作安全沙箱。

新片必须保留参考物体绘图、构图、笔划动作与字幕样式；不以「大段字幕轮播」或更高码率冒充同等级画面。目录外的专属道具不能凭空画出或退回文字框，应通过分镜校对选择适用物体；实际模型的语义选择和完整听审仍需验收。

### 3.3 原声对齐与渲染

- 引擎默认 timing.js 会根据字数/配音插入节奏间隔；原声模式必须使用统一的绝对时间轴适配，供 Node 与浏览器共用。禁止按字符平均估时后配上整段原声。
- 镜头覆盖完整音频时间，字幕间隙保持画面但不伪造台词；片尾只允许不足一帧的对齐差，不截断最后一句。
- 渲染前用户确认分镜。质检包括缺页、运行错误、布局/字幕安全区、关键帧和同一时间重复绘制的确定性。
- 通过参数数组调用可信的运行程序，不拼 shell 命令；运行目录固定为该任务的新目录。原音频只读，必要的转换写入任务临时目录。
- 用户看到阶段、已完成镜头/帧和错误。无可靠总进度时显示阶段，不编造百分比；禁止重复启动同一任务。
- 停止时宿主终止本任务所属的进程树；不得结束用户的普通 Chrome/Edge 或其他任务。Windows 子进程隐藏控制台。
- 失败保留字幕、分镜和已完成缓存，手动重试复用已验证阶段。不自动无限重试，不将中间 MP4 标为正式成片。

## 4. 产品入口

在书籍流程中增加「视频制作」，位于音频准备之后、发布准备之前。音频成品卡增加「用此音频制作视频」，按素材 ID 带入，不复制路径状态或自动改选最新音频。

视频制作页沿用现有墨绿/浅色控件，提供：

1. 当前音频选择、可用性、音频试听与运行环境检查。
2. 「生成字幕与分镜」，显示字幕校对和分镜预览；失败保留已完成内容。
3. 「确认并生成视频」、实际阶段、停止与手动重试。
4. 成片播放器、历史成片、打开目录、导出 SRT 和「用于发布」。

预览直接播放已校验的本地 MP4，支持暂停、拖动和全屏。采用按已登记素材 ID 授权的本地媒体读取，支持 Range/部分内容与正确 MIME；路径在 Rust 中解析，不把任意文件路径交给网页读取。不将长视频 base64 传过 IPC，也不把引擎 HTML/JS 嵌入有业务权限的主窗口。

字幕已烧录时播放器不重复显示软字幕。源码工程预览不作为首版要求，避免再维护一个带执行权限的浏览器环境。

## 5. 任务、素材和备份

只新增一个 video_jobs 业务表，不复用 NotebookLM 专用 audio_jobs 状态，也不建通用工作流平台。现已添加0014迁移，当前schema为v14；正式用户数据库尚未由本轮操作迁移。

记录项目/音频素材ID、音频摘要、字幕与分镜数据、运行代次、状态/阶段、错误、结果素材ID和时间。首版Prompt为video-storyboard-v2，P0已替换为v3且JSON形状不变；引擎仍为0.7.3。运行目录保存调用脚本/绘图/目录快照，未另加固定版本列。SQLite是唯一结构化业务状态；SRT和引擎工程是导出物或可重建缓存，不双向同步两套权威状态。

建议状态：draft、running、awaiting_confirmation、succeeded、failed、cancelled；识别/分镜/渲染/校验为阶段。重启后 running 转为需人工重试的失败状态，递增运行代次，旧进程结果不得提交。

每次新任务使用独立目录。输出通过容器/轨道/时长检查后，在一个 SQLite 事务中登记 MP4、SRT 和成功状态；重复完成返回同一结果。失败不覆盖旧成片、不删除源音频、不自动改变发布任务选择。

素材继续使用既有 assets；发布继续使用 audio_asset_id/video_asset_id。完整备份沿用当前「视频可选、默认不复制」规则，明确提示新生成 MP4 同样属于视频；字幕及业务数据需保留，渲染缓存可重建，不跟随 engine/engine_modules 目录链接备份外部依赖。实施时补充新表引用回填和运行中恢复限制测试。

## 6. 运行环境与许可边界

本机路径 `D:/MyWork/project/JIAZI-PROJECT/whiteboard-teacher-video` 只用于首次开发验证，不能写死在业务代码、发布包或默认配置中。首版在设置中选择已安装的独立引擎目录，并验证版本、Node、浏览器、FFmpeg、字体和可写输出目录；不可用时在生成前提示。

外层仓库 LICENSE 的自有脚本为 MIT，但本地 engine/package.json 明确为 `@frametell/engine 0.7.3 / AGPL-3.0-only`。本次读取上游 main 为 0.7.4，同样为 AGPL-3.0-only。不要把外层 MIT 理解成可以任意再许可引擎。

推荐开发阶段以用户独立安装的引擎作为外部工具，通过工程文件和进程通信接入，不先把引擎源码、依赖和字体复制进书引。进程隔离是技术边界，不保证自动豁免 AGPL 义务；正式捆绑或再分发前须确认组合使用方式、源码提供要求及字体/FFmpeg许可。需要不适用 AGPL 的发行方式时应取得适用授权，不能用改名或套进程规避。

因此「应用内完成操作」与「安装书引后零配置即可生成」是两个交付范围：前者可以先实现，后者还包含运行时/模型安装、许可和安装包验收。本轮不宣称后一项已具备。

## 7. 实施顺序与验收

| 顺序 | 完整业务切片 | 验收标准 |
| --- | --- | --- |
| 1 | 本书音频 → 字幕 | 实际音频识别、中文 SRT、时间码校验、试听校对、失败保留原件、重启读回 |
| 2 | 字幕 → 分镜 → 原声视频 | 拒绝非法模型输出；分镜可确认；短样片与一条完整书籍音频渲染成功；不替换配音 |
| 3 | 播放 → 素材 → 发布 | 应用内播放/拖动，SRT导出，结果事务唯一登记，明确选用成片，原发布/音频选择不被覆盖 |
| 4 | 交付可靠性 | 停止/重启、模型/引擎缺失、渲染失败、磁盘不足、中文路径、备份恢复、Windows独立安装验证 |

后续适用检查：`npm run lint`、`npm run typecheck`、`npm test`、`npm run test:rust`、`npm run check:rust`、`npm run build`，以及新增字幕/分镜/任务/进程/媒体 Range 测试。另做真实音频识别、成片抽帧/试听、桌面拖动播放，不能用单测或模拟 Provider 代替。

首版不做：重新配音、自动最终发布、云渲染、多项目并行、任意脚本执行、复杂时间线编辑和 macOS/Linux 已验收承诺。本轮功能影响为feat，不手动升版本；正式分发仍须单独验收。

## 8. 查证来源与本轮结果

- 本地：[素材实现](../src-tauri/src/database/assets.rs)、[当前 Provider](../src-tauri/src/ai.rs)、[发布检查](../src-tauri/src/database/publishing.rs)、[备份实现](../src-tauri/src/database/full_backup.rs)、[桌面安全配置](../src-tauri/tauri.conf.json)。
- 本地外部项目：`D:/MyWork/project/JIAZI-PROJECT/whiteboard-teacher-video/LICENSE`、engine/package.json、engine/tools/render.mjs、engine/tools/lib.mjs、engine/runtime/timing.js。
- [Frametell 上游 engine/package.json](https://github.com/sitor-ai/frametell/blob/main/packages/engine/package.json)：本轮读取版本和许可；动态 main 不是实施时的版本锁。
- [faster-whisper 官方 README](https://github.com/SYSTRAN/faster-whisper/blob/master/README.md)、[转写源码](https://github.com/SYSTRAN/faster-whisper/blob/master/faster_whisper/transcribe.py)、[下载实现](https://github.com/SYSTRAN/faster-whisper/blob/master/faster_whisper/utils.py)：本轮读取 CPU/int8、词级时间戳和模型目录参数。最新发行接口返回 [v1.2.1](https://github.com/SYSTRAN/faster-whisper/releases/tag/v1.2.1)，已额外核对其[标签源码](https://github.com/SYSTRAN/faster-whisper/blob/v1.2.1/faster_whisper/transcribe.py)中的时间戳和本地模型目录参数；实施须锁定实际测试的发行版本。

前置评估阶段只做源码/接口/许可核对；随后用户确认实施，现已安装D盘ASR环境/模型并完成隔离短片生成及桌面播放。没有调用付费模型、改变正式数据或打安装包。长片转写质量、真实模型分镜效果、整体性能和最终再分发合规仍需对应验证。

前置文档验证包含本地源码链接、设计入口、结构与空白检查。实施后的功能测试、构建和桌面验证以 [最新验收报告](MG-VIDEO-VERIFICATION.md) 为准，不将前置设计结果当作功能交付证据。
