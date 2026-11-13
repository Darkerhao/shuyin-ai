# 章节精讲完整流程验收

日期：2026-11-13。基线：dev / b55ec9c。接续上一轮未提交的章节实现；本轮未提交、推送或手工升版本，未修改正式数据库或使用真实生成/发布账号。

后续用户要求独立评审测试，补查5项P2并修复；下方538前端/295Rust及浏览器记录保留为实施阶段证据，最新代码的评审、补测与全量结果见[独立评审报告](CHAPTER-WORKFLOW-REVIEW.md)。

## 1. 交付范围

一本书下保存多个章节，每章与整书共用完整的内容创作、音频、三尺寸封面、视频和发布准备。仅来源与成果范围不同：本章正文用于本章精讲，章节之间的时长、草稿、版本、音视频、封面和发布记录独立。

撤除了上一方案中的仅视频完成目标及相应条件分支，恢复唯一“自动制作并准备发布”链路。章节同样会制作4:3、3:4、16:9封面，成片后继续官方网页准备，由用户最终检查并发布。原手动五步工作台继续可分别使用。

共用Prompt增加细致精讲要求，解释有来源的概念、细节、人物动机或论证过程，时长按本章深度推荐。整书五部分保持，章节支持1—5个讲解部分。

## 2. 主要文件

- 章节数据与升级：[chapters.rs](../src-tauri/src/database/chapters.rs)、[0025_chapter_creation.sql](../src-tauri/migrations/0025_chapter_creation.sql)、[database.rs](../src-tauri/src/database.rs)、[backup.rs](../src-tauri/src/database/backup.rs)。
- 唯一自动执行器：[automation.rs](../src-tauri/src/automation.rs)、[database/automation.rs](../src-tauri/src/database/automation.rs)、[AutomationSetup.vue](../src/components/AutomationSetup.vue)。
- 章节来源、版本和素材贯通：[database/ai.rs](../src-tauri/src/database/ai.rs)、[audio.rs](../src-tauri/src/database/audio.rs)、[covers.rs](../src-tauri/src/database/covers.rs)、[video.rs](../src-tauri/src/database/video.rs)、[publishing.rs](../src-tauri/src/database/publishing.rs)、对应Command、服务和类型。
- 统一入口与界面：[App.vue](../src/App.vue)、[ProjectNavigation.vue](../src/components/ProjectNavigation.vue)、[ProjectChapters.vue](../src/modules/projects/ProjectChapters.vue)、[ProjectPage.vue](../src/modules/projects/ProjectPage.vue)，以及现有创作/封面/视频/发布页、全局任务、发布队列、素材和分镜图片组件。
- Prompt：publishing-v8、publishing-revision-v7、audio-template-v4、audio-duration-v4、content-check-v2；历史版本及已保存结果保留。
- 回归：[chapter_tests.rs](../src-tauri/src/database/chapter_tests.rs)、原自动流程/迁移/恢复测试、[chapters.test.ts](../tests/chapters.test.ts)及原创作、封面、发布、导航、后台生成测试。
- 文档：PRD、技术设计、数据库、AI规则、开发计划、用户指南、[章节设计](CHAPTER-VIDEO-DESIGN.md)及本记录。

完整工作区文件清单保存在`output/chapter-implementation/changed-files.json`，不含忽略目录中的验证产物。

## 3. 数据库与兼容性

schema25新增book_chapters，并给8类原有记录增加可空chapter_id；文案版本增加只读来源快照。旧记录为NULL整书范围，版本号、目录和原始文件保留。草稿继续使用editor_drafts，按owner隔离，不增加第二套草稿或任务表。

旧自动输入在迁移事务内由sourceAssetId转换为统一source，保留账号、合集、目标、状态、子任务和上传标记。现行旧整书流程不改成另一条执行路线，早期workflowVersion 0/1继续原暂停交接。

已有v2—v24数据库在启动升级前，先使用SQLite备份接口写入数据库同级chapter-upgrade-backups；备份失败不开始迁移。隔离测试已验证升级前副本仍为v24、旧来源仍在，迁移后数据与外键有效，完整恢复保留各章正文、快照、草稿、封面和发布记录并重映射媒体路径。正式书库未在本轮执行升级。

## 4. 实际验证命令与结果

| 命令 | 实际结果 |
| --- | --- |
| `npx vitest run --maxWorkers=2` | 37个文件，538通过、0失败 |
| `npx vitest run tests/ui-refinement.test.ts --maxWorkers=1` | 最新顶部快捷入口修正后，17通过、0失败；与全量有重叠 |
| `npm run test:video` | Node 24.22.2下40通过、0失败 |
| `npm run test:release` | 80通过、0失败、1项macOS专属检查跳过；包含43项音频页面脚本测试 |
| `npm run test:queue` | 19通过、0失败 |
| `npm run test:rust` | 295通过、0失败、2项原有环境测试忽略 |
| `npm run check:rust` | rustfmt及Clippy所有target通过，未关闭警告 |
| `npm run lint` | 最终复核通过，0错误、0警告 |
| `npm run build` | TypeScript与Vite生产构建通过；仍有大于500KB的分块体积提示 |
| `python -X utf8 output/chapter-implementation/check-docs.py` | 8份文档、36个本地链接、C01—C16编号通过 |
| `git diff --check` | 最终复核通过 |

Rust命令先加载`. ./.tools/env.ps1`。测试进程TEMP、TMP、SQLITE_TMPDIR使用仓库`output/chapter-test-temp`。视频脚本使用已安装的`D:/MyWork/software/shuyin-video-node/v24.22.2/node.exe`，仅调整本次进程PATH，未安装或替换系统Node。

日志位于`output/chapter-implementation/`：`vitest-final.log`、`navigation-final.log`、`rust-final.log`、`rust-check-final.log`、`video-node-final.log`、`release-node-final.log`、`queue-node-final.log`、`lint.log`、`build-final.log`。

发布脚本测试使用本地夹具及本地Git仓库，不表示实际构建安装包、推送本项目或发布GitHub Release。

## 5. 浏览器与问题修复证据

使用ego-browser和当前Vite页面，在独立TaskSpace中注入内存Tauri接口，验证真实Vue组件与路由：

- 修改并保存本章目标时长；页内、侧栏、顶部快捷入口和快捷键传递章节。
- 第一章封面、视频及发布页均保持chapter-1，1440px三页无页面横向溢出、错误或未覆盖接口调用；保存了三张页面截图。
- 第一章与第二章切换时文案和素材对应当前章；第一章未提交发布标题可切走再恢复；整书视图继续显示整书数据。
- 新增第三章，保存独立正文和20分钟目标，正确进入第三章；最终DOM核对无错误。

证据在`output/chapter-implementation/browser-20261113/`：`workflow-pages.json`、`chapter-interactions.json`、`final-state.json`及封面/视频/发布截图。浏览器TaskSpace已关闭。

过程中修复了实际问题：

1. 仅视频目标跳过了章节封面和发布准备，已删除这条新增分支。
2. 全局任务、侧栏、顶部快捷入口和页面返回链接丢失章节；统一传递范围，精确成果不再回退到别章。
3. 新发布草稿仍使用整书owner；改为按章owner，已有任务草稿仍绑定任务ID。
4. Input的数字事件使章节时长调用trim报错；按组件实际string/number接口修正，并在表单内显示暂存失败。
5. 历史版本按项调整可能把当前目标混入旧音频；非音频调整保留旧目标，明确调整音频指导时保存新目标，正文始终继承所选版本。

首轮旧迁移夹具未移除新章节字段，另有上一轮机械修改破坏的v1测试INSERT；修正夹具后全量通过，未放宽迁移或备份校验。Clippy检查发现的静态SQL多余format已直接移除。

默认Node 24.11首先触发项目已有的Windows中文路径保护（视频脚本33通过、7失败）；使用现有24.22.2后40项全部通过，没有移除运行时保护。一次并行执行时原视频组件测试超过5秒，独立重跑及后续完整前端运行均通过，未增大测试超时。

开发期间浏览器夹具曾丢失，同URL的goto未重建文档；重新注入并reload后继续。新章节时长输入位于当前视口下方，按当前可见状态滚动后完成。额外窄屏/浅色截图受最小化窗口限制未完成，没有计为通过；现有三页1440px截图及功能验证保留。

## 6. 验收对应与边界

| 设计验收 | 本轮证据 |
| --- | --- |
| C01—C03 数据、迁移、草稿与切换 | SQLite升级/备份/重开/完整恢复，章节组件失败保稿及浏览器跨章恢复 |
| C04—C09 来源、时长、Prompt与旧版本 | 章节全文进入请求、旧版音频正文一致、跨书拒绝、非音频/音频调整目标区分、1—5段验证 |
| C10 字幕与校核 | 原声字幕、时间码、冻结章节校核输入及失败保留的本地回归；未评估真实模型讲解质量 |
| C11—C15 完整制作与成果隔离 | 同一自动流程三尺寸失败续作同时验证整书/章节；跨章文案/音视频/封面发布引用拒绝；单章发布不改变整书状态 |
| C16 真实外部完整链路 | 未运行。真实模型、NotebookLM登录/长文本/音频下载、实际成片与真实官方发布准备待用户场景验收 |

未运行新安装包构建/升级或macOS/Linux实机；本地数据库、组件、页面脚本和模拟浏览器不能替代这些结果。没有最终自动发布，实际发布仍由用户完成。

## 7. 下一步

在Windows新版应用中选同一本书的两个真实章节，依次完成正文、精讲方案、音频、三尺寸封面、视频和发布准备，检查讲解深度及跨章内容隔离；最后在官方页面由用户检查并发布。继续沿用现有模型、视频环境及账号配置。
