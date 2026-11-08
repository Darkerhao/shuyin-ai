# 自动制作与发布准备现场测试（2026-11-07）

## 最终结论（2026-11-08）

本次真实测试已完成到抖音官方待最终发布页，当前流程为`awaiting_publish/publishing`，同一发布任务为`ready`，`publish_started=1`、`publish_handed_off=1`，发布成功记录为0。用户最终点击、预约提交、平台审核及实际公开发布未执行，不能称为公开发布成功。

原书音频、字幕、分镜、文案、两次AI背景、双封面导出、完整1080p成片、官方视频上传、文案回读、禁止保存、不加入合集及未来01:11设置均有真实结果。本次因升级修复和草稿恢复，明确核对了账号、现有视频、处理完成与封面，因此不是一次启动后零人工点击通过。

最终官方时间为**2026-11-09 01:11（北京时间）**：测试跨过原候选时间，尚未交接前沿用原策略续期同一任务并重新生成任务包；交接后不再自动改期。官方页面明确支持2小时后至14天内，此时间被接受。最终正文选区与任务包经零宽字符及空行表示规范化后精确一致，证据为`output/automation-live-20261107/official-final-text.json`，`matchesTask=true`。

两张官方封面均与本地产物一致，平台显示“封面效果检测通过”；最终预览身份变化触发既有人工确认，目视核对后完成交接。右侧手机预览仍可见平台序列化的段落占位`*`，本次只证明编辑器正文与冻结文字一致，最终提交前应由用户检查手机预览；这项平台展示差异未以删除字符的规则掩盖。

交付限制：音频实际5分9秒，目标10分钟未达到；实测响度-21.6 LUFS偏低；仅抽帧检查，没有全片事实听审。官方页面现有“同时发布到红果短剧”及“记住我的选择”处于开启状态，本流程未新增控制，最终提交前用户需检查该选项和AI自主声明。本次未点击最终发布，也未记录虚假成功。

## 测试对象与验收标准

- 当前安装程序：`D:\书引 AI\shuyin-ai.exe`，界面版本v0.53.0；源码基线`b96fad6`。
- 新建独立书籍：`显微镜下的大明（全流程测试）`，作者马伯庸；原书为当前书单中的`186显微镜下的大明.epub`。原有16本书与发布记录保持保留。
- 入口默认目标10分钟；沿用已选moonupAPI文本/图片配置；双比例AI封面、视频、未来北京时间01:11、禁止保存、不加入合集。
- 验收以真实NotebookLM、实际生成文件和抖音官方页面为准；最终发布和真实发布成功记录由用户完成。

## 实际已完成

1. Windows桌面表单、原生文件选择、书名作者和原书关联；账号入口打开已登录的抖音官方上传页。
2. 首次账号读取提示进入上传页后再读；页面就绪后读取与本次账号确认通过。官方页未返回昵称，应用显示核对头像。
3. 勾选自动制作授权并仅启动一次。创建书籍与流程后，实际报错：`no such column: publish_started`。未创建音频/视频子任务，未进入模型调用或上传阶段。
4. 通过应用“高级维护→创建备份”取得运行中连接的一致性快照：`output/automation-live-20261107/shuyin-backup-c08e90016cc2c51664ffa087054a696e.sqlite3`；完整性检查为`ok`。

流程ID：`bc6a1a15ba8efe660050c734cc67ba48`；书籍ID：`5a4d040a6b70065b9df63570075993ed`。备份记录现场为schema20，表中缺少`publish_started`和`publish_handed_off`，原流程为`running/audio`，所有子任务ID为空。

## 根因与修复

已标记为v20的早期表不会重新执行后来补充字段的v20建表脚本，当前查询却已依赖新字段。新增v21迁移，统一两种v20的表结构，保留已有标记、原流程和关联。启动和恢复共用迁移；旧备份保持只读，备份结构校验只精确接受已观察到的旧形态。迁移失败必须保留旧表与版本。

回归`automation_legacy_v20_reopens_and_restores_without_losing_the_run`先在旧逻辑复现同一SQL错误，修复后通过。另覆盖已有上传/交接标记、重复重开、旧备份恢复、迁移失败回滚和拒绝额外字段。

## 当前验证记录

- `npx vitest run tests/automation.test.ts tests/audio-page.test.mjs tests/projects.test.ts tests/publishing.test.ts tests/douyin.test.ts tests/douyin-editor.test.ts --maxWorkers=2`：实际选中5个Vitest文件、120项通过；`.mjs`音频页面测试不在Vitest发现范围，不计为已运行。
- `cargo test --manifest-path src-tauri/Cargo.toml --locked automation`：修复前14项通过；新增首批回归后16项通过。
- `cargo test --manifest-path src-tauri/Cargo.toml --locked automation_legacy_v20_reopens_and_restores_without_losing_the_run -- --nocapture`：旧实现实际失败，错误与现场一致。
- `npm run build`：类型检查和前端构建通过，仍有既有大分块提示。

Rust命令先加载`. ./.tools/env.ps1`，本轮临时目录使用`output/automation-live-test-temp`。初次新增测试缺少`Path`导入、首次修复有Rust借用生命周期错误，均修正后再运行；编译失败不算测试通过。

## 真实服务与成片进展

现场备份经完整性校验后保留，重开时磁盘库比运行中快照旧；两份库分别保存为上文备份和`output/automation-live-20261107/disk-before-live-snapshot-restore.sqlite3`，未将较旧磁盘视为最新现场。恢复现场快照后由修复版实际升级为v21，17本书、12条原发布记录和本次流程保留。升级后数据库完整性为`ok`，原流程按启动规则暂停，无自动重放。安装位置程序已替换为本机编译的修复版，原程序保存在同一证据目录的`shuyin-ai-before-fix.exe`，界面版本仍为0.53.0；这不等于已发布新安装包。

恢复同一流程后真实NotebookLM音频完成并归档：`446e7ffecb19a2ed8d624c8e066acfaa`，AAC双声道44.1kHz、308.569977秒、9,948,427字节。目标为600秒，实际约5分9秒，未自动变速或重新生成补时。SHA-256为`20510c70e12c4ad0359388f951f147c4402a0aada6c227e6fc839c2648c11cfe`。本轮没有完整事实听审，不将字幕校对当作音频无误证明。

字幕首轮模型调用遇到network错误，已有转写保留。当前服务可连通后，通过应用明确重试当前阶段：132条字幕完成校对，29个分镜、基于实际字幕的1份文案、2次真实图片调用和2份封面导出完成；横封面1440×1080、竖封面1080×1440，均已查看PNG。背景供应商实际输出1536×1024与1024×1536，最终Canvas按所需比例导出。

视频任务`b47e5d77fc752e79883c23f6b8e28c76`实际渲染成功，成片`run-3-1794048327739915400/video.mp4`为308.583333秒、1920×1080、24fps、H.264 High/yuv420p、AAC双声道、26,058,142字节；音轨308.57秒，与原声长度一致。最终SRT为132条。已生成并查看6帧联系表`output/automation-live-20261107/video-contact-sheet.png`，中文字幕、手绘主体及分镜可见；这是抽帧验收，不等于全片逐帧或完整试听。

`ffmpeg-skill/scripts/probe.py`分别核验音频与成片，退出0。`look.py --tiles 3x2 --width 1920`生成联系表，退出0。`check.py --platform tiktok --json`退出1：预设要求竖/方画幅，而抖音官方上传页明确支持16:9并建议中长视频横屏；本片横屏符合应用当前规格，不据该海外TikTok预设改画幅。实测响度为-21.6 LUFS、真峰值-3.7 dBTP，比该预设-14 LUFS偏低，保留为质量待确认项；未改写原声或成片。预设也提示无内嵌字幕轨，本片字幕实际烧录在画面，另附SRT。

发布任务`3c96820b35fa192a1e20bc5153841e6d`与任务包已就绪，保存权限false、北京时间`2026-11-08T01:11:00+08:00`，两项人工确认false；未创建发布成功记录。因本次修复需要重开窗口，账号再次核对通过，官方视频上传及处理完成；后续重开使用官方暂存草稿及明确现有视频/处理完成核对，没有再次选择文件上传。

## 官方文案填写缺陷

首次官方填写丢失正文全部8个换行，245个非换行字符均在，但253字符的预期文案回读不一致。尝试逐行`insertText/insertParagraph`在真实受控编辑器中出现只保留首段，增加异步间隔后仍出现选择位置变化、片段重排；这两次现场尝试失败，不记为通过。随后尝试一次`text/plain`的paste事件，仍未作为最终交付保留；没有以改DOM或忽略实际文字差异绕过验证。

尝试中曾用host CDP等待Promise，最终已恢复原同步页面观察，只在正文输入复用已有host CDP，不开放调试端口。就绪且未修改的任务续接保留冻结输入和人工确认字段，并仍等待有效视频元数据；嵌入辅助卡在失败时提供“重新核对当前网页”，恢复同一任务而非重传。自动流程明确继续时可续接同一失败网页会话，重启后没有会话仍要求先人工核对。

一次纯文本paste事件方案也没有作为最终实现保留。最终代码只保留`prepareText`核对输入/选中正文，再由已有host CDP发送一次`Input.insertText`，并在宿主回读全文；不再包含脚本`insertText/insertParagraph`或paste并行路径。编辑器用零宽字符表示空行，草稿重开又可能产生`*`占位，因此输入合并连续空行为单换行，保留正文、顺序、段落边界与标签；比较仅忽略零宽字符、CRLF和重复空行，仍拒绝丢失换行、重排、增删文字。

新增用例覆盖原生输入准备、正文选区、空行规范化、重复继续不再输入标签、就绪自动任务不重写确认、失败卡恢复。Rust回读覆盖等价段落间距并拒绝丢失段落边界。最终官方页面正文、封面和发布设置已按本报告顶部结论完成验证；手机预览占位仍列为提交前检查项。

## 追加工程验证

- `npm run test:rust`：244通过、2项既有专项忽略。
- `npm run check:rust`：fmt/Clippy通过。
- `npx vitest run tests/douyin-editor.test.ts tests/douyin.test.ts tests/publishing.test.ts tests/automation.test.ts --maxWorkers=2`：最终4文件92项通过。
- `npm run lint`、`npm run build`、`git diff --check`：通过；构建保留既有大分块提示。
- `npm run tauri -- build --no-bundle --config output/automation-live-20261107/tauri.test.conf.json`：本地release可执行程序构建通过。
- `cargo test --manifest-path src-tauri/Cargo.toml --locked automatic_schedule`：1项同任务续期/交接后不改期回归通过。
- `python output/automation-live-20261107/inspect-state.py`：最终同一音频/视频/文案/双背景/双封面/发布ID，v21、完整性`ok`、外键冲突0、实际发布记录0。详情见`current-state.json`。

前端准备函数曾因跳过元数据等待造成3项现有回归失败，修复时保留等待及有效成片校验后通过；无删除测试或弱化检查。正文多种不可靠输入尝试均已删除，最终只保留宿主原生输入。

## 修改与交付

- 数据库：`src-tauri/migrations/0021_automation_publish_markers.sql`、`src-tauri/src/database.rs`、`src-tauri/src/database/backup.rs`及`automation_tests.rs`。
- 发布：`src-tauri/src/automation.rs`、`douyin.rs`、`douyin_flow.rs`、`douyin_editor.js`，以及`PublishingPage.vue`、`DouyinVerification.vue`和3份前端测试。
- 文档：数据库、技术设计、开发计划及本验收报告。
- 本地安装位置`D:\书引 AI\shuyin-ai.exe`已换为测试通过的修复版，旧程序及升级前数据库已保留在本轮证据目录。现场测试结束时UI版本仍为0.53.0，尚未提交、推送或发布安装包。本次修复按fix/patch，不手动升版本；11月8日用户追加授权先拉取代码、中文提交推送dev，再cherry-pick并推送main，实际Git交付结果另行回报。

## 下一步

保持应用和官方窗口打开，用户检查成片、音量、手机预览、AI声明、红果同步选项及时间后亲自提交。定时预约提交不等于实际公开发布，作品实际发布后再回应用记录真实结果。一次启动零人工确认的完整再验收应另选新书执行，不复用本次修复过程冒充无人值守通过。
