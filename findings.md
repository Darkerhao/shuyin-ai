# 创作体验续作决策（2026-10-25）

- SQLite v8 已有按项候选暂存；文案与封面的自动草稿使用独立表，不滥用正式版本或 app_settings。
- 封面所有预览/导出共用 coverRenderer；新增归一化裁切位置和缩放，旧配置缺省居中/1倍。
- 发布任务保存文字快照但未保存 writingVersionId；增加可空外键兼容旧任务，并保留自主编辑。
- 完整备份使用可搬移目录；只收集数据库登记文件及任务包已知文件，缺失必需文件明确失败，视频是否复制由用户选择。
- 备份哈希使用现有锁文件sha2 0.11.1直接依赖，64KB流式读取可选视频；恢复统一复用数据库快照校验/替换，不增加压缩协议或第二套恢复状态。
- 发布缩略图沿用已有decodeBackground返回data URL，符合当前CSP，无需开放blob来源。原生关闭API实际会await回调后destroy，增加最小事件/销毁能力即可在关闭前完成草稿写入。
- 草稿的内容修订序号只用于状态提示，存储仍为同一串行队列；慢写入完成后不提前宣称新内容已保存。
- 完整备份目录搬移后按原路径键映射新位置，任务包按源系统分隔符匹配；未包含视频保留原路径，缺失会撤销ready。当前Windows实测与Rust回归均通过，跨OS实机另验。
- Windows隔离验收全部结束，正式库SHA-256不变；本轮验收与范围见docs/CREATOR-WORKFLOW-VERIFICATION.md。

# M0 调研记录（历史）

## 当前安装升级边界（2026-10-24）

- 当前工作区干净，HEAD=69c8fe7。最近文档已记录本机 AI 封面通过，但开发计划 M3 段仍称 Provider 待实现，需要同步。
- 现有独立安装 com.shuyin.m3-install-verification 位于 output/playwright/m3-install/installed；可继续验证真实 v5→v6 升级，无需重建旧版或触碰正式产品。
- Database 初始化按 MIGRATIONS 顺序事务迁移，系统凭据服务名采用 app identifier；保留测试 identifier 即可验证升级/重启后的原凭据引用。

## 图片模型与异常保护验证（2026-10-24）

- 实际 Windows WebView2 中保存图片模型成功；模型用途、按用途默认值和系统凭据在正常关闭重启后保留，前端密钥输入回空。更换服务地址但未补密钥被拒绝，原配置/凭据不变。
- 本机 Images 契约服务确认真实 Rust HTTP 使用配置的模型、base64 PNG 和授权头（证据仅记录头是否存在，不记录密钥）；生成背景后双图导出、历史追加和备份恢复闭环通过。
- 7类服务异常、取消后迟到响应、真实背景目录不可写、第二张PNG数据库写入失败共10项保护检查通过。取消后无新素材；部分导出保留第一张及未保存草稿，解除故障可重试。旧文件哈希与正式库哈希不变，无业务源码修复需求。
- 验收脚本修正动态蒙层控件、离页确认、Windows扩展路径前缀以及暂移目录期间 existsStatus=missing 的预期；都是脚本断言问题，未修改业务代码规避结果。所有临时故障均在 finally 恢复，数据库检查确认测试触发器已删除。
- 本轮9张导出PNG全量解码通过，schema6完整性与外键正确；实际图片来自可控本机响应，外部生成质量、当前源码安装升级和其他平台仍未验证。

## 可选 AI 图片接入（2026-10-23）

- 实际模型配置和凭据集中在 database/ai.rs；ai_tasks.kind 无枚举约束，可共用取消与重启恢复。新增 model_kind 区分用途，默认模型按用途唯一；旧模型保留为 text。
- 官方 Images Generations 协议支持 model、prompt、n=1、size=1024x1024，GPT Image 返回 data[0].b64_json，不发送其不支持的 response_format。只接入此协议，图片模型 ID 由用户填写。
- 官方依据：https://developers.openai.com/api/reference/resources/images/methods/generate/index.md 。生成一份方形无字背景，由本地排版分别裁切到 4:3/3:4。
- image 解码限制与 base64 Engine API 已查官方文档/源码入口；落地前继续检查所锁版本源码，避免手写图片解码器。
- 最终复用已有传递依赖 png 0.18.1 与 base64 0.23.1，只将其声明为直接依赖，Cargo.lock 无新增包。已阅读本机锁定源码 Decoder::new_with_limits/read_info/next_frame/finish、output_buffer_size 及 Engine::decode；不用 image 多格式处理库。请求 output_format=png，完整像素和结尾解码通过后才入库。

## 当前发布图片封面结论（2026-10-23）

- 用户要发布作品时上传的两张图片：横版4:3（1440×1080）、竖版3:4（1080×1440）。一份草稿分别排版导出两张，比例选择用于预览；原四种尺寸记录仅作历史证据。
- 复用现有Canvas与逐版本保存，无新增迁移、依赖或成组状态。两张先渲染再保存，第二张保存失败保留首张、成功路径与草稿；旧尺寸仅在恢复编辑时转成目标比例，不改旧行和文件。
- 隔离Windows WebView2真实UI导出纯色/图片背景共四张PNG，尺寸、中文/英文边界、历史不覆盖及页面重新加载恢复通过；schema仍为v5。当前比例修正未重做安装升级或目标平台上传，详见docs/M3-VERIFICATION.md文末。

## M3 重新启用（2026-10-23）

- 后续安装补验：独立M3产品基线来自77bd9f6，实际NSIS 0.4.0安装成功，SQLite为v4七张表，创建了书籍、发布版本、模型配置、取消任务、图片关联与备份。正式产品注册和文件哈希不变。
- 两次独立升级完成：0.4.0→0.5.0保留旧记录并迁移v5；0.5.0→0.5.1八表完全一致，3个封面版本/PNG内容保留。正常退出重启与安装版备份恢复一致，原生文件选择/取消已实测。
- 原生文件对话框按元素点击曾被工具命中检查拒绝；重新截图后使用该模态截图坐标成功。验收脚本明确使用UTF-8读取快照与统一Windows扩展路径前缀。

- 用户明确重新启用原 M3；原三项生成及数据保留。初始工作区干净，HEAD 为 77bd9f6。
- 复用素材关联、项目目录、顺序迁移、备份 schema 核验。新增 cover_versions 一张表，配置用带版本号的 JSON 存储，历史仅追加。
- Canvas 同一渲染器负责预览与 PNG，中文按字素测宽、换行与缩小；本地字体和原生 Canvas 不增加依赖。
- 查阅 Tauri 官方 calling-rust 文档及已安装 core/dialog 源码：使用受限业务命令及 ipc::Response 读取已关联图片，不开放任意文件读取权限。
- 官方参考：https://v2.tauri.app/develop/calling-rust/ ；https://developer.mozilla.org/en-US/docs/Web/API/CanvasRenderingContext2D/measureText 。
- 实际 WebView2 四种尺寸长中文/中英文字边界全部合法；审阅导出 PNG 中文显示正常。6 个真实 PNG 解码通过，7 个版本完整重启和备份恢复一致。
- 1420 端口属于系统保留范围，隔离验收用15173。新 NSIS 构建通过但未安装；正式数据库哈希未变。

## 当前三项生成结论（2026-10-23）

- 用户只需要发布标题、发布内容和音频生成提示词；书名必填，作者和内容选填。单次请求生成一套结果，删除主流程的旧类型/数量/参考版本及任务列表控件。
- 音频模板使用甲子主持、8—10分钟、固定系列话术；模型只生成书籍引入及五段指导，Rust拼接，避免每次重写固定开场结尾。保留旧五类数据读取能力，不增加第二条生成路径或依赖。
- v4同时重建版本与任务表，事务中保留所有旧行及任务外键；真实升级后七表旧行逐字段一致。旧Prompt文件保留为既有版本来源，当前生成只引用publishing-v1及audio-template-v1。
- 《明朝那些事儿》和《白鹿原》真实服务各生成一次，书名和作者以外的资料为空；三项复制写入Windows剪贴板后读回一致，编辑追加v2并可切回v1。两本的五段指导与体裁相符。
- 隔离安装升级0.4.1→0.4.2通过，正式数据库SHA-256未变。实际音频、个人笔记、macOS/Linux和正式发布未验收。完整证据见 docs/PUBLISHING-VERIFICATION.md。

- 根目录仅 AGENTS.md、docs/ 下 5 份设计文档和原始 zip；无源码、package.json、锁文件或 Git。
- 已完整读取 AGENTS.md、PRD、TECHNICAL-DESIGN、DATABASE、AI-RULES、DEVELOPMENT-PLAN。
- Node.js 22.21.1；npm 10.9.4；pnpm 9.12.1；Bun 1.4.2；Yarn 1.22.22；Git 2.45.2。
- npm 缓存已在 D:\DevCache\npm-cache，使用官方 registry。优先 npm，不新增包管理器。
- Rust / Cargo / rustup / cl / link / CMake / Tauri 未出现在 PATH，常用 Rust 和 Visual Studio 目录未发现安装。
- 注册表有 Edge WebView2 Runtime 149.0.4022.80；OSVersion 为 Windows 10.0.22631（Windows 11）。
- M0 要求基础路由布局、SQLite 持久化、本地视频选择、错误处理和 Windows 包；完整业务迁移在 M1。
- 参考官方 Tauri prerequisites、dialog、windows-installer 和 Vite guide；具体版本仍待 registry 与源码核验。
- registry 核实：Vite 8.3.5、Vue 3.5.44、Tauri JS/CLI 2.12.4、Rust crate 2.12.5、Dialog 2.8.3；Node 22.21.1 满足主工具引擎要求。
- TypeScript 最新 7.0.2 超出 typescript-eslint 8.72.0 的 <6.1 范围，因此采用 6.0.3；Vue Router 使用 4.6.4 的基础路由 API，不引入 5.x 扩展。
- Vue Test Utils 2.5.1 -> js-beautify 2 -> nopt 10 / abbrev 5 要求更高 Node 补丁；2.4.6 解决引擎冲突。
- 阅读已安装 Dialog open 实现及类型、Tauri invoke/isTauri、Router hash history 和 Vitest 配置；文件选择返回 string/null，取消保持原路径。
- Rust 1.99.1 + Cargo 1.99.1 已安装到 D:\MyWork\software\rust；不修改全局 PATH，当前会话配置在忽略的 .tools/env.ps1。
- 已启动经微软签名验证的 Build Tools 2022 安装，主目录 D:\MyWork\software\VisualStudioBuildTools；Installer 与 Windows SDK 有不可避免的 C 盘共享组件。
- 方案：Rust rusqlite bundled SQLite；单条 m0_probe 技术记录；事务迁移 user_version=1；只暴露 read_probe/save_probe 和 dialog:allow-open。
- 最终 DOM 测试环境采用 jsdom 27.4.0，清除 happy-dom 可选 peer；不弱化类型检查。
- MSVC 14.44.35207 / SDK 10.0.26100.0 安装成功，Tauri info Environment 全通过。
- NSIS 缓存 rename 在用户目录报 os error 17，项目 target/.tauri 路径实测成功；不得把源码中不存在的临时目录跨盘流程写成根因。
- release NSIS 2,715,202 字节，实际安装至 D:\MyWork\software\shuyin-ai；安装版页面来源 tauri.localhost，读回相同文字和 UTC 时间。
- 终检初始 zip 不存在，本轮未移动或删除；不要恢复或覆盖用户工作区。

## M1 调研与取舍（2026-10-23）
- 数据库顺序迁移，保留 M0；只创建实际使用的项目/素材/普通设置三表。
- 不需要额外 UI 库、全局状态、UUID 包；沿用原生控件和已有 SQLite。
- rusqlite 0.40.2 backup.rs 的 run_to_completion 对 Busy/Locked 可持续循环；本实现单次 step(-1)，忙时明确报错，避免无限重试。
- Rust canonicalize 在 Windows 返回扩展路径；Shell 打开前转换 DOS/UNC 格式，官方 Rust 与 Microsoft 文档均说明文件系统与 Shell 路径要求不同。
- 桌面启动需写隔离应用数据目录，被沙箱拒绝；自动升级审核两次分别超时/服务断连。浏览器工具也被审核服务故障阻塞，未绕过审批。
- 原生桌面操作待验收，自动化通过不等同于真实桌面验收。

## M1 桌面实测补充
- 实际 CDP 连接 WebView2 可验证 UI→IPC→SQLite，无 mock；测试库标识 com.shuyin.m1-verification，与正式库隔离。
- 原生对话框可显示，桌面输入自动化焦点不稳定，因此成功选择/取消仍不计通过。进程重启后数据库字段和素材关联一致的证据独立有效。


## M1 安装升级证据
- 生成 NSIS 脚本的卸载 key 使用产品名，进程检查使用 mainBinaryName；隔离测试同时更改 productName / identifier / mainBinaryName，避免覆盖正式产品。
- Tauri --config 支持临时测试版本0.1.0/0.1.1；本轮构建未改变 Cargo.toml 或正式 tauri.conf.json 版本。
- 安装验证不能只看构建成功：本轮实际 NSIS exit0、生产URL、安装路径、注册版本与升级前后数据库字段均核对。
- 自动审批授权与服务可用性独立：明确授权后安装通过，但原生输入被审核服务断连阻塞，不能记为安全拒绝或验收通过。


## M1 核心验收完成
- 权限环境更新后，原生输入/确认成功；中文空格路径视频落库并显示 available。
- 第二次原生选择后 Esc 取消，check-native-cancellation.cjs 比较全部素材字段一致，按钮恢复启用。
- 1秒样本不适合肉眼验收，生成120秒绿色样本后经安装版打开；GeePlayer标题匹配、画面正确、进度00:11→00:13，总长02:00。暂停请求因窗口已最小化未执行，未改变播放器后续状态。
- 本轮无业务代码/依赖/迁移变更，更新验收文档。全部 M1 核心验收通过，未提交/发布，M2尚未开始。

## M2 实施接点
- 项目业务入口为 src-tauri/src/commands.rs，数据库在 database 子模块；当前迁移为 v2。
- 备份恢复校验绑定当前 schema，新增 AI 表时须同步升级与回归。
- 前端沿用 services/projects.ts 和项目详情路由，不需要为 M2 预先引入全局状态库。

## M2 本轮决策
- 实现 OpenAI Chat Completions 兼容适配器：只发送 model/messages/stream，不假设模型支持温度或 JSON mode；使用 JSON Prompt 和本地结构校验。
- crates.io 当前版本：reqwest 0.13.5、keyring 4.2.0、tokio 1.54.1；已读取对应下载源码及公开 API。keyring 4 的默认 v1 模式使用各平台系统存储，不使用旧版 feature 名称。
- 网络等待不持有数据库锁；任务成功与版本追加使用事务；取消和完成在同一数据库边界判定。
- 创作按大纲、讲稿、标题、描述、话题分别追加不可变版本；大纲/讲稿为 Markdown，其余为字符串数组。保留生成来源和 Prompt 版本。
- Tauri 的命令处理器之外，本仓库还有 build.rs AppManifest 命令清单与 permissions/capabilities 两级授权；新增命令必须完整登记。真实 WebView 验证发现并修正了此前遗漏。
- 原生 Windows 凭据库、进程重启、备份恢复实测通过；本机 HTTP fixture 9 次请求全部通过 Rust adapter 并读取系统凭据。没有真实远程模型 Key，不能将契约测试表述为供应商质量验收。

## 实际服务与安装升级准备（2026-10-23）
- 用户指定 Base URL 为 https://bmapi.020212.xyz/v1，模型 ID 为 gpt-6-astra。密钥尚待用户在应用中输入。
- 用户给出的 D:\书引 AI\output 存在但为空；随后明确目前没有读书笔记。改用有来源的《论语·学而》前四章作为公开原文基准，不记作用户个人笔记验收。
- 正式数据库仍为 schema v1；不从其他应用搜集凭据，不修改正式数据。安装测试采用独立产品名、identifier 和 mainBinaryName。
- HEAD 3093f35 是 M1 源码，可从 Git 归档创建独立构建快照，验证真实 v2→v3，而非仅更换测试版本号。
- 用户已在安装版直接保存密钥，list_model_providers 只返回 available；真实 HTTPS 连接测试成功，耗时 3966ms，没有读取或记录密钥内容。
- 验收资料与最小笔记模板已写入用户指定目录。原文可核对 Wikisource《论语/全览》学而 1.1–1.4；生成质量按原文归属、解释边界、现代类比、结构、时长和发布文案一致性评估。
- 五类真实生成各一次均成功，耗时分别 60.171/28.003/7.024/15.722/4.945 秒；事实归属、类比标注、待核实项和候选数量均符合公开原文基准。修订稿口播部分约 487 个中文字符，时长只作估算，未实际试读。
- M1→M2 安装升级原四表数据一致；M2 再次安装升级全部七表数据一致，7 个版本和 5 条成功任务保留。用户密钥在进程替换后仍可用，真实 HTTPS 连接复测 3.594 秒通过。
- 正式数据库、原安装包 SHA-256 和正式安装注册信息前后完全一致。独立测试 SQLite/WebView 仍使用 C 盘用户 AppData，安装与项目资料位于 D 盘；不是完全零 C 盘写入。

## 安装版 AI 封面续验结论（2026-10-24）

- 当前源码在真实 NSIS 0.5.1→0.6.0 升级中完成 v5→v6 数据迁移；旧行逐字段保留，新增 `model_kind=text` 与 `asset_id=NULL` 默认值符合设计。
- 安装版 UI、Rust HTTP、系统凭据、SQLite 与 Canvas 闭环通过：图片模型/凭据重启可用，生成仅替换背景，两比例 PNG 与历史追加正确。
- v6 备份恢复及恢复后再次重启均保持模型、凭据引用、素材、任务和封面；正式安装文件、数据库和注册信息未变化。
- 验收脚本曾误用封面历史升序假设，且 NSIS 后处理会使安装 EXE 与 target 预封装 EXE 哈希不同；已按真实排序和安装文件版本修正，未修改业务实现或弱化校验。
## 外部图片服务结论（2026-10-24）

- New API 兼容服务默认 `response_format=url`；图片客户端必须显式请求 `b64_json`，不能仅假定端点兼容。
- UGQ 的 `gpt-image-2` 在 `b64_json` 请求下返回 1024×1024 PNG；其绘图日志中单次实际费用为 $0.15。
- 图片格式应由响应真实内容决定，而不是固定为 PNG；完整解码后再按 PNG/JPEG/WebP 扩展名保存，避免伪造扩展名。
# M6 续作核对（2026-10-26）

- M7不需要新队列表：原PublishTask、writing_version_id和publish_records已覆盖来源/归档。全局查询关联书名后复用checked，精确task ID进入旧任务可避免默认选最新导致串篇；记录事务成功后释放匹配内存上传会话，避免下一篇被旧快照永久阻塞。
- 安装版clipboard.writeText可用；CDP触发额外clipboard.readText会等待授权，不用于产品。测试改为PowerShell只比较已知本轮写入链接，不读取其他剪贴板内容。隔离模拟记录必须明确“未在抖音发布”，真实M6任务不标记成功。

- 最新安装版首次上传的真实时序：标题/简介/封面区先挂载，预览面板稍后加载，视频再进入可预览状态。只用左侧控件判断 editorReady 会过早尝试切换视频预览；就绪检查必须包含真实预览页签。该页签包含提示图标，不能用“无子节点”的通用文字查找。
- 独立安装版远程网页调用workspace_settings返回ACL拒绝；官方暂存后已有草稿提示可拦截原生重传。升级前SQLite备份与当前12表逐行一致，schema9、完整性和外键通过。用户开发实例的人工合集/章节草稿仍需保护；仅重置本轮安装版自动生成的测试草稿以验证完整链。

- 当前真实页面验证了四个行为：非空简介恢复后移除 data-placeholder，改用 contenteditable/data-slate-editor/editor-comp-publish 唯一定位；预览封面页签移除 video 节点，开始填写前切换预览视频；只保存横封面会提示设置竖封面，改为同一弹窗依次设置后完成；暂存离开后上传页出现“上次未发布”入口，必须拒绝重新选视频。
- M6.1 真实 265922746 字节、697.621 秒视频已进入官方编辑页；文案与两张 1440×1080/1080×1440 图回读并保存草稿，未执行最终发布。正在复验新宿主命令完整链。

- 工作区已有独立抖音窗口、页面检查和技术验证组件，尚无文字/文件操作；设计文档状态未随初始代码更新。
- 当前 Tauri 2.12.5 / webview2-com 0.39.1 支持 with_webview、eval_with_callback、宿主 CallDevToolsProtocolMethod。文件操作应限定已校验任务的路径，远程窗口不授予 IPC。
- 发布包校验集中在 database/publishing.rs 的 checked/manifest/package_valid；直接在这里提供快照，不另建解析器。
- 初次 npx --no-install playwright-cli 无可执行项，改用当前可用 Playwright 工具和隔离 WebView2 联调。
