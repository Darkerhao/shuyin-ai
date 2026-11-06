# 自动制作与抖音发布准备验收

日期：2026-11-07。用户修订后采用现有网页路线，允许最后在抖音点击发布。代码基线 `91461ae`（dev）；新增能力按 `feat` / minor，版本由现有发布流水线处理。

## 实现

一个原书文件、书名作者及一次性准备 → 固定原书音频指导 → NotebookLM音频 → 现有字幕校对/分镜 → 根据实际完整字幕生成文案 → 两种比例AI背景 → 根级Canvas双封面 → 原视频任务自动渲染 → 冻结发布材料并自动上传填写 → 用户最终发布及原结果记录。

默认目标10分钟、4:3和3:4各一次背景生成、北京时间未来01:11、不允许保存、不加入合集。图片失败不降为纯色。正式支持格式沿用现有音频来源PDF/EPUB/TXT/Markdown，DOCX尚不纳入自动模式。

首次需要配置模型/视频环境、网站登录及核对当前抖音窗口账号；自动采用和计划内调用在启动前授权。账号冻结，不跨账号继续；验证码、未知处理状态、旧草稿和平台不接受设置时暂停。交接用户前候选时间失效可续算同一任务，交接后不改期。发布声明、最终按钮及实际结果记录由用户完成；不冒充已经公开发布。

新增文件包括Rust执行器、数据库流程模块、v20迁移、权限、两个版本化Prompt、Vue全局流程/启动组件、服务及回归；增量修改原音频/视频/AI背景/封面/发布业务方法以原子绑定。无新增依赖，不修改原文件或已发布任务默认值。

## 数据与恢复

v20 `automation_runs`存输入、子任务关联、运行代次、状态和上传/人工交接标记，唯一启动键和单活动流程索引防重复。子任务创建与绑定共用事务；暂停/过期绑定失败回滚。原手动人工确认字段保持原语义，自动发布使用完整策略快照且人工确认字段为false。

音频归档摘要初始化自动视频，原执行器验证文件；文案使用实际冻结音频指导，保存视频计划摘要，渲染和发布前核对。数据库/完整备份保留流程；恢复暂停，不重复发送未知上传。暂停阻止新阶段并暂停音频/视频；单次已发出的文本/图片请求可完成落库。停止通知绑定子任务停止后续动作，已发出的远端操作不等于已撤销。

## 实际验证

Rust命令先 `. ./.tools/env.ps1`；Node脚本验证在本次进程将 `D:/MyWork/software/shuyin-video-node/v24.22.2` 放PATH首位，未修改系统Node。

| 命令 | 结果 |
| --- | --- |
| `npm run lint` | 通过 |
| `npm run typecheck` | 通过 |
| `npm test` | 前端434、视频39、发布脚本78、队列19通过；macOS专属1项跳过 |
| `npm run test:rust` | 最终239通过、2项已有专项忽略（系统凭据、外部Node进程树） |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked automation` | 最终13项流程聚焦回归通过，包含音频停止后拒绝迟到来源与生成动作 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked transcript_output` | 1项实际冻结指导保留及拒绝额外模型字段回归通过 |
| `npx vitest run tests/automation.test.ts tests/publishing.test.ts tests/projects.test.ts` | 审核修复后76项通过 |
| `npx vitest run tests/automation.test.ts tests/projects.test.ts` | 最后表单保存保护调整后37项通过 |
| `npm run check:rust` | fmt及Clippy通过 |
| `npm run build` | TypeScript和Vite构建通过，保留既有大分块提示 |
| `git diff --check` | 通过 |

浏览器使用 `npx --yes --package @playwright/cli playwright-cli -s=shuyin-automation run-code --filename output/playwright/automation-20261107/browser.cjs`。隔离内存IPC验证启动前模型/账号/授权、一次启动、暂停/继续、切页保持根级流程、1440/1120/800/390四档无页面溢出。截图已检查，见忽略目录同名文件；不能代替真实网站上传。

桌面先执行 `npm run tauri -- build --debug --no-bundle --config output/playwright/automation-20261107/tauri.verify.conf.json`，前端成功但开发版EXE被用户当前运行实例占用，构建返回os error 5。保留该实例，改用 `TAURI_CONFIG` 指向同一隔离配置，`cargo rustc --manifest-path src-tauri/Cargo.toml --locked --bin shuyin-ai -- -o D:/MyWork/project/JIAZI-PROJECT/shuyin-ai/output/playwright/automation-20261107/shuyin-automation.exe` 成功生成隔离程序。仅本轮程序开放回环验收端口，产品配置不开放调试端口。

通过 `node output/playwright/automation-20261107/native.mjs` 运行真实WebView2、Tauri IPC、SQLite、Provider、Canvas和原视频引擎：隔离库预置已完成音频/字幕分镜作为起点，随后1次文本请求、2次图片请求、从未打开封面页导出两张准确尺寸PNG、原引擎渲染6秒1920×1080含音轨视频并登记，自动创建发布任务，01:11/禁止保存/人工确认false均正确，无发布成功记录。模型为本机契约服务，起点音频为生成的短测试音，不宣称本轮通过真实NotebookLM整书生成或事实听审。脚本证据为 `native-result.json`。

通过 `node output/playwright/automation-20261107/reopen.mjs` 验证隔离程序重启保留相同流程/子任务、1份文案和2份封面；上传意图已记录但网页会话丢失时转等待核对，不再次上传。真实正式账号、最终发布、完整原书、长视频、安装升级和macOS/Linux未执行。

初次测试夹具误将素材类型当成重定位ID、音频JSON使用错误大小写字段，均修正夹具后通过；浏览器初次同hash跳转未重载初始化脚本，增加明确reload后通过；重启验收先等待CDP端点就绪。未通过降低断言、删除测试或关闭检查处理失败。

## 代码审核

按 `code-review` 技能分工程规范、需求两个独立只读审核。修复并复核：文案/视频混合版本、执行器占用遗留running、封面迟到失败暂停另一比例、视频未绑定归档音频摘要、指导模板重算、上传跨日默认日期失效、停止父流程后子任务继续推进。两轴最终均未发现新增阻断项；审核结论与测试/真实服务覆盖分开记录。

## 验收对应与限制

本次修订的本地闭环、精确ID/摘要、默认设置、根级封面、切页、去重、迁移/备份、暂停恢复和旧手动回归有上述证据；网页填写继续沿用已有真实页面适配，本轮只回归其测试，未操作真实账号。公开API全程无人值守方案P0–P4/A1–A20保留为历史研究，不宣称全部通过。

下一步实际使用有权处理的原书，在已登录Windows环境核对完整音频/视频质量和抖音当前页面，最后用户点击发布并记录真实结果。平台登录/验证码/声明/页面变更是仍可能需要人工处理的部分。
