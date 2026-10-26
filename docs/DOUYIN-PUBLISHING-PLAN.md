# 抖音应用内辅助发布：产品与开发方案

日期：2026-10-26。当前状态：M6 Windows 核心验收及 M7 日常任务列表已完成；真实扫码、完整原生上传/文案/双封面/回读、独立安装升级与恢复已验证。证据和未覆盖项见 [M6-VERIFICATION.md](M6-VERIFICATION.md)、[M7-VERIFICATION.md](M7-VERIFICATION.md)。本文保留原设计与研究依据，不将设计阶段的未知项当作当前验证结论。

## 1. 已确认的目标

用户已选择：**自动上传和填写，我检查后点一次发布**。

目标流程：选择现有发布任务 → 应用内打开抖音创作者中心 → 首次扫码登录 → 自动上传视频与封面、填写文案 → 用户检查并亲自点击抖音网页的发布按钮 → 回到书引 AI 确认结果、保存作品链接。

首轮范围是 Windows、个人单账号、同一时间一个发布任务。技术验证通过后实现完整闭环。扫码、验证码、平台声明和最终发布由用户操作；程序不代点最终发布，不读取或导出 Cookie，不读取密码，不调用逆向得到的私有接口。

## 2. 可行性及证据边界

设计阶段确认了容器和浏览器基础能力；后续已在真实官方页面验证登录、视频、中文文案与双封面，原生安装版已连续完成整链。验证只对应当日页面和 Windows 环境，不承诺平台后续页面兼容性。下表为设计阶段依据与当时待验证项，最新结果以验收报告为准。

| 项目 | 本次确认的依据 | 仍需验证 |
| --- | --- | --- |
| 应用自己的网页窗口 | 当前 Tauri 2.12.5 源码提供 WebviewWindowBuilder、外部 URL、导航回调 | 当前抖音登录页、弹窗和回跳是否正常 |
| 登录会话保留 | WebView2 官方说明用户数据目录保存 Cookie、权限与缓存；Tauri 提供 data_directory | 抖音会话失效行为；不承诺永久登录 |
| 自动填写文字 | 当前 Tauri 提供 eval_with_callback，可执行受控脚本并读取结果 | 实际编辑器结构、输入事件和页面最终采用的值 |
| 指定本地上传文件 | 微软公开 WebView2 的 CDP 调用；Chrome 官方协议定义 DOM.setFileInputFiles | 当前网页控件、视频处理、双封面弹窗与裁切流程 |
| 官方 API 发布 | 抖音官方资料列出 video.create，需要申请权限和用户授权 | 本项目未申请或验证权限；网页登录不等于获得 API token |
| 网页适配支持范围 | 未取得抖音对本项目网页适配的支持或兼容性承诺 | 实施前核对当时平台规则与真实页面，不能用第三方工具案例代替 |

研究基于官方资料与当前安装源码。部分抖音动态文档本次仅获得官方索引摘要，未取得完整正文；申请资格、平台规则及账号能力不得据此下确定结论。

### 官方资料

1. [WebView2 容器介绍](https://learn.microsoft.com/en-us/microsoft-edge/webview2/)：网页内容可嵌入原生应用。
2. [Tauri WebviewWindowBuilder](https://docs.rs/tauri/latest/tauri/webview/struct.WebviewWindowBuilder.html)、[WebviewWindow](https://docs.rs/tauri/latest/tauri/webview/struct.WebviewWindow.html)：窗口与脚本接口。网页文档作为入口，具体签名已与锁定的 2.12.5 源码核对。
3. [WebView2 用户数据目录](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/user-data-folder)：会话由浏览器管理，业务代码无需提取 Cookie。
4. [WebView2 中使用 CDP](https://learn.microsoft.com/en-us/microsoft-edge/webview2/how-to/chromium-devtools-protocol)、[Chrome 官方协议 JSON](https://raw.githubusercontent.com/ChromeDevTools/devtools-protocol/master/json/browser_protocol.json)：本次实际读取 JSON，确认 DOM.setFileInputFiles 接受文件路径数组和节点标识；不代表抖音上传已通过。
5. [Tauri capabilities](https://v2.tauri.app/security/capabilities/)：远程来源访问 IPC 需要明确配置；发布网页不授予业务权限。
6. [抖音总体授权说明](https://open.douyin.com/platform/resource/docs/develop/permission/overall-permission/)：第三方 OAuth 授权与在创作者中心登录区分处理。
7. [抖音上传视频](https://open.douyin.com/platform/resource/docs/openapi/video-management/douyin/create/upload/)、[创建视频](https://open.douyin.com/platform/resource/docs/openapi/video-management/douyin/create/create-video/)：官方索引列出 video.create、申请权限及用户授权要求。
8. [抖音创建视频使用限制](https://developer.open-douyin.com/docs/resource/zh-CN/dop/develop/openapi/video-management/douyin/create-video/video-create)：官方索引说明创建后有审核过程；上传或创建完成不能视为审核通过。

本机依赖来源：`D:/MyWork/software/rust/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/`。已检查 `tauri-2.12.5/src/webview/webview_window.rs`、`tauri-2.12.5/src/webview/mod.rs`、`tauri-2.12.5/src/window/mod.rs` 和 `wry-0.57.0/src/lib.rs`。Cargo.lock 同时锁定 tauri-runtime-wry 2.12.3、webview2-com 0.39.1。

## 3. 当前代码复用

| 现有能力 | 实际位置 | 扩展方式 |
| --- | --- | --- |
| 选文案历史、视频、双封面和人工确认 | src/modules/publishing/PublishingPage.vue | 继续作为唯一发布准备入口 |
| 发布 IPC | src/services/publishing.ts、src-tauri/src/publish_commands.rs | 增加必要的受限操作 |
| 任务类型 | src/types/publishing.ts | 保留 draft / ready / published 的现有语义 |
| 任务包与校验 | src-tauri/src/database/publishing.rs：checked、manifest、package_valid、prepare_publish_task | 复用校验并生成填写快照，不另写任务包解析器 |
| 抖音入口 | 同文件 open_portal；新增 douyin.rs | 保留默认浏览器入口，应用内窗口与自动填写集中在新增模块 |
| 发布结果事务 | 同文件 record_publish_success | 保留人工确认后的记录和项目状态更新 |
| 最小授权 | src-tauri/capabilities/main.json、src-tauri/permissions/publishing.toml | 主窗口控制发布，远程网页无业务权限 |

任务包已有文字快照、视频路径、两张封面和 manifest，视频默认只引用。自动填写采用选定的已保存任务，复用 manifest 与包校验，上传包内对应封面及其引用的视频，不用“最新文案”替换已选历史。

TECHNICAL-DESIGN 中的 PublishProvider 是原有设计建议；当前源码实际为服务函数转发到 Rust，没有现成的 LocalPublishProvider 类。新实现按真实边界接入，不为对齐旧草图制造重复实现。

## 4. 产品交互

### 发布准备页

保留素材和文案编辑区，新增一个发布操作区：

- 展示当前任务、视频名称、封面缩略图与文案预览。
- 实际入口为「打开 / 显示抖音窗口」→「检查抖音页面」→人工确认账号→「上传并自动填写当前任务」。仅操作已保存的有效 ready 任务；未保存修改先完成原有保存/任务包流程，失败时指出问题。
- 步骤提示：「等待扫码」「上传视频」「填写文案」「设置封面」「请检查并发布」。无法获得百分比时只显示步骤。
- 活动期间提供「显示发布窗口」「停止自动填写」；重复点击只聚焦现有会话，不再次上传。
- 填写完成后提示用户在抖音网页检查并发布，停止自动写入，允许用户直接修改网页内容。
- 沿用「记录发布成功」和作品链接、实际时间、备注；上传完成或审核中不自动记成功。

### 发布窗口

首版使用应用管理的独立原生子窗口，标题「抖音发布 · 书名」，内部显示官方网页。它属于书引 AI，不启动默认浏览器；二维码、上传进度、封面裁切与平台提示直接可见。

选择原因：当前 Tauri 2.12.5 同窗多 WebView 的 add_child 受 unstable 特性控制；独立 WebviewWindow 已有必要 API，可避免扩大主窗口 CSP 和改造整个布局。

登录成功以页面出现账号和上传入口为依据，不能仅凭二维码消失。开始上传前展示当前账号供核对；若网页已有其他草稿或素材，暂停让用户处理，不自动覆盖。

| 本地任务内容 | 页面处理要求 |
| --- | --- |
| 视频引用路径 | 原生浏览器接口选中当前任务的视频，等待网页确认上传及处理结果 |
| 标题、发布内容 | 按实际编辑器填写，保留中文与换行，并回读核对 |
| 可选标签 | 采用用户已有标签，适配实际话题控件；不新增话题生成能力 |
| 4:3 / 3:4 封面 | 核实页面要求后对应上传与预览，不能仅凭本地名称断言成功 |
| 可见范围、原创/AI 声明等 | 用户按内容实际情况确认，不自动猜测或勾选 |

本地业务字段不代表网页必然有相同数量的输入框。若页面共用编辑区，适配层统一组合并只填写一次。若封面比例要求不同，先确认真实要求，再调整导出或由用户在官方界面裁切，不静默拉伸。

## 5. 最小技术实现

```text
发布准备页 → 受限 Tauri Command（taskId）
  → 现有任务/素材校验，生成只读快照
  → PublishProvider 边界下的 DouyinWebPublishProvider
      → 应用自己的 WebviewWindow
      → 页面识别和文案填写脚本
      → Windows WebView2 原生文件选择
  → 步骤结果回到主窗口

用户在官方网页手动发布 → 用户确认实际成功 → 现有结果记录事务
```

一个平台保留一个实现，用少量操作表达打开、填写、读取阶段和停止。不建插件注册器、通用 RPA 框架或第二套任务系统，不增加自动点击最终发布的操作。

1. 从异步 Command 创建窗口。当前 Tauri 源码提示 Windows 同步命令内创建窗口存在死锁问题。
2. 页面读写优先使用锁定版本的 eval_with_callback，返回简短结构化结果。脚本显式捕获错误：该 API 注释提示 Windows 下异常不会自动回传为错误。仍需回读页面值验证输入确已采用。
3. 文件上传验证路线为宿主进程内 WebView2/CDP 的 DOM.setFileInputFiles；不把大视频转为 base64，不向网页开放任意文件读取，不把路径当普通 input 文本赋值。
4. 发布产品不开放远程调试端口。若 Rust COM 需要直接依赖，先核对锁文件已有版本并做可编译验证，再确定最小变更，不预先承诺零依赖。
5. 仅对实测确认的官方发布页面执行适配；顶层导航、登录回跳、新开窗口按真实官方域名核对，不泛化为任意网址。
6. 控制命令仅由可信本地主窗口调用；发布窗口不加入主窗口 capability，也不授予 remote IPC。事件只发向主窗口，主窗口 CSP 保持原边界。
7. 使用独立 WebView2 数据目录，与创作主窗口隔离。登录态不进入 SQLite、备份、任务包或日志；退出登录走官方页面，清除会话只处理专用目录。
8. Rust 只维护一个活动会话，绑定任务、快照、窗口、步骤。上传前再次校验；活动期间禁止在应用中改写该任务。人工接管后停止写入，重新填写须明确触发。

预计影响：现有发布页、publishing service/types、Rust publish commands/lib、一个集中维护的发布 Provider 模块及页面脚本、权限声明和相关测试。浏览器操作不继续放入数据库模块，具体拆文件以实际规模为准。

## 6. 状态、异常与数据

首轮无需数据库迁移，复用 schema v9：draft 表示待完善；ready 表示任务包有效；published 表示用户确认实际成功。ready 不表示上传成功，published 不能由上传回调触发。

浏览器步骤是短暂会话，Rust 维护一份、前端仅显示。重启后不承诺恢复上传进度：先查看官方网页已有草稿，检查后继续，不能自动重传或发布。

| 情况 | 处理 |
| --- | --- |
| 登录失效、验证码、风控提示 | 暂停，用户在官方页面处理，再检查状态 |
| 文件缺失、封面变化、任务更新 | 拒绝继续，保留任务，修正并重新生成包 |
| 网络失败或上传超时 | 告知具体步骤并保留网页；用户重试前先检查是否已经上传 |
| 页面变化、目标不唯一 | 停止并指出未完成项，不猜测点击位置；用户在原网页继续 |
| 停止自动填写 | 停止后续脚本；已发起的网页上传可能仍继续，取消由官方页面处理 |
| 关闭窗口或程序 | 不清空本地内容、不记成功；会话终止，不自动重启操作 |
| 页面提交成功/审核中 | 如实提示，不自动写入 published |
| 用户改动网页文案 | 不覆盖；可在结果备注说明，首轮不自动反写文案历史 |
| 识别到作品链接 | 仅预填待确认字段，不作为成功证据或静默保存 |

若以后要持久化“已提交/审核中/未通过”，另做与平台事实来源对应的兼容迁移和备份恢复验证，本轮不提前加字段。

## 7. 后续开发顺序与验收

每阶段按「明确验收 → 最小实现 → 自动化验证 → Windows 真实运行 → 更新文档」执行。前一阶段失败就修正该环节，不带着未知能力继续堆界面。

| 阶段 | 交付 | 通过标准 |
| --- | --- | --- |
| M6.1 技术验证 | 隔离验证版的网页窗口、扫码、文字、视频和双封面试验 | 真实扫码进入编辑页；文件和文字实际显示正确；记录页面/版本证据；最终发布保持人工 |
| M6.2 单任务闭环 | 接现有包、步骤提示、账号核对、停止/接管、人工记录 | 一项有效任务完成全部填充；错误账号/旧草稿不误操作；源视频和历史保持不变 |
| M6.3 稳定性与交付 | 过期、断网、重复点击、关闭重开、已有草稿、页面缺项回归及 Windows 打包 | 不丢任务、不误报成功、不重复上传；安装版复现；旧备份可恢复 |
| M7 日常效率 | 基于现有任务的待发布列表、记住上次内容来源、逐篇处理、作品链接归档 | 多本书不串文案/素材；一篇处理完再进入下一篇，逐篇人工最终发布 |

M6.1 需要用户真实扫码及确认可上传的试验素材；浏览器模拟和本地夹具不能代替。不要求为技术验证发布测试作品。

优先完成发布闭环，随后做异常恢复、待发布列表和作品回溯。多账号并发、多平台矩阵、无人值守定时发布与运营数据抓取不在当前排期。官方 API 待有明确业务需要与权限证据再评估，不与网页路线并行开发。

### 验证要求（实际结果见 M6-VERIFICATION）

- 前端：npm run lint、npm run typecheck、npm test、npm run build。
- Rust：先 `. ./.tools/env.ps1`，再 npm run test:rust、npm run check:rust。
- 桌面：npm run desktop:build，使用隔离配置和数据目录完成 Windows 登录与上传验收。
- 回归：任务校验复用、命令来源/路径限制、重复触发、停止后不再写、页面缺项不猜测、验证码暂停、上传成功不写发布成功、人工记录事务保持正确。
- macOS/Linux 保持现有发布准备；原生上传未分别验收前不展示为已支持。

## 8. 原设计阶段交付记录

- 内容：完成可行性研究、产品流程、技术边界、开发顺序与验收设计，未修改应用功能。
- 文件：本文件、DEVELOPMENT-PLAN.md 中的计划入口。
- 数据库：无变更、无迁移；M6 首版计划复用 schema v9。
- 已实际执行：读取代码、锁定版本及依赖源码；检索官方文档；PowerShell Invoke-RestMethod 读取 Chrome 官方协议 JSON，确认 setFileInputFiles 定义。
- 文档检查：`git diff --check` 通过；PowerShell `Test-Path` 确认新增文档及引用的 7 个源码路径存在；新增文档行尾空白正则检查通过。`git diff --stat` 不包含未跟踪的新文档，已结合 `git status --short` 核对实际只改动这两份文档。
- 未执行：真实账号登录、抖音编辑器/上传/封面、安装包、跨平台；平台规则全文未核实。仅文档变更，本次未运行 lint、单测或构建。
- 验收对应：唯一目标、实际代码复用、阶段通过条件、证据与未知项均已明确；这不是功能验收通过记录。
- 下一步：实施 M6.1，先验证登录与文件上传，通过后做 M6.2。

以上为实现之前的 docs 记录。当前辅助发布是兼容功能增强 feat，按项目规则升一次 minor，不按工时或文件数量升版本；本地验收使用临时版本，不修改正式清单。
