# 书引 AI

本地读书创作工作台，提供 Windows、macOS 和 Linux 打包。已实现 **M1 项目与素材管理**及 **M2 文本创作基础闭环**：模型配置、系统凭据、连接测试、大纲/讲稿/发布文案生成、编辑与历史版本。M2 已通过自动化、Windows 安装版真实 HTTPS 服务生成及隔离安装升级验收；生成质量使用公开原文基准验证，个人读书笔记试用和新增平台凭据仍待验收。封面与发布业务尚未实现。

## 环境

- Node.js 22.21.1 + npm 10.9.4（本机复用）；依赖精确版本及完整解析见 package.json / package-lock.json。
- Rust MSVC stable，本次安装并使用 1.99.1；Rust crate 锁定见 src-tauri/Cargo.lock。
- Visual Studio C++ Build Tools、Windows SDK、WebView2 Runtime。
- 主工具安装在 D 盘；微软安装器、SDK 共享组件及应用的用户数据仍使用 Windows 标准目录。

本机新终端先载入安装时生成的本地环境（不入 Git）：

```powershell
. .\.tools\env.ps1
```

其他机器应安装 Rust MSVC 工具链并确保 cargo 在 PATH。若也采用本机的 D 盘位置，可设置：

```powershell
$env:RUSTUP_HOME = 'D:\MyWork\software\rust\rustup'
$env:CARGO_HOME = 'D:\MyWork\software\rust\cargo'
$env:PATH = "$env:CARGO_HOME\bin;$env:PATH"
```

下载代理由各机器按实际网络配置；不要照搬本地 .tools/env.ps1 中的代理端口。

## 开发与检查

在项目根目录执行：

```powershell
npm ci
npm run dev             # 仅浏览器预览，原生操作禁用
npm run desktop:dev     # Tauri 桌面开发，同时启动 Vite
npm run lint
npm run typecheck
npm test
npm run build
npm run test:rust
npm run check:rust
npm run desktop:build
```

`dev` 与 `desktop:dev` 不要同时运行，二者使用同一个固定端口 1420。
`desktop:build` 根据当前系统自动选择平台配置：Windows 生成 NSIS，macOS 生成 APP/DMG，Linux 生成 DEB/RPM/AppImage。安装包在 `src-tauri/target/release/bundle/`，显式指定 Rust target 时在 `src-tauri/target/<target>/release/bundle/`。

macOS 需安装 Xcode Command Line Tools；Linux 构建需 WebKitGTK 4.1、GTK 与对应开发依赖，完整安装命令见工作流。Windows 本地环境沿用上面的配置。

## 自动打包与版本同步

代码推送到 `main` 后自动按约定递增一次版本（破坏性变更 major、`feat` minor、其他 patch），支持 GitHub PR 合并、本地 merge/cherry-pick 后推送以及直接推送 main，使用同一提交并行构建四个平台目标。全部检查与打包成功后，将版本提交 cherry-pick 到 `dev`，原子推送两个分支及版本标签，最后发布 GitHub Release。dev 推送、标签推送和未合并 PR 不打包；流水线生成的版本提交跳过，避免循环触发。

| 平台 | Release 下载文件 |
| --- | --- |
| Windows x64 | 安装 `.exe`、免安装 `.zip`（需要 WebView2，数据仍在系统用户目录） |
| macOS Apple Silicon | `.dmg`、`.zip` |
| macOS Intel | `.dmg`、`.zip` |
| Linux x64 | `.deb`、`.rpm`、`.AppImage`、含 AppImage 的 `.tar.gz` |

所有文件统一命名为 `shuyin-ai-<版本>-<平台>-<架构>.<格式>`，并提供 `SHA256SUMS.txt`。Actions Artifacts 同时保留 90 天；macOS 当前为 ad-hoc 签名，Windows 未配置签名证书，正式分发前仍需对应平台的签名和安装验收。

版本规则、仓库写权限、首次启用和失败恢复见 [发布流程](docs/RELEASE.md)。本地发布脚本回归可运行 `npm run test:release`，也包含在 `npm test` 中。

版本级别按实际业务影响填写 PR/提交说明：日常修复和内部整理用 patch，兼容的新能力用 minor，明确的不兼容改动才用 major；同批改动取最高级别，只递增一次，不按改动量或提交数量累加。

## M0 手工验收

1. 启动桌面应用，进入「工程验证」，确认显示「Tauri 桌面环境」，数据库路径和当前状态可见。
2. 输入中文测试文字，点击「写入并回读」，核对已保存内容。
3. 完全关闭应用，再次启动，核对文字与原时间仍在。
4. 点击「选择本地视频」，选择包含中文或空格路径的视频，核对路径。
5. 再次选择并取消，确认原路径保留，原视频未被修改或复制。
6. 构建安装包后实际安装并启动，再核对上面的行为。

页面显示操作结果，不会把一次写入成功自动宣称为“重启验证完成”。文件路径仅用于当前页面，不持久化为素材记录。

数据库位于 `%LOCALAPPDATA%\com.shuyin.desktop\shuyin.sqlite3`。`m0_probe` 仅有一条技术记录；迁移在事务内执行，通过 `PRAGMA user_version` 记录版本，绝不通过删库恢复。

实际完成情况、失败记录和未验证范围见 [M0 验证报告](docs/M0-VERIFICATION.md)。规范入口为 [AGENTS.md](AGENTS.md)。

## M1 使用流程

1. 在「目录与备份」选择默认工作目录（可选 D 盘），仅影响以后创建的项目。
2. 在「书籍项目」新建，保存书名、简介、笔记、创作方向；进入项目详情后关联素材。
3. 移动源文件后点击「刷新文件状态」，在原关联上「重新定位」。移除关联、删除项目均不删除源文件。
4. 在项目状态选择「已归档」后保存，可通过筛选找回并修改为活动状态。
5. 定期创建数据库备份；恢复前会显示确认并自动备份当前数据。媒体文件需要单独备份。

M1 的实现边界与验证结果见 `docs/M1-VERIFICATION.md`。

## M2 文本创作流程

1. 在「文本模型」填写服务名称、API 根地址（例如可信服务的 `/v1` 地址）和实际模型名；支持多个配置、启用/停用与一个默认模型。
2. 填写 API Key 并保存，再点击「测试连接」。该操作会调用模型，可能产生服务费用。本机免密服务可以留空密钥；其余情况按服务认证要求填写。
3. 密钥仅保存在 Windows 凭据管理器、macOS 钥匙串或 Linux Secret Service。Linux 需有可用且已解锁的 Secret Service 会话。数据库和备份只记录凭据引用，恢复到另一台机器后需重新输入密钥。
4. 保存项目简介或笔记，从项目详情进入「AI 创作」。选择大纲、讲稿、标题、描述或话题；可引用已保存的大纲/讲稿，候选数量为 1–20。
5. 生成结果通过 JSON 结构、类型、长度与数量校验后，自动追加为新版本。每个内容类型独立编号；人工编辑后「保存为新版本」，可标记人工确认并切换历史。
6. 请求超时、认证失败、异常输出或取消均保留已有版本及当前草稿。生成时继续编辑的草稿也会保留，新结果可从历史选择。离开未保存页面会提示确认；未保存草稿仍需主动保存，不能替代崩溃恢复。
7. 请求最多等待 120 秒，不自动重试、不跟随重定向。支持 Chat Completions 文本协议；只支持 Responses 或返回非文本结构的服务需要另行适配。

数据库自动从 v2 升级到 v3，保留项目和素材。可恢复 v2/v3 备份，恢复前自动备份当前数据；生成任务执行中需要先取消或等待完成。备份不包含密钥和源媒体。

实现、验证命令与尚未验收范围见 [M2 验证报告](docs/M2-VERIFICATION.md)。后续完整 PRD 中的单分段重生成、收藏与复制、Markdown 导出、封面文案等增强尚未实施。
