# 原声 MG 视频实施与验收

日期：2026-11-01。范围：Windows 首版，独立 Frametell 0.7.3，中文原声，1920×1080 / 24 fps / 书卷风。

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

引擎和语音运行环境不随书引安装包捆绑。需要已安装的 Node 22.19+、Chrome/Edge，以及 whiteboard-teacher-video 的完整 engine 0.7.3、字体和 FFmpeg。引擎目录指向 `engine/`，不是外层仓库根目录。

语音环境在本机 Python 3.12 验证；依赖锁定在 `scripts/video-requirements.txt`。首次准备可从项目根目录运行：

```powershell
./scripts/setup-video.ps1 -Directory 'D:/ShuyinVideoRuntime'
```

脚本在选定目录安装 Python venv、缓存和 small 模型，不修改系统 Python 包，不安装或复制 AGPL 引擎。下载需要网络；转写阶段只读取本地完整模型，不下载缺失 tokenizer。模型锁定到 Systran/faster-whisper-small 的 `536b0662742c02347bc0e980a01041f333bce120`。

本轮已准备的路径，可填入「视频运行环境」：

| 设置 | 本机路径 |
| --- | --- |
| 引擎 | `D:/MyWork/project/JIAZI-PROJECT/whiteboard-teacher-video/engine` |
| Node | `D:/MyWork/software/nodejs/node.exe` |
| Python | `D:/MyWork/project/JIAZI-PROJECT/shuyin-ai/.tools/video-runtime/Scripts/python.exe` |
| 模型 | `D:/MyWork/project/JIAZI-PROJECT/shuyin-ai/.tools/video-model-small` |

运行环境、模型和下载缓存合计约869 MiB，均在 D 盘。沿用已有 C 盘 Python 基础解释器和 Chrome。隔离桌面验收的 SQLite/WebView2 profile 位于 `C:/Users/haozi/AppData/Local/com.shuyin.mg-verify`，约69 MiB；这不是正式书引数据。实际生成文件在仓库 `output/video-smoke/`。未移动或删除用户原文件。

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
