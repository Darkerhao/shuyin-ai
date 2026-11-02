# P0 导演优化与同源试片：实施与验收

日期：2026-11-02。基线：`046f054` / v0.31.0。用户已确认保留发布更新并实施P0；保留上一轮5份未提交方案文档，不提交、不发布、不替换正式安装或正式用户数据。

## 1. 实现内容

- 仍为85物体、13构图、6动作；唯一目录和85项`sketch`几何不变，归一化SHA-256仍为`96a6bc4e09c4440682d68e97f417b1357cf38c7a0a327482084d8f590b36fe9b`。
- 单个主体居中，双/三物体叙事突出首个主体，compare保持等权。物体从所选字幕开始绘制，移动/倾斜/勾叉同时按该cue触发；去掉首物体预绘、固定动作延迟、自动按kind加动作及无意义循环强调。
- 分镜入口统一为`video-storyboard-v3`，继续有界请求与上下文；24条上限内优先在接近批尾的完整句意处切分。不额外调用“导演模型”，不输出任意坐标、代码或路径。
- 确认页可调整物体、动作、锚点和构图；连续重复、拥挤、主体过晚、长停留为派生提醒，点击可定位，不自动重写分镜。
- 起始镜号+30/60秒试片，经同一渲染器产生MP4与最多12张真实分镜缩略图。保留源cue索引、场景序号、物体时刻和原镜头绘制时长，只截取显示/原声窗口；不足时使用剩余音频，不重做ASR或配音。
- 试片复用同一video_jobs/运行锁/run_id/停止进程树，结束回到等待确认，不登记为正式成片。修改立即隐藏，保存、重跑、源码签名不符或文件缺失后失效；完整视频仍须明确确认。

P0当时没有实施场景/扩库/景别或图片，本文数据保留为历史证据。用户随后确认P1，实际采用6类场景/三景别/8项物体，见 [P1验收](VIDEO-SCENES-VERIFICATION.md)；P2仍未实施，24候选未全量采用。

## 2. 文件与数据边界

| 范围 | 主要文件 | 说明 |
| --- | --- | --- |
| 契约/调度 | [video.rs](../src-tauri/src/video.rs)、[video_commands.rs](../src-tauri/src/video_commands.rs)、[任务数据](../src-tauri/src/database/video.rs) | 单一VideoWork，预览阶段不能进入正式finish事务 |
| 绘图/编译 | [illustrations.js](../src-tauri/video/illustrations.js)、[render.mjs](../src-tauri/video/render.mjs) | 同一可信draw；时窗不改源计划；真实第0帧避免引擎默认“未来封面” |
| Prompt | [当前video-storyboard-v4.md](../prompts/video-storyboard-v4.md) | 本轮P0曾使用v3替换v2；后续P1替换为v4，历史版本可经Git查看，不并留生成入口 |
| 页面 | [VideoPage.vue](../src/modules/video/VideoPage.vue)、[进度](../src/modules/video/VideoJobProgress.vue)、[提醒](../src/modules/video/storyboard.ts) | 复用同一编辑状态/现有墨绿浅色控件，无新增字体或UI依赖 |
| IPC/媒体 | [服务](../src/services/video.ts)、[类型](../src/types/video.ts)、[权限](../src-tauri/permissions/video.toml)、[配置](../src-tauri/tauri.conf.json) | 本书/当前代次/固定文件与有限镜号授权，img-src支持受限asset缩略图 |
| 回归 | [Node](../tests/video-render.test.mjs)、[页面](../tests/video.test.ts)、[进度](../tests/video-progress.test.ts)、[提醒](../tests/video-storyboard.test.ts) | 时窗、动作、主次、失效、确认与错误保留 |

无SQL迁移、新表/列、新依赖或版本号修改，schema仍为v14。新增两个命令只挂在主窗口原allow-video权限，asset默认scope仍为空，不开放任意文件路径或远端页面权限。预览清单/MP4/PNG属于派生缓存，不进入assets/备份；SQLite仍是唯一业务状态，缺缓存重新试片即可。

原始音频只读，合入MP4使用AAC，不承诺音频字节相同。旧视频、初始字幕、原分镜源文件、旧发布选择不自动更改；当前代码不会更新正在使用的旧安装程序。

## 3. 真实桌面与样片证据

隔离identifier：`com.shuyin.director-verify`。运行产物与临时目录在D盘仓库`output/playwright/video-director/`；隔离SQLite/WebView2 profile仍按系统约定位于C盘AppData，没有移动或删除原文件。应用以隐藏窗口启动，仅连接本次18482调试端口，不读取正式窗口或浏览器会话。

输入复用既有《我的前半生》83.666秒原声及39条SRT。语音程序/模型路径为空，实际没有重新转写；初次分镜使用本地契约HTTP夹具检查v3边界，然后明确保存原参考人工分镜作为画面对照。该夹具不代表真实模型的创作质量。

| 实际步骤 | 结果与证据 |
| --- | --- |
| 指定第2镜，生成30秒试片 | 原声10.6—40.6秒，1920×1080/24fps/H.264/AAC；2张真实缩略图，可定位/播放。preview-2完成后仍awaiting_confirmation，原plan与assets不变 |
| 非法起镜、并发重复、跨书媒体请求 | 全部拒绝，没有偷开新运行或登记成片 |
| 临时移开本次缩略图后读取 | preview为空；恢复本次文件后可继续，未修改原输入 |
| 编辑标题、保存校对 | 页面立即隐藏旧预览；后端旧代次媒体请求拒绝。保存成功而下一次启动失败的场景也有先失败后通过的页面回归 |
| 取消与重试 | 取消run3保留plan；同一任务run4重试60秒尾段，实际可播放约25.266秒，不补长、不加速 |
| 重启恢复 | 正常关闭/重启后，run4预览、源plan签名及媒体可回读；未触发新模型或ASR请求 |
| 完整确认 | 明确点击完整生成，run5完成83.666667秒MP4/SRT；只新增2份正式素材，旧试片SHA不变，重启/渲染模型请求0 |
| 原生视口 | 1440/1120/800/390均无页面或预览区横向溢出。390为WebView视口模拟，不改变800px原生最小窗口，也不是移动端验收 |

`native-result.json`、`native-context.json`、`native-restart-result.json`记录任务与媒体结果；`native-preview-{宽度}.png`、`native-full-player.png`记录真实界面，已查看。原生初次脚本在预览已成功后遇到测试脚本对Windows扩展路径的`EISDIR`，只修正脚本并复用原任务续验，没有重建书籍或重新转写；`native-failure.json`仍保留。

最终UI续验使用已完成作品的「用原字幕制作新版手绘」创建独立待确认任务，复用39条字幕、语音路径仍为空；2次本地契约分镜请求，不是付费AI。真实下拉控件可编辑物体/动作/锚点，保存回读一致；修改镜头标题后试片起点即时更新。该问题的根因为当前Reka SelectItemText仅在挂载时登记文字，按已核对的SelectValue插槽直接显示现有scenes，不重挂载组件或新增缓存。四档编辑区无溢出，原完成任务完整JSON摘要不变。证据`native-ui-result.json`与`native-editor-{宽度}.png`。

隔离窗口已正常关闭，18482/18483均无监听。只读SQLite最终检查schema14、integrity=ok、外键错误0、running=0；一份完整作品和一份独立待确认校对任务保留。源文件/内嵌脚本/文档的最终交叉核对记录在`verify-evidence.json`，没有清理原证据或操作正式数据库。

### 同源与新旧画面对照

- `preview-parity.mjs`在同任务完整工程与30秒预览工程中，对原声18/35/40.2秒分别截图；物体区`x=140,y=280,w=1640,h=600`的PNG逐字节相同，浏览器错误0。证据`preview-parity.json`，证明采样点的时钟/绘制一致，不声称整个MP4逐帧字节相同。
- 可观看的新试片：`output/playwright/video-director/sample-final/video.mp4`。以原参考书名、同一分镜及原声10.6—40.6秒生成，精确30秒。
- 旧版对照：`output/playwright/video-director/comparison-old.mp4`，从既有旧片先dry-run再准确裁切同段。受编码/音频包边界影响容器为30.023秒，并非修改旁白内容。
- `final-comparison.png`为相同片段22秒的旧/新画面对照，`final-contact.png`和`full-contact.png`为新试片/完整83秒画面抽样，已查看。主体大小、标签和字幕可读，抽样未见明显裁切或遮挡；字幕/原书事实仍需完整人工听审。
- 自定义16:9技术检查（不执行响度归一化）覆盖时长、比例、恒定帧率、音轨、黑帧/静帧/静音分析：30秒试片和83秒完整片均8通过、1软字幕轨提醒。字幕已经烧录，整片另存SRT；不能据此宣称某平台发布或响度标准通过。

技术通过不等于“用户一定更喜欢”。P0保留了现有风格，用户应先比较同原声试片，再决定P1画面层次的方向。

## 4. 实际验证命令

Rust命令先执行`. ./.tools/env.ps1`；本轮进程TEMP/TMP/SQLITE_TMPDIR指向D盘隔离临时目录。下列为本次实际执行，不引用历史测试数字代替当前结果。

| 命令 | 实际结果 |
| --- | --- |
| `npx vitest run --maxWorkers 2` | 18文件、283项通过 |
| `npx vitest run tests/video.test.ts tests/video-storyboard.test.ts tests/video-progress.test.ts` | 视频相关24项通过 |
| `npm run test:video` | Node16项通过，包括85物体×6动作、参考几何摘要、语义起点/主次、非法时窗、预览保持源时钟与同一入口 |
| `npm run test:release` | 78通过，1项macOS专用跳过；使用临时Git夹具，不是实际发布 |
| `npm run lint` | 通过；没有关闭规则或豁免 |
| `npm run typecheck` | 通过；隔离桌面构建还再次执行了完整前端build |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked` | 155通过、2项默认忽略 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked cancelling_video_worker_terminates_its_descendants -- --ignored` | 设置SHUYIN_TEST_NODE后专项1通过；系统凭据测试未执行 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | 通过 |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets -- -D warnings` | 通过 |
| `npx tauri build --debug --no-bundle --config output/playwright/video-director/tauri.verify.conf.json` | v0.31.0隔离桌面构建通过，含TypeScript/Vite；未生成或安装正式安装包 |
| `node output/playwright/video-director/native-check.mjs`，故障后`resume` | 初次已完成准备与30秒预览；续验复用原任务完成全部预览交互/取消/重试，见原始失败和最终结果 |
| `node output/playwright/video-director/native-check.mjs restart` | 重启读回、完整确认/归档/真实定位播放通过，模型请求0 |
| `node output/playwright/video-director/native-ui-check.mjs` | 原字幕制作独立新版、受控编辑/即时标题及保存回读、四档编辑区通过，旧完成任务不变 |
| `node output/playwright/video-director/preview-parity.mjs` | 3处物体区域一致、错误0 |
| `node src-tauri/video/render.mjs ...sample-final/input.json ...sample-final/result.json` | 最终同原声30秒试片真实渲染成功，严格布局/确定性检查保留 |
| ffmpeg-skill的`probe.py`、`look.py`、`check.py --platform custom --aspect 16:9 --no-loudness --content` | 媒体参数、抽帧和技术结果见上述证据；完整片全量音视频解码用于内容检测 |
| `git diff --check`、`git diff --cached --check` | 通过，索引不改变 |

过程中的编译/断言失败保留在输出日志：预览缺接口、时窗未生效、首物体提前、三物体无主次、页面没有预览入口/进度、旧预览重新出现等均先复现再修复，没有删测试/放宽检查。一次内联媒体验证脚本正则转义错误只影响检查脚本，改为明确前缀处理后继续已有产物检查。

## 5. 未验收与后续

- 未调用真实付费分镜/图片模型，没有把本地契约输出当作AI创作效果验收。
- 当前真实整片为83秒参考输入；未把它说成整本书长片、两小时性能或完整逐字听审。
- 未验收正式安装升级、签名发行、macOS/Linux、真实平台上传或最终发布；无关queue与Python转写回归未重跑，不声称完整`npm test`全绿。
- 构建保留约560kB主JS包提示，npm用户级python/msvs_version未知配置提醒也未通过修改全局配置掩盖。
- 本机仍使用独立Frametell 0.7.3；AGPL/字体/素材分发边界未改变，未捆绑外部引擎或下载新依赖。

P0业务闭环与上述本机证据完成后，下一步先由用户观看新旧样片；只有方向确认再实施P1，不自动扩展到图片服务、云渲染或自动发布。
