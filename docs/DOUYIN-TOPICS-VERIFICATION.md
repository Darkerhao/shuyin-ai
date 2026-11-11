# 抖音话题确认修复（2026-11-11）

## 根因与修复

用户截图中 `#读书 #富兰克林传 #沃尔特艾萨克森 #美国历史 #公共精神` 已作为文字填入，但需要手动选择建议后才变成蓝色标签。原调用链是 `prepareText` → `Input.insertText` → `read_editor` → `verify_editor`：一次性填入正文及标签，随后只比较文字，没有选择官方话题，也不读取话题节点，因此纯文本被当作填写成功。

当前官方编辑器使用 editor-kit。2026-11-11只读核对官方页面当前加载的运行包，观察空白编辑页中输入 `#读书` 出现同名建议，点击建议生成位于 `contenteditable="false"` 内的 `[data-mention="#"]` 节点；蓝底由官方节点样式产生。源码依据：

- [当前运行时](https://lf-fe-creator.douyinstatic.com/obj/douyn-creator-scm-cdn/douyin-creator-content-new/micro/static/js/builder-runtime.3c37f6a8.js)。
- [发布编辑器话题组件](https://lf-fe-creator.douyinstatic.com/obj/douyn-creator-scm-cdn/douyin-creator-content-new/micro/static/js/async/2320.ab744159.js)：光标变化触发建议，同名条目 `onClick` 执行 `insert-mention-#`，写入名称、官方话题ID和节点，额外插入空格。
- [当前编辑器基础库](https://lf-fe-creator.douyinstatic.com/obj/douyn-creator-scm-cdn/douyin-creator-content-new/micro/static/js/async/lib-editor-kit.1ac0c1fc.js)：实际可编辑DOM与选区事件实现。

新增 `prepareTopic` / `selectTopic` 页面动作，逐项定位尚未确认的完整标签，选择唯一同名建议，宿主最多等待5秒并回读节点。已确认项不重选；部分完成后继续保留节点。不存在同名选项、同名重复、光标被移动、弹窗或超时均保留内容，复用原文案待处理和人工采用网页结果入口；增加「重新核对文案与话题」按钮，沿用原page决定继续同一任务，仅清除旧文案确认凭据，不重传视频。程序不调用编辑器私有对象、不写入仿造节点或CSS，也不自动选相近热词。

回读增加topics并纳入既有文案确认凭据和最终比较；自动通过必须同时满足文字一致及每个任务标签有对应节点，重复标签按出现次数检查。官方节点周围的非断行空格和重复间隔只在纯标签行统一，普通正文的内部空格和段落顺序仍校验。人工明确采用网页文案仍可接受用户调整后的内容与话题，后续检查这份实际结果。

## 修改范围

- 实现：`src-tauri/src/douyin_editor.js`、`src-tauri/src/douyin.rs`、`src-tauri/src/douyin_flow.rs`、`src/modules/publishing/DouyinVerification.vue`。
- 测试：`tests/douyin-editor.test.ts`、`tests/douyin.test.ts`及Rust模块内测试。
- 文档：`docs/PRD.md`、`docs/TECHNICAL-DESIGN.md`、`docs/AI-RULES.md`、`docs/DEVELOPMENT-PLAN.md`、本记录。
- 无数据库迁移、依赖、capability或版本号变更；属于fix/patch，未提交、推送或发布。

## 实际验证

Rust命令先执行 `. ./.tools/env.ps1`。

| 命令 | 结果 |
| --- | --- |
| `npx vitest run tests/douyin-editor.test.ts -t "typed hashtags\|unconfirmed whole\|partially confirmed"`（修复前） | 3项失败，缺少话题状态和确认动作 |
| `npx vitest run tests/douyin-editor.test.ts tests/douyin.test.ts tests/publishing.test.ts` | 最终91项通过，包含原任务重试与显式采用的区别 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked douyin:: -- --nocapture` | 19项通过 |
| `npm run lint` | 通过 |
| `npm run build` | TypeScript检查和Vite构建通过，保留约842kB主分块提示 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | 通过 |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets -- -D warnings` | 通过 |
| `git diff --check` | 通过 |
| `ego-browser nodejs -e "await import('file:///D:/MyWork/project/JIAZI-PROJECT/shuyin-ai/output/douyin-topics/verify-live-06.mjs')"` | 官方空白编辑页5项全部生成真实话题节点、继续检查均已确认；末尾截图因浏览器最小化失败，命令退出1 |
| `ego-browser nodejs -e "await import('file:///D:/MyWork/project/JIAZI-PROJECT/shuyin-ai/output/douyin-topics/verify-live-07.mjs')"` | 退出0；5项节点一致，继续操作DOM不变，临时文字清空、任务页关闭 |

本地忽略目录 `output/douyin-topics/live-result.json`保留官方DOM验证结果。官方验证通过相同的页面适配脚本及原生输入操作完成，页面没有上传视频；测试文字已清空。没有操作用户截图中的发布草稿或代替用户最终发布。

## 完成后评审测试（2026-11-11）

按用户要求，对相对 `c6d334a4902fcc065a18593d1ba05ca67f7d25b6` 的本轮未提交话题修复进行独立Standards/Spec两轴评审。只评本轮实现、测试与文档；没有提交、推送或操作用户正式数据。

Standards：PASS，0项确定问题，未提出需要整改的smell。核对话题定位与选择、宿主等待和回读、重试与人工采用、最终核验的实际调用链，确认复用原会话、停止检查、操作锁及权限限制，符合最小增量与真实验证要求。

Spec首轮发现1项P2：`prepareTopic`定位 `#读书 #读书分享` 中首项后，用户将光标移至第二项的 `#读书|分享`，原 `selectTopic`只校验光标前缀，仍会点击 `#读书` 建议，可能拆开长标签。以实际适配器增加回归，先复现 `selected:true` 失败；最小修复为选择前同时检查光标后是空白、零宽结束符或文本结束，错误位置直接拒绝且不点击建议。

Spec复审：PASS，未解决问题0项。独立评审执行新增光标移动回归，1项通过、34项跳过，确认P2已关闭；其他需求没有确定的缺失或越界。最终Standards为0项问题，Spec首轮1项P2已修复、遗留0项。本轮复审未进行新的官方页面或桌面整链操作。

评审阶段补跑：

| 命令 | 结果 |
| --- | --- |
| `npx vitest run --maxWorkers=2` | 全量34个测试文件、494项通过 |
| `. ./.tools/env.ps1` 后执行 `cargo test --manifest-path src-tauri/Cargo.toml --locked` | 全量273项通过、2项按原配置忽略；包含数据库迁移与备份回归 |
| `npx vitest run tests/douyin-editor.test.ts -t "cursor moves inside"`（P2修复前） | 1项失败，误选建议的回归成立 |
| `npx vitest run tests/douyin-editor.test.ts -t "does not select a topic after the cursor moves inside a longer hashtag"`（独立复审） | 1项通过、34项跳过 |
| `npx vitest run tests/douyin-editor.test.ts tests/douyin.test.ts tests/publishing.test.ts`（P2修复后） | 92项通过 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked douyin:: -- --nocapture`（P2修复后） | 19项通过 |
| `npm run lint`、`npm run build`（P2修复后） | lint、TypeScript检查和Vite构建通过；保留原约842kB主分块提示 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check`、`cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets -- -D warnings`（P2修复后） | 均通过 |
| `git diff --check` | 通过 |

两项忽略分别为系统凭据存储实测及需指定Node路径的进程树终止测试。全量结果为P2补充修复之前的快照，修复后另执行相关回归与工程检查。本轮未重复执行无关视频渲染、发布打包和队列Node测试，也未把这些测试算入全量Vitest或Rust结果。

## 验收与下一步

- 纯文字标签不能误判为已确认：本地回归通过。
- 精确选择、不误选相近话题、保留正文：本地回归及官方5项操作通过。
- 已确认项继续时不重复或改写：本地回归及官方DOM比较通过。
- 缺少、重复建议和弹窗等异常保留内容：本地夹具通过；实际网络超时、原生宿主停止/恢复整链仍需桌面验收。
- 新安装包、用户当前成片上传后整链和最终发布尚未验证。下一步在包含修复的Windows构建中，核对本次5个标签、封面及发布设置，再由用户亲自提交。
