# 创作连续性优化验收（2026-11-06）

## 完成内容

本轮完成已确认的三个切片：

1. 文案生成、按项调整、时长推荐与封面背景由应用级服务持有请求。可切换页面/书籍，返回继续查看同一任务、候选或失败；不重复提交。文案保留当前稿；背景需明确采用到本次请求尺寸。取消后停止显示片段，最终成功/取消以Rust执行结果为准。
2. 书籍资料（包括新建、待绑定路径）及按任务隔离的发布准备增加400ms自动暂存、离页保存、失败重试与重开恢复。不完整标题/定时输入可暂存，不自动创建业务记录、关联文件或发布。恢复资料只恢复可编辑字段，不覆盖后台已更新的时长等元数据。正式保存清理对应草稿与业务保存同事务。
3. 内容创作首屏压缩，已配置时长默认收起，记忆本机展开偏好。正文14px、辅助说明12px，沿用已有深浅主题、快捷键和历史对照。

同时将关闭保护收敛为一个原生监听，等待全局任务确认与当前编辑暂存全部完成。核对已安装`@tauri-apps/api` 2.12.4的`window.js::onCloseRequested`：每个独立监听都可能自行调用destroy，因此不能为多个保护条件各注册一份。使用现有公开API，没有新增权限。官方入口：[Window.onCloseRequested](https://v2.tauri.app/reference/javascript/api/namespacewindow/#oncloserequested)。

## 修改文件

| 文件 | 内容 |
| --- | --- |
| src/services/generation.ts | 应用会话内唯一生成请求持有者、预览、结果、取消与请求尺寸/按项参数 |
| src/components/RunningTasks.vue | 运行/完成入口与关闭时任务确认 |
| src/modules/writing/WritingPage.vue、src/modules/covers/CoversPage.vue | 页面订阅任务、候选采用、离页恢复、时长偏好 |
| src/composables/useEditorDraft.ts、src/composables/useWindowCloseGuard.ts | 串行草稿写入、统一窗口关闭检查 |
| src/modules/projects/ProjectPage.vue、src/modules/publishing/PublishingPage.vue | 新建/已有表单恢复、任务隔离、失败阻止离页、暂存反馈 |
| src/modules/video/VideoPage.vue | 使用统一草稿状态重置，原视频存储方式不变 |
| src/services/drafts.ts、src/services/workflow.ts | 草稿类型和资料续作入口 |
| src/style.css | 创作页首屏、正文与说明文字 |
| src-tauri/migrations/0019_editor_owners.sql、src-tauri/src/database.rs | v19迁移登记 |
| src-tauri/src/database/drafts.rs、projects.rs、publishing.rs、workflow.rs | 所有者校验、事务清理、续作；包含新数据库回归 |
| tests/generation.test.ts、ai.test.ts、covers.test.ts、projects.test.ts、publishing.test.ts、ui-refinement.test.ts、window-close.test.ts、src-tauri/src/database/m4_tests.rs | 生成离页/取消竞态、候选保留、暂存失败/恢复/隔离、关闭协调及当前schema断言 |
| PRD、TECHNICAL-DESIGN、DATABASE、AI-RULES、DEVELOPMENT-PLAN、USER-GUIDE、本方案与验收 | 同步实际能力和验证范围 |

## 数据库

schema从18兼容迁移到19。editor_drafts改为(owner_id,kind)主键，保留project_id级联归属，原writing/cover数据和时间戳完整复制。新建书籍owner为new；发布owner为书籍ID:任务ID或书籍ID:new。视频草稿不迁移，仍附在原视频任务。

新增测试覆盖旧表升级并重复打开、未完成输入、不同所有者隔离、外书任务拒绝、正式保存失败保稿/成功清理、已发布拒写、删除书籍级联与数据库备份恢复。完整备份沿用SQLite快照，新建表单尚未绑定的文件只保存路径，不进入素材文件备份。旧应用不能读取v19；回退使用升级前备份。

## 实际命令与结果

| 命令 | 结果 |
| --- | --- |
| `npx vitest run --maxWorkers=2` | 全量31文件426项通过（取消竞态修正后的记录） |
| `npx vitest run --maxWorkers=2 tests/projects.test.ts tests/ai.test.ts tests/generation.test.ts` | 最后两项收尾后127项通过；新增1项后台时长不被旧资料暂存覆盖的回归，当前用例总数427，未为此重复运行其余文件 |
| `npm run lint` | 最终通过，零警告要求保持 |
| `npm run typecheck` | 通过，最终build再次包含类型检查 |
| `npm run build` | 最终通过，Vite保留既有大于500kB单块提示 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked database::drafts::tests` | 3项新增数据库回归通过 |
| `npm run test:rust` | 最终224通过，2项既有专项忽略，0失败 |
| `npm run check:rust` | 最终fmt/全目标Clippy通过，保持-D warnings |
| `npm run desktop:build -- --debug --no-bundle --config output/playwright/continuity-20261106/tauri.verify.conf.json` | Windows独立标识的debug桌面程序构建成功，包含TypeScript/Vite/Rust；未制作安装包 |
| `npm run dev -- --port 18616` | Vite启动成功，用于当前源码浏览器验收 |
| `npx --yes --package @playwright/cli playwright-cli -s=continuity-20261106 run-code --filename output/playwright/continuity-20261106/verify.cjs --raw` | 浏览器六组流程通过，4档窗口无横向溢出，pageerror为0 |
| `node output/playwright/continuity-20261106/native.mjs` | Windows WebView2、真实IPC、统一Provider与SQLite的四组流程通过，成功轮2次本机HTTP请求，pageerror为0 |
| `node output/playwright/continuity-20261106/reopen.mjs` | 实际关闭进程再启动，资料最后输入和未完成发布表单恢复，未创建发布任务 |
| 退出后Python sqlite3只读核对 | schema19、integrity=ok、外键问题0、活动AI任务0、对应书籍发布任务0，最后输入及未完成定时字段正确 |
| `git diff --check` | 通过 |

Rust命令先执行`. ./.tools/env.ps1`。npm既有python/msvs_version用户配置提示未阻断执行。没有运行与本次改动无关的视频引擎、发布脚本或队列脚本，也未声称`npm test`全套执行。

开发期间的失败记录：旧测试中的“生成时禁止离页/放弃编辑”预期按新行为调整并增加恢复断言；新增全局任务服务的测试独立清理会话状态；新Rust测试的usize读取改为SQLite支持的u32；既有迁移用例硬编码18改为当前迁移数。全量前端首次不限并发时更新页1项超过原5000ms超时，单独复测22项通过，随后只限制命令并发为2，全量426项通过，未放宽超时或关闭断言。

桌面脚本首次尝试使用前端close命令被现有capability拒绝。没有扩大应用权限，改为只向已核对进程路径和窗口标题的隔离窗口发送Windows WM_CLOSE，后续完整关闭/重开验收通过。最后资料元数据合并与按项修改要求保留两处收尾已做前端127项回归及最终构建，未重复执行桌面脚本。

## 浏览器与Windows证据

当前源码加内存IPC夹具验证：生成可离页、后台完成后当前稿保留、只执行一次、资料恢复、不完整定时恢复、暂存没有发布副作用、失败阻止导航与重试。1440×900、1280×720、800×900、360×900无横向溢出。1280×720下标题顶部从约773px提前至441px，正文从约909px提前至552px，正文14px，底部操作栏顶部约649px；查看深浅主题截图确认首屏可编辑。

Windows使用独立identifier `com.shuyin.continuity-20261106`，禁用自动更新。二进制、测试工作目录与WebView缓存位于D盘本仓库output/playwright/continuity-20261106；独立SQLite按Tauri默认路径位于C:/Users/haozi/AppData/Local/com.shuyin.continuity-20261106。未访问正式com.shuyin.desktop库、用户凭据、作品或安装版本。

桌面生成使用本机HTTP的真实SSE，经原Provider解析/校验后入库：页面卸载仍完成一个备选，返回当前稿不变；第二次请求中用系统关闭消息触发确认，取消后窗口与稿件保留，返回原页可取消同一任务。随后暂存未完成定时发布，确认发布任务数为0。正常关闭后重开，最后编辑的资料和发布表单恢复。退出后又只读检查数据库完整性。

截图、完整日志、browser-result.json、native-result.json、native-reopen-result.json、native-integrity.json保存在上述忽略目录。所有隔离浏览器、服务器和桌面程序在验收后关闭。

## 验收对应与剩余范围

| 标准 | 结果 |
| --- | --- |
| 生成时能继续其他页面，返回同一任务 | 前端回归、浏览器、Windows真实IPC通过 |
| 生成失败/取消保稿，后台结果不覆盖当前稿 | 回归及Windows生成/取消通过；完成先于取消竞态有单元测试 |
| 资料/发布不完整输入暂存，任务与书籍隔离 | 前端与Rust通过，Windows关闭重开通过 |
| 暂存失败留页、保存成功清理对应草稿 | 前端/浏览器失败重试及Rust事务测试通过 |
| v18迁移、旧草稿、备份与只读边界 | Rust通过，独立库完整性核对通过 |
| 1280×720首屏、深浅主题与窄窗口 | 当前源码浏览器验收通过 |

无已知阻塞本轮三个切片的问题。应用关闭后不继续AI请求；未采用的会话候选不承诺跨重启恢复，已保存文案历史与图片素材保留。未测试外部付费模型、真实Google/抖音登录发布、正式安装升级、macOS/Linux或长片渲染。

实施阶段未暂存、提交、推送或发布；后续提交授权与复核见下一节。本轮是feat/minor，源码版本保持0.51.1，由发布流水线统一处理。下一轮建议继续统一待处理入口，再做音频完成后由用户选择自动准备字幕与分镜。

## 提交前双轴评审与回归（2026-11-06）

用户后续明确授权评审、修复、生成中文提交信息、提交并推送dev，再将本次提交cherry-pick到main并推送。评审基线为dev的`079b638`，覆盖全部工作区修改和新增文件；远端刷新后确认main与此基线代码内容一致。

### Standards

发现2项P2，已修复，复核无剩余阻塞；没有额外架构或代码气味问题。

- 资料保存提交页面打开时的完整快照，UPSERT会把后台刚推荐的时长回写成旧值，同时保留`ai`来源，违反字段职责和成果保留约定。已有书籍更新现在保留数据库中的目标时长，仍由专用时长命令/推荐完成事务更新；新建初始化不变。
- 时长推荐失败/取消后被结果监听移出会话，重挂载误判为从未请求，再次自动推荐，违反不因重挂载重新请求和不自动重试约定。现在复用原任务保留失败/取消记录，自动推荐仅在没有会话记录时执行；明确点击仍可重试。

### Spec

独立需求评审同样发现上述2项P2，分别对应“后台成果不被旧资料覆盖”和“返回同一任务，不重复请求”。新增回归覆盖推荐完成后保存旧资料、手动时长后再次保存，以及失败/取消后连续两次离页返回仍只请求一次、主动重试成功。其他已确认切片未发现明确漏实现或越界。

两轴各发现2项，最高均为P2；去重后共2项，均已修复。新增测试先在原实现失败：前端请求次数为2而预期1，Rust时长为180而预期1200；修复后通过。补充修改`src-tauri/src/database/m2_tests.rs`，不新增迁移或依赖。

### 本次实际执行

| 命令 | 结果 |
| --- | --- |
| `npx vitest run --maxWorkers=2` | 评审修复前31文件427项通过 |
| `npx vitest run tests/generation.test.ts tests/window-close.test.ts --maxWorkers=2 --reporter=verbose` | 8项通过 |
| `npx vitest run tests/ai.test.ts --maxWorkers=2 -t 'does not repeat a'` | 修复前2项失败，均复现重复请求 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked saving_project_details_preserves_duration_completed_after_the_form_was_loaded` | 修复前失败；修复后1项通过 |
| `npx vitest run tests/ai.test.ts tests/projects.test.ts tests/generation.test.ts --maxWorkers=2 --reporter=dot` | 修复后129项通过，当前前端用例总数429；没有为局部修复重复其余已通过文件 |
| `npm run test:rust` | 修复后225通过，2项既有专项忽略，0失败 |
| `npm run lint` | 修复后通过，零警告 |
| `npm run build` | 修复后TypeScript与Vite通过，保留既有大于500kB单块提示 |
| `npm run check:rust` | 修复后fmt与全目标Clippy通过 |
| `npm run test:video` | 使用已有Node 24.22.1后39项通过 |
| `npm run test:release` | 78通过，1项macOS检查在Windows跳过 |
| `npm run test:queue` | 使用已有Node 24.22.1后19项通过 |

Rust命令先执行`. ./.tools/env.ps1`。默认Node为24.11.0，视频入口被现有Windows运行时检查拒绝，导致6个夹具未产出HTML；队列1项中文路径删除回归也失败。仅为复测将当前进程PATH前置本机已有`C:/Users/haozi/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/bin`（24.22.1，与CI一致），视频及队列原断言全部通过，没有修改全局环境、测试或运行时保护。日志位于忽略目录`output/continuity-review-*.log`。

本次复核未重新执行上文浏览器/Windows桌面脚本，没有新增外部模型、真实发布、安装升级或跨平台验收证据。既有实施阶段证据保留其原验证范围；两项提交前修复由前端挂载/路由回归和实际SQLite事务测试证明。后续按授权推送dev与main，main推送将触发现有版本/打包流水线，远端打包结果须以该流水线为准。
