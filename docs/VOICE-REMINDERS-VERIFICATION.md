# 流程完成语音提醒验收（2026-11-10）

## 播放状态与按钮动效补充（2026-11-10）

根因：原服务只公开开启和错误状态，顶栏只能显示开/关喇叭与固定试听图标，无法反映正在播报。现沿用实际音频节点的启动、结束和停止更新只读播放状态；整轮提醒未结束时保持声波，旧节点迟到结束不清除新播放。开启待命显示喇叭，关闭显示静音，播报显示四条律动声波；试听切换方形停止图标并可停止本轮语音，保持总开关开启。增加短过渡、按压、焦点和减少动态效果样式。

修改`src/components/VoiceReminderToggle.vue`、`src/components/AppIcon.vue`、`src/services/voiceReminders.ts`和`tests/voice-reminders.test.ts`，同步PRD、技术设计、使用手册、开发计划及本记录。无数据库迁移、新依赖、计时器或业务流程改动，版本影响为`fix`/patch。

| 实际命令 | 结果 |
| --- | --- |
| `npx vitest run tests/voice-reminders.test.ts`（修复前） | 原12项通过，新增3项失败，确认缺少播放状态与播放中图标 |
| `npx vitest run tests/voice-reminders.test.ts tests/ui-refinement.test.ts` | 24项通过，包括自然结束、队列结束、停止、迟到事件和界面控制 |
| `npm run lint` | 通过，0警告 |
| `npm run build` | TypeScript与Vite生产构建通过，保留已有大于500kB分包警告 |
| `node D:/DevCache/npm-cache/_npx/31e32ef8478fbf80/node_modules/@playwright/cli/playwright-cli.js -s=voice-motion run-code --filename output/playwright/voice-reminders/check-motion.js` | 真实内置WAV/AudioContext播放与自然结束、队列停止、再次试听、浅深色、375px图标、减少动态效果和键盘关闭均通过；无页面异常 |
| `git diff --check` | 通过 |

已查看`output/playwright/voice-reminders/motion-dark-playing.png`、`motion-light-playing.png`与`motion-narrow-reduced.png`，播放状态清晰且窄窗图标可见。浏览器使用本地预览与显式测试完成事件，没有触发模型或真实任务；本轮未构建Windows安装包或验证桌面WebView2新动效。验收标准在前端与浏览器范围已满足，下一步在包含本次改动的桌面构建中试听确认。

收尾浏览器检查发现新增焦点描边引用了未定义的颜色变量，已改用项目现有`--info`；实际键盘模式下可见焦点、播放状态和减少动态效果整套脚本复验通过，随后重新执行生产构建通过。测试脚本支持重置开关、主题及动态效果，并为每轮完成事件使用独立ID，避免真实去重规则干扰重复验收。

### 本次动效增量评审与测试

用户追加评审测试后，以`f2d11210c82b49c9c3137790fc6d2fec641579f5`（HEAD）为固定基准，检查当前9个未提交文件。两名只读代理分别检查工程规范和当前PRD/技术设计，主代理另对照用户原始播放图标及动效要求。

- Standards：0项确定问题。沿用原播放队列、停止入口及组件，未引入新依赖、计时器或持久化状态；可访问名称、焦点及减少动态效果具备对应实现。
- Spec：0项确定偏差。关闭/开启待命/播放中的图标、结束复位、停止保持总开关及按钮动效符合当前需求；主代理核对原始要求无遗漏。
- 新增3项回归：停止时丢弃尚在解码的旧语音且后续完成仍可提醒；播放失败维持待命图标并可试听恢复；快速关闭后不残留播放或声波。无需修改生产实现。

| 本轮实际命令 | 结果 |
| --- | --- |
| `npx vitest run tests/voice-reminders.test.ts` | Spec评审代理执行，18项通过 |
| `npx vitest run tests/voice-reminders.test.ts tests/ui-refinement.test.ts` | Standards评审代理执行，27项通过；此前实施阶段的24项记录保留为历史结果 |
| `npx vitest run --maxWorkers=2` | 主代理首次33文件/465项通过；补齐3项边界测试后最终33文件/468项通过 |
| `npm run lint` | 主代理复跑通过，0警告 |
| `npm run typecheck` | 主代理复跑通过 |
| `node D:/DevCache/npm-cache/_npx/31e32ef8478fbf80/node_modules/@playwright/cli/playwright-cli.js -s=voice-motion run-code --filename output/playwright/voice-reminders/check-motion.js --raw` | 主代理复跑通过：真实WAV自然结束、队列停止、再试听、浅深色、375px、减少动态效果与键盘关闭；页面异常0 |
| `git diff --check` | 通过 |

浏览器结果保存为`output/playwright/voice-reminders/motion-review-result.json`：正常状态四条声波有动画；减少动态效果时动画为none、过渡为0s、图标可见；键盘焦点为2px实线。验收标准在代码、前端回归及浏览器范围满足。

评审轮修改`tests/voice-reminders.test.ts`、本记录和开发计划；本地浏览器脚本与结果保存在忽略的output目录。数据库仍无变更。本轮未重跑Rust、桌面WebView2或安装包测试；生产实现与此前通过`npm run build`的版本相同，既有大于500kB分包提示仍存在。未提交、推送或发布。下一步在包含本次改动的Windows桌面构建中试听确认。

## 评审与修复结论

评审范围为HEAD之后本功能的未提交修改及新增文件，需求依据为用户原始语音提醒/总开关要求及PRD当前章节；工程依据为AGENTS.md。两名独立代理分别检查Standards与Spec，修复后由另一代理只读复核。没有提交、推送或发布。

### Standards

初评1项P2：任务包使用固定发布任务ID去重，同任务新的成功导出会被忽略。现改用实际产物目录作为事件标识，不新增状态表或导出计数器。复核无剩余确定发现。

### Spec

初评3项P2，其中任务包漏报与Standards重合：

1. `end_video_job`完成UPDATE影响0行仍返回Ok，使分镜/试片取消竞态误报成功。现在检查影响行数，已取消、过期代次和重复完成均失败；新增回归先失败后通过。
2. 音频首次下载后的缓存归档重试路径未通知。将通知与关闭音频窗口合并到现有`audio_completed`入口，两条成功归档路径复用；原有事务失败/缓存重试测试继续通过。
3. 同一发布任务再次导出新任务包被误去重。实际目录区分新产物，相同产物重复事件仍去重；新增前端回归并用Windows真实命令验证修复前漏报、修复后各次成功都提醒。

最终Spec无剩余确定发现。两个轴原始发现分别为1项和3项，合并为3个实际问题，均已修复。尚未验证的外部服务/设备范围列在文末。

提交前同步到远端v0.56.1（仅版本字段变化），语音改动无冲突恢复。`Completion::Audio`与其唯一Windows调用入口统一使用`cfg(windows)`，避免在macOS/Linux保留无构造调用的枚举项；没有关闭死代码规则。非Windows实际构建仍由跨平台流水线验证。

## 实现与验收标准

- 顶栏提供有名称及checked状态的语音总开关，默认关闭；开启时播报确认并显示试听按钮，本机保存选择。
- 文案/时长/按项调整、背景/插画、音频归档、字幕校对/内容核对、三尺寸封面导出、分镜/单镜、试片/视频、发布任务包、抖音准备和发布结果记录均从成功执行位置发送事件。
- 手动与自动流程共用成功入口，事件由应用级组件接收，不依赖当前任务页，也不扫描历史完成记录。
- 多项提醒用同一AudioContext时钟排队，不相互覆盖；同类型同任务/代次只播一次。关闭会停止当前和预约节点，并丢弃未完成的旧解码请求，重新开启不恢复旧队列。
- 提醒失败只显示提示，不修改任务状态或阻止后续创作；准备发布仅说「请到抖音窗口检查并确认发布」。
- 19段内置PCM WAV共3,886,152字节，包含18种完成提醒及开启试听。生成使用本机Microsoft Huihui Desktop，脚本及文本清单可复现；用户运行时无需系统TTS或联网。

## 修改范围与数据

新增`src/services/voiceReminders.ts`、`src/components/VoiceReminderToggle.vue`、`src-tauri/src/completion.rs`、音频资源/生成脚本与语音回归测试。App顶栏接入；Rust文本、图片、自动制作、音频下载、视频、抖音与发布命令在原成功点通知。CSP仅增加同源资源读取。

SQLite仍为schema23，无迁移、无新表或依赖；`shuyin.voice-reminders`为本机localStorage偏好，不属于业务备份。无模型调用、发布授权、原文件处理或已有成功校验的变更。未提交、推送或发布版本。

## 实际执行

| 命令 | 结果 |
| --- | --- |
| `./scripts/generate-voice-reminders.ps1` | 生成19个音频资源 |
| `npx vitest run tests/voice-reminders.test.ts tests/generation.test.ts tests/ui-refinement.test.ts tests/automation.test.ts` | 30项通过，包含11项语音回归 |
| `npm run lint` | 通过 |
| `npm run build` | 类型检查及Vite生产构建通过，19个WAV进入dist；现有大于500kB的JS分包警告仍存在 |
| `. ./.tools/env.ps1`后`cargo check --manifest-path src-tauri/Cargo.toml --locked` | 通过 |
| `. ./.tools/env.ps1`后`npm run check:rust` | fmt与Clippy（all-targets、禁止警告）通过 |
| `. ./.tools/env.ps1`后`npm run test:rust` | 262通过、2项按原设置忽略；无失败 |
| `npx vitest run --maxWorkers=2` | 33文件、461项全部通过 |
| `npx vitest run tests/voice-reminders.test.ts tests/ui-refinement.test.ts` | 顶栏自适应收尾后20项通过；随后lint及生产构建再次通过 |
| `git diff --check` | 通过 |
| `. ./.tools/env.ps1`后`npm run desktop:build -- --debug --no-bundle --config output/playwright/voice-reminders/tauri.verify.conf.json` | 独立标识的Windows debug应用构建成功；无安装包 |

评审修复后的最终检查：

| 命令 | 结果 |
| --- | --- |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked completion_requires_the_current_running_video_job` | 修复前1项失败，修复后1项通过 |
| `npx vitest run tests/voice-reminders.test.ts` | 12项通过 |
| `npx vitest run --maxWorkers=2` | 33文件、462项通过 |
| `npm run lint` | 通过 |
| `. ./.tools/env.ps1`后`npm run check:rust`及`npm run test:rust` | fmt/Clippy通过；263通过、2项按原配置忽略 |
| `. ./.tools/env.ps1`后`npm run desktop:build -- --debug --no-bundle --config output/playwright/voice-reminders/tauri.verify.conf.json` | 修复后三次问题的Windows debug程序构建通过，包含TypeScript与Vite生产构建 |
| Node24.22.2下`node --test tests/video-render.test.mjs` | 40项通过 |
| `npm run test:release` | 发布/音频脚本78通过、1项macOS专用跳过 |
| Node24.22.2下`node --test tests/codex-queue.test.mjs` | 19项通过 |

默认PATH的Node24.11.0触发项目已有Windows运行时拒绝规则，首轮视频检查32通过/8失败；队列首轮18通过/1失败。未更改业务代码、测试或系统安装，改用本机已存在的`D:/MyWork/software/shuyin-video-node/v24.22.2/node.exe`及其进程PATH重跑，视频40项与队列19项均通过。

首次新增测试发现浮点精度断言和测试运行器的import.meta.url文件定位不适用，已修正为容差比较与仓库路径；未修改业务逻辑以迁就测试。首次lint发现测试模拟器的this别名，已按现有规则改为实例列表。初次并发全量Vitest有一项既有更新页测试超过5秒，此时Rust也在全量编译；未调整测试或超时，之后限制两worker重跑全量461项通过。

## 浏览器验证

Playwright打开本地Vite页面，使用真实AudioContext和内置WAV；开启语音、检查偏好、模拟两个完成事件和重复事件，音频上下文为running，三次实际播放（含开启试听）按时钟顺序调度，重复事件未新增播放。关闭时两条当前/预约语音停止，关闭后的事件没有播放。

800×900开启试听后的截图发现顶栏面包屑被挤成竖排，已让顶栏按钮保持宽度、面包屑单行省略，窄窗隐藏存储徽标并提前收起冗余快捷导航。最终书架截图`output/playwright/voice-reminders/voice-dark-800.png`确认标题、开关、试听和主题按钮均正常。切换浅色主题并进入模型设置后，注入新视频完成事件，真实AudioContext仍为running，成功调度3.788秒语音；顶栏高度64、面包屑高度20，未挤压换行。模型设置页面自身的窄窗内容溢出不属于本次语音改动，未扩大范围处理。

此处完成事件为显式测试输入，证明界面和真实Web Audio播放链路，不代表远程AI或真实账号整链验收。

## Windows首次运行验证受阻记录

已用独立identifier `com.shuyin.voice-reminders-20261110`和隐藏主窗口构建，拟用本机18610调试端口验证真实Rust事件及后台声音。随后复制并通过`Start-Process -WindowStyle Hidden`启动隔离程序的命令被自动审批检查拦截，工具仅返回`blocked by policy`；该命令没有执行。没有通过其他启动方式绕过，未将原生事件投递或Windows后台播报记为通过。没有打开或修改正式业务数据库。

测试程序及配置位于D盘仓库的`output/playwright/voice-reminders`与`src-tauri/target/debug`；若后续启动，SQLite会按Tauri规则使用C盘LocalAppData下的独立标识目录，WebView缓存配置在本轮D盘输出目录。

## Windows评审补验

用户追加评审测试后，通过`Start-Process -WindowStyle Hidden`成功启动上述独立实例。测试数据只在`com.shuyin.voice-reminders-20261110`的独立SQLite与D盘本轮workspace创建；未打开正式库、远程账号或模型，关闭独立实例自动更新，未安装下载的更新。

Playwright CLI通过`attach --cdp=http://127.0.0.1:18610`连接实际WebView2，执行真实`save_project`、`save_cover_version`、`link_asset`、`save_publish_task`与`prepare_publish_task`，没有模拟Tauri事件或替换音频播放。只包装AudioContext节点以记录start/stop/ended，不改变播放行为。

- 修复前旧程序：开启、横/竖封面导出、首次任务包导出累计4次播放；第二个真实任务包目录已创建但无第5次播放，回归断言失败，现场确认漏报。
- 修复后程序：同一脚本累计7次播放，包含两次独立导出、关闭期间导出静音、重新开启后的新导出；失效PNG触发原Rust校验失败，没有成功语音；关闭停止所有当前及预约节点。事件到达时页面已切到模型设置，音频上下文running，离开原任务页仍生效。
- 正常退出测试应用后替换新构建，开关保留开启状态；脚本先关闭再开启重置本次播放队列，没有重播历史任务。
- 用Win32按标题及进程ID验证测试主窗口，再最小化；结束时`IsIconic=true`。最小化期间真实任务包成功事件仍按序调度4.353秒语音、上下文running，没有播放错误。WebView2的document.visibilityState仍返回visible，因此以原生窗口状态证明最小化，不用浏览器属性替代原生证据。
- 进一步等待自然播放结束：4.993秒开启语音与4.353秒任务完成语音分别触发ended，上下文时钟从701.051持续推进至710.552，主窗口仍为最小化；此轮没有提前stop。验证实际音频渲染链路完成，但不将该记录描述为人工听感验收。

脚本及结果保存在`output/playwright/voice-reminders/review-*.mjs`、`review-window.ps1`和`native-review-results.txt`。替换实例时一次结束进程/复制/启动组合命令被自动审批拒绝；随后通过应用原有destroy命令正常关闭，再复制和启动均成功，未关闭正式应用。

## 待验范围

Windows真实本地事件与最小化Web Audio链路已验证；正式安装包升级、macOS/Linux设备、用户扬声器主观听感以及真实模型/NotebookLM/抖音账号整链尚未在本轮执行。音频归档重试以源码复核及已有数据库事务回归覆盖，没有运行真实NotebookLM。系统静音、无可用输出设备或退出应用时不保证有声提醒。下一步在正式候选实际创作中开启提醒，确认用户设备的主观声音效果。
