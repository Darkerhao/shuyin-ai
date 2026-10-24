# 书引 AI 技术设计文档

当前产品范围为三项生成、M3 封面工坊（含可选 AI 背景）与 M4 半自动发布准备，已进入 M5 整体测试与试用。本文前面的完整工作台设计保留为历史记录；当前实现见文末对应说明及各里程碑验证报告。

## 1. 技术目标

构建 Windows 10/11、macOS 与 Linux 的本地优先桌面应用，围绕书籍项目完成 AI 创作、封面排版、视频关联、素材归档和半自动发布准备。人工桌面验收以 Windows 起步，新增平台需要分别验证安装、启动和持久化。

## 2. 技术栈

- Vue 3 + TypeScript + Vite：前端界面。
- Tauri 2 + Rust：桌面容器、系统能力和受控文件操作。
- SQLite：结构化业务数据。
- Pinia：跨模块状态。
- Vue Router：导航。
- Element Plus 或经项目确认的统一组件库：UI。
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
│   └── {书名}-{项目ID}/
│       ├── research/
│       ├── scripts/
│       ├── covers/
│       ├── publish/
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
- 工作目录默认在应用数据目录下，可在设置中选择其他磁盘。每个新项目创建 `projects/{id}` 目录，ID 命名避免书名改动和非法文件名引起搬迁。现有项目可重新指定目录，仅更新引用。
- 素材仅存路径及基本元数据；每次读取实际检测文件，区分可用、缺失和无法访问。重新定位保留素材 ID，校验素材类型和重复关联。文件打开只接受已登记且属于允许格式的素材 ID，目录打开只接受项目 ID。
- 基础备份使用 SQLite `VACUUM INTO` 创建唯一新文件，恢复使用现有 rusqlite 的 `backup` feature；未新增依赖包。备份不包括外部媒体文件。恢复先核验版本、完整性、schema 和外键，再自动备份当前数据库，最后使用 SQLite backup 原子替换数据库内容。
- M1 备份恢复只接受 schema v2；M0 数据通过正常启动迁移升级。目录重定位不自动迁移文件；媒体时长、分辨率、校验和提取不在本里程碑。

## M2 实际实现（2026-10-23）

- 页面：`/settings/models` 模型配置与测试，`/projects/:id/writing` 创作。业务组件只依赖 `services/ai.ts` 和 `types/ai.ts`，没有供应商请求、密钥读取或 SQL。
- Rust：`ai.rs` 的统一 `TextProvider::generate(TextRequest)` 接口与 `OpenAiCompatible` 适配器，`ai_commands.rs` 编排任务，`credentials.rs` 访问系统凭据，`database/ai.rs` 负责模型、任务与版本持久化。未新增 npm、UI 或全局状态依赖。
- 适配器调用 `{base_url}/chat/completions`，只发送 model/messages/stream=false，不假设模型支持 temperature、max_tokens 或 response_format。版本化系统 Prompt 位于 `prompts/writing-v1.md`，用户资料按独立用户消息中的 JSON 数据传入。
- reqwest 0.13.5 使用 Rustls 证书校验；120 秒总超时、15 秒连接超时、显式禁止自动重试和重定向。回环服务绕过环境代理，远程服务保留 reqwest 的环境代理行为；HTTP 仅允许 localhost/127.0.0.1/::1，其他服务必须 HTTPS，URL 不允许账号、查询或片段。
- HTTP 状态错误只返回本地固定文案，不向界面或日志返回服务响应体、授权头和底层网络错误。响应最多 2 MB，必须完整结束（finish_reason=stop）、有非空文本，再严格解析带 format 的 JSON 并核验内容类型、数量和长度才入库。连接测试验证文本通信，不代表生成 JSON 能力已验收。
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
- Rust `database/covers.rs` 校验两种固定尺寸、配置和 PNG 文件头/尺寸，将单张图的新版本与项目时间放入同一事务。每次导出写入项目 `covers/cover-{宽}x{高}-v{版本}-{随机ID}.png`，create_new 防止覆盖；文件写入或事务失败时回滚并清理本次新文件。正常错误不会覆盖旧版本；进程在落盘和事务提交之间被强制结束可能留下未登记的新 PNG，不损坏旧数据。
- cover_versions 配置包含 schemaVersion=1、模板、尺寸、文字、字体、颜色、布局及 backgroundAssetId。没有单独可变“当前封面”表，最新版本即重启默认配置。背景引用保留在配置快照中，素材被移除时可以显示缺失并重新选择，不改写历史。
- 旧尺寸历史读取不改写数据库或文件；恢复编辑时保留内容样式，将横向转为 4:3、竖向及方形转为 3:4，并显示原尺寸及转换提示。schema 仍为 v5，无新增迁移或依赖。
- PNG 导出与保存共用一个命令；导出总是追加版本，已保存的历史 PNG 不覆盖。系统字体和源背景属于外部依赖，跨机器字体/源图变化可能影响重新渲染，原 PNG 文件仍保持不变。
- 无新增 npm/Rust 依赖；三个封面业务命令同时登记 build.rs、lib.rs、permissions/covers.toml 和主窗口 capability。数据库备份仍由原有统一实现处理。

## M3 可选 AI 背景（2026-10-24）

- `database/ai.rs` 的模型配置增加 text/image 用途，默认模型按用途独立。继续使用现有系统凭据实现，不向前端提供密钥读取接口；更换 Base URL 必须重新输入已有密钥。
- `ai/images.rs` 实现唯一图片 Provider：请求 `/images/generations`，使用用户填写的模型 ID、`n=1`、`size=1024x1024`、`response_format=b64_json`，仅接受一张 base64 静态 PNG、JPEG 或 WebP；不下载响应 URL。沿用统一 HTTP 错误脱敏、禁止重定向和自动重试，图片超时为 180 秒。
- 图片响应写入素材前使用 `image 0.25.10`（仅启用 jpeg/png/webp）完整解码并限制 15 MB、4000 万像素和 160 MB 解码分配；拒绝动画内容，按实际格式保存 `.png`、`.jpg` 或 `.webp`。
- 将已解析的书名、作者和画面描述放入版本化 Prompt；只生成背景，中文文字继续使用原 Canvas 渲染器。`base64 0.23.1` 与 `png 0.18.1` 原已存在于锁文件；新增 `image 0.25.10`（仅 jpeg/png/webp）用于全量像素解码，静态格式和 15 MB/4000 万像素限制校验后才允许保存。
- `start_cover_generation`/`execute_cover_generation` 复用 ai_tasks、共享任务注册/取消和中断恢复。`database/image_generation.rs` 将新图片文件、素材引用和任务成功状态组成事务，失败清理本次新文件，旧背景及历史不改写。
- 封面页默认使用本地制作。主动展开 AI 区域才读取图片模型，生成成功后先读取和解码素材，再替换草稿中的背景；不自动保存封面版本。失败、取消与迟到响应保留当前草稿、背景和历史；生成期间阻止离页，现有双比例导出及部分成功提示继续复用。
- 数据库增量迁移为 v6，详见 DATABASE。Windows 真实 WebView2、IPC、系统凭据、SQLite 和文件验证通过；本机可控响应验证不代表外部图片服务可用性，完整边界见 M3-VERIFICATION。

## M4 半自动发布准备（2026-10-24）

- 页面为 `/projects/:id/publishing`，书籍、文案和封面页均可进入。沿用现有页面样式、操作状态、素材组件与 IPC 服务边界，无新增依赖。新任务使用最新已保存文案及可用封面作为初值，用户可编辑文字、选填标签并选择本书的视频与两张封面。
- 调用链为 PublishingPage → services/publishing.ts → publish_commands.rs → database/publishing.rs。当前只有本地发布准备实现，不为尚未接入的平台建立空接口或转发层。平台只提供固定抖音创作者中心入口，不调用上传、账号或发布接口。
- 任务保存发布文字快照与素材外键。输入验证归 Rust 业务服务，UI 只维护编辑草稿；生成时先保存，失败保留草稿和历史，路由/关闭保护未保存编辑。修改标题或封面选择会撤销对应 UI 确认。
- 发布包写入项目 `publish/{taskId}-{随机ID}` 唯一新目录，包含 `title.txt`、`description.txt`、`tags.txt`、`video-path.txt`、两张 PNG、`manifest.json` 和 `README.txt`。视频不复制。封面读取上限 15 MB，完整 PNG 解码并校验两种尺寸；写完后再次检查源文件快照，最后事务提交 ready。
- 写入或事务失败时仅清理本次创建的目录；旧任务包及源文件不删除。强制结束进程可能留下未登记的新目录，但不影响原素材。再次生成始终使用新目录，不覆盖历史。
- 清单随任务查询重新计算，视频/封面缺失、文件引用变化或任务包不完整会撤销 ready。检查使用文件路径、大小、毫秒修改时间及文字内容；不对大型视频进行全量哈希。平台上传后仍必须由用户明确确认发布成功，发布记录与任务/项目状态事务提交。
- 打开视频复用既有已登记素材与扩展名校验；目录与封面通过任务 ID 定位。打开网站仅使用固定 HTTPS 地址，不向 WebView 授予任意 Shell、SQL 或文件写入权限。五个命令在 build.rs、lib.rs、权限文件与 capability 同步注册。
- 官方实现核对：现有 `rusqlite 0.40.2` transaction 源码与文档、Tauri `invoke` 和权限文档，以及现有 `image 0.25.10` 解码限制实现。复用当前锁定版本，不升级依赖。

## M5 验证与修正（2026-10-24）

- `package_valid` 现在对两张封面副本与源 PNG 做有上限的逐字节比较，修复仅检查大小导致同大小损坏文件仍为 ready 的问题。复用已有 15 MB 文件读取限制，不新增哈希依赖、数据库字段或另一套校验状态；大型视频仍按原路径、大小和修改时间检查。
- 新增同大小损坏封面回归、v2–v7 备份恢复与恢复前安全备份回退矩阵、三项目文案/封面/发布任务/模型凭据在备份恢复和重启后的完整保留回归。
- Windows 通过独立 identifier、安装目录与数据库运行安装版，使用真实 WebView2、Tauri IPC、系统凭据、SQLite 和文件；三本书调用既有配置的外部文本服务，异常场景使用本机可控契约服务。截图、试用包和脚本保留于忽略目录 `output/playwright/m5/`。详细命令与结果见 `M5-VERIFICATION.md`。
