# 原声 MG 视频实施与验收

日期：2026-11-01。范围：Windows 首版，独立 Frametell 0.7.3，中文原声，1920×1080 / 24 fps / 书卷风。

## 中文路径原生崩溃修复（2026-11-02）

本轮基线94808c9，初始工作区干净；检测到发布更新020a57c/v0.32.0后暂停，用户确认保留并继续，最终检查基于020a57c。截图对应《我的前半生》第二个工作目录中的原469条字幕、72镜、933.46秒原声任务；日志有完整309片段/22403帧的PLAN，紧接`Render process exited 3221226505`，布局检查无错误，尚未进入编码。

根因是Node24.11.0的原生`fs.rmSync`错误转换非ASCII路径。Frametell0.7.3在`--plan`退出事件删除`out/.render.lock`时触发0xC0000409，故完整PLAN会留在日志里且锁文件未清理。不是字幕/分镜内容、超时、FFmpeg、Windows路径长度或stdout截断；不应重做AI内容或修改书名。

### 根因与最小修复证据

- 原计划复制到隔离英文目录通过；同样文件放入`我的前半生 (2)`中文目录，Node24.11.0连续3次以相同原生退出码失败，309片段全部到达。去掉引擎/字幕/长输出，直接对同目录锁文件执行`fs.rmSync`仍失败；读写和`unlinkSync`通过。
- 只更换Node24.22.1，原中文目录计划连续3次通过；随后使用官方Node24.22.2同样3次通过，没有修改独立引擎。证据在`output/video-plan-failure/repro-*.json`、`exact-cleanup-result.json`；英文目录的Rust文件句柄/Job Object重放也通过，排除把一般进程收集误判为根因。
- 已核对 [上游修复61108](https://github.com/nodejs/node/pull/61108) 的源码：改为`path.ToPath()`正确处理Windows编码。首次修复版本是 [24.13.1](https://nodejs.org/en/blog/release/v24.13.1) / [25.4.0](https://nodejs.org/en/blog/release/v25.4.0)。
- 现有Node入口在启动门之后、任务输入/引擎/音频之前拒绝已知缺陷版本；不添加兼容渲染器、文件系统补丁或无限重试。PLAN只由程序消费，普通诊断stdout/stderr及非零退出仍保留。现有前端提示直接说明Node升级和原任务重试入口。

### 本轮实际验证

工程命令在会话PATH前置`D:/MyWork/software/shuyin-video-node/v24.22.2`后执行，Rust命令先加载`. ./.tools/env.ps1`。日志位于`output/video-plan-failure/`，不与下文历史结果混用。

| 命令/验证 | 实际结果 |
| --- | --- |
| 旧Node：`node output/video-plan-failure/repro-plan.mjs 3 '我的前半生 (2)'` | 3/3复现3221226505；同计划英文目录通过 |
| 官方修复版Node执行同一命令 | 3/3通过，309片段/22403帧，中文目录不改名 |
| `npm run test:video` | 20通过；新增版本边界、真实入口提前拒绝及长PLAN错误收集回归，3项初始回归先失败后通过 |
| `npx vitest run tests/video.test.ts tests/video-progress.test.ts tests/video-storyboard.test.ts` | 24通过 |
| `npx vitest run --maxWorkers 2` | 全量283通过，18个测试文件 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked video` | 23通过、1项现有进程树专项默认忽略 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked` | v0.32.0全量155通过、2项默认忽略 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked cancelling_video_worker_terminates_its_descendants -- --ignored` | 设置SHUYIN_TEST_NODE为官方24.22.2后，专项1通过；系统凭据专项未运行 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | 通过 |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets -- -D warnings` | v0.32.0通过 |
| `npm run lint` | 通过 |
| `npm run build` | 最终v0.32.0 TypeScript/Vite通过；保留既有560kB主包提示，未改规则/阈值 |
| 原生`video_process::run` + Node24.11.0 + 当前入口 | 明确拒绝并提供设置入口，未创建缓存或读取任务/调用引擎；不再原生崩溃或输出长PLAN |
| 原生`video_process::run` + 官方Node24.22.2 + 原完整计划 | 中文隔离目录完整成功；严格排版检查0错误，22403帧、469条硬字幕及原声合入，宿主退出0 |
| `python "$ffmpegSkills/probe.py" "$video" --json` | 933.459002秒、1920×1080、24fps、22403帧、H.264/yuv420p、AAC双声道44.1kHz，74660771字节 |
| `python "$ffmpegSkills/check.py" "$video" --platform custom --aspect 16:9 --no-loudness --content --json` | 全音视频解码检查8通过、1软字幕提醒、0失败；无软字幕因字幕已烧录，不作平台响度验收 |
| `python "$ffmpegSkills/look.py" "$video" --tiles 4x3 --width 1600 -o output/video-plan-failure/full-contact.png --dry-run --json`，再去掉`--dry-run`执行 | 生成并查看12格抽帧；与浏览器首尾截图一起核对中文绘图/字幕，没有把抽样当完整美学或听审验收 |
| `node output/playwright/video-plan-failure/playback-check.mjs` | 独立Chrome在5.27/466/932秒定位、实际推进播放成功，媒体/页面错误0；浏览器正常关闭 |
| `node output/video-plan-failure/verify-evidence.mjs` | 4份原件、5份引擎源码摘要不变；3份实际运行源码与当前仓库一致；7份文档/21个本地链接、成片/计划/Node摘要及空索引核对通过 |
| `git diff --check` | 通过 |

上述`$ffmpegSkills`为`C:/Users/haozi/.agents/skills/ffmpeg-skill/scripts`，`$video`为仓库内`output/video-plan-failure/原任务完整复验 (2)/video.mp4`；媒体检查会话PATH前置已有`D:/MyWork/software/ffmpeg/current/bin`，未另装FFmpeg。完整重放命令是：

```powershell
./output/video-plan-failure/native-repro.exe "$PWD/output/video-plan-failure/原任务完整复验 (2)" 'D:/MyWork/software/shuyin-video-node/v24.22.2/node.exe' 21600
```

此命令已经执行，不应覆盖现有gate/输出再次运行；继续复验需新建隔离运行目录。成片和`verified-evidence.json`是验证产物，没有登记到正式素材库或将原失败任务标为成功。`full-contact.png`使用媒体工具的输入seek，格内计时从裁取起点约38.894秒重新计算，不是原声绝对时间；准确首/中/尾定位以`output/playwright/video-plan-failure/playback-result.json`为准。部分镜头在已保存字幕锚点触发前留白，本轮保留分镜内容，不扩展为美学改稿。

便携Node下载自 [官方24.22.2](https://nodejs.org/dist/v24.22.2/win-x64/node.exe)，对照官方SHASUMS256核验为`1c331340fa6cac5e4294725959070a143fa0507195a78ad77eed7943f719aac9`，OpenJS Foundation签名有效。约89.6MiB，仅安装到独立D盘目录，不替换全局Node或写入仓库依赖。

本轮修改渲染入口、设置提示、视频Node回归及相应技术/开发/使用/验收/规划记录；无SQL迁移、模型调用、业务依赖、权限或手动版本号修改，不提交或发布，保留用户的v0.32.0提交。正式数据库、任务状态、原声、已有计划/成片、外部引擎和全局Node不改。当前安装版可在「视频运行环境 → Node 程序」选择下方新路径并保存，随后在原任务确认生成；早期版本检查和日志精简需要包含源码修复的新构建。

验收对应：原错误红绿对照、已知缺陷版本明确拒绝、原469字幕/72镜完整同源渲染、实际播放/解码、原件保留均已完成。未替换正式安装、未代改用户运行环境设置、未通过正式任务归档、未完整人工听审或验证真实模型/跨平台，也未重跑无关release/queue/ASR回归。原生重放进程树已结束；测试输出和独立Node在D盘，常规Node回归/独立Chrome播放仍可使用C盘系统临时目录，不承诺零C盘写入。

2026-11-02 P0续作已完成导演校对/显式字幕触发与同源试片。独立记录见 [P0实施与验收](VIDEO-DIRECTOR-VERIFICATION.md)，包含283前端/16Node/155Rust、真实30秒/尾段/83秒渲染、取消重启和同源对照证据。下文为历史切片记录，不代表本轮重跑了其中所有检查；P0主次与时机调整不再承诺旧版布局逐像素一致，85物体几何仍保持。

## 同款手绘生成迭代（2026-11-01）

用户要求生成结果与whiteboard-teacher-video保持一致，不接受只接引擎后输出标题/文字框。初始HEAD=a7502e0、工作区和索引干净。本轮移除旧绘制，保留原声、历史字幕/成片、确认闸门、素材事务和半自动发布，不改外部引擎或正式数据。

### 实现与一致性标准

- `video/illustrations.js`复用参考《我的前半生》可编辑工程实际使用的85物体几何、13构图、逐笔绘制、移动/倾覆/强调及字幕CSS；顶栏改为系列名和当前书名，不写死创作者或书名。6种受控物体动作沿用同一绘制代码。
- 原参考和当前`sketch`函数归一化后SHA-256均为`96a6bc4e09c4440682d68e97f417b1357cf38c7a0a327482084d8f590b36fe9b`，不是重新画一套近似图标。Node回归覆盖完整85物体×6动作的有限数值和跳转确定性。
- 唯一`visual-catalog.json`由Rust验证、v2 Prompt、Node和Vue共用。模型只选择`cue/object/action/label`和构图；按原字幕时码触发，附首尾台词/最近分镜上下文，不执行模型代码。最多16条字幕/镜、1—3物体、8字标签；不保留第二套文字框渲染。
- 旧plan缺beats仍可读。未完成任务明确「保留字幕并升级手绘分镜」；已完成任务「用原字幕制作新版手绘」，原音频/SRT创建新任务。旧作品和发布选择不改。无SQL迁移，仍v14，无新依赖/权限或手动升版。

### 真实画面与桌面闭环

参考输入只读截取83.666秒原声（先dry-run，精确PCM时长83.666009秒），使用原工程39条字幕/5镜物体计划，通过当前正式Node渲染入口输出83.667秒MP4：1920×1080、24fps、H.264/yuv420p、AAC双声道44.1kHz。计划及切片证据在`output/video-reference/input.json`、`cut-plan.json`、`cut-result.json`；成片为`output/video-reference/video.mp4`。已查看成片六格抽帧`contact.png`，中文字体、物体、标签和底部字幕正常，无明显遮挡。

`reference-parity.mjs`在同一引擎、同一编译分镜和页面ID下分别加载当前绘图与原参考绘图，于5/18/35/53/74秒截图。物体区域x=140/y=280/w=1640/h=600五处PNG字节完全相同，浏览器错误0；证据`reference-parity.json`及`app-*.png`/`reference-*.png`。这是这些时间点的物体区域一致性，不宣称不同顶栏/字幕或任意书籍整片每一帧像素相同。

隔离identifier为`com.shuyin.visual-verify`，未启动或写入正式profile。`native/native-check.mjs`通过真实Tauri IPC、SQLite、本地ASR、一次本地分镜契约请求、用户确认入口、独立引擎、原声合成和素材归档：20秒/9条字幕/3镜/6物体，真实帧进度持续更新。WebView2定位12秒并播放至12.5秒后暂停。分镜编辑与渲染面板在1440/1120/800/390四档均无横向溢出；390是WebView视口模拟，不代表改变原生最小窗口。减少动画时animation=none、transition=0s，页面错误0，源输入摘要不变。

最终隔离窗口正常关闭，18472/18473无监听；只读SQLite检查schema14、integrity=ok、外键错误0、running=0，成功任务和素材保留。证据`native/native-result.json`、`native/editor-*.png`、`native/player.png`、`database-and-sources.json`。测试产物/临时目录在D盘；隔离SQLite/WebView2仍使用C盘标准profile，未移动或删除原文件。

续验`native/upgrade-check.mjs`实际点击已完成作品的「用原字幕制作新版手绘」，在暂时清空隔离语音程序/模型路径的条件下仍复用原9条字幕成功准备新任务，确认未重新ASR。仅在新隔离任务中置入旧版labels数据后，界面明确提示升级并禁用旧模板渲染；点击升级时模拟模型返回不支持的物体，真实校验拒绝且字幕/原声摘要保留。再从界面继续，同一新任务run3生成3镜物体分镜并等待确认，未自动渲染。总计3次本地契约请求，原成功任务完整JSON、原视频及音频摘要均不变。脚本重复执行通过，两个新版隔离任务保留等待确认；没有写正式数据。截图已查看，证据`native/upgrade-result.json`、`native/upgrade-database.json`、`native/legacy-upgrade.png`及`native/upgraded-storyboard.png`；随后正常关闭隔离应用，数据库完整性/外键正常且running=0。

### 本轮实际命令

Rust先执行`. ./.tools/env.ps1`，原生验收使用本轮D盘TEMP/TMP目录。下表只记录本轮执行；后续章节保留先前版本历史，不代替本轮结果。

| 命令 | 实际结果 |
| --- | --- |
| `node --test tests/video-render.test.mjs` | 10通过：原声时轴、JSON隔离、具体物体/字幕锚点、旧模板拒绝、完整目录/动作、缓存计数、初始化/绘制就绪 |
| `npx vitest run tests/video.test.ts tests/video-progress.test.ts` | 17通过：确认/失败/重试、物体编辑、旧分镜升级、完成作品新建版本及真实进度 |
| `npx vitest run --maxWorkers 2` | 276通过，17个测试文件 |
| `npm run lint`、`npm run typecheck` | 通过，未关闭规则或弱化类型检查 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked` | 152通过、2项默认忽略；包含旧plan只读、禁止旧模板渲染、明确升级不丢字幕/摘要 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | 通过 |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets -- -D warnings` | 最终通过；初次新测试的一处clone建议已按规则修正 |
| `npx tauri build --debug --no-bundle --config output/video-reference/tauri.verify.conf.json` | 隔离桌面程序通过，包含真实`npm run build`/TypeScript/Vite；不是安装包或正式升级。保留约551kB主JS分包提示 |
| `node src-tauri/video/render.mjs output/video-reference/input.json output/video-reference/result.json` | 首次绘制/字幕通过后在合原声时发现中文输出名不匹配；修复后同输入成功，复用25/25片段 |
| `node output/video-reference/reference-parity.mjs` | 5处物体区域完全相同，浏览器错误0 |
| `node output/video-reference/native/native-check.mjs` | 真实Windows20秒闭环、四档编辑/渲染视口、原生播放、一次本地模型契约通过 |
| `node output/video-reference/native/upgrade-check.mjs` | 真实完成作品新建、复用字幕跳过ASR、旧分镜明确升级、非法模型输出拒绝/保留字幕、同任务继续成功；原成功任务/视频/音频不变 |
| `node output/video-reference/native/close-native.mjs`、只读Python SQLite检查 | 窗口/端口关闭，数据库完整性及外键通过 |
| ffmpeg-skill `probe.py`、`look.py --tiles 3x2` | 成片参数通过，抽帧已查看 |
| ffmpeg-skill `check.py --platform youtube`、`loudness.py --measure-only` | YouTube检查10通过/1提醒/1失败；无独立软字幕轨为提醒（画面已烧录）；响度-21.9LUFS不符合其-14±2目标，原输入测得-21.89LUFS。本轮按保留原声约束不擅自归一化，不声称平台交付检查全过 |
| `git diff --check` | 通过，索引不改 |
| `node output/video-reference/verify-evidence.mjs` | 当前原生内嵌3份运行源码/输入摘要/5处画面/样片元数据/数据库及8份文档25个本地链接一致 |

### 过程修复、范围和边界

新增Node物体输出/旧模板拒绝和Vue旧分镜升级回归均先失败，实施后通过。目录检查首次漏算椭圆图元而误报锁链，按真实绘图API补正断言，未改绘图规避。真实样片发现Frametell的META.lang=zh输出`film-有字幕.mp4`，旧代码仍读取英文文件名；核对锁定引擎源码后按单一中文输出契约修复，不增加查找/猜测/回退路径。失败日志保留为`render.log`，成功续跑为`render-retry.log`。

修改范围：`src-tauri/video/`、`src-tauri/src/video.rs`/`video_commands.rs`、数据库video回归、`prompts/video-storyboard-v2.md`（替代v1）、前端视频类型/页面/进度预览及对应测试；同步PRD、技术/数据库/AI规则、接入/使用/开发文档和三份规划记录。原声音频制作、发布实现和独立引擎未改。

本轮完成的是同款可信绘图、受约束自动分镜链路、参考原声样片和隔离桌面验收。Native分镜使用本地契约服务，83秒样片使用参考人工分镜；未调用真实付费模型，不能把这些结果冒充其逐镜语义选择已验收。整本书新分镜质量/完整听审、正式安装升级及macOS/Linux仍未验收。无关release/queue和Python转写回归未重跑，不声称完整`npm test`全绿；本轮没有提交、推送、发布、替换正式安装或修改版本号，功能影响为feat/minor。更新实际使用的应用构建后，再通过明确升级/新版入口制作；已运行的旧版本不会自动获得新绘图。

## 首帧一致性修复（2026-11-01）

### 复现与根因

用户截图对应《我的前半生》原任务：437条字幕、113镜、933.37秒；原 `layout-check.json` 在p0001中点（报告5.3秒）记录45.21%差异。只读原缓存，以及使用当前源码重新生成同一计划，均复现 `nondeterministic`。失败截图为空白底色/纸张与完整图解，而两次目标时间的DOM摘要相同、字体为loaded、活动动画0、引擎错误0。裁剪到一镜后连续20次不复现，不能以短样片代替这个加载条件。

Frametell 0.7.3的 `runtime/main.js` 在建DOM、加载字体和设置时间0之后就完成 `window.__ready`；检查立即截图，首次绘制尚未完成时读到了旧画面。取消CSS图层提示仍复现45.21%，不采用该方案。仅等待初始绘制的对照连续20次通过。已核对本机当前源码/README、Playwright 1.61.1的截图实现和double-rAF稳定截图实现；本轮读取的 [上游0.7.4清单](https://github.com/sitor-ai/frametell/blob/main/packages/engine/package.json)、[检查实现](https://github.com/sitor-ai/frametell/blob/main/packages/engine/tools/checks.mjs) 与 [页面样式](https://github.com/sitor-ai/frametell/blob/main/packages/engine/runtime/base.css) 中相关检查仍相同，不通过盲目升级解决。

### 修改与边界

- `src-tauri/video/render.mjs`：生成HTML在boot后扩展同一个ready承诺，等待两次绘制回调，保留原时长及错误。所有检查/编码工作进程共享；不改引擎源码、不加固定睡眠、逐帧等待或自动重试，检查阈值不变。
- `tests/video-render.test.mjs`：覆盖引擎未就绪不提前等待绘制、第一次回调仍不能开始捕获、就绪时长不变、原错误继续拒绝；保留初始化退出与规范路径测试。
- `tests/video.test.ts`：渲染失败后明确确认，只重试同一任务的render分支，传入原字幕/分镜，不新建任务；停止后仍可继续编辑原字幕。
- 文档同步技术设计、开发计划、本文及根目录三份规划记录。无SQL、依赖、权限或版本号变更，版本影响fix/patch。

正式任务/输入只读；新渲染输出全部写入 `output/playwright/video-determinism/full-render`。未更新或重启正式安装版，未写正式数据库、重新ASR、调用分镜模型、覆盖原音频或旧成片。界面仍使用「确认并生成视频」重试，不必点击「保存字幕并重新生成分镜」。内嵌脚本需要重新构建/更新应用才在正式任务生效。

### 本轮验证记录

Rust命令先执行 `. ./.tools/env.ps1`。

| 命令 | 实际结果 |
| --- | --- |
| `node --test --test-name-pattern='episode readiness' tests/video-render.test.mjs`（修复前） | 1失败，生成页面缺少初始绘制就绪契约 |
| `node --test tests/video-render.test.mjs`（修复后） | 7通过 |
| `node --check src-tauri/video/render.mjs` | 通过 |
| `npx vitest run tests/video.test.ts tests/video-progress.test.ts` | 14通过，含失败重试保留原字幕/分镜 |
| `node output/playwright/video-determinism/replay-original.mjs <修复后缓存目录> 20 fixed` | 使用真实生成HTML，重新打开浏览器20次，原113镜加载/首镜确定性检查20次全部通过；没有额外绘制等待 |
| `npm run lint` | 通过 |
| `npx vitest run --maxWorkers 2` | 最终273通过，17个测试文件 |
| `npm run typecheck` | 当前0.30.0通过 |
| `npm run build` | 保留用户0.30.0更新后重新执行，TypeScript/Vite通过；保留既有主JS大于500kB提示 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked video` | 17通过、1项进程树专项默认忽略；本轮未修改进程管理 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked` | 当前0.30.0全量149通过，系统凭据和进程树2项默认忽略，未声称其在本轮实跑 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | 通过 |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets -- -D warnings` | 通过 |
| `cargo build --manifest-path src-tauri/Cargo.toml --locked` | 当前0.30.0桌面开发程序构建通过，包含内嵌渲染脚本；未启动或替换正式安装 |
| `node src-tauri/video/render.mjs output/playwright/video-determinism/full-render/input.json output/playwright/video-determinism/full-render/result.json` | 退出0，原437条字幕/113镜完整成片生成，933.375秒、1920×1080、24fps、22,401帧、H.264/AAC双声道44.1kHz；79,341,677字节 |
| `node output/playwright/video-determinism/replay-original.mjs <修复后缓存目录> 1 positive-control` | 故意加入状态相关背景后退出1并报87.69%差异；真实不一致仍被原检查拒绝 |
| `python <ffmpeg-skill>/scripts/probe.py <成片> --json` | 读取完整音视频参数通过；音轨933.371995秒，与画面相差约3毫秒 |
| `python <ffmpeg-skill>/scripts/look.py <成片> --tiles 4x3 --width 1600 -o <contact.png> --no-timecode --json-brief` | 12帧全片抽样图生成并人工视觉复核；另提取5.27秒原失败位置与932秒片尾，图解/中文字幕可见且无明显遮挡 |
| `ffmpeg -hide_banner -v error -xerror -nostdin -i <成片> -map 0:v:0 -map 0:a:0 -progress <decode-progress.txt> -nostats -f null -` | 全部22,401帧和音轨解码通过，错误日志为空，progress=end |
| `node output/playwright/video-determinism/playback-check.mjs` | 独立Chrome本地文件播放通过，5.27/466/932秒定位均就绪，实际播放超过6.3秒，媒体/页面错误0；不是正式WebView2验收 |
| `node output/playwright/video-determinism/verify-evidence.mjs` | 4个原文件摘要、字幕/分镜、当前源码缓存键、20次fresh检查、负向校验、解码及播放证据全部核对通过 |
| `git diff --check` | 通过；9个修复文件，索引保持空 |

原计划完整113镜布局检查通过：错误0；9条文字占比偏高、1条内容偏稀的非阻断提醒保留，没有通过丢弃提醒或提高阈值获取通过。首帧就绪回归、20次原加载条件检查、真实不一致负向校验、失败重试保留原计划、完整成片解码和抽样视觉核验均完成。原输入JSON、原运行脚本、原布局结果与原音频SHA-256保持不变；验证成片没有自动登记到正式任务，也没有将正式失败状态伪造为成功。

初次媒体脚本因PATH未包含ffprobe退出127，随后使用已存在的 `D:/MyWork/software/ffmpeg/current/bin`，仅修改验证命令的PATH；未安装软件或改全局配置。诊断脚本和证据统一保留在已忽略的 `output/playwright/video-determinism/`，其中README注明仅作回归证据，不是另一套生产渲染路径。完整成片位于 `output/playwright/video-determinism/full-render/video.mp4`。

用户确认后保留发布提交e7f0e12（v0.30.0），未回退版本、修改索引、提交或发布。无数据库结构/依赖/权限变更。正式安装升级、正式任务内重新归档、完整人工听审、其他长片/两小时上限和跨平台仍未验收；本轮只证明这一真实15分33秒原计划的完整渲染及上述回归。未运行与本修复无关的release/queue测试，不声称完整 `npm test` 通过。下一步通过既有发布流程打包更新，再在原任务使用「确认并生成视频」，不必重做字幕或分镜。

## 实时进度续作（2026-11-01）

承接字幕识别反馈及追加的视频制作进度需求。保留之前未完成的实现，不重新实现任务调度、不修改外部引擎，不替换正在运行的正式安装。

### 实现与取舍

- 字幕显示真实音频位置、识别条数与最近三条预览；分镜显示已保存覆盖数和标题。统一五步流程、本次用时、进度环和停止/失败/确认/完成状态。
- 绘制进度来自引擎 `--plan` 的帧区间，以及正式缓存或本轮已写完整moov的临时MP4；不按文件大小、等待时间或工作线程启动数估算。取消遗留的旧临时片段不计入新运行，正式缓存可以计入。
- 画面绘制、字幕合成、原声合入、成片检查分别反馈。没有可靠计数的阶段保持不定进度，不提供虚假整体百分比或ETA；画面100%不代表成片已经归档。
- 沿用墨绿视觉和现有控件；开始制作后定位到任务面板，窄窗标题全宽、步骤文字分行，减少动态效果时停用动画。临时同步失败保留上次快照并提示。
- 真实复验发现字体别名从258增长到262字符后Chrome报ERR_FILE_NOT_FOUND，而140字符真实路径可读。生成HTML改用规范引擎/依赖file URL；引擎初始化错误直接退出1，由原Job Object回收进程树，不再因遗留浏览器连接长期保持running。

### 本轮实际验证

Rust命令先执行 `. ./.tools/env.ps1`。以下为本轮实际结果，后文首版实施表的旧测试数量保留为历史证据。

| 命令 | 实际结果 |
| --- | --- |
| `npx vitest run --maxWorkers 2` | 最终272通过，17个测试文件 |
| `npx vitest run tests/video-progress.test.ts tests/video.test.ts` | 13通过：真实数字/字幕、分镜覆盖、确认、结束/失败/停止、重试计时、跨阶段隔离、同步恢复、计时器清理和开始渲染后定位 |
| `node --test tests/video-render.test.mjs` | 6通过：原声时轴、模型文本隔离、缓存/已封口/未完成片段、大MP4盒头、旧运行临时片段隔离，以及初始化失败及时退出和规范URL |
| `python -B -m unittest discover -s tests -p test_video_transcribe.py` | 4通过，含语段反馈、三条预览上限和原子快照 |
| `npm run lint` | 通过；模板换行和Node显式导入按现有规则修正，无豁免 |
| `npm run build` | TypeScript/Vite通过；仍有主JS约548kB的既有分包提示，未提高阈值 |
| `node --check src-tauri/video/render.mjs` | 通过 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked` | 149通过，2项默认忽略 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked cancelling_video_worker_terminates_its_descendants -- --ignored` | 设置 `SHUYIN_TEST_NODE` 后专项1通过；另一项系统凭据测试未运行 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | 通过 |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets -- -D warnings` | 通过 |
| `npx tauri build --debug --no-bundle --config output/playwright/video-progress/tauri.verify.conf.json` | 隔离桌面程序构建通过，不是安装包 |
| `node output/playwright/video-progress/native-check.mjs` | 首次真实IPC/ASR/分镜夹具/渲染/归档通过：20秒、9条字幕、3镜、480帧；一次本地模型契约请求 |
| `node output/playwright/video-progress/native-check.mjs a318311054f2d43682078d83bab28944` | 在先前字体失败的同一长目录、同一任务上从run2恢复到run3成功；未重新转写或调用模型。真实帧数0→80→181→308→366→424→452→480，最终成片1920×1080/20秒；原音频摘要不变 |
| `node output/playwright/video-progress/cache-check.mjs` | 实际复用10/10片段，绘制快照480/480，随后继续合成而非误报任务完成 |
| `node output/playwright/video-progress/close-native.mjs` | 正常关闭隔离应用，最终18462/18463无监听 |
| `python -B -`（只读SQLite/文档证据检查） | schema14、integrity=ok、外键错误0、原任务succeeded/run3、running任务0；8份Markdown、12个本地链接及原生/缓存/截图证据通过 |
| `git diff --check` | 通过 |

最终原生1440/1120/800/390四档视口均无页面或面板横向溢出；390为WebView视口模拟，未改变原生800px最小窗口。截图已复核，窄窗标签不再挤在一起；页面运行错误0，reduced-motion下动画为none、transition为0s。证据为 `output/playwright/video-progress/native-first-result.json`、`native-result.json`、`cache-result.json`、`render-panel.png` 及 `render-{宽度}.png`。

过程失败同样保留：新增回归先复现缺少渲染百分比、遗留临时片段被误计及初始化错误后4秒不退出；修复后全部通过。首轮lint/TypeScript错误已修复。第二次原生运行在262字符字体链接处超时，文件存在性及Chrome对照结果保留在 `font-result.json` / `native-long-path-failure.json`，随后只停止该隔离任务并在原目录成功复验。未通过改书名或缩短测试目录避开问题。

### 文件、数据和边界

- 前端：`src/modules/video/VideoJobProgress.vue`、`src/modules/video/VideoPage.vue`、`src/types/video.ts`。
- 后端/脚本：`src-tauri/src/video.rs`、`src-tauri/src/video_commands.rs`、`src-tauri/video/transcribe.py`、`src-tauri/video/render.mjs`。
- 回归：`tests/video-progress.test.ts`、`tests/video.test.ts`、`tests/video-render.test.mjs`、`tests/test_video_transcribe.py`。
- 文档：`docs/PRD.md`、`docs/TECHNICAL-DESIGN.md`、`docs/USER-GUIDE.md`、`docs/DEVELOPMENT-PLAN.md`、本文及根目录 `task_plan.md` / `progress.md` / `findings.md`。

本轮无数据库结构、迁移、依赖、权限或版本号变更，SQLite仍v14；瞬时快照不写入数据库、素材或备份。只在既有 `com.shuyin.mg-verify` 隔离库创建/恢复验收记录；正式库、原音频、旧成片及外部引擎源码未改。沿用已有D盘运行时，测试产物在D盘仓库output，隔离SQLite/WebView2沿用C盘标准profile。

隔离应用已正常关闭，数据库检查见 `output/playwright/video-progress/database-result.json`，最终Rust全量日志在同目录 `rust-check.log`。原正式安装进程保持运行，开始时已有的两处Python字节码缓存未删除，Git索引未改动。

验收对应：真实计数/内容反馈、重试隔离、异常结束、窄窗与减少动画、短片归档均有上述回归或实跑证据。整本书长片、真实分镜模型质量、完整试听、正式安装升级及跨平台仍未验收；不把本地契约服务当作真实AI服务。不重跑本轮未修改的release/queue测试，未声称完整 `npm test` 全绿；后文历史队列失败未在本轮修复或复验。进度为feat/minor兼容增强，附带可靠性修复，不手动升版本、不提交、不发布。

下一步是长片试用与按既有发布流程打包更新；已经运行的旧安装版不会自动获得这次源码中的进度面板。

## 已实现

- 音频成品卡的「制作 MG 视频」及书籍内「视频制作」页面；明确选用本书音频或已关联的 SRT。
- 本地 faster-whisper 转写，按词级识别时间断句，验证后导出 UTF-8 SRT；试听、校对、单独保存及另存导出。初始字幕文件保留，不用音频指导冒充逐字稿。
- 复用 TextProvider 与系统凭据；每批24条字幕生成有界 JSON 分镜，保存已完成批次。支持书籍、观点、对比、过程、时间线五类图解模板；可修改短标题/标签，须点击确认才渲染。
- 统一的原声绝对时间轴供 Node 和浏览器使用，逐页布局/确定性检查，错误拒绝编码；通过独立引擎渲染并合入原音频。不调用 TTS，不改变语速/响度，不加配乐；输出 AAC 编码，不保证音频字节不变。
- SQLite 视频任务、单活动任务、停止、重启中断识别、手动重试和分镜/渲染缓存。Windows Job Object 约束整棵子进程树；取消与旧运行结果使用状态及运行代次隔离。
- Tauri 自带 asset 协议，默认空 scope，仅通过本书已登记的音频/视频 ID 允许单个文件。原生播放器支持播放、暂停、定位和全屏，不把大视频转成 base64。
- MP4 与对应 SRT 在同一事务登记；旧成片/源音频不覆盖。通过「用于发布」为新草稿明确选择本任务视频和原音频，不改既有发布任务。

## 运行环境准备

引擎和语音运行环境不随书引安装包捆绑。Node推荐24.22.2 LTS，24.x至少24.13.1，避免旧版的Windows中文路径崩溃；另需Chrome/Edge，以及 whiteboard-teacher-video 的完整 engine 0.7.3、字体和 FFmpeg。引擎目录指向 `engine/`，不是外层仓库根目录。

语音环境在本机 Python 3.12 验证；依赖锁定在 `scripts/video-requirements.txt`。首次准备可从项目根目录运行：

```powershell
./scripts/setup-video.ps1 -Directory 'D:/ShuyinVideoRuntime'
```

脚本在选定目录安装 Python venv、缓存和 small 模型，不修改系统 Python 包，不安装或复制 AGPL 引擎。下载需要网络；转写阶段只读取本地完整模型，不下载缺失 tokenizer。模型锁定到 Systran/faster-whisper-small 的 `536b0662742c02347bc0e980a01041f333bce120`。

本机已准备的路径，可填入「视频运行环境」（2026-11-02仅更新Node，不替换全局安装）：

| 设置 | 本机路径 |
| --- | --- |
| 引擎 | `D:/MyWork/project/JIAZI-PROJECT/whiteboard-teacher-video/engine` |
| Node | `D:/MyWork/software/shuyin-video-node/v24.22.2/node.exe` |
| Python | `D:/MyWork/project/JIAZI-PROJECT/shuyin-ai/.tools/video-runtime/Scripts/python.exe` |
| 模型 | `D:/MyWork/project/JIAZI-PROJECT/shuyin-ai/.tools/video-model-small` |

首版记录中的运行环境、模型和下载缓存合计约869 MiB，均在 D 盘；此数不包含2026-11-02新增的约89.6MiB独立Node。沿用已有 C 盘 Python 基础解释器和 Chrome。首版隔离桌面验收的 SQLite/WebView2 profile 位于 `C:/Users/haozi/AppData/Local/com.shuyin.mg-verify`，约69 MiB；这不是正式书引数据。首版生成文件在仓库 `output/video-smoke/`。未移动或删除用户原文件。

## 数据库与权限

增加迁移 `src-tauri/migrations/0014_video_jobs.sql`，schema v14。表保存书籍/源音频/字幕/模型引用、状态、阶段、运行代次、唯一 plan_json、错误和结果素材 ID；活动任务有唯一索引。仍使用 assets，不建立第二套媒体库。

备份恢复保留任务和字幕/分镜数据，回填后的素材 ID 不变；原声/字幕随既有素材规则备份，MP4 仍受「包含视频」选项控制。缓存未登记为素材，可重建，不递归备份外部 engine 链接。运行中的源素材不能重新定位/移除，书籍不能删除，恢复数据库须先停止视频任务。

仅主窗口新增 allow-video capability。启用已锁定 Tauri 的 protocol-asset（新增其传递依赖 http-range），没有添加任意 shell/文件访问前端权限。外部进程参数数组复用已有 Windows shell_path，数据库路径原值不改写。

## 实测证据

隔离 identifier 为 `com.shuyin.mg-verify`。从已有《人生由我》原声只读截取约20秒作为输入；没有改写原片，也没有在正式库创建书籍或任务。

- `output/video-smoke/native-check.mjs`：真实 Tauri IPC、SQLite、本地 ASR、独立引擎、FFmpeg、字幕导出及 WebView2 播放链路通过。分镜使用本机 HTTP 契约服务（一次请求），不是付费真实模型验收。
- 生成8条字幕、3镜、20秒 MP4；WebView2 读取为1920×1080，定位12秒后实际播放到13秒以上；1440/1120/800/390 四档视口均无横向溢出，页面运行错误0。低于800px是 WebView 视口模拟，不代表修改原生窗口最小宽度。
- 独立渲染探针：H.264 High、yuv420p、24 fps、AAC双声道44.1kHz。引擎布局检查为空；FFmpeg默认 YouTube 技术检查0失败，2提醒：无独立软字幕轨（已经烧录且另有SRT）、保留原声约-21.6 LUFS（未擅自归一化）。这不是 YouTube/抖音发布验收。
- 已查看 `output/video-smoke/render2/contact.png` 和桌面截图：中文字体正常，字幕在底部安全区，图解无明显文字遮挡。识别曾将书名写成「人生有我」等，必须试听校对；未把可播放等同于逐字准确或成片内容审核通过。

- 最终正常关闭隔离主窗口后重新构建并启动，`output/video-smoke/restart-check.mjs` 通过：任务仍为 succeeded，字幕逐字回读一致，已登记媒体可加载，历史任务标签正确显示「成片已保存」，跨书素材访问被拒绝。四档视口再次通过；随后实际定位12秒并播放至12.499秒，readyState=4，已查看 `output/video-smoke/final-player.png`。
- 验收结束后正常关闭隔离应用；只读检查其数据库schema=14、integrity_check=ok、无外键错误且成功任务保留。18452/18453本地验收端口已无监听，未留下本轮服务。

## 实际验证命令

以下为本次功能实施期间实际执行的命令及结果，不代表每次文档修改都重跑全套。Rust 命令先加载 `. ./.tools/env.ps1`，并将本次进程 TEMP/TMP/SQLITE_TMPDIR 指向仓库 `output/video-temp`。

| 命令 | 结果 |
| --- | --- |
| `npm run lint` | 通过，无规则豁免 |
| `npx vitest run --maxWorkers 2` | 254通过，覆盖确认后渲染、保存失败保稿、历史标签、跨书迟到轮询及发布素材选择 |
| `npm run test:video` | Node 2通过：原声绝对时间轴、模型文本不成为可执行代码 |
| `python -m unittest discover -s tests -p test_video_transcribe.py` | 3通过：词时间码、本地模型约束、不兼容依赖和缺失模型拒绝 |
| `npm run test:release` | 78通过，1项macOS检查按平台跳过；日志 `output/video-smoke/release-check.log` |
| `npm run test:queue` | 18通过、1失败；未修改的「缺失前置报告时重跑」用例未重跑后继任务 |
| `node --test --test-name-pattern='a missing predecessor report' tests/codex-queue.test.mjs` | 同一队列失败聚焦复现，未修改脚本或测试规避 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked` | 146通过、2忽略；日志 `output/video-smoke/rust-check.log` |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked cancelling_video_worker -- --ignored` | 设置SHUYIN_TEST_NODE后专项1通过，证明取消后子进程不能继续执行；全量默认忽略的另一项为既有系统凭据测试 |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets -- -D warnings` | 通过 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | 通过 |
| `npm run build` | TypeScript及Vite构建通过；保留约538KB主JS块的大小提示，未调高阈值或关闭告警 |
| `npx tauri build --debug --no-bundle --config output/video-smoke/tauri.verify.conf.json` | 隔离identifier桌面程序构建通过；不是安装包构建或正式安装升级 |
| `node output/video-smoke/native-check.mjs` | 真实本机ASR/渲染/素材/SRT/播放链路通过，分镜使用本地契约服务 |
| `node output/video-smoke/restart-check.mjs` | 重启回读、媒体可读、任务标签、跨书访问拒绝及四档视口通过 |
| `node output/video-smoke/close-native.mjs` | 正常关闭隔离窗口；随后只读Python检查数据库通过 |
| `git diff --check`、`git diff --cached --check` | 工作区与暂存区空白检查通过 |

默认 `npm test` 曾在并发构建/桌面验收期间触发既有更新页面用例5秒超时；限制前端测试并发后完整254项通过，没有增加超时。队列用例仍单独失败，因此不能把整套 `npm test` 声称为全绿。发布脚本测试使用本地临时Git夹具，不是实际发布书引。

短片还执行了 ffmpeg-skill 的 `probe.py`、`check.py`、`look.py`，输入为 `output/video-smoke/render2/video.mp4`；原声截取先执行 `cut.py --dry-run --json` 再执行真实截取，不覆盖源文件。媒体参数和两项提示见实测证据。

## 修改范围与验收对应

| 范围 | 主要文件 | 验收结论 |
| --- | --- | --- |
| 字幕、分镜、任务及子进程 | `src-tauri/src/video.rs`、`src-tauri/src/video_commands.rs`、`src-tauri/src/video_process.rs`、`src-tauri/video/` | 短音频真实链路及聚焦回归通过，整本书性能和真实模型质量待验 |
| 数据、恢复、媒体授权 | `src-tauri/src/database/video.rs`、`src-tauri/migrations/0014_video_jobs.sql`、既有assets/backup/projects及Tauri配置 | 迁移、取消、事务、重复完成、恢复与同书授权测试通过 |
| 视频页面及流程接入 | `src/modules/video/VideoPage.vue`、`src/services/video.ts`、`src/types/video.ts`、音频/发布页、路由与导航 | 字幕校对/导出、确认、失败保稿、成片播放与新草稿明确选用通过 |
| 运行环境与回归 | `scripts/setup-video.ps1`、`scripts/video-requirements.txt`、`prompts/video-storyboard-v1.md`、前后端video测试 | 本机依赖/模型已准备并验证；独立安装脚本的全新机器执行未验收 |

已完成确认范围内的Windows首版与本机短片闭环。设计中的整本书长片、完整听审、正式安装/升级、跨平台及再分发许可确认仍是后续验收，不计为本次已通过。

## 过程中修复的实际问题

1. faster-whisper 1.2.1 使用 PyAV 的 metadata_errors 参数，而自动解析的新 PyAV 19.0.2 已移除；核对源码后锁定并验证 av 16.1.0，没有 monkey patch 或双路径回退。
2. Windows canonicalize 的扩展路径直接作为 Node 主脚本参数触发 `EISDIR ... lstat 'D:'`。复用项目原有 shell_path 处理外部进程参数和输入，不重写素材路径；子进程测试覆盖扩展路径。
3. 单独调用渲染脚本使用相对输入路径时，FFmpeg cwd 导致二次拼接。统一将入口输入/输出解析为绝对路径。
4. Reka SelectItemText 仅在挂载时登记 textContent，任务状态变化后默认 SelectValue 仍显示旧状态。按当前源码支持的默认 slot 显式绑定单一任务标签，回归先失败后通过，不通过反复重挂载组件修补。
5. 后台轮询在切书后迟到会回填旧书任务；回归复现后将响应限定在发起时的项目 ID，避免跨书回填。

## 验收边界

当前通过的是本机短音频链路与自动化回归。整本书长片性能、真实文本模型的分镜质量、完整人工听审、正式安装/升级，以及 macOS/Linux 未验收。首版模板化图解不等于原项目手工逐镜创作的专属人物/道具动画。

Frametell 仍为 AGPL-3.0-only，外层脚本 MIT 不改变引擎许可。没有复制或捆绑引擎；正式再分发仍需确认相关义务。未提交、推送、发布或手动升版本，功能影响为 feat。

用户已确认当前暂存区为手动暂存；助手保留索引不变。本次最后的验收文档和规划状态补充保持未暂存，不代替用户重新暂存或提交。
