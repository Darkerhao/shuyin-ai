# 书引 AI 开发计划与验收标准

## 模型调用诊断：独立评审与补测（2026-11-11）

按用户要求以工程规范、需求符合性两条独立只读评审检查全部未提交分页改动。Standards初审与复审0项；Spec初审2项P2，分别为策略写入成功/回读失败后无法关闭清理，以及备份恢复结束时旧读取造成重读丢失。新增确定性回归先4失败/11通过，最小修复后复审剩余0项。

本轮产品改动仅为ModelDiagnostics组件的异步状态衔接，新增9项组件/设置页回归。schema仍为24，未新增迁移、依赖、权限、版本或Git操作。实际测试、修复代码和验收边界见[评审记录](MODEL-DIAGNOSTICS-REVIEW.md)。

本轮新执行全量`npm test`：前端518、视频40、发布/音频78、队列19通过，1项macOS跳过；Rust285通过/2项原有忽略。lint、TypeScript/生产构建、fmt/Clippy、差异检查通过。正式数据库和已安装桌面未操作。

## 模型调用诊断分页与清理（2026-11-11）

已实现SQLite真正分页、总数/页码/条数选择、固定表头与受限表高，默认20条；历史不再受最近200条展示上限限制。增加确认后清除已结束记录及0/7/30/90天保留策略，缺省全部保留，策略和首次删除同事务；应用启动后及运行期间每小时检查，始终保护running记录。

沿用统一Provider、model_calls、app_settings及现有组件和维护保护，schema24仅替换分页索引。同步分页、清理、回滚、重启与备份测试，并修正旧版备份测试夹具中的索引形状；不弱化备份结构核验。前端及数据库验证的最终命令、结果和桌面边界见[分页验收](MODEL-DIAGNOSTICS-VERIFICATION.md)。属于兼容的用户功能增强`feat`/minor，不手工升版本、提交或发布。

## 账号与合集前置：独立评审测试（2026-11-11）

已完成工程规范与需求符合性两条独立评审。Standards首审/复审0项；Spec发现1项P2：页面已选同名合集时，前置名称绕过列表唯一性检查。新增回归先失败后修复，首次自动选择必须读取唯一官方选项，已核对的同任务恢复继续复用回读。复审剩余0项。

补测账号确认失败后的重新读取、合集输入保留、暂停任务展示不冒充确认，以及合集写入失败时发布任务/关联/策略一起回滚。实际`npm test`为前端500、视频40、发布/音频78、队列19项通过，1项macOS跳过；Rust279通过/2项原有忽略；lint、TypeScript/构建、fmt/Clippy、diff检查通过。使用既有Node24.22.2及4个Vitest worker，未弱化检查。

无新增迁移、依赖、权限、版本或提交。本轮未实测真实抖音/原生整链或安装升级；下一步在新版桌面构建补验。范围、修复原因、具体命令与结果见[评审测试记录](AUTOMATION-SETUP-REVIEW.md)。

## 自动制作开始时确认账号与合集（2026-11-11）

已实现起步区域的账号昵称核对、一次确认、指定现有合集名称或明确不加入，以及本条任务的选择持久化和后续自动匹配。合集列表目前只能通过上传后的编辑页适配器读取，因此起步输入完整名称；后续仍核对官方唯一同名选项。属于用户功能增强`feat`/minor，本轮未修改版本、提交或发布。

实现复用原账号会话、自动流程、发布任务和合集适配器。账号确认从共享结果派生；首页自动等待上传页，启动前只读复核；同一窗口切页、暂停/继续不重复询问。预选合集随发布任务事务保存，原自动任务账号验证通过后恢复本次合集授权，官方页带入其他合集时可直接选择目标。账号变化、窗口重开或官方选项无法验证时仍按原流程处理，最终提交由用户完成。

修改文件：

- 前端：`src/components/AutomationSetup.vue`、`src/components/AutomationTasks.vue`、`src/services/automation.ts`。
- 后端：`src-tauri/src/automation.rs`、`src-tauri/src/douyin_flow.rs`、`src-tauri/src/database/automation.rs`、`src-tauri/src/database/publishing.rs`。
- 回归：`tests/automation.test.ts`、`src-tauri/src/database/automation_tests.rs`，以及`douyin_flow.rs`内原会话测试。
- 文档：`docs/PRD.md`、`docs/TECHNICAL-DESIGN.md`、`docs/DATABASE.md`、`docs/AI-RULES.md`、`docs/USER-GUIDE.md`、本开发记录。

数据库仍为schema23，无SQL迁移或新依赖。原input_json.selection追加兼容读取的可空collectionName，发布任务继续用douyin_collection；未创建跨账号默认值或持久登录授权。新程序兼容旧输入，旧程序回退应使用更新前备份。

验收对应：

| 标准 | 实际结果与范围 |
| --- | --- |
| 启动前指定合集，不加入仍可用 | 组件回归及浏览器键盘选择通过；空名称不能开始，提交使用去首尾空白的完整名称 |
| 昵称在开始时补齐并确认一次 | 组件、Rust会话和隔离浏览器通过；失败重试及切页返回的账号确认调用总计1次 |
| 后续沿用本任务的账号和合集 | Rust验证各阶段及同窗口续作保留确认，恢复预选合集授权；不同账号与普通手动任务不套用自动授权 |
| 保存、重开和备份保留选择 | SQLite回归通过，合集与任务绑定同事务，重复绑定失败不留下第二条发布任务 |
| 失败保留输入与成果 | 起步失败保留昵称、合集和请求标识；合集仍用原精确匹配与回读，未改为自动选择近似项 |
| 页面风格、键盘及窄窗口 | 深浅主题×1440/1366/800/360px共8组无横向溢出，缩放/减少动态效果检查通过，已目视桌面深色和窄窗浅色截图 |

实际验证命令与结果：

- `npx vitest run tests/automation.test.ts`：修复前新增3项回归失败，复现缺少合集输入、空昵称不能前置补齐和旧确认残留；最终全部包含在下列全量验证中。
- `npm test`：使用本机已有`D:/MyWork/software/shuyin-video-node/v24.22.2`置于本次进程PATH首位，`VITEST_MAX_WORKERS=4`，TEMP/TMP指向仓库`output/automation-publish-setup-tests`。前端498项、视频40项、发布/音频脚本78项、队列19项通过；1项macOS检查按原配置跳过。
- `npm run lint`、`npm run build`：通过，构建包含TypeScript检查；保留已有大于500kB分块提示。
- `. ./.tools/env.ps1`后执行`cargo test --manifest-path src-tauri/Cargo.toml --locked --quiet`：277通过、2项原有专项忽略，包含数据库迁移与备份回归。Rust测试的TEMP/TMP/SQLITE_TMPDIR也指向上述D盘隔离目录。
- 最后补齐合集授权后执行`cargo test --manifest-path src-tauri/Cargo.toml --locked douyin::flow::tests --quiet`：15项通过；`npm run check:rust`：fmt与Clippy全目标零警告通过。
- `git diff --check`：通过。
- `npm run dev -- --port 14371 --strictPort`启动隔离预览；`ego-browser nodejs -e "await import('file:///D:/MyWork/project/JIAZI-PROJECT/shuyin-ai/output/automation-publish-setup-20261111-1125/round-07.mjs')"`完成键盘及起步表单，按相同命令执行`round-08.mjs`完成布局、`round-09.mjs`完成失败重试/切页恢复、`round-10.mjs`关闭测试空间。结果JSON、截图和日志保存在同一忽略目录；页面异常0，预览服务已停止。

验证过程中的环境问题：默认Node24.11.0下的完整Vitest两项5秒超时，单独运行`npx vitest run tests/updates.test.ts tests/voice-reminders.test.ts`为34项通过，4 workers全量为498通过；默认Node的`npm run test:video`有7项被现有Windows运行时版本检查拒绝。随后使用已安装的24.22.2和上述worker设置完整通过，未更改系统Node、项目测试超时或隔离规则。浏览器早期临时IPC注入被热更新清空，最终采用固定隔离入口重新完成验收。

未执行本轮真实抖音账号/合集/成片上传、Tauri安装升级或最终发布验收；浏览器IPC夹具不代表原生或平台整链通过。下一步在包含改动的Windows桌面构建中，用本人的账号和已有合集核对一次真实准备流程，最后仍亲自检查并发布。

## 抖音话题未转为蓝色标签修复（2026-11-11）

补齐文字填写后的逐项官方话题选择与节点回读，继续任务复用已经确认的项；失败保留内容并进入原文案待处理流程。标签状态纳入最终核对，避免仅凭文字一致误报完成。无数据库、依赖、权限或版本号变更，归类fix/patch。修复前失败回归、官方空白编辑页5话题验证及实际命令见[本轮记录](DOUYIN-TOPICS-VERIFICATION.md)；正式桌面新构建和真实成片整链待验。

完成后独立Standards评审0项问题；Spec首轮1项光标边界P2已先复现、修复并复审通过，遗留0项。评审期间全量Vitest494项、Rust273项通过（2项按原配置忽略）；P2修复后相关前端92项、Rust19项、lint、类型检查、构建和Rust fmt/Clippy通过。快照及验收边界详见本轮记录。

## 自动视频局部失败恢复修复（2026-11-10）

修复暂停/待处理自动流程仍拦截视频页「继续准备字幕与分镜」的问题。局部续作保留原流程、任务和成果，支持本次模型选择，字幕阶段维持原检查点；恢复全局后接续后续步骤。无迁移、依赖或新状态，影响为fix/patch，版本由发布流程处理。根因复现、修改文件、实际验证与桌面验收边界见[本轮记录](AUTOMATION-VIDEO-RECOVERY.md)。

11月11日补充独立Standards/Spec评审，两轴各0项确定问题。全量Rust271项通过（2项按原设置忽略），另增事务故障回滚测试通过；相关前端48项、lint、含类型检查构建、Rust fmt/Clippy通过。真实桌面与外部服务边界继续单独验收，详见本轮记录。

## 自动模型重试与切换（2026-11-10）

新增用户可用的自动模型失败恢复，发布影响为feat，版本交由流水线处理；按用户要求提交至dev并cherry-pick到main。修复视频续作/活动自动任务回显默认模型，按所选模型→最多3次额外重试→其他已启用同类模型执行，保留进度与重启记录。无SQL迁移或新依赖。规则、根因和验收范围见[本轮说明](AUTOMATION-MODEL-RETRY.md)。

最终本地验证：相关前端48项、Rust自动流程38项及视频执行5项通过；lint、TypeScript、Vite构建、Rust fmt/Clippy和diff检查通过。命令及修复前失败证据见本轮说明；真实外部模型、桌面整链、安装包与发布未验证，下一步为Windows安装版故障切换验收。

## 本地书籍选择区与拖入（2026-11-10）

已将资料页低对比度小文字入口替换为上传图标、虚线拖放框和44px高/14px文字的主题实色按钮。新建与已有书籍均通过Tauri原生事件接收区域内文件，点击/拖入共用待保存列表；校验五种格式及单文件、去重、保留非空书名与笔记，空书名按文件名预填。拒绝保存/自动制作期间和区域外的拖入，卸载清理监听；正式保存与暂存继续使用既有流程。

修改文件：`src/modules/projects/ProjectPage.vue`、`src/style.css`、`tests/ui-refinement.test.ts`，以及PRD、技术设计、使用手册和本计划。无数据库、Rust、依赖、权限或版本号变更；新增资料页拖入能力属于feat/minor，版本留给发布流程。

实际验证命令与结果：

- `npx vitest run tests/projects.test.ts tests/ui-refinement.test.ts tests/theme.test.ts --maxWorkers=2`：3文件53项通过，覆盖新建/已有资料、2倍缩放、区域移出/取消/外部放下、大小写格式、重复/已绑定/多文件/错误格式、保留字段、空标题预填、保存失败、自动制作锁及卸载/延迟注册清理。
- `npm run lint`：通过，0警告。
- `npm run build`：TypeScript与生产构建通过，仍有原有大于500kB分块提示。
- `git diff --check`：通过。
- `npm run dev -- --port 14359 --strictPort`：启动隔离开发预览。执行`ego-browser nodejs -e "await import('file:///D:/MyWork/project/JIAZI-PROJECT/shuyin-ai/output/source-drop-20261110/round-01.mjs')"`建立本机IPC测试夹具，使用同目录`round-03.mjs`完成键盘与模拟原生事件检查，`round-04.mjs`完成最终布局/对比度验证并关闭测试空间。中间`round-02.mjs`因主题按钮选择器匹配两项未执行验收，已在后续脚本收窄选择器。

验收：深浅主题×1440/1366/800/360px共8组无横向溢出，按钮44px高、14px字且完整位于拖放框；两主题文字对比度分别15.17:1与6.36:1。Tab/Enter选择、取消绑定、1.5倍像素比例拖放、高亮/移出、错误格式、重复去重与保留书名笔记通过，页面异常0，未调用正式保存/素材绑定。已目视检查最终浅色桌面和深色窄窗口截图。脚本、结果JSON及截图位于忽略目录`output/source-drop-20261110/`。

上述验收标准在组件与隔离浏览器范围满足；真实资源管理器到Windows WebView2拖入、Rust测试及安装包升级本轮未执行。下一步在桌面开发版核对真实系统拖放与显示缩放。未提交、推送或发布。

## 视频制作工作台升级（2026-11-10）

完成上一轮留下的视频页切片：制作设置按任务有无默认展开/收起，真实任务前置，字幕→分镜→试片/成片顺序、就地键盘定位、紧凑进度与成果，以及深浅主题下的阅读层级。复用现有编辑、暂存和生成链路；本轮属于style/patch，无数据库迁移、依赖、权限或版本号变更。

实际检查、ego浏览器隔离验收及桌面/引擎验证边界见[视频工作台升级](VIDEO-WORKBENCH-UPGRADE.md)。

追加独立Standards/Spec评审：Standards发现并关闭1项P2，制作设置折叠原会重置环境检查用途并丢失报告；新增回归先复现，改为保留设置子树、仅卸载播放器。两轴复审无剩余确定问题。最终34文件479项前端测试、lint、含类型检查构建、16组ego展开/收起布局与环境报告保留检查通过。编辑区截图仍受ego最小化限制，Windows实机/真实引擎/安装升级未补验。

## 内容创作与发布准备视觉升级（2026-11-10）

已完成主编辑区、独立生成候选、按项/历史对照与作品交付台。素材放大复用既有组件，辅助发布按真实阶段展示，最终官方检查与实际发布记录独立。176项相关前端测试、lint、含类型检查的构建及56组深浅主题/响应式浏览器布局通过；草稿、明确取用、只读与发布边界已回归。无迁移、依赖、权限或版本号改动，按style/patch评估。详情与未执行的Windows实机/真实服务验收见[本轮记录](CREATOR-DELIVERY-STYLE-VERIFICATION.md)，下一轮再升级视频制作。

追加评审测试：修复封面标题计数遮挡与mixed来源误标为按项调整两项P2；新增回归先复现后通过。最终34文件475项前端测试、64组浏览器布局、lint、含类型检查的构建通过，范围与桌面验收边界见同一记录。

## 工作台视觉升级（2026-11-10）

完成书架立体质感与排序过渡、自动制作六阶段卡、后台任务卡、三比例封面展示和历史成品放大预览，以及按钮/导航短反馈。基于公开效果自行使用现有Vue/CSS/Reka UI实现，没有引入React、动画库或付费组件源码。

验收覆盖深浅主题、窄窗口、减少动态效果、任务真实状态、草稿保留、尺寸独立和预览焦点返回；实际命令、截图、失败重跑及未覆盖桌面范围见[视觉升级验收](VISUAL-UPGRADE-VERIFICATION.md)。无SQL迁移、依赖和版本号修改。本轮主体为style，历史封面放大为兼容的用户功能增强，未来发布可按feat/minor声明；不自动提交或发布。

追加独立Standards/Spec双轴评审：发现并修复2项P2（成果卡复用文案版本时key重复、历史预览期间保存快捷键穿透），补充先失败后通过的回归。最终34个文件474项前端测试、lint、TypeScript/生产构建、26组浏览器布局及10类交互通过；两轴复审均无剩余确定问题。桌面安装版与真实外部任务仍未验收。

## 语音提醒播放反馈（2026-11-10）

顶栏语音开关新增实际播报状态：声音开始后显示律动声波，整轮结束或停止后恢复；试听按钮播放时切换停止图标，停止后仍保持总开关开启。补充180ms图标/颜色过渡、90ms按压反馈、键盘焦点与减少动态效果支持。复用原Web Audio节点和停止入口，无计时器、新依赖或数据库变更，属于`fix`/patch，未手动升版本。验收标准与本轮实际命令见[语音提醒验收](VOICE-REMINDERS-VERIFICATION.md#播放状态与按钮动效补充2026-11-10)。

追加评审测试：Standards与Spec两轴无确定问题；新增停止期间解码、失败图标恢复和快速关闭3项回归。最终33个文件468项前端测试、lint、类型检查及真实WAV浏览器验收通过；本轮未重跑桌面安装包或Rust检查，未提交发布。

## 流程完成语音提醒（2026-11-10）

已增加顶栏全局总开关、本机记忆与试听，以及手动/自动创作共用的成功事件。内置中文短语音覆盖文案、音频、字幕、封面、视频及发布准备；全局监听支持切页，串行播放、事件去重、关闭即时停止与丢弃旧解码请求。无数据库迁移、新依赖或新增业务权限，属于兼容用户功能增强`feat`/minor，未手动修改版本。

验收标准：开启后成功任务播报；关闭不播且清空待播；重开保留偏好但不补播历史；离开任务页仍接收；失败/取消不报成功；抖音准备与实际发布明确区分。独立双轴评审发现3项P2并修复：新任务包误去重、缓存音频归档漏报、视频取消竞态误报。修复后规范与需求两轴均无剩余确定问题；前端462项、Rust263项/2忽略、视频40项、发布及音频脚本78项/1跳过、队列19项通过，lint、TypeScript/生产及Windows debug构建、fmt/Clippy通过。真实Windows本地完成事件、切页、重复导出、关闭即停与最小化播放已补验。完整命令、证据与剩余边界见[语音提醒验收](VOICE-REMINDERS-VERIFICATION.md)。

## 视频页阻止进入发布详情修复（2026-11-09）

正式安装版0.56.0现场复现：自动制作已到抖音上传，视频页仍显示 draft/transcript_ready；点击「发布详情与结果记录」时，离页暂存被后端以「执行中或已完成的视频不能暂存编辑」拒绝。确认页面没有未保存编辑后，仅刷新工作台即可读取成片状态并进入原发布详情，原抖音会话保留；随后只读观察到文案、封面和设置均已核对、等待用户最终发布，未代用户执行确认或最终发布。

代码沿用原轮询同步非running任务的外部推进，并在任务状态/运行代次变化时同步编辑器；reset解除旧暂存队列错误，保持串行写入和真实保存失败拦截。新增3项回归先失败后通过。`npx vitest run tests/video.test.ts tests/drafts.test.ts tests/douyin.test.ts tests/publishing.test.ts tests/covers.test.ts tests/projects.test.ts` 177项通过；`npm run lint`、`npm run build`（含类型检查）通过，构建仍提示大于500kB的分包警告。无数据库迁移、依赖或版本变更，属于fix/patch；未打包安装、提交或推送，正式安装版当前仅完成现场恢复，需后续版本纳入代码修复。

## 取消音频时长门槛（2026-11-09，已实施）

按用户验收中修订，移除自动制作15分钟、视频制作2小时和推荐15分钟约束。复用统一目标输入验证，按完整实际时间轴制作，实际超过目标不裁切、不变速、不自动收费重生成。新增回归先失败后通过，Rust262/Node85及lint、类型/生产/release构建、fmt/Clippy通过；真实1099.72秒旧音频由新入口读取成功。无迁移或新依赖，已在保留任务成果的候选中续作；三本完整验收仍按[本轮记录](ACCEPTANCE-20261109.md)推进。

## 真实整链验收续作（2026-11-09）

用户已授权三本不同体裁原书的真实制作与网页草稿验收，最终发布保持人工。现用已更新0.55.0及最小修复候选验证；全量工程检查通过，批准恢复后的业务数据重开一致。首册真实流程复现音频网站失焦时上传器未初始化，已用同一网页验证最小修复并通过45项音频脚本回归。三本整链、下载归档、原生拖入仍在验收，P4未交付；不提前标记全部通过。完整证据与风险见[本轮验收](ACCEPTANCE-20261109.md)。

## 自动制作 P2/P3 实施与 P4 复核（2026-11-08）

P2/P3代码已完成：同任务字幕检查点、明确校核结论/最小字段修订、三比例背景/成品导出、封面后分镜/渲染、独立摘要检查和准确成果入口。schema23保留旧成果并暂停旧授权流程，新增第三尺寸关联和渲染摘要，无新依赖或手工升版本。属于兼容功能增强feat/minor。用户随后授权先拉取dev、中文提交推送，再备份/拉取main并cherry-pick推送；已拉取上游v0.54.0，Git实际结果以分支历史和交付回复为准。

已通过相关前端、Rust、迁移/备份及工程检查，隔离Windows使用本机Provider/现成SRT实际生成三张PNG、单任务6秒1080p原声视频及精确发布材料。主计划三本真实书籍、真实账号网页准备仍待验收，不能将本机合同测试标成整链零中途操作通过。详见[P2/P3验收](AUTOMATION-P23-VERIFICATION.md)。

P4重新读取9份官方公开文档，创建/上传/查询完整契约与前次一致；尚缺本应用获批准入和原生预约、AI声明、设置回读、最终结果闭环，正式自动提交未交付，见[P4复核](AUTOMATION-P4-VERIFICATION.md)。当前保持唯一网页准备路线和最终人工发布，不增加猜测字段或第二套发布实现。下文P1/P0及旧执行记录按日期保留。

## 自动制作顺序修订（2026-11-08，P1已实施）

按用户最新要求修订唯一的[自动制作流程计划](BOOK-TO-DOUYIN-AUTOMATION-PLAN.md)：书名、作者、原书文件 → 确定目标分钟数 → 生成并保存发布文案、音频提示词及三比例封面提示词 → 音频生成归档 → 实际字幕与文案校核 → 4:3/3:4/16:9 各一张 AI 背景及准确尺寸 PNG → 同任务分镜与视频渲染 → 抖音发布准备。

P1已将目标时长和完整创作前置：已保存目标不被覆盖，未配置时推荐并保存；音频引用真实创作版本和冻结秒数。v22分开前置方案与后置字幕文案关联，统一生成执行、恢复/取消及准确版本入口，保留手工草稿。PDF/EPUB的绑定不等于文本模型读取原书。当前后续仍用原字幕/分镜、字幕文案及双尺寸封面；P2/P3完整顺序尚未全部落实。

实施顺序为 P1 前置创作与音频绑定 → P2 字幕校核与三尺寸封面 → P3 视频及网页准备闭环；P4 最终自动提交需单独通过当前官方能力及实际结果核验。当前网页交付仍准确称为「等待用户最终发布」，不以此替代完整自动发布目标。迁移、失败续作、旧活动任务处理、调用次数和 A01–A17 验收均见主计划。

P1验证包括相关前端、Rust全量与迁移/备份、lint/类型/构建/fmt/Clippy，以及隔离Windows真实IPC/本机Provider的1次推荐和1次创作、准确音频任务绑定与页面检查；详见[P1验收](AUTOMATION-P1-VERIFICATION.md)。未进行真实NotebookLM生成、抖音投稿或安装升级，未新增依赖、手工升版本、提交或推送。下方历史实施和验收按原日期保留。

## 音频归档后自动关闭网站（2026-11-08）

已补齐原生下载及缓存重试归档的成功收尾：仅在素材事务成功后销毁对应网站窗口，保留登录资料与原笔记本入口。复用既有窗口关联和操作锁，失败保留页面，不新增状态、配置、依赖或数据库迁移；属于 fix/patch，不手工升版本。

隔离 Windows WebView2 验证已复现两条路径原先归档成功但窗口未关闭，并验证修改后自动关闭、失败保留、重新查看及网站存储保留。完整命令、检查结果与验收边界见 [音频验收记录](IN-APP-AUDIO-VERIFICATION.md)。未替换正式安装程序，未执行真实 Google 生成或发布。

## 自动制作真实全流程测试（2026-11-07至11-08，已到最终人工发布）

从已安装v0.53.0和真实抖音登录窗口启动《显微镜下的大明（全流程测试）》。修复早期v20缺少上传/交接字段、官方编辑器多行输入、失败会话续接及就绪任务不重写人工确认；新增v21迁移及回归。真实音频308.57秒、132字幕、29分镜、1文案、2背景/双封面及1080p视频完成，原草稿上传填写后进入awaiting_publish；北京时间11月9日01:11、禁止保存、不加入合集已回读。最终公开提交未执行。目标10分钟不足、响度和平台手机预览占位列为限制，修复过程包含明确人工核对，不能计为零点击验收。92项聚焦前端、244项Rust（2忽略）、lint、构建、fmt/Clippy及本地release构建通过；详见[现场验收](AUTOMATION-LIVE-TEST-20261107.md)。修复属于fix/patch，不手动升版本。用户于11月8日授权先拉取代码，再以中文提交推送dev，并将本次提交cherry-pick后推送main；Git实际结果以交付回复为准。

## 自动制作到抖音最终发布准备（2026-11-07，已实现）

用户允许最后手动点击发布后，采用现有网页路线完成固定自动流程：原书音频、实际字幕/分镜、字幕文案、两比例AI背景与根级Canvas导出、同任务自动渲染、发布材料快照及抖音上传填写。默认目标10分钟、未来北京时间01:11、不允许保存、不加入合集。配置后一次启动，最终按钮及实际发布记录保持人工。

v20新增流程持久化、原子子任务绑定、去重、暂停/继续/停止、运行代次与上传意图；模型/账号/原书/音频/视频计划绑定，恢复不自动重发未知操作。原手动确认与自动策略分离，不伪造人工勾选。无新增依赖或官方接入服务。

本地测试、浏览器四档、隔离Windows Provider/Canvas/6秒1080p渲染/发布任务及重启防重发均有证据；两轴审核问题已修复复核。详见 [实施验收](AUTOMATION-VERIFICATION.md)。真实NotebookLM整书、完整长片、正式抖音账号、最终投稿及新安装包未在本轮验证，不能用本机契约结果替代。用户已授权按中文feat提交、推送dev并cherry-pick推送main，Git实际结果以交付回复为准。

## 书籍到抖音全自动流程（2026-11-07，前序官方API P0 核验）

2026-11-07 的原全自动方案曾规划：输入书籍文件、书名和作者后，串联 NotebookLM 音频、字幕、发布文案、AI 背景加本地 Canvas 排字的双封面、手绘视频与拟接入的抖音官方发布。其默认值为 AI 背景图、北京时间凌晨01:11定时发布、不允许保存视频；自动选择满足平台预约窗口的未来日期并保存完整时间，旧任务不批量改写。以下是当日规划历史，现行顺序、切片与验收以本文顶部链接的 2026-11-08 修订为准。

当时推荐先核验新版 `video.create.bind` 资格、交互要求、原生定时和禁止保存回读、AI 声明及实际发布结果查询，再按原 P1–P4 实施；当日公开文档未证实定时字段，也不能证明本应用已获准无人值守发布。原方案包含源码缺口、固定编排、最小服务、迁移草案、提交去重与恢复、18–30人日初估及20项验收；该估算与验收编号不沿用到本次修订。图片失败不静默换纯色；创建意图前的默认候选日期失效自动计算下个有效01:11，已提交或未知时先核对原预约，不改立即发布。当日规划阶段未修改产品代码、SQL迁移、依赖或版本，也未进行真实生成/投稿。

最终核查明确：前期配置齐备、每本书启动一次后，正常流程从音频/文案/AI背景采用到视频渲染、定时提交和发布结果归档均零人工点击；预算内调用、正常Token续期及预约/审核等待自动处理。新增逐环节核对表和三本书整链零点击验收。若平台要求最终人工确认，P0与全自动验收不通过，不能把半自动流程作为本需求已完成交付。

实施启动核验：用户已授权实现及完成审核后的 dev/main 交付。当前先按计划核实官方能力与真实代码；已有网页辅助填写不等于官方发布 API，源码中尚无官方授权、创建、结果查询和发布服务。公开文档证据、未决接口及所需接入资料见 [P0 核验记录](BOOK-TO-DOUYIN-P0-VERIFICATION.md)。P0 未通过前不宣布全自动可用；实现及审核完成条件未满足时，不将调查文档作为功能提交推送 main。

## 创作连续性优化（2026-11-06）

已实现用户确认的三个切片：文案/按项/推荐时长/背景生成可离页与会话内返回；资料和发布准备按所有者自动暂存与恢复；创作页首屏压缩、时长面板偏好与可读字号。窗口关闭保护统一为一个原生监听，新增v19兼容迁移保留旧草稿，正式保存事务清除对应暂存。

验收包括前端任务取消/切页/保稿、资料/发布恢复、v18迁移及备份恢复、1280×720布局和Windows隔离WebView2真实IPC/本机SSE/正常退出重启。后续双轴评审定位并修复返回页面自动重试失败推荐、旧资料保存覆盖后台时长两项P2，均先用回归复现，再验证修复。完整命令与结果见[验收记录](CREATOR-CONTINUITY-VERIFICATION.md)。本轮按feat/minor，不手动升版本；用户已授权中文提交、推送dev、cherry-pick并推送main。自动续接、多本队列及计划发布列表属于后续切片。

## 添加一本书按钮交互（2026-11-06）

按用户要求增强书架顶部添加入口：悬停上浮、主题光晕、单次扫光、加号旋转及按下回弹；适配明暗主题、键盘焦点与减少动态设置。采用页面内 CSS，保留原路由、表单及浏览器预览禁用行为。修改 `src/modules/projects/ProjectsPage.vue`、技术设计及本计划；无 SQL 迁移、依赖、权限或版本号调整，属于 `style` / patch。

实际验证命令与结果：

- `npm run lint`：通过，0 ESLint 警告。
- `npm run build`：TypeScript 与生产构建通过；保留已有大于 500kB 的分块提示。
- `npx vitest run tests/projects.test.ts tests/theme.test.ts tests/ui-refinement.test.ts --maxWorkers=2`：3 个文件、45 项通过。
- `npm run dev -- --port 14347 --strictPort`：启动独立开发预览；使用下列 Playwright 命令复用现有内存 IPC 夹具。

```powershell
$pw = 'D:/DevCache/npm-cache/_npx/31e32ef8478fbf80/node_modules/@playwright/cli/playwright-cli.js'
node $pw -s=add-book-hover open http://127.0.0.1:14347 --json
node $pw -s=add-book-hover run-code --filename output/playwright/shell-scroll/setup.cjs --json
node $pw -s=add-book-hover run-code --filename output/playwright/add-book-hover/verify.cjs --json
```

浏览器 29 项检查通过：两种主题的扫光与旋转、2px 上浮且尺寸不变、按下收缩、移出复位与重入重播、Tab 焦点、Enter/鼠标进入原新书表单、减少动态、1366/800/360px 布局、无业务写入及浏览器预览禁用。页面异常为 0，已目视检查深浅主题截图；脚本与截图保存在忽略目录 `output/playwright/add-book-hover/`。

验收标准在上述范围全部满足，无本次未解决问题。浏览器使用隔离数据，未运行 Windows 原生 WebView2 或更新正式安装包；下一步在桌面开发版或后续安装包核对实际系统缩放下的交互。未提交、推送或发布。

## 抖音保存权限与定时发布（2026-11-06）

发布准备新增允许保存开关、立即/定时选择及北京时间输入，复用原任务与确认流程。SQLite v18 增加两个字段，设置进入快照、任务包与备份，旧任务默认允许保存、立即发布。辅助发布增加设置填写、回读和最终核对，异常进入人工确认；最终提交与实际发布结果仍由用户完成。

验收覆盖页面保存/回显/只读、时间格式与过期、设置精确回读、控件缺失/禁用/重复/官方调整、v17 迁移、重开、备份及旧默认任务包兼容。按后续要求完成独立工程规范与需求审核，修复旧 review 会话复用及部分控件失读时人工确认跳过可读冲突；替换全部发布页时间控件为 Reka UI 日历和时分下拉，共用现有组件及锁内日期库。验收记录同步后按用户授权在 dev 生成中文本地提交，未推送、改版本或发布。

本地检查、原始失败回归和真实账号现场限制见[发布设置验收](DOUYIN-SETTINGS-VERIFICATION.md)。本次为兼容功能增强 `feat` / minor，版本交给现有流水线。

## dev 分支评审问题修复

浏览器预览测试原先要求整个 App 的全部按钮禁用，新增仅操作本机显示偏好的主题按钮后，该断言不再符合功能边界。现将禁用检查限定为主内容区的桌面业务操作，并验证主题按钮可点击且不会调用项目读取、保存或工作目录查询。保留原有数据库隔离断言，不禁用主题功能。

封面顶部就绪提示只检查当前尺寸的排版和背景，因此将「全尺寸排版就绪」改为「当前尺寸排版就绪」。其他尺寸背景失效时，批量导出仍保持禁用；切换到失效尺寸后仍显示「排版待核对」。在已有缺失背景测试中补充这两种状态，无新增状态或渲染路径。

修改 `src/modules/covers/CoversPage.vue`、`tests/projects.test.ts`、`tests/covers.test.ts` 和本计划。无数据库、依赖、权限或版本号变更；属于 `fix` / patch。

已先运行两项定向用例确认原问题失败，修复后 `npx vitest run tests/projects.test.ts tests/covers.test.ts tests/theme.test.ts tests/ui-refinement.test.ts --maxWorkers=2` 为99项通过；`npm run lint`、`npm run build`（含TypeScript）通过。生产构建浏览器复验通过：预览模式的桌面按钮禁用、主题切换有效、其他尺寸背景失效时当前尺寸提示准确，以及切到失效尺寸后提示待核对；页面异常0。浏览器使用隔离IPC，未操作正式业务库。

完整验证在本次PowerShell进程将 `D:/MyWork/software/shuyin-video-node/v24.22.2` 临时放在PATH首位，并设置 `VITEST_MAX_WORKERS=2` 后执行 `npm test`：前端404项、视频39项、发布脚本78项、队列19项全部通过，另1项macOS专属检查按原设置跳过；未修改系统Node配置。`git -c core.safecrlf=false diff --check` 通过。构建保留已有大分块提示；本轮无Rust检查、数据库迁移测试、原生WebView2或安装升级验收。按用户要求生成中文修复提交并提交到本地dev，不推送。

## 浅色主题切换（2026-11-05）

按用户浅色HTML参考完成共享明暗主题、右上角切换、本机偏好恢复、全页面和浮层适配；播放器默认跟随主题，保留独立OLED配色。切换不重挂载页面、清空输入、改变封面画布或打断音频。清理原局部深色硬编码并复用原组件；主题不写业务库，无schema、依赖或权限变更。

已通过lint、TypeScript/生产构建、45项相关测试、154组浏览器布局及15组交互/补充检查；修复通知挡住切换入口和着色背景文字对比度不足。命令、截图与Windows待验项见[浅色风格验收](LIGHT-THEME-VERIFICATION.md)。新增主题能力按feat/minor处理，当前版本不手动调整；按用户要求以 `feat(ui): 新增浅色主题并支持明暗风格切换` 本地提交，不推送或发布。

## 关于与更新菜单恢复（2026-11-05）

已将“关于与更新”恢复到左侧主导航“模型设置”下方，具有完整文字、信息图标与当前页面高亮；窄窗口沿用图标、标题提示及可访问名称。底部版本号改为普通展示，清理旧的窄窗口重复入口和样式。修改 `src/App.vue`、`src/style.css`、技术设计及本计划；不改更新服务、数据库、依赖或版本号。沿用用户的中文提交要求，以 `fix(ui): 恢复关于与更新侧栏菜单入口` 本地提交。

实际验证：`npm run lint` 通过；`npm run build` 的TypeScript检查与生产构建通过，保留既有大于500kB的分块提示；`npx vitest run tests/ui-refinement.test.ts tests/updates.test.ts --maxWorkers=2` 共24项通过。

使用 `npm run dev -- --port 14347 --strictPort` 与既有隔离IPC数据运行 Playwright：`node D:/DevCache/npm-cache/_npx/31e32ef8478fbf80/node_modules/@playwright/cli/playwright-cli.js -s=about-menu run-code --filename output/playwright/about-menu/verify.cjs --json`。修复前捕获主菜单缺少入口；修复后1440×900、1366×768、900×500、767×800和360×800五档通过，覆盖菜单显示、唯一入口、点击/键盘跳转、当前项高亮、更新控件与横向溢出，并确认滚动后顶部/侧栏保持固定。页面异常0；截图与记录位于 `output/playwright/about-menu/`。

验收标准已满足，无本次未解决问题。浏览器使用隔离测试数据，未更新正式安装包；下一步可在桌面开发版或后续安装包中核对实际系统缩放，未执行推送或发布。

## 工作台顶部与侧栏滚动修复（2026-11-05）

已修复长页面滚动时顶部栏和左侧菜单一起离开窗口的问题。浏览器用15本隔离测试书籍复现：正文滚动650px时，顶栏和侧栏的top均从0变为-650px。逐项验证后移除未限高工作区的overflow，恢复侧栏吸顶和视口高度约束；正文仍使用原文档滚动，低高度侧栏独立滚动且底部入口可达。

修改文件为 `src/style.css`、`docs/TECHNICAL-DESIGN.md` 和本计划；无组件、路由、依赖、数据库结构或迁移变更，schema17与版本0.50.0不变。属于fix/patch，本次按用户要求以 `fix(ui): 修复顶部栏和侧栏随页面滚动的问题` 本地提交，不推送或发布。

实际验证命令与结果：

| 命令 | 结果 |
| --- | --- |
| `npm run lint` | 通过，0 ESLint警告 |
| `npm run build` | Vue TypeScript检查与Vite生产构建通过；保留大于500kB的分块提示 |
| `npx vitest run tests/ui-refinement.test.ts tests/confirmation.test.ts tests/projects.test.ts --maxWorkers=2` | 3个文件43项通过 |
| `npm run dev -- --port 14347 --strictPort` | 启动本轮独立开发预览 |
| 下列Playwright `reproduce.cjs` | 修复前捕获650px位移并失败；修复后通过 |
| 下列Playwright `verify.cjs` | 8档窗口的正文滚动、底部导航位置、内容可达及横向溢出检查通过；低高度侧栏独立滚动、页面切换及焦点、书籍菜单和另4个路由通过，页面异常0 |

```powershell
$pw = 'D:/DevCache/npm-cache/_npx/31e32ef8478fbf80/node_modules/@playwright/cli/playwright-cli.js'
node $pw -s=shell-scroll open http://127.0.0.1:14347 --json
node $pw -s=shell-scroll run-code --filename output/playwright/shell-scroll/setup.cjs --json
node $pw -s=shell-scroll run-code --filename output/playwright/shell-scroll/reproduce.cjs --json
node $pw -s=shell-scroll run-code --filename output/playwright/shell-scroll/verify.cjs --json
```

浏览器窗口为1847×1259、1440×900、1366×768、1120×800、800×600、560×700、360×800和900×500；页底位置允许1px内的子像素舍入差。另查添加书籍、模型设置、封面工坊、发布准备。900×500窗口侧栏滚动301px时正文仍为0，数据与备份及关于入口均在视口内。记录和截图位于 `output/playwright/shell-scroll/`，由现有规则忽略。

范围与下一步：使用真实Vue页面和CSS、隔离内存IPC，未操作正式数据，未运行Windows安装版或WebView2系统缩放验收。历史返回书架时，异步列表加载后的原滚动位置恢复未通过；用改动前样式同样复现，原路由代码保持不变，不计入本次通过项。下一步在桌面开发版或后续安装包中核对实际窗口缩放下的滚轮操作；无本次导航固定问题的待修复项。

## 关于与更新设计稿重构（2026-11-05）

按用户提供的 `ai (2)-关于与更新.html` 完成关于页三卡布局、渐变品牌符号、版本徽标、自动更新滑动开关、真实更新状态、平台标签及终端风格更新日志。旧 Checkbox 和分散日志展示由同一页面的新布局替代，更新检查、设置保存、下载、确认安装与重启继续使用原流程，无新增状态存储。

完成24项相关前端回归、lint、含TypeScript的生产构建、11种状态与7档窗口的77组浏览器布局检查，以及开关键盘操作、失败保留与重试、安装确认和离页保护。修复验证中发现的空更新说明显示问题；真实Windows安装与WebView2系统缩放另验。完整命令与截图见[关于与更新样式验收](ABOUT-STYLE-VERIFICATION.md)。按用户要求以 `style(about): 按设计稿重构关于与更新模块样式` 本地提交，版本号0.50.0与schema17不变。

## 发布准备设计稿重构（2026-11-05）

按用户提供的 `ai (2)-发布准备.html` 完成发布准备页的玻璃面板、任务卡、双栏编辑、实时字数、复制工具区、自检清单与双比例真实封面预览，并补齐本书导航入口。保留草稿、素材管理、来源替换确认、导出、辅助发布与人工结果记录；辅助发布空闲区域按需显示，恢复会话和错误继续可见。

验收包含64项相关前端回归、lint、含TypeScript的生产构建、五种任务/素材状态与七档窗口的35组浏览器布局，以及复制、保存、素材展开、导出、辅助发布、记录和失败保留检查。证据与Windows实机边界见[发布准备验收](PUBLISHING-STYLE-VERIFICATION.md)。本轮按 `style(publishing)`（patch）提交，版本号和schema17不变，交付为本地提交。

## 视频制作设计稿重构（2026-11-05）

前序23文件已按用户要求提交为 `1b59a1a feat(ui): 完善科技风格创作页面并支持封面按尺寸导出`。随后按 `ai (1).html` 改造视频模块的区域布局、紫色导航、任务状态、字幕对照与分镜卡片；现有原声、草稿、单镜候选、试片、渲染和发布流程继续使用。

本轮视频改动按style/refactor（patch）处理，无数据库、依赖、版本号或权限变更。相关回归、lint、生产构建及浏览器多状态/多宽度证据统一记录在[视频页面验收](VIDEO-STYLE-VERIFICATION.md)。用户随后明确要求提交，本次以 `style(video): 按设计稿重构视频制作工作台样式` 归档代码、测试、文档和截图；交付范围为本地提交，不推送。

## 第二版科技页面补齐（2026-11-05）

完成 `ai-2.html` 中封面工坊、发布任务、模型设置与工作目录/备份的布局，补齐封面导航；首批页面继续沿用之前实现。尺寸选择导出接入原 PNG 渲染与版本保存，不增加第二套工作流。验收包括截图对照、五档窗口、筛选/弹窗/保存/导出、减少动态与透明效果、相关测试、lint 和生产构建，证据及桌面实机边界见[第二版验收](TECH-STYLE-V2-VERIFICATION.md)。

视觉与内部清理使用 style/refactor；勾选导出是兼容的用户功能增强，包含该能力的合并发布应按 feat/minor 声明一次。无依赖、权限或数据库迁移，不手工升版本、不提交或发布。

## 参考稿科技页面升级（2026-11-05）

首批源码已按ai.html完成应用框架、我的书籍、书籍资料和发布文案/音频制作/封面素材。采用原稿的尺寸、字体与材质，保留真实业务状态、换行、草稿、历史、媒体和发布边界；其他路由完成共享深色适配。

属于style/refactor/fix（patch），无新增业务能力、SQL迁移、依赖、权限、手工版本调整或发布。实际验证、对照截图与未覆盖项集中于[本轮验收](TECH-STYLE-UPGRADE-VERIFICATION.md)，当前规范见[最终方案](TECH-STYLE-UPGRADE-PLAN.md)。历史浅色验收记录继续保留其原时间与范围。

## 发布记录弹窗样式优化（2026-11-04）

按用户截图优化“记录发布成功”弹窗：原下一篇选择框按内容收缩，接续按钮紧贴选择框且与保存底栏分离，标题、说明和字段之间的间距偏大。已收敛为最大600px白色弹窗，统一字段留白与选填标签；下一篇使用独立浅绿区域、全宽选择框和说明文字，接续按钮以描边样式与保存、取消一起放在分隔线下方。窄窗口按钮纵向排列，低高度窗口沿用原弹窗内部滚动。

浏览器检查进一步复现长选项撑宽菜单：560px窗口中菜单宽704.72px、选择框宽452px。已将这一处菜单宽度绑定到Reka公开的触发器宽度变量，长选项在菜单内换行。两种保存都经过同一表单原生校验；回车只保存本次记录，只有明确点击接续按钮才启动所选下一篇。复用原输入、任务与记录成功后的路由流程，无新增状态、依赖、权限或数据库迁移，schema仍为v17，版本影响为style/fix（patch）。

修改文件：`src/components/ConfirmAction.vue`、`src/modules/publishing/PublishingPage.vue`、`docs/TECHNICAL-DESIGN.md`和本计划。没有修改共享主题、其他页面布局、Rust或数据库结构。

实际验证：

| 命令 | 结果 |
| --- | --- |
| `npm run lint` | 通过，0 ESLint警告 |
| `npm run build` | TypeScript检查与Vite生产构建通过；保留既有大于500kB的分块提示 |
| `npx vitest run tests/publishing.test.ts tests/publish-queue.test.ts tests/confirmation.test.ts tests/maintenance.test.ts --maxWorkers=2` | 46项通过，覆盖发布记录、下一篇接续、共享确认和备份操作 |
| `npm run preview -- --port 18596 --strictPort` | 启动本轮生产预览 |
| `node D:/DevCache/npm-cache/_npx/31e32ef8478fbf80/node_modules/@playwright/cli/playwright-cli.js -s=publish-record-style run-code --filename output/playwright/publish-record-style/setup.cjs --json` | 加载独立内存Tauri IPC和本地示例封面，供界面验证使用 |
| `node D:/DevCache/npm-cache/_npx/31e32ef8478fbf80/node_modules/@playwright/cli/playwright-cli.js -s=publish-record-style run-code --filename output/playwright/publish-record-style/check-layout.cjs --json` | 1440×900、1366×768、1120×800、800×900、560×900、375×900、900×500共7组通过；全宽选择框、长标题与菜单无横向溢出，底栏响应式排列，初始取消焦点、取消/Escape、焦点返回和减少动态效果通过，页面异常0 |
| `node D:/DevCache/npm-cache/_npx/31e32ef8478fbf80/node_modules/@playwright/cli/playwright-cli.js -s=publish-record-style run-code --filename output/playwright/publish-record-style/check-actions.cjs --json` | 两个确认按钮阻止空时间和非法链接；记录失败保留输入及下一篇，处理中禁用表单和Escape，重试写入成功后只接续一次；普通保存与回车不接续，空队列可正常保存，页面异常0 |
| `git diff --check` | 通过 |

验收标准均已在上述范围验证：下一篇区域与表单对齐、按钮层级清晰、窄/短窗口可操作、长选项不溢出、既有记录和接续流程保留。浏览器报告与截图位于`output/playwright/publish-record-style/`，含`preview.png`、`selected-1366.png`、`selected-375.png`、`layout-cli-result.json`和`actions-cli-result.json`。

本轮仅验证本地前端与隔离内存业务数据，未操作真实抖音账号、正式用户发布记录或Windows原生窗口/SQLite，未更新安装包、修改版本号、提交或发布。下一步在包含本次源码的Windows桌面构建打开“记录发布成功”弹窗，核对实际系统缩放与原生日期输入。

## 发布标题与双封面确认入口修复（2026-11-04）

改版将人工确认收敛到「确认并开始辅助发布」与「确认并导出任务包」，但已填写标题的检查入口仍定位到输入框，用户指南也保留了旧勾选步骤。已在发布自检卡明确两项确认方式，将已填写标题与封面的待处理入口改为「去确认」，任务包入口改为「去准备任务包」。标题为空时定位输入框；素材不齐时确认和任务包入口定位到说明与缺项提示，操作继续禁用。

复用原确认字段与保存、封面验证、任务包、辅助发布流程。点击定位入口不保存或确认；明确导出/开始操作才确认当前组合。无数据库迁移、依赖或权限变更，schema仍为v17，影响为fix/patch。修改发布页、发布测试、技术设计、用户指南与本计划。

实际验证：

| 命令 | 结果 |
| --- | --- |
| `npx vitest run tests/publishing.test.ts -t 'takes pending confirmation'`（修复前） | 1失败、1通过；已填写标题的入口错误聚焦输入框，复现用户反馈 |
| `npx vitest run tests/publishing.test.ts tests/publish-queue.test.ts tests/douyin.test.ts --maxWorkers=2 --reporter=json --outputFile=output/playwright/publish-confirmation/unit-tests.json` | 48项通过，含标题/封面确认定位、三类缺素材入口、确认保存和原失败保稿回归 |
| `npm run lint` | 通过 |
| `npm run build` | 通过，包含TypeScript检查；保留既有大于500kB的块大小提示 |
| `node D:/DevCache/npm-cache/_npx/31e32ef8478fbf80/node_modules/@playwright/cli/playwright-cli.js -s=publish-confirmation run-code --filename output/playwright/publish-confirmation/check.cjs --json` | 生产预览18592：1440、1366、1120、800、375px共5档通过，无横向溢出；标题/封面入口正确聚焦，导出后确认与保存检查同步，空标题和缺素材正确阻止，减少动态效果下通过，页面异常0 |
| `git diff --check` | 通过 |

浏览器使用隔离的内存Tauri IPC及本地示例封面；报告和截图保存在`output/playwright/publish-confirmation/`。本轮未更新安装包、操作真实抖音账号或验证Windows原生窗口/SQLite；未修改正式用户任务，未提交或发布。下一步在包含修复的Windows构建打开现有任务，核对实际标题和两张封面，再用上述确认按钮准备发布。

## 原声波形播放器（2026-11-04）

按用户提供的现代播放器HTML与截图，实现共用玻璃面板、深墨绿/OLED配色、真实幅度波形、悬停与拖动寻迹、发光指针、EQ徽标及完整播放控制。下载成功任务卡直接试听，手动导入与历史成品列表、视频制作和逐句/分镜校对复用同一组件。保留本书素材校验、原声时间码与自动草稿；没有新依赖、IPC或SQL迁移，schema17。新增就地试听及波形控制属于feat/minor，不手动升版、提交或发布。

验收标准、修改文件、实际命令、真实本地音频与Windows原生结果记录在[原声播放器设计与验收](AUDIO-PLAYER-VERIFICATION.md)。该报告区分内存业务夹具和真实Tauri/SQLite验证；浏览器或本地成品试听不能表述为重新通过Google生成/下载、正式安装更新或两小时音频性能验收。

## AI发布标签（2026-11-04）

完成同一次文案生成中的独立tags字段、统一格式解析和500字校验；WritingPage复用原字段配置支持编辑、复制、自动暂存、历史对照/取用、流式候选及只调整标签。发布准备选择来源时同时带入标签，替换确认覆盖仅已填写标签的情况；辅助发布继续由既有适配器把标签追加到正文末尾，并回读后继续。Prompt版本为publishing-v7/publishing-revision-v6，无额外AI请求、新依赖或SQL迁移，schema17；属于feat/minor兼容增强，未手动升版。

验收标准：新生成标签与书籍主题相关且为 `#标签`；旧文案可读写及按项补齐；未选中字段保持原值；标签编辑/复制/版本/草稿恢复及备份保留；取消、失败和替换取消保稿；所选来源标签进入发布快照，抖音填写及继续不重复追加，最终发布由用户完成。

修改文件：`src/types/ai.ts`、`src/modules/writing/WritingPage.vue`、`src/modules/publishing/PublishingPage.vue`；`src-tauri/src/ai.rs`、`src-tauri/src/ai/{publishing,revision,preview}.rs`、`src-tauri/src/database/{m2_tests,m4_tests}.rs`；新Prompt `prompts/publishing-v7.md`和`prompts/publishing-revision-v6.md`；`tests/{ai,publishing,douyin-editor}.test.ts`；`docs/{PRD,TECHNICAL-DESIGN,DATABASE,AI-RULES,USER-GUIDE,DEVELOPMENT-PLAN}.md`。旧Prompt保留追溯，无新依赖、权限或迁移文件；未提交、发布或修改版本号。

实际验证：

| 命令 | 结果 |
| --- | --- |
| `npx vitest run tests/ai.test.ts tests/publishing.test.ts tests/douyin-editor.test.ts --maxWorkers 2` | 140通过；覆盖生成、失败保稿、标签调整采用、草稿恢复、来源切换确认、发布快照与标签追加一次 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked` | 214通过、2项既有专项忽略；覆盖新标签格式/长度、只改标签、原始旧JSON不重写、备份恢复与重启、三书文案/发布任务保留 |
| `npm run lint` | 通过，0 lint警告 |
| `npm run build` | TypeScript检查与Vite构建通过；保留既有超过500kB分块提示 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | 通过 |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets -- -D warnings` | 通过 |
| `git diff --check` | 通过 |
| `node D:/DevCache/npm-cache/_npx/31e32ef8478fbf80/node_modules/@playwright/cli/playwright-cli.js -s=shuyin-tags run-code --filename output/playwright/publishing-tags/setup.js` | 隔离浏览器加载当前源码及内存Tauri IPC夹具 |
| `node D:/DevCache/npm-cache/_npx/31e32ef8478fbf80/node_modules/@playwright/cli/playwright-cli.js -s=shuyin-tags run-code --filename output/playwright/publishing-tags/check.js --json` | 两页各1440/1120/800/375px共8组布局通过；标签有可访问名称和说明，编辑/复制/版本保存/来源带入/发布保存及键盘聚焦通过，页面错误0 |
| `npm run desktop:build -- --debug --no-bundle --config output/playwright/publishing-tags/tauri.native.conf.json` | 当前源码完成TypeScript/Vite与Tauri debug构建，保留既有500kB分块提示 |
| `node output/playwright/publishing-tags/native.cjs` | 隔离Windows WebView2完成本机Provider补旧标签、采用版本、编辑保存、来源带入、发布草稿与正常重启；原始旧JSON未被改写，最新版本使用`publishing-revision-v6`，`user_version=17`、integrity ok、外键异常0 |

Rust命令先执行 `. ./.tools/env.ps1`；测试进程TEMP/TMP/SQLITE_TMPDIR指向仓库 `output/publishing-tags-test-temp`。实施前回归明确失败：旧Rust拒绝未知tags字段，前端缺少标签编辑/调整/预览和来源带入；实施后上述最终命令通过。浏览器结果已解析核对，JSON与已查看截图位于 `output/playwright/publishing-tags/`，没有把CLI退出码单独当作成功证据。

本轮自动化与Windows原生闭环验收通过；3—5个标签及语义相关性由Prompt要求，真实模型质量仍需实际书籍试用。浏览器模拟IPC，Windows隔离构建使用本机契约模型与独立SQLite；未访问用户数据库、未调用外部模型、未打开抖音官方页面，实际平台话题识别、安装升级和跨平台仍未验收。下一步使用包含本次改动的正式桌面构建，为一份真实文案生成标签并检查官方发布预览。

## 书架十二套稳定配色（2026-11-04）

按用户确认方案，将书架四套示意封面配色扩为十二套，集中维护深书封/浅背景配对，并用书籍ID的FNV-1a哈希稳定选择。移除旧字符求和和`tone-*`样式，网格/列表共用同一组件；改名、排序、筛选及重开保持颜色。已有书籍采用新规则后重新分配一次，正式封面与业务数据保留。无新增依赖、权限或数据库迁移，属于style/patch，未手动修改版本。

实际验证：`npm run lint`通过；`npx vitest run tests/ui-refinement.test.ts tests/projects.test.ts`的39项通过；`npm run build`包含TypeScript检查并通过。Playwright CLI在18592端口的最终dist页面使用96本内存示例，核对全部十二套真实计算样式，最低文字对比度5.02:1；排序、搜索、状态筛选、网格/列表、刷新及改变书名/作者/状态后的颜色保持一致。1440×1000、1366×900、800×1000、360×900四档窗口无横向溢出，页面异常0，十二色截图已逐张查看。

本轮浏览器仅模拟Tauri IPC，未访问或改写用户数据库；脚本、JSON及已查看的截图位于忽略目录`output/playwright/book-colors/`，检查最终JSON结果无isError及页面异常。构建保留既有500kB块大小提示，npm保留既有python/msvs_version用户配置提示。未重跑Windows安装、系统缩放、升级或外部服务验收，未提交或发布，保留此前已暂存工作。

## P3流式候选预览（2026-11-04）

用户明确要求开始P3后，完成统一Provider正文观察、请求专属Channel、只读文字投影和创作页候选区。沿用原单次请求、300秒总上限、取消、完整校验与历史保存；预览不覆盖当前稿、不进入自动草稿，失败清除，按项仍须明确采用。无SQL、新依赖、权限、Prompt或手动版本调整，属于feat/minor。

验收：前端367项、Rust210项通过（2项既有忽略），lint、fmt/Clippy和Release无安装包构建通过；Windows真实IPC/Provider SSE/SQLite六组成功与失败流程通过，四档CSS窗口无横向溢出。此前原生文件拖入仍待人工核对，不补记通过；未更新正式安装版或调用真实外部模型。详见[P3验收](P3-STREAMING-PREVIEW-VERIFICATION.md)。


## 视觉规范收尾与Windows验收补充（2026-11-04）

修正视频进度区浅色底仍使用旧浅色文字的问题，移除持续流光/装饰渐变，复用主题变量与180ms进度过渡。58项相关回归、lint、Release NSIS构建通过；重新执行60组浏览器布局与交互，以及Windows设置中真实100%/125%/150%缩放的27组安装版页面检查。原生另存、同版覆盖安装和正常重启通过。无SQL、依赖、权限或业务状态变化，属于style/fix（patch）。

原生跨窗口拖入手势仍待人工核对，未冒充通过；正式旧版本升级、跨平台和外部服务仍单独验收。P3保持按需后续，不提前实施。详见[补充验收](WINDOWS-VISUAL-ACCEPTANCE.md)。


## 书架菜单视觉复核与修复（2026-11-04）

用户截图复现菜单透明、边界消失及More使用整张卡片定位的问题；原交互检查没有覆盖展开后的计算样式。修复scoped浮层挂载，复用一套DropdownMenu区分按钮锚点与右键坐标，保留原目录、导出、确认删除和焦点返回。补齐菜单可访问名称、更换明确的删除图标，缩短书架搜索占位文本以完整显示ISBN。

验收：不透明白底、1px边界、10px圆角和阴影实际生效；五档窗口不裁切；按钮/右键/菜单键/Shift+F10/Enter/Space/方向键、长书名、列表视图及取消删除通过。开发服务和最终生产构建页面均执行真实CSS检查；相关39项单测、lint和构建（含TypeScript）通过。补查当前Windows开发运行版真实14本书架的展开菜单、成片项与键盘开关；未重跑安装包及系统缩放验收。没有数据库、权限或依赖变更，属于fix/patch。此前P0–P2总体完成结论已更正，详见[本次验收](BOOK-MENU-VERIFICATION.md)。


## 视觉与创作交互优化（2026-11-03规划，2026-11-04实施）

已将本轮建议整理为 [视觉与创作交互优化方案](UI-UX-REFINEMENT-PLAN.md)：采用浅色侧栏与暖灰工作区、减少外框、精修书架封面，压薄五项导航；随后增强成果状态、快捷键、版本对照、复制与保存反馈、封面行内编辑、视频试听联动、发布前自检、文件拖入与上下文菜单。

P0/P1/P2已按切片实施，复用现有历史版本、草稿、背景拖动、字幕试听及辅助发布流程。已实现版本对照/取用、画布行内编辑、连续原声与分镜联动、自检一次核对、侧栏偏好、单文件拖入、右键/键盘菜单及成片另存。来源文件关联与正文解析明确区分；最终发布仍由用户在官方页面完成。P3逐字候选预览按方案为按需后续能力，本轮只完成真实等待和停止反馈。

无数据库迁移、新依赖、手工版本调整、提交或发布；侧栏偏好复用app_settings。涉及兼容新增用户能力，发布影响为feat/minor。已完成隔离浏览器及Windows WebView2/IPC/SQLite、原件摘要与正常重启检查；最终命令、逐项验收和未覆盖的安装/外部服务/跨平台边界见[本轮验收](UI-UX-REFINEMENT-VERIFICATION.md)。

## 已加入合集仍等待的修复（2026-11-03）

修复读取已选合集时把相邻“共24个作品”拼入名称的问题。页面适配器统一读取名称字段，保留现有已选值精确核对、不同选择授权和最终人工发布流程。

验收要求：带作品数的已选合集确认一次后进入检查发布；官方下拉收起时不重新选择；作品数变化后恢复不重新确认或上传；更改为不同合集仍等待明确决定。无依赖、数据库迁移或版本号变更，归类 fix/patch。实际验证与安装版边界见 `DOUYIN-FLOW-VERIFICATION.md`。

## 抖音一次账号确认与昵称预填（2026-11-03）

修复同一头像的 CDN 域名与显示尺寸变化被当作换号的问题，并把首次确认安排到官方上传页就绪后。最近一次人工确认昵称保存至既有设置，页面昵称缺失时自动预填；没有历史昵称时填写一次。原账号确认、停止和合集流程继续共用，不增加第二份确认状态。

验收标准：首页不额外确认；点击一次账号确认后自动上传/填写；同头像跨 CDN/尺寸不重复核对；正常继续不重复上传；重启自动带入昵称但仍要求一次明确确认；实际换号清除旧证明。无数据库迁移、依赖、权限或版本号变更，属于 fix/patch。实际命令、结果和真实账号/安装版边界见 `DOUYIN-FLOW-VERIFICATION.md`。

## 剩余三项创作体验（2026-11-03，已完成）

完成准确续作/全局运行提示、视频缓存占用及预览确认清理、最近成功备份及7天提醒、统一模型请求诊断。沿用已有任务、素材、备份和Provider流程，无新依赖；新增model_calls表，schema16→17。属于feat/minor兼容功能增强；当前源码版本仍0.44.0，不手动升版、提交或发布。此前“下一切片”“随后”的清单为当时计划，本节记录本轮实际完成范围。

验收目标与结果：

- 书架和全局入口直达指定视频、音频、发布任务；活动任务优先，无重复持久状态。真实WebView2/IPC路由通过。
- 先预览再清理；拒绝运行中和过期清单，保留已登记文件、运行输出和链接目标。Windows文件回归及实际清理通过；删除72个文件/17,417,262字节后，原计划重新生成1920×1080、20秒试片并播放，素材数量不增加。
- 成功备份记录时间，失败不刷新；隔离完整备份和强制重启回读通过，7天提醒由前端回归覆盖。
- 各阶段真实请求计时和服务用量记录；SSE用量、缺省NULL、HTTP524、取消和重启持久化通过，v16迁移及备份恢复中断通过。不保存正文和密钥。

实际命令：前端335项、Rust200项通过/2项原有专项忽略；lint、typecheck、Clippy、fmt、Vite及隔离Tauri debug/no-bundle构建通过。详情及未执行检查见[剩余三项验收](CREATOR-COMPLETION-VERIFICATION.md)。下一步建议进行包含本次改动的安装升级验收及真实服务试用；PR/开发分支CI和Node要求统一可作为独立工程维护切片。

## 单镜重新生成（2026-11-03）

完成按镜选择、修改要求/模型选择、候选对照、采用/放弃/停止的切片。后端固定镜头范围并拒绝多镜/非法物体/首锚点错误/图片替换；保留相邻上下文、原字幕和其他编辑。采用进入原自动草稿，正式保存及试片复用。无SQL迁移、新依赖或独立渲染实现，schema16，影响feat/minor；不手动升版、提交或发布。

验收：生成/失败/停止/放弃不改正式计划；采用只替换目标镜，保留其他镜头未保存修改与全部字幕；原任务或编辑变化时拒绝过期候选；采用后暂存、重启、正式保存及原声试片闭环。自动化与隔离Windows验证、真实外部模型边界见[单镜验收](VIDEO-SCENE-REVISION-VERIFICATION.md)。

## 视频环境检查与桌面恢复验收（2026-11-03）

完成原自动草稿的隔离Windows桌面补验：15分33秒/469条字幕/72镜任务通过真实WebView2、IPC和SQLite暂存，强制结束进程后恢复文本/镜头/筛选，任务切换不串稿，试听在句尾暂停，正常关闭后光标恢复；正式计划保持不变。原音频仅复制为隔离测试输入，不操作正式任务。

新增环境一键检查，复用原配置和渲染所用依赖入口；检查七项、已有字幕跳过语音环境、空项识别、显式填入、错误修复说明和配置变化失效。无新依赖或数据库迁移，schema16，影响为feat/minor；不手动升版、提交或发布。适用自动化、隔离桌面和未覆盖边界见[环境验收](VIDEO-ENVIRONMENT-VERIFICATION.md)。下一切片为单镜重新生成。

## 视频自动草稿与字幕对照试听（2026-11-03）

落实本轮优先切片：按视频任务自动暂存/恢复字幕、分镜及编辑位置；逐句原文与校对文对照、只看修改项、原声时间窗试听和单句恢复。复用现有暂存队列、视频任务和正式保存入口，v16增加草稿及原文快照两列；无新依赖。兼容用户能力增强，发布影响为feat/minor，不手动升版、提交或发布。

验收：暂存后重开保留未完成文字及位置；不同任务互不覆盖；慢写入/失败保稿，失败阻止切换及离页；正式保存独立且失败不清草稿；单句编辑/恢复保持其他字幕和时间码，试听绑定任务原声并在句尾停止；长字幕控件数量有界，旧原文缺失明确提示。实际命令、浏览器证据及桌面安装版边界见[视频草稿验收](VIDEO-DRAFTS-VERIFICATION.md)。运行环境检查及其余“随后”项目继续作为后续切片。

## 长SRT编辑与开镜主体修复（2026-11-03）

修复字幕逐字输入触发整片分镜控件更新和全量JSON序列化的问题；卡片移为独立组件，继续使用同一分镜状态，缓存分镜快照，输入不防抖、不延迟保存。物体/动作/锚点折叠区改为展开才创建控件，收起不丢已选数据。唯一分镜Prompt升级为v5，生成边界校验每镜首物体cue=first，后文结论不提前；违反时整批拒绝并保留已有成果。人工和旧分镜仍可保留有意停顿，原声/字幕/渲染时钟不改。

验收：输入不更新其他分镜控件、不重复序列化分镜，立即保存包含最后输入；折叠区首次关闭不创建选择器，展开修改、收起再展开及保存保留选择；还原、失败保稿、离开保护、试片失效继续有效；新生成首镜/中间镜/非零批次首主体均按第一条字幕出现，后续结论锚点保留；结构校验和旧分镜兼容性不放宽。属于fix/patch，无SQL迁移、依赖或权限变化，不手动升版本或提交。实际命令、性能对照和未验收边界见[本次验收记录](VIDEO-EDITING-VERIFICATION.md)。

## 原声字幕自动校对（2026-11-02）

实现：本地转写或读取SRT后，使用当前文本模型逐批校对全文，再生成分镜。每批60条附相邻上下文，首尾参考优先来自对应音频任务冻结提示词，无关联时读取本书最新已保存音频提示词；只提取明确首尾段落。校对Prompt独立版本化，不以指导补台词、不改时间码/条数，不确定文字保留供试听。

`subtitleReview`与cue在原plan_json中保存参考、版本和连续已完成条数；原始SRT独立保留，完成后另存校对SRT。停止/失败/重启可以继续，整批结构错误不改保存内容。原声字幕步骤显示实际校对进度与最近结果。旧任务已有分镜时继续保留；主动重新生成分镜会先重新校对全文。schema15不变，无SQL迁移、依赖或权限变化，属于feat/minor兼容功能增强，不手动升版本、不提交或发布。

修改文件：`prompts/subtitle-proofread-v1.md`、`src-tauri/src/video/subtitles.rs`、`src-tauri/src/{video,video_commands,ai}.rs`、`src-tauri/src/database/{audio,video}.rs`、`src/types/video.ts`、`src/modules/video/{VideoPage,VideoJobProgress}.vue`、`tests/video-progress.test.ts`与`docs/{PRD,TECHNICAL-DESIGN,DATABASE,AI-RULES,USER-GUIDE,DEVELOPMENT-PLAN}.md`。

验收对应：

- 首尾参考：覆盖三个已存在音频模板、无固定段落、优先使用音频冻结提示词、无音频任务使用本书最新保存版本、跨书隔离及未知作者留空。
- 全文与时间轴：覆盖65条跨两批输入，前后上下文不混入输出，所有文字逐条返回，条数/索引/时间码/SRT时间保持不变。
- 异常保稿：漏条、重复、乱序、越界、额外时间码、非法长度/控制符整批拒绝；模拟第二批失败保留首批60条，重启后只请求剩余5条，旧run_id及取消拒绝落库，原始SRT不覆盖。
- 页面与后续使用：已校对条数和最近三条只读保存结果，首批返回前不显示未校对字幕，校对阶段归属于原声字幕步骤，失败显示明确阶段；完成校对SRT登记后进入storyboard，重复继续不重新校对，显式重建清除旧游标。

实际验证（Rust命令先`. ./.tools/env.ps1`）：

| 命令 | 结果 |
| --- | --- |
| `npx vitest run tests/video-progress.test.ts tests/video.test.ts tests/video-storyboard.test.ts tests/video-image.test.ts` | 4个文件、39项通过 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked` | 181项通过、2项既有测试按原设置忽略（原生凭据、视频子进程专项） |
| `npm run lint` | 通过 |
| `npm run build` | TypeScript及生产前端构建通过，保留已有npm用户配置与包体积提示 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | 通过 |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets -- -D warnings` | 通过 |
| `git diff --check` | 通过 |

首轮`cargo test --manifest-path src-tauri/Cargo.toml --locked subtitle -- --nocapture`为8通过、1失败：新增音频快照测试夹具遗漏audio_jobs.run_id必填值，补齐夹具后上述全量测试通过；未放宽数据库约束。

未调用真实外部模型、未替用户处理截图中的音频、未进行人工桌面/安装包验收，也未更改正式任务数据。模型回归使用内存TextProvider，现有HTTP契约测试同时随Rust全量通过；不能据此保证真实长音频的纠错质量。下一步在包含该改动的桌面构建中对一份实际音频运行，试听首尾、人名与跨批次正文，并对照原始和校对SRT。

## 文本流式接收与视频524排查（2026-11-02）

确认了客户端固定非流式请求，以及复用字幕时首批分镜请求失败仍显示preparing两个问题。截图的524属于网关响应；本机未找到截图对应644条字幕的任务，不掌握当次推理档位、服务商日志或首包耗时，不能认定是推理参数不兼容。

实现：统一文本Provider请求stream=true，严格有界SSE接收，仅完整正文进入原有校验/保存；保留配置的推理强度和300秒总上限，不自动降档或重试。服务实际回完整JSON时读取同一次响应。字幕登记事务同步进入storyboard，续作错误定位准确且已有内容保留。无数据库迁移、依赖、权限或版本号变化，属于fix/patch。

回归先红后绿：`cargo test --manifest-path src-tauri/Cargo.toml --locked ai::tests::streaming -- --nocapture`在旧实现下2项失败，分别得到524和SSE不是JSON；本机契约网关明确模拟非流式等待全文超时，不代表复现真实服务商根因。`cargo test --manifest-path src-tauri/Cargo.toml --locked resumed_subtitles_enter_storyboard -- --nocapture`在旧实现下失败，实际阶段preparing，修复后为storyboard。

实际检查（Rust命令先`. ./.tools/env.ps1`）：

| 命令 | 结果 |
| --- | --- |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked` | 176通过、2项按原设置忽略；覆盖流式UTF-8分块、推理与正文分离、结束标记、断流/异常/超长、总超时、524/504脱敏及续作保稿 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | 通过 |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets -- -D warnings` | 通过 |
| `npm run lint` | 通过 |
| `npm run build` | TypeScript及生产构建通过；既有大包提示、npm用户配置提示保留 |
| `cargo build --manifest-path src-tauri/Cargo.toml --locked` | 本地开发可执行文件编译通过 |
| `git diff --check` | 通过 |

验收：请求流式、默认不发送推理参数、显式档位原样发送、错误不自动重试、完整结束前不交付正文、续作失败阶段与字幕保留均通过本地测试。未调用真实外部模型、未跑用户的20分钟视频、未安装/发布构建或修改用户任务。流式仍可能被上游首包慢或网关缓冲阻断，不能承诺彻底消除524；需要在包含修复的构建中继续原任务做真实服务验收。

修改文件：`src-tauri/src/ai.rs`、`src-tauri/src/ai/stream.rs`、`src-tauri/src/ai/tests.rs`、`src-tauri/src/database/video.rs`及`docs/{PRD,TECHNICAL-DESIGN,DEVELOPMENT-PLAN,USER-GUIDE}.md`。

## 分镜重试模型切换修复（2026-11-02）

根因：模型选择只传给新建任务，继续及重新生成入口未传当前providerId，后端复用了旧绑定。修复贯穿VideoPage、services/video、continue_video_job和claim_video_job，复用既有字段并原子更新，不新增依赖、权限或迁移，影响为fix/patch，不手动升版本。

验收：切换模型后两种准备入口均使用新模型；继续保留已有字幕/分镜，重新生成维持原清空分镜语义；无效模型及并发启动失败不改原任务；渲染/试片保持原流程。已先执行 `npx vitest run tests/video.test.ts -t 'uses the newly selected model'`，旧实现两项均失败，实际只传任务ID和render，遗漏m2；修复后以下检查通过：

| 命令 | 结果 |
| --- | --- |
| `npx vitest run tests/video.test.ts tests/video-storyboard.test.ts tests/video-progress.test.ts tests/video-image.test.ts` | 4个文件、37项通过 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked database::video::tests` | 12项通过 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked` | 169项通过、2项按原设置忽略 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | 通过 |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets -- -D warnings` | 通过 |
| `npm run lint` | 通过 |
| `npm run build` | TypeScript及生产构建通过；保留既有大包体积提示 |

Rust命令先加载`. ./.tools/env.ps1`。前端使用组件与IPC服务夹具，Rust使用隔离SQLite，覆盖新地址/模型ID、保留部分计划、无效模型拒绝及并发认领失败不改绑。未调用真实外部模型、未改正式数据、未替换安装版，未提交或发布。验收闭环为本地源码行为；升级到含修复的新构建后，在原任务选择模型并继续，核对真实服务记录。

修改文件：`src/modules/video/VideoPage.vue`、`src/services/video.ts`、`src-tauri/src/video_commands.rs`、`src-tauri/src/database/video.rs`（含回归测试）、`tests/video.test.ts`及`docs/{PRD,TECHNICAL-DESIGN,DATABASE,DEVELOPMENT-PLAN,USER-GUIDE}.md`。

## 视频P2：关键图片与主动插画（2026-11-02）

用户明确要求继续完成P2，基于05f14c6/v0.36.0实施本书静态图片选择/导入、受控裁切、来源/授权、摘要冻结、同源图片层和主动单张插画。全部复用现有Scene/视频任务/图片Provider/ai_tasks与完整备份；图片与物体二选一为主体，生成前确认、生成后显式采用，失败保稿。schema15不变，无新依赖；新增4个受限主窗口命令，图片CSP仅追加blob，正式安装和原数据保留。属于feat/minor兼容功能增强，不手动改版本、不提交或发布。

验收标准：本书合法静态图片可编辑保存；跨书、缺失、非法格式/裁切与未知字段拒绝且不丢稿；保存冻结内容摘要，源变化使试片失效；图片必须decode成功后才就绪；预览/整片同一时间画面一致；完整备份/恢复及重启保留图片与使用信息；插画1张/模型/上传范围确认，取消确认零请求，结果不自动采用；真实桌面图片可见且窄窗口无内部溢出；完整成片与SRT唯一归档。实际命令、截图、样片与真实模型/安装/长片边界见 [P2验收](VIDEO-IMAGES-VERIFICATION.md)。

## 视频渲染实时反馈（2026-11-02）

补齐字幕准备数量、字幕合成与原声合入的真实编码位置/倍速/阶段百分比，复用原渲染器、任务、停止入口和Progress。进度区域增加刻度环、流光轨道与四步状态；15秒无新快照提示等待，减少动态效果和同步失败时关闭装饰动效。

验收标准：字幕合成途中观察到多个真实中间位置；帧/字幕条数/视频秒数不混用；阶段100%不当成任务完成；试片保持原声时窗；确认、停止、失败和已有成果保留；1440/1120/800/390/360px无横向溢出。无SQL迁移、依赖或权限变化。属于兼容功能增强，发布影响为feat/minor，不手动升版本、不提交或发布。实际命令、截图与安装/长片验收边界见 [渲染反馈验收](VIDEO-RENDER-FEEDBACK-VERIFICATION.md)。

## 业务基础控件补齐（2026-11-02）

已统一本次盘点的 7 处原生确认、共享确认面板及发布结果表单，3 处原生折叠区、3 处业务卡片按钮和更新/视频两种直线进度。业务仍使用现有 shadcn-vue / Reka UI，不增加依赖或数据库迁移；发布影响为 style/fix（patch），不手动升版本。

验收要求：取消保留内容与选项，Tab/Shift+Tab 留在弹窗内，Escape 取消并返回原入口，运行时禁止退出/重复提交；发布记录失败保留备注；折叠不清空字段；未知进度不显示百分比；1440px 与360px窗口中的弹窗均可操作。命令、结果、截图及桌面验收边界见 `UI-COMPONENTS-VERIFICATION.md`。

## 文本模型推理强度（2026-11-02）

验收标准：文本模型可选择七档或服务默认；保存、切页回读、正常重启与备份恢复保留；未保存不能测试连接，保存失败保留选择；图片模型隐藏且保存清空；统一文本请求在配置时准确传参，默认不传；v14升级保留原配置、迁移冲突回滚、旧备份源不改写；不支持参数时显示错误并保留原内容。

沿用现有Provider/IPC/Select和统一300秒超时；新增v15兼容迁移，无新依赖或权限。属于feat/minor增强，不手动修改版本。

实际验证（2026-11-02）：

| 命令 | 结果 |
| --- | --- |
| `npm run lint` | 通过 |
| `npm run build` | TypeScript及生产构建通过；保留Vite的大包体积提示 |
| `npx vitest run` | 18个文件、286项通过 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked` | 161项通过、2项按原设置忽略（系统凭据实测、视频子进程专项） |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | 通过 |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets -- -D warnings` | 通过 |
| `npx --yes --package @playwright/cli playwright-cli --raw -s=reasoning-ui run-code --filename output/playwright/reasoning-check.cjs` | 隔离浏览器夹具通过；保存xhigh、刷新回读、连接按钮、键盘恢复默认、图片隐藏及1440/1120/800/360px无横向溢出 |
| `npx --yes --package @playwright/cli playwright-cli -s=reasoning-ui console error` | 浏览器错误0条 |
| `git diff --check` | 通过 |

Rust命令先加载 `. ./.tools/env.ps1`；本轮TEMP/TMP/SQLITE_TMPDIR设为仓库 `output/reasoning-test-temp`，仅影响测试进程。首次检查暴露两处旧断言（120秒提示、v14固定版本）及视频页模型夹具缺新字段，已同步；迁移断言保持核对实际最新版本，不跳过校验。浏览器键盘测试按锁定reka-ui 2.11.0的异步焦点完成状态等待，不增加产品延时或补丁。

本地验收标准全部通过。协议以本机HTTP契约服务验证，浏览器使用模拟IPC和独立localStorage；均未接触真实密钥或业务数据库。未验证第三方服务是否接受各档位或实际推理质量，未进行原生桌面/正式安装升级、真实模型计费调用或macOS/Linux实机验收。下一步在包含本改动的桌面构建中保存所需档位并测试真实服务；不支持时改回服务默认。

修改文件：
- 前端：`src/modules/settings/ModelsPage.vue`、`src/types/ai.ts`。
- 后端与迁移：`src-tauri/src/ai.rs`、`src-tauri/src/ai_commands.rs`、`src-tauri/src/database.rs`、`src-tauri/src/database/ai.rs`、`src-tauri/migrations/0015_reasoning_effort.sql`。
- 测试：`tests/ai.test.ts`、`tests/video.test.ts`、`src-tauri/src/ai/tests.rs`、`src-tauri/src/database/m2_tests.rs`、`src-tauri/src/database/video.rs`（仅当前schema断言）。
- 文档：`docs/PRD.md`、`docs/TECHNICAL-DESIGN.md`、`docs/DATABASE.md`、`docs/DEVELOPMENT-PLAN.md`、`docs/USER-GUIDE.md`。

浏览器检查脚本、结果及截图位于忽略的 `output/playwright/reasoning-setup.cjs`、`reasoning-check.cjs`、`reasoning-result.txt`、`reasoning-desktop.png`、`reasoning-narrow.png`，不作为产品代码或发布产物。

## P1 场景层次与画面内容扩展（2026-11-02）

用户确认后基于0e86f39/v0.32.1实施：6类轻场景、三景别和8项真实字幕缺口物体（共93），保留plain/reference、13构图/6动作和3主体上限。两项Scene字段贯通唯一目录、Rust校验、v4 Prompt、Node和原确认页；detail+focus使用一次受控轻推近，标题/字幕不动。旧85项几何、原声、旧作品和既有任务/试片流程保留，不新增表/依赖/权限或手动升版，属于feat/minor兼容增强。

验收包含新旧JSON/备份、非法字段拒绝、编辑保存/预览失效、全部物体动作确定性、各景别边界、同原声预览/整片一致性和隔离桌面流程。实际命令、样片和未覆盖边界见 [P1验收](VIDEO-SCENES-VERIFICATION.md)。原24候选中其余16项、P2图片/插画、真实模型主观质量及正式发行另行验收，不自动扩大范围。

## 视频中文路径崩溃修复（2026-11-02）

以用户确认的94808c9为基线。原469条字幕/72镜任务在Node24.11.0清理中文路径锁文件时崩溃，英文目录不能复现，最小`fs.rmSync`对照与Node官方61108修复一致。使用独立修复版Node，不修改引擎或中文业务路径；现有渲染入口提前拒绝已知缺陷版本，设置说明同步，机器PLAN JSON不再淹没失败原因。

验收：同一中文目录原计划在旧/新Node下形成失败/通过对照；不安全版本在读取任务和调用引擎前明确拒绝；一般诊断和非零退出继续保留；原计划/原声不变，完整渲染及适用检查另记实际结果，详见 [MG视频验收](MG-VIDEO-VERIFICATION.md)。无SQL迁移、业务依赖、权限或版本号变更，影响为fix/patch；不提交、不发布、不替换正式安装或全局Node。

完成：保留用户020a57c/v0.32.0更新，原469字幕/72镜在中文目录经真实原生宿主渲染933.459秒成片，解码/播放通过；Node20、前端283、Rust155及取消进程树专项1通过，lint/类型/构建/fmt/Clippy通过。正式用户设置未代改；已有安装版选择独立Node24.22.2并保存后可重试原任务，不需重新生成字幕/分镜。未完成的正式安装/归档/完整听审和跨平台边界保留。

## 视频画面内容与质感升级（2026-11-02，P0历史记录）

已形成 [画面扩展与质感升级方案](MG-VIDEO-VISUAL-UPGRADE.md)。推荐依次完善原声驱动分镜与同源试片、6类轻场景/景别及高频物体、少量有来源的关键图片；保留书卷手绘、单一渲染流程和人工确认，不靠堆图标或特效提升观感。

用户确认后已实现P0：显式字幕触发、单主体/叙事主次、可定位观看提醒及受控分镜编辑、30/60秒同源试片/缩略图。单一任务调度/运行代次负责停止与恢复，预览不登记成片，修改后失效，完整视频仍需确认。85物体几何与目录不变，无SQL迁移、新依赖或手动升版；主窗口增加两个受限视频命令和缩略图asset CSP，不开放任意路径。

验收要求包括同原声新旧对照、预览/整片绘制时钟一致、窗口尾段、原生取消/重启/编辑失效、素材事务及适用工程检查。P0当时的命令、证据和未验收边界见 [P0验收](VIDEO-DIRECTOR-VERIFICATION.md)。用户随后确认P1，当前进展见本文件首节；P2仍单独确认。P0当时保留046f054/v0.31.0，不提交、不发布、不替换正式安装。

## 同款手绘视频生成（2026-11-01）

移除原五类文字框绘制，复用参考读书成片85物体/13构图及字幕样式，以单一目录约束6种动作和原声字幕锚点。前后端确认页展示物体/动作/触发字幕，旧分镜明确保留字幕升级，已完成作品可创建独立新版。原声音轨、素材、历史和半自动发布边界不变；无依赖、权限、SQL迁移或手动升版，影响为feat/minor兼容增强。

验收包含85物体几何摘要、全部动作的有限数值/确定性、旧数据读取及明确升级、5处参考物体区域逐像素对照、83.667秒原声样片和真实Windows隔离IPC/转写/分镜契约/渲染/归档/播放。详细实际命令、结果、过程失败、响度提醒及正式安装/真实模型边界见 [MG视频验收](MG-VIDEO-VERIFICATION.md)。

## 视频首帧一致性失败修复（2026-11-01）

已用《我的前半生》原437条字幕/113镜复现首镜 `Layout check failed: not deterministic`。根因为引擎DOM/字体就绪早于首次绘制，首次截图可能为空白；两次目标时间的DOM实际一致，并非分镜丢失或随机生成。生成页面在同一个ready承诺内等待初始绘制，检查/编码共享，不改外部引擎，不关闭检查、不固定睡眠、不自动重试。

验收：原失败加载条件重复检查通过；全113镜保持严格布局/确定性检查；失败后复用已保存字幕/分镜确认重试；原输入不变；完成真实原声渲染与适用回归。实际命令、结果与未完成项记录在 [MG视频验收](MG-VIDEO-VERIFICATION.md)。无数据库迁移、依赖、权限或版本号变更，影响为fix/patch；源码修复不等于正在运行的旧安装版已更新。

## 视频制作进度与等待反馈（2026-11-01）

接通真实字幕位置/最近字幕预览、已保存分镜覆盖、已完成视频片段帧数及本次用时；统一五步流程、进度环和具体渲染阶段，窄窗口标签分行、支持减少动画。无法计量的字幕合成等阶段不虚构百分比或ETA，画面100%不冒充已归档。重试隔离旧运行与遗留临时片段，正式缓存仍可复用。

复验同时修复深层Windows字体链接超过路径限制，以及引擎初始化异常后Node进程未退出导致的持续等待。只改书引调用层，不修改独立引擎；无SQL迁移、依赖、权限或版本号变更，不提交或发布。进度属于兼容功能增强，发布影响为feat/minor，随附上述可靠性修复，不手工升版本。

验收标准：真实数据持续反馈；确认、停止、失败、重试和完成状态明确；原声/已保存成果保留；窄窗口可读；初始化错误不让用户无限等待。完整文件、实际命令和短片/长片/正式安装边界见 [MG 视频实施与验收](MG-VIDEO-VERIFICATION.md) 的实时进度续作章节。

## 保存书籍后误报历史音频错误修复（2026-11-01）

根因：书籍保存成功后进入内容创作页，该页读取本书最近任务及全局活动音频任务，直接把 `shownAudioJob.errorMessage` 交给即时浮层。历史错误因此在每次进入时重新弹出，也可能属于另一本书；保存书籍本身没有启动音频生成。

修复：初次读取只恢复任务卡；后续 `applyAudioView` 按任务 ID、错误内容/代码和运行编号识别新错误，复用现有操作反馈并注明所属书籍。不因相同轮询、切页或主动停止重复弹红色错误；新失败与继续后的再次失败仍通知，旧错误及恢复入口保留在任务卡。没有增加通知缓存、重复任务状态或外部网站重试。

修改文件：`src/modules/writing/WritingPage.vue`、`tests/ai.test.ts` 和本文件。无数据库、权限、依赖或版本号变更，归类 fix/patch。只读核对正式数据库，未改写书籍或音频任务，未调用真实生成服务。

验证：修复前 `npx vitest run tests/ai.test.ts -t "does not replay another book audio error after saving book details"` 在真实资料保存及创作页面组件间复现原始 `[audio_prompt_missing]` 浮层；扩展后的8项回归先失败，修复后通过。`npx vitest run` 全部262项前端测试通过；`npm run lint`、`npm run build`（含 TypeScript）及 `git diff --check` 通过。构建仍有既有500 kB分包提示。未运行 Rust/安装包构建、替换正式安装或验收真实外部音频网站；安装版须在后续打包更新后获得此修复。

## 应用内 MG 视频首版（2026-11-01）

已实现字幕转写/校对/导出、受约束分镜及确认、独立引擎原声渲染、应用内播放和明确选用发布；SQLite增加v14任务，补齐取消、恢复、缓存和素材事务。Windows隔离短片真实IPC/ASR/渲染/播放通过，分镜模型使用本地契约服务；整本书长片、真实文本模型质量、完整试听和正式安装/跨平台尚待验收。不捆绑AGPL引擎、不新增配音或自动最终发布，版本影响为feat但不手工升版本。命令与完整边界见 [MG 视频实施与验收](MG-VIDEO-VERIFICATION.md)。

## 音频固定口播与重生成指引（2026-11-01）

以提示词优化为主：去掉开场结尾的节目分钟数与动态系列名，区分不朗读的制作参数与逐字朗读的固定段，全量/按项模板同步版本；页面只同步系列名称。归档后复用现有开始生成入口，新任务保留旧成品，不增加任务状态、音频处理依赖或数据库迁移。属于 fix/patch，不手动改版本。

验收：含18分钟在内的多种时长不改变固定口播；按项只更新选中内容；成功归档后可用新指导生成并保留两份素材与文件。真实音频遵从性仍须试听，不宣称提示词提供逐字保证。文件清单、执行命令与证据边界见 `AUDIO-PROMPT-REGENERATION.md`。

实际验证：前端250项、Rust139项通过（1项既有忽略），lint、TypeScript/Vite构建及Rust fmt/Clippy通过。未执行真实生成/试听、安装包构建或替换正式安装；旧历史与已有音频不改写。

## 各尺寸封面独立编辑（2026-11-01）

实现三尺寸独立文字/排版/背景/裁切/描述、主动共用背景或整套配置、单张和全部导出、16:9 专属提示词全量生成及按项调整。复用当前 Provider、编辑草稿、版本记录及备份，无 SQL 迁移和新增依赖。

验收标准：切换尺寸不串改；生成/导出只作用于目标尺寸；共用前确认且之后可独立调整；旧文案可单独补 16:9；失败保留原稿；多尺寸草稿和历史可恢复。发布影响为 `feat` / minor，不手动改版本、不提交或发布。实际验证和未验证边界见 `COVER-INDEPENDENT-VERIFICATION.md`。

## 合集误报与重复账号确认修复（2026-11-01）

修复官方页已选中目标合集、下拉框收起时仍报“合集重名、未加载或已失效”的问题。确认与恢复先回读已选值；需要变更时才自动展开列表、有界等待唯一选项并回读。账号只在当前官方窗口确认一次，导航期间账号控件挂载和头像 URL 参数变化不再误判换号；退出登录、实际账号变化、关闭窗口仍重新核对。已有视频确认文案不再重复要求核对账号。最终发布、验证码和真实内容冲突保留人工决定。

相关前端50项、全量前端243项、Rust133项通过（1项既有忽略），lint、fmt/Clippy及隔离桌面构建通过。隔离原生7项场景覆盖一次账号确认后自动上传/填写、延迟合集选项、已选合集收起后确认、恢复不重传、更改合集及账号变化。完整 `npm test` 仍有既有队列测试1项失败。无数据库迁移、依赖、权限或版本号变更，属于 fix/patch；未操作真实抖音账号、替换正式安装或自动点击最终发布。完整文件清单、命令与边界见 `DOUYIN-FLOW-VERIFICATION.md`。

## 音频页面间距与路径展示修复（2026-11-01）

保留现有品牌与控件，移除挤压说明文字的通用负边距，补齐音频来源/格式区域内留白、说明间距、长名称省略与窄窗口单列。全站路径统一在展示层转换 Windows 盘符/UNC扩展前缀，素材搜索同步使用显示路径；原始保存路径、数据库和文件操作保持不变。无迁移、依赖或手动升版，归类 fix/patch。

前端241项、lint、TypeScript/Vite构建及隔离浏览器1440/1024/720/375px验证通过。原始参数保留、长中文路径、无来源/文本来源、格式选择与音频ID均有回归；保留既有构建大小提示。未替换正式安装或触发真实网站任务，完整命令、文件范围、截图与验收边界见 `WORKSPACE-UI-UPGRADE.md`。

## 抖音启动误报与发布入口等待修复（2026-11-01）

复现新窗口尚处 `about:blank` 就报“当前不是抖音创作者中心”的启动时序问题。页面检查改为有界、可停止的加载等待；账号确认后复用固定官方「发布视频」地址自动导航，等待上传控件挂载后才发送，保留旧草稿、弹窗和最终人工发布边界。未增加菜单点击实现、依赖、权限或数据库迁移，归类 fix/patch，不手动升版本。

已在隔离 Windows WebView2、真实 IPC 和本地页面夹具中验证初始等待、加载超时、停止、自动导航、延迟挂载后的原生文件选择及草稿保留。前端 225 项、Rust 130 项通过（1 项忽略），lint、fmt/Clippy、类型检查和独立桌面构建通过；完整 npm test 仍有 1 项既有队列测试失败。不代表真实抖音账号或最终发布验收，正式安装版未替换。完整命令与证据见 `DOUYIN-FLOW-VERIFICATION.md` 最新章节。

## 真实音频配置、下载与归档验收（2026-10-31）

继续用户指定的《人生由我》原笔记本。真实网站停在 `audio_prompt_missing`，本轮重新读取原弹窗，按实际中文名称、原生单选及语言组合框补齐语义匹配；统一关联标签解析并限定配置弹窗，不增加兜底输入框、重试或任务状态。新回归修改前复现相同错误，修改后验证两种格式、完整指导、语言已选不重开、短时长及配置不提交生成。无数据库迁移、依赖、权限或手动升版，归类 fix/patch。

本轮沿用原任务实际完成：1659 字完整指导与冻结摘要一致，深入探究/简体中文，生成一次，成品 16:39；原生下载 32,174,918 字节 M4A、唯一音频素材登记和最终 release 正常重启回读通过。下载阶段另实测修复 `window.open` 地址捕获及同一 NavigationId 服务器重定向；未重建书籍/Notebook、未重新上传或生成。恢复授权快照后保有 10 本书、5 个音频任务、原有文案/封面/草稿/模型/发布记录，SQLite 仍 v13。页面/打包 45、前端223、Rust128通过（1项忽略）、Clippy/lint/类型/构建通过；最终 NSIS 已生成，尚未安装或发布。旧数据库视图差异原因、首次登录、其他格式/系统及安装升级仍未验收，完整恢复与真实服务证据见 `IN-APP-AUDIO-VERIFICATION.md` 最新章节。

## 上传器延迟初始化修复（2026-10-31）

当前 NotebookLM 上传器在首次聚焦或鼠标经过时创建文件输入框；直接脚本点击会稳定触发 `file_input_missing`。仅在文件上传步骤先聚焦唯一上传按钮，保留弹窗范围、冻结文件、原生选择和不重复提交规则，不改数据库或引入依赖。回归按网站实际懒初始化行为创建输入框，不再仅依赖预置输入框夹具。旧核对阶段任务仍不自动重传，处理步骤见使用手册；验证命令和真实服务边界见 `IN-APP-AUDIO-VERIFICATION.md` 最新章节。归类 fix/patch，不手工升版本。

## 添加来源与上传文件流程修复（2026-10-30）

按用户确认的实际流程，先核对本任务笔记本地址，再点击来源面板顶部「添加来源」及弹窗中的「上传文件」，只向该弹窗的唯一输入框交付冻结文件。不再自动改名或要求标题输入框；保留旧任务 URL、创建意图、人工确认及不重复提交规则，新建后等待跳转不重复点击。页面异常结构化回传错误代码和完整消息，不再依赖 CDP 异常描述的第一行。复用同一来源面板判定恢复空白旧任务，不引入新依赖、权限或数据库迁移，schema 保持 v13，属于 fix/patch。

验收：页面与打包 41 项、前端 223 项、Rust 127 项通过（1 项既有凭据测试忽略）；lint、TypeScript、fmt/Clippy、前端及独立桌面构建通过。隔离原生 WebView2/IPC/SQLite 的文件和资料文本任务均完成模拟生成、blob 下载及唯一素材登记；文件入口点击顺序和次数、原书字节、标题保留及无关输入框均有断言。正式安装、真实 Google EPUB 上传/生成/下载及旧任务升级恢复尚未执行。完整命令、失败回归、文件范围和证据边界见 `IN-APP-AUDIO-VERIFICATION.md` 最新章节；下一步打包正式版本后恢复当前原任务。

## 自动化流程复核（2026-10-30）

音频隐藏入口/来源处理中误判、抖音含图标状态文字漏读已复现并最小修复。最终前端 223、音频页面 31、Rust 125 项通过（1 项凭据测试忽略），lint、构建、fmt/Clippy 通过。整体 npm test 的既有队列缺报告重跑测试仍失败。真实未登录页面识别及安装版旧任务阻塞已观察；真实账号生成/上传闭环尚未完成，安装版未替换。完整范围见 `AUTOMATION-AUDIT-2026-10-30.md`。

## 笔记本身份登记与恢复修复（2026-10-29）

修复命名前清除新建标记、命名失败时未保存 URL 导致恢复锁死的问题。新任务先保存确认创建的笔记本地址，再命名回读；命名失败或重启沿用原页。旧无地址任务经页面内确认当前 URL、宿主核对空白笔记本后恢复原行，不替换已记录或已提交的任务。停止跟进复用 `ConfirmAction`，取消/Escape 不修改任务，不新增原生消息框权限。

修改音频页面、IPC 类型/服务、Rust 宿主/命令/数据库状态流与对应回归，同步技术设计、数据库、使用手册和验收报告；SQLite 仍为 v13，无迁移、依赖或手动升版，属于 fix/patch。最终前端219项、页面/打包33项、Rust音频24项通过，lint、TypeScript/构建、fmt/Clippy与独立Windows桌面构建通过。隔离WebView2/IPC/SQLite验证命名恢复、新建仅一次、旧任务确认关联、取消保留、非空及变化URL拒绝，未提交生成。完整命令、失败过程及边界见 `IN-APP-AUDIO-VERIFICATION.md` 的身份恢复修复记录。

未替换正式安装或修改用户数据；真实 Google 上传/生成/下载和正式升级仍待验收。下一步在包含修复的正式构建中恢复当前旧任务。

## 停止任务后继续跟进（2026-10-29）

当前源码已有 failed/cancelled 恢复原任务的能力，本轮将「继续跟进」移至任务操作区首位，停止提示和确认框明确说明可恢复；恢复期间显示「正在继续…」及 aria-busy，并复用已有操作锁避免重复点击。失败后保留恢复入口，可再次继续。沿用原任务、笔记本和冻结输入，提交不明时先核对，不重复生成；已有其他活动任务时不抢占。无新增依赖、命令或数据库迁移，SQLite 仍为 v13，属于交互完善 fix/patch，不手动升版。

实际验证：`npx vitest run tests/ai.test.ts` 61 项通过，新增停止再继续、恢复失败重试及操作中重复点击回归；`cargo test --manifest-path src-tauri/Cargo.toml --locked audio:: -- --nocapture` 22 项通过，新增停止后关闭并重开数据库、恢复原下载、拒绝重新提交和旧运行写入、最终唯一音频登记回归。`npm run lint`、`npm run build`（含 TypeScript）通过，保留约515 kB分块提示；Vite 清理 dist 和 Rust 格式化/构建锁首次被沙箱权限阻止，经工具审批后重跑通过。Rust 先加载 `. ./.tools/env.ps1`，TEMP/TMP/SQLITE_TMPDIR 指向 `output/audio-resume-test-temp`。

验收对应：停止后恢复入口、同一任务继续、恢复中的反馈与防重复点击、失败重试、重启后继续下载均有本地回归。未替换正式安装或操作用户数据库；本轮未执行真实 Google 账号生成、桌面人工验收或安装包构建。下一步使用含本轮源码的新桌面构建，核对截图中已停止任务的继续入口。

## 音频网站刷新与故障恢复（2026-10-29）

补齐刷新网站、重新打开、失败及停止后的原任务恢复。明确识别 404、访问限制、服务错误、网络错误与人工验证；暂停状态不再展示持续等待。刷新先暂停并使旧运行失效，继续时不重复未知上传或生成，禁止抢占其他活动任务。复用 v13、现有窗口与登录会话，无新依赖。实际验证及边界见 `IN-APP-AUDIO-VERIFICATION.md` 的网站恢复补充。

## macOS CI DMG 打包修复（2026-10-29）

日志显示 Intel release 编译、APP 生成与 ad-hoc 签名完成，Tauri DMG 脚本卸载临时磁盘连续三次返回 `Resource busy` 后失败；具体占用进程无法从现有日志确定。已核对锁定的 CLI 2.12.4 源码，`CI=true` 只跳过 Finder 脚本，未移除该挂载/卸载流程。macOS 两种架构的 CI 统一改为 Tauri `--bundles app`，再由现有发布资源脚本暂存 APP 与 Applications 入口，直接创建 UDZO DMG 并校验。保留 Tauri 更新归档、签名及原有分发文件契约；不增加依赖、重试或强制卸载逻辑。

修改 `.github/workflows/release.yml`、`scripts/release-assets.mjs`、`tests/release-assets.test.mjs`，同步 `RELEASE.md` 和本计划；数据库仍为 v13，无迁移或手动升版，按 fix/patch 处理。验收覆盖显式挂载调用移除、创建/校验失败中止、临时目录清理和四平台实际构建参数；macOS 原生用例在两个 macOS runner 上创建并校验真实 DMG 后继续测试归档。

本机 `node --test tests/release-assets.test.mjs`：15 项通过、1 项 macOS 原生用例按平台跳过；完整 `npm run test:release`：60 项通过、1 项跳过、0 失败，耗时约 214 秒；前两次因同步 Git 集成测试长时间没有输出被中断，最终在限制外运行同一命令完成。`npm run lint`、`.tools/actionlint/actionlint.exe .github/workflows/release.yml`、`git diff --check` 通过。通过 Git Bash 对工作流实际脚本进行命令替身验证，macOS Intel/ARM64 使用 APP 目标，Windows/Linux 保留原构建参数。真实 macOS 打包与安装未在 Windows 本机执行，需修复合入 main 后的新 CI 运行确认；重跑旧运行不会使用本次修改。未提交、推送或发布，已有暂存工作保留。

## 查看音频网站黑屏修复（2026-10-29）

已定位到官方入口重定向域名遗漏及同任务空白窗口未重新导航：本机请求从 notebook.google.com 经 notebooklm.google 到 notebook.google/?location=unsupported；旧白名单取消跳转，查看网站只显示原空白窗口。补齐宿主/页面/数据库的精确官方域名，恢复空白窗口，并在首次导航前有限等待；地区限制明确提示、登录页保留、提交未知不重发。范围为 fix/patch，无迁移、依赖或版本字段变化。

页面与打包 27 项、Rust 音频 17 项、lint、TypeScript/前端构建、Rust fmt/Clippy 及独立标识桌面构建通过。原生 WebView2 回归覆盖初始空白恢复、两个官方跳转、地区提示和未提交、登录页保留。命令及边界见 IN-APP-AUDIO-VERIFICATION.md；未更新正式安装，真实 Google 账号生成仍未验收。

## 应用内自动音频替换（2026-10-29）

用户在设计后明确要求实施。已以专用 Tauri 窗口、Rust 单一调度、语义脚本、不可变来源快照及精确原生下载替换扩展/helper，复用 v13 audio_jobs 和素材事务。删除安装/连接门槛及旧打包依赖，保留旧任务、手动音频、书籍来源绑定及用户其他工作。

设计见 NOTEBOOKLM-IN-APP-AUDIO-DESIGN.md，实际命令和桌面证据见 IN-APP-AUDIO-VERIFICATION.md。真实 Google 登录/控件/生成/下载及正式安装升级仍待验收，模拟不能证明服务可用。不绕过限制，不提交、发布或替换正式运行应用。无数据库迁移或版本手动递增。

本地验证：前端 204、Rust 117（1 忽略）、页面/打包/发布资源聚焦 38（1 平台跳过）通过，lint、类型检查、生产构建、fmt/Clippy 通过；最终隔离 NSIS 包生成并核对清理 hook，未打包扩展/helper。真实 Tauri/WebView2/SQLite 中，资料文本及中文文件来源均完成 blob 和真实本机 HTTPS 下载、字节一致校验和唯一音频素材登记；每任务只生成一次。上述均为明确标注的本地模拟证据。


## 新增书籍绑定本地文件（2026-10-28）

已实现新增/编辑书籍选择本地 PDF、EPUB、DOCX、TXT、Markdown，展示文件名/路径/待保存或缺失状态，保存前可取消；保存失败保留编辑与选择。复用既有 document 素材和音频来源默认选择，书籍及新增文档同事务保存；权限拒绝、文件缺失、格式错误、重复关联均回滚，新建失败清理本次空目录。只记录路径，源文件不复制、不改写、不删除，已有绑定通过原素材管理重新定位/移除。

修改文件：`src/modules/projects/ProjectPage.vue`、`src/services/projects.ts`、`src-tauri/src/commands.rs`、`src-tauri/src/database/projects.rs`；回归位于 `tests/projects.test.ts`、`tests/ai.test.ts`、`src-tauri/src/database/m1_tests.rs`、`src-tauri/src/database/audio_tests.rs`；同步 `PRD.md`、`TECHNICAL-DESIGN.md`、`DATABASE.md`、`USER-GUIDE.md` 和本计划。数据库保持 v13，无迁移或新增依赖，属于兼容 feat/minor，不手动升版本。

实际验证：

- `npx vitest run tests/projects.test.ts tests/ai.test.ts`：78 项通过。
- `npm run test:rust`：116 项通过，1 项原生凭据测试按既有配置忽略；覆盖文件绑定持久化、重定位、重复/缺失/类型/访问拒绝回滚和音频任务读取绑定文件。
- `npm run check:rust`：fmt 与 Clippy 通过。Rust 先加载 `. ./.tools/env.ps1`，TEMP/TMP/SQLITE_TMPDIR 指向仓库 `output/source-binding-test-temp`。
- `npm run lint`、`npm run build`（含 TypeScript）：通过，构建保留大于500 kB的分块提示。受限环境最初拒绝源码格式化、Cargo 锁及 Vite 清理 dist，经授权在受限环境外重跑成功。
- `npm test`：Vitest 204 项通过；发布/扩展测试65项通过、1项平台相关跳过；队列测试18项通过、1项失败，因此整条命令退出1。失败为未修改的 `tests/codex-queue.test.mjs:118`「缺少前序 Markdown 报告应重跑后续任务」，实际没有重跑，本轮未扩展修改队列脚本。
- 独立 Playwright CLI + 模拟 Tauri IPC：选择本地文件、保存参数和跳转验证通过；1440/1024/720px 均无横向溢出，长中文路径可换行。命令为 `node D:/DevCache/npm-cache/_npx/31e32ef8478fbf80/node_modules/@playwright/cli/playwright-cli.js -s=source-binding run-code --filename output/source-binding-visual.js`。脚本与截图位于本地 `output/source-binding-visual.js`、`output/source-binding-desktop.png`、`output/source-binding-narrow.png`，不作为原生文件选择器或真实 Google 验收。
- `git diff --check`：通过。

验收对应：新书绑定、旧书补充、取消选择、重复选择、失败保留、事务回滚、源文件保留、重启读取、缺失重定位及音频来源复用均有本地验证。未执行安装包构建、真实桌面文件选择器及 Google 生成下载；未提交、推送或发布。下一步用新版桌面添加一本书并绑定本地原书，进入音频制作核对默认来源后执行真实生成。

## 工作台样式与整体导航升级（2026-10-28）

- [x] 统一墨绿品牌、浅灰白工作区、面板、页签与可见焦点；不增加依赖。
- [x] 书架最近创作、网格/列表切换与 URL 搜索/筛选/排序状态恢复。
- [x] 主导航发布任务入口与书籍内四步流程，精确当前步骤、页面标题和切页焦点。
- [x] 发布空态操作、任务搜索清空与 Escape；刷新失败保留已有数据。
- [x] 隔离浏览器检查 11 个路由 × 4 种宽度，补充真实点击与减少动态效果检查。
- [ ] Windows Tauri/WebView2 实机验收及安装包验收。

本轮属于样式调整与兼容交互增强，新增网格/列表及状态恢复能力按 `feat` 处理；不手工修改版本号。实际命令、结果及边界见 `WORKSPACE-UI-UPGRADE.md`。

## 全站浮层提示统一（2026-10-28）

新增固定版本 `vue-sonner@2.0.9`，应用根部只挂载一个 `AppNotifications`。现有 `OperationFeedback` 统一转发操作错误、成功、信息和警告，移除旧成功反馈 CSS；书架、书籍、素材、创作、封面、模型、备份、发布、更新和技术验证页接入。自动暂存、封面预览和任务失败可即时弹出，持久业务状态与确认对话框保留。复制操作先清理上次反馈，保证重复操作仍有提示。无数据库或权限变化，按 fix/patch 处理，保留前序音频参数修复。

验收：`npm run lint` 通过；`npx vitest run --reporter=json --outputFile=output/notification-tests.json` 181 项通过，业务断言改为检查真实 Sonner 节点，另补充提示生命周期回归；`npm run build`（含 TypeScript）通过。独立 Playwright 浏览器中，1280×720 页面滚动 772px 后错误提示仍位于 y=24–136；390×844 窗口提示位于 x=16–374，无水平溢出。截图在 `output/notifications-desktop.png`、`output/notifications-mobile.png`。浏览器验收使用真实组件和隔离展示页，未替代安装后的 Windows WebView2 验收；展示页唯一控制台错误为未提供 favicon 的 404。

`npm run desktop:build -- --bundles nsis --config output/audio-source-fix/local-bundle.json` 成功；复制校验通过，交付包为 `output/notifications-fix/书引AI-统一提示与音频修复-0.20.0-setup.exe`。本地包仅用于手动安装，未发布或替换正在运行的应用；包含本轮提示统一及前序音频修复，无数据库迁移。`git diff --check` 通过。

## 音频文件来源 IPC 参数修复（2026-10-28）

真实页面通过 `start_audio_job` 发送 `source: { kind: "asset", assetId }`，Rust `AudioSourceInput` 的枚举重命名原先只作用于变体名，内部字段仍要求 `asset_id`，导致任务创建前报 missing field。补充 `rename_all_fields = "camelCase"`，不增加双命名兼容或默认素材 ID；数据库冻结快照仍使用既有 `FrozenSource` 格式，无迁移、权限或依赖变更，属于 fix/patch。

新增真实 JSON 契约回归，修复前复现相同的 `missing field asset_id`；原有文件分块集成测试改从前端 JSON 创建任务，覆盖解析、素材关联和分块读取。`cargo test --manifest-path src-tauri/Cargo.toml --locked database::audio::audio_tests` 10 项通过，`npm run check:rust` 通过。Rust 命令先加载 `. ./.tools/env.ps1`，TEMP/TMP/SQLITE_TMPDIR 指向仓库 `output/native-diagnostic`。不以本地回归替代真实 Google 生成下载验收；已安装的旧二进制需要更新才能生效。

本地交付：`npm run desktop:build -- --bundles nsis --config output/audio-source-fix/local-bundle.json` 成功，包含 TypeScript/Vite 生产构建、release helper、release 桌面程序及 Windows x64 NSIS 包；临时配置仅关闭自动更新签名产物，本包用于手动安装，不修改正式发布配置或版本号。安装包复制到 `output/audio-source-fix/书引AI-音频修复-0.20.0-setup.exe`，与原产物 SHA-256 一致，并保存 `.sha256` 文件。包同时包含上一轮桥接路径修复。尚未运行安装器、替换用户运行中的应用或验证真实 Google 完整流程。`git diff --check` 通过。

## Chrome 扩展连接诊断修复（2026-10-28）

实际连接根因：宿主清单把 `canonicalize` 的 `\\?\D:\书引 AI\browser-bridge.exe` 交给 Chrome，经过命令解释器启动失败；改为普通盘符形式后，同一轮询成功。当前用户清单已备份并修正，用户确认扩展显示“已连接”。源码将既有素材路径转换提取为共享 `paths::shell_path`，注册清单复用它，不新增依赖或重复转换逻辑。当前安装的旧版桌面程序未替换，重启旧版仍会重写旧路径；永久生效需要包含源码修复的新构建。音频生成和下载全流程不在本次连接验收范围。

扩展连接正式版失败后再尝试开发版，原实现仅保留最后一次错误，导致正式版真实原因被“开发版宿主不存在”覆盖。现按正式版/开发版分别保留错误，弹窗分行显示；检查期间显示“检查中…”并禁用按钮，完成后恢复。无依赖、权限或数据库变更，按 fix/patch 处理。

`node --test tests/audio-extension.test.mjs`：27 项通过，新增回归覆盖正式版错误保留、开发版成功连接、弹窗检查中及失败反馈；修复前错误保留与检查中反馈两项失败。`npm run lint` 通过。`cargo test --manifest-path src-tauri/Cargo.toml --locked shell_`：路径转换及宿主清单 2 项通过；清单回归修复前实际失败。Rust 先加载 `. ./.tools/env.ps1`，TEMP/TMP 指向 `output/native-diagnostic`。

## 自动音频评审修复（2026-10-28）

修复 `HEAD~5..HEAD` 评审中的三项问题：Windows 桥接协议及专属数据库接口按平台编译（数据库测试跨平台保留）；Native Messaging 注册清理移到 NSIS 卸载完成钩子；上传首尾校验来源且在扩展侧校验完整传输摘要，删除“已校验”缓存。无数据库迁移或新增依赖，属于 fix/patch，不手动升版本、不提交或发布。回归覆盖上传期间同大小修改、改写后恢复、指纹漂移及失败提示。实际检查和打包证据、macOS/Linux CI 与真实 Google P0 的待验收边界见 `AUDIO-AUTOMATION-VERIFICATION.md` 的评审修复记录。

## Gemini Notebook 自动音频（2026-10-28，功能已完成，真实账号待验收）

按 [实施计划](NOTEBOOKLM-AUDIO-PLAN.md) 完成 Windows + Chrome + MV3 扩展 + Native Messaging helper + Rust/SQLite `audio_jobs` 闭环。数据库升至 v13；前端提供来源/格式/任务控制，Rust 冻结输入并唯一写状态，扩展执行受限页面动作，下载经精确 ID、容器与 SHA-256 校验后登记为 `assets.audio`。旧“只打开网站”命令已移除，手动导入、历史提示词、发布任务选择及最终抖音人工发布保持不变。

本地验证覆盖前端 178 项、Rust 109 项、扩展/打包/发布脚本、lint、类型检查和生产构建；Windows helper 与 NSIS debug 安装包已实际生成，安装包包含 helper 和扩展目录。真实 Google 登录、上传、长任务生成、下载目标、正式扩展上架、签名安装升级/卸载未在当前环境执行，不能写成已通过。Windows ZIP/NSIS 安装包现在携带 `browser-bridge.exe` 与 `browser-extension/`；macOS/Linux 只保留手动音频流程。详见 `AUDIO-AUTOMATION-VERIFICATION.md`。自动能力属于兼容 `feat` / minor，不手动升版本、不提交或发布。

## 封面文件命名（2026-10-27）

- 新导出使用 `书名-比例-尺寸-v版本.png`，例如 `围城-3比4-1080x1440-v3.png`。版本对应封面历史；已有同名文件或目录则递增版本，原文件保留。AI 背景使用真实比例/尺寸及「背景」标记，同名背景独立递增版本。旧随机文件名与数据库引用不迁移，schema v12 不变，无新增依赖；本次按 fix/patch 处理，不手动升版本、不提交或发布。
- 修改文件：`src-tauri/src/database/{covers,image_generation,projects,m3_tests}.rs`、`src-tauri/src/ai/{images,tests}.rs`、`docs/{PRD,TECHNICAL-DESIGN,USER-GUIDE,DEVELOPMENT-PLAN}.md`。书名清理复用项目目录规则，比例约分并以「比」代替 Windows 禁用冒号；图片格式和尺寸直接取已有完整解码结果。
- 验收覆盖三种封面比例、版本与历史一致、同名文件/目录跳过、中文/非法/保留名/过长书名、改书名后导出、旧随机路径重启可读、PNG/JPEG/WebP 背景真实尺寸、备份恢复后跳过磁盘占用版本，以及既有写入/事务失败清理与发布/完整备份流程。
- 实际验证：修复前 `cargo test --manifest-path src-tauri/Cargo.toml --locked database::m3_tests:: -- --nocapture` 为9通过、4项命名回归失败；修复后同范围13通过，后续补充背景实际尺寸及备份恢复回归纳入全量。`npm run test:rust` 为100通过、1项既有原生凭据测试忽略；`npm run check:rust`（fmt/Clippy）、`npm run lint`、`npm run build`（含 TypeScript）及 `git diff --check` 通过；`npx vitest run tests/covers.test.ts tests/coverRenderer.test.ts tests/publishing.test.ts` 为68通过。Rust 先加载 `. ./.tools/env.ps1`，进程 `TEMP`、`TMP`、`SQLITE_TMPDIR` 指向仓库 `output/cover-naming-test-temp`。Cargo 构建锁与 Vite 清理 dist 首次遭受限环境拒绝，在确认仓库输出路径后于受限环境外重跑通过。
- 未构建安装包或执行桌面人工验收。下一步在新版 Windows 桌面中导出三种比例并再次导出，核对文件名、历史版本和旧封面读取。

## 音频目标时长增量（2026-10-27）

已实现AI首次推荐/再次推荐、生成前自定义分钟与快捷选项、按书籍保存、离页和正常关闭前等待保存，以及全量/按项的时长贯通。模板统一调整总时长、预计字数、开场、五部分安排、结尾与系列文案；新封面和发布示例不预设十分钟。v12仅增加来源标记，复用原目标秒数，旧历史不改写。

验收对应：推荐可覆盖/失败保留、保存重启与备份恢复、任务快照不随在途修改漂移、各段时长合计准确、音频调整不改其他字段、取消迟到响应不写入、v11升级保留与冲突回滚、四种屏宽无溢出。实际命令、结果、既有队列测试失败和未执行的真实模型/桌面安装验证见 `AUDIO-DURATION-VERIFICATION.md`。本轮属于feat/minor，不手动修改版本、不提交或发布。

## 封面排版推荐增量（2026-10-27）

范围：全量与按项 AI 字号/配色推荐、独立副标题色、五套可视化样式、三比例导出和旧数据兼容。使用既有 JSON、Provider、候选采用、个人样式及渲染链路，无新增依赖或数据库迁移。验收覆盖新封面自动带入、已有封面主动应用、按项推荐不改其他文字、副标题独立调色及历史/草稿/备份兼容。验证记录见 `COVER-STYLES-VERIFICATION.md`。

## 当前实施：抖音智能辅助流程（2026-10-27）

已实施单一辅助流程：一个开始操作串联保存/检查、包准备、宿主登录等待、已知页面定位、视频上传、文字、双封面及合集；Rust 持有会话与运行 ID，继续仅接受当前运行的限定决定，停止等待在途操作结束再解锁。已核对的字段按网页身份回读，无法关联的旧草稿不清空、不重传；最终发布及实际结果继续人工确认。成功记录后可开始用户明确选择的下一篇。

schema v11 新增独立的任务合集意图，账号默认复用 app_settings；不重写旧任务包，不持久化浏览器进度。当前留存官方 DOM 仅提供昵称和头像，适配器明确返回空稳定账号 ID，因此不会凭昵称启用跨重启默认合集；相关数据库能力已实现。处理完成信号或合集控件无法回读时，明确等待人工确认。用户选择本轮先完成代码与本地验证，真实账号验收稍后进行。具体证据、边界与下一步见 [DOUYIN-FLOW-VERIFICATION.md](DOUYIN-FLOW-VERIFICATION.md)。本轮为 feat / minor，不手动升版本、不提交、不发布。

## 当前实施：音频制作与成片回填（2026-10-26）

- 保留“外部生成音频 → 剪映字幕/背景/导出 → 应用回收最终成片”的人工制作边界。
- 升级“音频提示词”为“音频制作”，支持固定外部入口和音频成品导入。
- v10 增加音频素材与发布任务音频关联；发布任务明确最终成片，检查 MP4/MOV 正时长、可读取画面和音轨，不限制横竖屏比例。
- 不自动操作外部音频网站，不读取 Cookie，不接管剪映，不上传或复制音频到抖音。

## 上一阶段：应用内抖音辅助发布（2026-10-26）

用户确认目标为「自动上传和填写，我检查后点一次发布」。M6 Windows 独立官方窗口、扫码、就绪任务上传/文案/双封面、账号核对、停止接管及人工记录的核心验收已完成；M7 跨书任务列表、精确任务入口和作品链接回溯已实现并完成独立安装版验证。证据见 [M6-VERIFICATION.md](M6-VERIFICATION.md)、[M7-VERIFICATION.md](M7-VERIFICATION.md)，方案与依据见 [DOUYIN-PUBLISHING-PLAN.md](DOUYIN-PUBLISHING-PLAN.md)。

开发顺序：M6.1 Windows 隔离技术验证（扫码、文字、视频和双封面）→ M6.2 接现有任务的单任务闭环 → M6.3 异常恢复与安装验收 → M7 待发布列表和作品归档。采用应用管理的独立发布窗口，最终发布由用户在官方页面点击。首版复用 schema v9、现有任务包与人工结果记录，不并行建设官方 API 发布路线。

已在0.14.3隔离安装版从空白页完整连续完成视频上传、等待处理、中文文案、双封面和回读，修复预览页签提示图标与分阶段挂载两项真实差异。0.14.4继续验证M7跨书任务、已保存来源、较早任务入口、模拟人工记录、下一篇和链接复制。131项前端、76项Rust通过（1项既有原生凭据忽略），lint、TypeScript/构建、fmt/Clippy和独立NSIS构建/升级通过；此前发布脚本30通过、1项macOS跳过。安装使用D盘独立目录及identifier，正式库、源视频和用户开发版人工草稿保留。原任务未发布；M7模拟记录只在隔离库中。发布影响为feat（minor），正式版本仍由流水线处理，不提交或发布。

## 书名目录命名（2026-10-25）

- 新建书籍使用 `projects/{书名}`，新发布包使用 `publish/{书名}-发布任务包`；完整恢复的项目与任务包使用相同命名规则。同名追加 ` (2)`、` (3)`，不复用或覆盖已有文件/目录。
- 保留中文，处理文件名禁用字符、Windows 设备保留名、尾部空格/句点与过长名称；数据库内完整书名和项目 ID 不变。已有目录及编辑书名时不自动搬迁，素材引用保留。
- 验收覆盖中文书名、同名文件/目录、删除后残留目录、非法/过长名称、重启、旧随机目录素材可读、任务包重复生成/失败清理，以及完整恢复后的重名书籍与任务包就绪。
- 本次为 `fix`（patch），无依赖、权限或数据库结构变更，schema v9 不变；不手动升版本、不提交或发布。
- 修改文件：`src-tauri/src/database/{projects,publishing,full_backup}.rs`、`src-tauri/src/database/{m1_tests,m4_tests,workflow_tests}.rs`、`docs/{TECHNICAL-DESIGN,USER-GUIDE,DEVELOPMENT-PLAN}.md`。
- 实际验证：先执行 `cargo test --manifest-path src-tauri/Cargo.toml --locked project_directories_use_book_titles`，确认旧代码把“三体”创建为随机ID，回归失败；修复后 `npm run test:rust` 为70通过、1项既有原生凭据测试忽略，`npm test` 为106项前端测试与30项发布脚本测试通过、1项macOS检查跳过。`npm run lint`、`npm run build`（含TypeScript）、`npm run check:rust`（fmt / Clippy）及 `git diff --check` 均通过。Rust 命令先执行 `. ./.tools/env.ps1`，测试进程的 `TEMP`、`TMP`、`SQLITE_TMPDIR` 指向仓库 `output/naming-test-temp`。
- 未进行安装包构建、正式数据迁移或桌面人工验收。下一步使用新版在 Windows 新建书籍并重复生成任务包，核对资源管理器中的中文目录。

2026-10-25 新增「关于与更新」切片（feat，兼容功能增强，版本仍由发布流水线统一递增）：当前版本与说明、启动自动检查/后台下载、手动检查/下载、进度与错误重试、人工确认安装/重启；复用 app_settings 持久化偏好，schema v9 不变。同步接入官方 Tauri 更新插件、签名更新包、四平台 latest.json 和统一 Release 说明。验收证据、首次 Secrets 配置及真实安装/发布边界见 `UPDATES-VERIFICATION.md` 与 `RELEASE.md`。

## 当前执行范围（2026-10-25）

用户七项清单已落实：已有按项调整继续保留，本轮新增文案/封面自动暂存与重启恢复、完整备份目录与可选视频、封面拖动/缩放与三比例并排预览、明确文案历史来源及封面缩略图、个人封面样式、TXT/Markdown 来源资料导入。数据库 v9 兼容迁移；发布影响为 `feat`，本轮不手动升版本、不提交或发布。

最终验证与逐项验收见 `CREATOR-WORKFLOW-VERIFICATION.md`。前端95项、发布脚本27项通过/1项macOS跳过、Rust66项通过/1项既有原生凭据忽略；lint、TypeScript/构建、fmt/Clippy通过。独立 Windows WebView2/IPC/SQLite 已验证草稿正常退出重启、三比例 PNG、个人样式、历史文案任务包与含素材完整备份恢复。新安装包/升级、外部付费模型、原生文件选择器全流程和 macOS/Linux 本轮未验收。

## 按项调整已完成记录（2026-10-25）

本轮完成「按项调整」：七项内容可选一项或多项、填写修改要求、预览对比后采用，未选项原样保留。复用生成任务、Provider 与音频模板，候选不直接写入正式历史；采用事务提交并支持重复请求返回同一版本。数据库增加 v8 候选字段，兼容旧备份。业务影响为 `feat`，版本由发布流水线处理，无新增依赖。

验收完成：lint、TypeScript / 构建、81项前端测试、Rust fmt / Clippy、58项 Rust 测试通过，1项已有原生凭据测试保持忽略。独立 Windows WebView2、真实 Tauri IPC、SQLite 与本机契约服务通过生成、预览、采用、放弃、取消、额外字段拒绝及音频模板验证；数据库与备份的完整性、外键和字段比对通过。命令及边界见 `REVISION-VERIFICATION.md`。外部真实模型、新安装包与 macOS/Linux 实机未验证。

## 封面素材生成历史范围（2026-10-24）

本轮追加封面素材业务切片：在一次文本生成中增加封面主副标题、4:3 与 3:4 图片提示词，支持编辑/复制/历史保存及带入封面工坊。验收要求：新输出完整且通过长度/比例校验；旧三项数据仍可编辑保存；模型失败不丢稿；历史直达使用选中版本；已有封面由用户主动应用标题；新字段经备份恢复和重启保留。下述 M5 为此前完成的历史范围，不代表本轮新功能已完成安装或外部模型验收。本次业务影响为 `feat`，后续发布应升 minor，不手工改变现有版本号。

此前 M5 使用用户确认的《论语》《西游记》《朝花夕拾》和验收视频，完成原三项生成、双比例封面与半自动发布准备的整体测试、Windows 安装升级、数据恢复、异常验证和用户文档。历史证据不代替本轮封面素材生成验收。

沿用 M1–M4 已实现的业务，不重新启用历史大纲/完整讲稿、运营统计或自动发布。视频是流程验收素材，实际音频制作、对应书籍成片、平台上传预览与最终发布需在真实创作时完成。

M5 证据见 `M5-VERIFICATION.md`，使用步骤见 `USER-GUIDE.md`。M0–M4 的历史记录分别保留在对应验证报告中。

## 1. 开发策略

采用分阶段、可验证的业务切片开发。每阶段完成后进行验收，再进入下一阶段。MVP 不包含抖音官方自动发布能力。

## 2. 里程碑

### M0：工程初始化与技术验证
预计 1–2 个工作日。

当前状态（2026-10-23）：核心验收已通过。已实际启动 Tauri 开发版，构建并安装 NSIS 包，启动安装后的程序，核对 SQLite 在进程关闭后保留文字与原时间戳，并观察到原生对话框返回本地 MP4 路径。lint、类型检查、前端 5 测试、Rust 5 测试、Clippy 和构建通过。证据及尚未覆盖的发布场景见 `M0-VERIFICATION.md`。该记录对应 M0 验收；后续 M1 进度见下一节。

任务：
- 初始化或检查 Vue 3 + TypeScript + Vite。
- 初始化或检查 Tauri 2。
- 配置开发、构建、测试命令。
- 建立基础路由与布局。
- 验证 SQLite 读写。
- 验证本地文件选择。
- 建立基础错误处理。
- 验证 Windows 打包。

验收：
- 开发环境可启动。
- Windows 安装包可构建并启动。
- SQLite 写入后重启仍存在。
- 可以选择本地视频。
- 没有未解决的构建错误。

M0 后补充工程能力（2026-10-23）：新增 main 代码推送后的自动语义版本、Windows x64 / macOS ARM64 与 x64 / Linux x64 构建矩阵、GitHub Release 产物汇总和版本提交回流 dev。支持 PR 合并及本地 cherry-pick 后推送，版本提交不重复触发打包。Windows EXE 与 ZIP 已本地构建；macOS/Linux 的原生构建、安装及真实 GitHub 发布需首次远端运行验收。实现与失败恢复见 `RELEASE.md`，不等同于 M5 业务交付完成。

### M1：项目与素材管理

当前状态（2026-10-23）：核心验收完成。项目、素材、工作目录、备份恢复、迁移及重启持久化通过；安装版原生素材选择/取消、默认播放器实际播放、独立 Windows 安装及 0.1.0→0.1.1 升级保留数据通过。自动化与构建通过，完整证据和发布边界见 `M1-VERIFICATION.md`。

预计 2–3 个工作日。

任务：
- 数据库迁移。
- 项目 CRUD、搜索、筛选和归档。
- 素材文件关联。
- 文件存在性检测与重新定位。
- 工作目录管理。
- 基础备份。

验收：
- 项目创建、编辑、查询和归档正常。
- 重启后数据不丢失。
- 删除项目不默认删除源视频。
- 视频路径失效可被识别并重新关联。
- 迁移不会重复破坏数据。

### M2：AI 创作

当前状态（2026-10-23）：文本创作基础闭环已实现并完成 Windows 基础验收。用户指定的 HTTPS 服务与 gpt-6-astra 模型 ID 已在隔离安装版通过五类真实生成、修订及历史切换；M1→M2 的 v2→v3 数据迁移和 M2 安装升级保留项目、7 个版本、5 条任务及系统凭据，升级后远程连接成功。质量使用有来源的《论语·学而》公开原文基准复核；用户暂无个人读书笔记，个人试用与 macOS/Linux 凭据未验收，完整 PRD 增强也未全部实施。证据、完整命令和范围边界见 `M2-VERIFICATION.md`。

预计 3–5 个工作日。

任务：
- 文本模型 Provider 接口。
- 至少一个真实可用的模型适配器。
- 模型配置与安全凭据存储。
- 连接测试。
- 大纲、讲稿、标题、描述和话题生成。
- 结果编辑和版本管理。
- AI 任务状态与错误处理。

验收：
- 可调用已配置模型。
- 请求失败有明确提示。
- 输出通过结构校验后才入库。
- 失败不会丢失草稿。
- 可保存并切换历史版本。
- 日志不包含 API Key。
- 更换模型不需要改业务组件。

### M3：封面工坊
当前状态（2026-10-24）：本地模板、背景图、中文排版、预览、PNG 导出、版本管理和可选 AI 背景均已实现。SQLite v6、v5 旧数据升级、图片模型与系统凭据恢复、双比例导出、v6 备份恢复及独立 Windows 安装重启已通过；UGQ.ai 外部图片服务已实测；正式安装升级与 macOS/Linux 实机待验收。证据见 `M3-VERIFICATION.md` 文末。

预计 2–4 个工作日。

任务：
- 模板数据结构。
- 背景图选择。
- 标题、副标题和品牌标识。
- 字体、字号、颜色和布局。
- 实时预览。
- PNG 导出。
- 封面版本管理。
- 可选图片生成 Provider。

验收：
- 不配置图片 API 也能导出封面。
- 中文显示正常且文字不越界。
- 输出尺寸正确。
- 历史版本不被覆盖。
- 重启后可以恢复编辑配置。

### M4：发布中心
当前状态（2026-10-24）：核心验收完成。书籍内发布准备、任务包、素材检查、中文复制及人工结果记录通过；独立 Windows NSIS 安装、v6→v7 旧数据保留、重启和 v7 备份恢复通过。未操作真实平台上传/最终发布，正式安装与 macOS/Linux 实机未验证；完整证据见 `M4-VERIFICATION.md`。
预计 2–3 个工作日。

任务：
- 标题、描述、标签、封面和视频选择。
- 发布任务创建。
- 任务包生成。
- 发布检查清单。
- 一键复制。
- 打开文件和发布入口。
- 发布结果记录。

验收：
- 一键生成完整任务包。
- 视频引用路径正确。
- 缺失视频时不能进入 ready 状态。
- 未确认标题或封面时有提示。
- 中文复制正确。
- 用户确认发布后记录正确状态。
- 任务包生成失败时保留源素材。

### M5：测试、打包与试用
当前状态（2026-10-24）：当前产品范围内七项核心验收完成。《论语》《西游记》《朝花夕拾》通过真实文本模型生成、编辑/复制/历史切换、六张中文封面和三套完整发布任务包；Windows 隔离安装、v6→v7 旧数据保留、正常退出重启和备份恢复通过。用户继续后，正式安装 0.1.0→0.5.1、v1→v7 原记录保留、正式库备份恢复及 ZIP 启动也已验证，并保留升级前回退材料。修复同大小损坏封面仍显示就绪的问题；命令、失败记录和本地交付文件见 `M5-VERIFICATION.md`。真实视频作品、平台上传/最终发布、macOS/Linux 实机仍待对应使用验收。

预计 2–4 个工作日。

任务：
- 单元测试和关键流程测试。
- 验证迁移、备份与恢复。
- 验证文件缺失和重新定位。
- 验证模型失败与异常响应。
- 验证 API Key 安全。
- 验证中文封面排版。
- 验证任务包完整性。
- 验证安装、启动、退出和升级。
- 完善用户文档。
- 使用至少 3 个真实书籍项目试用。

验收：
- 核心测试通过。
- 失败场景有明确提示。
- 草稿不会因模型失败丢失。
- 安装包可在目标 Windows 环境启动。
- 备份可恢复。
- 3 个真实项目可完成发布准备流程。
- 无已知高优先级数据丢失问题。

## 3. 总体时间估算

预计 MVP 约 2–4 周，取决于开发者投入、AI 服务调试、Tauri 环境和封面排版复杂度。该估算不包含官方发布权限申请周期。

## 4. 发布前检查清单

以下完成标记对应 M5 Windows 验收，包含后续正式安装升级与 ZIP 启动；不代表平台作品发布或其他系统验证。

- [x] 新建项目正常。
- [x] 项目数据持久化。
- [x] 发布标题、发布内容和音频制作指导生成正常（历史大纲/讲稿已不在当前产品范围）。
- [x] AI 输出可编辑并有版本记录。
- [x] 文案生成与复制正常。
- [x] 封面模板独立可用。
- [x] 中文封面导出正常。
- [x] 视频关联和重新定位正常。
- [x] 原始视频不会被意外删除。
- [x] 发布任务包内容正确。
- [x] 缺少素材时给出提示。
- [x] 发布记录可手动维护。
- [x] 密钥不写入日志或 Git 工作区（实际凭据扫描与系统凭据回归）。
- [x] 备份与恢复验证通过。
- [x] Windows 安装包验证通过。

## 5. 风险与应对

- AI 输出不稳定：结构校验、Prompt 版本、人工审核、保留历史。
- API 不兼容：Provider Adapter 和契约测试。
- 密钥泄漏：系统凭据存储、日志脱敏、最小权限。
- 文件移动：路径检测、重新定位、保留记录。
- 图片服务不可用：本地模板降级。
- 无法取得抖音发布权限：保留半自动发布价值。
- 范围膨胀：冻结 MVP，新增功能需评估对当前验收的影响。

## 6. shadcn-vue 全页面接入（2026-10-25）

已完成：统一组件基础、应用侧栏与顶栏、书架、书籍表单、内容创作、封面工坊、发布准备、模型设置、数据备份、素材管理、技术验证页及被替代样式的清理。数据库和业务能力范围不变，发布影响按 `style` / `refactor` 处理。

验收：lint、TypeScript / 生产构建、最终 73 项前端测试通过；发布脚本测试 27 通过、1 个 macOS 专属检查跳过。浏览器使用隔离的内存 IPC 检查 9 个路由在 1440px、1120px、800px 与 360px 下的 36 组布局，均无横向溢出、遗留原生控件和未命名下拉框；另验证封面键盘选择/保存、取消历史切换、模型停用联动与发布确认重置。证据与未覆盖范围见 `UI-COMPONENTS-VERIFICATION.md`。

Windows 桌面补充核对（2026-10-25）：独立 identifier 的当前源码开发运行版已验证 WebView2 下拉浮层、焦点返回、背景滑块、原生日历键盘选日，以及目录、素材、背景图片的真实选择和取消保留。颜色输入保存、SQLite、双比例 PNG、任务包与备份文件比对通过，页面错误为 0；lint、构建和完整前端 / 发布脚本测试重跑通过。详细证据见 `UI-COMPONENTS-VERIFICATION.md` 的桌面补充记录。

剩余：原生颜色弹层鼠标选择受 Computer Use 窗口归属检查拦截，随后窗口激活失败，未将该项记为通过；原生备份文件选择器也未完成。恢复桌面交互后先补测这两项。安装包更新、缺少 WebView2 的环境及 macOS/Linux 实机验证另行执行；本次独立开发运行版结果不代表正式数据库、真实模型或安装版验收。

## 封面 16:9 尺寸扩展（2026-10-25）

- 新增横版 16:9（1920×1080），与已有 4:3、3:4 共用编辑配置、预览和导出。属于兼容的用户功能增强（feat），版本交由发布流水线处理。
- 验收：可选中 16:9 并保存/恢复；一次导出三种准确尺寸；已有 16:9 历史不被转换；任一比例排版失败不写文件，部分写入失败保留已完成版本与草稿。
- 数据库 v7、既有模型输出和发布准备的两封面要求不变。
- 已验证：`npx vitest run tests/covers.test.ts tests/coverRenderer.test.ts tests/publishing.test.ts`（44 通过）、`npm run lint`、`npm run build`（含 TypeScript）、`npm run check:rust`（fmt / Clippy）均通过。Rust 命令先加载 `. ./.tools/env.ps1`；首次 `npm run test:rust` 因 C 盘临时目录空间不足失败，将本次进程的 `TEMP`、`TMP`、`SQLITE_TMPDIR` 指向仓库 `output/cover-16x9-test-temp` 后重跑，53 通过、1 项系统凭据测试按原设置忽略。
- 未执行真实桌面人工验收或安装包构建；下一步在 Windows 桌面版选择 16:9，确认预览和三张 PNG 导出。C 盘剩余空间不足的问题未作清理。
