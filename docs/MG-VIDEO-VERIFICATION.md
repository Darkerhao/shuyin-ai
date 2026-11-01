# 原声 MG 视频实施与验收

日期：2026-11-01。范围：Windows 首版，独立 Frametell 0.7.3，中文原声，1920×1080 / 24 fps / 书卷风。

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
