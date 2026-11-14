# 合集名称下拉与管理

2026-11-15。兼容的用户功能增强，按 feat/minor 判断，版本仍交发布流水线处理。本轮不手动升版本或替换正式安装版。

## 实现与范围

书籍资料的自动制作设置保留「不加入合集 / 加入指定合集」，将下方「合集完整名称」换为可输入下拉。可以直接选择已保存名称，输入时搜索，回车保存并选中。去首尾空白并合并重复值，中文输入法确认不触发保存。每行可编辑、删除；编辑当前选项同步未启动表单，删除只移除候选，保留当前输入。键盘方向键进入列表，Escape关闭并返回输入。

模型设置的原 `ModelOptionInput.vue` 上移为 `src/components/SavedOptionInput.vue`，两个业务入口共用同一实现，说明文案由调用方传入。无新增依赖。复核本地 Reka UI 2.11.0 `PopoverRoot.vue`、`PopoverAnchor.vue`、`PopoverContent.vue` 与 `PopoverContentImpl.vue` 的公开类型、事件及内嵌文档；使用原有受控浮层、锚点、焦点与边界避让方式。

`publishing.ts` → `publish_commands.rs` → `Database` 两条窄命令读写 `app_settings.douyin_collection_options` 的 JSON 字符串数组；仅在原主窗口发布权限组登记命令。Rust共用自动任务启动的名称规范化，整组校验1—200字符并去重后单条UPSERT。读写失败保留原输入、列表及数据库值，读取失败提供重试，保存期间阻止重复保存与启动。

候选跨书籍共用，不表示账号身份或远端合集清单。候选维护不写 `publish_tasks.douyin_collection` 或 `automation_runs.input_json`，任务仍在启动时冻结实际名称与确认账号，上传后沿用官方唯一同名匹配和回读。新增/编辑/删除候选不创建、修改或删除抖音上的真实合集。

## 数据库

仍为schema25，无表结构或SQL迁移。新增普通设置键 `douyin_collection_options`，缺键返回空列表，删除全部候选保留空数组。没有历史数据导入或正式库改写，候选随原SQLite数据库和完整备份机制保留。

## 实际验证

| 命令 / 操作 | 结果 |
| --- | --- |
| `npx vitest run tests/model-options.test.ts tests/automation.test.ts`（最初接入） | 19项通过 |
| `npx vitest run tests/model-options.test.ts tests/automation.test.ts tests/publishing.test.ts tests/douyin.test.ts`（新增两项合集场景后） | 79项通过 |
| `npm run lint` | 首次4条Vue模板换行警告，修正后通过，无关闭规则 |
| `npx vitest run tests/automation.test.ts tests/model-options.test.ts`（新增保存期间互斥场景后） | 22项通过 |
| `npm run typecheck` | 通过 |
| `npm run build` | TypeScript与Vite生产构建通过；保留现有大于500kB分块提示及本机npm旧配置提示 |
| `. ./.tools/env.ps1` 后 `cargo test --manifest-path src-tauri/Cargo.toml --locked collection_ --quiet` | 10项通过 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked --lib --quiet` | 308项通过，2项原有忽略 |
| `npm run check:rust` | 首次新增测试两处不必要clone被Clippy指出；改用借用后fmt/Clippy全目标通过 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked collection_options --quiet`（测试断言清理后） | 1项通过 |
| `npx vitest run` | 537项通过；更新页与语音提醒各1项因5000ms限制超时，当时同时执行Rust与构建 |
| `npx vitest run --maxWorkers=2` | 37个测试文件、539项全部通过；未调整测试、超时、断言或项目并发配置 |
| `git diff --check` | 通过 |
| `npm run dev -- --port 1431` + `ego-browser nodejs` 隔离界面核验 | 9项交互与6组布局检查通过，见下方边界 |

Rust新增回归覆盖：缺省空候选、去空白和重复值、编辑删除不改变已保存任务、删空后备份恢复和数据库重开、中文200字符接受/201字符拒绝、任一无效值拒绝整组且保留原候选。原自动合集冻结、任务绑定事务、账号和发布回归继续通过。

浏览器通过 `output/collection-options/fixture.html` 挂载真实 `AutomationSetup`，使用隔离IPC夹具，不连接正式数据库或网站。候选在该浏览器夹具中用独立localStorage键模拟保存，真实SQLite持久化由上述Rust回归验证。先检查并修正测试脚本对弹层关闭的假设后，逐项确认了：已有项选择、回车保存/去空白、重复回车不写入、编辑同步、删除保留输入、搜索与方向键、Escape返回焦点、维护候选不启动任务、页面重开读取候选。记录为 `output/collection-options/interactions.json`。

深浅两主题分别在1440、800、375像素宽度检查：无水平滚动，下拉与输入等宽，浮层在可视范围，编辑/删除始终可操作。记录在 `output/collection-options/layouts.json` 及同目录6张PNG，已人工查看深色1440和浅色375截图。窄窗口的长名称省略并保留完整title，浮层空间不足时自动向上展开。

## 验收与剩余边界

| 标准 | 对应证据 |
| --- | --- |
| 输入回车保存并选中，下次可直接选择 | Vue组件/集成回归、浏览器交互、SQLite重开回归 |
| 搜索、编辑、删除、重复名及中文输入法可用 | 共用组件6项原回归和合集3项新增集成回归；浏览器操作 |
| 失败不丢输入或列表，可重试 | Vue存储失败与重读回归、Rust无效值不写入 |
| 重启、数据库备份恢复保留 | Rust独立SQLite测试，空数组及非空名称重开 |
| 不改已启动任务或远端合集 | 数据库任务隔离断言、保留原冻结/发布调用链；浏览器夹具拒绝其他命令 |
| 深浅主题、窄窗口与键盘可用 | 6组布局和9项浏览器交互；原模型键盘回归 |

本轮为本地源码、Rust、前端与浏览器隔离验证，未执行Windows原生WebView2/真实IPC人工验收，未构建安装包，未修改或替换 `D:\书引 AI` 正式安装版，未操作真实抖音账号或最终发布。未运行与本轮UI/SQLite修改无关的FFmpeg视频、Node网页打包及队列专项套件，未将上述结果表述为完整 `npm test` 通过。

下一步在包含本轮源码的桌面构建中选择一个当前账号已有合集，回车保存、重开复选后按原流程核对官方唯一同名匹配；最终发布仍由用户检查确认。
