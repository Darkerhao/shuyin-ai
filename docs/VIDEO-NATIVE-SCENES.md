# 视频专属场景组件接入（2026-11-26）

## 已实现范围

在原 42 套视频风格基础上，直接调用外部 Frametell 0.7.3 的原生 `kit.js`。不复制引擎，不增加 npm/Rust 依赖，不新增数据库字段、迁移、IPC 或模型请求。

| 主题 | 实际组件 | 可用主体及行为 |
| --- | --- | --- |
| pixel 像素游戏 | `P.pixel`、`g.hero`、`g.sprite` | 机器人、小虫、宝箱、星星；原 93 种物体复用已有几何生成低分辨率精灵，硬币和钥匙使用原像素组件 |
| manga 黑白漫画 | `P.grid`、`P.panel`、`P.chara`、`P.prop` | 两种虚构讲解角色；书、时钟、日历、床、杯等使用原道具组件，其他原物体保持同一几何并进入分格；按原字幕锚点逐格出现 |
| pop 波普漫画 | `P.grid`、`P.panel` | 彩色网点分格，原物体进入对应格，保留原标签和动作 |
| cosmos 宇宙星空 | `P.space().body/label/cameraTo` | 地球、月球、太阳、火星；示意、不按真实距离和比例排列 |
| diorama 微缩城市 | `P.diorama().city/zone/landmark` | 城市、住宅、森林、农田、工厂，使用有种子的微缩场景，不声称具体地点复原 |
| exploded 产品拆解 | `P.studio().turn/explode/assemble` | 手机、电机、电池、芯片、涡扇，使用组件自带结构模型；不自动标注部件参数，多个产品按不同字幕锚点依次展示 |
| globe 地球仪 | `P.globe`、原 `world.js` 海岸线 | 一个三维地球仪，不添加地点坐标、国界、路线或统计数字 |
| molecule 微观世界 | `P.micro().molecule/helix/cell` | 水、氧气、二氧化碳分子，DNA、细胞结构示意；不输出真实序列、尺寸或医学机制 |

共新增 26 个受控主体，唯一目录为 `src-tauri/video/scene-kits.json`。普通物体目录仍为原 `visual-catalog.json` 的 93 项。非对应主题的专属主体、未知主体、未知动作均拒绝；三维专属主体只开放 draw/check/cross。地图每镜一个，多个产品不能共用同一个 cue。

## 业务闭环

- 自动/手动任务沿用原 `theme`；分镜生成将冻结主题加入模型输入。模型只返回现有 `Beat.object/action/cue/label`，不获得执行代码、任意坐标或远程资源的权限。
- 主提示词组合独立 `video-scene-kits-v1` 规则，单镜调整版本记为 `video-scene-revision-v2`；单镜生成以数据库任务主题为准，不接受调用方伪造主题。
- 编辑器将当前主题专属主体放在物体下拉最前面，随后保留93个通用物体；切换到三维主体时不兼容动作恢复为 draw，后端再次验证。保存、单镜候选采用、自动渲染交接和实际渲染均检查主题边界。
- 三维主题只有选择专属主体时才创建三维场景；其他内容仍是原图解。像素和两种漫画主题对普通物体也使用原生画布/分格。图片镜头和历史逐镜水墨仍走原图片路径。
- 不为匹配主题编造天体、产品、地点或生物结构。漫画角色是虚构阅读/讲解角色，不是作者或历史人物肖像；不增加对白、不替换原声音频。
- 源场景文件与 three vendor 文件进入缓存键；缺少 kit/world/vendor 时明确报错。Rust 将 `scene-kits.json/js` 写入独立任务目录，环境检查也带同一目录；浏览器环境检查实际启动 WebGL2。
- 所有绘制复用原 cue 秒数和完整 drawDuration。试片只截取观看/音轨范围，不按较短时长加速产品拆解等动作。
- 为避免引擎两次初始化（发现字体、正式绘制）重复创建场景，三维资源在实际页面首次绘制时创建，隐藏文本先完成字体发现；像素精灵按物体缓存，解码完成后才开始捕获。
- 文字传入源 kit 的 HTML 标签接口前转义，普通文字使用原安全文本接口。专属模型没有未经校验的执行内容。

## Windows 三维渲染修复

本机对照发现源 `engine/tools/lib.mjs` 对所有 WebGL 影片统一使用 `--use-angle=metal`。Windows 原参数下，电机示意的原确定性预检报 6.28% 像素差异，宇宙六秒样片一帧缺地球；HTTP Chromium 同内容未复现。仅通过 `FRAMETELL_CHROME_ARGS=--use-angle=d3d11` 重跑，原确定性检查通过，联系表主体连续出现。

因此 `videoBrowserArgs` 仅在 Windows 三维主题且用户没有显式选择后端时，设置引擎已有的尾部参数为 D3D11；macOS/Linux 和已有显式参数保留。未修改源引擎、未放宽原确定性检查。此边界有 Node 回归测试；真实重编的电机和宇宙样片已核对。

## 验证记录

以下从仓库根目录执行，Node 使用 `D:/MyWork/software/shuyin-video-node/v24.22.2/node.exe`，Rust 前执行 `. ./.tools/env.ps1`；媒体命令把 `D:/MyWork/software/ffmpeg/current/bin` 加入 PATH。本机夹具、脚本、截图、诊断 JSON 与样片在忽略目录 `output/native-scenes-20261126/`，不读取正式书库、不调用付费模型。

| 实际命令 | 结果 |
| --- | --- |
| `node node_modules/vitest/vitest.mjs run tests/video.test.ts tests/video-scene-revision.test.ts tests/video-storyboard.test.ts tests/automation.test.ts tests/automation-tasks.test.ts tests/video-environment.test.ts --maxWorkers=2` | 6 文件、77 项通过；新增原生主体的动作修正与保存测试 |
| `node node_modules/vitest/vitest.mjs run tests/video.test.ts -t 'edits a native subject' --maxWorkers=1` | 调整测试选择器类型后单项通过，其余40项按筛选跳过 |
| `$env:SHUYIN_TEST_FFMPEG='D:/MyWork/software/ffmpeg/current/bin/ffmpeg.exe'; npm run test:video` | 56 项通过、0 跳过；包括实际音频标准化、所有主题原时钟编译、26主体归属与动作校验、资源缺失与缓存来源、Windows后端选择 |
| `npm run lint` | 通过 |
| `npm run build` | 类型检查与生产构建通过；保留原有 npm 本机配置提示、Vite 大分块提示 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked --lib -- --quiet` | 338 通过、2 原有忽略；包括26主体校验、主题持久化、跨主题编辑拒绝、单镜以冻结主题生成/拒绝异类候选、旧流程兼容 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | 通过 |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets -- -D warnings` | 通过 |
| `node output/native-scenes-20261126/prepare.mjs`、`node output/native-scenes-20261126/extra-fixtures.mjs` | 按共享目录生成原生主体、普通物体、混合分镜、两镜完整/试片夹具；专属主体总数26 |
| `ego-browser nodejs -e "await import('file:///D:/MyWork/project/JIAZI-PROJECT/shuyin-ai/output/native-scenes-20261126/round-03.mjs')"` | 37组夹具（26单主体＋主题/混合场景）实际浏览器无运行错误、无缺失资源，正放/回退/同帧结果一致；五套3D均加载three r180并实际输出canvas |
| `ego-browser nodejs -e "await import('file:///D:/MyWork/project/JIAZI-PROJECT/shuyin-ai/output/native-scenes-20261126/round-05.mjs')"` | 93种通用物体逐项在像素画布真实解码、出现；固定背景下每种物体新增212—2494个逻辑像素，非空白精灵 |
| `ego-browser nodejs -e "await import('file:///D:/MyWork/project/JIAZI-PROJECT/shuyin-ai/output/native-scenes-20261126/round-09.mjs')"` | pixel/manga/pop/五套3D共8主题，12秒双镜全片第8.5秒与从第二镜开始3秒试片第2.5秒一致。微缩城市Canvas有1个抗锯齿像素差异（0.000048%），低于源引擎0.05%容差，其他Canvas完全相同 |
| `ego-browser nodejs -e "await import('file:///D:/MyWork/project/JIAZI-PROJECT/shuyin-ai/output/native-scenes-20261126/round-15.mjs')"` | 真实Vue组件＋隔离IPC：宇宙主题下拉97项，4个专属主体排前；选择地球自动把slide修正为draw，动作仅draw/check/cross，实际save_video_plan正确提交space_earth，未知IPC为0，无横溢 |
| `node src-tauri/video/render.mjs output/native-scenes-20261126/renders/pixel-book/input.json output/native-scenes-20261126/renders/pixel-book/result.json`，manga-book执行同样路径模式 | 两份真实MP4均6秒、1920×1080、24fps，布局检查 `[]`；联系表已查看，像素角色和漫画分格按第3秒锚点出现 |
| `node src-tauri/video/render.mjs output/native-scenes-20261126/final-renders/cosmos/input.json output/native-scenes-20261126/final-renders/cosmos/result.json`，product_motor执行同样路径模式 | 两份真实三维MP4均6秒、1920×1080、24fps、144帧；H.264＋AAC双声道48000Hz，原引擎布局/确定性检查通过，电机展开/合拢可见 |
| `node src-tauri/video/render.mjs output/native-scenes-20261126/previews/molecule/input.json output/native-scenes-20261126/previews/molecule/result.json` | 真正3秒试片（容器3.008秒），1920×1080、24fps；第二主体未到原第3秒不出现。有1项原引擎稀疏构图建议，未忽略为零提醒 |
| `python C:/Users/haozi/.agents/skills/ffmpeg-skill/scripts/probe.py output/native-scenes-20261126/final-renders/product_motor/video.mp4 --json-brief`，对cosmos执行相同probe | 读取实际尺寸、帧率、帧数与音视频时长 |
| `python C:/Users/haozi/.agents/skills/ffmpeg-skill/scripts/look.py output/native-scenes-20261126/renders/pixel-book/video.mp4 --tiles 3x2 --width 1200 --json-brief` | 对pixel-book、manga-book、cosmos、product_motor和molecule试片均生成联系表并实际查看；修复后宇宙主体各采样帧连续可见 |
| `python C:/Users/haozi/.agents/skills/ffmpeg-skill/scripts/check.py output/native-scenes-20261126/final-renders/product_motor/video.mp4 --platform custom --json-brief` | 5 PASS、0失败、1提醒（本来采用烧录字幕，没有软字幕轨）；ok=true、verified=false，不宣称全项目媒体验收 |
| `git diff --check` | 通过 |
| `node output/native-scenes-20261126/check-runtime.mjs` | 本机实际Node、42风格/组件资源、FFmpeg、浏览器WebGL2和字体5组均ok |

首轮lint出现新props默认值/属性顺序、Node测试全局声明提示，均已修复；最终构建还发现泛型Select的测试工具类型需用已声明的组件名UiSelect，已改用公共组件事件，未弱化类型规则。真实浏览器发现旧HMR计算属性仍显示旧排序，完整刷新后确认专属主体在前；旧长下拉的屏外点击未通过，改为前置原生主体后完成真实点击和保存。首次试片比较直接比较PNG字符串过严，微缩城市出现1个GPU抗锯齿像素差异，随后沿用原引擎的颜色差值和像素比例阈值完成8主题验证。没有放宽产品实际渲染的检查或删除失败断言。

### 修改文件

新增 `scene-kits.json/js`、Rust `video/scene_kits.rs`、独立提示词和本说明；修改 `runtime.mjs`、`render.mjs`、`illustrations.js`、`video_commands.rs`、`video/environment.rs`、`video.rs`、数据库 video/automation/revision 校验；前端 video 类型、VideoPage、VideoSceneEditor、VideoSceneRevision、AutomationSetup；相关测试、五份主文档、VIDEO-THEMES和渲染README。任务规划在 `docs/plans/native-scenes-20261126`，根级原规划记录完整保留。

## 未覆盖范围

本轮采用原组件有限主体与固定构图适配，不是任意三维建模、语义生成任意产品或地理路线。不提供漫画对白/气泡编辑、虚构角色服装配置或新的产品部件编辑器。尚未验证真实长音频、多小时显存、安装包、正式桌面、macOS/Linux 实机与真实模型输出质量；后续仍由用户检查内容和最终发布。
