# 创作体验与完整恢复验收（2026-10-25）

基线为 `a9af075`，已有「按项调整」继续复用。本轮完成用户清单其余六项，无提交、版本修改或发布。Windows 使用独立 identifier `com.shuyin.workflow-verification-20261025`，没有替换正式安装或数据库。

## 验收对应

| 用户需求 | 实际行为与证据 | 结果 |
| --- | --- | --- |
| 按项重新生成 | 沿用已有字段选择、修改要求、逐项预览和采用；原有契约/界面回归继续通过，本轮未再次调用外部模型 | 已有实现保留 |
| 草稿自动暂存、重启恢复 | 400ms防抖、串行写入，未完成文字可暂存；离页/正常关闭等待写完，失败阻止离开；文案与封面重启后分别恢复，正式文案仍只有2版 | 通过 |
| 包含素材的完整备份 | 可整体搬移的目录，含数据库、登记素材、导出封面及当前任务包，视频可选；预览确认后复制到新目录并重建路径，原文件及恢复前数据库保留 | 通过 |
| 裁切位置、缩放及并排预览 | 真实指针拖动、键盘缩放至1.5倍，三个完整分辨率画布及PNG尺寸正确；1440/1120/800宽度无横向溢出 | 通过 |
| 发布选择历史文案和封面版本 | 新任务文案为空，明确选v1而非最新v2；实际PNG缩略图解码显示，任务及包内持久化来源v1和文字快照 | 通过 |
| 我的封面样式 | 保存、应用、删除；只带品牌、字体、字号、颜色、蒙层和布局，跨书应用保留本书文字/背景/尺寸/裁切 | 通过 |
| TXT/Markdown资料导入 | UTF-8/BOM、来源名和路径、预览追加；保留旧笔记，随书籍保存；无效编码、空白、缺失及超限拒绝 | 通过 |

## 代码与数据库范围

主要界面为 `src/modules/{writing,covers,projects,publishing,settings}`；新增 `CoverThumbnail.vue`、`useEditorDraft.ts` 和 `services/{drafts,backup,notes}.ts`，沿用一个封面渲染器及原有正式版本保存路径。

Rust 新增 `database/{drafts,full_backup}.rs` 与 `note_import.rs`；修改现有 covers/publishing/backup 服务，统一登记 commands、build清单与最小权限。直接声明 `sha2 = 0.11.1` 复用锁文件已有包，Cargo.lock只增加根包引用，没有新解析包或压缩依赖。

`0009_creator_workflow.sql` 新增 `editor_drafts`、`cover_presets` 和可空 `publish_tasks.writing_version_id`。当前schema v9，旧数据保留；结构冲突回滚，数据库备份支持v2–v9。裁切字段存现有封面JSON，旧配置读取默认居中/1倍，不改写旧行。

完整备份是 `manifest.json + database.sqlite3 + files/` 目录，格式版本1。登记文件及数据库SHA-256全部核验，流式处理可选视频。恢复到唯一新目录后重建项目、素材、导出及任务包引用；未包含的视频保持原路径，缺失时任务退回草稿。仅收集数据库登记文件和当前登记包的固定八个文件，不包含密钥、未登记文件或已无引用的旧包。

## 实际执行命令

均在仓库根目录执行，Rust先加载 `. ./.tools/env.ps1`。

| 命令 | 实际结果 |
| --- | --- |
| `npm run lint` | 最终通过，0 warning |
| `npm run build` | TypeScript检查及Vite生产构建通过 |
| `npx vitest run` | 最终8个文件、95项通过 |
| `npm test` | 此前阶段94项前端通过；其内 `npm run test:release` 27项通过、1项macOS专用跳过；后续新增草稿竞态测试由最终95项覆盖，发布脚本无修改未重复运行 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | 通过 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked --offline` | 66项通过、1项既有系统凭据显式测试忽略 |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets --offline -- -D warnings` | 通过 |
| `python output/playwright/workflow/inspect.py` | v9、完整性ok、外键0异常，备份/恢复文件与正式库哈希核验通过 |
| `git diff --check` | 通过 |

真实桌面启动与验收命令：

```powershell
npm run dev -- --port 1420 --strictPort
. ./.tools/env.ps1
npm run desktop:dev -- --config output/playwright/workflow/tauri-test.json --no-watch
npx --no-install --package @playwright/cli playwright-cli -s=shuyin-workflow attach --cdp=http://127.0.0.1:19367
npx --no-install --package @playwright/cli playwright-cli -s=shuyin-workflow run-code --filename output/playwright/workflow/seed.js
npx --no-install --package @playwright/cli playwright-cli -s=shuyin-workflow run-code --filename output/playwright/workflow/verify.js
npx --no-install --package @playwright/cli playwright-cli -s=shuyin-workflow run-code --filename output/playwright/workflow/finish.js
```

启动命令在两个独立终端运行；隔离配置关闭Tauri内置beforeDevCommand，复用已有Vite。seed仅创建隔离数据；verify需按脚本预置页面运行，不可对正式数据执行。finish在重启后核验恢复及布局。本轮已正常关闭隔离程序并停止Vite，正式应用继续运行。

## Windows 文件与重启证据

本地证据保留在忽略目录 `output/playwright/workflow/`，脚本、fixture、SQLite与截图不进入Git。

- 首次进程13716正常关闭并确认退出，重新启动28408；文案显示“正常关闭后恢复的草稿”，封面显示“导出后继续编辑的草稿”。2条自动草稿、2个正式文案版本、3个封面版本及1个个人样式保留。
- 实际裁切为X≈0.42907132、Y≈0.39491832、缩放1.5；1440×1080、1080×1440、1920×1080三张PNG全量解码，恢复前后SHA-256一致。
- 完整备份 `shuyin-full-backup-6e5e121012702ad3615baf1d7c2c8909` 共13个文件、611513字节（文件合计，不含数据库/清单），包含测试视频；数据库及每个文件哈希核验通过。
- 恢复到 `restore/shuyin-restored-4ec56831d0486addbe2a3060c0bf9ee7`，素材可读，八文件任务包和视频路径、文案来源已重建，任务保持ready且检查项全通过。
- 恢复前库保留为隔离AppData `backups/shuyin-backup-3b2be12df836758d29b1d30539c36a55.sqlite3`。Rust回归另覆盖旧备份逐版本恢复、遗漏视频、原文件移除、同大小损坏、路径越界及失败不替换当前库。
- `evidence.json`、`inspection.json`、`covers.png`、`publishing.png`、`recovered-covers.png`记录结果；最后恢复/布局检查期间页面与控制台error为0。
- 正式库验收前后SHA-256均为 `e41393cca31f34ff0a919315101c8e1b080cf6d4b45b69dffef3ad688369aa9b`，正式应用进程17304及安装保留。隔离关闭后再次检查相同。

## 验证中修正

按真实新行为更新了旧测试中“离页必须放弃”和“新任务自动选最新文案”的断言；保留失败不丢稿、历史不覆盖等断言。修正SHA-256 0.11输出格式和SQLite数值映射编译错误；补充慢写入期间新编辑不误报已暂存的竞态回归。缩略图沿用data URL以符合现有CSP，未扩张图片来源。数据库/完整恢复选择互斥，取消选择保留原预览。

末次lint发现封面模板缩进warning，修正后lint通过。运行时收尾脚本首次精确匹配“主标题”失败，实际可访问名称为“主标题 *”，更正后通过；开发热更新触发的beforeunload对话框处理后重跑。均不算作首次运行成功。

## 未验证范围与下一步

本轮没有重新构建安装包、验证安装升级或发布Release，没有调用外部付费文本/图片模型，没有进行macOS/Linux实机及跨系统备份搬移。Windows验证为真实WebView2、Tauri IPC、SQLite与文件系统；原生文件选择器本轮未走端到端。测试视频仅51字节，用于文件复制/引用校验，未验证真实成片播放、大视频吞吐或目标平台发布。

自动恢复覆盖已提交SQLite的编辑，强制终止前尚在400ms等待窗口的输入及按项候选预览不承诺恢复。书籍表单和发布任务仍手动保存，符合本轮范围。

下一步：用真实书籍和成片试用这些入口，再执行本轮Windows安装升级及原生选择器验收。后续发布按兼容新增功能 `feat` 处理，同批仅升一次minor，由既有流水线决定版本。
