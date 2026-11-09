# 书引 AI 技术设计文档

## 视频任务外部推进与页面同步（2026-11-09）

自动流程会把视频任务从 draft/transcript_ready 继续推进到分镜、渲染和完成，不能仅在页面缓存中存在 running 任务时轮询。视频页沿用原有1.5秒轮询，在页面加载完成后持续读取任务；选中任务状态或运行代次变化时同步编辑状态，运行中停止暂存，非运行状态重读字幕、分镜及素材。任务状态和代次未变化时保留人工未保存编辑。

编辑器 reset 同时吸收旧队列的失败，保留串行等待关系；旧任务已失效的暂存错误不再阻止新快照离页。正常编辑中的保存失败仍保留内容并阻止跳转，后端仍拒绝修改执行中或已完成的视频。无新状态、依赖、权限或数据库迁移。

## 取消音频制作时长门槛（2026-11-09）

固定自动流程不再在字幕检查点或内容校核中拒绝超过900秒的实际音频。删除独立validate_target，目标输入统一复用ai::audio::validate_duration；自动推荐不发送durationLimits，唯一推荐Prompt更新为audio-duration-v3。video::validate_cues与原声渲染入口只校验时长为正有限数值，不再设置7200秒上限。UI显示按完整原声时间轴制作。

目标数值、冻结方案和已有任务均沿用原结构，不回填旧目标或重做音频；SQLite仍schema23，无新依赖或权限。模型请求超时、视频进程等待、文件/字幕/上下文校验属于原有执行规则。新规则替代下方历史章节中的音频时长门槛；验收见[本轮记录](ACCEPTANCE-20261109.md)。

## 音频来源上传不依赖窗口焦点（2026-11-09）

真实NotebookLM当前上传器首次mouseover/focus后才创建文件输入。后台WebView中focus()可能不产生事件，导致原书上传报file_input_missing。audio_page.js改为在原上传按钮派发mouseover，再沿用点击、原弹窗内唯一input核对和原生文件写入；不强制抢前台、不增加状态或重试。真实失焦前后和测试证据见[本轮验收](ACCEPTANCE-20261109.md)，无需SQL迁移或新依赖。

## 自动制作 P2/P3 当前实现（2026-11-08）

唯一执行器顺序调整为duration → writing → audio → transcript → content_check → landscape_image/cover → portrait_image/cover → widescreen_image/cover → storyboard → render → publishing。同一VideoWork增加Transcript，复用原转写/字幕校对，在完整校对及字幕素材登记后保存draft/transcript_ready；封面完成后以Prepare继续该任务，已有字幕及校对批次不会再次请求。

content-check-v1通过统一Provider输出verified/reason/changes，未核实则失败暂停，修订复用原revision与发布结构校验。检查输入冻结来源版本、音频任务/素材ID及音频SHA/实际时长/完整cues摘要；完整字幕仍限8万字符，不截断。通过且无变化时引用原版本，必要修订追加based_on；audioPrompt禁止修改，音频任务的原版本关联与目标秒数再次核对。旧transcript-publishing-v1整篇重写Prompt已移除。

字幕内容摘要不含分镜。分镜完成后读取三个实际封面导出，再保存独立render_plan_sha256；渲染和发布分别验证实际计划与内容校核。只改分镜不会误使文案失效，冻结渲染后改变分镜会被拒绝。活动新流程的手动视频启动由原claim入口拒绝，避免越过封面阶段；停止自动流程后可手动续作。

三尺寸由COVER_STEPS映射阶段、宽高和子任务位置，根级AutomationTasks仍用唯一coverPng。最终发布任务从成功校核任务获取精确version_id，绑定同任务音频/视频及4:3/3:4导出，16:9只归档。UI的coverVersion链接用现有CoverThumbnail只读查看指定成品，编辑草稿不被替换。无新依赖/Provider/权限或第二套队列，300秒统一请求上限不变。数据库变更见[数据库](DATABASE.md)，验证及边界见[P2/P3验收](AUTOMATION-P23-VERIFICATION.md)。

## 自动制作前置创作 P1（2026-11-08）

固定执行器增加duration/writing阶段，原字幕文案阶段改为content_check并单独绑定；当前顺序为duration（已有目标时跳过）→writing→audio→video_prepare→content_check→两尺寸背景/封面→render→publishing。P2的字幕检查点、必要字段修订和第三尺寸尚未实施。

`ai_commands::execute_writing_task`由原手动Command和自动执行器共用，同一Provider、取消、300秒上限与校验。`start_bound_generation`事务绑定子任务；`prepare_generation`对自动任务读取冻结资料和模型配置。`audio-duration-v2`接受可选1—15分钟范围，后端严格验证，推荐结果/原因/项目值与任务成功同事务保存；手动推荐未传范围时沿用既有1—1440分钟能力。

音频创建要求真实版本ID，并采用流程冻结秒数，不再调用独立固定指导构造器。前置writing_task_id与后置content_check_task_id分别表示音频指导来源和实际成片发布材料，保留原字幕/视频计划摘要检查。内容页按writingVersion参数精确打开版本，有恢复草稿时仅展示对照；根级任务快照推动方案到达反馈，手动/自动同时启动由后端拒绝。

schema22新增两个任务关联，旧流程标识0并暂停交接原业务页面，新流程标识1；不维护第二套自动执行顺序。完整证据及尚未验收范围见[P1验收](AUTOMATION-P1-VERIFICATION.md)。

## 自动流程现场迁移修复（2026-11-07）

安装版首次启动自动任务时，早期v20表缺少上传/交接标记导致 `load` 查询失败。新增v21事务迁移统一结构，保留两类v20中已存在的标记和所有关联；`database::migrate`由启动与备份副本升级共用。备份精确校验仅增加已观察到的早期v20结构，无忽略未知字段或跳过校验。见 [数据库](DATABASE.md)和[现场验收](AUTOMATION-LIVE-TEST-20261107.md)。

11月8日真实网页补测：正文由`prepareText`核对旧值、返回单换行文本并选择当前编辑器，宿主通过已有CDP发送一次`Input.insertText`，随后严格回读。移除脚本`execCommand`输入，重复继续不重复填写；仅规范化零宽字符、CRLF及重复空行，文字和段落边界仍严格匹配。失败嵌入卡可重核当前网页，就绪未改任务续接不改冻结确认且仍等待有效成片元数据。自动发布继续前可续期尚未交接的候选时间，交接后不改期，原草稿与任务ID保留。真实准备已到awaiting_publish，手机预览段落占位差异、实际时长和响度限制见现场报告，不表示实际公开发布通过。

## 固定自动制作流程（2026-11-07）

`automation.rs`顺序推进固定阶段，`database/automation.rs`保存输入快照与子任务ID，子任务继续作为阶段事实来源。创建方法增加内部可选绑定，在原事务写流程关联。自动音频使用`automatic-audio-v1`和原模板，不伪造前置文案；正式文案用`transcript-publishing-v1`处理本视频完整字幕，合并音频任务实际冻结指导。超过80,000字符明确暂停，不截断。

App根级`AutomationTasks`调用原`coverPng`导出；成功与失败回传匹配流程ID、运行代次和阶段。文案任务冻结视频计划摘要，渲染与发布前核对；自动视频使用音频归档摘要，原执行器验证文件。网页发布复用`douyin_flow`，启动前确认并冻结账号，上传时持续核对。自动策略与人工确认分开，发布输入变化即失效。

`publish_started`在上传前保存，重开丢失会话不自动重传；`publish_handed_off`在交接用户前保存，之后不再改期。上传期间日期过期，仅未交接且策略一致时更新同一任务、重建包并同步网页快照。暂停/停止沿用音频、视频控制与AI取消信号，恢复/备份将活动流程暂停并保留关联。无新增依赖、官方API或自建服务。参见 [验收](AUTOMATION-VERIFICATION.md)。

复查修正（同日）：任务始终使用启动时的模型ID；阶段推进只比较该模型的用途、服务地址、模型名、推理强度及启用状态。显示名称和默认选择不改变既有请求配置，不再使任务误暂停；冻结快照仍保留原始完整配置用于追溯。

## 应用级生成与表单暂存（2026-11-06）

`services/generation.ts`按书籍/用途持有本次运行的invoke、Channel、正文预览和最终结果，编辑页只订阅并消费结果。复用原start/execute/cancel与Rust `ai_tasks`，不新增执行器、轮询或重试。取消时停止展示片段，最终状态以execute的后端返回为准，完成先于取消时仍展示已校验成果。文案保留来源草稿，结果只追加原历史；图片明确采用至请求时的宽高对应配置。状态只在应用会话中保留，重启后的任务中断仍由原Rust恢复逻辑记录。

失败或取消的时长推荐保留现有会话任务，返回页面只恢复结果，不自动再次请求；用户点击推荐才开始下一次。书籍资料UPSERT只在新建时初始化目标时长，更新已有书籍时保留数据库当前值；后续时长变更由原专用保存/推荐完成事务负责，防止旧资料表单覆盖后台成果。

`useEditorDraft`继续串行化400ms暂存，并复用`useWindowCloseGuard`。窗口保护统一为一个原生`onCloseRequested`监听，依次等待各页面保存与全局任务确认；当前Tauri API每个监听独立决定destroy，不能为同一窗口并列注册多个自动销毁回调。浏览器beforeunload仍只做未保存保护，不承诺强制关闭前异步写入。

2026-11-09修正初始化时序：自动暂存同时观察内容、未保存状态和就绪状态；首次就绪或重新载入时，未修改的已保存内容不安排写入。Vue批处理中的载入赋值不再被误认成编辑；就绪后真实修改、未保存内容的带入、封面尺寸选择等需保留的工作区状态仍按400ms暂存，显式保存及离页/关闭的串行写入保持原流程。确定性回归与发布检查见[发布故障记录](RELEASE.md#prepare-封面初始化测试修复2026-11-09)。

v19将editor_drafts扩展为owner_id主键：旧文案/封面以书籍ID作为owner，资料为书籍ID或new，发布为书籍ID:任务ID或书籍ID:new。project_id保留可空外键用于书籍删除级联；仅新建书籍草稿可无project。使用已有read/save_editor_draft命令传owner，无新增权限。资料/发布正式保存事务清理对应owner，文件与业务校验仍在显式保存执行，恢复只填编辑器。续作查询识别资料和发布草稿，不将其误映射到文案。

时长展开偏好使用本机localStorage的shuyin.writing-duration-open，不存业务数据库。CSS改动限制在创作页及其编辑文字。详见[验收记录](CREATOR-CONTINUITY-VERIFICATION.md)。

## 添加书籍按钮交互（2026-11-06）

书架顶部的渐变背景覆盖了共享按钮 `hover:bg-primary/90` 的底色变化，因此悬停反馈不明显。在 `ProjectsPage.vue` 的原 RouterLink 上增加 `add-book-cta` 与 scoped CSS：悬停或键盘聚焦时上浮 2px、增强主题色光晕、650ms 单次扫光，加号以 380ms 弹性过渡旋转 90° 并放大；按下时以 80ms 收缩至 97%，离开后复位。扫光使用隔离层和不接收指针事件的伪元素，退出时直接复位，避免反向扫光；颜色引用现有主题变量。

键盘保留独立焦点描边及原 Enter 跳转；减少动态模式禁用位移、旋转和扫光，保留静态光晕。只作用于顶部可用的添加入口，浏览器预览仍使用原禁用按钮；复用原 `/projects/new` 表单，不新增事件、状态、依赖或数据库变更。验证见[开发计划](DEVELOPMENT-PLAN.md)中的同日记录。

## 抖音发布设置（2026-11-06）

`PublishInput` 增加 `allowDownload: boolean` 与 `scheduledAt: string | null`，贯通既有页面、保存命令、SQLite、任务快照和任务包；`null` 唯一表示立即发布，不另存一份发布时间模式。定时以 `YYYY-MM-DDTHH:mm:00+08:00` 保存，前端显示北京时间，Rust 通过 SQLite 日期规范化与当前时间校验，保存失败不写任务。时间已过时禁止重新保存、导出或上传，但保留就绪任务供实际发布后人工记录结果；历史查看与人工记录不要求时间仍在未来。

沿用 `douyin_flow.rs` 单一会话编排，在合集之后增加 `settings` 步骤。适配器使用官方区域标题、`label[data-checked]` 和日期控件定位；原生 checkbox 的 checked 属性在留存官方 DOM 中与选中标签不一致，不能单独作为选择证据。设置日期时从固定北京时间转换到设备当地时间，使用原生输入事件和 Semi Input 的 `keypress` Enter 提交，然后按官方回读值转换回北京时间比较。未接受、被限时范围调整或无法读取时等待用户；已读到与目标不同的值不允许人工确认跳过。

最终阶段再次核对任务快照、有效时间与发布设置。未知控件可明确人工核对并标记为人工完成，停止与重试沿用原会话；不增加浏览器轮询服务、权限或自动发布接口。任务包 manifest 默认缺失字段按旧行为解读，默认设置沿用原文本文件，定制设置增加 `publish-settings.txt`；旧默认任务包继续有效。数据库 v18 与验证边界见[发布设置验收](DOUYIN-SETTINGS-VERIFICATION.md)。

提交前审核修复：读取当前任务快照后才判断是否复用会话，`resumable_session` 要求绑定快照完全相同且未停止；修改设置后的旧 `review` 不能直接返回，准备完成也提供停止入口。官方观察分别保留可空的保存权限、发布时间模式和时间，自动完成要求全部匹配，人工确认及最终复核仍拒绝任意可读冲突，只有无法读取的字段交由用户明确核对。

`components/ui/date-time-picker/DateTimePicker.vue` 复用 Reka UI 2.11.0 的日历/浮层及现有 Select 时分组件，替代发布页两处原生日期输入。外部只传分钟精度墙上时间字符串，定时边界拼接 `+08:00`，人工记录边界保持原设备时区转换。已保存值决定初始月份；选择日期保留小时分钟，清空后阻止提交，浮层按可用高度滚动。`@internationalized/date` 3.12.4 已在原锁中作为 Reka 依赖，本次显式声明直接依赖以使用其公开日期类型及解析 API，没有新增另一套日期实现或改变锁内版本。依据为[Reka DatePicker](https://reka-ui.com/docs/components/date-picker)、[CalendarDateTime](https://react-spectrum.adobe.com/internationalized/date/CalendarDateTime.html)及安装版本源码。

## 双主题共享颜色（2026-11-05）

`useTheme.ts` 持有唯一主题状态，在 `main.ts` 挂载 Vue 前读取本机 `localStorage.shuyin.theme`。仅接受 `light` / `dark`，无值或未知值使用原深色；切换先持久化，成功后更新根文档的 `dark` 类。读取失败可继续启动，写入失败保留原主题并通过原通知组件反馈，用户可重试。主题不参与路由 key、组件挂载、草稿或业务保存。

`style.css` 使用 `:root` 深色值与 `:root:not(.dark)` 浅色值维护唯一语义颜色，包括标题、正文、控件、面板、网格、品牌、渐变按钮和阴影。各页面替换原硬编码深色值，不增加浅色专用页面或全局覆盖补丁；原生控件通过 `color-scheme` 同步。封面/视频内容和书封自有色彩保持独立，AudioPlayer 默认引用主题变量，OLED 在组件内覆盖完整播放器颜色。

App 顶栏复用 Button/AppIcon；新增主题按钮后，小于1536px收起辅助快捷导航，小于480px隐藏按钮文字，保留侧栏、图标和可访问名称。vue-sonner 2.0.9 的公开 `theme` prop 跟随主题，`offset` / `mobileOffset` 设置顶部80px以避开64px顶栏，通知颜色共用语义变量。不新增依赖、IPC、权限或数据库迁移。详见[浅色风格验收](LIGHT-THEME-VERIFICATION.md)。

## 关于与更新主菜单恢复（2026-11-05）

`App.vue` 的唯一导航数组在“模型设置”之后恢复“关于与更新”，复用 `/about`、信息图标、选中高亮、标题提示和窄窗口图标模式。此前该入口在宽窗口只显示为侧栏底部版本号，难以辨识。底部版本改为只读展示，移除单独的窄窗口关于链接及其废弃样式；侧栏保留一个明确的关于菜单入口。更新服务、确认安装和顶部/侧栏固定布局沿用现有实现，无数据库或依赖变更。

## 工作台导航固定与滚动边界修复（2026-11-05）

页面继续由文档滚动，`.app-workspace` 不创建额外滚动容器，使现有 `.app-topbar` 的 `position: sticky; top: 0` 相对视口生效。科技主题改版曾给未限高的工作区设置 `overflow-y: auto`，顶栏因此吸附到不实际滚动的祖先，随文档一起移出窗口。

`.app-sidebar` 恢复 `sticky/top: 0`，高度为 `100dvh`；菜单超过窗口高度时在侧栏内部滚动，`overscroll-behavior-y: contain` 避免侧栏滚到底后带动正文。删除原仅在低高度下设置侧栏 overflow 的重复规则。继续使用原两列网格、256px/64px响应式宽度、顶部留白及页面内吸顶偏移，不新增监听、状态或路由滚动实现。验证命令与范围见[开发计划](DEVELOPMENT-PLAN.md)中的同日修复记录。

## 关于与更新样式对齐（2026-11-05）

`AboutPage.vue` 在原组件中重排三张卡片并维护 scoped 样式，复用共享主题、本地 Inter/JetBrains Mono、Button、Progress 和 ConfirmAction。自动更新改为带 `role="switch"`、`aria-checked`、关联标签和说明的原生按钮，直接调用原 `setAutomatic`，没有新增偏好状态。检查中禁用检查按钮，下载与安装状态沿用唯一 `useAppUpdates`；错误详情继续由 App 的 OperationFeedback 提示。

更新日志按原始行展示，将实际说明中的 Conventional Commit 类型标为彩色标签；所有片段仍用 Vue 文本插值，保留空行、长文本换行及缺失说明提示，不使用 HTML 渲染。更新卡展示真实时间与状态，平台标签描述支持范围。渐变品牌符号按参考稿绘制，使用描述性的 DESKTOP 标记，不加入无依据的 Pro 版本或内核稳定性声明。没有新增服务、依赖、IPC、权限、SQL 迁移或版本号调整。见[关于与更新样式验收](ABOUT-STYLE-VERIFICATION.md)。

## 发布准备样式对齐（2026-11-05）

`PublishingPage` 在原组件内重排返回栏、标题、任务选择、文案与素材双栏、复制和自检区，替换原局部样式，继续使用本地字体、共享颜色变量、Select、CopyButton 与 CoverThumbnail。字段计数复用原计算值；保存检查摘要由非空 checklist 推导，未保存时优先提示当前修改，双封面保持实际导出比例。菜单宽度绑定触发器，长路径换行，窄窗口单列。

原 `assetsOpen` 同时控制素材管理展开与可见性；原 `DouyinVerification` 保持挂载和订阅，仅在内嵌且无会话、无操作、无错误时隐藏，避免漏掉恢复会话或读取错误。其唯一实例移至自检操作后，保存、文件校验、任务准备和用户确认顺序不变。App 使用同一本书路由补齐侧栏与顶部入口；顶部快捷导航在小于1280px时收起，1439px及以下隐藏说明标签，避免新增入口撑宽顶栏。没有新增服务、状态存储、IPC、依赖、权限或SQL迁移。见[发布准备验收](PUBLISHING-STYLE-VERIFICATION.md)。

## 视频制作样式对齐（2026-11-05）

`VideoPage` 复用原任务/草稿状态，将输入、环境、任务、字幕校对、试片、提醒和分镜分成独立视觉区域。任务容器仍持有唯一 `jobPanel`，确保开始渲染后的滚动和分镜定位有效；`VideoJobProgress` 使用操作与媒体插槽承接原按钮/播放器，按真实状态切换颜色，集中展示错误详情，移除重复错误呈现和旧圆环装饰。百分比、更新时效、渲染子步骤及计时计算保持不变。

`VideoSubtitleEditor` 与 `VideoSceneEditor` 仅重排控件和样式；共享 `AudioPlayer` / `AudioWaveform` 通过视频模块内样式调整边框、色彩与高度，保留原声解码和播放控制。App 及现有导航使用同一路由补齐视频入口；紫色token在 `style.css` 统一定义。没有新增组件库、服务、状态存储、IPC或SQL迁移。设计与验证见[视频页面验收](VIDEO-STYLE-VERIFICATION.md)。

## 第二版页面落地（2026-11-05）

`ai-2.html` 的新增布局增量接入现有 CoversPage、PublishQueuePage、ModelsPage、SettingsPage 与 ModelDiagnostics。共享主题新增封面强调色，ProjectNavigation 按步骤显示图标颜色，App 增加使用当前书籍 ID 的封面入口。页面样式分别由原组件维护，替换原局部样式；不增加 UI 库、路由、业务服务或主题存储。

封面尺寸勾选仅为页面内的导出选择，默认全选；批量导出在原 `save` 流程中筛选配置，全部保存不筛选，当前尺寸单独导出不读取勾选。仍先渲染本轮全部待导出尺寸，再调用原保存接口；选中尺寸的背景必须可用，失败保留编辑。模型设置、任务筛选与链接复制、备份确认和真实调用诊断沿用原数据流。减少透明效果时新增玻璃区域使用实面，减少动画沿用共享规则。证据见[第二版验收](TECH-STYLE-V2-VERIFICATION.md)。

## 参考稿科技主题与局部页面（2026-11-05）

根文档声明dark与原生深色语义，共享主题将背景、控件、菜单、弹窗和通知统一到参考稿的配色、尺寸、玻璃与光晕。Inter/JetBrains Mono的woff2与OFL许可随前端构建打包，不调用字体CDN。App.vue复用原路由/目录/偏好/更新，并将目录与版本入口收敛到侧栏底部。

BookCard采用横向展示并复用原菜单；网格/列表通过继承CSS变量传递显示方式与预览高度。移除误将:global后代规则作用于整个书架的选择器。WritingPage九个字段继续共用Textarea，紧凑字段固定单行外观，保留换行与复制；历史使用一个Collapsible和原historyOpen，触发器移至保存状态。排版色卡复用CoverDesignPreview及提示插槽，媒体仍复用真实AudioPlayer/AudioWaveform。

原稿脚本、固定状态、样例版本和遥测不进入业务服务。Rust、Provider、SQLite、草稿、正式版本、Canvas导出、素材与发布权限均不变。详细落点与证据见[方案](TECH-STYLE-UPGRADE-PLAN.md)、[验收](TECH-STYLE-UPGRADE-VERIFICATION.md)。

## 共用原声播放器与真实波形（2026-11-04）

`components/AudioPlayer.vue`负责唯一媒体元素、源加载、播放/暂停、倍速和音量；`AudioWaveform.vue`负责实际幅度包络、悬停时间、进度指针与原有Slider的指针/键盘交互。媒体URL统一为`services/projects.ts::mediaSource`，复用`media_asset_path`的本书已登记音视频校验及单文件授权；移除视频service中的重复入口。WritingPage成功任务卡和其他音频列表、VideoPage制作原声、VideoSubtitleEditor任务原声均使用该组件。

`services/audioWaveform.ts`通过授权URL读取本地音频，使用浏览器OfflineAudioContext在8kHz解码，按声道计算256个RMS幅度区间并归一化。只在播放器可见且启用时请求，一次只解码一份文件，最多保留4份小型包络Promise；不保留PCM或写磁盘缓存。同源并发请求合并，失败移除缓存。SVG按实际容器宽度显示24–128条幅度柱，播放时仅逐帧读取媒体currentTime并更新裁切与指针，不向父页高频发送字幕事件或持久化播放位置。悬停只预览，拖动仍通过现有Reka Slider发出seek。时间格式统一为`lib/mediaTime.ts`，补齐Slider的aria-valuetext传递。

主窗口CSP的connect-src增加既有`asset:`与`http://asset.localhost`，用于本地解码fetch；默认asset scope继续为空，只能读取原命令显式授权的文件，没有新增IPC或capability。格式徽标只展示文件扩展名，不把M4A写成无损。两套配色仅为组件状态；EQ动画和流体微光只表达播放状态，减少动态效果时停止。document的捕获play监听在任意音视频开始时暂停其他播放器，卸载解除监听。字幕试听继续使用任务原声、现有句尾定时检查与迟到pause隔离，手动寻迹清除单句停止边界。

解码不参与声音输出，实际播放仍使用原audio文件。波形读取失败不阻塞播放；长文件首次波形需要等待整文件解码，播放无需等它完成。数据库仍为v17，详细依据、验证及长文件性能边界见[原声播放器验收](AUDIO-PLAYER-VERIFICATION.md)。

## 发布标签生成与带入（2026-11-04）

唯一全量生成Prompt升级为 `publishing-v7`，按项调整为 `publishing-revision-v6`。`PublishingDraft.tags` 必填，`parse_tags`在生成和标签调整两个接收边界共用：要求每个非空话题以ASCII `#` 开头，正文为Unicode字母/数字或下划线，以一个空格重新连接。`WritingContent::Publishing.tags` 使用serde默认空字符串兼容旧JSON，并沿用现有字段长度校验限制500字；人工编辑可清空。全量输出仍完整校验后才追加版本，未选择tags的调整保持原标签。

TypeScript `PublishingContent/emptyPublishing`同步该字符串；WritingPage仅向现有字段配置增加「发布标签」，共用编辑、复制、自动草稿、版本对照与按项预览。`PublishingField::Tags`进入既有流式只读文字投影，未完成片段不保存。无新状态、生成接口或Provider请求。

`PublishingPage.selectWriting`带入来源版本的tags，同时扩大原替换确认到已有标签；任务保存仍写既有 `publish_tasks.tags`。抖音页适配器的 `[description, tags].join('\n\n')`及宿主回读保持同一路径，已有一致内容继续时不重复插入。任务包 `tags.txt`与manifest沿用本次快照，不改源版本、官方最终发布操作或表结构。旧Prompt文件保留作为历史版本追溯资料，运行时只引用新版本。

## 书架十二套配色（2026-11-04）

`BookCard.vue`集中维护十二组只读的书封色/背景色，通过`project.id`的32位FNV-1a哈希对配色数组长度取模。计算属性只依赖ID，颜色以`--book-cover`、`--book-backdrop`传入现有书封样式；移除旧四套`tone-*`选择器和简单字符求和路径。统一米白文字，十二套文字对比度均高于5:1。网格和列表仍共用原组件，无新依赖、服务、持久化配置或数据库迁移；已有书籍会在采用新算法后重新分配一次颜色。

## P3流式候选预览（2026-11-04）

`OpenAiCompatible`可选正文观察回调复用原SSE接收与完整结束校验，约80ms合并预览并在结束时补齐。`ai/preview.rs`只读提取JSON中的完整/未完整字符串，正确处理转义和Unicode代理对，按现有字段枚举投影；音频引入与五部分指导仅用于正文预览。正式结果仍由原PublishingDraft/revision解析及业务校验决定，不用预览修复JSON或生成正式字段。

`execute_generation`接收可选`JavaScriptChannelId`，按调用方WebView创建任务Channel。前端`executeGeneration`将类型化文字快照交给当前候选区，结束清空回调；页面卸载和取消解除当前接收，旧任务迟到片段不进入新预览。无全局事件订阅、新任务状态机、持久化预览或额外模型请求，既有300秒请求总上限不变。原始JSON、推理内容、配色对象不发送给候选区。无新增依赖/权限，schema17。完整证据见[P3验收](P3-STREAMING-PREVIEW-VERIFICATION.md)。


## 工作台视觉与交互优化（2026-11-04）

共享主题与控件维护唯一材质和间距规范，移除被替代的深侧栏和双行导航规则。`BookCard.vue`统一网格/列表卡片与一套Reka UI DropdownMenu，主链接与更多按钮并列，避免嵌套交互。更多及键盘从按钮定位，右键通过公开reference属性从点击位置定位；保留12px窗口避让。Content使用as-child挂载组件自己的div，使scoped菜单外壳样式实际命中，不能依赖第三方多层转发保留作用域属性。`CopyButton.vue`与`useCopyFeedback`统一800ms状态及稳定按钮宽度。沿用锁定组件库、系统字体和现有业务服务，无新增依赖。

`project_progress`在线程池读取草稿、版本、封面文件校验、成片元信息及发布任务，运行任务优先，返回五项只读摘要；没有持久化完成状态。`ProjectNavigation`保留RouterLink、精确当前态和Tooltip，当前未保存编辑单独提示；App只在书籍主窗口注册Ctrl/Cmd+1～5，输入法组合、重复按键和模态交互不跳页。副作用在卸载时清理。

文案版本栏只维护预览选择，当前正式来源仍由原`selected`/草稿控制。当前编辑来源与最新历史不同时，即使正文未改，也沿用`editor_drafts`保存当前内容与`based_on`，离页或正常关闭后仍恢复当前稿；主动采用最新版本或正式另存后按原规则清除不再需要的草稿。恢复内容与来源版本相同时不提示未保存。备选生成追加原`writing_versions`，字段取用写入原草稿，保存仍追加版本。Provider继续单次流式接收、完整解析/校验/提交；保留真实等待反馈，P3只在候选区逐步显示正文，不将未完整片段写进业务字段。

`renderCover`在原排版计算中返回标题/副标题命中范围，DOM输入层直接编辑当前`CoverConfig`；预览、导出仍为一个Canvas渲染器。行内编辑与背景拖动互斥，输入层限制在可见舞台内；160ms预设过渡不改输出像素。

`VideoSubtitleEditor`在原单句播放器上增加连续播放、跟随与播放字幕索引，VideoPage按scene.first/last映射镜头；共享`revealInScroller`只滚动内部列表，不聚焦控件、不更新正式字幕时间码或每帧持久化位置。媒体暂停事件仅在播放器当前确已暂停时清除试听边界，忽略切句后迟到的旧暂停事件。

发布页一次明确核对后先验证选中PNG，再复用保存/任务包/辅助发布流程。元信息查询保留当前请求Promise，辅助接续在核验前等待同一查询完成；无音轨或读取失败继续阻止启动并保留输入，不额外请求或自动重试。`video_asset_metadata`及`export_video_asset`复用已有按书籍/素材/类型校验的`video_asset_path`，在线程池检查元信息；另存使用create_new拒绝覆盖，失败只清理本次新文件。新增能力仅授权主窗口，文件选择增加dialog:allow-save，未提供任意Shell或文件删除接口。

2026-11-04发布确认入口修复：自检卡显式说明标题和双封面通过同一次操作确认并保存。检查清单区分标题待填写与待确认，已填写标题和封面确认项的「去确认」定位到现有确认按钮；素材不齐时定位到确认说明，保留禁用与缺项提示。任务包项定位到导出操作。复用原确认字段，不增加确认状态或调用路径。

主窗口`onDragDropEvent`仅在书架接收单文件，将原始路径通过source查询参数带入原添加表单；保存复用`save_project`事务，取消不写库。侧栏显隐使用既有app_settings，数据库仍为v17。Vite预优化限定index.html，避免扫描本地验收输出。实现与运行边界见[本轮验收](UI-UX-REFINEMENT-VERIFICATION.md)。

视频进度区视觉收尾（2026-11-04）：`VideoJobProgress`直接使用共享主题变量，移除旧深色配色、局部渐变、装饰光晕和循环流光；进度环及共用`Progress`数值过渡为180ms，未知进度的等待指示与减少动态效果规则保留。没有新增进度状态或修改任务数据。Windows安装版三档真实缩放、原生另存及覆盖安装边界见[补充验收](WINDOWS-VISUAL-ACCEPTANCE.md)。

## 已选合集名称回读（2026-11-03）

官方合集选择值将名称放在 `selected-item-title-*`，作品数放在相邻 `selected-item-extra-text-*`，读取整个 `.semi-select-selection-text.textContent` 会把二者拼接，造成任务目标与网页选择不一致。`douyin_editor.js` 的合集动作统一从名称节点读取；已选值和列表选项共用同一名称提取函数，仍支持官方仅显示文本标签的状态，不靠正则删去名称的一部分。

宿主继续使用现有精确匹配、唯一选项和最终回读。已选目标一致时直接通过，不重新打开或选择；作品数变化不改变合集身份，不同合集仍需要原有明确决定。无会话、IPC、数据库或权限变化。公开页面构建依据及本地复现见 `DOUYIN-FLOW-VERIFICATION.md`。

## 抖音账号确认收敛（2026-11-03）

`douyin_flow.rs::drive` 在官方首页/上传控件加载状态下先完成固定上传页导航，再比较账号并等待唯一确认，不增加新入口或确认状态。`douyin_page.js` 对已观察到的 `https://p数字.douyinpic.com/aweme/宽x高/aweme-avatar/对象` 地址只以头像对象生成当前窗口指纹；CDN 域名、尺寸、查询参数和 CSS 候补图不构成换号证据，不同头像对象仍触发原有重新核对。

账号决定核对当前页面后，通过发布数据库服务校验并保存昵称至既有 `app_settings.douyin_account_name`，再沿用原有内存确认。等待账号时优先显示页面昵称；缺失时读取保存值。该值只作可编辑提示，不产生稳定账号 ID、不持久化确认、不授权合集或上传。复用现有 IPC、会话锁和备份，无新增依赖、权限或迁移；验证见 `DOUYIN-FLOW-VERIFICATION.md`。

## 续作、缓存备份与请求诊断（2026-11-03）

- `database/workflow.rs::workflow_snapshot`以一次SQL UNION读取video_jobs（含草稿更新时间）、audio_jobs、publish_tasks、活动ai_tasks、editor_drafts和文案/封面版本，排除归档书籍，按运行优先、时间倒序选每书续作目标并返回全部运行项。`useWorkflow`共享只读快照并合并并发刷新；`RunningTasks`每3秒刷新，失败保留上次快照并提示。`services/workflow.ts`统一状态文案及路由，视频用job、音频用audioJob、发布用task查询参数，不持久化第二份进度。
- 音频快照增加可选任务ID，后端验证任务属于本书；WritingPage初次读取和轮询均使用明确的audioJob，避免被另一本书的活动任务替代。切换明确任务仍经过原暂存与离页检查。单镜/插画AI任务没有视频任务外键，进入对应书籍视频页，由原视频页选择最近编辑任务。
- `database/storage.rs`扫描工作目录内普通文件，不跟随symlink或Windows reparse point。只有已知video_jobs对应`video/{job}/cache/`及`preview-{数字}/`内的未登记文件可清理；全库assets、导出封面及发布任务包路径受保护，全部`run-*`保留。目录链接及空目录不删除，保留渲染器既有引擎junction，清理后可继续生成。
- 预览返回相对路径、大小及排序文件清单（路径/大小/mtime）的SHA256令牌；确认时重新扫描并比对令牌，在同一数据库连接锁内检查运行状态与引用，逐文件复核解析后的绝对路径和父目录链接后unlink。路径或清单变化停止并要求重预览；文件删除失败返回已释放字节数。扫描/清理由spawn_blocking执行，仍持有数据库连接锁，大目录扫描期间其他数据库操作可能等待。设置页维护操作期间禁止离页和关闭。
- 成功备份复用`app_settings.last_successful_backup`，记录kind/path/completedAt；只有SQLite快照或完整目录写完才更新。恢复前自动安全快照不刷新用户备份时间。备份中的数据库快照早于成功记录更新，恢复后可能显示更早的成功时间。设置页只提醒，不增加自动备份调度。
- `OpenAiCompatible.observed(database, task, stage)`挂载请求级诊断，保持TextProvider/String、ImageProvider/bytes接口不变。`ai/diagnostics.rs::Call`进入请求时计时，`send().await`返回响应头时记录首响应，Provider完成时记录总耗时；总耗时含响应接收、解析，图片请求还包括base64解码和图片校验，不含后续业务存储/用户采用。Future被丢弃时Drop记录cancelled；启动及备份恢复将遗留running记录置interrupted，不编造中断时长。
- 文本请求发送`stream_options.include_usage=true`，SSE每个完整事件与普通JSON响应读取usage；图片JSON也读取实际usage。兼容prompt_tokens/input_tokens、completion_tokens/output_tokens，只保存有效非负整数；缺项保持NULL，不通过输入+输出补总数，不累加服务已返回的整次请求累计值。OpenAI官方[流式响应定义](https://developers.openai.com/api/reference/resources/chat/subresources/completions/streaming-events)已读取核对：最后一块可为空choices并带全请求usage，中断/取消可能收不到此块。兼容服务是否提供用量仍以其实际返回为准，不增加重试或协议回退。
- 连接测试、文案生成/调整/时长建议、封面/视频插画、字幕校对、分镜批次和单镜调整均挂载诊断。设置页读取最近200条，运行记录存在时轮询，支持手动刷新。新表和恢复规则见[数据库](DATABASE.md)，受限主窗口命令在原projects权限登记，不新增依赖、网络权限或供应商。

验收及真实Windows边界见[剩余三项验收](CREATOR-COMPLETION-VERIFICATION.md)。

## 单镜候选生成（2026-11-03）

- `VideoSceneEditor`只发出revise事件，`VideoPage`排空现有草稿队列后冻结jobId/runId/字幕/完整分镜编辑快照。页面只挂载一个`VideoSceneRevision`弹窗，生成及对照期间阻止背后的编辑、任务切换和离页；弹窗复用原AlertDialog、Select、Textarea，正文滚动，操作区保留可见。
- `start_video_scene_revision`复用ai_tasks建立`video_scene_revision`记录。input_json记录当前编辑快照及正式plan的SHA256；输入最大4MiB，要求有效字幕/完整分镜、固定镜头数量/范围、有效文本模型。不会保存video_jobs或清空分镜。
- `execute_video_scene_revision`复用`ai_commands::run_task`、TextProvider、OpenAiCompatible的流式接收、300秒上限和cancel_generation。准备和结果返回时检查视频可编辑、run_id及正式plan摘要，取消/过期/失败不交付候选。记录仅表明模型请求成功，不能表示候选已采用。
- `video/revision.rs`复用v5整批分镜规则和唯一目录，追加`video-scene-revision-v1`的单镜范围约束；模型只接收本镜、相邻镜头/字幕与修改要求，不发送全片字幕、音频、图片路径/素材ID/来源备注。结果必须为单元素数组，固定first/last，禁止image/labels/额外字段；继续复用validate_scenes和首beat锚点规则。图片镜头由宿主恢复原image，禁止新增beats。
- 候选只在弹窗暂存，不建立新的计划存储。采用时再次比对前端快照，仅赋值`scenes[index]`，触发已有自动草稿与试片失效；正式保存不变。改要求或换模型会清除旧候选；未采用候选关闭即放弃，已采用的内容按现有草稿恢复。
- 无SQL迁移、新依赖或新渲染路径。主窗口视频权限新增start/execute两条命令；停止继续使用原AI任务权限。当前RekaUI2.11.0的AlertDialogContent、DialogContent和Teleport源码已核对，浮层复用原根挂载与焦点返回行为。实际证据见[单镜验收](VIDEO-SCENE-REVISION-VERIFICATION.md)。

## 视频运行环境检查（2026-11-03）

- `VideoEnvironmentCheck`嵌入原配置区，使用相同VideoSettings，不增加配置持久化路径；service调用主窗口限定的`check_video_environment`。检查结果仅保存在当前页面，输入/用途变化后失效，应用检测路径与保存均由用户显式操作。
- `video/environment.rs`只补齐空路径：Node/Python查PATH（排除WindowsApps启动占位符），引擎查工作目录及当前目录附近的已知独立工程布局，Python兼顾`.tools/video-runtime`和引擎旁runtime，模型从虚拟环境所属目录查model-small/video-model-small。不扫描磁盘，不强行覆盖显式错误路径。
- 每次检查在配置工作区创建独立`.video-check-*`临时目录，复用`video_process::run`的Windows Job Object、隐藏子进程、输出文件和45秒阶段上限。Node和Python分别探测，单项失败继续返回其余结果；已有字幕完全跳过Python进程。只清理本次创建的检查目录。
- `video/runtime.mjs`为渲染与检查共用的Node中文路径保护、引擎0.7.3识别；实际渲染每代复制此模块。检查从引擎当前`tools/bins.mjs`读取FFMPEG/findChrome，执行FFmpeg版本命令、通过引擎自身Playwright启动浏览器；从`tools/project.mjs`读取实际字体目录，并在浏览器加载boot.js所需的四组字体。不会因系统另有无关FFmpeg而误报就绪。
- `transcribe.py`复用语音依赖版本及模型四文件校验；检查模式验证faster-whisper/av可导入，并以CPU/int8/local_files_only加载实际模型，不执行转写。原转写时间码、进度及输出流程不变。依据已安装Frametell0.7.3的README、package.json、bins/project/lib/boot源码，以及当前锁定Playwright1.61.1实际API实现验证；无新增依赖。
- schema16不变，无新表、列或SQL迁移；本次检查不产生业务任务、草稿或成片。证据见[环境检查验收](VIDEO-ENVIRONMENT-VERIFICATION.md)。

## 视频编辑恢复与对照（2026-11-03）

- 复用`useEditorDraft`的400ms防抖和串行队列，`DraftKind=video`通过`read_video_draft/save_video_draft`以任务ID读写；存储位于`video_jobs.editor_draft_json`，不扩展文案/封面草稿的项目唯一键。字幕存字符串，分镜使用已有缓存JSON快照，位置保存section/index/onlyChanges/cursor；暂存不执行正式字幕/分镜校验，允许未完成SRT。最大4MiB。
- 暂存的basedOn为run_id，运行中/已完成及过期代次拒绝写入。正式保存和新一轮任务认领分别在原UPDATE内清除旧草稿；页面先排空写入，再暂停自动暂存并执行正式操作。失败保留编辑，成功保存后可另存位置。加载时按draftUpdatedAt选最近编辑任务，显式job路由优先；读取失败不展示其他任务的编辑缓冲。
- 新转写/导入结果在AI校对前冻结为`original_cues_json`，后续校对、保存和重新生成不改原文。旧任务只读取本任务已登记run目录中最早的`transcript.srt`，目录按Windows规范化路径比较；无该记录时使用任务明确的source_subtitle_id。原文件不可读时显示错误，不使用校对文补齐。
- `VideoSubtitleEditor`共用页面SRT字符串，按正文偏移替换单句，保留原编号、时间码和换行。原文以毫秒起止时间对应，无法完整解析的SRT仅在全文入口编辑；正式保存仍以Rust校验为准。按页最多渲染20句。播放源绑定selected.audioAssetId，与新建任务的音频选择独立；以媒体currentTime在句尾暂停，切句、任务切换和卸载停止播放，不使用墙钟时长推测缓冲进度。
- 无新依赖，不更改模型调用、音频、渲染器或发布流程。主窗口视频权限仅增加任务草稿读写与原字幕读取三条命令。验证证据见[视频草稿与字幕对照验收](VIDEO-DRAFTS-VERIFICATION.md)。

## 字幕输入性能与生成时机（2026-11-03）

根因：`VideoPage`的SRT双向绑定使每次输入都重新执行全页渲染，原内联分镜循环为每镜重新创建控件和动态插槽；dirty检查还将全文SRT和全部响应式scenes一起JSON序列化。字幕数量与镜数越多，输入期间重复工作越多。64条/12镜的性能回归复现两次输入触发145次下拉更新（含试片选择器）。

将原分镜卡片与其局部样式移入`VideoSceneEditor`，通过Vue `defineModel('scene')`继续编辑父页同一份Scene；字幕字符串不传给卡片，传入的cues/素材/模型引用和禁用状态保持稳定。`sceneSnapshot`作为computed仅随分镜内容变化重新序列化，dirty仍同步比较当前字幕和分镜快照。不新增草稿、定时保存、输入防抖或另一份字幕状态；保存/试片/重建/确认均读取当前值。依据当前Vue3.5.44的shouldUpdateComponent/useModel/computed源码及官方[组件更新优化](https://vuejs.org/guide/best-practices/performance.html#props-stability)、[组件v-model](https://vuejs.org/guide/components/v-model.html)。

大规模验证还确认原物体折叠区的`unmountOnHide=false`始终挂载全部选择器。当前锁定RekaUI2.11.0的`CollapsibleRoot.vue`/`CollapsibleContent.vue`公开类型与实现表明默认true只在展开时创建slot；`SelectContent.vue`则会把关闭的选项挂载进DocumentFragment，因而不能靠下拉菜单未打开来省去选项成本。移除本地折叠区的强制保留配置，复用默认按需挂载；物体、动作和cue全部在同一Scene中，收起再展开从模型回显，不丢输入，也不新增挂载标记或缓存状态。

首主体偏晚的生成根因是v4只要求cue语义对应且位于镜内，没有要求从开镜台词选取已成立的主体；渲染器照原cue执行，没有额外等待。唯一生成入口升级为`video-storyboard-v5`，要求每镜首beat.cue=first；先选中性主体，不能提前后文的结论/否定。将准备流程原来的JSON解析、结构/目录校验和禁止模型选图片收敛到`parse_storyboard`，在同一接收边界增加首主体锚点校验，通过整批才保存。违反时保留字幕、此前完整分镜，不自动改写或重试。一般`validate_scenes`继续用于已保存/人工分镜，不套用生成专属规则，不重写旧任务。其他物体、动作、首条字幕前原声停顿和原有渲染时钟保持。

v5同步明确1.2秒相邻出场和12秒长停留的现有提示阈值，仍是语义导演建议，不作为人工分镜的硬错误。无数据库/IPC/权限/依赖变化。验证与边界见[字幕编辑与分镜时机验收](VIDEO-EDITING-VERIFICATION.md)。

## 原声字幕校对（2026-11-02）

`video/subtitles.rs`与独立版本化`subtitle-proofread-v1.md`定义校对契约：每批60条，前后各6条只作上下文，输入包含全局字幕索引、原时间码与冻结首尾参考；模型只返回完整有序的`[{index,text}]`。宿主严格拒绝漏条、重复、乱序、越界、额外字段、时间码、空文本、控制符和超过32字的条目，整批通过才替换文字并推进completed。时间码和条数始终由已有cue持有，不接受模型重排或重切。

`video_commands::execute`在识别/读取SRT与分镜之间接入proofreading阶段，复用TextProvider、当前选择的文本模型及推理配置、流式接收、300秒单次上限和现有取消标记；同一运行校对/分镜使用同一配置。识别结果先保存并独立登记`transcript.srt`；每批结果与completed在同一次plan_json更新中保存，完整后另存并登记`proofread.srt`，再开始分镜。失败不自动重试，旧run_id不能落库。续作从completed开始，全部完成时不再调用校对。

首尾参考优先通过本书audio_asset_id找到audio_jobs.input_json.prompt；无关联音频任务时读取本书最新publishing版本的audioPrompt。只提取明确的开场/固定开场与结尾/固定结尾Markdown段落，不把整份指导当逐字稿；无段落则留空。首次校对将书名、已填作者和首尾参考保存到plan.subtitleReview，后续继续不受新文案影响。不会上传音频、原书文件或正文指导。

已有分镜且无subtitleReview的旧任务续作保留原内容；明确保存并清空分镜后，再启动全文校对。人工更改字幕或重新生成分镜会清除旧校对游标，避免把新稿当作已校对；仅调整画面保持游标。字幕校对进度和最近三条已校对文字直接来自同一plan，沿用五步任务卡的原声字幕步骤，不增加轮询器、调度器、依赖或权限。新旧记录读取及回退边界见DATABASE。

## 文本流式响应与分镜失败阶段（2026-11-02）

排查确认：TextProvider原先固定stream=false；视频已有cues时跳过转写分支，也跳过其中唯一的首次storyboard状态更新，导致首批模型失败被记为preparing。截图524只能证明网关超时，当前本机无对应644条字幕任务及当时模型配置，未确认真实上游耗时或推理强度。

统一OpenAiCompatible发送stream=true，公共post_response保留认证、禁止重定向、禁自动重试及HTTP错误脱敏；图片继续原JSON协议。文本按Content-Type解析SSE或服务实际返回的完整JSON，不发送第二次请求。ai/stream.rs按完整SSE行/事件读取，支持跨HTTP分块的UTF-8、LF/CRLF、多行data、心跳及usage；仅收集index=0的delta.content，忽略推理字段。finish_reason=stop且收到[DONE]、正文非空才返回，断流、length、服务内嵌错误及非法数据不交给保存层。流总量限制16MB（含逐token信封及推理），正文2MB，整个请求300秒截止时间不随心跳延长；取消仍通过现有调用方丢弃请求future实现。

register_video_subtitle在登记本轮字幕的原事务中同步stage=storyboard，使首次生成和字幕续作在调用模型前具有一致阶段，无需新增状态、迁移或重复存储计划。现有分批及保存规则保持。

协议依据：[OpenAI Chat流式事件](https://developers.openai.com/api/reference/resources/chat/subresources/completions/streaming-events.md)；当前锁定reqwest 0.13.5源码async_impl/response.rs的chunk实现及client.rs的timeout文档确认可逐块读取，且timeout覆盖连接至响应体完成。没有新增库、自动重试、模型降档或付费调用。流式不能保证消除首包超时及服务商缓冲造成的524，真实服务仍需复测。

## 视频关键图片与插画（2026-11-02，P2）

`Scene.image`为可选的本次使用快照：`assetId/source/rights/creator?/fit/x/y/sha256`。fit只允许contain/cover，x/y为0—1；图片存在时beats必须为空。缺字段的旧物体分镜仍走原绘图，旧文字分镜仍需明确升级；文本模型继续只生成目录物体，图片由用户选用。`VideoImageEditor`直接编辑现有Scene，不增加第二份视频计划；生成候选在主动采用前保持独立。

Rust在保存校对时按本书assets解析图片，复用ImageProvider静态格式/尺寸/完整解码校验，冻结SHA-256。试片/整片启动后将已校验字节写入本次目录；Node只根据宿主映射复制到`assets/{sha256}.{真实扩展名}`，编译器不从模型或计划读取路径/URL。图片摘要、来源、裁切继续随整个plan参与原缓存键与预览签名。读取旧试片及归档前再次核对源摘要，缺失或变化即失效，不用旧缓存替代当前图片；运行中禁止移除/重新定位被引用图片。

锁定Frametell0.7.3的`runtime/media.js`已提供`P.img`、fit/pos和预加载。直接使用同一绘图中的受控图片层，关闭图片镜头的场景/景别运动，HUD标题和来源、引擎字幕独立固定。书引在原`window.__ready`中明确等待每张使用图片decode成功，再等待首轮绘制；引擎原本仅记录图片加载错误后继续的行为不作为成功依据。图片与预览沿用原cue/scene时钟、严格布局/确定性检查、任务取消和正式MP4/SRT事务。

视频插画使用独立`video-illustration-v1` Prompt与`ai_tasks.kind=video_illustration`，复用既有ImageProvider、系统凭据、单张请求、180秒图片超时及run_task取消；封面与视频图片共用内部生成/落盘实现，封面命令和历史用途保留。生成结果写本书`video/illustrations`并登记assets，失败补偿删除仅限本次新文件，显式采用才改变分镜。不增加供应商SDK、表或SQL迁移；四个新增主窗口命令只处理本书图片/任务，图片预览经有界二进制IPC创建blob，CSP仅img-src新增blob来源，远程窗口无这些权限。证据见 [P2验收](VIDEO-IMAGES-VERIFICATION.md)。

## 视频编码实时进度（2026-11-02）

根因：原`rendering_subtitles`只在启动Frametell `--skip-video`前写一次快照，锁定引擎0.7.3内部用`-loglevel error`启动FFmpeg，没有返回编码位置。因此已有帧进度到字幕阶段便退回不定进度。

书引调用层继续使用原`--skip-subs`绘制与引擎`tools/subs.mjs`生成字幕图层，随后只保留一个由书引启动的字幕合成FFmpeg调用；滤镜、libx264/veryfast/CRF16、24fps、yuv420p和faststart与该版本源码一致。原`--skip-video`调用被替换，不修改独立引擎，不新增绘图/字幕实现。依据为本机0.7.3的`tools/render.mjs`、`tools/subs.mjs`、README及FFmpeg `-h full`的`-progress`/`-stats_period`接口。

字幕合成和原声合入均使用`-progress pipe:1 -stats_period 1 -nostats`，按完整`progress`块发布`out_time_us`转换的秒数及有效`speed`。位置单调且不超过当前时窗，成功退出后该阶段计满，失败保留原错误。准备字幕每秒统计本轮时间戳内的`cue-*.png`，排除空图及旧运行文件；原画面片段统计不变。试片使用局部字幕时窗及相同原声裁取。

实跑同时复现Windows原子替换进度文件偶发`EPERM`：即使无额外轮询，密集替换也可触发；未推测具体占用进程。`writeProgress`只对Windows替换快照时的EPERM/EBUSY保留上次完整快照，等待下一次正常上报，不循环重试、不生成另一份任务状态；其他写入错误、编码错误和非零退出照常失败。并发读写测试校验每次读取均为完整JSON且释放占用后更新继续，避免辅助进度错误中止有效视频。

既有`VideoProgress`侧车快照和任务视图增加可选`unit`、`speed`及`updatedAt`，不写SQLite，不新增IPC、权限、依赖或任务状态。前端只在阶段与单位一致时计算百分比，从原快照派生步骤、速度与反馈年龄，复用Progress组件与现有计时器。15秒无反馈或同步失败时隐藏速度、停止装饰动效；减少动态效果关闭流光与过渡。最终完成仍由原校验/归档事务决定。命令、实测数据及验收边界见 [渲染反馈验收](VIDEO-RENDER-FEEDBACK-VERIFICATION.md)。

## 基础交互组件收敛（2026-11-02）

- 在现有 `components/ui` 增加 AlertDialog 与 Progress，沿用 shadcn-vue 源码分发模式和已锁定的 Reka UI 2.11.0。参考当前官方 [AlertDialog registry](https://www.shadcn-vue.com/r/styles/new-york-v4/alert-dialog.json)、[Progress registry](https://www.shadcn-vue.com/r/styles/new-york-v4/progress.json) 及本地依赖源码；只保留实际使用的导出，不增加依赖。
- `ConfirmAction` 是确认弹窗的共同入口，内容通过 Portal 挂载到 body，焦点限制在弹窗内，默认聚焦取消，处理中禁止退出和重复确认。表单确认通过 `form` 属性关联真实表单，保留原生校验；确认按钮不自动关闭，只有业务成功才关闭，因此失败不丢字段。
- 2026-11-04发布记录弹窗优化：`ConfirmAction`关闭自动属性继承，将调用方属性直接传给`AlertDialogContent`，使页面专属样式命中实际浮层。`PublishingPage`使用最大600px白色弹窗、20px字段间距和独立浅绿下一篇区域；选择框全宽，菜单使用当前Reka源码公开的`--reka-select-trigger-width`与触发器同宽，长选项换行。接续按钮通过原`actions`插槽进入带分隔线的底栏，使用次级描边，窄窗口纵向排列。两种保存均使用同一表单的原生校验；接续按钮先调用所属表单`reportValidity()`再复用`recordSuccess(true)`，回车仍只保存本次记录。原记录、忙碌锁定、失败保留和成功后接续调用链继续复用，不新增业务状态、依赖或SQL迁移。实际验证见`DEVELOPMENT-PLAN.md`的“发布记录弹窗样式优化”。
- `requestConfirmation()` 返回 Promise，`AppConfirmation` 在应用根部唯一挂载。所有原生确认调用及其路由/历史切换调用链改为等待结果；并发请求直接取消后来的操作，应用卸载按取消结束待确认请求。下拉框触发的确认显式指定稳定触发器 ID，避免选项卸载后焦点丢失。浏览器 `beforeunload` 的系统级关闭保护继续保留。
- 视频/合集折叠区复用 Collapsible，设置 `unmountOnHide=false` 保留输入；运行环境使用 `defaultOpen` 按首次加载的配置决定展开，不新增重复展开状态。封面与试片卡片使用 Button，保留业务布局，删除重复焦点样式。
- Progress 仅接受当前阶段的百分比或空值；空值显示不定进度，ARIA 不提供数值；确定进度与减少动态效果规则在基础组件内统一。视频环形阶段展示保留业务职责，直线进度不再维护另一份动画实现。

不涉及 Rust、IPC、capability、数据库、版本号或包依赖变更。验证见 `UI-COMPONENTS-VERIFICATION.md` 的 2026-11-02 补齐记录。

## 文本推理强度配置（2026-11-02）

`ProviderInput.reasoningEffort` 为可空的七档字符串，前端沿用现有 Select、保存、未保存提醒和连接测试；Rust保存时校验档位，图片用途统一清空。`database/ai.rs` 将其保存为 `model_providers.reasoning_effort`，v15增量迁移不改历史配置的值或时间戳。

唯一 `ai_commands::request` 将配置传入 `TextRequest.reasoning_effort`，全量/按项生成、时长推荐、连接测试及 `video_commands` 的分镜批次都复用该入口。`OpenAiCompatible::generate` 仅在显式配置时向 `/chat/completions` 增加 `reasoning_effort`，空值完全省略；不附加temperature等其他可选参数，不推断能力、不在失败后降档或重试。视频每次运行仍冻结批处理前读取的配置。

协议按当前 [OpenAI Docs Chat Completions](https://developers.openai.com/api/reference/python/resources/chat/subresources/completions/methods/create) 核对：none/minimal/low/medium/high/xhigh/max，不同模型支持范围不同。本地契约测试验证请求字段与HTTP错误，不能证明第三方网关遵从或真实模型质量。无新增依赖、Command或权限；统一300秒超时不变，修正模型页原120秒提示。

## 轻场景、景别与内容扩展（2026-11-02，P1）

唯一`visual-catalog.json`新增`environments`（plain及6场景）、`framings`（reference/wide/medium/detail）和8项物体，共93项；13构图/6动作/3主体限制不变。`Scene`仅增加`environment`/`framing`字符串字段，Rust缺字段分别默认为plain/reference并按目录校验，显式null/非法键/未知字段拒绝。Node使用同一白名单，编译到原PAGES，不改cue、beat.at、drawDuration或原声。旧记录只读时不重写JSON，现有保存与备份恢复直接承载新字段。

`illustrations.js`同一`sketch`增加8个case，原85项几何及primitive保留原SHA基准；背景是一个低对比结构层，不调用sketch生成额外叙事物体。路径环境复用原route路径常量，避免两条路线重叠。新景别调整既有摆位/主体比例，reference走同一函数的既有摆位。非reference的route/transfer箭头端点随主体比例留白，wall隔墙取最后两个主体边缘之间的中点；单主体则放在主体右缘与画面右界之间。reference保留原辅助线坐标，避免特写三主体错指或隔墙穿过物体。全体主体仍按字幕触发，背景可预先出现。

锁定Frametell 0.7.3的`Builder.cam`/独立`hud`经本机源码核对：仅detail+focus且首beat后有至少3秒时，生成一次`{at: firstBeat.at, x:960, y:540, s:1.06, dur:3}`；其他组合静止，不开放camera字段。标题、系列信息、页号和进度线使用HUD，字幕仍由引擎固定层负责。镜头时钟使用原scene，窗口截短不会加速或取消源镜头动作。2493组实测发现特写叠加倾斜会碰到标题/标签，因此主比例收敛到1.2，次物体标签按尺寸留白；没有提高校验阈值。

生成入口唯一替换为`video-storyboard-v4`，不并留v3生成路径。Vue继续使用原scenes状态，新选择立即触发dirty/预览失效；未选择时只显示默认值，不在adopt中改写历史。重复提醒加入环境/景别的归一化键，语义变化不再误报完全重复。任务、IPC、Provider、权限、依赖、缓存键机制和SQL结构不变。验证见 [P1验收](VIDEO-SCENES-VERIFICATION.md)。

## Windows 视频运行时检查（2026-11-02）

原任务在 Frametell `--plan` 输出完成后清理 `out/.render.lock` 时，触发 Node24.11.0 原生 `fs.rmSync` 的非ASCII路径缺陷，以0xC0000409退出；不是模型超时或FFmpeg编码失败。依据为原中文目录复现、最小文件删除对照及 [Node官方修复61108](https://github.com/nodejs/node/pull/61108)。24.x首次修复于24.13.1，25.x于25.4.0，视频运行环境推荐24.22.2 LTS。

既有Node入口在宿主启动门打开后、读取任务/加载引擎/探测音频前检查实际运行版本，Windows拒绝已知受影响的23.x、24.0—24.13.0和25.0—25.3.x，明确指出「视频运行环境 → Node 程序」的修复入口。其他平台不新增此限制，不引入版本库或自动替换系统Node。修复方案是选择正常运行时，不修改外部引擎、重写中文路径或忽略非零退出码。

`--plan`的机器JSON仍用于真实帧进度，但不再回显到日志；普通诊断stdout和stderr继续保留。既有任务状态、字幕/分镜、原声和人工确认不变，无迁移、新调度器或额外重试。实际原任务复验及安装边界见 [MG视频验收](MG-VIDEO-VERIFICATION.md)。

## 导演与派生试片（2026-11-02，P0）

现有JSON分镜形状不变，生成入口统一替换为`video-storyboard-v3`，有界24条上限内优先在末6条的完整句尾切批；不增加模型调用层或任意脚本。`storyboard.ts`从当前scenes派生观看提醒，Vue复用一份编辑状态；物体/动作/构图按唯一目录选择，锚点选择当前镜内字幕，保存仍由Rust严格校验。

Node的`buildEpisode(plan, range?)`生成同一绘图工程：源cue ID、原场景序号、`beat.at`及`drawDuration`保持，窗口只平移显示字幕、截断输出时长；引擎index使用窗口内连续索引。原声按相同窗口裁取合入AAC，不重新ASR/TTS、不调整响度。单物体居中、叙事主次和动作显式触发共用同一draw，85项sketch几何不变；不保留旧版渲染器。第0帧使用真实时间0的cover，避免引擎默认插入后续“封面帧”泄露结果。布局/确定性检查继续严格执行。

`VideoWork`在原调度器中区分准备、完整渲染和预览；`preview_video_job`只接受起始镜号与30/60秒，复用视频运行锁、run_id、Job Object及进度。预览运行写入`preview-{run_id}`，结束回到awaiting_confirmation/stage=previewing，禁止进入正式素材finish事务。`list_video_jobs`仅在当前阶段/代次、计划+源码签名和文件都一致时派生preview；输出清单及缩略图属于可重建缓存，不成为第二份权威业务状态。

`video_preview_media`校验本书任务、run_id及清单中的有限镜号，只授权该运行目录的MP4/PNG单文件；默认asset scope仍为空。主窗口原allow-video增加这两个受限命令，CSP的img-src允许受限asset协议以显示缩略图，不向远端页面增加权限。保存成功立即清空前端预览视图，避免下一次启动失败时重新显示旧结果。预览缓存不跟随备份；恢复后按缺失处理，不承诺旧程序读取previewing新阶段。实现与证据见 [P0验收](VIDEO-DIRECTOR-VERIFICATION.md)。

## 同款物体分镜与绘图（2026-11-01）

使用单一 `video/visual-catalog.json` 定义物体、构图、动作及长度限制，Rust、版本化 `video-storyboard-v2` Prompt、Node和Vue共用。Scene保留first/last/title/kind，增加beats（cue/object/action/label）；Rust验证连续覆盖、1—3物体、锚点在本镜且有序、目录白名单和短文本。每批24条字幕附首尾台词、最近3镜和后续3条上下文，不能将上下文纳入本批覆盖。

`video/illustrations.js`复用参考读书成片的85物体几何、13构图、原声驱动移动/强调及字幕CSS；不是复制AGPL引擎。Node只把验证后的JSON编译为script，固定页面调用同一EPISODE.draw，旧文字框绘制已删除；模型不能提供代码。绘图、目录、编译器和plan共同参与缓存键，Rust内嵌并写出全部运行脚本，不依赖开发机原影片路径。中文META对应锁定引擎的`film-有字幕.mp4`输出契约。

旧plan缺beats时仍可读取、播放/发布已有成片，但不能继续旧模板渲染。未完成任务经原save_video_plan保存字幕并清空分镜，再走原准备/确认流程；已完成任务通过原start_video_job及原音频/SRT创建新版本，不修改旧任务。没有第二套渲染器、新调度器或SQL迁移。运行结果与画面对照见 [验收记录](MG-VIDEO-VERIFICATION.md)。

## 应用内 MG 视频（2026-11-01）

`video.rs` 定义字幕/分镜契约，`database/video.rs` 管理 v14 任务与素材事务，`video_commands.rs` 调度独立 ASR/渲染和现有 TextProvider。`video_process.rs` 通过带启动门的 Windows Job Object 管理子进程，取消/退出不遗留渲染树；继续及重新生成分镜通过IPC显式传入当前providerId，claim_video_job先校验已启用文本模型，再将provider_id与运行状态/代次在同一UPDATE中更新；校验或启动冲突失败不改旧绑定。渲染及试片传空值保留绑定，继续准备不改已保存plan。同一运行的分镜模型配置在批处理前读取，后续配置编辑不切换该轮请求端点。Python 转写只读取完整的本地模型；Node 编译受控物体分镜，不执行模型提供的代码，浏览器与Node共用绝对原声时间轴。任务保存已完成分镜批次，渲染缓存按计划、绘图源码、目录和引擎路径隔离。

前端增加 VideoPage 和音频入口，沿用原素材/发布流程；新草稿按明确的 videoJob 选择对应视频及原音频，历史任务不变。Tauri protocol-asset 默认空 scope，主窗口经同项目素材ID校验后只允许单个媒体文件，复用其Range实现。运行时是独立安装工具，不捆绑AGPL源码。具体目录、源码/许可依据、版本锁定、迁移与实跑边界见 [实施与验收](MG-VIDEO-VERIFICATION.md)。

进度仍由同一调度器管理：`VideoRuntime` 记录当前任务ID、run_id、启动时间与本轮目录，`list_video_jobs` 将匹配的 `progress.json` 合入现有任务视图。Python 每个已识别语段原子替换快照，最多附三条字幕；Node 使用锁定引擎的 `--plan` 取得真实帧区间，每秒统计正式缓存及本轮已封口的临时MP4，只读取顶层盒头、不扫描视频数据。临时片段须有完整moov且写入时间不早于本轮启动，取消遗留片段不冒充新进度。原引擎 `--skip-subs` 负责绘图，`tools/subs.mjs`生成相同字幕图层，书引用原滤镜合成并读取FFmpeg进度，随后合入原声并校验；未复制渲染算法。细节见本文「视频编码实时进度」。

`VideoJobProgress` 仅从当前任务/进度派生状态，不建立第二份任务缓存；沿用1.5秒轮询并提示临时同步失败，组件销毁或任务结束时清理计时器。转写的 completed/total 单位为秒，rendering_frames 为已完成片段的帧数，分镜比例来自已保存字幕覆盖；准备字幕以条数计，字幕合成及原声合入以编码视频秒数计，其他无法量化的阶段保持不定进度。侧车快照不进入SQLite、素材或备份，不新增迁移、Tauri权限或依赖；原声和已保存成果仍由原流程管理。

浏览器的引擎脚本和依赖入口使用规范 file URL，并转义HTML属性，不经深层缓存junction访问字体；解决本机262字符字体别名在Chrome中报ERR_FILE_NOT_FOUND、真实路径可读的问题。若openStage初始化抛错，独立Node入口明确退出1，避免仅设置exitCode后遗留浏览器连接让任务一直running；Rust沿用原Job Object回收进程树并记录失败，不增加重试或另一套调度。

生成页面在引擎boot之后，将原 `window.__ready` 串联两次 `requestAnimationFrame`，保留时长返回值及初始化错误。引擎原就绪信号只覆盖DOM/字体，首次绘制未完成时可能把空白底色与正常画面作确定性比较；统一页面就绪边界后，布局检查与所有编码工作进程都等待初始绘制，不另设固定延时或逐帧等待。继续执行完整布局/确定性检查，不放宽阈值、不重试掩盖失败。源码变更自然更新既有缓存键；已保存字幕/分镜和原声不变，重试仍走确认并生成视频。复现、实际原计划验证与安装边界见 [MG视频验收](MG-VIDEO-VERIFICATION.md) 的首帧一致性修复章节。

## 音频固定口播模板（2026-11-01）

全量 `publishing-v6` 与按项 `publishing-revision-v5` 共用 `AudioDraft::into_prompt` 的 `audio-template-v3`。`audio.seriesName` 固定为「读懂一本书」；目标秒数、字数和分段计算不变，仅用于不朗读的制作参数。固定开场结尾独立于分钟数并声明逐字朗读，模型生成的引入/正文不重复固定段或时长承诺。提示词遵从性不等同于输入回读校验，也不等同于归档成功。

不增加重生成接口：成功任务不占活动任务名额，保存新指导后复用 `start_audio_job` 新建冻结输入和任务，按任务 ID 归档到新路径。旧任务/文件、未选中字段与发布关联不自动改变。无依赖或 SQL 迁移，具体取舍、生效边界和验证见 `AUDIO-PROMPT-REGENERATION.md`；后文旧模板说明为历史记录。

## 多尺寸封面工作区（2026-11-01）

`types/cover.ts` 定义 `CoverWorkspace`（schemaVersion=2、activeSize、三份 CoverConfig）；页面只维护这份工作区，当前表单通过 computed 指向选中配置。背景解码按素材 ID 复用，画布与导出均使用对应尺寸的配置/背景，仍只有一个 Canvas 渲染器。历史按尺寸取最新版本；旧单份草稿展开为三份独立配置。加载历史只替换其所属尺寸。

自动保存复用 `useEditorDraft` 的串行队列。封面启用 retainSavedDraft：即使配置已保存为正式历史，也保留完整工作区和当前尺寸；内容创作原有保存后清理草稿行为不变。保存全部配置依次追加三条历史；单张导出只追加当前尺寸，其他编辑继续暂存。批量导出先读取所有所需背景、渲染全部 PNG，再写版本；部分写入失败明确保留已完成版本和全部编辑，不宣称全部成功。其他尺寸的背景缺失不阻止有效尺寸单张导出。

文本协议为 `publishing-v5` / `publishing-revision-v4`，增加 coverPromptWidescreen；全量要求三份非空、不同且标明各自比例的提示词，单项修改沿用候选预览/采用。旧 WritingContent 缺失字段按空字符串读取。CoverConfig 新增 backgroundPrompt，旧配置默认为空。

图片任务冻结 width/height，Rust 只接受三种封面尺寸；`cover-background-v2` 以指定比例独立构图，不再要求方形兼容全部尺寸。现有 GPT Image 适配器按方向请求 1536x1024 / 1024x1536，本地裁切为准确目标尺寸；不猜测第三方自定义尺寸、不静默重试其他协议。服务不支持时返回原错误并保留配置。接口依据：[OpenAI Images Generate](https://developers.openai.com/api/reference/resources/images/methods/generate?site_locale=en)（本轮已读取官方当前页面）；兼容供应商实际支持情况需单独验收。

无新增依赖、Command、权限或 SQL 表/列。此前“同一配置改变尺寸”的运行路径已替换，旧版本 Prompt 仅作为历史保留。验证见 `COVER-INDEPENDENT-VERIFICATION.md`。

## 音频表单与路径展示（2026-11-01）

音频来源/格式沿用现有控件与品牌色，以字段区域内边距和明确的标签、控件、说明间距替代负边距；980px 以下单列，长文件名省略并可悬停查看，完整路径和说明自然换行。说明通过 `aria-describedby` 关联控件。

`src/lib/displayPath.ts` 是前端唯一的 Windows 路径展示格式化入口：盘符路径去掉扩展前缀，扩展 UNC 转为普通网络共享路径。书籍、素材、音频、发布、封面、备份/恢复、验证页及提示消息共用；素材搜索匹配显示路径。只转换已识别的盘符/UNC 前缀，不改变其他平台路径、设备路径或路径主体。不修改 DTO、数据库、原始选择值、文件身份和文件操作参数；Rust 的原生 shell 路径转换保持原用途。无迁移和依赖变化，SQLite 仍 v13。验证见 `WORKSPACE-UI-UPGRADE.md`。

## 书籍来源绑定（2026-10-28）

`ProjectPage.vue` 暂存新增文档路径，选择不写库，已保存来源由 `list_assets` 读取。`save_project` 接收可选 `sourcePaths`，`Database::save_project_with_documents` 将书籍写入与既有 `assets::register_existing` 的 document 关联放在单个 SQLite 事务；只读打开校验可读性，任一失败回滚，创建失败仅清理本次空工作目录。未传文件仍走同一保存实现。音频页沿用 `setAssets` 的可用文档优先选择及 asset ID 来源契约，不新增来源表、权限、依赖或文件上传路径。数据库保持 v13。

## 工作台交互与视觉统一（2026-10-28）

- 续轮将书架的查询状态提取为 `useQueryState`，复用于创作/封面 `tab` 和发布队列 `q/status`；任务定位使用实际 `task` ID，新任务使用 `task=new`，内部保存同步 URL 时跳过重复读取，避免覆盖刚保存的编辑。
- `ConfirmAction` 复用现有行内确认布局，非模态、取消优先焦点、Escape 及发起位置焦点恢复；不新增对话框依赖。`useWindowCloseGuard` 为资料页和数据操作提供浏览器卸载与 Tauri 关闭监听及卸载清理，资料表单仍由用户明确保存。
- `workspaceOperation` 仅保存当前会话中的最近数据操作反馈；页面读取设置单独维护加载状态，不清除操作结果。数据操作期间阻止离页/正常关闭，不引入后台任务框架、数据库记录或重启恢复承诺。
- 封面预览共享顶栏偏移，桌面按可用高度滚动，≤980px 在相同位置切换编辑/预览并保留挂载控件；保存/导出使用现有操作状态显示阶段。备份突出完整备份，数据库维护折叠；素材组合筛选从当前列表计算，不新增缓存。
- `style.css` 维护共享配色、面板、流程导航、书架网格/列表、窄窗口页签与低高度侧栏样式；不增加 UI 依赖或外部字体请求。
- 2026-11-02 操作区统一：普通 `.actions` 横纵间距均为12px，与前文间隔20px；生成操作同为12px。音频成品按钮复用该容器，封面个人样式输入使用可换行的最小布局基准；分段筛选与视图切换保持原组合样式。视频任务网格使用 `minmax(0, 1fr)`，下拉框可随列收缩，网格内按钮组不叠加顶部外边距。验证见 `WORKSPACE-UI-UPGRADE.md` 文末。
- `ProjectsPage` 通过 Vue Router 查询参数 `q/status/sort/view` 维护筛选状态，非法枚举回到默认值；返回书架通过原 URL 恢复，不增加本地缓存或业务持久化。最近创作仅从当前列表推导。
- `App` 监听成功导航后的 `route.path` 更新页面标题和主要内容焦点；参数变化不重建页面或抢焦点。Router 按路径切换回页首，并使用可用的浏览器历史滚动位置。
- `ProjectNavigation` 使用精确激活样式及 `aria-current=step`，资料入口与子页面不重复高亮；导航继续经过现有离页保存和发布任务守卫。
- 发布队列空态提供回到创作流程的操作，搜索使用显式清空按钮及 Escape，保留异步失败时的已有列表。
- 数据库仍为 v13，Rust Command、Provider、文件安全和最终发布确认均不变。验证范围见 `WORKSPACE-UI-UPGRADE.md`。

## Gemini Notebook 应用内自动音频（2026-10-29）

2026-10-31 音频配置适配：实测自定义弹窗的指导名称为“AI 主持人在本集节目中应着重于哪些方面？”，格式为“深入探究 / 摘要”，语言为 `role=combobox`。页面语义匹配统一读取原生关联 label 和 `aria-labelledby`，移除编辑框重复的 label 查询；支持原生 radio、组合框及实际中文选项。指导、格式、语言触发器和时长只在当前弹窗内定位，选项仍从网站浮层读取；已显示目标语言的组合框不重复展开。配置成功仍只返回观察，生成保留独立提交步骤。SQLite、任务恢复与提交去重流程不变。真实续验状态见 `IN-APP-AUDIO-VERIFICATION.md` 最新章节。

2026-10-31 真实成品下载适配：当前音频菜单调用 `window.open` 打开媒体地址，页面脚本只在选定下载动作的短窗口内临时捕获 HTTPS/blob 地址，成功或失败都恢复原方法；沿用锚点捕获与原生下载单一路径。真实媒体从 googleusercontent.com 跳转至 google.com，原生 `NavigationStarting` 以最初完整地址登记 `NavigationId`，只放行同 ID 且 `IsRedirected=true` 的服务器重定向，并记录实际 URL 链。原生下载 URI 必须命中最初地址或这条观察链，继续核对任务/运行/文档代次、15 秒窗口、不存在的目标路径与唯一下载对象。Tauri URL 回调仅在绑定下载窗口内委托该原生检查，普通网站导航白名单不扩展，不读取 Cookie 或调用私有下载 RPC。API 已核对锁定 webview2-com 0.39.1 / Wry 0.57.0 源码；微软说明重定向保留同一 NavigationId，见 https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/navigation-events 。无数据库迁移或新依赖。

笔记本身份恢复：创建意图与官方笔记本路径核对成功后先返回 `notebook_identified`，在既有 `notebook_url` 保存地址，阶段仍为 `creating_notebook`；下一步核对记录地址后清除新建标记，返回 `notebook_created` 进入添加来源。2026-10-30 移除自动命名及猜测创建弹窗的逻辑，笔记本身份只依赖任务 URL，不以标题输入框是否存在决定能否上传。旧任务已丢失地址时，「继续跟进」刷新窗口快照，展示当前 URL 并通过既有 `ConfirmAction` 等待明确确认；宿主串行核对窗口归属、当前 URL、文档代次、0 来源和无音频结果后才关联，在同一任务中恢复上传。已有 URL、提交记录、成品或另一活动任务时拒绝关联。停止和关联均复用页面内确认，取消或 Escape 不修改任务；不依赖 Tauri 覆写的异步 `window.confirm`，不增加原生消息框权限。

来源上传：根据「来源 / Sources」标题定位最近包含添加入口的面板，在该面板内选择顶部入口，允许同一面板空态中的等效入口重复；无法定位面板时仅接受全页唯一入口。必须等到来源弹窗，文件任务必须点击其中唯一的「上传文件」，仅标记该弹窗内唯一的文件输入框，再由原有宿主 `DOM.setFileInputFiles` 选择冻结文件。没有弹窗不回退到整页找文件控件。空白笔记本恢复共用这一来源入口判定，其余零来源、无音频及地址核对不变。

2026-10-31 上传器初始化修复：当前网站的 `xapUploaderTrigger` 在首次 `focus` / `mouseover` 时才创建隐藏文件输入框，直接 `HTMLElement.click()` 不会触发初始化。文件任务在确认运行未过期后先聚焦已唯一识别的上传按钮，再点击并等待弹窗内文件输入框；不改变其他按钮动作，不放宽输入框范围，不延长超时。原生文件选择、冻结来源、状态流和提交不明时只读核对保持不变。

页面动作统一在 `runStep` 边界将异常转换为 `{kind: page_error, code, message}`，避免 CDP 仅返回异常类名。宿主解析后仍走 `audio_step_failed`，完整提示包含错误代码；添加来源或提交状态不明时仍进入 `verify_remote`，不把异常当作成功观察，也不重复上传或生成。

网站故障恢复：`control_audio_job` 新增 `refresh_browser` / `reopen_browser`，沿用 v13 状态与 run_id 使旧动作失效；非终态先 paused，手动 resume 再执行。failed/cancelled 可继续原任务，先验证全局活动任务不冲突。添加来源或提交状态不明时继续转 verify_remote，只读核对。宿主复用原窗口、profile、任务 URL，调用 Tauri 2.12.5 的 reload/navigate；刷新保留登录页，同一笔记本跨官方域名不反复跳回。页面脚本在执行动作前识别明确错误标题、网络错误码、地区限制和人工验证，返回既有 page_changed 观察，不增加重试循环、依赖或浏览器权限。

audio.rs 管理专用窗口与单一串行调度；audio_page.js 按角色、标签、中英文文本执行当前动作，运行令牌阻止暂停后的异步写入。抖音/音频共用 webview_host.rs，以 CDP Runtime.evaluate(awaitPromise) 获取结构化结果，30秒上限；不开放调试端口或给远程窗口业务 capability。

SQLite v13 仍是唯一持久状态。原书通过大小/SHA-256 冻结副本和 DOM.setFileInputFiles 上传，不再 base64 分块传输。audio_download.rs 将精确 URL、任务/运行/文档代次和不存在的目标路径绑定具体 WebView2 下载对象，完成后校验并归档。完整摘要缓存可跨运行重试素材事务；迟到结果、HTML 和未确认部分文件不能登记。

新任务从 opening_site 开始，旧等待扩展任务继续时归一；已开始任务重启先核对，提交不明不重复生成。用户点击网站关闭按钮仍只隐藏窗口；2026-11-08 起，原生下载回调及缓存重试仅在归档成功后调用统一的 close_completed_notebook，以 Tauri destroy 销毁本任务窗口并清除窗口关联。下载完成处理复用 operation 锁，与调度及打开窗口串行；关闭前核对 window_task，避免旧任务关闭其他任务窗口。失败保留窗口，关闭失败也不回退已成功的归档状态。主工作台退出仍销毁网站窗口。每阶段有限等待，登录由用户操作，网站资料独立存于 audio-webview，完成关闭不删除该目录，之后「查看网站」可重建窗口并打开原笔记本。

2026-10-29 黑屏修复：实测入口重定向经过 notebooklm.google 到 notebook.google，宿主、页面和数据库补齐这两个精确官方域名。查看网站/继续会恢复同任务的 about:blank，首次导航提交前沿用阶段超时等待，Google 登录页保持原状。location=unsupported 转为明确地区限制提示并保留任务。无新增依赖或迁移，回归证据见 IN-APP-AUDIO-VERIFICATION.md 的黑屏修复记录。

扩展/helper/管道/启动注册及旧打包资源已移除；NSIS 按精确宿主路径清理旧注册和指定随包文件。windows-core 0.62.2 由既有间接依赖显式声明，没有新增库版本。真实 Google 和正式安装升级仍待验收；详见 NOTEBOOKLM-IN-APP-AUDIO-DESIGN.md、IN-APP-AUDIO-VERIFICATION.md。

## 音频时长贯通（2026-10-27）

`audio.rs` 统一时长校验、展示、请求上下文与推荐输出解析。`GenerationInput` 新增可选 `targetDurationSeconds` 和默认false的 `recommendDuration`，复用原文本任务、Provider、凭据、300秒超时及取消；推荐使用 `audio-duration-v1`，不生成文案版本，成功时事务保存书籍时长和任务成功状态。任务创建时冻结实际时长，后续修改项目不会改变在途任务。普通生成仍兼容省略时长的调用，使用项目值。

全量 `publishing-v4`、按项 `publishing-revision-v3` 接收 `audio` 中的秒数、中文时长、系列名和预计字数；指导正文不自带时间承诺，程序通过 `audio-template-v2` 统一插入时长。开场取总秒数三分之一且不超过60秒，结尾不超过45秒，余数精确分配到五段，总和等于目标。参考每分钟220字，并提示按时长取舍内容；短节目精简引入。原音频工具、音频/视频素材与发布边界保持原流程。

创作页首次对未配置书籍自动推荐，手动值经独立受限 `set_audio_duration` 命令保存，输入与在途/候选状态互锁。退出准备支持异步等待目标时长保存，再串行暂存文案；失败保留页面和输入。新封面默认副标题及发布话题示例去掉固定十分钟；已有封面/历史均不改写。无新增依赖，数据库v12见DATABASE，验证边界见 `AUDIO-DURATION-VERIFICATION.md`。

## 封面排版推荐（2026-10-27）

启用 `publishing-v3` 和 `publishing-revision-v2`，沿用统一文本 Provider，在发布内容中增加可选 `coverDesign`：titleSize、subtitleSize、textColor、subtitleColor、accentColor、backgroundColor、overlayOpacity。全量 AI 输出必须包含完整推荐，Rust 校验整数范围、六位颜色与蒙层后才存储；旧内容允许缺失。按项调整仅合并选中字段，推荐对象不进入文字编辑和复制循环，原有失败/取消/候选采用流程复用。

`CoverConfig.subtitleColor` 是独立副标题色；旧 JSON 缺失时使用其 textColor。前端在历史、草稿和个人样式入口归一，渲染器为历史缩略图保留同一默认行为。五个内置样式仍由 `types/cover.ts` 的单一配置定义，卡片、应用和默认封面复用；Canvas 预览和三比例 PNG 导出继续使用同一渲染器。无新增依赖、IPC 或权限。新封面自动带入推荐；已有配置仅主动应用，保留标题、背景素材与裁切。

## 智能辅助发布实现（2026-10-27）

`douyin.rs` 保留官方窗口、安全页面执行和原生文件选择；`douyin_flow.rs` 作为同一模块的会话编排，替代原 upload/complete 两条入口。Rust 独占运行 ID、任务快照、账号确认、视频/文案/封面回读证据和合集选择；前端订阅结构化事件，挂载重读快照，按任务与运行 ID 过滤。开始复用已保存 ready 快照；前端先保存修改并准备包，宿主重新校验。继续只接收当前运行的明确决定。停止先设置取消标志，再等待串行操作结束后回应，不在前端提前解锁。

2026-11-01 合集与重复账号确认修复：合集确认和恢复共用同一校验，先读取官方控件已选值；已与目标一致时不依赖下拉列表展开，也不重复选择。需要更改时才打开列表，在原有停止检查下最多等待 5 秒并要求唯一精确选项；不能读取控件不等于“不加入合集”。保留不同手工选择，只有本次明确授权才替换。账号指纹只使用头像主图片地址，排除 CSS 包装、候补图片和查询/片段参数；已知官方页账号控件未挂载或头像未就绪，复用 30 秒只读等待，不按退出登录处理。网络中断本身不清除确认，同一窗口中的已确认账号沿用；真实登录页、账号变化及窗口关闭仍需重新确认。无持久信任、依赖、权限或 schema 变更，最终发布仍人工操作。回归与隔离原生证据见 `DOUYIN-FLOW-VERIFICATION.md`。

2026-11-01 启动与发布入口修复：WebView2 首次导航提交前的 `about:blank` 仅作为只读检查的等待状态，不放宽官方 URL 白名单，也不在空白页执行脚本。页面检查对初始空白页和文档加载中的空结果有限等待 30 秒，期间检查停止及窗口关闭；其他来源仍立即拒绝。账号确认后，从已识别的首页/管理页直接导航固定官方视频上传地址，不依赖「作品发布」菜单是否展开；已经在上传页则不重复导航。等待唯一、可用且未选择文件的上传控件后才预占发送状态；登录页、草稿和弹窗交回原流程，未知或未就绪页面不再因“离开首页”而被误当成跳转完成。隔离原生验证与真实账号边界见 `DOUYIN-FLOW-VERIFICATION.md`。

2026-10-29 视频回读修复：涉及视频身份的人工决定和最终核对统一通过 `read_video_editor` 恢复「预览视频」，最多等待 10 秒取得可播放且有来源地址的播放器；等待时保留停止检查。普通封面回读保持只读，视频标识排除弹窗内播放器。有可见弹窗时先要求用户处理，已激活的视频页签不重复点击。仍严格核对本会话视频地址，预览未就绪给出单独提示，不沿用缓存值冒充当前视频。夹具复现及真实页面验证边界见 `DOUYIN-FLOW-VERIFICATION.md`。

只在已识别且没有编辑内容的首页/管理页导航至已知上传页。平台完成信号不能确认时等待人工核对；保留已有文案、章节和设置。封面逐比例记录预览身份，已确认且未变的内容跳过，发送结果不明时等待人工确认而非重传。合集在线意图独立于包 manifest，v11 存任务字段，默认值按稳定公开账号 ID 存 app_settings；当前只读适配器尚未验证稳定公开 ID，返回 null，昵称确认限当前窗口会话。UI 提供当前任务摘要、五步进度、就地决定和人工结果记录；记录成功才允许启动用户明确选择的下一篇。真实账号、原生上传和安装验证边界见 `DOUYIN-FLOW-VERIFICATION.md`。

2026-11-07品牌更新：图形统一使用 `assets/app-icon.svg` 的「折页向前」标识；应用侧栏与关于页直接导入同一资源，favicon 与桌面 PNG/ICO/ICNS 通过 `npm run icons:generate` 同步生成。移除两处各自绘制的品牌图形及仅服务旧图形的主题变量，深浅主题保持同一Logo，不新增组件封装、依赖或状态。设计、色值和验证边界见 `BRAND.md`。

当前产品范围为发布文案及标签、音频指导、封面素材生成与按项调整、自动草稿、封面工坊（裁切/个人样式/可选 AI 背景）、明确文案来源的半自动发布准备、Windows 应用内抖音辅助填写、资料导入与完整备份。本文前面的完整工作台设计及里程碑切片保留为历史记录；当前扩展见本文最新章节，历史验收见各里程碑验证报告。

应用更新（2026-10-25）：`services/updates.ts` 封装 Tauri Updater/Process 与设置 IPC；`useAppUpdates.ts` 仅维护一份跨页面更新状态，应用壳启动初始化并显示新版本提示，`AboutPage.vue` 展示版本、说明、进度及确认安装。只在关于页确认安装，创作页先按原导航守卫完成草稿暂存。Updater 2.14.0 / Process 2.4.0 使用官方当前 Tauri 2 稳定版 API，并已核对安装源码；主窗口只授予 check/download/install、资源释放、读取版本、重启和两条设置命令。公钥内置，私钥仅在打包时读取。发布脚本从实际提交生成统一说明与四平台更新清单，签名校验由官方插件执行。无需新增通用网络、任意文件执行或进程退出权限。

## 1. 技术目标

构建 Windows 10/11、macOS 与 Linux 的本地优先桌面应用，围绕书籍项目完成 AI 创作、封面排版、视频关联、素材归档和半自动发布准备。人工桌面验收以 Windows 起步，新增平台需要分别验证安装、启动和持久化。

## 2. 技术栈

- Vue 3 + TypeScript + Vite：前端界面。
- Tauri 2 + Rust：桌面容器、系统能力和受控文件操作。
- SQLite：结构化业务数据。
- Pinia：跨模块状态。
- Vue Router：导航。
- shadcn-vue + Tailwind CSS 4：统一 UI，已覆盖应用壳层及全部页面；业务布局保留共享样式，交互控件统一使用组件。
- Vitest：单元测试。
- Playwright 或适用的桌面测试方案：关键交互验证。
- SVG/Canvas 或专用渲染模块：封面文字排版与导出。

依赖版本须在初始化时核验兼容性，不可盲目使用未经验证的版本。

### M0 实际落地（2026-10-23）

- 使用 npm 和精确版本锁；Vue 3.5.44、Vite 8.3.5、TypeScript 6.0.3、Vue Router 4.6.4、Tauri 2.12.x。
- 当前仅一个技术验证路由，采用 hash history；没有跨模块状态，Pinia 与 UI 组件库延后到有真实使用场景时引入。
- SQLite 统一由 Rust `rusqlite`（bundled）访问，前端通过受限的 `read_probe` / `save_probe` 命令调用；不向 WebView 暴露任意 SQL。
- 文件选择使用官方 Dialog 插件，只授权主窗口 `dialog:allow-open`；没有文件写入、Shell 或网络插件权限。
- UI 位于 `src/modules/verification/`，IPC 边界位于 `src/services/verification.ts`，数据库位于 `src-tauri/src/database.rs`。
- Windows 仅配置当前用户 NSIS 安装包；WebView2 已存在时复用，否则由安装器下载官方 bootstrapper。
- 打包工具启用 `bundle.useLocalToolsDir`，缓存到项目 `target/.tauri/`；本机用户级缓存目录下的 NSIS 重命名曾报 os error 17，改用项目缓存后实测构建成功。
- M0 测试包括 Vitest + jsdom 的界面行为与 Rust 独立临时 SQLite 数据库测试；真实桌面验收单独记录在 `M0-VERIFICATION.md`。

## 3. 架构原则

1. UI、业务服务、领域模型和基础设施分层。
2. Vue 组件不直接编写复杂 SQL 或供应商协议逻辑。
3. 数据库是结构化业务数据的权威来源。
4. 文件系统保存素材内容，数据库保存路径和元数据。
5. AI Provider 通过统一接口接入。
6. 发布平台通过 PublishProvider 适配。
7. Tauri Command 仅暴露必要能力，权限按最小化原则配置。
8. 失败操作不得破坏已保存内容。
9. 数据库变更必须有迁移脚本。
10. 所有核心流程有测试或人工验收步骤。

## 4. 建议目录

```text
shuyin-ai/
├── src/
│   ├── app/
│   │   ├── router/
│   │   ├── layouts/
│   │   └── stores/
│   ├── components/
│   │   ├── common/
│   │   ├── editor/
│   │   ├── cover/
│   │   └── project/
│   ├── modules/
│   │   ├── dashboard/
│   │   ├── projects/
│   │   ├── writing/
│   │   ├── covers/
│   │   ├── assets/
│   │   ├── publishing/
│   │   ├── records/
│   │   └── settings/
│   ├── services/
│   │   ├── ai/
│   │   ├── project/
│   │   ├── asset/
│   │   ├── cover/
│   │   └── publishing/
│   ├── types/
│   └── utils/
├── src-tauri/
│   ├── src/
│   │   ├── commands/
│   │   ├── database/
│   │   ├── filesystem/
│   │   └── security/
│   ├── capabilities/
│   └── migrations/
├── prompts/
├── docs/
├── tests/
└── README.md
```

允许根据实际脚手架调整，但必须保持职责边界。

## 5. 分层职责

### UI 层
负责展示、输入、校验、加载状态和错误提示。

### Application / Service 层
负责项目流程、AI 任务编排、封面生成、发布任务包构建和业务状态转换。

### Domain 层
定义 BookProject、ScriptVersion、PublishMaterial、CoverAsset、VideoAsset、PublishTask、PublishRecord 和 ModelProviderConfig 等核心对象。

### Infrastructure 层
负责 SQLite、文件系统、AI API、图片服务、系统凭据存储和 Tauri Command。

## 6. AI Provider 设计

文本模型接口建议从以下契约起步：

```ts
interface TextGenerationRequest {
  systemPrompt?: string;
  userPrompt: string;
  temperature?: number;
  maxTokens?: number;
  responseFormat?: "text" | "json";
}

interface TextGenerationResult {
  content: string;
  model: string;
  provider: string;
  usage?: {
    inputTokens?: number;
    outputTokens?: number;
  };
}

interface TextModelProvider {
  generate(
    request: TextGenerationRequest
  ): Promise<TextGenerationResult>;
}
```

正式实现需要补充超时、取消、错误码、重试策略、结构校验和日志脱敏。

每个 Provider 负责将统一请求转换为供应商实际协议。OpenAI 兼容接口并不保证所有模型支持相同参数。

### AI 任务执行流程

1. 校验输入。
2. 创建任务记录。
3. 调用 Provider。
4. 解析响应。
5. 校验结构。
6. 保存结果版本。
7. 更新任务状态。
8. 向用户展示结果或可理解的错误。

任务状态：pending、running、succeeded、failed、cancelled。

不得对所有错误无限重试，不得在失败时覆盖已有草稿。

## 7. 封面架构

封面拆为背景素材生成和程序化排版两个步骤。

封面配置至少保存模板 ID、画布尺寸、背景素材引用、主副标题、字体样式、颜色、位置、对齐和账号标识。导出 PNG 时检查字体、图片、文字边界、输出尺寸和目录权限。

AI 图片服务是可选增强能力，本地模板必须独立可用。

## 8. 发布适配层

建议定义：

```ts
interface PublishProvider {
  getCapabilities(): Promise<PublishCapabilities>;
  prepare(taskId: string): Promise<PrepareResult>;
  publish?(taskId: string): Promise<PublishResult>;
  getStatus?(taskId: string): Promise<PublishStatus>;
}
```

第一版实现 LocalPublishProvider，提供素材检查、复制、文件打开和任务记录。未来的平台 Provider 必须基于官方授权、明确权限和实际验证实现，不得假设接口一定可用。

## 9. 文件系统约定

建议工作目录：

```text
书引AI工作目录/
├── projects/
│   └── {书名}/
│       ├── research/
│       ├── scripts/
│       ├── covers/
│       ├── publish/{书名}-发布任务包/
│       └── exports/
└── backups/
```

- 原始视频默认保留在用户原位置。
- 数据库记录关联路径。
- 文件缺失时保留记录并支持重新定位。
- 删除关联不等于删除文件。
- 清理素材前必须预览并确认。
- 工作目录迁移必须有明确流程。

## 10. 安全要求

- API Key 使用操作系统凭据存储或经确认的安全方案。
- 密钥不得出现在日志、错误提示、普通设置或 Git。
- 自定义 Base URL 应提示用户仅连接可信服务，避免向不可信地址发送密钥。
- Tauri 权限采用最小权限原则。
- 不读取浏览器 Cookie，不存储抖音账号密码。
- 不实现绕过平台授权的自动发布。

## 11. 测试策略

- Domain 和业务服务使用单元测试。
- Provider 使用契约测试及模拟响应测试。
- SQLite 使用独立测试数据库验证迁移和事务。
- 文件服务覆盖文件存在、缺失、移动、无权限和重复文件场景。
- 发布任务包覆盖素材齐全和素材缺失场景。
- 桌面安装、升级和凭据存储需要真实 Windows 环境验收。

## 12. 跨平台自动打包

GitHub Actions 仅监听 main 的 push，支持 PR 合并及本地 merge/cherry-pick 后推送。版本提交通过 job 条件及已发布标签检查跳过，避免循环触发；dev 和标签推送不构建。prepare 生成一次语义版本提交，通过 Git bundle 传递给 Windows x64、macOS ARM64、macOS x64、Linux x64 四个独立 runner，所有构建均校验并使用同一个提交 SHA，打包前不推送版本。

Tauri 通过三个平台配置自动选择 NSIS、APP/DMG、DEB/RPM/AppImage，`desktop:build` 不再固定 Windows。Linux 的产品名使用 `Shuyin AI`，确保包管理器包名为 `shuyin-ai`；应用标识符和中文界面不变。macOS 使用现有 SVG 生成的 ICNS，当前为 ad-hoc 签名，尚未配置 Apple Developer ID 与公证。

所有平台的检查与构建成功后，汇总校验 10 个分发文件，生成 SHA-256 清单；然后将版本提交 cherry-pick 到 dev，原子推送 main、dev 和标签。最后在 GitHub Release 草稿中上传完整产物，上传完成才公开。版本与产物逻辑只使用 Node 内置模块、系统归档工具及 runner 自带 gh，没有新增 npm/Rust 依赖。main 快照变化、cherry-pick 冲突或写权限拒绝均中止同步；发布阶段可单独重跑失败任务。详细格式、权限和恢复流程见 `RELEASE.md`。

## M1 实际实现（2026-10-23）

- 页面：`/projects` 搜索/筛选，`/projects/new` 新建，`/projects/:id` 资料与素材，`/settings` 目录与备份；M0 验证保留独立入口。
- 调用链：Vue 页面 → `services/projects.ts` → 受限 Tauri `commands.rs` → `database/{projects,assets,backup}.rs`。状态以 SQLite 为准，页面只维护当前编辑草稿，不增加 Pinia 或 UI 依赖。
- 项目字段增加主题；所有状态经同一保存命令更新。归档后可选择任意活动状态恢复；支持 ISBN/作者/书名子串查询，百分号与下划线作为普通字符。
- 工作目录默认在应用数据目录下，可在设置中选择其他磁盘。新项目创建 `projects/{书名}` 目录，同名追加 ` (2)`、` (3)`。目录名由 `projects.rs::create_named_directory` 统一生成：保留中文，禁用字符替换为下划线，处理尾部空格/句点及 Windows 设备保留名，UTF-8 名称限制为120字节并保留完整字符。原子创建目录，遇到已有文件或目录继续编号，不覆盖历史文件。项目 ID 仍作数据库主键，已有路径及修改书名时的工作目录保持不变；重新指定目录只更新引用。
- 素材仅存路径及基本元数据；每次读取实际检测文件，区分可用、缺失和无法访问。重新定位保留素材 ID，校验素材类型和重复关联。文件打开只接受已登记且属于允许格式的素材 ID，目录打开只接受项目 ID。
- 基础备份使用 SQLite `VACUUM INTO` 创建唯一新文件，恢复使用现有 rusqlite 的 `backup` feature；未新增依赖包。备份不包括外部媒体文件。恢复先核验版本、完整性、schema 和外键，再自动备份当前数据库，最后使用 SQLite backup 原子替换数据库内容。
- M1 备份恢复只接受 schema v2；M0 数据通过正常启动迁移升级。目录重定位不自动迁移文件；媒体时长、分辨率、校验和提取不在本里程碑。

## M2 实际实现（2026-10-23）

- 页面：`/settings/models` 模型配置与测试，`/projects/:id/writing` 创作。业务组件只依赖 `services/ai.ts` 和 `types/ai.ts`，没有供应商请求、密钥读取或 SQL。
- Rust：`ai.rs` 的统一 `TextProvider::generate(TextRequest)` 接口与 `OpenAiCompatible` 适配器，`ai_commands.rs` 编排任务，`credentials.rs` 访问系统凭据，`database/ai.rs` 负责模型、任务与版本持久化。未新增 npm、UI 或全局状态依赖。
- 适配器调用 `{base_url}/chat/completions`，发送 model/messages/stream=true；显式配置时附加 reasoning_effort，不假设模型支持 temperature、max_tokens 或 response_format。版本化系统 Prompt 位于 `prompts/writing-v1.md`，用户资料按独立用户消息中的 JSON 数据传入。
- reqwest 0.13.5 使用 Rustls 证书校验；300 秒总超时、15 秒连接超时、显式禁止自动重试和重定向。回环服务绕过环境代理，远程服务保留 reqwest 的环境代理行为；HTTP 仅允许 localhost/127.0.0.1/::1，其他服务必须 HTTPS，URL 不允许账号、查询或片段。
- HTTP 状态错误只返回本地固定文案，不向界面或日志返回服务响应体、授权头和底层网络错误。HTTP 504/524 统一归类为 timeout，提示服务网关等待上游响应超时并保留状态码；本地300秒上限不能延长远端网关的等待限制。连接测试只要求回复OK，不能证明较长的分镜请求也会及时完成。响应最多 2 MB，必须完整结束（finish_reason=stop）、有非空文本，再严格解析带 format 的 JSON 并核验内容类型、数量和长度才入库。连接测试验证文本通信，不代表生成 JSON 能力已验收。
- keyring 4.2.0 默认 v1 API 选择 Windows Credential Manager、macOS Keychain、Linux Secret Service。service 使用实际应用 identifier，凭据引用随机生成，验证产品与正式产品隔离；WebView 没有凭据回读命令。更换服务地址需重新输入已有密钥，避免把旧密钥送到新地址。
- start_generation 创建 pending；execute_generation 转 running 并调用 Provider；取消使用瞬时 watch 信号，同时 SQLite 把任务转 cancelled。网络等待期间不持有数据库锁；版本与成功状态事务提交，并在数据库锁下检查任务仍为 running。重启和恢复将未完成任务标记 interrupted。
- 历史内容共用 writing_versions，按项目和内容类型分别编号，人工修订保留模型/Prompt 来源。生成期间的手动编辑保留，新生成内容留在历史；手动保存期间禁用编辑，避免保存响应覆盖同时输入。路由离开和窗口关闭前提示未保存状态。
- 新业务命令同时登记于 `build.rs`、`lib.rs`、`permissions/ai.toml` 与主窗口 capability；无新增任意网络、Shell、SQL 或文件写入权限。
- 备份恢复支持 v2/v3，并在内存快照中升级旧备份。详见 DATABASE 第 9 节；已执行命令与实测边界见 `M2-VERIFICATION.md`。

## 当前三项生成切片（2026-10-23）

- 书籍录入简化为书名、选填作者及资料。结果页只编辑/复制标题、发布内容及音频提示词，历史折叠，无类型、数量、引用选择或任务列表。
- 新 GenerationInput 只包含项目和 Provider ID；一次请求生成 PublishingDraft（标题、内容、书籍引入、五段指导），Rust 校验后用独立版本化音频模板组装完整提示词。
- 固定模板保证甲子、8—10分钟、简体中文、系列开场与结尾；书籍引入和五段章节随书变化。不给音频工具假装传输书籍来源，不在应用生成音频文件。
- 一套结果保存为 writing_versions 的 publishing 类型；沿用既有任务、取消、凭据及事务。旧 Markdown/List 类型仅保留读存历史与备份所需，旧 Prompt 不再用于新生成。
- 复制使用 WebView 的 Clipboard API，用户操作触发、只写当前编辑字段；失败提示手动选择复制，没有引入插件或新权限。
- v4 迁移和旧备份兼容见 DATABASE；验证见 PUBLISHING-VERIFICATION。

## M3 本地封面工坊（2026-10-23）

- `/projects/:id/covers` 从书籍资料/结果页进入，主导航不变。Vue 维护单份草稿；`types/cover.ts` 定义配置和模板，`services/coverRenderer.ts` 为唯一 Canvas 渲染器，`services/covers.ts` 提供 IPC 边界。
- 预览直接使用导出分辨率画布。等待系统字体就绪，按字素（Intl.Segmenter）测宽换行，逐步缩小字号，固定安全边距，超限提示；主副标题、品牌文字均由程序排版。
- 复用 assets 的图片关联；link_asset 返回登记 ID，原调用方可继续忽略返回值。读取命令仅接受本项目已关联的图片 ID，检查扩展名、文件头和 15 MB 上限，通过 Tauri ipc::Response 返回 ArrayBuffer；WebView 解码并检查像素数。使用 data URL，无任意文件读取或 asset protocol 权限。
- 发布封面仅输出 4:3（1440×1080）和 3:4（1080×1440）。页面维护单份草稿，比例选择仅切换预览；导出先用同一渲染器生成两张 PNG，再依次复用保存命令。第二张写入失败保留第一张的版本/路径及页面未保存草稿，不宣称整组完成。
- Rust `database/covers.rs` 校验三种固定尺寸、配置和 PNG 文件头/尺寸，将单张图的新版本与项目时间放入同一事务。每次导出写入项目 `covers/{书名}-{宽比高}-{宽}x{高}-v{版本}.png`，比例约分并用「比」替代 Windows 禁用的冒号，例如 `围城-3比4-1080x1440-v3.png`。复用项目目录的 `safe_book_name` 统一清理书名；`create_versioned_image` 使用 create_new，遇到同名文件或目录递增版本，并将最终版本写入历史。文件写入或事务失败时回滚并清理本次新文件。旧文件路径保持原样；进程在落盘和事务提交之间被强制结束可能留下未登记的新 PNG，再次导出跳过占用版本。
- cover_versions 配置包含 schemaVersion=1、模板、尺寸、文字、字体、颜色、布局及 backgroundAssetId。没有单独可变“当前封面”表，最新版本即重启默认配置。背景引用保留在配置快照中，素材被移除时可以显示缺失并重新选择，不改写历史。
- 旧尺寸历史读取不改写数据库或文件；恢复编辑时保留内容样式，将横向转为 4:3、竖向及方形转为 3:4，并显示原尺寸及转换提示。schema 仍为 v5，无新增迁移或依赖。
- PNG 导出与保存共用一个命令；导出总是追加版本，已保存的历史 PNG 不覆盖。系统字体和源背景属于外部依赖，跨机器字体/源图变化可能影响重新渲染，原 PNG 文件仍保持不变。
- 无新增 npm/Rust 依赖；三个封面业务命令同时登记 build.rs、lib.rs、permissions/covers.toml 和主窗口 capability。数据库备份仍由原有统一实现处理。

## M3 可选 AI 背景（2026-10-24）

- `database/ai.rs` 的模型配置增加 text/image 用途，默认模型按用途独立。继续使用现有系统凭据实现，不向前端提供密钥读取接口；更换 Base URL 必须重新输入已有密钥。
- `ai/images.rs` 实现唯一图片 Provider：请求 `/images/generations`，使用用户填写的模型 ID、`n=1`、`size=1024x1024`、`response_format=b64_json`，仅接受一张 base64 静态 PNG、JPEG 或 WebP；不下载响应 URL。沿用统一 HTTP 错误脱敏、禁止重定向和自动重试，图片超时为 180 秒。
- 图片响应写入素材前使用 `image 0.25.10`（仅启用 jpeg/png/webp）完整解码并限制 15 MB、4000 万像素和 160 MB 解码分配；拒绝动画内容，按实际格式保存 `.png`、`.jpg` 或 `.webp`。
- 将已解析的书名、作者和画面描述放入版本化 Prompt；只生成背景，中文文字继续使用原 Canvas 渲染器。`base64 0.23.1` 与 `png 0.18.1` 原已存在于锁文件；新增 `image 0.25.10`（仅 jpeg/png/webp）用于全量像素解码，静态格式和 15 MB/4000 万像素限制校验后才允许保存。
- `start_cover_generation`/`execute_cover_generation` 复用 ai_tasks、共享任务注册/取消和中断恢复。`database/image_generation.rs` 从原有完整解码校验结果取得格式和真实尺寸，复用封面命名/独占创建函数，写入 `covers/{书名}-{比例}-{宽}x{高}-背景-v{版本}.{扩展名}`；同一名称从 v1 递增并跳过已有文件，素材 original_name 与真实文件名一致。新图片文件、素材引用和任务成功状态组成事务，失败清理本次新文件，旧背景及历史不改写。
- 封面页默认使用本地制作。主动展开 AI 区域才读取图片模型，生成成功后先读取和解码素材，再替换草稿中的背景；不自动保存封面版本。失败、取消与迟到响应保留当前草稿、背景和历史；生成期间阻止离页，现有双比例导出及部分成功提示继续复用。
- 数据库增量迁移为 v6，详见 DATABASE。Windows 真实 WebView2、IPC、系统凭据、SQLite 和文件验证通过；本机可控响应验证不代表外部图片服务可用性，完整边界见 M3-VERIFICATION。

## M4 半自动发布准备（2026-10-24）

- 页面为 `/projects/:id/publishing`，书籍、文案和封面页均可进入。沿用现有页面样式、操作状态、素材组件与 IPC 服务边界，无新增依赖。新任务使用最新已保存文案及可用封面作为初值，用户可编辑文字、选填标签并选择本书的视频与两张封面。
- 调用链为 PublishingPage → services/publishing.ts → publish_commands.rs → database/publishing.rs。当前只有本地发布准备实现，不为尚未接入的平台建立空接口或转发层。平台只提供固定抖音创作者中心入口，不调用上传、账号或发布接口。
- 任务保存发布文字快照与素材外键。输入验证归 Rust 业务服务，UI 只维护编辑草稿；生成时先保存，失败保留草稿和历史，路由/关闭保护未保存编辑。修改标题或封面选择会撤销对应 UI 确认。
- 发布包写入项目 `publish/{书名}-发布任务包` 唯一新目录，同名追加序号，复用项目目录命名函数；不使用发布标题作为书名。包含 `title.txt`、`description.txt`、`tags.txt`、`video-path.txt`、两张 PNG、`manifest.json` 和 `README.txt`。视频不复制。封面读取上限 15 MB，完整 PNG 解码并校验两种尺寸；写完后再次检查源文件快照，最后事务提交 ready。
- 写入或事务失败时仅清理本次创建的目录；旧任务包及源文件不删除。强制结束进程可能留下未登记的新目录，但不影响原素材。再次生成始终使用新目录，不覆盖历史。
- 清单随任务查询重新计算，视频/封面缺失、文件引用变化或任务包不完整会撤销 ready。检查使用文件路径、大小、毫秒修改时间及文字内容；不对大型视频进行全量哈希。平台上传后仍必须由用户明确确认发布成功，发布记录与任务/项目状态事务提交。
- 打开视频复用既有已登记素材与扩展名校验；目录与封面通过任务 ID 定位。打开网站仅使用固定 HTTPS 地址，不向 WebView 授予任意 Shell、SQL 或文件写入权限。五个命令在 build.rs、lib.rs、权限文件与 capability 同步注册。
- 官方实现核对：现有 `rusqlite 0.40.2` transaction 源码与文档、Tauri `invoke` 和权限文档，以及现有 `image 0.25.10` 解码限制实现。复用当前锁定版本，不升级依赖。

## M5 验证与修正（2026-10-24）

- `package_valid` 现在对两张封面副本与源 PNG 做有上限的逐字节比较，修复仅检查大小导致同大小损坏文件仍为 ready 的问题。复用已有 15 MB 文件读取限制，不新增哈希依赖、数据库字段或另一套校验状态；大型视频仍按原路径、大小和修改时间检查。
- 新增同大小损坏封面回归、v2–v7 备份恢复与恢复前安全备份回退矩阵、三项目文案/封面/发布任务/模型凭据在备份恢复和重启后的完整保留回归。
- Windows 通过独立 identifier、安装目录与数据库运行安装版，使用真实 WebView2、Tauri IPC、系统凭据、SQLite 和文件；三本书调用既有配置的外部文本服务，异常场景使用本机可控契约服务。截图、试用包和脚本保留于忽略目录 `output/playwright/m5/`。详细命令与结果见 `M5-VERIFICATION.md`。

## 封面素材生成扩展（2026-10-24）

- 文本入口改用 `prompts/publishing-v2.md`，沿用原 GenerationInput、书籍 title/author/description/sourceNotes、统一 Provider、300 秒超时及任务事务。同一 PublishingDraft 增加 coverTitle、coverSubtitle、coverPromptLandscape、coverPromptPortrait；音频仍复用 audio-template-v1。
- AI 草案的四项封面字段必须存在且非空；主副标题分别限制30/60字，提示词各2000字，两份分别包含 4:3/3:4 且不能完全相同。校验后才组装 WritingContent 和保存版本；不通过无限重试补齐内容。具体构图、内容忠实度与吸引力由 Prompt 约束并由用户审核。
- 结果页沿用统一字段编辑/复制，不增加第二套封面素材存储。保存编辑后通过 writingVersion 查询参数进入封面页，直接使用选中版本；保存失败保留原页。普通封面入口使用最新 writing version。
- 新封面在初始配置上带入主副标题，保持未保存状态。已有封面仅在点击应用按钮时替换主副标题，背景、样式和历史不改动。提示词比例独立于画布预览，切换不会替换用户编辑的 AI 描述；点击「用于 AI 背景」才应用。读取文案失败不阻止本地封面，重读只更新文案来源。
- 内置图片 Provider 继续输出方形背景并进行双向裁切；不把文本中目标比例当作图片 API 的实际输出尺寸。外部工具可使用两份独立构图提示词，中文标题仍由 Canvas 排版。
- 无新增依赖、IPC Command、权限或数据库表。核对本地 Vue 3.5.44 的 ref/computed 实现、serde_derive 1.0.229 的默认字段反序列化实现及官方文档；历史 JSON 兼容与验证见 DATABASE 文末及 COVER-MATERIALS-VERIFICATION。

## 界面体验改造（2026-10-24）

- 应用壳层统一为固定侧栏、工作区顶栏和响应式内容区；侧栏保留「我的书籍」「模型设置」两个主入口，项目内使用「内容创作 → 封面工坊 → 发布准备」三步导航，窄屏收缩为图标导航。
- 书架增加即时搜索、状态筛选、排序和清空操作，筛选只在已读取的数据上完成，不重复请求数据库；书籍卡片提供状态、更新时间和继续创作入口。
- 内容创作页按「发布文案／音频提示词／封面素材」分组，编辑内容跨页签保留，支持单项复制反馈、键盘 `Ctrl/⌘ + S` 保存及未保存离页保护。
- 封面页改为编辑器与实时预览并排布局，文字、排版、背景和可选 AI 背景分 tab 管理；预览与导出共用同一渲染器，保留双比例导出、历史版本和未保存提示。
- 模型设置改为服务列表与配置详情布局，按文本/图片用途展示默认状态和凭据状态；切换或离开前会确认未保存修改，密钥输入仍只写入系统凭据库。
- 发布准备页、操作成功/失败反馈、图标、按钮、表单、空状态和窄屏断点统一使用同一套颜色、间距、圆角、焦点和 reduced-motion 规则；新增 SVG favicon。
- 本次只调整 Vue/CSS/测试与文档，没有新增依赖、IPC 命令、权限或数据库字段/迁移；数据库 schema 与已有数据保持不变。

## shadcn-vue 全页面接入（2026-10-25）

- 使用官方 `new-york-v4` registry 的 Button、Input、Textarea、Badge、Select、Checkbox、Slider、Tabs、Collapsible 源码，保存在 `src/components/ui/`，并保留 MIT LICENSE。`components.json` 管理源码路径，`src/lib/utils.ts` 是唯一的类名合并工具。没有引入运行时 UI 插件或第二套组件库。
- 依赖锁定：Reka UI 2.11.0、VueUse 14.4.0、Tailwind CSS / Vite 插件 4.3.3、class-variance-authority 0.7.1、clsx 2.1.1、tailwind-merge 3.7.0。VueUse 与 Reka 的依赖范围对齐，避免重复版本。显式加载 `@types/web-bluetooth` 只为满足 VueUse 类型声明，不增加蓝牙功能或桌面权限；类型检查仍覆盖依赖声明。
- 当前机器 Node 22.21.1 不满足初始化 CLI 的间接依赖 `validate-npm-package-name@8` 的 Node 要求，因此按官方源码分发方式直接导入组件；没有强制安装、跳过引擎检查或修改全局 Node。后续使用 CLI 前应先核验其当前 Node 要求。
- Select 的图标复用现有 `AppIcon`，不新增图标依赖；移除本切片不用的 Select 分组、标签、分隔线及文字包装组件。组件名称与可选属性默认值遵守现有 ESLint 规则，不关闭检查。未引入动画依赖。
- `src/style.css` 集中定义暖白、灰阶和墨绿主题变量；保留业务布局、提示与响应式规则，删除已被组件替代的原生按钮、输入、下拉框、标签和手写页签样式。Textarea 保留 `rows` 与纵向调整大小；颜色、日期时间等原生输入能力由 Input 统一外观。
- 书架保留即时搜索、状态计数、按书名/更新时间排序、刷新失败保留列表、空状态和浏览器预览边界。排序改用带键盘操作与焦点返回的 Select。书籍表单复用 Input / Textarea / Button，保存调用、未保存离页保护及删除确认语义保持一致。
- 全部编辑页与辅助页迁移到相同组件体系。删除手写 `EditorTabs.vue`，由 Tabs 管理键盘和 ARIA 关联；设置 `unmountOnHide=false` 保留切换前后的表单。封面文字/样式表单与背景生成表单保持独立，避免嵌套表单。
- Select 采用带类型参数的 `modelValue` / `update:modelValue`；历史切换由已保存版本控制，取消放弃修改时不更新选中值。空素材使用 `null`，尺寸和字号保持数字。Select、Checkbox、Slider 显式绑定业务禁用条件，避免仅依赖原生 fieldset 而漏掉自定义交互。
- 发布任务标题/状态及视频文件状态会在同一 ID 下更新。当前 Reka 2.11.0 的 SelectItemText 注册文字不会随普通 DOM 文字更新，因此这两个 SelectValue 使用插槽直接渲染业务数据；标签格式在各自的小函数中复用，不修改依赖源码、不引入缓存或重复状态。
- 本次影响为 `style` / `refactor`，默认 patch；不手动修改版本。数据库仍为 schema v7，无 Rust、IPC、权限或迁移变更。验证见 `UI-COMPONENTS-VERIFICATION.md`。

## 封面 16:9 尺寸扩展（2026-10-25）

- `coverSizes` 增加 1920×1080，沿用现有尺寸选择、Canvas 排版、背景裁切和历史恢复流程；Rust 保存校验同步接受该尺寸，无新依赖、IPC 或迁移。
- 批量导出按 4:3、3:4、16:9 顺序追加版本，全部 PNG 排版成功后才开始写入；任一写入失败保留已成功路径及未保存草稿，进度按实际数量显示。
- 文本生成继续提供两份现有构图提示词，图片生成仍产出单张背景；本轮仅扩展本地预览和导出尺寸。发布准备仍选择 4:3、3:4 两张封面。

## 按项调整（2026-10-25）

- `GenerationInput` 增加可选 `revision`：来源版本 ID、非空且不重复的七项字段选择和最多2000字的修改要求。原整套生成输入与返回 JSON 保持兼容。
- 复用 `start_generation` / `execute_generation`、统一 Provider、取消信号和300秒超时。`prompts/publishing-revision-v1.md` 约束模型只返回选中字段；Rust `ai/revision.rs` 严格核验键集合、内容长度和比例，并从来源版本合并其余字段。资料与要求仍经独立用户消息传入。
- 音频调整返回书籍引入和五段结构；与整套生成共用 `AudioDraft::into_prompt` 和 `audio-template-v1`，避免重复模板实现。
- `execute_generation` 的调整结果是 `GenerationPreview`，整套生成仍返回 `WritingVersion`。候选以 `ai_tasks.preview_json` 保存，任务成功但不追加版本；新增受限 `apply_generation` 命令，事务内追加 mixed 版本、关联任务并清空候选，重复采用返回同一版本。
- 页面先保存已有修改，再发起调整。生成和预览期间锁定原编辑器；逐项对比后采用，失败保留候选以便重试。放弃候选或离页不生成正式版本，不承诺跨重启恢复预览。
- 数据库使用独立 v8 迁移；备份仍使用现有 SQLite 快照和恢复流程。无新增 npm/Rust 依赖，无新增供应商协议、任意文件或网络权限。

## 创作体验与完整恢复（2026-10-25）

- `useEditorDraft` 共用于文案、封面：400ms 防抖、串行写入 SQLite `editor_drafts`，记录来源版本及未完成 JSON，不复用正式历史。修订序号避免慢写入把更新的内容误报为已暂存；显式保存前后刷新队列。路由离开及原生 `onCloseRequested` 等待写入，失败留在当前页。原生事件能力仅增加监听/取消监听和窗口销毁，按实际 Tauri 2.12.4 实现等待异步回调。
- `coverRenderer.backgroundRect` 统一计算归一化位置和1–3倍缩放，旧配置默认为居中/1倍。主画布支持指针拖动与具名滑块，另三个完整分辨率画布并排检查比例。预览/导出共用渲染器，无第二套裁切实现。
- `cover_presets` 保存风格白名单，Rust 从有效 CoverConfig 提取品牌、字体、字号、颜色、蒙层和布局；不保存书名、副标题、素材 ID、尺寸和裁切。页面应用风格后继续使用本书的同一草稿。
- 发布新任务的文字初值为空，用户明确选择历史或手填。`writingVersionId` 保存来源，正文保留独立可编辑快照；Rust 校验同书籍。`CoverThumbnail` 经受限命令读取实际导出 PNG，沿用图片解码和 CSP 已允许的 data URL，不开放任意文件或 blob 来源。
- `full_backup.rs` 创建可搬移目录：`manifest.json`、`database.sqlite3`、`files/`。由数据库快照收集登记文件与任务包固定八个文件，视频可选，64KB 流式复制并核验 SHA-256；直接声明已在 Cargo.lock 中的 sha2 0.11.1，不增加包或压缩依赖。
- 完整恢复先核验清单、哈希、schema、完整性/外键及路径边界，再复制到所选位置的新目录，重建项目目录、素材/封面路径和任务包清单、视频引用及文字文件。恢复出的 `projects/{书名}` 与 `packages/{书名}-发布任务包` 复用同一目录命名函数，重名编号。共享 `backup.rs` 的快照读取/替换流程，替换前保存当前库；失败只清理本次新建目录，原文件及源备份保留。不包含的视频保持原路径，失效就绪任务退回草稿。跨系统引用按源目录分隔符匹配，跨系统实机仍待验证。
- `note_import.rs` 只读取 TXT/MD、UTF-8/BOM，限制1 MB/10万字并规范化换行；ProjectPage 预览后追加正文及来源，随原书籍保存流程写入，不新增解析依赖或资料状态表。
- schema v9 的兼容迁移和本轮验收见 `DATABASE.md`、`CREATOR-WORKFLOW-VERIFICATION.md`。

## 应用内抖音辅助发布（2026-10-26）

- `services/douyin.ts` 封装五个主窗口命令和定向进度事件；`DouyinVerification.vue` 提供打开、检查、账号确认、上传填写、重新核对后继续和停止。`PublishingPage.vue` 在操作期间锁定编辑、任务切换与路由，沿用现有人工发布记录。
- `douyin.rs` 统一持有一个活动快照、操作锁和停止标志。`database/publishing.rs::publish_snapshot` 复用 checked/manifest/package_valid，仅接受当前 ready 任务，从有效包取横竖图、引用原视频。接收命令的主窗口不能提供任意路径或脚本。上传前、填写和设置封面期间重查任务/素材；文件目标已定位后、发送前预留任务，超时不自动重传。
- 独立 `douyin-publish` WebView 仅允许 `https://creator.douyin.com`，拒绝新窗口，使用专用 `douyin-webview` 数据目录。capability 仍只绑定 main；命令检查调用窗口，事件只发 main。主窗口销毁时关闭发布窗口；浏览器会话不进入 SQLite、备份或日志。
- Windows `webview_host.rs`（原 `douyin_webview.rs`）通过 WebView2 宿主 CallDevToolsProtocolMethod 调用 Runtime.evaluate / DOM.setFileInputFiles，释放临时节点引用，供抖音和音频共用。直接声明锁文件已有的 webview2-com 0.39.1；产品不开放远程调试端口。非 Windows 明确返回未支持。
- `douyin_page.js` 只读来源、登录/联网和编辑器就绪情况；`douyin_editor.js` 集中维护真实页面的有限动作，包含带提示图标的视频预览页签、恢复后的 Slate 编辑器、同一弹窗上传两种封面及完成后回读。没有最终发布动作；页面缺项或已有不同内容即停止。
- 视频等待上限30分钟，封面处理/保存上限120秒；停止只阻止后续操作，已送入网页的上传由官方页面控制。会话不持久化，重开先检查官方草稿并人工核对，不自动重传；schema v9 和既有备份格式不变。真实脚本、原生命令与安装证据分列于 `M6-VERIFICATION.md`。

## 日常发布任务列表（2026-10-26）

- `PublishQueuePage.vue` 从书架进入，提供待发布/草稿/已发布、搜索、缺项说明和作品链接复制，沿用同一套组件。`list_publish_queue` 只给主窗口，SQL关联书名并按创建时间排序，每项复用 checked；类型通过flatten复用PublishTask，没有第二套状态或任务表。
- 列表链接携带明确task ID，路由把它传给PublishingPage。页面只在本书已校验列表中查找，找不到时报错并不显示可操作表单；同一路径更换query经过原导航守卫后重新加载，避免默认选最新任务。来源版本及内容继续以已保存任务为准，新任务不隐式带入来源。
- `record_publish_success` 仍先提交原有记录/任务/项目事务，成功后仅释放匹配ID的内存上传快照；失败和其他任务均不释放。下一篇仍由用户点击，官方页面仍须回到空白上传页，已有草稿保护不变。版本、数据与原生安装验证见 `M7-VERIFICATION.md`。

## 全局滚动条样式（2026-10-25）

- `src/style.css` 的基础层统一页面、文本框和可滚动面板的原生滚动条：横纵均为 6px，透明轨道、灰绿色圆角滑块，悬停加深、拖动使用主题墨绿，并隐藏原生两端箭头与滚动条交角底色。
- 不支持 WebKit 滚动条伪元素的浏览器使用标准 `scrollbar-width: thin` 和相同配色；Reka Select 已隐藏的视口滚动条继续由组件管理，保留原有上下滚动按钮和键盘选择。
- 仅修改共享 CSS，无新增依赖、组件、脚本状态或数据库迁移。发布影响为 `style`，默认 patch。
- 验证：`npm run lint`、`npm run build`（含 TypeScript）、`npx vitest run`（95 项通过）及 `git diff --check` 通过。Playwright CLI 在 Chrome 有界面模式下运行本地 `output/playwright/scrollbars-verify.js`，确认页面 6px 实际宽度、滚轮与滑块拖动、文本框滚轮/键盘滚动及纵向缩放、40 项下拉列表的键盘选择，以及 360px 布局检查；页面错误为 0，截图保留在同目录的 `scrollbars-*.png`。浏览器使用隔离内存数据，未验证 Tauri 安装版及 macOS/Linux 实机。

## 音频制作与成片回填（2026-10-26）

- 历史 v10 边界：`WritingPage` 的音频页签保留 `audioPrompt`，当时只提供固定 HTTPS 入口、音频导入与打开。2026-10-28 已由本文顶部自动任务替代旧 `open_audio_tool` 单一入口；手动导入和剪映流程继续保留。
- `assets.asset_type` 增加 `audio`，文件类型为 MP3/M4A/WAV/AAC/FLAC/OGG；仍只保存原路径与文件元数据，不复制音频。`publish_tasks.audio_asset_id` 记录任务所用音频，任务包 manifest 升级到 schema 2，并保存可空音频引用。
- 发布页将“本地视频”明确为“剪映导出的最终成片”，并分别选择音频与成片。抖音上传仍只接收成片、文字和双封面，不接收音频。
- 新增纯 Rust `media` 模块，使用锁定版本 `mp4parse 0.17.0` 解析 MP4/MOV/M4V 的 `ftyp` 与 `moov` 盒。只读取元数据盒，不遍历完整 `mdat`，避免每次任务查询扫描大视频。检查最终视频时长为正、画面尺寸可读取且至少有一条音轨，不限制横竖屏比例。
- 发布检查是只读验证，不转码、不修复、不复制视频。解析失败或格式不支持时任务保持草稿，源文件和历史不变。旧任务包 manifest 没有音频字段时仍可反序列化，但旧待发布任务会因缺少音频退回草稿。
- UI 与数据迁移验证覆盖音频导入、外部入口、任务关联、v9→v10 保留、旧记录、MP4 元数据、损坏成片拒绝以及完整备份恢复。
