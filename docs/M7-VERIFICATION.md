# M7 日常发布任务验收

日期：2026-10-26。实现、回归和独立 Windows 安装版核心验收完成。只验证本地任务处理与归档；没有代点抖音最终发布，没有提交或发布代码。

## 实现与文件

- 书架增加「发布任务」入口；`PublishQueuePage.vue` 汇总跨书待发布、草稿和已发布，按创建先后排列，可搜索书名/标题、查看缺项和复制作品链接。已有数据在刷新失败时保留，可重试；浏览器预览不访问本地任务。
- `list_publish_queue` 关联书名并逐项复用原有checked校验，失效任务包退回草稿。`PublishQueueItem` 直接扩展/flatten原PublishTask，无队列表、缓存状态或新依赖。
- 列表携带书籍和task ID进入PublishingPage，精确选择已保存任务及文案来源。无效/跨书ID报错，不改选另一篇；切换query沿用原有离页保护后再加载。新任务仍明确选择来源，不隐式复用另一篇或最新文案。
- 人工确认成功时先完成原有数据库事务，再仅释放该任务的活动上传快照，回列表继续下一篇。失败不释放，无关任务不释放；列表、记录查看和下一篇入口均不自动上传。

修改位置：`src/modules/publishing/{PublishQueuePage,PublishingPage}.vue`、`src/modules/projects/ProjectsPage.vue`、`src/App.vue`、`src/app/router/index.ts`、`src/{services,types}/publishing.ts`；Rust的`database/publishing.rs`、`publish_commands.rs`、`douyin.rs`、`lib.rs`、`build.rs`及发布权限；`tests/{publish-queue,publishing}.test.ts`、Rust的`database/m4_tests.rs`和douyin测试。产品、技术、数据库、使用说明、开发计划与规划文件同步。

数据库保持schema v9，无迁移。来源仍是已有writing_version_id，结果仍是publish_records；Windows原生辅助上传范围沿用M6，不增加官方API或抓取路线。

## 独立安装验收

使用M6的独立identifier与D盘安装目录，从0.14.3升级到临时0.14.4，NSIS退出0，正式版本清单仍0.14.0。M6官方测试草稿已暂存；M7不打开官方窗口。用户原开发实例的人工合集/章节草稿保留。

创建两本明确命名为「M7验收 · 模拟作品」的测试书籍、各自文案来源和素材引用；第一本另建更新的草稿，以验证列表可以准确打开较早的就绪任务。原M6真实素材任务始终ready、无发布记录。

| 验收标准 | 实际结果 |
| --- | --- |
| 跨书待发布列表 | 安装版主窗口进入列表，搜索M7显示两篇就绪作品；待发布/草稿/已发布计数与SQLite一致，布局截图已检查 |
| 准确选择任务 | 第一册有更新草稿，列表仍打开指定较早ready任务；标题、文案来源ID、视频素材ID一致 |
| 记住内容来源 | 两册各自保存writing_version_id，入口与重启后均保留；没有自动替换成最新文案 |
| 逐篇人工处理 | 仅对第一篇专用模拟任务，通过确认表单写本地模拟记录，再回列表打开第二册；第二册保持ready，不自动上传 |
| 链接归档 | 模拟链接使用example.com并明确备注“仅安装验收模拟记录：未在抖音发布”；已发布列表显示，复制后Windows剪贴板比对一致 |
| 重启持久化 | 正常销毁隔离主窗口，重新启动后全局任务逐字段一致，来源与模拟记录保留 |
| 失效与备份 | Rust回归删除第一册临时包内图后，队列将其退回draft；第二册记录保持，备份恢复后队列一致 |
| 失败保护 | 组件回归覆盖刷新失败保留列表、重试、无效任务拒绝、浏览器预览不访问数据，以及M6已有的编辑/导航/确认锁定 |
| 原任务隔离 | M6任务逐字段不变、仍ready；M7未打开官方窗口，未在平台发布。正式库及源视频最终哈希复核另存本地证据 |

本地模拟published用于验收SQLite和UI，不代表任何真实平台作品。未用该记录证明M6最终发布，也未对用户真实任务写虚假成功状态。

## 实际命令与结果

Rust命令先执行`. ./.tools/env.ps1`，测试进程TEMP/TMP/SQLITE_TMPDIR指向仓库output。

- `npx vitest run tests/publishing.test.ts tests/publish-queue.test.ts`：19通过。
- `npx vitest run`：12个文件、131项通过。
- `npm run lint`：通过。首轮仅新增Vue属性换行警告，按原规则格式化后通过，没有关闭规则。
- `npm run typecheck`、`npm run build`：M6已独立执行；M7的`desktop:build`再次实际执行含TypeScript的完整前端构建，通过。
- `cargo test --manifest-path src-tauri/Cargo.toml --locked --offline`：76通过、1项既有原生凭据测试忽略，包含跨书队列/损坏包/备份恢复和匹配任务释放。
- `cargo fmt --manifest-path src-tauri/Cargo.toml --check`、`cargo clippy --manifest-path src-tauri/Cargo.toml --locked --offline --all-targets -- -D warnings`：通过。
- `npm run desktop:build -- --config output/playwright/m6/tauri-install.json`：0.14.4隔离NSIS构建通过，安装升级退出0。
- `node output/playwright/m6/m7-verify.cjs seed`：仅首次初始化两册专用验收数据，不重复执行。
- `node output/playwright/m6/m7-verify.cjs verify`：完成列表、旧任务、来源、确认表单、下一篇与复制。脚本原先额外读取浏览器剪贴板时等待授权，未触及业务失败；停止该测试Node进程后用PowerShell只比较刚写入的已知链接，结果一致，`finish`命令完成剩余快照和原任务核验。产品只使用已验证的writeText。
- `node output/playwright/m6/m7-verify.cjs close`、重启后`restart-check`：通过，数据逐字段一致。
- 发布脚本未改动，沿用本次M6初轮`npm test`中30通过、1项macOS专用跳过的结果，没有重新运行该四分钟脚本组。

证据保存在Git忽略的`output/playwright/m7/`：tasks、before-restart快照、ready/published截图；脚本位于`output/playwright/m6/`。不把临时数据、个人网页快照或安装包加入Git。

最终只读核验见`output/playwright/m7/final-data.json`：正式数据库与源视频SHA-256均不变；隔离库schema9、完整性ok、无外键错误；原M6任务仍ready且记录0条，隔离库全局唯一1条记录与专用模拟备注完全对应。原开发进程仍保留。`git diff --check`通过。

## 风险及完成边界

本轮M6/M7开发与Windows隔离核心验收完成，按当前任务范围停止扩展。没有正式发布、正式安装升级、macOS/Linux原生上传、真实最终发布、多账号或无人值守能力验收。真实账号自然过期、30分钟上传超时和平台后续改版仍需日常观察；停止/任务变化保护以本地回归为主。正式交付前按项目发布流程选择feat/minor并进行真实安装升级，不能把临时0.14.4当作正式Release。
