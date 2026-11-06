# 书引 AI 数据库设计文档

## 发布设置（2026-11-06，v18）

`0018_publish_settings.sql` 为 `publish_tasks` 增加 `allow_download INTEGER NOT NULL DEFAULT 1 CHECK (allow_download IN (0,1))` 和可空的 `scheduled_at TEXT`。旧任务默认允许保存、立即发布，原文案、素材、合集、状态、时间戳和发布记录不回填或重写。保存两个字段与原任务更新使用同一事务。

`scheduled_at=NULL` 表示立即发布；非空值只能为北京时间 `YYYY-MM-DDTHH:mm:00+08:00`，由 Rust 在保存前校验真实日期与未来时间。导出与上传快照再次校验未来时间，过期不再允许上传或导出；就绪任务仍可在作品实际发布后记录成功，已发布任务可展示历史时间。数据库与完整资料备份沿用现有机制，旧备份先升级到 v18。已验证 v17 升级、旧备份恢复、重开、快照和任务包兼容，见[发布设置验收](DOUYIN-SETTINGS-VERIFICATION.md)。

## 本机主题偏好（2026-11-05，仍为v17）

主题属于设备显示偏好，保存为当前应用 WebView 的 `localStorage.shuyin.theme`，值仅为 `light` / `dark`；不是业务数据，不写入 SQLite、编辑草稿或素材配置，不随数据库/完整资料备份恢复。清除应用 WebView 数据后回到默认深色，可再次选择。无表、列、索引和SQL迁移，SQLite继续作为结构化业务数据权威来源。见[浅色风格验收](LIGHT-THEME-VERIFICATION.md)。

## 第二版页面与选择导出（2026-11-05，仍为v17）

无数据库结构、迁移或数据契约变更。封面批量勾选仅存在于页面，不写配置或草稿；选中尺寸调用原封面版本接口，未选尺寸不新增版本。所有尺寸的编辑仍由已有草稿保留，保存全部配置仍为各尺寸追加原格式版本。模型、发布任务、备份及调用诊断继续读取原表；本次浏览器使用隔离 IPC 示例数据，未操作正式数据库，见[第二版验收](TECH-STYLE-V2-VERIFICATION.md)。

## 科技主题页面升级（2026-11-05，仍为v17）

无表、列、索引或SQL迁移，不新增主题、配色、书架完成度或播放器持久化状态。目录/创作提示/版本入口复用原设置和查询；历史入口复用原historyOpen，文案与换行继续保存到已有版本和草稿。配色不改封面配置、素材、字幕、视频计划或发布快照。Windows验证仅使用独立identifier的SQLite与D盘测试工作区，不以隔离数据代替正式库验收。见[本轮验收](TECH-STYLE-UPGRADE-VERIFICATION.md)。

## 原声试听界面（2026-11-04，仍为v17）

本轮无表、列、索引或SQL迁移。音频来源继续读取audio_jobs.asset_id和本书assets；成功任务与列表通过同一素材ID避免重复播放器。进度、音量、倍速、配色和波形包络只在界面或受限内存缓存保留，不写app_settings、编辑草稿或素材表。试听、寻迹与波形解码不更改源文件、字幕、计划、音频归档状态或历史。原SQLite备份和完整恢复规则不变，验证见[原声播放器验收](AUDIO-PLAYER-VERIFICATION.md)。

## 文案发布标签（2026-11-04，仍为v17）

`writing_versions.content_json`的publishing对象新增 `tags` 字符串（最多500字），例如 `"#读书 #我的前半生 #身份认同"`。新生成必填合法标签；旧对象缺字段时默认读为空，不UPDATE、回填或重写历史JSON。人工编辑允许空标签，新保存仍追加版本。`editor_drafts.content_json`沿用完整编辑对象保存，标签不单建表或重复状态。

发布来源选择同时带入标签，之后以既有 `publish_tasks.tags`保存实际可编辑文字快照；来源版本不被修改。任务包 `tags.txt`与manifest仍读取任务快照。数据库及完整备份自动保留上述JSON和现有列，无SQL迁移、表/索引/权限变化，`user_version=17`不变。旧程序不具备新增字段契约，回退应使用升级前备份。

## P3预览不持久化（2026-11-04，仍为v17）

生成中的文字快照只在任务Channel和当前页面保留，不写`editor_drafts`、`writing_versions`或`ai_tasks.preview_json`。当前稿的正常编辑继续由原自动草稿保存；全量结果完整校验后追加原历史，按项完整候选仍保存在原任务输出，明确采用才新增版本。无表、列、索引或迁移变化。取消/断流/结构错误及实际IPC/SQLite证据见[P3验收](P3-STREAMING-PREVIEW-VERIFICATION.md)。


## 视觉与创作交互优化（2026-11-04，仍为v17）

本轮无表、列、索引或SQL迁移。侧栏提示显隐使用`app_settings.sidebar_prompt_visible`，值为JSON布尔字符串，未设置默认为true；读取失败提示，写入失败保留原显示状态。原SQLite快照与完整备份自然包含此设置。

导航成果状态是对原书籍、editor_drafts、writing_versions、cover_versions、video_jobs、audio_jobs、assets及publish_tasks的查询结果，不新增完成度表或手动状态。文案历史只追加；预览不写库，字段取用进入原草稿。封面行内文字写原三尺寸配置，视频播放位置和正在播放的字幕/分镜只在界面保留。

发布前一次明确核对继续保存原title_confirmed/covers_confirmed字段。素材和标题变化撤销对应核对，任务包与最终人工发布记录沿用现有校验与事务。拖入文件复用书籍与来源原子保存；另存成片只写用户选择的新文件，不重写素材路径或历史。验证见[本轮验收](UI-UX-REFINEMENT-VERIFICATION.md)。

## 抖音账号昵称预填（2026-11-03，仍为v17）

复用 `app_settings` 的字符串键 `douyin_account_name`，仅保存最近一次人工确认的昵称，去除首尾空白，长度限定1–200字符。未设置返回空字符串；无效输入或写入失败不覆盖已保存值。该昵称随原有 SQLite 快照和完整备份保留，不保存账号 ID、头像指纹、浏览器会话或确认权限。

官方页昵称优先于保存的提示值，提示可修改；读取昵称不会自动确认账号或启用跨重启默认合集。本次不增加表、列、索引或迁移，数据库仍为v17。重开数据库和无效编辑回归见 `DOUYIN-FLOW-VERIFICATION.md`。

## 模型调用诊断与备份时间（2026-11-03，v17）

`0017_model_calls.sql`新增请求记录表model_calls：id主键；project_id可空外键，删除书籍时SET NULL；task_id为可空任务标识；model、stage、status、started_at必填；elapsed_ms、first_response_ms、input_tokens、output_tokens、total_tokens、error_code可空。status限定running/succeeded/failed/cancelled/interrupted；按started_at倒序建立索引，设置页读取最近200条。表不保存提示词、响应正文、密钥或原始供应商错误。书籍删除后保留请求元数据，未增加自动清理历史的规则。

每次真实Provider请求各写一条记录，长字幕/分镜的不同批次分别记录。首响应为HTTP响应头到达耗时，总耗时为Provider请求处理完成耗时；字段未知即NULL。响应完成状态只说明Provider返回有效文本/图片，不表示业务输出校验、正式保存或人工采用成功。启动和备份恢复把未完成running置interrupted，取消沿用请求Future生命周期，不把未知用量补成零。

最近一次用户备份成功后向既有app_settings写`last_successful_backup`，JSON为`{kind: "database" | "full", path, completedAt}`。只有备份完成才更新，失败保留旧值；完整备份path为manifest路径。记录随现有SQLite备份/恢复保存，但本次备份快照生成在本次成功时间写入之前，恢复它将回到快照中更早的备份记录。恢复前安全快照不计作新的用户备份。

续作目标只查询已有任务、草稿和版本，不新增表；缓存预览令牌和占用不入库。v16→v17新增表，不重写原任务、版本、素材或草稿。旧备份在隔离快照中按现有迁移列表升级，原备份不改写；低于v17的程序不能打开已升级数据库，回退需使用升级前备份。迁移、恢复、中断及实际重启证据见[剩余三项验收](CREATOR-COMPLETION-VERIFICATION.md)。

## 单镜重新生成（2026-11-03，仍为v16）

无表、列或SQL迁移。复用ai_tasks自由TEXT用途`video_scene_revision`，prompt_version为video-scene-revision-v1，input_json记录`{input:{jobId,runId,providerId,sceneIndex,subtitles,scenes,instruction},source}`；source是正式plan序列化后的SHA256字节数组。状态沿用pending/running/succeeded/failed/cancelled，模型选择与错误记录沿用现有字段。

start/prepare/finish均不写video_jobs.plan_json或editor_draft_json。准备与返回结果时检查源任务可编辑、run_id及正式计划摘要；取消状态在相同SQLite连接锁内检查，迟到候选不能成功返回。成功仅标记模型调用结束，不写writing_versions、素材或候选历史。候选由页面先对照再采用，采用只变更当前编辑的scenes[index]并进入原视频自动草稿；正式保存仍由save_video_plan校验写入。未采用候选不恢复；已采用草稿随原备份/恢复流程保留。重启时pending/running继续按既有规则标为中断。

## 视频运行环境检查（2026-11-03，仍为v16）

无表、列或迁移变化。检查读取原工作目录，接收页面当前VideoSettings并返回临时结果，不写app_settings或video_jobs。只有用户点击原“保存运行环境”才更新原video_runtime设置。识别路径、逐项结果与探测版本不持久化，避免把旧检查结果当成当前可用性。

## 视频编辑草稿与原文快照（2026-11-03，v16）

`0016_video_editor.sql`为video_jobs增加两个可空TEXT列：editor_draft_json保存`{basedOn,content:{subtitles,scenes,position},updatedAt}`，original_cues_json保存首次转写/导入的原始Cue数组。原有任务、plan_json、素材、时间戳及v9文案/封面草稿不改写；无需新表或新外键。草稿与任务同行，删除任务/书籍自然删除其草稿。

草稿basedOn须等于当前run_id且任务可编辑；内容允许不完整，大小上限4MiB，updatedAt由SQLite生成。暂存不改任务状态、正式plan_json或正式更新时间。正式编辑成功与清除旧草稿同一UPDATE，校验失败保留两者；新一轮认领清除上一代草稿并递增run_id。原文快照以COALESCE只写一次，且要求当前运行代次；原声时间码保持不变。

重启与SQLite快照备份/恢复保留两列，旧备份在隔离快照中自动迁移；v15升级与重复打开不覆盖旧任务。旧任务原文仍通过已登记SRT读取，缺失时提示；新任务的原文快照不依赖缓存文件。低于v16的程序不能打开升级后的数据库，回退应使用升级前备份。实际命令与覆盖范围见[本次验收](VIDEO-DRAFTS-VERIFICATION.md)。

## 字幕校对进度与参考（2026-11-02，仍为v15）

无表、列或SQL迁移。`video_jobs.plan_json`增加可省略的`subtitleReview: {promptVersion, completed, reference: {book, author, opening, ending}}`，记录已校对的连续字幕条数与开始校对时的参考快照。每个合法批次的cue文字、completed与stage=proofreading通过现有带run_id/status条件的UPDATE一起写入。字幕登记按是否还有未校对条目进入proofreading或storyboard；原始与校对SRT分别登记为本书素材，不覆盖源文件。

校对参考只读取本书对应音频任务的冻结prompt，没有关联记录时读取本书最新publishing.audioPrompt；不增加音频任务关联字段。取消/失败/正常重启/恢复保留已校对内容与游标，旧运行无法写回。继续未完成任务不重新取参考，已完成校对不重复请求。人工修改字幕或清空分镜重建时移除subtitleReview；只保存画面修改则保留。

旧plan缺字段按None读取，不自动重写或宣称已校对；旧任务已有分镜时继续保留，主动重建才校对。完整备份沿用原始/校对SRT素材及SQLite快照，不依赖运行缓存。新程序可读旧记录，旧程序严格JSON读取不保证读懂新字段，回退须使用升级前备份。

## 分镜重试的模型绑定（2026-11-02，仍为v15）

无表、列或迁移变化。继续准备或重新生成分镜时，校验当前选择是已启用的文本模型，再在同一条UPDATE中写入既有provider_id、running状态及递增run_id；模型校验或任务认领失败时保留原绑定和任务数据。继续准备不改plan_json，已保存字幕和分镜可复用；试片与完整渲染不改绑模型。

## 视频图片引用与插画用途（2026-11-02，仍为v15）

无表、列或SQL迁移。`video_jobs.plan_json.scenes[].image`保存本书图片assetId、本次source/rights、可省略的creator、contain/cover、0—1的x/y和宿主计算的sha256；有image时beats为空。没有image的旧记录不重写，未知字段和非法引用仍被拒绝；历史成片不改写，制作新版继续新建任务。

保存校对先完整校验本书图片，全部成功后才更新原plan_json；失败保留旧稿。试片签名/最终归档核对图片摘要，保存仍撤销旧试片资格。运行中移除或重新定位图片被拒绝；非运行任务的缺失素材记录保留，须重新定位/选用并保存。完整备份沿用assets文件收集和路径重映射，plan只引用稳定ID，来源/授权/裁切/摘要随数据库保留；预览缓存不备份。

`ai_tasks.kind`自v4起为无枚举约束TEXT，本轮新增用途`video_illustration`和Prompt版本`video-illustration-v1`，无需重建任务表。成功图片沿用asset_id引用，生成文件位于本书video/illustrations，不写cover_versions，也不自动更新video_jobs。失败、取消、重启与备份恢复沿用现有任务状态规则；原封面和历史任务保留。旧程序严格JSON读取不能保证读懂新图片字段，回退应保留升级前备份。测试与完整备份证据见 [P2验收](VIDEO-IMAGES-VERIFICATION.md)。

## 文本模型推理强度（2026-11-02，v15）

新增迁移 `0015_reasoning_effort.sql`，`model_providers.reasoning_effort` 为可空TEXT：NULL表示服务默认，仅文本模型可存none/minimal/low/medium/high/xhigh/max。SQL约束拒绝非法值及图片模型的非空值；保存层将图片用途的推理强度清空。

原v14配置新增列后均为NULL，模型、凭据引用、默认/启用标记及原时间戳保留。迁移及版本更新复用现有事务，失败保留v14；旧备份恢复在快照内升级，源备份不改写。v15备份与重启保留显式值，API Key仍仅存系统凭据库。低于v15的应用不能打开升级后的库，回退需使用升级前备份。

## 场景与景别 JSON 扩展（2026-11-02，仍为v14）

`video_jobs.plan_json.scenes[]`新增`environment`与`framing`，值来自唯一绘图目录；不增加表、列、迁移、素材用途或第二份状态。缺字段的旧物体计划读取为plain/reference，不执行UPDATE；显式空值/非法枚举和任意镜头字段不能保存或渲染。旧文字图解仍需原来的明确升级入口，已完成记录不能编辑。

只有用户保存校对或新生成分镜时才写入完整新计划，保存后撤销旧试片资格。数据库备份/恢复原样保留新字段、物体键、音频摘要及原字幕，不依赖预览缓存。回归覆盖旧JSON读取不变、新选择保存回读、备份恢复和失败保稿。新程序可读旧记录；旧程序的严格反序列化不保证读取新字段，回退须使用升级前备份，不能静默丢字段。证据见 [P1验收](VIDEO-SCENES-VERIFICATION.md)。

## 派生试片生命周期（2026-11-02，仍为v14）

无表/列或SQL迁移变更，`plan_json`形状也不变。同一video_jobs通过stage=previewing执行试片，run_id递增，状态仍使用running/awaiting_confirmation/failed/cancelled。预览成功返回确认；`finish_video_job`仅接受validating阶段，合法MP4也不能在预览阶段登记为正式成片。

保存字幕/分镜回到storyboard阶段，撤销旧试片展示资格；失败、取消和重启仍按原代次隔离，不覆盖旧音视频。预览清单/MP4/PNG保存在项目的当前运行缓存目录，不登记assets、不写业务表，运行结果的计划+源码签名和文件完整性决定是否可用。SQLite备份保留计划和任务；预览缓存不备份，恢复后缺失即重新试片。完整确认后仍在一个事务追加正式MP4/SRT。迁移、备份、停止和正式归档回归见 [P0验收](VIDEO-DIRECTOR-VERIFICATION.md)。

## 物体分镜 JSON 迭代（2026-11-01，仍为v14）

无表/列或迁移变更。`video_jobs.plan_json.scenes[].beats`保存`cue`（全局零基字幕索引）、`object`、`action`、`label`；目录由受控源码定义，不存模型代码或文件路径。旧场景的labels仍可解码，缺beats默认为空，仅用于历史读取；不能把旧文字图解当作新版绘图输入。升级由用户明确保存字幕并清空分镜，保留duration/audioSha256/cues，随后复用原任务准备流程。已完成任务不可改写，制作新版使用原音频和已登记SRT新建任务。历史素材、发布快照、备份和恢复不重写。

## 原声视频任务（2026-11-01，v14）

新增 `0014_video_jobs.sql`，user_version=14。video_jobs 记录书籍、源音频、可选源字幕、文本模型、状态/阶段、递增run_id、plan_json、错误、字幕/视频结果素材ID及时间。plan_json 保存原声SHA-256、标题、时长、带时间码字幕和受约束分镜；不存密钥。状态包括draft/running/awaiting_confirmation/succeeded/failed/cancelled，部分唯一索引限制单个running任务。MP4与最终SRT同一事务登记，重复完成返回已有结果，迟到结果不能写入。

重启/恢复将running置failed并递增run_id，保留已保存字幕和分镜，须手动重试。素材ID复用assets；完整备份沿用原声/字幕必需、视频可选的规则，缓存不备份。运行中禁止源素材移除/重新定位、书籍删除及数据库恢复。v12/v13升级和备份恢复检查见 [MG视频验收](MG-VIDEO-VERIFICATION.md)。后文v13及之前条目为历史切片记录。

## 多尺寸封面 JSON 扩展（2026-11-01）

SQLite 结构保持 v13，无迁移脚本或表/列变更：

- `editor_drafts(kind=cover).content_json` 保存 `{schemaVersion:2, activeSize, covers:[4:3配置,3:4配置,16:9配置]}`，含各尺寸未完成内容。即使正式保存后仍保留该工作区，记录当前选择和完整配置；based_on 继续引用本书合法封面版本。旧 schemaVersion=1 单份草稿在前端读取时展开，不改写原历史。
- `cover_versions.config_json` 仍为单个 CoverConfig schemaVersion=1，新增 `backgroundPrompt`（缺失默认为空，正式保存最多2000字）。每条版本及 PNG 仍只有一种尺寸，发布准备的双封面选择不变。
- `writing_versions.content_json` 增加 `coverPromptWidescreen`（缺失默认为空），新全量 AI 输出要求完整；旧数据仍可编辑/按项补全。
- `ai_tasks(kind=cover).input_json` 新任务记录目标 width/height。启动时既有中断任务恢复策略不变，不自动重发历史任务。

多尺寸配置和工作区随 SQLite 备份/恢复保留；背景仍引用 assets 的 ID，完整备份既有素材收集和路径重映射不变。兼容方向为新版读取旧记录；未承诺旧版应用能读取新版 JSON。验证见 `COVER-INDEPENDENT-VERIFICATION.md`。

## 新建书籍与来源文件原子保存（2026-10-28）

本地书籍绑定复用 `assets.asset_type=document` 与现有项目外键，不新增表或列，版本保持 v13。书籍和本次新增文档关联在同一事务内写入，路径重复、不可读、缺失或不支持的文件均使整次保存回滚；重开应用及备份恢复继续使用现有素材记录。

## 自动音频任务 v13（2026-10-28）

2026-10-30 来源流程调整：沿用 `notebook_identified` 先保存地址，再由 `notebook_created` 进入添加来源，但不再自动命名；旧命名阶段任务继续时核对原 URL 后进入上传。页面结构化错误经宿主转换为 `audio_step_failed`，复用现有错误字段、人工核对和提交不明保护。无新表、列或迁移，schema 仍为 v13。下段为之前命名恢复修复的历史说明。

2026-10-29 身份恢复修复：`notebook_identified` 在命名前保存既有 `notebook_url`，保留 `creating_notebook` 阶段。已记录地址的命名失败及重启可继续命名，只有命名回读成功才进入添加来源。旧无地址任务仅在用户确认且宿主核对为空白笔记本后原行关联，递增 `run_id`；禁止替换已记录的笔记本或重新关联已提交任务。继续使用 v13，无表、列或迁移变更。

2026-10-29 网站恢复补充：刷新/重新打开使用既有 paused 状态及递增 run_id；已完成与停止记录仅查看网页，不自动恢复。用户明确继续 failed/cancelled 时原行恢复，检查活动任务唯一性；adding_source/submitting 阶段恢复先只读 verify_remote。冻结输入、原笔记本、下载摘要及成品引用保持，无新迁移。

新增 `0013_audio_jobs.sql`，`user_version=13`。`audio_jobs` 保存项目与发布文案版本、冻结 `input_json`、唯一持久状态/阶段、递增运行编号、错误、远端 Notebook 身份、下载结果、素材引用及创建/提交/检查时间。`input_json` 只保存本次指导、目标秒数、格式和唯一来源快照；文档来源包含素材 ID、名称、大小和 SHA-256，文本来源保存资料文本。密钥、Cookie、临时签名下载 URL、浏览器 profile、标签页和 downloadId 不进入数据库。

部分唯一索引保证全局至多一个 `status NOT IN ('succeeded','cancelled')` 的任务；失败任务仍占位，只有用户明确停止跟进后才可开始新任务。状态为 created/waiting_login/preparing/generating/downloading/importing/succeeded/waiting_user/paused/failed/cancelled。页面观察只由 Rust 根据当前状态和阶段接受；提交结果不明时进入人工核对，不自动重新点击生成。

下载完成后 Rust 校验真实音频容器并复制到项目 `audio/`，再在同一事务中登记 `assets.asset_type=audio` 和任务 succeeded；重复恢复返回同一素材。完整备份通过既有 assets 规则自动包含最终音频，不包含未完成浏览器下载。应用启动与恢复把已开始任务置为 waiting_user/verify_remote 并递增运行编号；尚未开始、无 URL/提交记录的 opening/旧等待扩展任务可从 opening_site 继续。迟到结果不能写入。有未结束音频任务时拒绝恢复数据库，需先停止跟进。

当前数据库支持 v2–v13 备份恢复，旧备份在内存顺序升级且源文件不改写。v12→v13、单活动任务、冻结快照、跨书来源拒绝、文件指纹、真实音频归档、事务失败重试、备份恢复和发布素材关联均有 Rust 回归。2026-10-29 内置窗口替换无新迁移；缓存路径按任务/运行推导，摘要沿用 output_size/output_sha256。当前证据见 IN-APP-AUDIO-VERIFICATION.md；旧 Chrome 历史证据另见 `AUDIO-AUTOMATION-VERIFICATION.md`。

## 音频目标时长 v12（2026-10-27）

迁移 `0012_audio_duration.sql` 仅为 `book_projects` 新增可空 `audio_duration_source`（ai/manual），复用原 `target_duration_seconds`。NULL表示尚未设置，允许首次AI推荐；旧目标秒数、书籍资料、ID、时间戳和历史提示词均保留。迁移与user_version=12事务提交；冲突回滚保留v11。新入口只接受60—86400秒且为整分钟，数据库旧字段约束不收紧。

推荐任务复用 `ai_tasks.kind=publishing`，Prompt版本为 `audio-duration-v1`，input_json存推荐标记及项目/模型ID；不保存凭据或原书全文。有效推荐成功时，项目目标秒数/source=ai与任务succeeded在同一事务提交，不追加writing_versions；取消、结构错误或事务失败均不改变书籍时长。手动时长只改目标/source=manual及更新时间，不回写整份项目资料。常规生成input_json冻结targetDurationSeconds，音频按项仍先预览再追加版本。

既有备份校验按迁移清单支持v2—v12，保存的秒数和source随数据库备份恢复；旧备份在内存升级，不改写源文件。历史结果JSON不新增字段、不批量替换时长。回退旧程序需使用升级前数据库备份。

## 封面推荐兼容扩展（2026-10-27）

无表结构变更，数据库保持 v11，不新增 SQL 迁移。`writing_versions.content_json` 可包含 `coverDesign` 推荐对象；旧记录读取为无推荐且不改写原 JSON。`cover_versions.config_json`、封面草稿和 `cover_presets.style_json` 增加 `subtitleColor`，缺失时沿用该配置的 textColor；兼容 classic/night，新增 spotlight/ember/ocean 模板 ID。推荐和副标题色使用原有追加历史与备份恢复流程。

## 智能辅助发布 v11（2026-10-27）

`0011_douyin_collection.sql` 仅为 `publish_tasks` 增加可空且校验 JSON 的 `douyin_collection`。NULL 表示未选择；对象的 `name=null` 表示明确不加入合集，否则为真实选项名称，另存可空合集 ID、公开账号 ID 和显示昵称。既有任务、时间戳、素材、文字快照与发布记录不回填、不重写。

`save_douyin_collection` 在同一事务内保存任务意图及可选默认值。默认值复用 `app_settings` 的 `douyin_collection:{公开账号ID}`，昵称不足以启用记忆。当前适配器尚未验证稳定公开 ID，会返回 null，因此 UI 不提供持久化默认复选框。更改本次合集不会改账号默认；修改默认不会改旧任务。在线归类不属于媒体 manifest，修改合集不会失效 ready 包，也不修改包或原文件。

当前 `user_version=11`，数据库备份接受 v2–v11，旧备份在内存中升级。合集意图与默认随 SQLite 和完整目录备份恢复；浏览器会话、上传进度、头像指纹及 Cookie 不持久化。任务删除继续随项目级联。回归覆盖 v10→v11 原数据保留、迁移冲突回滚、旧备份逐版本恢复、合集重启/恢复与完整备份恢复；实际结果见 `DOUYIN-FLOW-VERIFICATION.md`。

应用更新偏好（2026-10-25）：复用既有 `app_settings`，键 `auto_update` 保存字符串 `true` / `false`，未设置默认为开启；通过受限 bool 命令读取/修改，随数据库备份恢复。版本、下载内容与进度属于运行状态，不新增业务表。schema 保持 v9，无迁移脚本。

## 1. 设计原则

- SQLite 作为第一版结构化业务数据主存储。
- 使用迁移脚本维护数据库版本。
- 使用事务保证关联数据一致性。
- 业务主键建议使用 UUID。
- 时间统一保存为 UTC 时间戳或规范 ISO 8601 字符串，并在 UI 转换为本地时间。
- 大型文件存储在文件系统，数据库保存路径与元数据。
- API Key 不存入普通业务表。
- 删除项目不得默认删除原始视频文件。
- 对常用过滤字段建立索引。
- JSON 字段仅用于适合扩展的结构，不用 JSON 取代所有关系表。

## 2. 核心表

> 下列为整体模型设计；M0 实现见第 7 节，M1 实际表及暂缓字段见第 8 节，M2 实际模型及创作版本见第 9 节。

### 2.1 book_projects

| 字段 | 类型 | 说明 |
|---|---|---|
| id | TEXT PK | 项目 ID |
| title | TEXT NOT NULL | 书名 |
| author | TEXT | 作者 |
| publisher | TEXT | 出版社 |
| isbn | TEXT | ISBN |
| description | TEXT | 简介 |
| source_notes | TEXT | 用户资料与笔记 |
| target_audience | TEXT | 目标受众 |
| content_style | TEXT | 内容风格 |
| target_duration_seconds | INTEGER | 目标时长 |
| status | TEXT | 项目状态 |
| workspace_path | TEXT | 工作目录 |
| created_at | TEXT | 创建时间 |
| updated_at | TEXT | 更新时间 |
| archived_at | TEXT | 归档时间，可空 |

### 2.2 script_versions

| 字段 | 类型 | 说明 |
|---|---|---|
| id | TEXT PK | 版本 ID |
| project_id | TEXT FK | 所属项目 |
| version_no | INTEGER | 版本号 |
| outline_json | TEXT | 大纲结构 |
| content_markdown | TEXT | 讲稿 |
| source_type | TEXT | ai / manual / mixed |
| prompt_version | TEXT | Prompt 版本 |
| model_name | TEXT | 模型名称 |
| is_confirmed | INTEGER | 是否确认 |
| created_at | TEXT | 创建时间 |

一个项目可以有多个讲稿版本。建议对 (project_id, version_no) 建立唯一约束。

### 2.3 publish_materials

| 字段 | 类型 | 说明 |
|---|---|---|
| id | TEXT PK | 素材 ID |
| project_id | TEXT FK | 所属项目 |
| material_type | TEXT | title / description / hashtags / cover-copy |
| content | TEXT | 内容 |
| version_no | INTEGER | 版本号 |
| is_selected | INTEGER | 是否选中 |
| is_confirmed | INTEGER | 是否确认 |
| model_name | TEXT | 模型名称 |
| created_at | TEXT | 创建时间 |

建议对项目、素材类型和版本号建立索引。

### 2.4 assets

| 字段 | 类型 | 说明 |
|---|---|---|
| id | TEXT PK | 素材 ID |
| project_id | TEXT FK | 所属项目 |
| asset_type | TEXT | video / cover / image / subtitle / document |
| file_path | TEXT | 路径 |
| original_name | TEXT | 原始文件名 |
| mime_type | TEXT | MIME 类型 |
| file_size | INTEGER | 文件大小 |
| checksum | TEXT | 校验值，可空 |
| width | INTEGER | 宽度 |
| height | INTEGER | 高度 |
| duration_seconds | REAL | 视频时长 |
| exists_status | TEXT | available / missing / unknown |
| created_at | TEXT | 创建时间 |

### 2.5 cover_versions

| 字段 | 类型 | 说明 |
|---|---|---|
| id | TEXT PK | 封面版本 ID |
| project_id | TEXT FK | 所属项目 |
| background_asset_id | TEXT FK | 背景素材，可空 |
| exported_asset_id | TEXT FK | 导出文件，可空 |
| template_id | TEXT | 模板 ID |
| layout_json | TEXT | 可编辑排版配置 |
| version_no | INTEGER | 版本号 |
| is_selected | INTEGER | 是否选中 |
| created_at | TEXT | 创建时间 |

### 2.6 model_providers

| 字段 | 类型 | 说明 |
|---|---|---|
| id | TEXT PK | 配置 ID |
| name | TEXT | 显示名称 |
| provider_type | TEXT | 服务类型 |
| base_url | TEXT | 服务地址 |
| model_name | TEXT | 模型名称 |
| reasoning_effort | TEXT NULL | v15：服务默认或文本模型的显式推理强度 |
| capabilities_json | TEXT | 能力声明 |
| is_default | INTEGER | 是否默认 |
| is_enabled | INTEGER | 是否启用 |
| credential_ref | TEXT | 系统凭据引用，可空 |
| created_at | TEXT | 创建时间 |
| updated_at | TEXT | 更新时间 |

不得保存 API Key 明文。credential_ref 只保存安全凭据引用或标识。

### 2.7 ai_tasks

| 字段 | 类型 | 说明 |
|---|---|---|
| id | TEXT PK | 任务 ID |
| project_id | TEXT FK | 所属项目 |
| task_type | TEXT | 任务类型 |
| provider_id | TEXT FK | 服务配置 |
| model_name | TEXT | 模型名称 |
| status | TEXT | 任务状态 |
| input_json | TEXT | 脱敏后的任务输入 |
| output_json | TEXT | 输出摘要 |
| error_code | TEXT | 错误码 |
| error_message | TEXT | 脱敏错误 |
| started_at | TEXT | 开始时间 |
| completed_at | TEXT | 完成时间 |
| created_at | TEXT | 创建时间 |

禁止将密钥、完整敏感请求头写入任务输入或日志。

### 2.8 publish_tasks

| 字段 | 类型 | 说明 |
|---|---|---|
| id | TEXT PK | 发布任务 ID |
| project_id | TEXT FK | 所属项目 |
| platform | TEXT | douyin 等 |
| video_asset_id | TEXT FK | 视频素材 |
| cover_version_id | TEXT FK | 封面版本 |
| title_material_id | TEXT FK | 标题素材 |
| description_material_id | TEXT FK | 描述素材 |
| status | TEXT | 状态 |
| task_directory | TEXT | 任务包目录 |
| checklist_json | TEXT | 检查结果 |
| created_at | TEXT | 创建时间 |
| updated_at | TEXT | 更新时间 |

### 2.9 publish_records

| 字段 | 类型 | 说明 |
|---|---|---|
| id | TEXT PK | 记录 ID |
| publish_task_id | TEXT FK | 发布任务 |
| platform | TEXT | 发布平台 |
| published_at | TEXT | 实际发布时间 |
| post_url | TEXT | 作品链接 |
| status | TEXT | 状态 |
| views | INTEGER | 播放量 |
| likes | INTEGER | 点赞量 |
| comments | INTEGER | 评论量 |
| favorites | INTEGER | 收藏量 |
| notes | TEXT | 备注 |
| created_at | TEXT | 创建时间 |
| updated_at | TEXT | 更新时间 |

### 2.10 app_settings

| 字段 | 类型 | 说明 |
|---|---|---|
| key | TEXT PK | 设置键 |
| value_json | TEXT | 普通设置 JSON |
| updated_at | TEXT | 更新时间 |

仅保存工作目录、默认模型 ID、封面尺寸和文案偏好等普通配置。不得将密钥写入该表。

## 3. 关系

```text
book_projects
  ├── script_versions
  ├── publish_materials
  ├── assets
  │    └── cover_versions
  ├── ai_tasks
  └── publish_tasks
       └── publish_records

model_providers
  └── ai_tasks

app_settings
  └── 应用级普通设置
```

## 4. 状态值

### 项目状态
- draft
- researching
- scripting
- material_preparing
- ready_to_publish
- published
- archived

项目流程允许跳步，状态不能强迫用户完成所有创作环节。

### 发布任务状态
- draft
- preparing
- ready
- publishing_manual
- published
- failed
- cancelled

第一版只有在用户确认发布成功后才能将任务标记为 published。

### AI 任务状态
- pending
- running
- succeeded
- failed
- cancelled

## 5. 数据一致性与迁移

- 启用 SQLite 外键约束。
- 为常用查询字段建立索引。
- 重要多表操作使用事务。
- 每次结构变更新增迁移脚本。
- 启动时检查并按序执行未执行迁移。
- 迁移失败时明确报错并保留恢复路径。
- 不允许通过删除数据库来“修复”迁移问题。
- 删除项目时使用明确的关联清理策略，不删除外部原始视频。
- 对文件系统操作与数据库写入设计补偿逻辑，避免单侧成功后数据失配。

## 6. 备份与恢复

- 提供手动备份。
- 备份数据库和必要配置。
- 不默认复制原始视频。
- 恢复前先备份当前数据。
- 恢复后校验数据库可读。
- 对路径失效的素材保留记录，并允许用户重新定位。

## 7. M0 技术验证数据库（2026-10-23）

- 文件：`%LOCALAPPDATA%\com.shuyin.desktop\shuyin.sqlite3`。
- 迁移：`src-tauri/migrations/0001_m0_probe.sql`，版本号 `PRAGMA user_version = 1`。
- 表：`m0_probe(id INTEGER PRIMARY KEY CHECK(id = 1), note TEXT NOT NULL, updated_at TEXT NOT NULL)`；文字限制 2000 字符，UTC 时间由 SQLite 生成。
- 仅保存一条可更新的技术验证文字，不保存视频路径、书籍内容或配置密钥。
- 首次建表及版本更新在同一事务中执行；重复启动只读取版本；迁移失败保留原数据库并返回错误；更高版本拒绝打开，不降级或删库。
- M1 应从版本 2 开始添加业务迁移，保留本地已有数据；不要重置版本号或复用 0001。

## 8. M1 实际数据库（2026-10-23）

新增迁移 `0002_projects_assets.sql`，`user_version=2`。按顺序、每版本一个事务执行；版本 1 升级保留 `m0_probe`，结构冲突回滚本次迁移，较高版本拒绝打开。

实际新增表：
- `book_projects`：第 2.1 节字段，加 `theme TEXT NOT NULL`；可选文字用空字符串，目标时长 1–86400 秒；限制状态和非空书名；归档时间随归档/取消归档更新。
- `assets`：仅实现 id、project_id、asset_type、file_path、original_name、file_size、modified_at（Unix 秒）、created_at。`exists_status` 每次读文件系统计算，不把过期状态作为数据库事实。MIME、尺寸、视频时长和 checksum 暂未实现。
- `app_settings(key TEXT PRIMARY KEY, value TEXT NOT NULL)`：目前只有纯文字 `workspace_root`，不提前保存 JSON 配置或密钥。

主键使用 SQLite `randomblob(16)` 生成的 32 位十六进制随机 ID，无新增 UUID 依赖。索引包含项目状态/更新时间、素材项目 ID；同一项目的规范化文件路径唯一。素材外键级联删除只清理数据库关联，不进行文件删除。

M1 当时的备份包含这三张业务表及 M0 验证表，仅接受 v2。M2 后的备份策略见下节。源文件、工作目录内容需要另行保管。恢复前的自动备份位于数据库同级 `backups` 目录；拒绝损坏、未知结构、额外触发器或外键失效的备份。恢复不会搬迁或复制路径指向的素材。

## 9. M2 实际数据库（2026-10-23）

新增 `0003_ai_writing.sql`，`user_version=3`；迁移失败回滚本次版本，保留 v2 数据。新增三表：

- `model_providers`：id、name、provider_type（当前仅 openai-compatible）、base_url、model_name、is_default、is_enabled、credential_ref、created_at、updated_at。部分唯一索引保证至多一个默认模型，默认模型必须启用。credential_ref 为随机系统凭据条目标识，不保存密钥。
- `writing_versions`：id、project_id、kind、version_no、content_json、source_type、model_name、prompt_version、is_confirmed、created_at。kind 为 outline / script / titles / descriptions / hashtags；`(project_id,kind,version_no)` 唯一。大纲和讲稿保存 `{format:"markdown",text}`；候选列表保存 `{format:"list",items}`。与第 2 节早期规划不同，M2 共用一张版本表，不并行创建 script_versions/publish_materials。
- `ai_tasks`：id、project_id、kind、provider_id、model_name、prompt_version、status、input_json、version_id、error_code、error_message、created_at、completed_at。输入仅记录项目/模型/参考版本 ID、类型和数量，不保存密钥、授权头或完整书籍资料；成功关联结果版本，错误为本地固定脱敏信息。

版本只追加，不修改或覆盖历史已确认内容；来源为 ai/manual/mixed。生成成功时，新版本、项目更新时间和任务成功状态在同一事务提交。任务取消后迟到响应不能写版本。应用重启将 pending/running 任务标为 failed/interrupted，不自动重试。

项目删除级联清理创作版本和任务，不删除媒体。模型删除保留历史任务和模型名，provider_id 设 NULL；有未完成任务时拒绝删除模型。系统凭据与 SQLite 不能共享事务：密钥替换先写新随机条目，数据库保存失败时删除新条目，旧条目保留；成功后清理旧条目，清理失败明确返回提示。

备份包含全部上述表但不含密钥。恢复接受 v2/v3：先核验对应原始 schema、完整性与外键，再在内存快照迁移到 v3，自动备份当前数据库后恢复。源备份文件保持原版本；有运行任务时拒绝恢复。备份中的未完成任务恢复为 failed/interrupted；恢复后按当前系统凭据库重新检查密钥，不伪造“已配置”状态。

## 10. 三项结果数据库 v4（2026-10-23）

新增 0004_publishing.sql。事务内同步重建 writing_versions 和 ai_tasks，扩展 kind=publishing，复制旧行后替换，保留任务到旧版本的外键引用、ID、时间戳及旧五类内容。模型表和凭据引用不改动。

publishing 的 content_json 为 `{format:"publishing",title,description,audioPrompt}`。一套结果一个版本，新生成/编辑只追加。标题≤30字，内容≤1000字，提示词≤20000字；AI 原始草案必须有五个完整章节，模板组装后再保存。任务 input_json 仅记录 projectId/providerId，不保存密钥或完整资料。

备份接受 v2/v3/v4，仍在内存中迁移旧备份，原备份不改写；恢复前备份当前数据库。迁移失败回滚，保留旧 schema 和数据，不重置数据库。旧格式只用于保留历史数据，新界面查询 publishing。

## 11. M3 封面版本 v5（2026-10-23）

新增 `0005_covers.sql`，user_version=5。只增加 cover_versions，原七张表及其记录不改写。迁移和版本更新在同一事务内执行。

用户澄清发布封面比例后，新保存/导出仅接受 1440×1080（4:3）与 1080×1440（3:4），本次调整不改变表结构或 schemaVersion。一次导出生成两条独立版本记录。旧尺寸 config_json、exported_path 与文件保留；转换比例只发生在页面恢复编辑后的新草稿，不重写旧行。

实际字段：id、project_id（关联 book_projects，ON DELETE CASCADE）、version_no、config_json、exported_path（配置保存时可空）、created_at。唯一约束 `(project_id, version_no)`；没有更新历史配置的接口。

与第 2.5 节早期草图不同，本轮将模板与背景素材 ID 保存在完整 config_json 中，不重复维护 template_id/background_asset_id 列；不增加导出素材表或 is_selected 状态。JSON 包含 schemaVersion=1，反序列化及保存时校验固定模板、尺寸、文字长度、颜色、字号、对齐与位置。

新版本和项目更新时间事务提交。PNG 只写项目 covers 目录中的唯一新文件；写入或数据库提交失败不会产生成功版本，并尝试清理本次新文件。删除项目只级联删除版本行，保留原图片与所有已导出文件。

备份现在接受 v2–v5，按原 schema 核验并在内存中顺序升级到 v5，不修改源备份。备份包含封面配置与路径，不包含背景/PNG 二进制；外部文件需另行备份。被删除或移动的背景保留引用以便提示恢复，不以删除历史记录修复缺失。

## 12. 可选图片生成 v6（2026-10-24）

`0006_image_generation.sql` 在模型表增加 `model_kind`（text/image，旧行默认 text），将单个默认文本模型索引替换为按用途唯一的默认模型索引；在 ai_tasks 增加可空 `asset_id`，外键指向 assets，移除关联时设 NULL。不新增表，不改写既有封面配置、路径或凭据引用；迁移失败整次回滚并保留 v5。

图片任务使用 `kind=cover`，input_json 仅保存 projectId/providerId/description，不包含 API Key。生成成功时以事务新增 assets、设置任务 asset_id/succeeded 及项目更新时间；实际 PNG 写入项目 covers 子目录中的唯一 `ai-background-{随机ID}.png`。失败清理本次未登记文件；取消后不允许迟到结果写入。背景未自动成为封面历史，用户保存/导出时才追加 cover_versions。

备份接受 v2–v6，继续核验原 schema 后在内存中迁移，不修改源备份；媒体不包含在数据库备份内。v5→v6 数据/凭据引用保留、迁移回滚、旧备份恢复均有 Rust 回归；隔离 Windows 实测 v6 备份恢复后模型、背景引用和封面历史一致，完整性/外键检查通过。

## 13. M4 发布准备 v7（2026-10-24）

新增 `0007_publish_tasks.sql`，事务内增加两张表及索引，旧表及记录不改写：

- `publish_tasks`：id、project_id、title、description、tags、video_asset_id、landscape_cover_id、portrait_cover_id、title_confirmed、covers_confirmed、status、task_directory、created_at、updated_at。标题/内容/标签是发布时的文字快照；不并行新增早期草图中的 publish_materials。视频和封面外键移除时设 NULL，任务仍可重新选素材。
- `publish_records`：publish_task_id（主键兼外键，一项任务一次确认）、published_at、post_url、notes、created_at。仅在明确确认后插入；记录、任务 published 状态和项目状态在同一事务提交。已发布任务固定，另一作品另建任务；不引入运营指标。

当前单一平台为抖音，平台名记录在任务包 manifest 中；未增加可选平台或重复配置。实际任务状态只使用 draft / ready / published。保存修改退回 draft，生成完整且有效的任务包后为 ready；每次查询按素材路径、文件大小/修改时间及任务包文件重查，失效时退回 draft。published 为已确认的历史事实，不因源文件后来移动而撤销。

迁移支持 v1 顺序升级至 v7。备份恢复接受 v2–v7，在内存迁移并提前备份当前库；旧备份文件保持原样。备份不含视频、封面和任务包目录，需另行保管。源视频仅引用；删除项目只级联数据库行，不删除视频、封面或任务包。

## 14. M5 数据验证（2026-10-24）

本轮没有数据库结构变更，schema 保持 v7。新增 v2–v7 逐版本恢复及安全备份回退回归，保留原备份字节、书籍资料、时间戳和缺失素材引用；三项目回归核对文案/封面历史、任务包、任务记录与凭据可用状态。任务包封面内容校验只影响查询时的就绪判定，沿用现有 ready→draft 转换，不新增持久化校验字段。

## 15. 封面素材结果扩展（2026-10-24）

表结构、索引与 `user_version=7` 均不变，无 SQL 迁移。`writing_versions.content_json` 的 publishing 对象新增四个字符串：`coverTitle`（≤30字）、`coverSubtitle`（≤60字）、`coverPromptLandscape` 和 `coverPromptPortrait`（各≤2000字）。新生成必填全部七项，并标注 `publishing-v2`。

旧三项 JSON 读取时由 serde 默认补空字符串，不执行 UPDATE 或批量回填，原始 JSON、ID、版本、时间及引用保持原样。人工编辑允许封面字段为空，因此旧版无需先生成封面才能保存。新保存仍追加版本；不把结果页封面素材重复写入 cover_versions，只有在封面页确认保存/导出才生成封面版本。

备份继续接受 v2–v7，现有快照包含完整 content_json。Rust 回归验证旧三项原始 JSON 在备份恢复后字节不变、新四项在重启后完整保留、超长字段拒绝新增版本，以及三书全流程的新字段随备份保留。升级后的结果由本次新代码读取；旧程序不具备新字段契约，回退需使用升级前的数据库备份。

## 16. 封面 16:9 尺寸扩展（2026-10-25）

保存校验增加 1920×1080，沿用 `cover_versions.config_json` 的宽高字段和 `schemaVersion=1`；数据库仍为 v7，无 SQL 迁移。批量导出追加 4:3、3:4、16:9 三条独立版本，原有行与文件不重写。已有 1920×1080 历史配置可直接恢复编辑，不再转换为 4:3。发布任务的两张封面引用保持原有比例要求。

## 17. 按项调整 v8（2026-10-25）

新增 `0008_generation_preview.sql`，只为 `ai_tasks` 增加可空且校验 JSON 的 `preview_json`，原记录、内容、时间戳及凭据引用不变。迁移与 `user_version=8` 在同一事务内提交；失败保留旧版本。

按项任务仍使用 `kind=publishing`，`input_json` 增加可选 `revision`：`basedOn`、`fields`、`instruction`。不记录原书全文、当前内容快照或密钥。请求使用来源版本，拒绝跨项目/跨类型引用、空字段集合、重复字段及超长要求；Prompt 版本为 `publishing-revision-v1`。

候选生成成功把完整合并结果写入 `preview_json` 并标记 succeeded，不新增 `writing_versions`。采用时事务内追加来源 mixed 的版本、写入实际模型和 Prompt 版本、设置 `version_id` 并清空候选；重复采用返回已关联版本。取消后的迟到响应不能写入候选，采用失败保留候选和旧版本。

备份接受 v2–v8，继续核验原 schema、完整性和外键，在内存中迁移旧备份；源备份不改写。v8 备份包含候选数据，正式 UI 不提供跨重启恢复候选。回退旧程序须使用升级前数据库备份，不对 v8 强制降级。

## 18. 创作草稿、个人样式与发布来源 v9（2026-10-25）

迁移 `0009_creator_workflow.sql` 在同一事务内新增：

- `editor_drafts`：联合主键 `(project_id, kind)`，kind 为 writing/cover，保存 `based_on`、`content_json`、`updated_at`；项目删除级联。来源版本由业务服务核验同项目/类型；允许未完成对象，JSON 上限512 KB。不新增历史版本，显式保存成功清除对应草稿。
- `cover_presets`：id、唯一 name、style_json、created_at，全应用共享；风格只含白名单字段，不携带书籍文字/素材引用。名称重复明确拒绝，应用与删除不改动封面历史。
- `publish_tasks.writing_version_id`：可空外键引用 writing_versions，删除来源时置空；旧任务为空，继续保留原文字快照。新保存校验 publishing 类型与同项目，任务包 manifest 记录来源和实际文本。

`cover_versions.config_json` 的 `schemaVersion=1` 增加 `backgroundX`、`backgroundY`（0–1）和 `backgroundZoom`（1–3）；旧 JSON 不重写，读取缺省0.5/0.5/1。所有比例共用同一草稿裁切值，分别追加导出版本。

v9 当时的 `user_version=9`，支持顺序升级和 v2–v9 数据库备份恢复。兼容迁移不删除旧行；结构冲突回滚整个 v9，保留 v8 数据与版本。回退旧程序需使用升级前数据库备份。

完整备份格式版本为1，与数据库版本独立。目录包含一致性 SQLite 快照、文件清单及 SHA-256，登记的素材/导出封面/当前任务包均复制；可选原始视频。草稿及样式随数据库保留。恢复在内存核验/迁移快照，将文件复制到新目录后重建 `workspace_path`、`file_path`、`exported_path`、`task_directory` 及包内引用，再自动备份当前库并替换。不复制未登记文件、旧的无引用任务包或系统凭据；未包含视频继续引用原路径，缺失时记录保留且任务退回草稿。

回归覆盖 v8→v9 保留/失败回滚、草稿重启、样式白名单、跨书来源拒绝、备份目录搬移、源文件移除后的恢复、遗漏视频、同大小损坏与越界路径拒绝。Windows 真实文件/数据库证据见 `CREATOR-WORKFLOW-VERIFICATION.md`。

## 19. M6 辅助发布（2026-10-26）

M6 当时 schema 为 v9，无迁移或新增持久化状态。`publish_snapshot` 只读取已校验 ready 任务的文字快照、视频引用和包内封面；上传会话与停止标志保留在 Rust 内存，不存网页、账号、Cookie 或浏览器进度。上传/填写完成不改任务状态、不插入发布记录；仅原有人工确认事务写入 published。会话数据不纳入业务备份，旧版数据库备份继续按原校验/恢复路径处理。独立安装版的完整性、外键、备份及升级数据比较见 `M6-VERIFICATION.md`。

M7沿用相同schema：全局任务列表关联book_projects读取书名，逐项使用现有checked校验；不新增队列表或缓存状态。文案来源仍是publish_tasks.writing_version_id，作品链接仍是publish_records.post_url；记录成功仅释放匹配任务的内存快照。备份恢复保留这些已有行，测试与隔离安装模拟记录边界见`M7-VERIFICATION.md`。

## 20. 音频制作与成片回填 v10（2026-10-26）

新增 `0010_audio_workflow.sql`，在同一事务内重建 `assets`、`publish_tasks` 与 `publish_records`：

- `assets.asset_type` 增加 `audio`，原视频、图片、封面、字幕与文档行原样复制。
- `publish_tasks` 增加可空 `audio_asset_id`，同项目音频外键；旧任务该字段为空。其余文字快照、视频、封面、状态、时间戳、任务目录和文案来源不变。
- `publish_records` 原样保留并继续引用重建后的任务。迁移后重新建立 `assets_project` 与 `publish_tasks_project` 索引。

v10 当时的 `user_version=10`，数据库备份支持 v2–v10。任务包 manifest 从 schema 1 升至 2，增加可空 `audio` 文件引用；旧任务包仍可读取，但缺少音频的待发布任务不会保持 ready。发布检查使用 `mp4parse 0.17.0` 只读取 MP4 ftyp/moov，不扫描完整 mdat，验证正时长、可读取画面尺寸与音轨，不限制横竖屏比例。音频只登记路径，不复制，不写入发布记录，也不上传抖音。
