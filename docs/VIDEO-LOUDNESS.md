# 视频导出响度修复（2026-11-14）

## 需求与验收标准

用户反馈生成视频音量偏低，确认在默认导出中加入自动响度标准化。修复前最后一次合成仅将原音频转为 AAC，没有音量处理；播放器音量只控制试听。目标是提高较轻的人声，让整书、章节、手动及自动制作共用现有导出逻辑，不增加配置、AI 调用或第二套渲染器。

1. 完整成片以 -16 LUFS 为响度目标；验收样片允许 ±1 LU。编码后真峰值不高于 -1.5 dBTP。遇到无法同时满足的高动态输入，峰值优先，不承诺每条输入恰好达到 -16 LUFS。
2. 试片使用整段来源的测量、动态处理和峰值修正结果，再按选定时间窗截取；不单独把短片提升到 -16 LUFS。同一片段与完整成片差异应小于 0.3 LU。
3. 原文件摘要、字幕时间码和画面不变；音频转换为 48 kHz 双声道 AAC 192 kbps，AAC 时间窗允许一个编码帧的边界误差，保持现有成片时长校验。
4. 完全数字静音保持静音；测量/编码错误明确失败，不归档成片，不覆盖已有文件；正常完成或可捕获异常时清理临时音轨。取消仍由原 Windows Job Object 终止整个进程树，强制终止可能保留本次任务目录中的临时文件。
5. 界面显示整段音量分析、调整和校验的真实进度；完成音轨处理不等于视频任务归档成功。

## 实现

`src-tauri/video/render.mjs::muxVideoAudio` 是唯一音轨出口。先对整段原声执行 FFmpeg loudnorm 测量，再使用 measured_I/TP/LRA/thresh 和 offset 执行第二遍。优先线性增益；不满足峰值或动态范围条件时，由 FFmpeg 使用动态模式。LRA=11，滤镜 TP=-2 dBTP，为编码预留 0.5 dB。测量和处理前共用 48 kHz 双声道格式，以免单声道在输出转为双声道后改变响度。

先在本次任务目录生成完整的临时 M4A，实测编码后的真峰值。若超过 -1.5 dBTP，只允许从原音频重新编码一次，按实测超出量再预留 0.2 dB 降低增益；第二次仍超标则停止。不会重复转码已经有损编码的音轨，也不会无限重试。正常路径需两遍处理和一次编码后测量，较旧版增加本地音频处理时间与少量临时音轨空间。

完整视频与试片最终均复制处理后的 AAC 包和现有视频流。试片只在这一步按时间窗截取，保留同一全片基准。临时文件位于本次新建的独立子目录，正常完成或可捕获异常时由 `finally` 清理；取消、超时或进程崩溃会跳过 JavaScript 清理，残留文件仍位于该次任务目录，不入库、不复用。沿用现有任务目录保留规则，不增加第二套清理状态。原音频、旧成片与正式素材不改写。播放器仍用于原声试听，其音量不参与导出。

现有 Rust 入口在同一 `render.mjs` 中执行完整/试片合成；渲染缓存键及试片签名已包含脚本源码，代码变更自然使旧缓存失效。无 Rust API、数据库、迁移、依赖、权限或版本号变更。归类为 `fix` / patch。已经归档的旧视频不会自动变响，需重新制作新版；本轮不增加旧视频批量处理入口。

## 依赖依据

本机独立引擎为 Frametell 0.7.3，实际 `bins.mjs` 选择 ffmpeg-static 中的 FFmpeg 6.1.1；另用本机 FFmpeg 8.1.1 核对兼容性。已读取引擎实际解析/调用源码，以及 FFmpeg 的当前主干与所用版本源码和公开参数：

- [6.1.1 loudnorm 源码](https://github.com/FFmpeg/FFmpeg/blob/n6.1.1/libavfilter/af_loudnorm.c)
- [6.1.1 滤镜文档](https://github.com/FFmpeg/FFmpeg/blob/n6.1.1/doc/filters.texi)
- [当前 loudnorm 源码](https://github.com/FFmpeg/FFmpeg/blob/master/libavfilter/af_loudnorm.c)
- [当前公开参数说明](https://ffmpeg.org/ffmpeg-filters.html#loudnorm)

6.1.1 实现会在不满足线性条件时使用动态限幅，并在动态模式下升采样到 192 kHz，因此输出显式重采样到 48 kHz。不能仅依据滤镜 TP 宣称最终 AAC 峰值合格，必须实测编码结果。

## 实现阶段验证记录

证据目录：`output/video-loudness/`（Git 忽略）。使用已有 Node 24.22.2；机器默认 Node 24.11.0 存在项目已明确拒绝的 Windows 中文路径问题，不改全局 Node。

- 修复前：从现有合成代码提取同一函数，真实 FFmpeg 回归失败，导出低音量夹具仍为 -36.23 LUFS，证明原路径没有提升响度（`red.log`）。
- 首次加入 loudnorm 后：低音量/试片通过，但峰值夹具实测 -1.18 dBTP，验证 AAC 编码超调，随后增加编码后校验及一次有限修正。
- 接续时旧日志中静音分支的 NaN 错误已由原实现修复；重新运行最终代码确认静音保持静音。首次续跑前端536通过，视频42通过/2失败：编码次数测试误将重复零进度算作重编，已改为统计实际阶段切换，仍要求最多两次；另发现 FFmpeg 6.1.1 在 `-n` 拒绝已有输出时打印错误但退出码为0，原文件未改却可能误报成功。独立最小复现确认后，在合成入口直接拒绝已存在的目标，保留 FFmpeg 的禁止覆盖参数。

### 最终实际命令与结果

Node 测试使用 `D:/MyWork/software/shuyin-video-node/v24.22.2`，仅加入当前命令的 PATH；`VITEST_MAX_WORKERS=2`。真实 FFmpeg 用例需要 `SHUYIN_TEST_FFMPEG`，本轮设为 `D:/MyWork/project/JIAZI-PROJECT/whiteboard-teacher-video/node_modules/ffmpeg-static/ffmpeg.exe`（6.1.1）。未配置该变量的普通测试会明确跳过4项音频集成测试，本轮无此跳过。

| 实际命令 | 结果与证据 |
|---|---|
| `npm test` | 前端37文件536项、视频44项（含4项真实音频）、发布/音频85项、队列19项通过；仅1项原有 macOS 专属测试跳过。`final-tests.log`，退出0 |
| `npm run lint` | 通过；`final-lint.log`，退出0 |
| `npm run build` | TypeScript检查及Vite构建通过；`resumed-build.log`，退出0。保留原有大于500kB产物提示及本机npm配置提示 |
| `node --test tests/video-audio.test.mjs` | 改用本机 FFmpeg 8.1.1，4通过/0跳过；`ffmpeg-8-tests.log`，退出0 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked video_process::tests::cancelling_video_worker_terminates_its_descendants -- --ignored --exact` | 先加载 `.tools/env.ps1` 并设置 `SHUYIN_TEST_NODE` 为上述Node；Windows真实子进程取消测试1通过，`cancellation-test.log`，退出0。此为定向验证，不代表本轮重跑全量Rust测试 |
| `node output/video-loudness/verify-real-audio.mjs` | 实际调用产品 `muxVideoAudio` 处理已有完整口播与30秒试片，响度/峰值/音轨时长/原文件及视频流校验通过；`real-audio.log`、`real-audio-result.json`，退出0 |
| `git diff --check` | 通过 |

### 真实口播验收

使用已有《我的前半生》口播原声 `D:/films/视频风格实验/09-我的前半生-溥仪/audio/source.MP3`，完整长度933.459563秒，并复用对应已烧录字幕的画面。所有输出只写入本轮 `output/video-loudness/`，没有替换原片或调用模型。

| 指标 | 实测 |
|---|---|
| 完整原声响度 → 成片响度 | -21.19 → -16.01 LUFS，提高5.18 LU |
| 成片编码后真峰值 | -1.79 dBTP，满足不高于-1.5的上限 |
| 完整成片 | H.264、1920×1080、24fps；AAC、48kHz、双声道；容器933.500秒，与源时长差约0.040秒，满足现有0.2秒校验 |
| 720—750秒相同片段 | 从全片截取为-13.92 LUFS，试片为-13.93 LUFS，差0.01 LU；均为-1.89 dBTP |
| 30秒试片音轨 | 实测30.016秒，偏差小于一个48kHz AAC编码帧 |
| 原声与画面 | 原声SHA-256不变，完整成片的视频流SHA-256与输入一致，烧录字幕画面保持 |
| 临时轨与耗时 | 正常完成后无临时音轨目录；本机完整音轨处理与合成约130秒（不含画面渲染及独立复测，受并行测试负载影响） |

实录试片的画面是从已有视频按关键帧复制出的独立夹具，画面长度30.209秒；本项验证的是实际合成函数的音轨时长和全片增益一致性，不把该画面夹具计为应用完整试片渲染验收。完整视频流保持及精确时间窗回归另由上述真实音频测试覆盖。

### 独立评审与验收对应

- Standards：首轮1项P2（退出清理承诺过宽），文档按实际取消语义修正，复审0项待修。
- Spec：首轮1项P2（已有输出被误报成功），最小复现后添加入口拒绝，复审0项待修；独立使用FFmpeg 6.1.1复跑4项音频测试全部通过。
- 验收1：低音量、尖峰夹具及实录完整口播通过；高动态输入仍按峰值优先。
- 验收2：动态夹具与真实口播同段试片均通过，全片响度不会被短片单独重置。
- 验收3：原文件和视频流摘要一致，原渲染时间轴回归通过，实测输出音频参数及时间窗符合要求。
- 验收4：静音、缺失输入、已有输出、正常/异常清理回归通过；Windows取消子进程实测通过。强制终止可能残留任务临时文件的边界已明确。
- 验收5：组件测试覆盖完整视频与试片三个音频阶段，百分比及文案来自实际进度，音轨处理结束不会提前显示归档成功。

无数据库迁移、依赖、权限、Rust API或版本号变化；未提交或推送。本轮未重打安装包、未做正式桌面归档或完整画面重新渲染验收，也未验证用户反馈的具体成片、平台转码、手机外放及主观听感。下一步在打包后的桌面版本使用反馈样片核对导出与试听；已生成的旧成片须重新制作才会采用新响度。

## 追加独立评审与补测（2026-11-14）

用户追加要求完成后评审测试。本次以 `746a7e3a0dd890fa9983db19093347d6ef6e6c34` 为固定基线，评审工作区全部11份变更文件（包括两份未跟踪文件），没有把上方实现阶段的验证记录当作本次执行结果。本次仅补充 `tests/video-audio.test.mjs` 和本文档，未修改产品实现。

- Standards：初审1项P2验证缺口。旧错误测试在临时音轨创建前退出，不能证明音轨创建后的异常清理；文件权限错误也缺少覆盖。新增真实FFmpeg合成失败清理和Windows写入权限错误两项回归后，独立复审0项待修。
- Spec：0项待修。核对唯一合成入口、全片基准、有限峰值修正、原声摘要检查、任务归档与进度调用链；另用真实FFmpeg 6.1.1验证6组整数/小数起点的试片，解码时长误差0.0043—0.0197秒，小于一个48k AAC帧；6组尖峰边界试片与全片同段响度差最大0.26 LU，试片真峰值最高-1.75 dBTP。

权限回归仅在新建且已核实位于系统临时目录下的测试目录设置拒绝写入ACL，`finally` 移除本测试添加的规则。首轮5通过/1失败：产品已按预期拒绝写入，但测试在恢复权限前枚举目录而收到EPERM；随后将文件状态检查移到恢复权限后，并明确断言错误必须来自 `mkdtemp`，没有弱化错误断言或修改产品代码。原失败日志保留为 `audio-tests.log`。

本次证据目录为 `output/video-loudness-review/`（Git忽略）。Node仍使用24.22.2，仅调整当前命令的PATH；全套测试设置 `VITEST_MAX_WORKERS=2`。`SHUYIN_TEST_FFMPEG` 分别指向上方列出的6.1.1引擎和 `D:/MyWork/software/ffmpeg/current/bin/ffmpeg.exe`（8.1.1）。

| 本次实际命令 | 结果与证据 |
|---|---|
| `npm test`（新增两项补测前） | 前端37文件536项、视频44项、发布/音频85项、队列19项通过；仅1项macOS专属检查跳过，退出0。`tests.log` |
| `npm run test:video`（补测后，FFmpeg 6.1.1） | 46通过、0失败、0跳过，退出0。`video-final.log` |
| `node --test tests/video-audio.test.mjs`（补测后，FFmpeg 8.1.1） | 6通过、0失败、0跳过，退出0。`audio-ffmpeg8.log` |
| `npm run lint`（补测后） | 通过，退出0。`lint-final.log` |
| `npm run build` | TypeScript及Vite生产构建通过，退出0。`build.log`；之后仅修改测试与本文档，产品代码未再变更 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked video_process::tests::cancelling_video_worker_terminates_its_descendants -- --ignored --exact` | 先加载 `.tools/env.ps1` 并指定 `SHUYIN_TEST_NODE`，真实Windows子进程取消1通过、0忽略，退出0。`cancellation.log`；未重跑全量Rust测试 |
| `git diff --check` | 通过 |

独立Spec样本数据在同一证据目录的 `spec-timing-hDGfMe/result.json` 与 `spec-peaks-xaC57N/result.json`。本次需求验收1—5在自动化覆盖范围内符合预期；已有低音量、尖峰、静音、全片/试片一致性、原声/视频流保持和真实进度回归继续通过，新增回归补齐音轨就绪后合成失败的清理与Windows权限错误。

剩余边界：生产构建仍有原有大于500kB分块提示及本机npm配置提示。本次未重新处理上方933秒真实口播，也未重打安装包、操作正式桌面归档或验证具体反馈样片、平台转码和主观听感。数据库仍为schema v25，无迁移、新依赖或版本变更；未提交或推送。下一步用打包后的桌面版本重新导出反馈样片，核对播放、归档及实际听感。

## 修改文件

| 文件 | 改动 |
|---|---|
| `src-tauri/video/render.mjs` | 唯一合成出口的两遍响度处理、编码后峰值校验、有限修正及已有输出拒绝 |
| `src/modules/video/VideoJobProgress.vue` | 展示整段音量分析、调整、核对的实际进度 |
| `tests/video-audio.test.mjs` | 真实 FFmpeg 音量、峰值、试片、原文件/画面保持、静音、合成失败清理与Windows写入权限错误回归 |
| `tests/video-progress.test.ts` | 完整视频及试片的新音频阶段进度回归 |
| `package.json` | 将真实音频回归接入 `test:video`，没有依赖变化 |
| `docs/PRD.md`、`docs/TECHNICAL-DESIGN.md`、`docs/DATABASE.md`、`docs/AI-RULES.md`、`docs/DEVELOPMENT-PLAN.md` | 同步当前默认响度规则、技术边界和验收入口 |
| `docs/VIDEO-LOUDNESS.md` | 需求、取舍、修改清单及实际验收证据 |
