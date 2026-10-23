# M0 调研记录

- 根目录仅 AGENTS.md、docs/ 下 5 份设计文档和原始 zip；无源码、package.json、锁文件或 Git。
- 已完整读取 AGENTS.md、PRD、TECHNICAL-DESIGN、DATABASE、AI-RULES、DEVELOPMENT-PLAN。
- Node.js 22.21.1；npm 10.9.4；pnpm 9.12.1；Bun 1.4.2；Yarn 1.22.22；Git 2.45.2。
- npm 缓存已在 D:\DevCache\npm-cache，使用官方 registry。优先 npm，不新增包管理器。
- Rust / Cargo / rustup / cl / link / CMake / Tauri 未出现在 PATH，常用 Rust 和 Visual Studio 目录未发现安装。
- 注册表有 Edge WebView2 Runtime 149.0.4022.80；OSVersion 为 Windows 10.0.22631（Windows 11）。
- M0 要求基础路由布局、SQLite 持久化、本地视频选择、错误处理和 Windows 包；完整业务迁移在 M1。
- 参考官方 Tauri prerequisites、dialog、windows-installer 和 Vite guide；具体版本仍待 registry 与源码核验。
- registry 核实：Vite 8.3.5、Vue 3.5.44、Tauri JS/CLI 2.12.4、Rust crate 2.12.5、Dialog 2.8.3；Node 22.21.1 满足主工具引擎要求。
- TypeScript 最新 7.0.2 超出 typescript-eslint 8.72.0 的 <6.1 范围，因此采用 6.0.3；Vue Router 使用 4.6.4 的基础路由 API，不引入 5.x 扩展。
- Vue Test Utils 2.5.1 -> js-beautify 2 -> nopt 10 / abbrev 5 要求更高 Node 补丁；2.4.6 解决引擎冲突。
- 阅读已安装 Dialog open 实现及类型、Tauri invoke/isTauri、Router hash history 和 Vitest 配置；文件选择返回 string/null，取消保持原路径。
- Rust 1.99.1 + Cargo 1.99.1 已安装到 D:\MyWork\software\rust；不修改全局 PATH，当前会话配置在忽略的 .tools/env.ps1。
- 已启动经微软签名验证的 Build Tools 2022 安装，主目录 D:\MyWork\software\VisualStudioBuildTools；Installer 与 Windows SDK 有不可避免的 C 盘共享组件。
- 方案：Rust rusqlite bundled SQLite；单条 m0_probe 技术记录；事务迁移 user_version=1；只暴露 read_probe/save_probe 和 dialog:allow-open。
- 最终 DOM 测试环境采用 jsdom 27.4.0，清除 happy-dom 可选 peer；不弱化类型检查。
- MSVC 14.44.35207 / SDK 10.0.26100.0 安装成功，Tauri info Environment 全通过。
- NSIS 缓存 rename 在用户目录报 os error 17，项目 target/.tauri 路径实测成功；不得把源码中不存在的临时目录跨盘流程写成根因。
- release NSIS 2,715,202 字节，实际安装至 D:\MyWork\software\shuyin-ai；安装版页面来源 tauri.localhost，读回相同文字和 UTC 时间。
- 终检初始 zip 不存在，本轮未移动或删除；不要恢复或覆盖用户工作区。

## M1 调研与取舍（2026-10-23）
- 数据库顺序迁移，保留 M0；只创建实际使用的项目/素材/普通设置三表。
- 不需要额外 UI 库、全局状态、UUID 包；沿用原生控件和已有 SQLite。
- rusqlite 0.40.2 backup.rs 的 run_to_completion 对 Busy/Locked 可持续循环；本实现单次 step(-1)，忙时明确报错，避免无限重试。
- Rust canonicalize 在 Windows 返回扩展路径；Shell 打开前转换 DOS/UNC 格式，官方 Rust 与 Microsoft 文档均说明文件系统与 Shell 路径要求不同。
- 桌面启动需写隔离应用数据目录，被沙箱拒绝；自动升级审核两次分别超时/服务断连。浏览器工具也被审核服务故障阻塞，未绕过审批。
- 原生桌面操作待验收，自动化通过不等同于真实桌面验收。

## M1 桌面实测补充
- 实际 CDP 连接 WebView2 可验证 UI→IPC→SQLite，无 mock；测试库标识 com.shuyin.m1-verification，与正式库隔离。
- 原生对话框可显示，桌面输入自动化焦点不稳定，因此成功选择/取消仍不计通过。进程重启后数据库字段和素材关联一致的证据独立有效。


## M1 安装升级证据
- 生成 NSIS 脚本的卸载 key 使用产品名，进程检查使用 mainBinaryName；隔离测试同时更改 productName / identifier / mainBinaryName，避免覆盖正式产品。
- Tauri --config 支持临时测试版本0.1.0/0.1.1；本轮构建未改变 Cargo.toml 或正式 tauri.conf.json 版本。
- 安装验证不能只看构建成功：本轮实际 NSIS exit0、生产URL、安装路径、注册版本与升级前后数据库字段均核对。
- 自动审批授权与服务可用性独立：明确授权后安装通过，但原生输入被审核服务断连阻塞，不能记为安全拒绝或验收通过。


## M1 核心验收完成
- 权限环境更新后，原生输入/确认成功；中文空格路径视频落库并显示 available。
- 第二次原生选择后 Esc 取消，check-native-cancellation.cjs 比较全部素材字段一致，按钮恢复启用。
- 1秒样本不适合肉眼验收，生成120秒绿色样本后经安装版打开；GeePlayer标题匹配、画面正确、进度00:11→00:13，总长02:00。暂停请求因窗口已最小化未执行，未改变播放器后续状态。
- 本轮无业务代码/依赖/迁移变更，更新验收文档。全部 M1 核心验收通过，未提交/发布，M2尚未开始。
