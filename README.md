# 书引 AI

书引 AI 为每本书一次准备**发布标题、发布内容、音频制作指导、封面主标题、副标题，以及横版 4:3 / 竖版 3:4 图片提示词**。填写书名和可选资料即可生成、编辑并复制，封面素材可直接带入制作页面。音频默认采用“甲子 · 十分钟读懂一本书”的8—10分钟节目模板，按书籍内容设计不同的五段指导；应用不直接生成音频，可打开外部音频工具并回填下载后的音频成品。

从安装到备份、封面和发布准备的操作步骤见 [使用手册](docs/USER-GUIDE.md)；三本书试用、异常保护、Windows 安装升级与测试结果见 [M5 验证报告](docs/M5-VERIFICATION.md)。

M5 本机交付已完成正式安装 0.1.0→0.5.1 升级及 ZIP 启动验证，旧数据和回退副本保留。本地 EXE、ZIP、使用手册和 SHA-256 清单位于 `output/delivery/m5-windows-0.5.1/`，属于尚未发布的本地验收构建。

书籍页的「制作封面」一次导出横版 4:3（1440×1080）、竖版 3:4（1080×1440）和横版 16:9（1920×1080）三张 PNG，支持本地模板、背景图拖动/缩放、三比例并排预览、个人样式和历史版本，无需配置图片 API。发布准备选用前两种比例，并显示实际封面缩略图和版本。

可选 AI 背景：在模型设置新增「图片」用途的模型，填写兼容 GPT Image 的服务根地址、模型 ID 和密钥，再到封面页展开「使用 AI 生成背景」。服务需通过 `/images/generations` 返回一张 base64 静态 PNG、JPEG 或 WebP。生成只替换背景，确认后点击「导出 3 张封面」；失败或取消保留草稿。历史模型服务验证见 `docs/M3-VERIFICATION.md`。

文案可按项填写要求、预览后采用；编辑文字和封面配置会自动暂存，重启恢复，点击保存才新增正式历史。资料支持 UTF-8 TXT/Markdown 预览追加并保留来源。发布时明确选择文案历史或手填，任务保存版本来源及文字快照。完整备份目录包含数据库、登记素材、封面与当前任务包，视频可选；恢复会在新位置重建路径并保留旧文件。实现和 Windows 隔离验收见 [创作体验验收](docs/CREATOR-WORKFLOW-VERIFICATION.md)。

## 环境

- Node.js 22.21.1 + npm 10.9.4（本机复用）；依赖精确版本及完整解析见 package.json / package-lock.json。
- Rust MSVC stable，本次安装并使用 1.99.1；Rust crate 锁定见 src-tauri/Cargo.lock。
- Visual Studio C++ Build Tools、Windows SDK、WebView2 Runtime。
- 主工具安装在 D 盘；微软安装器、SDK 共享组件及应用的用户数据仍使用 Windows 标准目录。

本机 Rust 的 `CARGO_HOME`、`RUSTUP_HOME` 和 Cargo `bin` 路径已保存到当前用户环境变量。修改前已打开的终端仍使用旧环境；请完整退出并重开终端，IDE 集成终端需重开 IDE。

若当前 PowerShell 运行 `desktop:dev` 报 `cargo metadata ... program not found`，先载入安装时生成的本地环境（不入 Git），再验证并启动：

```powershell
. .\.tools\env.ps1
cargo --version
npm run desktop:dev
```

该脚本只更新当前会话；无需重新安装已有的 Rust 或修改 npm 启动脚本。

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

## Codex 串行任务

编辑根目录 `codex-tasks.mjs`，填写每项任务的 `prompt` 并将 `enabled` 改为 `true`。运行 `npm run codex:queue -- --dry-run` 预览，再运行 `npm run codex:queue`。脚本按清单顺序启动独立 Codex CLI 会话，共用项目文件；每项完成后才进入下一项，失败或受阻即停，再次运行跳过已完成任务。验收入口为 `output/codex-queue/SUMMARY.md`。完整用法、续跑及权限说明见 [任务队列说明](docs/CODEX-TASK-QUEUE.md)。

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

## M0 历史手工验收

1. 启动桌面应用，进入「工程验证」，确认显示「Tauri 桌面环境」，数据库路径和当前状态可见。
2. 输入中文测试文字，点击「写入并回读」，核对已保存内容。
3. 完全关闭应用，再次启动，核对文字与原时间仍在。
4. 点击「选择本地视频」，选择包含中文或空格路径的视频，核对路径。
5. 再次选择并取消，确认原路径保留，原视频未被修改或复制。
6. 构建安装包后实际安装并启动，再核对上面的行为。

页面显示操作结果，不会把一次写入成功自动宣称为“重启验证完成”。文件路径仅用于当前页面，不持久化为素材记录。

数据库位于 `%LOCALAPPDATA%\com.shuyin.desktop\shuyin.sqlite3`。`m0_probe` 仅有一条技术记录；迁移在事务内执行，通过 `PRAGMA user_version` 记录版本，绝不通过删库恢复。

实际完成情况、失败记录和未验证范围见 [M0 验证报告](docs/M0-VERIFICATION.md)。规范入口为 [AGENTS.md](AGENTS.md)。

## M1 历史使用流程

以下步骤对应当时的工作台界面；当前使用方法见后面的“使用流程”。

1. 在「目录与备份」选择默认工作目录（可选 D 盘），仅影响以后创建的项目。
2. 在「书籍项目」新建，保存书名、简介、笔记、创作方向；进入项目详情后关联素材。
3. 移动源文件后点击「刷新文件状态」，在原关联上「重新定位」。移除关联、删除项目均不删除源文件。
4. 在项目状态选择「已归档」后保存，可通过筛选找回并修改为活动状态。
5. 定期创建数据库备份；恢复前会显示确认并自动备份当前数据。媒体文件需要单独备份。

M1 的实现边界与验证结果见 `docs/M1-VERIFICATION.md`。

## 使用流程

1. 在「模型设置」填写可信服务的 API 地址、模型名与 API Key，保存后测试连接。调用可能产生服务费用。
2. 添加书名，选填作者和书籍内容/创作重点，保存并继续。
3. 点击「一键生成」，得到一个标题、一段发布内容、一份音频制作指导和封面素材。
4. 在「音频制作」复制指导并打开外部音频工具，选择对应原书来源；下载音频后导入本项目，再在剪映生成字幕、添加视频背景并导出最终成片。
5. 历史版本折叠显示；失败、取消及生成中编辑均保留原有内容。文案/封面自动暂存，离页及正常关闭等待写完，重启可继续；切换历史仍提示放弃修改。

制作封面：进入「制作封面」，选择模板或「我的封面样式」，填写本书文字，可选背景图。拖动画布或调整背景滑块检查裁切，在三个比例并排预览后点击「导出 3 张封面」。PNG 写入本书 `covers`，每张追加版本；失败保留已有成功路径和草稿。应用个人样式保留本书文字、背景与裁切；原图失效可重新定位。完整备份包含登记背景及导出文件，单独数据库备份仅含配置与路径。

M3 本地制作曾完成独立 Windows 安装、两次升级、原生选图/取消和重启持久化验证；旧数据、历史PNG和源图片保留。新增 AI 背景已完成隔离 Windows 开发版及新版安装升级、重启和 v6 备份恢复验证。外部图片服务已完成 UGQ.ai 实测；正式安装升级与 macOS/Linux 实机仍需验收。

发布准备：在书籍、文案或封面页进入「发布准备」。新任务文案为空，明确选择历史版本或手填；选择制作音频、剪映导出的最终成片和两张封面，核对实际缩略图与版本，勾选确认后生成完整任务包。最终成片需为 MP4/MOV，发布检查会验证竖版 9:16 画面与音轨。任务包包含文字快照、来源版本、音频引用、两张封面、视频路径及说明，不复制音频或视频。自行上传并确认平台成功后记录结果；修改或素材失效后重新生成，已发布记录保留。

密钥仅存在系统凭据库，不随备份。数据与备份入口提供完整备份和单独数据库备份；完整备份是目录，须整体保管，视频默认不包含，未登记文件需另存。恢复前预览确认并自动备份当前数据库，完整恢复在新目录重建文件引用。数据库自动迁移到 v10，保留旧项目、素材引用、模型凭据引用及历史，支持 v2–v10 数据库备份。旧版待发布任务需补选音频并重新生成任务包。

当前需求以 [PRD](docs/PRD.md) 为准，本轮创作体验证据见 [验收报告](docs/CREATOR-WORKFLOW-VERIFICATION.md)，音频制作与成片回填证据见 [音频验收报告](docs/AUDIO-WORKFLOW-VERIFICATION.md)。此前 M5 以《论语》《西游记》《朝花夕拾》和验收视频完成流程试用，历史结果见 [M5 验证报告](docs/M5-VERIFICATION.md) 和 [三项生成验收](docs/PUBLISHING-VERIFICATION.md)。真实作品的视频制作、平台上传预览和最终发布由用户完成。
