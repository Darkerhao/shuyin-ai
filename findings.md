# 自动流程提交后复查（2026-11-07）

- 初始dev为`ce42def`且工作区干净，main为本轮cherry-pick `ddb7d66`。现有计划第6项仍写进行中，已依据实际refs补为完成。
- 使用code-review两轴复查原功能提交，两轴未找到新阻断项；主线程新增模型元信息变更回归实际复现P2：`verify_automation_input`比较整份ProviderInput JSON，改显示名、变更同用途默认模型都会误暂停冻结了模型ID的任务。
- 最小修复在原校验内只比较用途、服务地址、模型名、推理强度、启用状态；显示名和默认标志仍保留在原快照，但不影响已启动的调用。更改服务地址等实际调用配置仍拒绝。
- 测试首次不带模块路径配合`--exact`筛出0项，修正为完整测试路径后得到1项真实失败，未将空测试记为通过。无数据库结构或前端行为改动。

# 自动制作与网页发布实施发现（2026-11-07）

- 用户接受最终人工点击发布后，直接复用现有网页适配，官方API缺口不再作为阻塞。一个Rust固定执行器、一个流程表；根级封面只是原Canvas执行端，不再建设官方服务或重复视频引擎。
- 原音频创建方法拆出共享业务入口，自动任务只用固定指导/真实文件，writing_version_id=NULL。字幕文案通过统一Provider处理全部字幕并读取实际冻结音频指导，保留原手动生成。
- 自动绑定与子任务创建共用事务，SHA和账号/模型快照防混用；人工确认字段false，通过完整自动策略自检。上传开始和交接用户分别记账；交接前日期过期可同任务重建包，交接后及未知会话不能自动重发。
- 双轴审核定位并修复混合字幕版本、执行器占用遗留running、迟到封面失败、音频摘要/指导脱节、预约日期跨日及父停止不影响子任务等问题。最终复核无新阻断项。
- 真实隔离WebView2验证从已准备音频字幕夹具开始：1文本/2图片调用、2张Canvas PNG、6秒1080p含音轨成片和正确发布任务全部自动完成。重启未知上传不重发；本机契约服务不代表真实账号/整本书内容质量。验收见docs/AUTOMATION-VERIFICATION.md。

# 全自动实施 P0 复核（2026-11-07，前序记录）

- 用户已授权实现及完成审核后的 Git 交付；先按原计划 P0 核验。当前 dev 基线 `91461ae`，0.51.1、schema19，开始时已有五份规划文档改动，均保留。
- 本轮实时获取9个主要官方文档及公开列表，并解析创建页完整请求/响应结构：21个正文参数含 `download_type=1`（作者不允许下载），无已证实的预约/AI声明字段；4个创建回执字段不包含设置回读。禁止下载枚举由未知更新为已核实，实际效果与回读继续待验证。
- `video_basic_info` 要另一个 `posting.behavior` 权限，示例 `video_status=5` 缺正式结构/枚举依据；分享结果和 `create_video` 事件属于 `aweme.forward`，不能直接充当本路线的公开审核结果。
- 当前源码只有网页辅助填写和人工发布记录，没有官方OAuth、发布服务或跨阶段持久编排。现有网页设置不能证明API原生定时可用，也不能自动写人工确认字段来冒充新流程。
- 证据及所需非密钥接入资料见 [P0核验记录](docs/BOOK-TO-DOUYIN-P0-VERIFICATION.md)。P0尚未通过，未实现功能、未改数据库、未执行投稿；用户要求的完成及审核通过条件未满足，因此不提交调查文档冒充功能，也不推送或cherry-pick。

# 书籍到抖音全自动流程：官方发布研究（2026-11-07，前序规划依据）

用户后续明确的默认值：AI 背景图、北京时间凌晨01:11定时发布、不允许保存视频。它们是需求，不能据此推断平台接口已支持。计划因此将原生定时和禁止下载回读升级为P0必要条件；复用现有图片Provider按4:3/3:4各生成一张无字背景，禁止失败后静默改用纯色。定时采用满足预约窗口的未来01:11完整日期，旧任务不被默认配置回填。

最终全自动核查：用户要求配置后正常运行不再操作，因此明确AI结果自动采用、视频校验后自动渲染、预算内调用不再确认、自动提交预约并自动归档实际结果。创建意图/调用之前可以按既定策略自动计算下个有效01:11；已提交/未知或固定绝对日期失效时核对并暂停。需要逐次人工最终发布的路线不满足需求，下面研究中提到的半自动只能作为应急或阶段成果，不能冒充全自动交付。新增三本书启动后零点击及正常等待/续期验收，未新增真实平台验证。

本节为本轮公开文档研究，不代表已获得平台权限或完成真实接口验收。通过 Python 标准库只读获取官方页面并提取正文；未访问已登录控制台、读取Cookie或操作真实作品。下列官方资料访问日期均为2026-11-07。

- 新版[创建视频](https://developer.open-douyin.com/docs/resource/zh-CN/dop/develop/openapi/video-management/douyin/create-video/video-create)正文于2026-11-07可读：`POST https://open.douyin.com/api/douyin/v1/video/create_video/`，Scope为`video.create.bind`；需要在控制台「应用详情 > 能力管理 > 能力实验室 > 代替用户发布内容到抖音」申请权限，并需要用户授权。旧平台上传页仍显示`video.create`，实施不能混用旧文档与新端点。
- 同页明确：创建后有审核过程，期间只有自己可见；返回`item_id`或`video_id`只证明接口受理，不能直接显示「公开发布成功」。文案字段`text`总字数上限1000，支持话题；`custom_cover_image_url`实际填写图片上传接口返回的`image_id`，不可按字段名称假设接受任意图片URL。可设置`download_type`及`private_status`。当前页错误码列出15分钟时长上限、同一用户同一应用每日最多75个作品；实际可用配额和账号权限仍需控制台及实测核验。
- 新版[登录与授权](https://developer.open-douyin.com/docs/resource/zh-CN/dop/develop/sdk/mobile-app/permission/overall-permission)与[获取access_token](https://developer.open-douyin.com/docs/resource/zh-CN/dop/develop/openapi/account-permission/get-access-token)已读到完整正文：OAuth2授权码模式，code仅10分钟且一次性；access_token为15天，refresh_token为30天，续期存在次数和总时长限制（页面写195天），用户撤销授权会立即失效。因此一次授权不等于永久无人介入。实现以每次返回的有效期为准，并避免并发刷新令牌。
- 获取access_token需ClientKey、ClientSecret和code；官方明确建议将access_token存储在服务端。网站回调要求预先配置完整URL（域名与path），不是任意桌面localhost地址均可用。推荐由小型服务端持有应用secret、交换/刷新令牌和完成正式发布请求，桌面不内置公共应用secret；此为基于公开接口约束的架构建议，不代表平台已经批准本产品。
- [创建移动应用和网站应用](https://developer.open-douyin.com/docs/resource/zh-CN/dop/develop/app-mgmt/create-mobile-and-web-app)明确新建应用先是测试应用，相关操作对线上不生效，开发完成需上线后才能商用。不能把测试应用或普通个人账号的网页登录成功当成发布能力可用。主体、应用类别和具体能力申请资格仍需控制台核验。
- 当前创建视频页链接的「需要用户授权」目标`/dop/ability/user-authorization/solution`只返回站点外壳，未取得有效正文；上述授权结论采用有完整正文的登录与授权和令牌页面，不根据空白页猜测要求。
- 新版[上传视频](https://developer.open-douyin.com/docs/resource/zh-CN/dop/develop/openapi/video-management/douyin/create-video/upload-video)为`POST /api/douyin/v1/video/upload_video/`，同样要求`video.create.bind`。正文明确超过50MB建议分片、超过300MB必须分片、总大小4GB内，错误码还列15分钟上限。旧页的128MB阈值不能套用到新版。官方页比例描述「16:9」与「竖版」彼此矛盾，因此计划不把该句作为画幅硬约束，实施以实际接口验证为准。
- 当前[抖音发布能力使用规范](https://developer.open-douyin.com/docs/resource/zh-CN/dop/operation-standard/platform-capabilities/useclue)明确：SDK/H5投稿导入抖音编辑/发布页后由用户手动发布；相应开放范围对正式应用、主体、应用类型和使用场景有要求（页面列企业注册资本至少50万且成立至少1年、不再向个体工商户开放等）。**该表描述SDK/H5投稿，不能直接推定能力实验室`video.create.bind`具有同样主体门槛或每次确认要求。** 自动发布计划必须单独核查本项目可获批的「代替用户发布内容到抖音」能力和授权合同；不能用H5/SDK替代全自动发布。
- 同一规范要求内容具备原创度与价值，禁止他人原创内容直接投稿、同质低质及营销导流；若违规或连续90天无投稿/分享，可回收能力。读书二创不能退化为整本书机械朗读或未经授权的内容搬运；具体作品权利需用户确认，AI生成本身不证明版权合规。
- 当前名为[查询视频发布结果](https://developer.open-douyin.com/docs/resource/zh-CN/dop/develop/openapi/video-management/douyin/search-video/video-share-result)的接口实际是`/share-id/`、Scope `aweme.forward`，用于分享SDK传入share_id后接收Webhooks，share_id有效期一小时。**它不是本次`create_video`通用审核状态查询接口，不能凭页面名称直接用于确认作品公开发布。** 本次需要另行核验与`video.create.bind`相配套的作品查询/审核回调权限。
- 官方[查询视频基础信息](https://developer.open-douyin.com/docs/resource/zh-CN/dop/develop/openapi/video-management/posting-task/video-basic-info)提供`POST /api/douyin/v1/video/video_basic_info/`，但Scope是另一个`posting.behavior`，需申请「用户投稿任务」能力及用户授权，仅能查询对应token用户上传的视频。响应示例包含`video_status`，本次读取的正文未明确各枚举语义；不能把示例数字5推断为公开成功。该页可作为状态核验的候选，是否适用于本应用及审核中的作品须在P0实测。
- 官方[Webhooks概述](https://developer.open-douyin.com/docs/resource/zh-CN/dop/develop/webhooks/summarize)要求事件对应scope、已验证回调地址及订阅，说明事件会重试并提供`Msg-Id`去重和`X-Douyin-Signature`验签。[事件列表](https://developer.open-douyin.com/docs/resource/zh-CN/dop/develop/webhooks/event-list)中的`create_video`仍是`aweme.forward`分享事件；不能将其假定为`video.create.bind`审核完成事件。实现按实际获准的事件或查询进行状态回收，回调重复、乱序和应用重启不能再次创建作品。
- 官方[图片上传](https://developer.open-douyin.com/docs/resource/zh-CN/dop/develop/openapi/video-management/douyin/create-image-text/image-upload/)提供`POST /api/douyin/v1/video/upload_image/`及`image_id`，Scope同为`video.create.bind`。当前创建视频正文可证明一个自定义封面参数，不能据此承诺网页4:3/3:4双封面可逐一映射；本地仍可生成多尺寸，平台只提交能力明确支持的封面。
- 当前创建视频请求结构未找到AIGC声明、定时发布时间或独立双封面参数；这只说明本次公开文档未证实，不能据此断言平台永远不支持。自动化上线前必须确认AI内容声明的正式提交方式、平台实际所需确认动作，以及用户选择的发布设置是否可由已批准接口表达；不能用文案中写「AI生成」代替尚未核验的平台字段。

推荐结论：本项目的无人值守发布只有一条推荐路线——先取得并验证官方`video.create.bind`等实际所需能力，再用小型服务端处理OAuth、凭据、发布与结果核验，桌面负责书籍生产任务。服务端属于该发布路线所需的最小边界，不扩张为通用云创作平台。P0若无法获得权限、无法满足平台声明或无法可靠核验发布结果，则先交付自动制作与现有半自动发布，产品状态明确为「待人工发布」，不得标成完整自动发布已完成。网页最终按钮自动点击缺少本项目可用的官方支持证据，也不符合当前仓库明确保留最终人工发布的边界，本次不建议将其作为兜底路线。

P0必须拿到的证据：实际开发者主体/应用类别和能力审批结果；目标账号完整scope及授权回调；代表性成片上传和封面采用；文案/话题/保存权限的实际映射；AI声明方式；创建受理与审核后公开状态及作品标识；网络超时后的去重核对；撤销授权/令牌到期/配额不足行为。发布测试需使用明确授权的专用账号及测试作品，研究阶段不执行。

研究局限：未验证任何真实token、控制台权限、API响应或平台审核；某些旧入口和试探性路径返回空壳页，已改用有完整正文的当前官方页面，不把空页当作能力不存在。未找到适用于本项目的官方网页自动化支持承诺，未声称平台存在全面禁止或全面支持网页自动操作的条款。

# 视觉与创作交互优化发现（2026-11-03）

- 当前工作区仅 docs/DEVELOPMENT-PLAN.md 与新方案未提交；npm包0.45.0，Node24.11.0，npm11.6.1。保留既有语义版本与真实数据。
- 书架本来已有书脊/纸页边线、投影和宋体；本轮精修已有结构。五项导航本来都是 RouterLink，可自由跳转，不新增线性向导。
- useWorkflow仅提供最近续作与运行任务；完成状态需要实际版本、草稿、文件与任务查询，不能由人工书籍状态或最近任务推断全部完成。
- 文案生成已追加历史，复制按钮已有2秒成功态；可复用一套版本与草稿逻辑，加预览对照和800ms就地反馈。
- renderCover当前返回实际字号，可在同一排版计算中增加文字命中范围；背景拖动已经完整具备。
- 视频逐句试听使用同一原声元素和原时间码，当前只播放时间窗；连续进度与分镜联动可增量加入而不改字幕/计划。
- 来源文件只关联；TXT/Markdown读入与PDF/EPUB/DOCX文件绑定为不同能力。拖放复用原添加表单及save_project事务。
- 本次未采用要求新建.ai_state的PACE路径；延续项目已有三文件计划。

# 新增实现要点

续作由任务union派生，不持久化第二状态；缓存只逐个删除cache/preview普通文件，保留junction/空目录和所有run目录，使原引擎链接保持可用。诊断在统一Provider层通过请求作用域记录，Drop记取消，真实headers计首响应，服务usage字段记用量。新增schema17模型调用表。首次编译修正了SQLite i64映射、查询迭代器生命周期；无运行数据影响。

实际清理后同计划20秒1080p试片可重新生成并播放，证明保留引擎junction的必要边界已闭环。备份快照早于成功时间写入，恢复后出现更早记录属于快照语义。OpenAI官方streaming-events页面实际读取确认include_usage末块/空choices、中断可能无usage；兼容服务未返回字段仍为空。首响应为HTTP headers而非TTFT，UI及文档已明确。

# 剩余三项分析（2026-11-03）

书架继续入口固定writing；任务状态已在audio_jobs/video_jobs/publish_tasks。备份复用SQLite快照，成功时间可存app_settings。render缓存与preview可再生，run目录可能包含已登记字幕/成片，不能整目录盲删。TextProvider返回String，优先保持现有业务协议，通过统一调用跟踪记录元数据。

# 视频 P2 实施发现（2026-11-02）

- 图片恢复只需稳定assetId：现有完整备份已包含全部图片并重映射assets路径；真实文件缺失后的恢复副本读回与计划摘要均通过，无需新表或第二套备份实现。
- 最终图片区域使用P.img(480,282,960,540)，来源HUD在y834/26px；图片无额外相机运动，预览保持原scene时钟和cue起点。严格布局0问题，图片内容三处/前后定位连续5轮逐字节相同。
- 本次静态图片/插画完整业务在真实WebView2通过，HTTP模型为本机契约夹具；不能将单张请求成功写成真实模型画质或商业权利保证。正式安装和用户作品未改写。


- 当前dev/05f14c6，版本0.36.0，schema15；P1与渲染进度更新已经提交，工作区干净。
- P2方案明确先本书静态图片，再主动视频插画；不引入新供应商或新渲染器。现有ai_tasks.kind自v4起无用途CHECK，可新增video_illustration而不迁移SQL。
- 已读取Frametell0.7.3 runtime/media.js：真实API是P.img(src,x,y,w,h,{fit,pos,...})，复用同一图片预加载；引擎遇错只console.error后继续，宿主需补上明确解码失败检查，不能靠空白占位。
- assets已有图片登记、校验读取和完整备份文件收集；图片引用只存assetId与本次来源/授权/裁切，运行路径由Rust解析，图片摘要由运行快照冻结。两种画面内容共用原Scene/确认/预览/归档。
- 本轮不采用缺少.ai_state的PACE初始化流程；延续仓库现有三份规划记录。只读定位两处不存在的猜测文件路径已改为实际枚举，无文件影响。

# P1 场景层次与画面扩展发现（2026-11-02）

- 当前 HEAD=0e86f39、版本 0.32.1，工作区干净。P0 与中文路径 Node 修复已在当前源码中；此次是现有 P1 的续作，不重新初始化工程。
- 已确认方案只给 Scene 增加 environment/framing 两个受控字段；旧物体分镜按 plain/reference 读取，继续使用 plan_json/v14，不重写旧任务。
- 6 类环境和三景别共用原绘图；24 项仅为候选，需结合真实字幕选择。保留 85 项原几何、6 动作、13 构图和每镜 3 主体上限，P2 另行确认。
- 只读核对已有《我的前半生》《人生由我》校订 SRT 和睡眠/咖啡知识样片字幕：指南针、大脑、杯子为直接缺口；选择/休息/山里/果树/自然光可用岔路、床、山峰、树、窗户表达。本批采用 8 项（共 93），其余 16 项候选暂缓；比喻不冒充史实，不宣传连续生长。
- 已阅读锁定引擎 0.7.3 的 Builder.layer/cam、engine.camAt/HUD 和 study 样式源码。cam 作用于整页主体，HUD 与共享绘制时钟独立；仅 detail+focus 采用一次 1.06 倍/3 秒轻推近，默认静止。标题、页号和进度须移至 HUD；背景只画结构，不额外放人物或结论。
- UI 复用原 scenes 编辑状态和 Select，旧记录显示默认值但不在 adopt 中改写；显式字段变化自然复用 dirty/预览失效。ui-ux-pro-max 命中 Form Labels/Input Labels，采用关联可见标签。
- 探索中的路径错误仅为只读定位（不存在的 tests.rs/preview-input.json/study GRAMMAR 与 Windows rg 通配符）；改为枚举实际路径后读取，不创建兼容文件或修改外部引擎。
- 真实 Chrome 28 镜严格布局/确定性检查 0 问题；6 环境联系表已查看。旧 P0 的 5 处物体区与新程序 plain/reference 逐字节相同；P1 试片与整片 3 处物体区相同。
- 扩大到 2493 个目录/动作/景别/环境/构图/容量组合、4 个时刻后，发现特写比例 1.34 叠加倾斜/轻推近使宫门顶部到 y249、钢笔等压到标签；三物体特写的次级标签固定148px无法容纳摇篮。新增回归复现后，按实测边界将特写主物体缩至1.2，并按次物体尺寸给标签留白；不改旧几何、参考布局或校验阈值。
- 最终2493组合0越界/碰撞，原生30秒试片/83.667秒整片5处对照一致、HUD文字不位移；6份运行源码与仓库匹配。隔离保存/失效/取消/重启/素材事务通过，原声和SRT摘要保留。仅本机夹具，不据此宣称真实模型选图、整本书节奏或跨平台已验收。

# 视频再次渲染失败排查（2026-11-02）

- 原生video_process::run将stdout/stderr合入同一个日志，失败时返回末4000字节；截图包含片段PLAN JSON而非可定位错误，需要检查原日志。
- 当前render.mjs用引擎0.7.3的--plan收集片段，再分别执行--skip-subs、--skip-video和原声合入；保留P0预览/整片同源流程。
- 正式profile只读检查仍为schema13，不等于用户视频运行所用数据库；本轮应从本书已存在的运行快照复现，不以旧profile推断任务状态。
- 准确根因：Node24.11.0的Windows原生rmSync错误地处理非ASCII路径；Frametell0.7.3在--plan退出事件中用rmSync删除out/.render.lock，导致0xC0000409原生崩溃。完整PLAN已经到达，故不是模型、字幕、FFmpeg、超时或输出截断。
- 最小对照：相同中文目录中的write/read/unlink成功，rm/rm-on-exit均退出3221226505；英文目录全通过。只换Node24.22.1后原309片段计划成功。普通“中文”短目录未触发，必须保留真实书名目录条件，不能用过浅用例代替。
- 官方修复将`std::filesystem::path(path.ToStringView())`替换为`path.ToPath()`，正确处理Windows路径编码；24.x首次包含于24.13.1，25.x于25.4.0。依据：https://github.com/nodejs/node/pull/61108 和 https://github.com/nodejs/node/commit/0ed0a30f74388d9f0a5e0715cabae5dcc1d6ea10 。
- 采用升级独立Node运行时并在现有渲染入口拒绝已知缺陷版本的单一方案；不修改书名/原路径，不monkey-patch fs，不忽略非零退出码，不放宽布局检查。PLAN是机器数据，不应继续回显到用户错误日志。
- 官方24.22.2完整重放原469字幕/72镜成功，证明修复不仅通过了短片或英文目录：22403帧、933.459002秒、音视频全量解码及首中尾播放通过，原9份数据/引擎摘要保留。版本检查提前报具体恢复步骤，但当前安装版仍需用户明确更换Node设置，未静默代改正式数据库。

# P0 导演与试片实施发现（2026-11-01）

- 当前仍为d4a9dac，只有上一轮已知5份Markdown改动。video_jobs.stage是自由文本，运行代次/取消/确认可直接复用；正式finish_video_job才登记成片，预览应独立返回确认而不触达素材事务。
- 当前整片编译用完整plan定位全局cue，preview需保留原页的绘制时长/beat局部时刻，不能因截短最后一页改变动作速度；字幕显示时轴单独平移/截窗，源plan不变。
- 现有确认页只可改标签，不能实际修改物体/动作/锚点；P0需复用这份scenes编辑状态开放受控选择，才可闭环处理质量提醒。
- UI保持现有变量/字体和输入组件；ui-ux-pro-max命中Inline Validation，用于就近可定位提醒，不采用与桌面无关的Haptic Feedback结果。
- 引擎0.7.3的render.mjs默认从首镜末尾抓取封面并替换视频第0帧；已在同一工程写入真实第0秒cover，避免后续结论提前出现。引擎页index须保持本地连续，原序号另由编译派生sourceIndex保留。
- 预览不更改plan：beat.at和drawDuration保留源镜头时钟，显示字幕和窗口平移；截断末镜不会加快动作或重编号全局cue。实际10.6—40.6秒样片已渲染成功，待完整检查与对照。
- 2026-11-02原生隔离库完成30秒预览/缺缩略图拒绝/跨书拒绝/编辑后失效/取消重试/25.266秒尾段/重启读回/83.667秒完整渲染；全程未启用ASR，初次分镜用本地契约服务，重启与渲染模型请求0。预览资产数不变，完整确认后只追加MP4/SRT两份素材。
- 同一任务的30秒试片与完整渲染工程在原声18/35/40.2秒的物体区域PNG逐字节相同，3处浏览器对照通过；这证明时钟/绘制一致，不是对整片美学或真实模型质量的保证。
- 续验发现保存修改后若下一次preview启动失败，前端可能因dirty重置重新展示旧试片。新增用例先复现后修复，保存成功即同步清空当前缓存视图的preview，后端仍按stage/签名失效。
- 最终校对发现所选试片起点标题滞后：已核对当前Reka 2.11.0的SelectItemText仅onMounted登记文字、SelectValue支持默认slot。新增用例复现后直接从现有scenes渲染已选标题，原生新版本入口/物体动作锚点/保存回读再验通过，旧完成任务JSON不变；不新增状态或重挂载兼容层。

# 视频画面美学与内容扩展发现（2026-11-01）

- 当前基线为d4a9dac；唯一visual-catalog.json实际列出85物体、13构图、6动作，每镜最多16条字幕、3个beat、标签8字、标题20字。
- video-storyboard-v2已要求具体物体、动作、字幕锚点及首尾呼应，video.rs每批24字幕附首尾、最近3镜与后续3条上下文；不能将这些能力当作待新增。
- 当前VideoBeat只有cue/object/action/label；VideoPlan没有章节、景别、外部画面素材或词级锚点字段。扩展方案需解释契约如何承载，不能只列素材网站或声称改Prompt即可支持。
- illustrations.js::draw实际主要使用三组places，13种kind还包含语义/连线/运动差异，并非13套独立布局。一个beat创建一个绘图实例；不能单靠再次输出同名object表达同一个物体的后续动作。
- 首物体的绘制时刻固定为页内-0.6秒，其余取对应cue起点；移动从at+1.3秒推进，勾/叉在at+2.7秒出现。字幕锚点不等于所有动作都在该句首精确发生，优化需区分铺底绘制与语义事件。
- 已查看既有output/video-reference/contact.png六格图：纸张/手绘/字幕风格统一，样本以孤立物体和固定标题区为主。仅作已有样本的静态观察，未在本轮播放或重新渲染，不据此断言整片节奏。
- 转写内部使用word_timestamps，但只输出句级start/end/text；当前任务未保留词级对齐。图片Provider已有静态PNG/JPEG/WebP解码，业务任务仍为封面专用，不能直接当作视频关键帧服务。
- 只读核对本地Frametell 0.7.3：core.js已有cam(keys)，engine.js已有不随镜头运动的hud；默认push为0.28秒/1.25倍，不宜直接套用舒缓读书片。当前书引标题等仍画在主体层，接镜头前需分离；不改外部引擎。
- 本轮npm run test:video实际10/10通过；Node读取目录实算85/13/6。测试不包含真实模型/新风格成片；npm提示用户级python/msvs_version未知配置，未改无关配置。
- 方案只给一条增量路线：P0导演/同源试片，P1轻场景/景别/按真实缺口扩物体，P2本书静态图片后再主动生成插画。拟议新增environment/framing字段明确当前不支持；24个候选键不重名且均不在现有85项中。
- 本轮仅设计文档；历史验收记录中的样片、模型夹具及测试数量不能当作本轮新验收结果。

# 同款手绘视频生成发现（2026-11-01）

- 已对照同一本《我的前半生》：书引437字幕/113镜/5类文字图解，参考工程454字幕/69镜/13类构图/85种物体；二者实际指向同一个Frametell 0.7.3目录，均为1080p/24fps。
- 根因是Scene只有kind/title/labels，固定render模板不具备具体物体与动作；24条批次未传前镜上下文。提高码率或只改Prompt无法突破契约。
- 参考工程的extras/visuals.js是成片实际使用的可信绘图代码（不是引擎源码），包含逐笔物体、字幕锚点、移位/倾斜/强调及字幕样式。实现将以此为唯一绘制路径，移除旧文字框绘制。
- 旧plan_json真实存在，必须可读且保留成片；旧分镜不能静默冒充新画质。沿用现有“保存字幕并重新生成分镜”入口明确升级，不自动覆盖字幕或旧成片。
- 参考完整工程每镜最多13条字幕，因此目录上限采用16（不再机械限制8条）；物体触发锚点允许在本镜任一句，第一物体仍按参考代码在翻页前预绘。所有85物体的sketch函数与原片源码SHA-256完全一致。
- 已建立唯一visual-catalog.json，Rust校验、Prompt、Node和Vue共用；新旧分镜没有并行渲染路径。旧计划只读兼容，显式清空分镜重新设计；已完成作品用原音频/SRT创建新任务，不改旧作品。
- 真实渲染暴露中文META改变引擎输出名为film-有字幕.mp4，旧调用仍读取英文名而失败。已核对0.7.3源码并按中文单一契约修正，原输入复用25/25缓存成功，没有猜文件名或额外回退。
- 同一编译计划/页面ID加载新旧绘图，5处物体区域PNG完全相同；原生实跑时落盘的render/illustrations/catalog摘要均与当前源码一致。83秒样片响度-21.9LUFS源于原输入-21.89LUFS，不擅自归一化；这项YouTube目标失败不应伪称通过。
- 续验真实界面两条升级入口都已闭环：在无ASR路径条件下复用旧SRT；模型返回不支持物体时拒绝落盘，字幕/摘要不丢；继续后只重新设计分镜。旧成功任务和原音视频摘要保持不变，无需再添加第二套升级接口或状态。

# 视频画面一致性失败排查（2026-11-01）

- 截图任务已保存437条字幕和113镜，错误来自渲染前的布局检查：同一帧差异45.21%。这条错误文案列出的随机数/时间/CSS只是引擎提示，不能直接当作根因。
- 当前书引调用 `lintPages(..., determinism: true)`；独立引擎0.7.3在每页中点比较从0和页尾跳转后的两张截图。先用真实失败输入建立回放，不跳过或放宽检查。
- Rust每次运行写入内嵌 `render.mjs`，缓存键包括模板脚本源码；修复生效需要重新构建应用。已保存plan是重试输入，不需要新建模型任务。
- 已找到截图对应《我的前半生》的原运行目录与布局结果：437条字幕、113镜、933.37秒，p0001中点5.3秒差异45.21%。只读打开原缓存回放，首次通过、下一次复现同页12.55%差异；两张截图分别为空纸和完整书籍图解。属于可复现的间歇性错误，尚不能据此断言绘图逻辑或截图时序谁是根因。
- 合成5镜及113镜计划均通过，不能用短片或镜头数量独自证明修复。实际安装缓存脚本与当前源码摘要不同，后续须同时验证当前生成代码和原输入。
- 当前源码用原计划重新生成后同样复现。诊断记录两次目标帧DOM摘要完全相同、fonts=loaded、活动动画0、ENGINE.errors为空；去掉will-change仍复现原样45.21%，排除图层提示作为修复方案。
- 根因是引擎 `runtime/main.js` 的 `__ready` 在建DOM/加载字体/设置时间0后立即完成，不等待初始绘制；`tools/checks.mjs`直接截图，首次截图可能还是空白底色/纸张。在就绪后等待两次requestAnimationFrame，原113镜加载条件下连续20次通过。Playwright 1.61.1自身稳定截图路径也使用double-rAF，普通screenshot只等字体。
- 最小修复扩展生成页面的同一个 `window.__ready`，放在boot脚本之后，保留引擎返回的时长与拒绝原因；所有openStage调用（检查/编码）共享它。没有复制校验算法、关闭校验、固定睡眠、重试或改第三方源码。上游main当前0.7.4的同段检查与CSS仍相同，不盲目升级。
- 原933.37秒计划已完整通过113镜检查及渲染；成片精确933.375秒/22,401帧，原声AAC音轨933.371995秒。全音视频解码、首/中/末定位与播放及14张抽样画面检查通过，不能据此推断字幕逐字准确或所有长片都已验收。
- 用户的版本更新提交e7f0e12只改5个版本声明/锁文件，明确确认后保留0.30.0续验。当前代码构建通过，但未更新正在运行的正式安装版；独立验证成片未写入正式任务。

# 视频制作进度续作发现（2026-11-01）

- 上次聊天已接入转写 progress.json、Rust 当前运行快照和 VideoJobProgress，但渲染仅报告阶段，追加的制作进度需求尚未实施；前次检查不能替代本轮复验。
- 独立 Frametell 0.7.3 的 render.mjs 提供 --plan 输出每个片段的精确帧区间/缓存文件，以及 --skip-video/--skip-subs 阶段入口。render-chunk.mjs 每600帧输出一次，但片段最多6秒/144帧，不能把该日志当作持续帧进度。
- 引擎仅在整个 worker 分组完成后才将 .part.mp4 改名；只数缓存文件会长时间不变。以 FFmpeg 已写完整 moov 的片段统计完成帧数，仍由原引擎绘制、合成和校验，不复制渲染算法、不修改外部源码。
- 取消后缓存目录可能留有已封口、未正式提交的 .part.mp4；新增回归复现其让新运行误报96/96而非24/96。临时片段只统计本轮启动后写入的文件，正式缓存照常复用，避免重试初始进度虚高后回退。
- 视觉沿用现有墨绿/白底及字体，以真实百分比环、五步流程和内容预览为重点。ui-ux-pro-max 的 long task progress feedback 查询命中 Progress Indicators；未知阶段不用伪百分比，支持减少动态效果。
- 第二次原生复验遇到确定的路径问题：重复书名使字体链接路径从258增长到262字符，Chrome报ERR_FILE_NOT_FOUND，但文件实际存在；同一浏览器对258字符别名和140字符真实路径均加载成功。生成HTML现在始终引用引擎/依赖的规范file URL，不经深层junction访问字体，不改变外部引擎或业务路径。
- 引擎openStage在返回browser前可能因字体失败抛错，原main只设置exitCode，遗留Playwright连接使任务仍running。新增子进程回归先实测4秒超时，现失败边界明确退出1，由Rust现有Job Object回收进程树并写失败状态，不再等6小时渲染超时。

# 应用内 MG 视频实施发现（2026-11-01）

- 真实Windows调用中，数据库canonical路径直接传Node主脚本触发EISDIR/lstat D:；外部调用统一复用已有shell_path，DB原值不动。带扩展路径的Windows进程树取消回归已通过。
- faster-whisper1.2.1与自动解析出的PyAV19.0.2不兼容(metadata_errors已移除)；核对16.1.0源码后锁定依赖，真实20秒ASR通过。模型缺tokenizer时SDK会调用from_pretrained，因此先校验完整模型文件，拒绝隐式下载。
- 本地small模型可运行，但实测存在书名/人名误识别，加入书名作者热词也不保证逐字准确；必须保留试听校对，不将生成成功当作人工审核通过。
- Reka2.11.0 SelectItemText仅onMounted登记textContent，历史任务默认label不会随status更新；真实截图与回归均复现。使用SelectValue已有slot直接绑定单一jobLabel，不重新挂载或增加重复状态。
- 切书时旧轮询可能迟到覆盖新书任务；回归复现后按发起时projectId接收响应。五步流程在窄窗口隐藏次要说明/编号，四档真实WebView视口无溢出。
- 分镜配置按单轮读取，JSON必须连续覆盖批次且只含固定图解类型；渲染脚本不执行模型代码。原声节点没有引擎TTS/响度处理，最终由FFmpeg合入原音频。

# 应用内 MG 视频接入评估（2026-11-01）

- 本仓库初始工作区干净；whiteboard-teacher-video 已有未跟踪的计划、素材等文件，本轮仅只读检查，不改动该项目。
- 书引已经登记 audio/video/subtitle 素材，但音频页和发布页仍明确要求剪映制片；没有应用内视频播放器与音频转写 Provider。当前 TextProvider 只请求 chat/completions，不能拿音频指导冒充逐字稿。
- 外层工具许可 MIT；实际本地 @frametell/engine 0.7.3 声明 AGPL-3.0-only。2026-11-01 读取上游 main 的 package.json 为 0.7.4，仍为 AGPL-3.0-only；本轮不更新引擎。独立进程不等于自动免除许可义务。
- 本地 runtime/timing.js 默认依照配音或字数排时；D:/films/视频风格实验/08-人生由我 已存在原声绝对时间轴适配和逐页动画工程。既有 SRT 是输入，不代表当前工具具有音频识别能力；既有影片也不代表自动生成新书分镜已实现。
- 官方 faster-whisper 当前 README 与 transcribe.py 确认 CPU/int8、word_timestamps、download_root/local_files_only；只核对 API，没有安装模型或进行音频识别。模型/缓存应使用用户选定的 D 盘位置，不隐式下载到默认 C 盘缓存。
- 真实预览需要按已登记素材限制本地媒体读取和支持 Range；当前 Tauri CSP 未开放媒体协议，不能仅增加 video 标签或把长视频转 base64。

# 音频页面间距与路径展示（2026-11-01）

- `.editor-section .muted` 对所有嵌套说明设置 `margin-top: -3px`，来源与格式没有内部间距结构。修复直接收敛该规则，表单使用明确 gap/padding，不堆叠负边距覆盖。
- Rust `database/assets.rs` 保存 canonicalize 后的路径，Vue 直接插值。`paths::shell_path` 只供原生 shell 使用；不修改数据库/IPC，前端展示共用一个纯格式化函数。
- 设计沿用背景 #f5f6f4、卡片 #ffffff、主色 #285b46、正文 #202d28、说明 #56635b、边界 #dfe5df 和本机字体。左右来源/格式，窄窗纵向；标签/控件 8px，说明 12px，区块 24px，字段区内边距 20px。保留原生交互，不新增装饰/动效。
- 项目 ui-ux-pro-max 的 form spacing helper text 命中可换行、内容自适应和触点间距。Vue 两次查询未命中，不使用虚构推荐；以当前组件源码为准。通知组件为 OperationFeedback.vue，无 services/notifications.ts。
- 样式主体位于 components 层，不能通过提高选择器权重覆盖 utilities 层的控件类；最终在 SelectTrigger 使用现有类合并设置白底和文本省略，删除无效覆盖。实测标签8px、说明12px，四档宽度无横向溢出，375px标题/标记正确分行。

# 真实音频续验发现（2026-10-31）

- 最终原生重定向修复已在真实服务下载成功，文件32,174,918字节、16:39、SHA-256与库一致，原任务仅一次生成、唯一音频素材。最终无调试端口release正常重启仍读回10本书和已归档音频；原文案/封面/草稿/模型/发布记录及冻结输入保持。没有将调试端口写入产品配置。
- 原生下载还需跟踪真实 HTTP 重定向：日志确认原始 CDN 导航 exact-match=true，随后 lh3.google.com 导航 same=false，被旧白名单拒绝。修复以 WebView2 同一 NavigationId + IsRedirected 证明实际链路，只放行已请求下载的服务器跳转；不添加任意 CDN 白名单。
- Playwright CDP 默认连接会设置下载行为；已查当前本机源码并采用 noDefaults=true 保持原生行为。恢复默认行为不能消除上述服务器重定向拒绝，两个问题分别记录。
- 真实成品下载失败根因已验证：当前菜单通过 window.open 打开 HTTPS 媒体地址，不产生原代码等待的锚点点击。最小修复在既有捕获函数内临时拦截仅下载动作触发的 HTTPS/blob 地址，finally 恢复，沿用原生精确 URL 下载与素材事务，不新增第二条下载流程。
- 成品已真实生成，16:39（目标 15 分钟，网站不保证精确时长）。原任务 waiting_user/downloading 保留，整个过程仅生成一次；完整指导、格式和语言均有提交前实测回读。
- 用户确认恢复后，源快照 13 张表全部逐行恢复并校验；修复版读回 10 本书，文件路径与运行连接当前一致。旧读写视图差异原因仍未确认，不将本次恢复冒充原因修复。
- 原 Notebook 登录和 EPUB 保留，当前网站真实配置支持本轮修复：1659 字指导提交前摘要一致，深入探究/简体中文/默认时长，生成按钮仅点击一次；网站与应用都识别 generating。尚待成品下载与重启验证。
- 用户关闭后，调试版和未修改的原安装重启都读取到同一个空书架；直接路径库已为 v13，书籍/素材/任务均 0。真实旧句柄快照保有 10 本书、16 份文案、37 个素材引用、5 个音频任务。尚未恢复，不能把新启动的空实例当作原任务环境。
- 原书 EPUB 存在，快照完整性和外键通过；当前空库已另存一致性备份，恢复前预览与哈希已落盘。现有素材引用 37 个中 33 个路径存在，缺失引用保留。未验证旧文件视图与新打开路径差异的根因。
- 最近会话已由用户协助新建《人生由我》，完成真实模型文案/指导、笔记本创建及 EPUB 上传；报错为 `audio_prompt_missing`。开始本轮时工作区干净，HEAD=37a7f5c / 0.24.2。
- 本轮重新读取原窗口可访问性树：原笔记本地址末段 `2b83b06f-41b0-4dff-849e-b6dd16e349df`，自定义音频弹窗仍打开，来源 1 个；指导输入框名称为“AI 主持人在本集节目中应着重于哪些方面？”，格式为“深入探究 / 摘要”，语言“选择语言 / 中文（简体）”，时长“短 / 默认”。尚未点击生成。
- 实际主窗口显示数据库为 `C:/Users/haozi/AppData/Local/com.shuyin.desktop/shuyin.sqlite3`；继续沿用原任务前需备份。旧规划文件停在 10 月 29 日，不能据其认定最新任务已完成。
- 同路径直接读取为 schema7，而微软签名 Handle 核对的运行进程 SQLite 句柄只读快照为 schema13；后者完整性/外键通过，原任务 `waiting_user/configuring_audio/run_id=1/submitted_at=NULL`。只保留快照，不修改或恢复数据库；读取视图差异根因未确认。
- 回归先复现相同 `audio_prompt_missing`，最终 43 项页面/打包测试通过。统一原生 label 与 aria-labelledby 解析后删除重复 label 查询；格式、指导和语言触发器限定弹窗，已选中文的组合框不重复展开。
- 桌面工具无法可靠操作原生“选择目录”弹窗：索引错误 `element 133 is not available in cached app state for shuyin-ai.exe`；坐标/键盘动作也未确认成功。原应用与笔记本保留，修复调试版未启动，需用户先取消目录对话框再关闭旧应用。

# 应用内自动音频实施发现（2026-10-29）

- 开始按钮和 Rust 启动命令双重要求扩展连接；初始 stage=awaiting_extension，是替换的直接调用链。
- 复用 v13 音频任务及既有抖音宿主能力；不将抖音的已验证登录推断为 Google 支持。Google 嵌入式 OAuth 限制仍是实际服务验收风险。
- 已读取规划文件和五份项目文档；session-catchup 无额外报告。UI 技能查询中仅“逐步进度提示”适用，保留现有样式和可操作的暂停/继续反馈。
- 正式应用进程 142132 位于 D:/书引 AI/shuyin-ai.exe；本轮不替换或停止该实例，桌面验证须隔离。
- 已收敛为专用 Tauri 窗口、Rust 调度、共享网页宿主、原生文件选择/下载一条执行链；数据库仍 v13。
- 原生联调定位到 JS/Rust 观察字段命名、指导误选标题、同名窗口重建、归档 worker 竞争及下载导航代次问题；按真实调用链修复并添加相应回归。
- WebView2 文件输入和下载路径不能沿用 Windows 扩展路径前缀，统一复用 paths::shell_path 后，本地中文文件上传和字节一致的音频落盘通过。
- blob 与真实本机 HTTPS 下载均完成文本/文件两种来源的原生闭环，每任务生成一次、音频素材一个。拦截返回 HTTPS 文件的夹具本身未触发下载，改真实本机 HTTPS 响应后通过，未为此增加产品分支。
- 证据只覆盖明确标注的模拟网站，没有真实 Google 登录或生成结果。缓存保留但自动清理未实现；NSIS 构建与正式安装升级验收分开。

# NotebookLM 音频自动化规划发现（2026-10-27，历史）

## 续验发现（2026-10-28）

- P0 仍不能由本地夹具替代：需用户选定试验书籍文件和授权一次真实生成；当前浏览器检查连接未能初始化，未导航至 Notebook 页面。
- `docs/DEVELOPMENT-PLAN.md` 与实施计划引用 `docs/AUDIO-AUTOMATION-VERIFICATION.md`，但该文件原先不存在；本轮按实际证据创建，明确区分本地测试和真实账号验收。
- `npm run test:queue` 可重复失败于缺失前置报告的重新执行用例；队列脚本及其测试均非本轮改动，音频功能的其他前端、Rust、扩展和 helper 检查通过。
- 标准 Vite 清理 `dist` 与 Cargo 打开现有构建锁在受限执行中被拒绝；隔离输出构建和使用项目本机工具链重试分别通过。未删除或覆盖现有输出以绕过错误。

## 实施验证发现（2026-10-28）

- 当前网站品牌为 Gemini Notebook，入口重定向到 `notebook.google.com`；扩展必须同时保留旧域名兼容，不能继续只匹配 `notebooklm.google.com`。
- Native Messaging helper 必须使用 GUI subsystem 仍保留继承 stdio；短连接轮询避免卸载时长期占用 helper，开发/正式 host 名需分离。
- 扩展协议中出现过 source_chunk 嵌套、generation_complete 强制 URL、download_pending 阶段名和 verify_remote 登录短路四处跨层不一致，均已由联合测试和最小修复收敛为唯一扁平契约。
- 归档事务失败后继续会更换 runId；只有同 jobId 且处于 verify_remote/downloading/importing 才能继承原精确 downloadId，否则可能重复下载或丢失恢复路径。
- Chrome/Google 页面的真实 DOM、长提示词输入、音频生成和下载 URL 形态仍未在登录账号下验证；本地模拟和语义控件测试只能证明代码路径与防御行为。

- 用户希望从现有音频提示词进入应用内网站，人工 Google 登录后自动新建笔记本、添加书籍来源、填写指导、生成、长时间等待并下载归档；本次仅编写计划。
- 当前音频入口仍由 external::open_url 启动系统浏览器；现有抖音宿主已有独立 WebView2、页面脚本和受限文件上传，可复用底层经验，但 Google 登录需要独立验证。
- 当前产品规则明确音频网站操作尚未实现；本次需求是下一阶段规划，不把旧规则当作必须再次审批的理由。
- 开始时已有四份文档和六份 Rust 文件的未提交封面改动，保持原样。记忆索引无本项目相关命中。
- Google 官方 OAuth 政策的 Use secure browsers 明确禁止开发者可控的嵌入式 user-agent 承载 Google OAuth；不能因为 Tauri 可以打开网页就承诺 Google 登录成功。来源：https://developers.google.com/identity/protocols/oauth2/policies 。
- 用户随后确认接受独立浏览器窗口，优先完成自动化闭环。推荐收敛为现有 Tauri + 普通 Chrome + 专用小扩展 + Native Messaging；登录由浏览器与 Google 处理，程序不导出 Cookie、不采用私有 RPC。
- 已读官方个人版音频帮助与企业 API：个人网页支持来源、自定义指导、后台生成和下载；深入探索为双主持人，简报才是短时单人，精确分钟数和甲子单人不能由提示词保证。Play Books 来源可能限制下载。企业音频 API 为独立 Cloud/IAM 路线，不把个人 Pro 当作企业 API 权限。
- Chrome 官方接口提供页面 DOM、Native Messaging、downloads 与 alarms；扩展 worker 会休眠，不能用页面 setInterval 或一次长等待承载生成。downloads 文件名只能相对下载目录，需下载完成后由宿主复制到项目目录再事务登记。
- 当前 schema 为 v12；ai_tasks 有120秒文本请求及中断处理，网站音频需一张独立且持久化的任务表；assets 和发布 audio_asset_id 继续复用。新库版本只在实施时追加迁移，本轮不执行。
- 单一路线已写入 docs/NOTEBOOKLM-AUDIO-PLAN.md。P0要求证明File/DataTransfer原书上传、长指导完整回读和真实下载URL可用；不假设网页公开artifact ID，每任务独占笔记本，身份不明时暂停。
- 长等待不使用固定总时限误判失败；明确仍生成中就持续低频观察。应用完全退出期间不承诺本地下载，重启先核对原远端任务；取消本地跟进不会取消Google生成。
- 官方企业音频文档本次只确认create/delete与回网页加载下载，不把其“验证”章节误写为已确认GET接口。个人帮助的时长档位标English Only，中文控件可用性纳入P0。

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

## UI 与页面交互改造（2026-10-28）

- 当前工作区最初无未提交代码改动；项目为 Vue 3 + TypeScript + Tailwind 4 + Reka/shadcn-vue + Tauri 2。
- 现有 UI 已于 2026-10-25 接入 shadcn-vue 页面组件；应在此基础上收敛布局和交互，不新增第二套组件库。
- `src/App.vue` 侧栏、顶栏和页面主区共用壳层；`src/style.css` 同时含主题 token、业务布局和历史语义类，需先检查真实页面再决定局部整理方式。
- 用户授权仅涉及视觉与交互体验；不触及 schema、AI Provider、发布/音频业务流程。
- 项目级已安装 `frontend-design` 与 `ui-ux-pro-max`；环境原有 `web-design-guidelines` 可用于实施后审查。
- 实际浏览器证明普通 `#main-content` 锚点会被 hash 路由解析并退回书架；通过阻止默认跳转、聚焦 `main` 修正，测试核对路由不变。
- 深色侧栏中的通用链接色覆盖了底部导航继承色；统一底部链接显式颜色后实测对比度为10.17:1。
- 最终5个主要页面在1440/1024/390px均无横向溢出；浏览器隔离夹具不能替代Tauri桌面实机与真实平台验收。
## 视频制作设计稿核对（2026-11-05）

- 参考文件 `D:/谷歌浏览器下载/工作账号下载/ai (1).html`，视频区为 `#view-videostudio`，按原声输入、环境配置、任务成片、字幕校对、观看提醒、分镜精修排列。
- 原稿使用深蓝黑底、紫色模块强调、青绿成功/主动作与冰青校对色；输入为两列，字幕来源独占一行，生成按钮通栏。任务状态有运行/失败/成功三套外观，计数与进度为原型模拟值，不能照搬成真实业务。
- 当前所有业务入口已在 VideoPage / VideoJobProgress / VideoSubtitleEditor / VideoSceneEditor 内；沿用同一编辑草稿、任务轮询和AudioPlayer，仅调整结构/样式。
- 已提交的前序改动为 `1b59a1a`，本轮开始时工作区干净。既有默认主题及本地字体可直接复用；共享导航已有视频步骤，但App侧栏/顶栏尚无视频快捷入口。
