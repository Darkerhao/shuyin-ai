# 关于与应用更新验收（2026-10-25）

本次为兼容的功能增强（feat），保留当前源码版本 0.11.0，交由原发布流水线统一升 minor。未提交、推送、创建 Release、修改 GitHub Secrets 或替换正式安装。

## 实现与验收对应

| 需求 | 实现与证据 |
| --- | --- |
| 关于页面 | 侧栏「关于与更新」进入 `/about`，原生读取应用实际版本，显示更新设置与说明。 |
| 自动更新 | 默认启动时检查一次并后台下载。跨页面共享同一状态；检查、下载、安装互斥。下载后仅提示，由用户确认安装并重启。 |
| 手动更新 | 关闭自动模式后仍能手动检查、下载；错误可重试；设置写入失败保留原开关，读取失败可重试。 |
| 下载反馈 | 已下载体积、可确定时的百分比、检查与安装状态、网络/签名错误；保留更新说明与重试入口。 |
| 更新内容 | 发布说明从上一个正式标签之后的实际提交标题生成；同一内容写入构建、latest.json 与 GitHub Release。已安装版本没有新更新时仍显示构建时附带的说明。 |
| 发布闭环 | 官方 Updater 2.14.0 / Process 2.4.0，固定公钥；原四平台流水线生成更新包/签名，缺少私钥明确失败。16 个包/签名附件加清单与校验文件共 18 个。 |
| 数据与退出 | schema v9 不变，复用 app_settings.auto_update，默认 true，备份恢复与重开保持。安装入口仅在关于页；原文案/封面页的离页守卫先等待草稿写入，生成中禁止离页。 |

## 修改文件

- 前端：`src/App.vue`、`src/app/router/index.ts`、`src/modules/settings/AboutPage.vue`、`src/composables/useAppUpdates.ts`、`src/services/updates.ts`。
- 桌面：`src-tauri/src/lib.rs`、`src-tauri/src/updates.rs`、`src-tauri/permissions/updates.toml`、`src-tauri/capabilities/main.json`、`src-tauri/tauri.conf.json`。
- 依赖：`package.json`、`package-lock.json`、`src-tauri/Cargo.toml`、`src-tauri/Cargo.lock`。只新增两套官方插件及必要间接依赖，未升级原有依赖版本。
- 发布：`.github/workflows/release.yml`、`scripts/release-assets.mjs`。
- 测试：`tests/updates.test.ts`、`tests/release-assets.test.mjs`。
- 文档：`docs/PRD.md`、`docs/TECHNICAL-DESIGN.md`、`docs/DATABASE.md`、`docs/DEVELOPMENT-PLAN.md`、`docs/RELEASE.md`、`docs/USER-GUIDE.md`、本文。
- 无数据库迁移，无旧接口、旧依赖或重复更新实现需要删除。

## 实际命令及结果

Rust 命令先加载 `. ./.tools/env.ps1`；测试时 TEMP、TMP、SQLITE_TMPDIR 仅在当前进程指向仓库 `output/update-test-temp`，未改变系统设置。

| 命令 | 实际结果 |
| --- | --- |
| `npm run lint` | 最终通过，0 警告。 |
| `npm run typecheck` | 通过；最终打包前的 `npm run build` 再次执行通过。 |
| `npm test` | 首轮全量通过：前端 104 项；发布脚本 29 项通过、1 项 macOS 检查按原条件跳过。后续新增测试见下面两行。 |
| `npx vitest run` | 最终 106 项通过。 |
| `npx vitest run tests/updates.test.ts` | 最后调整设置保存状态后再次验证，11 项通过。 |
| `node --test tests/release-assets.test.mjs` | 最终 12 项通过、1 项 macOS 检查跳过；含原有归档、草稿发布恢复以及新签名配对、清单和说明测试。 |
| `npm run check:rust` | 最终 fmt / Clippy 全部通过。 |
| `npm run test:rust` | 67 项通过、1 项原生凭据测试沿用原设置忽略。 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked updates::tests` | 增强备份恢复断言后重跑，1 项通过。 |
| `npm run build` | TypeScript 与 Vite 生产构建通过。 |
| `.tools/actionlint/actionlint.exe .github/workflows/release.yml` | 通过。 |
| `git diff --check` | 通过。 |
| `npm run desktop:build -- --target x86_64-pc-windows-msvc -- --locked` | 使用仓库外私钥，最终按正式 identifier/更新地址重新构建，Windows NSIS EXE 与 `.exe.sig` 均成功；没有安装。 |
| `node scripts/release-assets.mjs collect windows-x64 x86_64-pc-windows-msvc output/updater-windows` | 真实整理出 Windows EXE、对应签名和 ZIP 三个文件。 |

首次 lint 报新增页面格式、测试 URL 导入和重复测试宿主组件问题，已修正，未弱化规则。首次 `cargo update` 曾更新无关的间接依赖，随后恢复基线锁文件，仅解析新增插件所需依赖；最终锁文件没有原有依赖版本升级。

## 浏览器与原生验证

使用 Playwright CLI 的独立 `shuyin-updates` 会话，针对 `http://127.0.0.1:1435/#/about` 执行：

```text
npx --yes --package @playwright/cli playwright-cli -s=shuyin-updates run-code --filename output/playwright/update-ui-smoke.js
```

1440、1120、800、360px 均无横向溢出，页面错误为 0。截图 `output/playwright/about-preview.png` 已目视核对；浏览器预览正确禁用原生命令。

原生使用独立 `com.shuyin.updater-verification-20261025`、本机临时 HTTP 更新服务以及 WebView2 本机调试端口，执行：

```text
npx tauri dev --release --target x86_64-pc-windows-msvc --no-watch --config output/playwright/updater-test.conf.json -- --locked
node output/playwright/updater-native-check.mjs
```

验证配置将当前应用版本设为 0.10.0，检查本次真实签名的 0.11.0 NSIS 包。实际插件完成网络检查、下载、100% 进度与公钥校验，到达待安装状态；更新说明在真实 WebView2 显示。实际 SQLite 命令保存/读取关闭自动更新的偏好。确认安装面板可打开并取消，未执行安装。服务端只修改包末尾一个字节后，真实插件以 `The signature verification failed` 拒绝下载。结果和原生截图保存为 `output/playwright/native-updater-result.json`、`output/playwright/about-native-ready.png`。

最初测试清单误把 0.11.0 的签名包标为 0.12.0，插件正确拒绝版本与签名不一致；改为上述较低的测试应用版本后下载通过。首次开发启动还因原有 debug 程序占用可执行文件失败，改用未被占用的 release 目标进行隔离验证，没有结束用户原有进程。

HTTP、模拟旧版本、独立 identifier 和本机调试端口只在忽略的 output 验证配置中使用。测试服务和原生测试实例已结束；最终交付包重新使用正式 HTTPS 更新地址、正式 identifier 和版本 0.11.0。未打开或改写正式业务数据库。

## 待完成的发布验收与下一步

1. 参考 `RELEASE.md`，将仓库外私钥内容配置为 GitHub Actions 的 `TAURI_SIGNING_PRIVATE_KEY`，有密码时配置密码 Secret；本次没有远程配置。私钥须保留，公钥已固定在客户端。
2. 以 feat 提交合入 main，发布流水线生成下一个 minor 版本；当前公开更新清单请求为 404，首次签名 Release 公开后才有正式在线更新源。
3. 从旧版本先手动安装一次带更新能力的版本，再验收真实 GitHub 下载、NSIS 安装/重启及创作数据保持。局部原生下载测试不是安装升级测试。
4. macOS Intel/Apple Silicon 与 Linux 仍需各原生 runner 打包并实机验收；Linux AppImage 才使用应用内安装，DEB/RPM 交由系统包管理器更新，Windows ZIP 手动替换新版。

本次已完成代码、自动化、Windows 签名打包、页面与真实插件下载/拒绝篡改的本地验收；未声明线上发布、实际安装、系统安装包代码签名或其他平台实机验证通过。
