# 模型调用诊断分页与清理验收（2026-11-11）

用户希望避免一次显示200条，并可分页浏览保存的调用历史、定期或手动清理。核对实际链路后确认：统一Provider早已将全部请求写入SQLite，旧查询固定LIMIT 200且组件全量渲染。此次保留原数据源，修改查询边界与维护操作。

用户追加的独立评审与补测已定位并修复2项P2，原始失败回归、最终复审和本轮新执行的全量测试见[评审记录](MODEL-DIAGNOSTICS-REVIEW.md)。下文保留首次实现时的46项相关前端及浏览器验收证据，不能替代评审修复后的新结果。

## 实现与验收标准

| 验收项 | 当前实现与证据 |
| --- | --- |
| 默认小页，旧历史可达 | 默认20条，可选10/20/50/100；245条夹具可遍历13页，无重复、无丢失，末页5条。 |
| 真实数据库分页 | LIMIT/OFFSET只读取当前页，按started_at/id稳定倒序；全局总数与运行数独立于当前页，非法参数拒绝，越界页夹取最后有效页。 |
| 页内阅读与失败反馈 | 表格最高520px、固定表头，显示总数/范围/页码；翻页或改条数回到表内顶部，读取失败保留旧页，不误报空库。 |
| 手动清除 | 明确确认后只删已结束记录，显示实际删除数；运行中请求能继续回写真实耗时/用量，项目/文案/素材和原文件保持不变。 |
| 可选定期清理 | 默认全部保留；7/30/90天保存前确认，保存与立即删除同事务，启动后及每小时检查，关闭策略不删记录。 |
| 持久化与恢复 | 策略及记录随原SQLite重启/备份恢复，旧运行中记录先恢复为interrupted，过期清理不制造耗时。 |
| 操作保护 | 维护写入期间禁止冲突维护、离页和关闭；弹窗可取消，失败在弹窗内可重试，轮询不覆盖未保存的选项。 |

## 修改范围

- `src/modules/settings/ModelDiagnostics.vue`、`SettingsPage.vue`：分页、保留策略、清理确认、错误/空/加载状态及维护保护。
- `src/services/maintenance.ts`：单一分页契约与两条窄维护接口。
- `src-tauri/src/database/diagnostics.rs`：分页、清理、策略事务与7项诊断测试；`commands.rs`、`lib.rs`和`permissions/projects.toml`：阻塞任务调度、主窗口命令及每小时间隔。
- `src-tauri/migrations/0024_model_call_pagination.sql`、`src-tauri/src/database.rs`：仅将旧时间索引替换为时间/ID联合索引，schema23→24。
- `tests/modelDiagnostics.test.ts`、`maintenance.test.ts`、`projects.test.ts`、Rust `ai/tests.rs`与`database/automation_tests.rs`：前端交互、既有契约和真实旧版本备份夹具更新。
- 产品、技术、数据库、AI边界、开发计划和使用说明同步；本轮规划记录追加在原三份工作文件顶部。

不新增依赖、不更改模型调用策略、Prompt、原始素材、凭据或已保存作品。默认保留所有已有调用，只有用户确认清理或启用保留策略后才删除诊断行。SQLite空闲页复用，不进行自动全库VACUUM。

## 实际验证

Rust命令先加载`. ./.tools/env.ps1`，本次进程`TEMP`、`TMP`、`SQLITE_TMPDIR`指向仓库`output/diagnostics-test-temp`。前端使用当前Node v24.11.0和已有依赖。

| 命令 | 结果 |
| --- | --- |
| `npx vitest run tests/maintenance.test.ts tests/modelDiagnostics.test.ts tests/projects.test.ts --maxWorkers=4` | 46通过。 |
| `npm run lint` | 通过，0错误/警告。 |
| `npm run build` | 通过，含vue-tsc；Vite提示既有主包超过500kB，无构建失败。 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked -- --quiet` | 285通过、2项既有原生凭据/进程测试忽略，0失败；含7项诊断及全部旧版备份恢复测试。 |
| `npm run check:rust` | 通过，含cargo fmt和全部target的Clippy，警告按错误处理。 |
| `git diff --check` | 通过。 |
| `python output/diagnostics-browser-20261111/check-docs.py` | 10份Markdown中的195个本地链接通过。 |

浏览器使用`ego-browser`和`npm run preview -- --port 1427 --strictPort`启动的独立生产预览。在1440/1000/720/375px的深浅主题共8组布局中，页面无水平溢出，表格最高520px、表头sticky，默认只渲染20条。已查看深浅宽屏与375px截图；截图和结构化结果在本机`output/diagnostics-browser-20261111/`。

真实浏览器中的隔离演示IPC交互通过：第13页显示最后5条、上一页正常、改为50条时从第1页请求；启用30天策略前取消保留245条，确认后清除185条过期记录；手动清除前取消无写入，确认清除59条后剩1条running；关闭自动清理仍保留该条，请求结束刷新后可清空；读取失败不冒充空库，再次刷新恢复。最终未知IPC与页面错误均为0；空列表另在1000px及减少动态效果模式截图。

执行脚本命令为`ego-browser nodejs -e "await import('file:///D:/MyWork/project/JIAZI-PROJECT/shuyin-ai/output/diagnostics-browser-20261111/round-XX.mjs')"`，最终有效轮次为03/04/07/08/09/10；`layouts.json`与`interactions.json`记录结果。演示IPC没有连接正式数据库，SQLite正确性另由Rust测试验证，不将此页面测试记为Windows原生IPC或已安装程序验收。完成后已关闭测试浏览器空间。

## 检查中修正

1. 当前rusqlite的u64 SQL转换需要未启用的fallible_uint特性；按照当前源码改用SQLite原生i64边界读取非负计数、传递有界偏移，不增加依赖特性。
2. 新前端分页契约发现projects测试仍返回旧数组，已同步真实返回类型；组件DOM类型使用globalThis限定。
3. 数据保护测试初次在创建文案前截取书籍快照，文案准备本身会更新书籍时间。改为准备完成后取快照，保留完整逐字段断言。
4. 全量Rust检查发现旧备份夹具只下调user_version并删除自动流程字段，仍携带v24索引。同步恢复各旧版本真实的单列索引；版本断言改为当前迁移数量，备份结构比较继续严格执行。
5. 浏览器首轮误用普通路径而当前路由使用hash历史；已按实际路由修正为`/#/settings`，不修改产品路由。
6. Ego的CDP注入与视口覆盖随命令轮次结束释放。跨轮导航后重新安装演示IPC；不跨视口保留打开的菜单，恢复到稳定视口后通过逐项点击完成。两次未达到预期的菜单点击/连续按键轮次不计作成功，未修改共享Select来掩盖验证工具的视口变化。

## 验证边界与下一步

未操作正式数据库、调用付费模型或清除用户记录；未制作安装包或验证安装升级、macOS/Linux。小时调度已接入应用全局，删除函数通过隔离数据库回归；尚未用桌面进程连续运行一小时做时钟验收。使用包含此次修改的新构建后，建议在Windows桌面实际切页并核对保留设置。此次属于feat/minor兼容增强，不手动升版本、不提交或推送。
