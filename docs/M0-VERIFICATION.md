# M0 工程初始化与技术验证记录

日期：2026-10-23。范围仅 M0。本报告区分单元测试、编译结果与真实桌面行为。

## 初始环境和方案

初始目录不是 Git 仓库，仅有 AGENTS.md、5 份设计文档及原始 zip，没有源码或依赖。本轮初始化 main，未提交代码。终检时初始 zip 已不在根目录；本轮没有删除或移动它，未据此恢复或覆盖工作区。

复用 Node.js 22.21.1、npm 10.9.4、Git 2.45.2、WebView2 149.0.4022.80。虽已存在 pnpm 9.12.1、Bun 1.4.2、Yarn 1.22.22，工程统一使用 npm。

原环境没有 Rust、MSVC 和可用 Windows SDK。本轮将 Rust/Cargo 1.99.1 安装到 `D:\MyWork\software\rust`，C++ Build Tools 安装到 `D:\MyWork\software\VisualStudioBuildTools`；Windows Installer 和 SDK 使用 C 盘系统共享路径。npm 缓存复用 `D:\DevCache\npm-cache`。Rust 仅通过本地 `.tools/env.ps1` 配置当前终端，不改全局 PATH。

采用单一路径：Vue 技术页 → typed IPC service → Tauri 命令 → Rust rusqlite。无任意 SQL IPC、无文件写入权限、无业务表。一个 hash 路由、原生 HTML 控件；当前不需要 Pinia 或 UI 库。

## 数据库变更

- 新增 `0001_m0_probe.sql` 和版本 1。
- 新数据库仅 `m0_probe`，最多一条记录。
- 写入后回读；中文及 SQL 特殊字符使用绑定参数。
- 迁移失败事务回滚；不重置数据库；较高版本拒绝打开。
- 文件选择仅显示路径，不持久化为素材，也不复制、删除视频。

## 已执行命令与结果

以下命令除说明外均在项目根目录执行，Rust 命令前先 `. .\.tools\env.ps1`。

| 命令 | 实际结果 |
|---|---|
| `git status --short` / `git rev-parse --show-toplevel`（初始化前） | 失败：不存在 Git 仓库；随后 `git init -b main` 成功 |
| `node --version` / `npm --version` / 包管理器和 Git 版本查询 | 通过，版本见上 |
| `Get-Command rustup,rustc,cargo,cl,link,tauri`（初始化前分别检查） | 未找到；进一步核实安装目录/注册表后补齐环境 |
| `Get-CimInstance Win32_OperatingSystem` | 沙箱拒绝访问，改用 Environment.OSVersion，得到 10.0.22631 |
| `npm view ...`（沙箱） | EACCES；通过网络权限升级访问官方 registry 后成功 |
| `rustup-init.exe ... --no-modify-path` | 首次官方连接被重置；显式使用系统现有代理后 `rustup toolchain install stable --profile minimal --component rustfmt,clippy` 成功 |
| `cargo fetch --manifest-path src-tauri/Cargo.toml` | 通过，生成 Cargo.lock |
| `npm run tauri -- info`（安装完成后） | Environment 全部通过，识别 MSVC / SDK / WebView2 / Rust |
| `npm run test:rust` | 通过，5 个 Rust 测试；首次真实 MSVC 编译 2m30s |
| `npm run check:rust` | 通过，cargo fmt --check 与 cargo clippy --locked --all-targets -- -D warnings |
| `npm install` | 最终组合安装成功；已清除 happy-dom 和不兼容间接依赖 |
| `npm ci --no-audit --no-fund` | 通过，262 包，无 Node 引擎冲突；仅 glob 弃用提示 |
| 微软签名校验 + `vs_BuildTools.exe --quiet --wait --norestart --nocache --installPath D:\MyWork\software\VisualStudioBuildTools --add Microsoft.VisualStudio.Component.VC.Tools.x86.x64 --add Microsoft.VisualStudio.Component.Windows11SDK.26100` | 安装器退出码 0；MSVC 14.44.35207 / SDK 10.0.26100.0 |
| `npm run tauri -- icon assets/app-icon.svg` | 成功生成图标，只保留 Windows 配置引用的 4 个文件；系统字体 mstmc.ttf 解析警告不影响无文字的 SVG |
| `npm run lint` | 通过，0 警告；初次格式警告已自动修正 |
| `npm run build` | 通过，包括 `vue-tsc --noEmit`；34 模块；JS 约 94.46 kB，gzip 36.85 kB |
| `npm test` | 通过，1 文件 / 5 测试 |
| `npm audit --audit-level=moderate --json` | 全依赖 0 漏洞 |
| `npm run dev` + `Invoke-WebRequest http://127.0.0.1:1420/` | 服务启动、HTTP 200，包含前端入口 |
| `npm run desktop:dev` | 通过，真实 Tauri 窗口启动，读取数据库就绪；后续通过 Ctrl+C 主动停止，停止码不作为启动失败 |
| `npm run desktop:build`（首次） | release 编译成功；NSIS 解压后 `fs::rename` 在用户缓存目录报 os error 17，安装包未完成 |
| `npm run desktop:build`（改用项目缓存后） | 通过，生成 1 个 x64 NSIS 安装包，2,715,202 字节 |
| `Start-Process <安装包> -ArgumentList '/S','/D=D:\MyWork\software\shuyin-ai' -WindowStyle Hidden -PassThru -Wait` | 实际安装成功，退出码 0，得到 shuyin-ai.exe 和 uninstall.exe |
| `Start-Process D:\MyWork\software\shuyin-ai\shuyin-ai.exe -PassThru` | 实际启动；独立安装目录进程及窗口可访问；页面使用 http://tauri.localhost/#/verification |

NSIS 错误定位依据：读取与安装版本一致的 `tauri-cli-v2.12.4` 源码，`get_and_extract_nsis` 解压后执行用户工具目录中 `nsis-3.13` → `NSIS` 的 `fs::rename`。启用官方 `bundle.useLocalToolsDir` 后在项目 `target/.tauri/` 下载、校验、解压和打包均通过，未修改打包器源码或增加重试分支。未进一步断言用户缓存目录底层卷映射的成因。

安装包：`src-tauri/target/release/bundle/nsis/书引 AI_0.1.0_x64-setup.exe`。
SHA-256：`1B96F01D5606CA929820F62A1053BB706E898880E50DBF2934BA6258C01EBB75`。

原始日志在忽略的 `output/verification/`；安装包信息另存 `installer.json`。

## 真实桌面验收证据

| M0 验收项 | 实际证据 | 结论 |
|---|---|---|
| 开发环境可启动 | Vite HTTP 200；Tauri 开发窗口显示完整验证页面，SQLite 初始化就绪 | 通过 |
| 安装包可构建并启动 | NSIS 生成；静默安装退出 0；安装目录中的 exe 有独立进程和可访问窗口，生产页面来源为 tauri.localhost | 通过 |
| SQLite 重启仍存在 | 开发版界面写入 `M0 持久化验收 2026-10-23：中文与空格，关闭重开仍在。`，回读时间 `2026-10-23T03:37:15.828Z`；停止开发进程后启动安装版，自动读回相同内容与时间，未再次写入 | 通过 |
| 可选择本地视频 | 原生对话框实际打开；交互式验收中返回 `D:\MyWork\project\测试文件\20240826_184455.mp4`，界面显示「已取得本地视频路径」；只读文件检查确认文件存在、8,333,567 字节 | 通过 |
| 无未解决构建错误 | lint / 完整类型 / 前端测试 / Rust 测试 / Clippy / 前后端构建通过；NSIS 初次缓存错误已复测解决 | 通过 |

界面可访问性证据保存在 `desktop-before-restart.txt` 与 `installed-after-restart.txt`。桌面控制曾遇到 cursor manager、窗口最小化、用户输入与遮挡；部分文件选择由交互协作完成，不将失败的自动化调用算作通过。结论依据后续实际读取到的应用结果。未保存其他应用截图作为项目证据。

为验证准备了独立视频样本：通过 ffmpeg-skill 的 `background.py` 生成 `output/verification/本地 视频.mp4`，1 秒、320×180、25 fps、H.264、无音轨；`look.py --at 0` 的画面已查看。最终交互使用的是上表中的已有 MP4，不能用样本的哈希比较代替原视频的前后对比。

## 测试边界与待验收

- 前端 mock 测试只证明交互状态，不证明 Tauri IPC、原生对话框或 SQLite 运行。
- Rust 数据库测试通过：中文与 SQL 特殊字符保存、关闭连接重开、重复迁移、单条更新、长度拒绝及 SQLite query_only 写失败保留旧内容、冲突迁移保留数据/版本、更高版本拒绝、不可用路径错误。
- 开发版原生文件选择通过；“取消后保留路径”已有前端回归测试，尚未单独完成安装版原生文件选择及取消回归。文件选择本身不检查媒体内容或读取元数据，这属于后续素材管理。
- Windows 10、其他机器、升级、代码签名、无 WebView2 环境下的下载尚未验证，属于后续发布验收。
- Vue Test Utils 2.4.6 的间接依赖 glob 10.5.0 有弃用提示；当前完整 npm audit 为 0 漏洞。

## 修改文件清单

新增工程配置：`.gitattributes`、`.gitignore`、`.npmrc`、`package.json`、`package-lock.json`、`index.html`、`tsconfig.json`、`vite.config.ts`、`eslint.config.js`、`README.md`。

新增前端：`src/main.ts`、`src/App.vue`、`src/style.css`、`src/app/router/index.ts`、`src/modules/verification/VerificationPage.vue`、`src/services/verification.ts`、`tests/verification.test.ts`。

新增桌面和数据库：`src-tauri/Cargo.toml`、`src-tauri/Cargo.lock`、`src-tauri/build.rs`、`src-tauri/tauri.conf.json`、`src-tauri/src/main.rs`、`src-tauri/src/lib.rs`、`src-tauri/src/database.rs`、`src-tauri/migrations/0001_m0_probe.sql`、`src-tauri/capabilities/main.json`、`src-tauri/permissions/probe.toml`。

新增图标：`assets/app-icon.svg`、`src-tauri/icons/32x32.png`、`128x128.png`、`128x128@2x.png`、`icon.ico`。

新增记录：`task_plan.md`、`findings.md`、`progress.md`、`docs/M0-VERIFICATION.md`。

更新原文档：`docs/TECHNICAL-DESIGN.md`、`docs/DATABASE.md`、`docs/DEVELOPMENT-PLAN.md`。AGENTS.md、PRD.md、AI-RULES.md 未修改。

本地生成且不入 Git：`.tools/`、`output/`、`node_modules/`、`dist/`、`src-tauri/target/`、Tauri 生成的 schema/permissions。全新 Git 仓库中原有设计文件也显示为 untracked，不代表全部由本轮新增。

## 下一步

保持本轮止于 M0。进入 M1 前单独确认下一里程碑范围；从数据库版本 2 增量建立业务表，保留现有数据。发布到其他机器前补充 Windows 10/11、升级、代码签名、无 WebView2 环境和原生取消操作验收。

## 版本和源码依据

依赖版本以当日官方 npm registry / crates.io 返回数据、安装包 package.json / Cargo.toml 与锁文件为准；实际阅读了 Tauri invoke/isTauri、Dialog JS/Rust open、Tauri build AppManifest、rusqlite transaction、Vue Router hash history、Vitest 配置和 jsdom API 实现。

- https://v2.tauri.app/start/prerequisites/
- https://v2.tauri.app/plugin/dialog/
- https://v2.tauri.app/distribute/windows-installer/
- https://vite.dev/guide/
- https://learn.microsoft.com/en-us/visualstudio/install/use-command-line-parameters-to-install-visual-studio

TypeScript 采用 6.0.3：typescript-eslint 8.72.0 的兼容范围为 <6.1，故不采用 registry 最新的 7.0.2。Vue Test Utils 2.5.x 的间接依赖要求 Node 22.22.2，采用 2.4.6 复用当前 Node；happy-dom 21 的类型声明不兼容 Node 22，最终替换为 jsdom 27.4.0，未弱化 TypeScript 检查。
