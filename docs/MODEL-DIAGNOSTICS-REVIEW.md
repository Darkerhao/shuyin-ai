# 模型调用诊断：独立评审与补测（2026-11-11）

本轮按用户「完成之后进行评审测试」要求，评审上一轮尚未提交的分页、持久调用记录及可选清理改动。固定基线为main/`411320d898470fecd55916f48be59242f2eb6c2c`，差异命令`git diff 411320d898470fecd55916f48be59242f2eb6c2c -- .`，同时单独纳入未跟踪的v24迁移、组件回归和首次验收文件。没有创建提交或操作远端。

## Standards

独立初审及最终增量复审：**0项规范违规、0项需处理的代码异味，PASS**。

依据AGENTS.md、AI-RULES、用户的最少代码/最小复杂度原则及code-review基线核对：原Provider、SQLite和UI组件继续复用；数据库层负责分页与删除；保留设置/首次清理同事务；主窗口权限维持窄命令；索引迁移及旧备份结构检查保留。修复直接更新同一结果中的已保存设置，用单个待刷新标记协调维护后读取，没有增加第二套状态或请求框架。

## Spec

独立初审：**2项P2**。真实组件回归确证并修复后，独立复审：**0项剩余发现，PASS**。

| 问题 | 触发、根因与修复 |
| --- | --- |
| P2：已启用的自动清理不能立即关闭 | 初始全部保留，保存30天成功而列表回读失败。已保存天数仍停在旧0，选回全部保留时“保存设置”误禁用，违反“可随时改回全部保留”。策略写入成功后立即同步`result.retentionDays`，保持列表错误反馈。 |
| P2：恢复后显示旧调用数据 | 旧查询仍在途时，恢复完成的`disabled=true→false`触发读取被busy拦截；旧结果或旧错误随后写回，无运行请求时不再刷新。违反“恢复备份等维护结束后重读”。单个待刷新标记保留延期读取需求，旧结果和错误失效；原查询结束后再读取，卸载后不重新开始。 |

修复位置为`src/modules/settings/ModelDiagnostics.vue`的`readPage`、`load`、`apply`和维护状态watch。原SQLite分页、running保护、策略事务、迁移和小时调度本轮没有发现新的需求偏差。

## 先复现、后修复

首先新增实际挂载组件的回归，执行：

```powershell
npx vitest run tests/modelDiagnostics.test.ts --maxWorkers=4
```

修复前**4失败、11通过**：无法关闭已保存策略、维护后旧请求成功、维护后旧请求失败、维护中旧响应仍展示。修复后原15项全部通过，再加入清理成功回读失败和设置页集成测试，相关3文件共**55通过**。确定性Promise控制响应顺序，不依赖sleep或放宽超时。

本轮新增9项回归覆盖：保存成功/回读失败仍可关闭策略；旧读取成功或失败时维护重读不丢；维护期间结果丢弃；卸载不重读；设置写入失败保留重试；清除成功后读取重试不重复删除；通过真实SettingsPage与Router执行备份恢复后重读；清理期间的备份/目录/缓存互斥、离页和关闭保护及完成后释放。

## 实际执行的检查

全量npm测试使用本机已有Node v24.22.2并设置`$env:VITEST_MAX_WORKERS='4'`，沿用隔离测试，不修改工程配置。其他前端检查使用默认Node v24.11.0。Rust先执行`. ./.tools/env.ps1`，临时目录为仓库`output/diagnostics-review-test-temp`。

| 命令 | 本轮实际结果 |
| --- | --- |
| `npx vitest run tests/modelDiagnostics.test.ts tests/maintenance.test.ts tests/projects.test.ts --maxWorkers=4` | 55通过；包含本轮新增9项。 |
| `npm test` | 通过：前端518、视频脚本40、发布/音频脚本78、队列脚本19；1项macOS专属检查跳过。 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked -- --quiet` | 285通过、0失败、2项原有原生凭据/进程测试忽略。 |
| `npm run lint` | 通过，0错误/警告。 |
| `npm run build` | 通过，含TypeScript和生产构建；保留Vite主包大于500kB提示。 |
| `npm run check:rust` | 通过，fmt和全部target的Clippy。 |
| `git diff --check` | 通过。 |
| `python output/diagnostics-review-20261111/check-review.py` | 11份Markdown中的198个本地链接通过。 |

全量npm日志保存在本机`output/diagnostics-review-20261111/npm-test.log`。独立Spec复审另外运行组件16项通过，不叠加计数。Standards复审只读，未将审查本身记为执行测试。

## 本轮修改及边界

- 产品：`src/modules/settings/ModelDiagnostics.vue`，仅修复上述状态衔接。
- 测试：`tests/modelDiagnostics.test.ts`、`tests/projects.test.ts`。
- 文档：本文件、首次验收、技术设计、开发计划、使用说明，以及原三份工作记录。
- 数据库：沿用上一轮schema24索引迁移，本轮无新增迁移、字段、依赖或权限，不修改正式数据。

评审发现均已关闭，原功能及新增异常路径达到本轮代码与自动化验收要求。上一轮深浅主题8组浏览器布局仍是首次实现证据，本轮未重跑浏览器/Windows原生进程、小时调度实钟、新安装包或macOS/Linux验收，不将单测模拟IPC写成正式桌面结果。下一步用含最终修复的Windows构建做一次实际分页和保留策略操作。
