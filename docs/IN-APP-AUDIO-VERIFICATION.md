# 应用内自动音频实施与验收

日期：2026-10-29。平台：Windows x64 / WebView2。源码版本字段：0.21.0；SQLite：v13。

## 1. 交付结论

已按用户要求，以应用管理的专用网站窗口替换 Chrome 扩展和 Native Messaging helper。点击「开始生成音频」后，由 Rust 跟进网页阶段，自动添加来源、填写完整指导、提交生成、接收具体下载对象并登记音频素材。保留「查看网站」、暂停、继续、停止及手动导入。

**本地代码和隔离模拟网页闭环已验证；真实 Google 登录、网站控件和生成下载未验收。** 模拟音频和模拟笔记本均明确标注，不是 Google 服务结果。正在运行的正式安装没有被替换；本轮没有提交、推送或发布。

单一方案、状态流程、恢复规则和官方依据见 [设计文档](NOTEBOOKLM-IN-APP-AUDIO-DESIGN.md)。

## 2. 实现与文件范围

| 范围 | 主要文件与结果 |
| --- | --- |
| 网站宿主与调度 | `src-tauri/src/audio.rs` 管理专用窗口、串行执行、任务/运行/文档关联及有限等待；`audio_commands.rs` 提供主工作台入口 |
| 页面动作 | `src-tauri/src/audio_page.js` 复用一份中英文语义控件适配，完整回读指导，不明确时等待人工核对 |
| 上传和下载 | `webview_host.rs` 从抖音宿主整理共用；`audio_download.rs` 使用 WebView2 下载开始、完成、中断与取消事件 |
| 数据和归档 | `src-tauri/src/database/audio.rs` 保留冻结输入，制作核验后的原书副本，下载完成后校验格式、大小和 SHA-256，事务登记素材 |
| 前端 | `WritingPage.vue`、`services/audio.ts`、`types/audio.ts` 删除扩展连接门槛和安装按钮，显示当前完整网址与实际任务阶段 |
| 旧链路清理 | 删除 `browser-extension/` 的源码、`src-tauri/browser-bridge/` 工程源码、桥接构建脚本及旧扩展测试；根忽略规则继续排除 Rust 构建缓存 |
| 打包 | Tauri 配置、NSIS hooks、发布 workflow、ZIP 收集和测试删除 helper/扩展资源；升级清理仅处理精确对应本产品的旧宿主注册和指定旧文件 |
| 文档 | 同步 PRD、技术设计、数据库说明、AI 规则、使用手册、开发计划、README 和规划记录；旧音频方案保留历史标识 |

已有书籍来源绑定改动继续保留。`windows-core = 0.62.2` 由既有间接锁定改为显式声明，没有增加库版本；移除旧桥接所需的 Registry/FileSystem 直接 feature 声明。

## 3. 数据库与恢复

本次替换仍使用 **v13**，没有新表、列或迁移，也没有修改历史迁移。旧成功记录、已归档音频及发布引用保留。

- 旧 `awaiting_extension` 任务继续时转为 `opening_site`，无需扩展。
- 应用重启后，尚未开始的任务等待继续；已开始或提交状态不明的任务先核对原 Notebook，不重复生成。
- 暂停/停止使旧运行失效，旧下载回调不能写入新运行。
- 完整缓存有大小和摘要记录时可只重试归档；部分文件不作为成功结果。
- `.audio-jobs` 缓存当前保留，不进入完整备份；自动缓存清理未交付。

## 4. 实际命令与结果

Rust 命令先执行 `. ./.tools/env.ps1`。表中阶段区分前段验证与最终回归，不把未执行的检查写成通过。

| 命令 | 实际结果 |
| --- | --- |
| `npx vitest run` | 本轮前段：204 项通过 |
| `npm run lint` | 本轮前段：通过 |
| `npm run build` | 前段单独运行及最终打包的 beforeBuildCommand 均通过，包含类型检查；保留 513.42 kB 的既有分块提示 |
| `npm run test:release` | 本轮前段：54 项通过，1 项 macOS 平台测试跳过；随后补充页面用例并执行下列聚焦回归 |
| `node --test tests/audio-page.test.mjs tests/audio-package.test.mjs tests/release-assets.test.mjs` | 收尾聚焦回归：38 项通过，1 项平台测试跳过；包含 21 项页面用例和 4 项打包用例 |
| `npm run check:rust` | 收尾回归：fmt 和 Clippy 通过 |
| `npm run test:rust` | 收尾回归：117 项通过，1 项既有原生凭据测试按原配置忽略 |
| `npm run desktop:build -- --debug --bundles nsis --config output/playwright/in-app-audio/tauri.json` | 清理诊断输出后的最终隔离调试 NSIS 构建通过，产物 7,028,075 字节 |
| `python output/playwright/in-app-audio/audit.py` | 最终二进制完成四个组合并关闭后，v13、完整性、外键、四项成功记录、文件大小和 SHA-256 均核对通过 |
| `git diff --check` | 通过；另核对四份当前文档的 16 个本地 Markdown 链接，目标均存在 |

`npm test` 还包含独立的 Codex 队列测试。此前已经重复定位到未修改的 `tests/codex-queue.test.mjs` 缺少前序报告重跑用例失败，详见开发计划中的书籍来源绑定记录；本次没有修改队列实现，也没有将整个 `npm test` 宣称为全绿。

普通沙箱运行曾拒绝 Rust 格式化、构建缓存和 Vite 输出写入；使用获准的本机开发命令完成相应检查。停止隔离测试进程也需要同一执行权限；未停止正式安装实例。

最终调试包：`src-tauri/target/debug/bundle/nsis/Shuyin Audio Verification_0.21.0_x64-setup.exe`。SHA-256：`E67772D62FA6AAA698BF002C9986F507FD2F6C7CF57378662B86FFF941021BBA`。只生成隔离标识测试包，没有安装、签名发布或替换正式程序。

已实际检查生成的 `src-tauri/target/debug/nsis/x64/installer.nsi`：含旧资源清理 hook 及安装/卸载宏，没有打包 helper/扩展的 `File` 条目。旧注册和文件的清理行为由脚本检查覆盖，未操作正式注册表或用户 Chrome 资料。

## 5. Windows 原生隔离闭环

隔离标识：`com.shuyin.audio-in-app-verification-20261029`，数据库位于该标识独立的 AppData 目录。测试使用真实 Tauri 命令、Rust 调度、WebView2、原生文件输入和下载完成回调；网页内容与音频数据来自明确标注的模拟夹具。

命令：

```powershell
$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = '--remote-debugging-port=0'
# 启动采用独立配置编译的测试应用；调试端口只用于此隔离验收。
$env:SHUYIN_AUDIO_TEST_DOWNLOAD = 'blob'
node output/playwright/in-app-audio/native.cjs
$env:SHUYIN_AUDIO_TEST_DOWNLOAD = 'https'
node output/playwright/in-app-audio/native.cjs
```

HTTPS 测试使用临时自签名本机服务，测试会话接受该证书；没有安装系统证书，也没有在产品代码加入证书例外。产品没有开放远程调试端口。

| 来源 / 下载方式 | 实际结果 |
| --- | --- |
| 资料文本 / blob | 创建笔记本、添加文字、填写指导、一次生成、下载与素材登记通过 |
| 本书原书文件 / blob | 原生选择副本，文件名和中文内容一致；一次生成、下载与素材登记通过 |
| 资料文本 / HTTPS | 真实本机 HTTPS 响应触发原生下载，下载与素材登记通过 |
| 本书原书文件 / HTTPS | 原生文件输入、一次生成、HTTPS 下载与素材登记通过 |

每个任务均断言：生成计数恰为 1、完整指导一致、恰有一个音频素材、落盘字节与合成 WAV 的 844 字节完全一致。文件来源还核对上传文本与原文件一致。任务依次复用同一个网站窗口，验证切换任务无需重建同名 WebView。清理诊断输出并完成最终打包后，已重新启动最终二进制，四个组合再次通过。

最后通过仅面向隔离主窗口的原生销毁命令关闭测试应用，并确认测试进程 150716 退出、正式进程 142132 仍位于 `D:/书引 AI/shuyin-ai.exe`。关闭后只读检查 SQLite：`user_version=13`、`integrity_check=ok`、外键错误为 0，四个最终任务的成功状态、唯一音频素材、文件大小和摘要持久化一致。

本机证据位于忽略目录，不作为仓库分发资源：

- `output/playwright/in-app-audio/native.cjs`、`fixture.html`：隔离验收脚本与明确标注的模拟网页。
- `native-report.json`：首轮 blob 文本与文件成功记录。
- `native-report-blob.json`、`native-report-https.json`：最终二进制的 blob/HTTPS 文本与文件成功记录。
- `audit.py`、`audit-report.json`：最终四个组合的只读数据库、文件大小和摘要核对。
- `text-https-main.png`、`asset-https-main.png`：真实主工作台任务完成与音频列表；已查看文件来源截图，布局和操作可见。
- `text-site.png`、`asset-site.png`、`text-https-site.png`、`asset-https-site.png`：明确标注的模拟网站结果。

## 6. 验证中定位并修复的问题

| 实际现象 | 根因与最小处理 |
| --- | --- |
| 创建后无法解析网页观察 | JS 返回 `notebookUrl`，Rust 枚举字段未使用 camelCase；统一序列化契约并加回归 |
| 未进入自定义入口就填写 | 无匹配标签时，标题输入框被选作指导；显式标签必须语义匹配 |
| 第二个任务打不开网站 | 销毁后立即创建相同标签发生竞争；改为复用唯一窗口并更新任务绑定 |
| 下载尚未归档就重复处理 | 调度先执行了缓存归档重试；改为先等待当前具体下载与归档 worker |
| 下载导航使自身结果失效 | 下载也触发导航事件；仅已绑定精确 URL 允许在有效期内保持文档代次 |
| 原生下载中断 | 传给 WebView2 的目标包含 Windows 扩展路径前缀；上传、下载统一复用现有 `paths::shell_path`，中文文件成功闭环 |
| 模拟 HTTPS 不触发原生下载 | 直接拦截并返回文件的夹具未触发下载事件；改为真实本机 HTTPS 文件响应后通过，没有为该测试改产品下载逻辑 |

隔离连接还显式恢复浏览器默认下载处理，避免测试端接管下载替代原生宿主。临时诊断输出已从产品源码移除。

## 7. 验收对应与待完成项

| 用户要求 | 对应情况 |
| --- | --- |
| 应用内部打开网站 | 已实现专用应用窗口，真实 Windows WebView2 已运行 |
| 不安装扩展或 helper | 已移除连接门槛、工程源码、随包资源与构建依赖，打包配置回归通过 |
| 自动找到入口执行任务 | 已实现并以模拟中英文控件及原生桌面流程验证；真实 Google 控件待验收 |
| 成品回到本书 | 模拟 blob/HTTPS 成品校验、落盘、素材事务和列表展示通过 |
| 保留控制和失败恢复 | 页面/数据库回归覆盖暂停、失效运行、未知提交不重发、只读核对、缓存归档和旧任务继续 |
| 文档与实现一致 | 当前设计、用户说明、数据库边界和本报告同步 |

发布前仍须验证 Google 官方允许的登录路径、真实中英文页面、实际文件类型和完整提示词、一次真实生成、服务侧 HTTPS/blob 下载及长时间隐藏窗口。嵌入式登录支持不能从模拟或抖音登录经验推断；Google 拒绝时保留任务并使用既有手动导入，不绕过限制。

NSIS 构建不等于安装升级/卸载验收；本轮没有修改用户正式安装或旧 Chrome 用户资料，也未实跑 macOS/Linux 编译与桌面流程。这两个系统保留手动音频导入。下一步先在隔离新版中完成一次真实 Google 账号验收，再决定分发；不以本报告代替真实服务验收。
