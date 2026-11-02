# 视频 P2：关键图片与主动插画验收（2026-11-02）

## 后续代码交付授权（2026-11-02）

下文记录的是提交前的业务实现与隔离验收。用户随后明确授权提交并推送dev，再将同一P2提交cherry-pick到main并推送；本节取代下文当时“不提交、不发布”的操作限制。远端已自动发布v0.37.0，本轮保留该版本并由原main流水线判断下一版本，不手动升版。本轮另外执行`npm run test:queue`，19项全部通过（Node24.22.2）。Git交付结果以实际分支历史/远端读回为准，自动构建与安装包发布另以Actions结果为准。

## 1. 本轮交付

基于dev/05f14c6、v0.36.0和干净工作区，实现原方案P2：本书静态图片导入/选择、来源与授权、裁切、内容摘要冻结、同源试片/整片，以及主动单张视频插画。沿用同一Scene、视频任务、ImageProvider、取消、备份及正式成片事务；没有新供应商、新依赖、新渲染器或第二套调度状态。

图片与手绘主体二选一，默认完整显示，可改裁切铺满并调整0—1水平/垂直位置。来源、授权/自有说明必填；创作者未知留空。画面来源截短显示，完整信息留在plan；图片从本镜第一条字幕开始出现，标题/来源/字幕固定。小图仅预览裁切，最终排版看原声试片。

插画先确认模型、1张及可能费用，只发送书名、已提供作者和描述，不上传图片/音频；结果先入本书素材，显式采用才改分镜，仍需填写使用权。失败、取消和不采用均保留原稿，正常重启后可从图片列表找回已生成素材。

## 2. 修改文件

| 范围 | 文件 |
| --- | --- |
| 校对与图片编辑 | [VideoPage.vue](../src/modules/video/VideoPage.vue)、新增[VideoImageEditor.vue](../src/modules/video/VideoImageEditor.vue)、[类型](../src/types/video.ts)、[服务](../src/services/video.ts) |
| 约束与原生任务 | [video.rs](../src-tauri/src/video.rs)、[video_commands.rs](../src-tauri/src/video_commands.rs)、[任务数据](../src-tauri/src/database/video.rs)、[素材保护](../src-tauri/src/database/assets.rs) |
| 图片生成复用 | [image_generation.rs](../src-tauri/src/database/image_generation.rs)、[cover_commands.rs](../src-tauri/src/cover_commands.rs)、[静态图片校验](../src-tauri/src/ai/images.rs)、新增[视频插画Prompt](../prompts/video-illustration-v1.md) |
| 同源绘图 | [render.mjs](../src-tauri/video/render.mjs)、[illustrations.js](../src-tauri/video/illustrations.js)、[说明](../src-tauri/video/README.md) |
| 权限与配置 | [build.rs](../src-tauri/build.rs)、[lib.rs](../src-tauri/src/lib.rs)、[video.toml](../src-tauri/permissions/video.toml)、[tauri.conf.json](../src-tauri/tauri.conf.json) |
| 回归 | [视频页面](../tests/video.test.ts)、新增[图片编辑](../tests/video-image.test.ts)、[Node](../tests/video-render.test.mjs)、任务数据内联测试、[图片任务测试](../src-tauri/src/database/m3_tests.rs) |
| 文档与记录 | PRD、技术设计、数据库、AI规则、开发计划、使用手册、画面方案、本文，以及原有task_plan/findings/progress三份记录 |

## 3. 数据、兼容与权限

SQLite仍为v15，没有SQL迁移或表/列变更。Scene.image可选，包含assetId/source/rights/creator?/fit/x/y/sha256，只有图片镜头的beats为空。旧分镜读取不重写，旧文字卡仍明确升级，旧成片仍创建新版本。旧程序严格JSON反序列化不保证读取图片字段，回退要保留升级前备份。

保存时只解析本书登记图片，完整解码后冻结摘要，全部成功才更新plan_json。运行时将已校验字节写入本轮目录，Node再次核对摘要，再以摘要与真实扩展名构建图片层。缺失/源变化使旧试片失效，归档前再次校验；运行中的图片不能移除或重新定位。完整备份沿用assets收集和ID引用，不新增路径重写逻辑；重启、恢复与原文件缺失后的恢复副本均有回归。

ai_tasks.kind自v4起无枚举约束，本轮使用video_illustration及video-illustration-v1，图片存于本书video/illustrations，不写cover_versions、不自动改video_jobs。封面和视频插画共用内部Provider/生成/校验/版本化落盘，保留封面外部命令。取消与中断沿用现有AI任务状态。

新增read_video_image/import_video_image/start_video_illustration/execute_video_illustration四个受限主窗口命令。图片预览经校验后的二进制IPC和blob URL，CSP仅img-src追加blob；没有为远程窗口增加权限，也不开放任意文件或网络图片渲染。

## 4. 实际命令与结果

工作目录为仓库根。视频Node使用`D:/MyWork/software/shuyin-video-node/v24.22.2/node.exe`，媒体工具使用`D:/MyWork/software/ffmpeg/current/bin`；仅修改相应命令进程PATH，不替换全局程序。Rust先加载`. ./.tools/env.ps1`，测试TEMP/TMP/SQLITE_TMPDIR指向忽略目录`output/video-p2/rust-temp`。

| 实际命令 | 结果 |
| --- | --- |
| `npx vitest run` | 20文件/304项通过 |
| `npx vitest run tests/video.test.ts tests/video-image.test.ts` | 最终UI调整后相关19项通过，包含确认取消零请求、独立候选、主动采用、失败/取消保稿、导入及图片镜头保存 |
| `npm run test:video` | 最终38项通过；包括原93物体几何/动作、图片来源/裁切/摘要、未知字段、缺失输入、解码就绪、HUD来源与无额外镜头运动 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked video` | 30通过、1项原有进程树专项默认忽略 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked` | 166通过、2项原有专项默认忽略（系统凭据、视频进程树） |
| `npm run lint` | 最终通过，没有关闭规则 |
| `npm run typecheck` | 通过；后续隔离构建再次包含类型检查 |
| `npm run check:rust` | 最终fmt及Clippy所有目标通过 |
| `npm run test:release` | 78通过、1项macOS专用跳过；测试在隔离仓库运行，不是真实发布 |
| `npx tauri build --debug --no-bundle --config output/playwright/video-p2/tauri.verify.conf.json` | 最终隔离Windows应用构建通过，包含TypeScript/Vite；未制作安装包 |
| `node output/playwright/video-p2/smoke.mjs` | 最终20秒原声、contain/cover图片与手绘末镜完整渲染通过，源音频/图片SHA不变 |
| `node output/playwright/video-p2/smoke.mjs --preview` | 原声4.4—12.4秒的8秒试片通过，保留镜号/时钟与原件 |
| `node output/playwright/video-p2/verify-parity.mjs` | 图片内容在原声5.4/8.4/11.4秒逐字节相同，前后定位一致；最终连续5轮通过，严格布局0问题 |
| `node output/playwright/video-p2/verify-native.mjs` | 真实WebView2/IPC导入、裁切保存、CSP图片显示、单张Provider请求、取消确认、候选不自动采用、20秒尾段试片播放及失效通过 |
| `node output/playwright/video-p2/verify-native-restart.mjs` | 真实重启保留计划，五档内部溢出0；明确确认后完整20秒归档且仅新增MP4/SRT各1份，播放通过，计划摘要不变 |
| `python C:/Users/haozi/.agents/skills/ffmpeg-skill/scripts/probe.py <完整MP4> --json` | 最终20秒、1920×1080、24fps、H.264/yuv420p、AAC双声道，原声未归一化 |
| `python C:/Users/haozi/.agents/skills/ffmpeg-skill/scripts/check.py <完整MP4> --content --no-loudness --json` | 完整音视频解码与内容检查0失败、1提醒（无独立软字幕轨，字幕已烧录）；未指定发布平台，不据此声明平台合规 |
| `python C:/Users/haozi/.agents/skills/ffmpeg-skill/scripts/look.py <完整MP4> --tiles 3x2 -o output/playwright/video-p2/final-contact.png --json` | 六格抽帧已查看，图片、来源、字幕和手绘末镜可见；联系表时码是seek后的相对PTS |
| 只读Python SQLite/运行源码校验 | schema15、integrity=ok、外键0、running=0；唯一验收视频succeeded/complete/run3；独立插画用途成功；3份运行源码与当前源码SHA一致 |
| `git diff --check`、`git diff --cached --check` | 通过；没有暂存、提交或发布 |

最终核对12份Markdown、87个本地链接、原音频/图片摘要、15份迁移、0.36.0版本和空索引通过，记录在final-evidence.json。

完整MP4路径、具体命令参数和元数据保存在render-result.json/native-restart-result.json/full-probe.json/full-check.json/full-look.json。脚本和证据均在忽略的`output/playwright/video-p2/`，不加入应用资源或Git索引。

## 5. 原生与画面证据

隔离identifier为com.shuyin.p2-verify，数据库位于`C:/Users/haozi/AppData/Local/com.shuyin.p2-verify/`，按Tauri数据目录规则产生少量C盘数据。WebView缓存、验收工程和媒体在D盘output目录；没有安装软件、替换正式应用或迁移用户资料。

- 第一次运行使用已有20秒音频和9条SRT，没有重做ASR；本机HTTP分镜服务1次，图片服务1次，图片请求n=1/1536x1024/b64_json，不上传音频/图片。服务返回已有自有验收画面，仅验证协议与闭环，不证明外部模型画质。
- 真实选择本书图片、25%/75%裁切位置、填写来源/授权后保存。图片经二进制IPC在实际CSP下显示，未知创作者没有代填。取消首次生成确认时请求数0；再次明确确认后产生一张候选，数据库原计划未变，选择暂不采用后仍保留素材。
- 30秒试片因尾段仅20秒输出20秒，可定位/播放；无正式videoAssetId，也没有新增视频素材。修改即隐藏旧试片，保存后preview=null。
- 正常关闭并用最新构建重启，计划摘要一致。1440/1120/800/390/360px分别核对图片编辑区/内容区/裁切区内部滚动宽度，没有横向溢出。360px只是窄桌面窗口验收，不是手机端支持声明。
- 最终通过真实页面确认完整视频，只有MP4/SRT各新增1份，既有图片、原声和计划保留；成片20秒/1080p可播放。正常关闭后数据库无running任务，源码、目录和绘图摘要与本轮构建相同。

可查看`native-final-360.png`、`native-complete.png`、`image-frame.png`、`final-contact.png`；结果在native-result.json/native-restart-result.json/parity-result.json/database-evidence.json。

## 6. 过程定位与修正

- 图片脚本新增前，正向回归明确失败于Unknown scene field；接通后通过。初始测试错填字幕覆盖、旧就绪夹具未提供SCRIPT、模板回调可空类型、任务视图未暴露Prompt版本，均按实际输入/API修正，不删除断言或放宽类型。
- OperationFeedback通过toast展示错误，前端测试核对其真实error属性，不假定页面内联文字。首次lint的globalThis/模板排版问题已按现有规则修正。
- 首次360px原生实测虽然页面没有溢出，但图片组件内部出现内容宽度179px/滚动212px。根因是子组件未继承父组件scoped字段样式且网格采用最小内容宽度；本组件补minmax(0,1fr)、控件宽度和必要换行后，五档内部溢出均为0。
- 首次严格布局提示新来源文字22px过小，调整为26px并留出图片/来源间距；最终严格布局无警告或错误，未修改检查阈值。
- 早期截图对照包含图片框外纸张与圆角背景，曾有3104个像素出现RGB单级差异。对照两份完全相同的整片页面也复现同样差异，图片内容区域不同像素为0；对应原有radial-gradient vignette的浏览器栅格舍入。对照范围明确为图片框内内容，继续逐字节比较、固定几何和前后定位检查，连续5轮通过；没有放宽生产布局/确定性检查或给渲染器加重试。
- 原生验收初次连接早于WebView/CDP就绪，另一次脚本在about:blank导航时执行；验收脚本增加有界端点和实际页面就绪等待，未为产品加固定延迟。
- 一次大段测试写入命令被自动审批策略拒绝，工具未给具体规则；后续使用较小写入和临时样本重命名，未删除用户文件或申请额外权限。

## 7. 验收对应与边界

| 验收项 | 本轮证据 |
| --- | --- |
| 本书静态图片/来源/裁切可保存 | Vue/Node/Rust回归与真实IPC通过 |
| 跨书、路径、非法来源/格式/裁切拒绝并保稿 | 本书SQL解析、完整解码、未知字段和保存失败回归通过 |
| 源摘要、失效及恢复 | 文件变化/缺失拒绝、完整备份路径重映射、重启通过 |
| 图片就绪、同源与原声 | 解码失败阻止就绪；20秒整片/8秒时窗、三处图片内容一致与定位通过 |
| 主动插画不覆盖/不自动收费 | 确认取消零请求，单张本机Provider、候选/采用及失败取消回归通过；真实付费调用0 |
| 完整工作流与旧内容 | 试片不归档、编辑失效、重启、完整MP4/SRT唯一登记、原件不变通过 |
| 桌面可用与工程检查 | 五档内部无溢出；304前端/38Node/166Rust、lint、类型、fmt/Clippy及隔离构建通过 |

没有提交、发布、手工改版本、制作安装包或替换正式安装。此阶段未运行完整npm test中的队列检查；后续交付前已单独补验19项通过，见本文顶部。当前构建保留既有Vite大于500kB提醒与npm未知用户配置提醒；没有调整阈值或全局配置。

未验收真实远程图片模型质量/费用/服务参数兼容、史料或商用授权、整本长片与完整听审、正式安装升级及macOS/Linux。本机短片和契约服务不能代替这些验证。下一步使用包含本改动的桌面构建，用少量真实关键图片或明确确认的单张插画试片，审核内容与来源后再制作正式作品；正式版本交给原feat/minor流水线处理。
