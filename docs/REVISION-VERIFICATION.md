# 按项调整验收记录

日期：2026-10-25。范围：已有七项结果的部分调整，不包括草稿自动恢复、完整素材备份或其他新增建议。

## 实现与验收标准

| 标准 | 结果 |
| --- | --- |
| 选择一项或多项，选填修改要求 | 页面与真实 WebView2 通过 |
| 使用当前历史版本，生成前保存修改；保存失败不调用模型 | 页面回归通过 |
| 原内容与候选逐项对比，采用前不追加正式版本 | 页面、Rust 和真实 IPC 通过 |
| 未选中内容逐字段保持一致 | Rust、真实数据库和备份比对通过 |
| 放弃、取消、异常输出不覆盖原稿 | 页面、Rust 和真实 IPC 通过 |
| 采用失败保留候选，重复采用只追加一次 | 页面失败重试与 Rust 只读故障测试通过；真实 IPC 重复采用通过 |
| 音频保留固定主持人、系列、时长和五段结构 | Rust 与本机契约服务真实调用通过 |
| 旧数据迁移、失败回滚及 v2–v8 备份恢复 | Rust 迁移/恢复矩阵通过 |

候选预览不承诺跨重启恢复。原编辑器在调整生成和候选预览期间锁定；整套生成仍保留生成中编辑的行为。

## 修改文件

- 页面和 IPC：`src/modules/writing/WritingPage.vue`、`src/services/ai.ts`、`src/types/ai.ts`。
- 生成与排版：`src-tauri/src/ai.rs`、`src-tauri/src/ai/revision.rs`、`src-tauri/src/ai/publishing.rs`、`src-tauri/src/ai_commands.rs`、`prompts/publishing-revision-v1.md`。音频共用一份模板实现。
- 数据及命令注册：`src-tauri/src/database.rs`、`src-tauri/src/database/ai.rs`、`src-tauri/migrations/0008_generation_preview.sql`、`src-tauri/src/lib.rs`、`src-tauri/build.rs`、`src-tauri/permissions/ai.toml`。
- 回归：`tests/ai.test.ts`、`src-tauri/src/database/m2_tests.rs`；`m3_tests.rs`、`m4_tests.rs` 同步可选输入字段和当前 schema 断言。
- 文档：PRD、技术设计、数据库设计、AI 规则、开发计划、使用手册与本记录。

## 数据库影响

schema v7→v8，仅为 `ai_tasks` 增加 `preview_json`，不重写历史版本、素材、发布任务或凭据引用。候选成功时暂存完整合并结果；采用时事务追加 mixed 版本、关联任务并清空候选。备份接受 v2–v8；回退旧程序须恢复升级前备份。无新增 npm/Rust 依赖，无手动版本号修改。

## 实际执行的命令

| 命令 | 结果 |
| --- | --- |
| `npx vitest run tests/ai.test.ts` | 新用例首次因缺少界面失败；实现后19项通过，补充多项选择/取消/保存失败用例后由全量测试覆盖 |
| `npm run lint` | 通过，零 warning |
| `npm run build` | TypeScript 与 Vite 生产构建通过 |
| `npx vitest run` | 6个文件、81项通过 |
| `. ./.tools/env.ps1` 后 `npm run check:rust` | fmt / Clippy 通过，`-D warnings` |
| 同一环境下 `npm run test:rust` | 最终58项通过、1项原有原生凭据测试忽略 |
| `npm run desktop:dev -- --config output/playwright/revision/tauri-test.json --no-watch` | 当前实现编译并启动独立 Windows WebView2 窗口 |
| `python output/playwright/revision/server.py` | 启动本机固定响应契约服务，未连接付费服务 |
| `npx --no-install --package @playwright/cli playwright-cli -s=shuyin-revision attach --cdp=http://127.0.0.1:19357` | 连接独立验收窗口 |
| 上述 CLI 的 `run-code --filename=output/playwright/revision/seed.js`、`run-code --filename=output/playwright/revision/verify.js` | 六项真实 IPC/界面断言通过 |
| 上述 CLI 的 `resize 1120 900` / `resize 800 900` 与页面宽度读取 | 两种宽度无横向溢出 |
| 上述 CLI 的 `console error` | 0个页面错误 |
| `python output/playwright/revision/verify_files.py` | 独立应用库与备份均为 v8，目标项目各3个版本、5个任务；完整性及外键正常，未选字段一致 |
| `git diff --check` | 通过 |

发布脚本未改动，本轮未重复执行发布脚本测试；不把此前结果当作本轮安装或远端发布验收。

## Windows 真实运行证据

独立 identifier：`com.shuyin.revision-verification-20261025`。在此 identifier 的 SQLite、系统设置与 WebView 数据上运行。测试工作目录及备份在 D 盘 `output/playwright/revision/`。未调用正式 identifier 的 IPC，未修改正式安装数据、用户模型或原始视频。

六项断言覆盖：标题候选与采用时保留其他字段、重复采用幂等、多字段候选放弃、Provider 返回未选字段被拒绝、音频固定模板、界面取消保留历史。截图 `preview.png` 已视觉核对，运行证据 `evidence.json`，数据库和备份比对 `filesystem.json`。脚本及证据受 Git 忽略。

验收脚本曾因使用旧默认工作目录而找不到备份目录，改为明确的 D 盘验收目录后重新创建独立书籍并完整重跑通过；没有为此修改业务逻辑。文档热更新触发过未采用候选的离页提示，关闭提示后继续验证。

## 未验证与下一步

尚未验证外部真实模型的新 Prompt 输出质量、新安装包升级、macOS/Linux 实机。Windows 结果来自当前源码的独立开发运行版，模型来自本机契约服务。下一步用实际模型试用至少一本书的单项和音频调整，再按 `feat` 交由既有发布流水线构建和发布。
