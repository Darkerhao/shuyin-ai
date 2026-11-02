# 视频渲染进度与等待反馈验收（2026-11-02）

## 实现与验收标准

用户截图停在「正在把字幕放入画面」，只有本次用时和不定进度。当前源码确认该阶段只在调用引擎前上报一次，内部FFmpeg没有进度输出；已有帧百分比不能用于字幕合成。

本次接通真实字幕准备数量、合成/原声合入的视频秒数与倍速，增加刻度进度环、墨绿流光轨道、四步渲染状态和最近反馈时间。阶段和单位一致时才显示百分比；超过15秒无新反馈提示等待、隐藏旧速度；同步失败与减少动态效果关闭装饰动画。阶段100%仍不代表归档成功，确认、失败、停止、重试和原有已保存内容的流程继续保留。

复用Frametell 0.7.3绘图与字幕图片；原`--skip-video`被同参数的单一FFmpeg字幕合成调用替换，以读取公开`-progress`输出。独立引擎未改动，无新增依赖、IPC或权限。SQLite仍为v15，没有表、列、迁移或业务数据变更。

## 实际执行的命令

Node相关检查在本次进程PATH前置`D:/MyWork/software/shuyin-video-node/v24.22.2`，未替换系统Node。Rust命令先执行`. ./.tools/env.ps1`；相关Rust测试TEMP/TMP/SQLITE_TMPDIR指向已忽略的`output/playwright/video-render-feedback/rust-temp`。证据脚本、日志和图片位于`output/playwright/video-render-feedback/`。

| 命令 | 实际结果 |
| --- | --- |
| `npx vitest run` | 全量前端293通过；后续进度派生逻辑简化后执行相关两文件，26通过 |
| `npx vitest run tests/video-progress.test.ts tests/video.test.ts` | 26通过，覆盖真实位置/速度/更新时间、字幕准备、单位隔离、阶段切换、确认/失败/停止/重试/完成、同步失败和定时器释放 |
| `npm run test:video` | 33通过，含FFmpeg完整进度块、无效值、真实字幕图片、本轮隔离及Windows快照并发回归 |
| `npm run lint` | 通过 |
| `npm run typecheck` | 通过 |
| `npm run build` | 通过，包含TypeScript检查；保留现有大于500kB分块提醒 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked video_commands::tests` | 4通过，包含单位、倍速、时间戳经既有任务视图序列化 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked video` | 26通过，1项进程树专项按原设置忽略；包含视频事务、恢复、试片、备份和迁移回归 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked cancelling_video_worker_terminates_its_descendants -- --ignored` | 配置`SHUYIN_TEST_NODE=D:/MyWork/software/shuyin-video-node/v24.22.2/node.exe`后专项1通过 |
| `npm run check:rust` | fmt与Clippy所有目标通过 |
| `node output/playwright/video-render-feedback/smoke.mjs` | 20秒真实原声、3镜、1080p完整渲染通过，源音频SHA-256不变 |
| `node output/playwright/video-render-feedback/smoke.mjs --preview` | 4.4—12.4秒原声时窗生成8秒试片、对应镜号及进度通过，源音频不变 |
| `npx --yes --package @playwright/cli playwright-cli -s=video-render-feedback run-code --filename output/playwright/video-render-feedback/browser-check.js` | 浏览器加载实际Vue组件，1440/1120/800/390/360px无页面、面板或轨道横向溢出；阶段切换/同步失败/过期数据/完成态通过；页面运行错误0 |
| `D:/MyWork/software/ffmpeg/current/bin/ffmpeg.exe -hide_banner -v error -xerror -nostdin -i output/playwright/video-render-feedback/render-kZts1Y/video.mp4 -map 0:v:0 -map 0:a:0 -f null -` | 完整音视频解码退出0，错误输出为空 |
| `git diff --check` | 通过 |

完整渲染采集到字幕合成的真实位置`0 → 6.583333 → 15.5 → 19.875`秒，对应编码速度5.76×、7.23×、8.19×；原声合入采集到19.992381秒和23.8×。试片独立采集到7.875/8秒及8.21×。这些速度仅为该机器和短片当次测量，不作为长片性能承诺。证据在`smoke-result.json`、`preview-smoke-result.json`及各运行目录的`snapshots.json`。

浏览器在正常模式检测到流光动画；`prefers-reduced-motion: reduce`时动画为none，直线与环形过渡均为0s。截图为`render-panel.png`和`render-{宽度}.png`；浏览器数据使用隔离展示夹具，真实编码数据来自独立引擎实跑，未将夹具作为用户正式任务结果。

## 过程问题及修复依据

- 初次脚本检查沿用系统Node24.11.0时，4个入口夹具因既有Windows运行时拦截而未创建工程；使用已安装24.22.2重跑全部通过，没有修改检查规则。
- 试片首次实跑在高频读取`progress.json`时复现Windows原子替换EPERM；密集写入且无额外读取的对照也可触发，具体占用进程未定位。使用同一写入入口的并发读写回归先失败，再验证修复通过。仅Windows快照替换的EPERM/EBUSY保留上次完整快照并等下一次常规上报，其他写入错误仍抛出；没有循环重试、改变视频结果或另设任务状态。
- 浏览器检查脚本最初按未经过Vue scoped重命名的动画名断言，调整为验证实际`render-scan-*`动画；真实减少动态效果规则没有降低要求。新增Node测试显式导入timers接口后通过lint。

## 修改文件与交付边界

- `src-tauri/video/render.mjs`：进度解析、字幕准备统计、编码调用与快照更新。
- `src-tauri/src/video.rs`、`src-tauri/src/video_commands.rs`：扩展既有运行快照和序列化回归。
- `src/types/video.ts`、`src/modules/video/VideoJobProgress.vue`：同步类型、阶段展示、真实量化、速度/反馈时间与动效。
- `tests/video-progress.test.ts`、`tests/video-render.test.mjs`：行为和实际并发回归。
- `docs/PRD.md`、`docs/TECHNICAL-DESIGN.md`、`docs/DEVELOPMENT-PLAN.md`、`docs/USER-GUIDE.md`及本文：同步需求、实现与验收。

没有提交、发布、重打安装包或替换当前安装，没有中断用户当前视频任务。未进行新宿主的人工桌面验收、整本长片重跑、macOS/Linux或真实模型质量验收；短片和浏览器证据不代替这些检查。使用包含本次修改的应用构建后，新一轮渲染才能输出完整编码反馈，已启动的旧进程无法补发。下一步在新版应用中制作真实完整作品，观察各阶段数字及归档结果。发布影响按兼容用户功能增强feat/minor判断，版本仍由原流水线处理。
