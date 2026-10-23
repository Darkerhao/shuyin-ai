# 书引 AI 技术设计文档

当前产品范围以 PRD 的三项生成为准：发布标题、发布内容、音频生成提示词。本文前面的完整工作台设计保留为历史记录；当前实现见文末“三项生成”说明和 `PUBLISHING-VERIFICATION.md`。

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
