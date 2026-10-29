# 跨平台自动打包、发布与版本同步

## 触发与交付

`.github/workflows/release.yml` 仅监听 `main` 的 push：GitHub PR 合并、本地 merge/cherry-pick 后推送，以及直接推送 main 都触发。push 到 dev、创建标签、关闭未合并 PR 不打包。流水线生成的 `chore(release): v...` 提交通过 job 条件跳过；脚本也检查已有标签，避免版本回写重复升版。

工作流只检出本仓库 main，不执行未合入的 PR head；触发依据是代码实际进入远端 main，不依赖 GitHub PR 关闭事件。2026-10-23 的首次配置通过本地 cherry-pick 进入 main，当时仅监听 `pull_request_target`，因此没有产生运行；此次修复改用 push。修复提交推送到 main 后才会生效，不会补触发过去的推送。

每次生成一次版本，四个 runner 使用同一个 Git 提交构建。所有平台成功后同步分支，再创建对应版本的 GitHub Release；完整产物上传到草稿后才公开，避免用户下载不齐全的版本。版本标签 `vX.Y.Z` 指向实际打包的版本提交。

| 平台 | Runner / Rust target | 分发格式 |
| --- | --- | --- |
| Windows x64 | `windows-2025` / `x86_64-pc-windows-msvc` | NSIS `.exe`、应用 `.zip` |
| macOS Apple Silicon | `macos-15` / `aarch64-apple-darwin` | `.dmg`、保留 APP 包结构的 `.zip` |
| macOS Intel | `macos-15-intel` / `x86_64-apple-darwin` | `.dmg`、保留 APP 包结构的 `.zip` |
| Linux x64 | `ubuntu-22.04` / `x86_64-unknown-linux-gnu` | `.deb`、`.rpm`、`.AppImage`、包含 AppImage 的 `.tar.gz` |

共 10 个常规分发文件，命名为 `shuyin-ai-<版本>-<平台>-<架构>.<格式>`；另增加 Windows EXE 签名、两种 macOS `.app.tar.gz` 更新包及签名、Linux AppImage 签名，共 16 个包/签名文件，加上 `latest.json` 和 `SHA256SUMS.txt`，每个 Release 共 18 个附件。同时在 Actions Artifacts 保存 90 天。Windows ZIP 包含程序和运行说明，需要系统已有 WebView2，数据保存在系统用户目录。ZIP 是解压运行分发，不提供应用内自更新，升级时下载新版 ZIP 解压替换程序；应用内安装更新以 Windows NSIS 安装版验收。Linux TAR.GZ 包含 AppImage；应用内更新使用 AppImage，DEB/RPM 安装由系统包管理器更新。

平台默认配置分别位于 `src-tauri/tauri.windows.conf.json`、`tauri.macos.conf.json`、`tauri.linux.conf.json`。Linux 产品名为 `Shuyin AI`，使 DEB/RPM 包名为 `shuyin-ai`；应用内界面和标识符保留。macOS ICNS 从已有 SVG 生成，当前配置 ad-hoc 签名（`signingIdentity: "-"`），未接入 Developer ID 与公证；Windows 也未配置签名证书。首次分发仍需完成真实系统上的安装与信任提示验收。

macOS CI 显式使用 `--bundles app`，由 Tauri 构建并签名 APP、生成 `.app.tar.gz` 更新包及签名，再运行 `node scripts/release-assets.mjs dmg <target>`。脚本通过 `ditto` 将 APP 保留元数据地复制到临时目录，加入 `/Applications` 快捷入口，用 `hdiutil create -srcfolder ... -format UDZO` 直接生成压缩 DMG，并执行 `hdiutil verify`；任一步失败均中止发布，临时目录始终清理。最终文件继续通过原 collect 流程按平台和版本命名。CI 不再调用 Tauri 的 DMG 脚本，不设置 Finder 背景或图标位置；本地默认 Tauri 构建配置仍生成 APP 和 DMG。

实现依据已核对锁定的 CLI 2.12.4 官方源码：[DMG 的挂载和三次卸载逻辑](https://github.com/tauri-apps/tauri/blob/tauri-cli-v2.12.4/crates/tauri-bundler/src/bundle/macos/dmg/bundle_dmg)、[CI 仅跳过 Finder 脚本](https://github.com/tauri-apps/tauri/blob/tauri-cli-v2.12.4/crates/tauri-bundler/src/bundle/macos/dmg/mod.rs)、[APP 目标仍生成更新归档](https://github.com/tauri-apps/tauri/blob/tauri-cli-v2.12.4/crates/tauri-bundler/src/bundle.rs)。

本项目使用 Tauri 官方 Updater。应用从 GitHub Release 的 `latest.json` 获取版本和更新内容，按 `windows-x86_64`、`darwin-aarch64`、`darwin-x86_64`、`linux-x86_64` 选择更新包，下载后使用固定公钥校验签名。Windows 使用 NSIS EXE；macOS 使用 Tauri 生成的 APP TAR.GZ，不能用手工 ZIP 替代；Linux 使用 AppImage。

## 更新签名与首次启用

仓库的 `src-tauri/tauri.conf.json` 只保存公钥。对应私钥首次生成在仓库外的 `D:\MyWork\software\shuyin-ai\keys\updater.key`，公钥副本为同目录 `updater.key.pub`。私钥未提交；应长期保管此密钥，后续版本必须沿用，与系统安装包代码签名证书是两回事。

在 GitHub 仓库 Settings → Secrets and variables → Actions 配置：

- `TAURI_SIGNING_PRIVATE_KEY`：私钥文件的完整内容，不是本机路径。
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`：私钥密码；本次生成的密钥密码为空，该项可留空。

缺少私钥时构建明确失败，远端版本不会递增。密钥不会注入前端、日志或发布附件。本地签名打包前，在当前 PowerShell 会话加载 `. ./.tools/env.ps1`，将 `$env:TAURI_SIGNING_PRIVATE_KEY` 设为上述私钥文件路径，再运行 `npm run desktop:build -- --target x86_64-pc-windows-msvc -- --locked`；有密码时同时设置密码环境变量。不要重新生成密钥替换已经发布客户端信任的公钥。

更新说明来自前一个正式版本标签之后的非合并提交标题，排除流水线版本提交并去掉提交类型前缀。各平台构建时将相同说明注入 `VITE_RELEASE_NOTES`，安装后即使没有新版本也可查看本版内容；发布脚本将同一说明写入 `latest.json` 和 GitHub Release。说明由 Vue 文本插值展示，不执行远端 HTML。开发预览或未设置该环境变量的本地构建会明确显示未附带说明。

首个包含此功能的签名版本公开后更新源才完整；旧版没有更新入口，需要先手动安装一次。其后可在「关于与更新」使用自动检查/后台下载，或关闭自动模式后手动检查、下载，再确认安装并重启。自动模式每次启动检查一次，不在创作时自动安装。网络失败和签名失败可重试，不显示为“已是最新”。当前 GitHub 更新清单尚未发布，真实跨版本安装须在首次配置 Secrets 并发布后验收。

## 版本规则

以 `package.json` 为当前版本来源，读取上一个版本标签之后尚未发布的 Git 提交说明（包括合并提交中保存的 PR 标题），按实际业务影响声明取最高级别，一批发布只递增一次：

| 内容 | 递增 | 示例 |
| --- | --- | --- |
| `feat!:`、`refactor(api)!:`，或独立行 `BREAKING CHANGE:` / `BREAKING-CHANGE:` | major | `0.1.2 → 1.0.0` |
| `feat:`、`feat(scope):` | minor | `0.1.2 → 0.2.0` |
| `fix:`、`docs:`、`chore:` 及其他说明 | patch | `0.1.2 → 0.1.3` |

判断尺度如下：

- **日常修订（patch）**：修复保存失败、优化查询性能、调整样式、重构内部代码、维护文档或流水线。即使改了很多文件或包含 100 个修复提交，`1.4.8` 也只升到 `1.4.9`。
- **功能迭代（minor）**：新增封面导出、项目搜索、素材管理等用户可用能力，并兼容已有数据与使用方式。一个或多个新功能在同一批发布中，`1.4.8` 都只升到 `1.5.0`。
- **不兼容升级（major）**：移除用户正在使用的能力，或改变旧资料格式、配置协议等，导致用户必须迁移或改变使用方式。即使只改一个接口，也应从 `1.4.8` 升到 `2.0.0`；提交说明必须解释影响及迁移方案。

大重构、依赖主版本升级、兼容数据库迁移，以及开发时间长、修改行数多，都不是自动升 major 的依据。当前 `0.x` 阶段也不因累积若干功能就自动宣布 `1.0.0`。

PR 标题和提交说明应基于实际改动选用上述格式，尤其是 rebase 合并时应在提交说明中保留变更级别。流水线解析提交标题、GitHub 合并提交中保存的 PR 标题和非空的破坏性变更声明；正文中普通的 `feat:` / `feat!:` 示例、计划以及 Markdown 围栏代码示例不参与升版。自动判断依据这些经过代码审查的影响声明，不声称能仅靠源码差异识别业务兼容性。

第一个版本无需预先创建标签；之后使用当前版本对应的标签划定已发布范围。不要手动改版本或占用自动版本标签。

同步更新五个文件中的六个版本字段，保留原格式和依赖解析：

- `package.json`
- `package-lock.json`（根版本及根包版本）
- `src-tauri/tauri.conf.json`
- `src-tauri/Cargo.toml`
- `src-tauri/Cargo.lock`（仅 `shuyin-ai` 包）

版本字段不一致时直接失败，不自动掩盖不一致。

## 执行顺序

1. 检出当前 main，获取 main、dev 和标签；验证事件来自 main 的代码推送，且推送提交在 main 中。
2. 若现有版本标签已包含该推送提交，直接跳过，避免重跑、版本回写或排队事件再次递增。
3. prepare 在本地生成只修改五个版本文件的 `chore(release): vX.Y.Z` 提交，运行 lint、前端与发布测试，并用 Git bundle 导出这一个提交，尚不推送远端。
4. build 矩阵分别下载由 artifact ID 指定的 bundle，校验并恢复预期提交；安装锁定依赖与 Rust 1.99.1，运行各平台 Rust 格式/Clippy/数据库测试和归档测试，再运行 Tauri（包含 TypeScript 检查与 Vite 构建）。
5. 各平台整理分发文件，上传以版本提交 SHA 和平台命名的 artifact。缺失、空文件或多个候选安装包均失败。
6. sync 只在四个平台全部成功后运行：恢复同一个提交，下载并校验全部 16 个包/签名文件、生成包含更新说明的 `latest.json` 与 SHA-256 清单，再次读取远端 main/dev。确认 main 仍是本次构建的父提交，然后在最新 dev 执行 `git cherry-pick -x <版本提交>`。
7. 一次 `git push --atomic` 提交 main、dev 和版本标签；任意引用被拒绝则全部不更新，不使用强制推送。
8. publish 下载同一批产物，再次校验并创建 GitHub Release 草稿，上传完整文件与校验清单，最后公开。已公开版本不替换文件；失败草稿可重跑继续上传。

prepare/build 仅拥有仓库读取权限；sync/publish 才拥有 `contents: write`。构建任务不能自行修改 main/dev。

main 与 dev 的版本一致，但提交 SHA 不同，dev 未发布的业务改动保留。这个流程只同步版本提交；如果其他业务分支直接合入 main，其业务代码不会通过版本 cherry-pick 自动回流 dev。

同一流水线同时只运行一个任务，并保留最新的一个等待任务。每次以启动时的最新 main 为构建快照，读取全部未发布提交，可能覆盖多个已合并 PR；已被成功标签覆盖的后续事件跳过。构建期间若 main 又有新合并，本次在推送前停止，后续排队任务会构建更新后的 main；也可在 Actions 重跑失败的运行。连续合并时请把版本级别写入提交说明，保证被合并到同一次构建的变更也能参与级别判断。

## 仓库设置与失败恢复

- `main`、`dev` 必须已存在，仓库开启 GitHub Actions，并允许本工作流请求 `contents: write`。
- 分支与标签保护必须允许工作流身份写入。`GITHUB_TOKEN` 不会自动绕过保护；如果组织规定必须使用具有绕过权限的 GitHub App，需要先配置该身份，再调整 checkout 的 token。不要为运行流水线关闭整个仓库的保护。
- 将包含本工作流的提交推送到 main 即可触发，不需要额外创建 PR 或标签。仅保存在本地不会执行，旧推送不会自动补跑；工作流没有手动触发入口。
- 构建或产物上传失败：远端版本未递增，可选择 **Re-run failed jobs** 复用原版本与成功平台产物；如果 main 已前进，选择 **Re-run all jobs** 或等待下一次合并，以构建最新代码。
- 如果修复涉及工作流或脚本代码，必须把修复提交推送到 main，启动新运行；重跑旧运行仍使用旧代码。Windows 的 Git Bash 中，GNU tar 会把绝对归档路径中的 `C:` 解释为远程主机并报 `Cannot connect to C: resolve failed`；归档脚本及测试统一通过工作目录和相对归档文件名执行，NSIS 打包不受此调整影响。
- macOS 出现 `bundle_dmg.sh` / `hdiutil: couldn't eject ... Resource busy`：应用编译与 APP 签名可能已完成，失败发生在临时镜像卸载阶段。2026-10-29 的修复移除了 CI 中显式挂载、修改、卸载镜像的流程。Tauri CLI 2.12.4 的内置脚本对此错误最多尝试三次；`CI=true` 仅跳过 Finder AppleScript，仍会挂载和卸载镜像，因此重复设置 CI 或增加同样的重试不能移除此失败点。日志未表明具体占用进程；缺少 Apple 公证变量的警告不是本次退出原因。修复需要合入 main 后的新运行验证，Windows 上的命令替身测试不等同于真实 DMG 构建。
- main 前进或远端拒绝写入：main/dev/标签均不由本次更新，重新运行会重新读取当前版本。已上传 artifact 是失败运行的候选产物，以**整个任务成功并存在对应标签**作为交付依据。
- dev 冲突：自动中止 cherry-pick，保留远端分支。检查 dev 上的版本文件，把版本字段与已发布版本对齐并保留其他开发内容，再重跑；不通过覆盖 dev 解决冲突。
- 同步成功而 Release 上传/公开失败：选择原运行的 **Re-run failed jobs**，只重跑 publish，使用保留的全部平台产物恢复草稿；不再次增加版本。此时 main/dev/标签已经同步，Git 分支与 GitHub Release API 不属于同一事务。
- sync 若在原子推送成功后因 runner 中断被判定失败，重跑会验证相同标签与 dev cherry-pick 记录并跳过重复推送。
- 同步成功后选择 **Re-run all jobs**：prepare 发现标签已覆盖推送提交，跳过后续阶段。恢复发布失败时应选择 **Re-run failed jobs**；需要在 90 天 artifact 保留期内操作。

## 非 Windows 音频死代码检查修复（2026-10-29）

`audio_checked` 和 `audio_step_failed` 仅由 Windows 音频调度器调用。原来的 `cfg(any(target_os = "windows", test))` 使 macOS/Linux 测试构建也包含这两个没有测试调用的方法，因 `cargo clippy --all-targets -- -D warnings` 的死代码检查而失败，尚未进入安装包构建。两处条件改为 `cfg(target_os = "windows")`，与实际调用方一致；保留严格检查和其他跨平台数据库测试。

本次为 `fix`（patch），无依赖、数据库结构或迁移变更，不手动修改版本。Windows / Rust 1.99.1 本地验证：

- `npm run check:rust`：fmt 与 Clippy 通过。
- `npm run test:rust`：117 项通过，1 项既有原生凭据测试忽略；临时目录使用仓库 `output/audio-build-fix-test-temp`。
- `node --test tests/audio-page.test.mjs tests/audio-package.test.mjs tests/release-assets.test.mjs`：38 项通过，1 项 macOS 专属归档测试跳过。
- 最小 Rust 编译夹具提取实际方法的条件编译标记，并用自定义条件模拟 Windows 调用方的存在与缺席；修复前只有非 Windows 测试配置出现相同的两方法死代码错误，修复后两种配置均通过。此项不等于 macOS/Linux 原生构建。

本轮未生成安装包或执行 macOS/Linux 原生构建，四平台结果仍需新一轮 CI 确认。修复合入 main 并推送后触发新运行；重跑旧运行不会使用这次源码修改。

## 本地验证

```powershell
npm run test:release
npm run lint
npm test
npm run check:rust
npm run test:rust
npm run desktop:build -- -- --locked
node scripts/release-assets.mjs collect windows-x64 '' output/release
.\.tools\actionlint\actionlint.exe .github/workflows/release.yml
```

本机先按 README 载入 `.tools/env.ps1`。上面的 collect 示例用于 Windows 默认 target 输出；CI 显式传入矩阵 target。actionlint 是下载到忽略目录的本地检查工具，不是应用依赖。

发布测试使用临时本地 bare 仓库，不连接或推送真实 origin。覆盖版本优先级、首发、字段一致性、版本专属提交、dev 内容保留、重跑、历史变更隔离、main 前进、cherry-pick 冲突、原子推送拒绝、标签碰撞、跨 runner bundle 恢复与重复同步。产物测试覆盖全平台清单、缺失/空/额外文件、校验和、系统归档，以及草稿发布和失败恢复；macOS/Windows 专属归档测试在对应 runner 上执行。

Windows 本地实际构建及 EXE/ZIP 整理已验证；macOS/Linux 原生构建、三平台安装升级、真实 GitHub runner/权限/事件/Release 仍需远端与人工验收。本地使用测试文件验证归档和发布逻辑，不代表这些系统的原生安装包已经构建成功。
