# 书引 AI 技术设计文档

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
