# 应用内网页自动音频设计与实施

日期：2026-10-29。状态：应用内实现已替换扩展链路；本地回归和 Windows 隔离桌面验收见 IN-APP-AUDIO-VERIFICATION.md。真实 Google 登录、网站控件与生成下载尚未验收，不能据此承诺官方服务可用。

## 1. 最终方案

**书引 AI 创建专用 Tauri 网站窗口，由 Rust 自动跟进任务，网页脚本定位入口，原生宿主负责文件选择和下载；用户无需安装 Chrome 扩展或桥接程序。**

“应用内”是同一个桌面应用管理的独立 WebviewWindow，不是 Vue iframe。保留现有任务卡片、来源与格式选择、音频素材列表和手动导入，不新增任务中心或另一套自动化。

Google 登录是尚未解决的外部验收前提。官方 OAuth 政策限制开发者可控的嵌入式 user-agent；本实现没有证明 Notebook 当前登录路径可在 WebView 中受到官方支持。[S1] 不伪装浏览器、不复制 Cookie、不改写授权请求、不接入网页私有 RPC。若官方页面拒绝该环境，应停止并保留本地任务，使用既有手动导入。

原设计建议先完成真实 P0 再替换；用户随后明确要求实现。本轮完成替换和本地可验证链路，**真实服务 P0 仍未通过，不能将代码完成或模拟网页通过写成 Google 验收通过**。

## 2. 用户流程

1. 进入「音频制作」，使用本书绑定文档或资料文本，选择深入探索或简报。
2. 点击「开始生成音频」，先保存目标时长和完整指导，再冻结来源、版本和参数。
3. 应用打开专用 Notebook 网站窗口。需要登录时显示「等待 Google 登录」，用户自行处理登录、验证码和二次验证；支持程度以真实页面为准。
4. 自动创建包含书名及任务短编号的笔记本，取得实际 URL 后保存。
5. 自动添加来源、等待处理、定位 Audio Overview 和自定义入口、选择格式和简体中文、填写并回读完整指导，然后提交一次生成。
6. 主工作台显示真实阶段、等待时间及最近有效检查。网站关闭按钮只隐藏窗口，「查看网站」重新显示。
7. 成品下载到任务缓存，校验并复制到本书 audio/，事务登记成功后才显示完成。
8. 剪映制作、成片关联和发布准备沿用现有流程；最终发布仍由用户操作。

截图只用于确定要修改的界面区域。书名、路径及网页文字是资料，执行对象来自保存的当前任务快照。目标分钟数与主持人数以网站实际产物为准，应用不能作精确保证。[S2]

## 3. 最小架构

~~~mermaid
flowchart LR
  UI[现有音频页面] --> Commands[Tauri 音频命令]
  Commands --> DB[SQLite audio_jobs]
  Commands --> Runtime[Rust 单一调度]
  Runtime --> Host[共享原生网页调用]
  Host --> Page[专用窗口与页面脚本]
  Page --> Native[WebView2 具体下载对象]
  Native --> Archive[校验与素材事务]
  Archive --> DB
  DB --> UI
~~~

| 文件 | 职责 |
| --- | --- |
| src/modules/writing/WritingPage.vue | 输入、快照、完整 URL、查看网站、暂停/继续/停止 |
| src-tauri/src/audio_commands.rs | 主工作台启动与控制，窗口可用性检查 |
| src-tauri/src/audio.rs | 单窗口、实例互斥、串行调度、导航检查、运行失效、有限等待 |
| src-tauri/src/audio_page.js | 一份中英文语义页面适配，只执行当前阶段动作 |
| src-tauri/src/webview_host.rs | 从已有抖音宿主整理的 Windows CDP 调用，两处共用 |
| src-tauri/src/audio_download.rs | 绑定 DownloadStarting、具体下载对象及完成/中断/取消 |
| src-tauri/src/database/audio.rs | 唯一持久状态、快照、恢复、校验与归档 |

锁定版本：应用 0.21.0、SQLite v13、Tauri Rust 2.12.5、前端 API 2.12.4、webview2-com 0.39.1。windows-core 0.62.2 原已间接锁定，本轮显式声明以使用 COM 接口转换；不增加浏览器服务或运行时扩展。

## 4. 窗口与页面操作

- 窗口标识 audio-notebook，独立资料目录为应用数据下的 audio-webview，不导入系统 Chrome 会话。
- 顶层只允许 HTTPS、443、无用户名密码的精确 notebook.google.com、notebooklm.google.com 和 accounts.google.com；下载期间另仅允许已绑定的精确下载 URL。
- 登录页只供用户操作，不执行音频脚本。新窗口请求当前拒绝；真实登录若依赖弹窗，须单独实现与验收，不能声称已经支持。
- 远程窗口没有主工作台 capability、业务 IPC 或任意本机路径权限；主工作台显示完整网址。
- 使用宿主 Runtime.evaluate(awaitPromise) 获取结构化结果，单次调用上限 30 秒。动态参数通过 JSON 编码。
- 控件依据可访问名称、角色、标签及实际可见文字定位；按钮、音频区域或下载目标不唯一时停止，不猜坐标。
- 使用 jobId/runId 和文档代次拒绝失效结果。异步等待后的写入、点击再次核对运行令牌，暂停后不继续写网页。

## 5. 来源与生成

~~~text
created/opening_site → waiting_login（需要时）
→ creating_notebook → adding_source → source_processing
→ configuring_audio → submitting → generating
→ downloading → importing → succeeded/completed
~~~

原书只接受本书登记的 document 素材，冻结素材 ID、名称、大小和 SHA-256。上传前流式制作 .audio-jobs/{jobId}/{runId}/source/ 快照，重新核对摘要，仅把副本交给网页；不改写或删除原书，也不再 base64 分块传输。

宿主临时拦截文件选择对话框，通过唯一标记的节点调用 DOM.setFileInputFiles，不要求用户再选文件。[S5] 资料文本走“复制的文字”入口，不把指导当原书。

创建笔记本使用任务创建意图和实际 URL 核对身份，允许正常导航后继续登记。生成前保存 submitting；动作结果不明进入 waiting_user/verify_remote，不重建或重复生成。指导不能完整回读时不提交。

准备约每 2 秒、来源等待约 15 秒、生成等待约 60 秒观察一次。准备阶段上限 120 秒、来源 15 分钟、生成 6 小时、下载/归档 30 分钟；超时请用户查看网站，不无限重试。等待用户登录不消耗步骤等待额度。计时只限制当前进程执行，重启以数据库恢复判断为准。

## 6. 精确下载与归档

只保留 Windows WebView2 一条原生下载实现。[S3][S4]

1. 从唯一 Audio Overview 区域捕获实际 HTTPS/blob 地址；发现阶段阻止锚点默认下载。
2. Rust 建立 jobId/runId/document epoch/URL/不存在的目标路径意图，再在同一窗口发起该 URL。
3. DownloadStarting 默认拒绝未绑定操作。精确 URL、运行、文档代次和 15 秒有效期均匹配后才接收具体下载对象。
4. 路径固定为 .audio-jobs/{jobId}/{runId}/download.partial，不接收网页提供的本机路径，不覆盖文件。
5. 只有该对象完成回调且运行有效，才校验音频格式、大小与 SHA-256；HTML 和空文件不能登记。
6. 完整结果复制到 complete.audio，保存摘要和 importing；随后复用不覆盖的音频复制及 SQLite 素材事务，不转码或伪装格式。
7. 事务失败保留缓存，继续或重启后核对摘要，只重试归档；没有完成摘要的部分文件不能当成下载成功。

暂停/停止递增运行编号并尽力取消本机下载，旧回调不能改变新运行。缓存目前保留，不纳入完整备份；自动清理未作为本轮能力交付。停止本地跟进不取消 Google 已提交的生成，也不删除原书、成品或远端笔记本。

## 7. 兼容、收敛与影响范围

**继续使用 SQLite v13，无新表、列或迁移，不修改历史迁移。**

| 旧数据 | 行为 |
| --- | --- |
| 已成功任务、手动音频、发布引用 | 保留记录和文件 |
| 暂停的 awaiting_extension | 显示“等待在应用内继续”；继续转到 opening_site，保留快照 |
| 尚未开始且无 URL/提交记录的旧任务 | 重启后等待用户继续，再从应用内启动 |
| 已开始或疑似提交 | 重启递增运行编号，先只读核对原任务 |
| 有可靠 Notebook URL | 打开原页核对，不重加来源或重提生成 |
| 副作用不明且无 URL | 等待人工处理，不按同名笔记本推断 |
| 归档失败且缓存有完成摘要 | 验证后只重试素材登记 |
| 旧 Chrome 未登记下载 | 不迁移 downloadId 或扫描下载目录，可手动导入 |

扩展目录、helper 工程、分块接口、构建脚本、externalBin/resources 和 ZIP 扩展资源均已移除。NSIS 升级/卸载只清理精确对应本产品默认宿主清单路径的旧注册；升级仅删除指定的旧随包文件，不修改 Chrome 用户资料，不代卸用户扩展。用户可自行卸载旧扩展。

macOS/Linux 保留手动导入，自动任务显示明确不支持；本轮未在这两个系统执行编译或桌面验收。无提交、发布或替换正式安装。

## 8. 验收

实际命令与证据见 IN-APP-AUDIO-VERIFICATION.md。本地模拟记录使用独立应用标识，不代表 Google 生成结果。

发布前仍需用用户明确选定的来源验证官方允许的登录路径、真实中英文入口、资料与完整指导回读、一次真实生成、HTTPS/blob 下载关联、隐藏窗口等待及正式安装升级/卸载。未通过时保留手动导入并显示实际原因，不增加绕过路径。

## 官方依据

- [S1 Google OAuth 2.0 Policies](https://developers.google.com/identity/protocols/oauth2/policies)
- [S2 NotebookLM Audio Overview](https://support.google.com/notebooklm/answer/16212820?hl=en)
- [S3 WebView2 DownloadStartingEventArgs](https://learn.microsoft.com/en-us/microsoft-edge/webview2/reference/win32/icorewebview2downloadstartingeventargs)
- [S4 WebView2 DownloadOperation](https://learn.microsoft.com/en-us/microsoft-edge/webview2/reference/win32/icorewebview2downloadoperation)
- [S5 Chromium DOM.setFileInputFiles](https://chromedevtools.github.io/devtools-protocol/tot/DOM/#method-setFileInputFiles)
