# 自动制作账号与合集：评审测试

日期：2026-11-11。评审基线：`a9c337ad0aed95f2347e28095d18ee4fb99831ad`（本轮开始时的HEAD），使用`git diff a9c337ad0aed95f2347e28095d18ee4fb99831ad --`检查工作区改动。没有新增提交或推送。

需求：用户在自动制作开始时确定账号和目标合集，同一任务后续正常步骤沿用，不反复要求确认。官方最终发布继续人工完成；真实账号变化、窗口重开或无法验证官方选项时保留已有成果并提示处理。

## Standards：工程规范

独立首审及修复后复审均为0项确定问题。

依据根目录AGENTS.md、AI-RULES、TECHNICAL-DESIGN和DATABASE的对应规则检查实际调用链：复用原账号会话、合集适配器、发布任务和事务，未增加第二套状态存储、上传执行器、依赖或权限。昵称不作为稳定身份，普通手动任务不套用自动任务的合集替换授权。未关闭检查规则或修改测试超时。

## Spec：需求符合性

首审发现1项P2，修复后独立复审通过，剩余0项。

**P2：官方页已选同名合集时，首次使用前置名称跳过唯一性校验。**

- 触发：开始时填写“读书”；官方页当前选择也显示“读书”，但选项列表有两条同名合集。
- 原因：`drive`在`matches_choice`为true时直接完成，`Collections::validate_choice`也先接受已选名称，未验证列表是否唯一。
- 影响：与PRD和AI-RULES的“官方列表唯一精确匹配”不符，无法证明使用了用户想要的那条合集。
- 修复：自动任务首次使用命名合集时，即使当前已选同名，也读取列表并严格验证唯一匹配。使用原`Session.completed`区分首次校验与同任务恢复；已完成且回读未变时不重复加载或选择。不加入合集、普通手动任务和明确人工采用结果沿用原规则。

回归`initial_automatic_collection_checks_uniqueness_even_when_already_selected`修复前实际失败，修复后通过；同时覆盖唯一选项通过、列表未加载拒绝、已核对后列表收起仍可回读。

## 补充测试与验收对应

| 关注点 | 本轮证据 |
| --- | --- |
| 首次同名合集必须唯一 | 新增Rust回归先失败后通过；复审检查`drive`的真实加载与验证调用 |
| 一次确认后沿用 | 原会话回归保留，相关前端与Rust全量通过；没有增加正常流程确认按钮 |
| 账号确认失败后可恢复 | 新增组件回归验证先阻止启动，再重新读取并确认新账号；合集输入保留 |
| 重开后的任务展示 | 新增组件回归读取已暂停任务的账号和合集，不把持久昵称显示为新会话已确认 |
| 不加入合集 | 启动回归断言`collectionName: null`，Rust验证无合集不依赖选项列表 |
| 合集写入失败不能留半条发布任务 | 新增SQLite trigger故障注入，验证任务、流程关联和发布策略一起回滚；移除故障后原流程可保存 |
| 原业务、迁移和备份 | Rust全量及前端/视频/发布/音频/队列脚本通过；不自动执行最终发布 |

## 实际验证

前端与脚本使用本机已有的`D:/MyWork/software/shuyin-video-node/v24.22.2`，仅在本次PowerShell进程置于PATH首位，`VITEST_MAX_WORKERS=4`。TEMP/TMP使用仓库`output/automation-setup-review-20261111/temp`。Rust命令先加载`. ./.tools/env.ps1`，SQLITE_TMPDIR也指向该隔离目录。

| 实际命令 | 结果 |
| --- | --- |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked initial_automatic_collection_checks_uniqueness_even_when_already_selected --quiet`（修复前） | 1项失败，已选同名时错误接受重复选项 |
| `npx vitest run tests/automation.test.ts tests/automation-tasks.test.ts tests/douyin.test.ts tests/douyin-editor.test.ts tests/publishing.test.ts` | 5文件110项通过 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked --quiet`（修复后） | 279项通过、2项既有专项忽略，包含迁移、备份及新增故障注入 |
| `npm test` | 前端500项、视频40项、发布/音频脚本78项、队列19项通过；1项macOS检查按原配置跳过 |
| `npm run lint` | 通过，0条ESLint警告 |
| `npm run build` | TypeScript及Vite构建通过，保留既有大于500kB分块提示 |
| `npm run check:rust` | fmt及Clippy全目标通过 |
| `git diff --check` | 通过 |

本轮日志位于忽略目录`output/automation-setup-review-20261111/`。两位评审仅进行了独立只读审查，实跑测试由主任务执行；不将复审结论当作额外的运行时测试。

## 修改与边界

本轮评审修复涉及`src-tauri/src/douyin_flow.rs`，新增/补充测试位于同文件、`src-tauri/src/database/automation_tests.rs`及`tests/automation.test.ts`。同步更新`docs/TECHNICAL-DESIGN.md`、`docs/DEVELOPMENT-PLAN.md`及本记录，其余上一轮的账号与合集改动继续保留。

无新增SQL迁移，数据库仍为v23。无版本、依赖、权限、提交或推送变更。首轮1项P2已关闭，当前没有已确认的遗留评审问题。

本轮未重跑浏览器视觉检查、Windows原生WebView2整链、安装升级、真实抖音账号/合集/视频上传或最终发布；上一轮的隔离浏览器证据仅属于上一轮范围。下一步在包含本次改动的Windows桌面构建里核对真实账号和现有合集，最后由用户亲自检查并发布。
