# 书引 AI

Windows 本地读书创作工作台。当前仅实现 **M0 工程初始化与技术验证**，不包含书籍项目、AI 创作、素材管理或发布业务。

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
Windows NSIS 安装包输出到 `src-tauri/target/release/bundle/nsis/`。本轮只配置 NSIS，不生成 MSI。

## M0 手工验收

1. 启动桌面应用，确认显示「Tauri 桌面环境」，数据库路径和当前状态可见。
2. 输入中文测试文字，点击「写入并回读」，核对已保存内容。
3. 完全关闭应用，再次启动，核对文字与原时间仍在。
4. 点击「选择本地视频」，选择包含中文或空格路径的视频，核对路径。
5. 再次选择并取消，确认原路径保留，原视频未被修改或复制。
6. 构建安装包后实际安装并启动，再核对上面的行为。

页面显示操作结果，不会把一次写入成功自动宣称为“重启验证完成”。文件路径仅用于当前页面，不持久化为素材记录。

数据库位于 `%LOCALAPPDATA%\com.shuyin.desktop\shuyin.sqlite3`。`m0_probe` 仅有一条技术记录；迁移在事务内执行，通过 `PRAGMA user_version` 记录版本，绝不通过删库恢复。

实际完成情况、失败记录和未验证范围见 [M0 验证报告](docs/M0-VERIFICATION.md)。规范入口为 [AGENTS.md](AGENTS.md)。
