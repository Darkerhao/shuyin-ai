# 书引 AI

书引 AI 为每本书准备三项内容：**发布标题、发布内容、音频生成提示词**。填写书名即可一键生成、编辑并复制。音频默认采用“甲子 · 十分钟读懂一本书”的8—10分钟节目模板，按书籍内容设计不同的五段指导，不在应用中生成音频文件。

从安装到备份、封面和发布准备的操作步骤见 [使用手册](docs/USER-GUIDE.md)；三本书试用、异常保护、Windows 安装升级与测试结果见 [M5 验证报告](docs/M5-VERIFICATION.md)。

M5 本机交付已完成正式安装 0.1.0→0.5.1 升级及 ZIP 启动验证，旧数据和回退副本保留。本地 EXE、ZIP、使用手册和 SHA-256 清单位于 `output/delivery/m5-windows-0.5.1/`，属于尚未发布的本地验收构建。

书籍页的「制作封面」用于准备发布作品时上传的图片封面，一次导出横版 4:3（1440×1080）和竖版 3:4（1080×1440）两张 PNG，支持本地模板、背景图、中文排版和历史版本，无需配置图片 API。

可选 AI 背景：在模型设置新增「图片」用途的模型，填写兼容 GPT Image 的服务根地址、模型 ID 和密钥，再到封面页展开「使用 AI 生成背景」。服务需通过 `/images/generations` 返回一张 base64 静态 PNG、JPEG 或 WebP。生成只替换背景，确认后点击「导出两张封面」；失败或取消保留草稿。Windows 本机契约验证、UGQ.ai 外部模型实测、凭据重启、导出与异常保护结果见 `docs/M3-VERIFICATION.md` 文末。

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
3. 点击「一键生成」，得到一个标题、一段发布内容和一份音频提示词。
4. 可以直接修改，点击「保存修改」。分别复制到发布页面和音频工具；音频工具需要选择对应原书来源。
5. 历史版本折叠显示；失败、取消及生成中编辑均保留原有内容，未保存修改离开时提示确认。

制作封面：进入「制作封面」，选择纸页/深林模板并填写文字，可选本地背景图；通过「预览比例」检查横版 4:3 和竖版 3:4，点击「导出两张封面」。两张 PNG 分别排版，保存在当前书籍工作目录的 `covers` 子目录，文件名包含像素尺寸；页面显示两条完整路径，可打开项目目录后在发布页面分别上传。每张导出图追加一个版本，可通过「封面历史」恢复排版；旧尺寸历史继续编辑时转为对应的横/竖比例，原版本与原文件保留。第二张写入失败时保留第一张并显示路径，页面草稿不清空。背景文件失效时重新选择或改用纯色。数据库备份包含配置和路径，背景及导出文件需另行备份。完整验收见 `docs/M3-VERIFICATION.md`。

M3 本地制作曾完成独立 Windows 安装、两次升级、原生选图/取消和重启持久化验证；旧数据、历史PNG和源图片保留。新增 AI 背景已完成隔离 Windows 开发版及新版安装升级、重启和 v6 备份恢复验证。外部图片服务已完成 UGQ.ai 实测；正式安装升级与 macOS/Linux 实机仍需验收。

发布准备：在书籍、文案或封面页进入「发布准备」。展开素材管理关联本地视频，选择标题、内容、可选标签和横/竖两张已导出封面，勾选标题与封面确认后点击「生成完整任务包」。任务包位于项目 `publish` 子目录，包含发布文字、两张封面、视频原路径及操作说明，不复制视频。可分别复制文案、打开任务包及固定抖音创作者中心；自行上传并确认发布成功后，在应用中记录时间、可选作品链接和备注。保存修改或素材失效后须重新生成；已发布记录保留，另一作品另建任务。验证见 `docs/M4-VERIFICATION.md`。

密钥仅存在系统凭据库，不随数据库备份。备份/目录辅助入口位于模型设置页。数据库自动迁移到 v7，保留旧项目、素材引用、模型凭据引用及创作/封面历史；支持恢复 v2–v7 备份，恢复前先备份当前数据。数据库备份不包含视频、封面 PNG 和任务包文件，需独立保管文件目录。

当前需求以 [PRD](docs/PRD.md) 为准；本轮按用户指令执行 M5，以《论语》《西游记》《朝花夕拾》和验收视频完成流程试用。真实作品的视频制作、平台上传预览和最终发布由用户完成。实现、测试与真实模型证据见 [M5 验证报告](docs/M5-VERIFICATION.md)、[三项生成验收](docs/PUBLISHING-VERIFICATION.md) 及各里程碑历史报告。
