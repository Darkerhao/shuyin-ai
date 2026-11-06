# 抖音保存权限与定时发布验收（2026-11-06）

## 实现与使用

发布准备增加「抖音发布设置」卡，提供「允许保存视频」与「立即发布 / 定时发布」。定时输入按北京时间精确到分钟，两个选项随本条发布任务、队列、快照、导出和备份保留。旧任务与新任务均默认允许保存、立即发布；已发布任务只读。

辅助发布在封面、合集之后设置官方保存权限与发布时间，并回读及最终复核。已选中相同选项不重复点击；缺失、重复、禁用、弹窗阻挡、时间被官方调整或回读不符会停止在发布设置步骤，允许明确人工核对或重试。已经读到的不同值不能通过人工确认跳过。没有增加权限、私有接口、后台计时器或最终发布点击。

按用户追加要求，定时发布和记录实际发布时间均改用 `DateTimePicker`：中文 Reka 日历浮层、小时/分钟 Select、清空与确定，替代发布页全部原生日期控件。定时统一北京时间，实际发布记录保留设备当地时区；两处共用实现。显式声明原锁内已有的 `@internationalized/date` 3.12.4 直接依赖，复用 Reka 日期类型和公开 API，不变更已有包版本；依据已核对[官方 DatePicker 文档](https://reka-ui.com/docs/components/date-picker)、[CalendarDateTime 文档](https://react-spectrum.adobe.com/internationalized/date/CalendarDateTime.html)和实际安装源码。

无效、非北京时间格式及过期时间不能保存、重新导出或上传，失败保留原任务。定时作品到点以后仍可查看原任务、人工记录实际发布结果；不会因时间已过而丢失记录入口，也不会把提交定时发布写为已发布。

## 数据库与兼容性

- 新增 `src-tauri/migrations/0018_publish_settings.sql`，schema 从 v17 升至 v18。
- `publish_tasks.allow_download` 为布尔整数，默认 1；`scheduled_at` 为可空文本，`NULL` 表示立即发布，非空为 `YYYY-MM-DDTHH:mm:00+08:00`。
- 保存设置复用原事务，查询、上传和任务包使用同一份 `PublishInput`。
- 旧任务包 manifest 缺失字段按允许保存、立即发布解读，原默认文本保持兼容；定制设置的任务包增加 `publish-settings.txt`。
- 测试只使用隔离临时数据库，未打开或升级用户正式数据库。v17 迁移、旧备份恢复、重开、备份保留设置和旧默认任务包有效性均已验证。

## 页面适配依据

使用仓库留存的官方页面 `output/playwright/m6/page.html` 核对保存权限及发布时间的区域与单选结构，再于 2026-11-06 下载与该页面对应的公开静态构建核对实现：

- [发布时间组件](https://lf-fe-creator.douyinstatic.com/obj/douyn-creator-scm-cdn/douyin-creator-content-new/micro/static/js/async/2699.5f441ead.js)：立即/定时单选、`dateTime` 日期框、「日期和时间」占位及 `yyyy-MM-dd HH:mm` 显示格式。
- [发布表单](https://lf-fe-creator.douyinstatic.com/obj/douyn-creator-scm-cdn/douyin-creator-content-new/micro/static/js/async/6770.0977d9a9.js)：保存权限字段及收集逻辑。
- [时间范围工具](https://lf-fe-creator.douyinstatic.com/obj/douyn-creator-scm-cdn/douyin-creator-content-new/micro/static/js/async/1025.94b3a8d9.js)：范围读取账号 `user_timer_version` 配置，不能把某个账号的固定上限套用到全部账号。
- [Semi 组件实现](https://lf-fe-creator.douyinstatic.com/obj/douyn-creator-scm-cdn/douyin-creator-content-new/micro/static/js/async/lib-semi2.583e5bad.js)：Input 的 `onEnterPress` 来自 `keypress`，DatePicker 以输入值完成解析；适配器使用该事件提交并在移开焦点后回读。

留存 DOM 的 native checkbox `checked` 与外层 `data-checked` 存在不一致，按官方已选标签读取。没有读取账号 Cookie、密码或存储；本轮未检查已登录真实账号的最新现场 DOM，因此上述依据不等同于现场账号验收。

## 实际验证

Rust 命令先执行 `. ./.tools/env.ps1`，`TEMP`、`TMP`、`SQLITE_TMPDIR` 指向 `output/douyin-settings/temp`。完整 npm 测试使用已有的 `D:/MyWork/software/shuyin-video-node/v24.22.2` 临时置于本次进程 PATH 首位，并设置 `VITEST_MAX_WORKERS=2`，不修改系统 Node 配置。

| 命令 | 实际结果 |
| --- | --- |
| `npx vitest run tests/douyin-editor.test.ts -t 'save permissions\|matching options\|rejected scheduled' --maxWorkers 2`（新增适配动作前） | 新增 3 项失败，旧适配器返回“不支持的页面操作”，确认回归覆盖新增行为 |
| `npx vitest run tests/douyin-editor.test.ts tests/publishing.test.ts tests/douyin.test.ts tests/publish-queue.test.ts --maxWorkers 2` | 当时的 84 项通过；其后追加“到点后仍可人工记录”的回归包含在下述完整测试中 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked publish_settings` | 5 项通过，包含精确回读、设置持久化/备份、无效与过期时间、到点后人工记录、旧任务包及 v17 迁移 |
| `npm test`（Node 24.22.2、并发 2） | 全部通过：前端 413、视频 39、发布脚本 78、队列 19；另 1 项 macOS 专属检查按原设置跳过 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked` | 219 通过、2 项既有专项忽略，无失败 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | 通过 |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets -- -D warnings` | 通过 |
| `npm run lint` | 通过，零警告 |
| `npm run typecheck` | 通过；生产构建时也再次执行 |
| `npm run build` | 通过，保留原有大分块提示 |
| `git -c core.safecrlf=false diff --check` | 通过 |
| `npx --yes --package @playwright/cli playwright-cli -s=publish-settings run-code --filename output/playwright/douyin-settings/setup.cjs`，随后运行 `check.cjs` | Chrome 真实 Vue/组件，隔离内存 IPC：保存权限、定时选择、北京时间、刷新回显、过期拦截及切回立即发布通过。浅/深主题各 1440、800、375px 无横向溢出，页面错误 0；无上传或发布记录调用 |

最初默认并发运行的全量前端检查有一项更新页测试触发 5 秒超时；未改测试或超时配置，限制并发后全量通过。本机 PATH 默认 Node 24.11.0 被项目既有视频版本保护拒绝，导致 6 项视频夹具缺少产物；按项目已有记录切换现成 Node 24.22.2 重跑，未修改视频源码。

浏览器证据位于 `output/playwright/douyin-settings/check-result.log`、`dark-{1440,800,375}.png` 和 `light-{1440,800,375}.png`，属于本地忽略输出。截图发现的空错误样式已改为仅在有错误时显示，并重跑浏览器验证。

## 提交前独立审核与最终复验

工程规范审核（Standards）：发现 1 项会话复用问题，规范性要求是设置按实际调用链完整填写和核对。原 `start_douyin_task` 在读取当前快照之前就返回相同任务的旧 `review`，而准备完成状态允许保存修改，导致界面新设置与官方页旧值不一致。修复为先取最新快照，再由 `resumable_session` 检查绑定快照与停止状态，设置变化不再复用旧就绪结果；准备完成新增停止入口。回归先失败后通过，独立增量复核无新增 P1/P2。

需求审核（Spec）：发现 2 项，包括上述旧会话复用和部分控件无法读取时的人工确认缺口。原完整读取失败会掩盖另一项可读取的冲突；现分别观察可空的保存权限、立即/定时模式和时间，自动完成要求全部精确匹配，人工确认及最终复核仍拒绝任意可读冲突，仅不可读取部分交由用户明确核对。部分失读与停止入口新增 3 项前端回归先失败后通过。独立增量复核确认两项已闭环、UI 时间控件符合最新要求，无新增可行动问题。

本轮追加的实际验证（取代上表较早的测试计数）：

| 命令 | 结果 |
| --- | --- |
| `npx vitest run --maxWorkers 2`（Node 24.22.2） | 30 个测试文件、420 项通过，包含 3 项真实 UI 组件日历/时分/清空/禁用回归 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked` | 221 项通过、2 项既有专项忽略；新增旧 review 快照和部分可读冲突回归 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | 通过 |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets -- -D warnings` | 通过 |
| `npm run lint` | 通过 |
| `npm run build` | TypeScript 与生产构建通过，保留原有大分块提示 |
| `npm ls @internationalized/date --depth 1` | 直接依赖与 Reka 依赖共用已安装 3.12.4，无重复版本 |
| `npx --yes --package @playwright/cli playwright-cli -s=publish-ui-review run-code --filename output/playwright/douyin-settings/setup.cjs` 后运行 `check-ui.cjs` | 非原生 UI 日历选择、时分下拉、保存北京时间、刷新回显、日期切换保留时间、清空/过期拦截、Escape 关闭并回焦通过；浅/深主题各 1440/800/375px 浮层在视口内，实际发布记录模态弹窗中的日期/时分选择及清空校验通过；页面异常 0，无上传/发布记录调用 |
| `git -c core.safecrlf=false diff --check` | 通过，提交前另核对暂存差异 |

浏览器最终证据为 `output/playwright/douyin-settings/ui-check-result.log`、`ui-{dark,light}-{1440,800,375}.png` 和 `ui-record-popup.png`，均为忽略的本地输出。首次检查复现日历高度超出浮层可用空间，使用 Reka 公布的可用高度变量限制并允许滚动后复验通过；单测还确认已保存日期决定初始月份。Rust 有一次增量缓存目录写入权限提示，但测试及 Clippy 均以成功状态退出，未修改编译检查规则。

视频 39 项、发布脚本 78 项（另 1 项 macOS 跳过）及队列 19 项已在同一任务上一轮 `npm test` 通过；本轮审核修复未变更对应源码，不重复执行这些耗时脚本。

## 验收对应与待验范围

| 验收标准 | 结果 |
| --- | --- |
| 允许/不允许保存视频可以选择并随任务保留 | 前端、SQLite 重开、快照及备份回归通过 |
| 立即/定时可切换，分钟与北京时间准确 | 前端、Rust 校验及浏览器通过；切回立即发布清空定时时间 |
| 非法日期、过期时间及失败不覆盖原稿 | 前端、Rust、浏览器通过 |
| 设置填写后核对，拒绝不同的回读结果 | DOM 夹具、Rust 精确比较、前端人工决定入口通过 |
| 旧任务、备份及任务包可继续使用 | v17→v18、恢复、重开及旧 manifest 回归通过 |
| 定时实际发布后仍可人工记录 | 前端入口及 Rust 发布记录事务通过；不自动标记成功 |
| 两种主题、窄窗口操作 | Chrome 六组布局通过，已查看截图 |
| 最新真实账号控件、定时提交和新安装包 | 本轮未执行，不能据本地夹具宣称现场已通过 |

## 修改文件与后续

业务文件：`src/types/publishing.ts`、`src/services/douyin.ts`、`src/modules/publishing/PublishingPage.vue`、`src/modules/publishing/DouyinVerification.vue`、`src/components/ui/date-time-picker/DateTimePicker.vue` 及导出入口、`src-tauri/src/database.rs`、`src-tauri/src/database/publishing.rs`、`src-tauri/src/douyin.rs`、`src-tauri/src/douyin_flow.rs`、`src-tauri/src/douyin_editor.js`，以及上述 v18 迁移。依赖声明：`package.json` 与 `package-lock.json`，仅直接声明既有锁内日期库。

测试文件：`tests/date-time-picker.test.ts`、`tests/publishing.test.ts`、`tests/douyin-editor.test.ts`、`tests/douyin.test.ts`、`tests/publish-queue.test.ts`、`src-tauri/src/database/m4_tests.rs`。同步更新 `docs/PRD.md`、`TECHNICAL-DESIGN.md`、`DATABASE.md`、`AI-RULES.md`、`DEVELOPMENT-PLAN.md`、`USER-GUIDE.md` 及本记录。

下一步在包含本次源码的 Windows 桌面构建中，用真实账号检查允许/不允许保存、立即/定时切换、当前账号可选范围和实际回读，再由用户决定是否在抖音提交作品。新安装升级及 macOS/Linux 实机尚未验收。本次属于 `feat` / minor，按用户后续授权在 dev 生成中文本地提交，版本交给既有流水线；不推送或发布。
