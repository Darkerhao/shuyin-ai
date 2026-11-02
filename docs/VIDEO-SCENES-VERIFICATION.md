# P1 场景层次与画面内容：实施与验收

日期：2026-11-02。基线：`0e86f39` / v0.32.1，开始时工作区和索引干净。用户明确推进P1，并在中途要求继续任务。未提交、发布、手动升版、替换正式安装或操作正式数据库。

## 1. 实现与取舍

- 同一绘图目录新增6类轻场景：书房案头、庭院门墙、城市街道、河岸远山、窗边独处、路径地形；保留plain空纸。背景只画结构，不把人物、道具或叙事结论藏进背景。
- 增加wide远景、medium中景、detail特写；保留reference原布局。主体大小/摆位有区别，compare继续等权，每镜仍1—3个主体。
- 只在`kind=focus + framing=detail`且首beat后至少3秒时，执行一次1.06倍、3秒轻推近；其余组合静止。标题、系列信息、页号和进度线移至引擎HUD，字幕仍由固定字幕层负责。
- 新增`brain/compass/road_fork/mountain/tree/window/cup/bed`，总计93物体/13构图/6动作。依据是只读核对已有《我的前半生》《人生由我》及睡眠/咖啡知识样片的实际字幕；指南针、大脑、杯子是直接缺口，其余覆盖选择、休息、山地、树木、光照表达。原方案另外16项暂缓，不以凑满109项为目标。
- Rust、Node、Vue及唯一v4 Prompt共享目录；只增加Scene.environment/framing两字段，没有任意坐标、camera、图片URL或模型代码。删除v3生成入口，不保留两种生成路径。
- 继续原任务/保存/试片/确认/归档流程。场景与景别编辑复用同一scenes状态，修改即隐藏旧试片；重复提醒识别环境/景别的实际变化。路径背景复用现有route路径，不叠画第二条路线。

不新增依赖、IPC、capability、Provider、数据库表或调度器。属于feat/minor兼容功能增强，版本由后续正常发布处理，本轮仍v0.32.1。

## 2. 文件范围

| 范围 | 修改文件 |
| --- | --- |
| 唯一目录与绘图 | [visual-catalog.json](../src-tauri/video/visual-catalog.json)、[illustrations.js](../src-tauri/video/illustrations.js)、[render.mjs](../src-tauri/video/render.mjs)、[绘图说明](../src-tauri/video/README.md) |
| 契约与生成 | [video.rs](../src-tauri/src/video.rs)、[video-storyboard-v4.md](../prompts/video-storyboard-v4.md)；v3文件移除 |
| 确认页与提醒 | [VideoPage.vue](../src/modules/video/VideoPage.vue)、[storyboard.ts](../src/modules/video/storyboard.ts)、[video.ts](../src/types/video.ts) |
| 回归 | [Node](../tests/video-render.test.mjs)、[页面](../tests/video.test.ts)、[提醒](../tests/video-storyboard.test.ts)、[数据库回归](../src-tauri/src/database/video.rs)，以及video.rs内的契约测试 |
| 当前文档 | [PRD](PRD.md)、[技术设计](TECHNICAL-DESIGN.md)、[数据库](DATABASE.md)、[AI规则](AI-RULES.md)、[开发计划](DEVELOPMENT-PLAN.md)、[使用手册](USER-GUIDE.md)、[画面方案](MG-VIDEO-VISUAL-UPGRADE.md)、[接入说明](MG-VIDEO-INTEGRATION.md)、[P0历史验收](VIDEO-DIRECTOR-VERIFICATION.md)、本文 |
| 工作记录 | [task_plan.md](../task_plan.md)、[findings.md](../findings.md)、[progress.md](../progress.md) |

本机验证脚本、截图、音视频和JSON证据位于`output/playwright/video-p1/`，沿用已有忽略目录，不进入安装资源或Git索引。

## 3. 数据与兼容性

SQLite仍为v14，无SQL迁移。旧物体计划缺字段时读取为plain/reference，不执行UPDATE、不修改历史JSON；旧文字图解继续原来的明确升级入口。用户显式保存才将新字段写入plan_json。Rust拒绝显式null、目录外枚举及未知字段；Node再次校验场景白名单。

新增数据库回归覆盖旧JSON逐字保留、新字段/新物体保存与备份恢复；原音频摘要和字幕保留。现有成功任务不可编辑，失败/取消/重启与正式MP4/SRT事务仍用原流程。新程序可读旧数据，但旧程序不保证读取新JSON，回退需升级前备份，不自动删字段迁就旧版本。

原85项sketch分支和primitive未变；测试仅排除本轮追加的8个case后验证原归一化SHA-256：`96a6bc4e09c4440682d68e97f417b1357cf38c7a0a327482084d8f590b36fe9b`。并以当前HEAD的真实P0绘图和新程序plain/reference渲染对比5个时刻，物体区PNG逐字节一致。

## 4. 实际验证命令

工作目录为仓库根目录。视频进程使用独立`D:/MyWork/software/shuyin-video-node/v24.22.2/node.exe`，不替换全局Node。下表`node`执行项均实际指向该程序；媒体脚本使用已有`D:/MyWork/software/ffmpeg/current/bin`，只在本次进程PATH中加入。

| 实际命令 | 结果 |
| --- | --- |
| `npm run lint` | 通过；没有关闭规则 |
| `npm run test:video` | 26项通过：93物体×6动作有限值/确定性、旧85几何、目录/字段、背景、景别、HUD、轻推近、预览时钟及边界回归 |
| `npx vitest run` | 284项前端测试通过 |
| `npx vitest run tests/video.test.ts tests/video-storyboard.test.ts tests/video-progress.test.ts` | 视频相关25项通过 |
| `npm run test:release` | 发布/打包/页面脚本78项通过、1项macOS专用跳过；测试使用隔离仓库，不是真实发布 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked` | 157项通过，2项显式原生专项默认忽略 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked video::tests` | 契约与数据库相关15项通过，含新增旧记录/新字段/备份回归 |
| `$env:SHUYIN_TEST_NODE='D:\MyWork\software\shuyin-video-node\v24.22.2\node.exe'; cargo test --manifest-path src-tauri/Cargo.toml --locked cancelling_video_worker_terminates_its_descendants -- --ignored` | 原有取消进程树专项1项通过；凭据专项未运行 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | 通过 |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets -- -D warnings` | 通过 |
| `npx tauri build --debug --no-bundle --config output/playwright/video-p1/tauri.verify.conf.json` | 隔离Windows应用构建通过，包含`npm run build`和TypeScript检查；未制作安装包 |
| `node output/playwright/video-p1/verify-render.mjs` | 7环境×4景别共28镜严格布局/确定性检查0问题；5处P0参考布局对照、3处P1派生试片对照逐字节一致 |
| `node output/playwright/video-p1/verify-matrix.mjs` | 2493组合×4时刻：全部93物体/6动作/三景别，及7环境/13构图/1—3主体边界；0越界、0物体/标签碰撞、0引擎错误 |
| `node output/playwright/video-p1/verify-native.mjs` | 新字段/新物体真实UI编辑保存、4档窗口、30秒预览、原稿/素材不变、失效、非法字段拒绝、取消通过 |
| `node output/playwright/video-p1/verify-native.mjs restart` | 正常重启保留计划；显式确认后完整83.667秒成片归档，MP4/SRT仅新增各1份，实际播放通过；此阶段模型请求0 |
| `node output/playwright/video-p1/verify-native-parity.mjs` | 原声43.9/45.9/57.9/65.3/72.8秒，实际原生试片与整片物体区逐字节一致；运动中HUD文字位置/尺寸/transform不变；两次运行的3份脚本/目录摘要均与源码相同 |
| `python <ffmpeg-skill>/scripts/probe.py <preview/full MP4> --json` | 30秒及83.666667秒，1920×1080、24fps、H.264/yuv420p、AAC双声道44.1kHz；完整片6,856,610字节 |
| `python <ffmpeg-skill>/scripts/check.py <preview/full MP4> --content --no-loudness --json` | 两份分别12项PASS、1项WARN、0FAIL；全解码无黑片/全冻结/静音错误。WARN为无独立软字幕轨，实际字幕已烧录 |
| `python <ffmpeg-skill>/scripts/look.py <preview MP4> --tiles 3x2 -o output/playwright/video-p1/preview-contact.png --json` | 6格抽帧已查看，字幕、主体和场景可见 |
| `python <ffmpeg-skill>/scripts/look.py <full MP4> --tiles 4x3 -o output/playwright/video-p1/full-contact.png --json` | 12格抽帧已查看；联系表时码为抽取后的相对PTS，不用作原声绝对定位依据 |
| `git diff --check`、`git diff --cached --check` | 通过，无暂存或提交 |
| `node output/playwright/video-p1/verify-evidence.mjs` | 15份Markdown/106个本地链接、4份源/派生文件摘要、6份运行源码摘要、媒体/矩阵/数据库证据及版本/空索引核对通过 |

`<ffmpeg-skill>`实际为`C:/Users/haozi/.agents/skills/ffmpeg-skill`；具体输入路径、完整媒体命令、probe及检查结果在`preview-*.json`/`full-*.json`。未指定发布平台，没有调整原声响度，不能将默认媒体检查写为平台响度或发布验收。

## 5. 原生闭环与证据

隔离identifier为`com.shuyin.p1-verify`。应用数据依Tauri规则存于`C:/Users/haozi/AppData/Local/com.shuyin.p1-verify/`，测试工程/缓存/成片在仓库D盘output目录；没有移动或清理用户原文件。

1. 复用原有《我的前半生》83.666秒音频及39条SRT，不重新ASR。初次生成用本机HTTP契约夹具2次，确认v4目录/字段经过真实Provider和Rust校验；这不是外部模型质量证明。
2. 真实选择场景、特写和指南针，保存后IPC回读正确。随后还原人工审核样片计划，测试修改不混入对照成片。1440/1120/800/390px均无页面或场景控件溢出；390px只是窄窗口布局，不是手机端验收。
3. 从第4镜原声42.9秒生成30秒试片，物体/镜头时钟不改。无正式成片ID，素材数量不变，播放成功。场景修改立即隐藏试片，保存后旧媒体请求拒绝；非法场景保存失败不改计划。
4. 取消下一次试片，正常关闭并重启。计划摘要相同，不重新调用分镜模型；明确确认后run4生成完整视频，仅正式MP4/SRT各追加1份，旧试片摘要不变。
5. 正常关闭隔离应用后只读核对：schema14、integrity=ok、foreign_key_check=0、唯一任务succeeded/complete/run4，无running任务。18492/18493端口关闭，正式安装进程未被替换或停止。

主要证据：`native-result.json`、`native-restart-result.json`、`native-parity.json`、`database-evidence.json`、`render-evidence.json`、`matrix-evidence.json`。

可直接查看：

- `output/playwright/video-p1/preview.mp4`：30秒真实原声试片，非发布素材。
- `output/playwright/video-p1/environments-contact.png`：6种场景的真实引擎绘制联系表。
- `output/playwright/video-p1/path-detail.png`：特写/主次样例。
- `output/playwright/video-p1/native-preview.png`、`native-editor-390.png`：原生试片及窄窗口编辑截图。
- 完整成片路径记录在`native-restart-result.json`的`videoPath`，不改名或移动已归档文件。

## 6. 发现的问题与修正

- 新功能先测试：Node新增5项、Rust字段/备份2项、Vue编辑/提醒2项先失败，再实施。后续特写安全区专项也先复现失败。
- 2493组合首轮发现166条重复时刻/环境边界记录：1.34倍特写叠加倾斜/轻推近使宫门顶部达到y249，钢笔等可能碰标签；次物体标签固定148px容不下摇篮。直接把新特写比例收敛到1.2并按物体尺寸留白，原几何/原布局/严格阈值不改。复验全部0问题，实测主体范围x276.47—1626.73、y277.54—765.52。
- lint发现测试正则连续空格，改为等价量词；TypeScript发现泛型Select被Vue Test Utils判为DOMWrapper，依据当前组件名UiSelect和库声明使用名称选择器，不加any或关闭检查。
- 原生截图辅助脚本最初用`duration === 30`识别缓存，实数减法的尾差使筛选失败；改按已知源镜号识别，不改业务时间轴。HUD整块像素包含引擎随镜头变化的衬底，不能用它证明标题位移；改比较文字实际边界与transform。首次即时截图有一处不一致、加入像素诊断后未再次出现；最终明确等待绘制帧后5处逐字节一致。没有据此修改业务绘图、放宽引擎校验或加入渲染重试。
- 媒体probe首次因进程PATH没有ffprobe失败；使用已有D盘工具并运行doctor确认后通过，未安装依赖或修改全局PATH。探索中的不存在路径/Windows rg通配符问题均通过实际枚举解决，不新增兼容文件。

## 7. 验收结论与剩余边界

本轮P1实现和上述本机技术/工作流验收完成：场景、景别、8物体、唯一目录、严格校验、旧记录/备份、同源试片/整片、编辑失效、取消/重启和事务归档均有实际证据。背景与线条保持原手绘风格；是否更贴题、更耐看仍需用户实际观看。

未运行外部付费文本/图片模型、三类真实内容完整听审与评分、P1整本书长片、正式安装升级/签名/发行、macOS/Linux实机或真实平台发布。P2图片/插画及剩余候选物体不属于本轮。无关队列和Python转写未重跑，不声称完整`npm test`全绿；npm用户级python/msvs_version和Vite约562kB主包提示保留，不改全局配置或构建阈值。

下一步先观看30秒试片与6场景联系表，再用实际选中的文本模型测试一段自己的字幕；方向满意后补长片听审。正式使用需包含P1的新构建，当前正式安装不会自动获得源码改动。


## 8. 未提交代码审查后的辅助线修复（2026-11-02）

审查用真实Chromium复现：detail三主体布局把末两主体移至(1132,433)/0.76与(1476,604)/0.64，route/transfer仍用固定±180端点，末箭头向左下；wall仍在x=1212，穿过第二个palace。此前矩阵只检查物体/标签，未覆盖辅助线与物体的关系。

修复仅调整同一draw中的辅助线：新景别箭头按主体缩放预留间距，隔墙位于最后两个主体边缘之间；单主体时位于主体右侧留白。reference仍使用原坐标。未改变物体几何、构图摆位、字幕触发、原声、模型契约、数据库、依赖或权限；属于fix。修改绘图、Node回归、技术设计及本验收记录，保留已有暂存改动，未重新暂存或提交。

本轮实际命令（仓库根目录；node/npm在当前进程PATH前置独立Node 24.22.2，不改系统配置）：

- `node --test --test-name-pattern='reframed' tests/video-render.test.mjs`：有效输入下先复现箭头方向与隔墙穿物体两项断言失败，修复后两项通过。
- `npm run test:video`：29项通过，新增箭头方向/字幕触发/预览一致性、三景别1—3主体隔墙边界、原布局辅助线保留共3项回归。
- `node output/playwright/uncommitted-review/verify-guides.mjs`：真实引擎108组（3构图×3景别×1—3主体×2环境×draw/tilt），每组3时刻，0辅助线方向/隔墙边界问题、0引擎错误；route/transfer/wall的reference画面分别与修复前暂存绘图逐像素一致。脚本复用P1 atlas启动引擎并注入当前源码，未编码MP4。
- `npm run lint`、`npm run build`（含TypeScript）：通过；已有npm用户配置及Vite大包提示保留。
- `git diff --check`、`git diff --cached --check`：通过。

证据在忽略目录`output/playwright/uncommitted-review/`：`guides-evidence.json`、`route-detail-fixed.png`、`transfer-detail-fixed.png`、`wall-detail-fixed.png`，截图已查看。验收满足箭头正确朝向后续主体、隔墙不穿主体、原布局不变。本轮未重跑Rust、全量前端测试、完整视频编码、安装包或外部模型；此前P1验收命令是历史记录，不能视为本次重跑。下一步在包含修复的新桌面构建中重新生成试片检查。
