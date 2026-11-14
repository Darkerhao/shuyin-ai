# 章节笔记遮挡音频入口修复

日期：2026-11-14。基线：dev / 48e39b7。归类：fix / patch；不手动升版本。

## 现场与根因

用户在《三国演义》第二章的音频任务中遇到“等待来源处理”超时。应用内 Notebook 已显示完整章节笔记。用户指出右上角展开/收起图标可以回到音频入口；现场点击确认，该控件的实际语义是“关闭笔记视图”，关闭后显示 Studio 和“音频概览”。

在当前真实 Notebook 网站用不含用户资料的测试章节复现：

1. “复制的文字 → 插入”创建来源中的笔记，并自动打开笔记详情。来源行的可访问名称取正文首个非空行，与本地章节标签不同。Notebook 文档计数显示 0，但查询框与音频对话框显示 1 个可用来源。
2. 原 `sourceIsReady` 全页搜索本地标签或正文前 80 字；标题不一致、正文按段落渲染时无法命中，返回 null，一直等待到超时。
3. 即使通过来源检查，原 `configureAudio` 也没有关闭笔记详情，因此找不到被遮挡的音频入口。
4. 当前音频对话框同时提供“稍后生成”和“立即生成”。原来以“生成”作包含匹配，会得到两个候选并停止。

## 最小修复

- `src-tauri/src/audio_page.js`：只在网站实际 `.single-source-container` 来源行内按可访问名称/完整标题识别本次来源；文本增加正文首行匹配。删除全页文本搜索、通用列表项匹配与猜测来源父节点的逻辑。
- 检查整行处理状态及选择框；未勾选时即时提示，笔记正文和页面总数不作为完成依据。
- 配置音频前点击“关闭笔记视图”并确认关闭，继续原有格式、简体中文、默认时长和完整指导回读。
- 生成按钮只接受明确标签的精确匹配；“立即生成”可以提交，“稍后生成”不能作为提交入口。
- `verify_remote` 继续只读核对，不关闭笔记、不重新上传、不再次生成。超时保留原来源处理阶段，继续后沿用原任务。

无依赖、权限、数据库结构、迁移或持久状态变更。整书与章节仍共用同一个音频执行器。

## 验证与边界

修复前命令：

- `node --test --test-name-pattern="recognizes a pasted chapter|chapter note text|returns from the pasted chapter" tests/audio-page.test.mjs`：3 项均失败，分别为 null 来源状态、无法识别处理状态、`audio_overview_missing`。
- `node --test --test-name-pattern="current audio dialog submits" tests/audio-page.test.mjs`：失败，`ambiguous_control`，两个生成按钮无法区分。
- 自查补测 `node --test --test-name-pattern="exact source title" tests/audio-page.test.mjs`：先发现处理提示旁的来源名未识别，再补齐来源行内 span 标题匹配。

真实网站脚本在忽略目录 `output/chapter-source-fix/round-*.mjs`，按以下格式执行：

`ego-browser nodejs -e "await import('file:///D:/MyWork/project/JIAZI-PROJECT/shuyin-ai/output/chapter-source-fix/round-11.mjs')"`

- round-07/08：实际插入测试全文，处理结束后旧识别仍返回 null；记录当前来源行 DOM。
- round-09：确认关闭笔记控件的实际名称、笔记 DOM 和 Studio 恢复结果。
- round-11：重新打开笔记，用修复脚本连续返回 `source_ready`、`audio_configured`；完整指导、简体中文、默认时长已回读。
- round-12：同一测试笔记本只提交一次“立即生成”，返回 `generation_submitted`。
- round-13/14：网站真实产生 17:56 音频；脚本返回 `complete`，并捕获准确下载入口 `download_target`。截图与状态摘要保存在同目录。

直接访问用户任务的笔记本时，ego 当前账号显示无访问权限，因此在同一测试空间建立独立公开测试文本笔记本；未请求访问权限或更改分享。应用内原账号仅核对页面、关闭笔记详情，没有重新提交用户章节音频。

本轮未执行测试音频的原生下载入库、字幕、封面、成片或发布整链；网页生成成功不表示这些步骤全部验收。正在运行的安装版未替换，原任务/方案未修改。更新到修复程序后，原超时任务沿用“继续跟进”，无需新建来源或重做方案。

实现阶段自动检查（后续独立评审见下节）：

| 命令 | 结果 |
| --- | --- |
| `node --test tests/audio-page.test.mjs` | 56 项通过，0 失败；新增 8 项回归 |
| `npx vitest run tests/chapters.test.ts tests/automation.test.ts tests/generation.test.ts tests/ai.test.ts --maxWorkers=2` | 4 文件、118 项通过 |
| `npm run lint` | 通过，0 lint 警告 |
| `npm run tauri -- build --no-bundle` | TypeScript、Vite 及 Windows release 程序构建通过；保留原有 >500 kB 分块提示与 npm 用户配置提示 |
| `git diff --check` | 通过 |

实现阶段程序另存为 `output/chapter-source-fix/shuyin-ai.exe`，当时 SHA-256 为 `E8DB2FBF5E59F4C388643F2774BB9520DAC19D81574A74A81A2E04223E3D661E`；评审后的程序信息以下节为准。未生成安装器、替换安装程序、提交或推送。实现阶段未运行 Rust 单元测试、本机原生下载和安装升级。

验收对应：来源就绪、自动返回 Studio、完整指导回读、只点击立即生成、真实音频完成及下载入口均满足；章节资料/版本/自动任务相关回归通过。原用户任务的更新后桌面续作仍待验收，不将独立测试笔记本结果记作用户章节已完成。

## 独立评审与补测（2026-11-14）

固定基线为 `48e39b7474af1e0ff4f8818a6aadfc45264d5ab9`，范围为页面脚本、对应测试、技术设计、开发计划和本记录的未提交修改。两名独立评审分别检查工程规范与需求符合性；不把实现阶段记录当作本轮新执行的验证。

### Standards

首轮和最终复评均为0项待修。沿用单一页面执行器、取消与等待机制，未新增依赖、权限、数据库状态或另一套实现。整书来源的既有夹具同步实际来源行结构，原处理状态、只读恢复和禁止重复生成的断言保留。

### Spec

首轮发现1项P2：全页的 `li` / `[role="listitem"]` 候选会把笔记正文中的同名列表项当成来源。没有真实来源时误报就绪；真实来源存在时又误报重名，阻断音频流程。`node --test --test-name-pattern="note lists" tests/audio-page.test.mjs` 在修复前为2失败，分别复现 `true !== null` 与 `ambiguous_source`。

最小修复仅保留当前网站真实来源行选择器，并移除随之多余的 `row.matches` 判断；新增两个负例均通过。最终两路独立复评各运行12项相关回归并检查差异，结果均通过。Standards最终0项，Spec最终0项待修，原P2已关闭。

### 本轮实际验证

视频、发布和队列脚本在当前测试进程将已安装的 `D:/MyWork/software/shuyin-video-node/v24.22.2` 置于PATH首位，未修改系统Node配置。其余Node检查使用本机默认24.11.0。Rust命令先执行 `. ./.tools/env.ps1`。

| 命令 | 结果与执行范围 |
| --- | --- |
| `node --test tests/audio-page.test.mjs` | 评审修复前56项通过 |
| `node --test tests/audio-page.test.mjs tests/audio-package.test.mjs` | 最终修复后62项通过，包含新增的2项列表回归 |
| `npx vitest run --maxWorkers=2` | 37文件、536项通过；随后改动仅限页面脚本及其Node测试、文档 |
| `npm run test:video` | 40项通过，6项因未设置 `SHUYIN_TEST_FFMPEG` 跳过 |
| `npm run test:release` | 评审修复前93项通过，1项macOS专属打包检查跳过；修复后受影响的页面/打包部分已按上列62项重跑 |
| `npm run test:queue` | 19项通过 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked audio -- --nocapture` | 最终修复后49项通过，包含数据库音频恢复、章节快照和自动流程相关测试；251项未被该筛选选中 |
| `npm run lint` | 最终修复后通过，0 lint警告 |
| `node --check src-tauri/src/audio_page.js` | 最终修复后通过 |
| `npm run build` | 通过TypeScript与Vite构建 |
| `npm run tauri -- build --no-bundle` | 最终修复后通过TypeScript、Vite与Windows release程序构建 |
| `git diff --check` | 最终差异检查通过 |

最终程序已更新至 `output/chapter-source-fix/shuyin-ai.exe`，与 `src-tauri/target/release/shuyin-ai.exe` 的SHA-256一致：`9C7C219EC2AD50B3BE089861DC602C1255A71BC307BC07886AA00BCECE5245F9`。未生成安装器、替换已安装程序、手动升版、提交或推送。构建保留已有大于500kB的分块提示和npm用户配置提示。

真实网站复核脚本位于忽略目录 `output/chapter-source-review/round-*.mjs`，执行形式为 `ego-browser nodejs -e "await import('file:///D:/MyWork/project/JIAZI-PROJECT/shuyin-ai/output/chapter-source-review/round-05.mjs')"`。重新打开原公开测试笔记本，确认来源行为 `DIV.single-source-container` 且存在已选选择框；最终脚本返回 `sourceReady: true`、`sourceRows: 1`，摘要保存于 `live-readonly.json`。

首次导航等待load超时，但页面已提交并处于interactive，继续读取同页成功。尝试打开笔记时，语义按钮被网站span拦截，未打开笔记；本次真实网站只验证来源识别，不宣称重新验证了笔记关闭或生成链。没有重新上传、生成、下载或操作用户原章节任务；浏览器验证空间已关闭。

数据库仍为schema25，无迁移或数据改写。来源行识别、笔记正文隔离、原处理状态、音频配置与单次提交满足本地回归；最终桌面安装升级、用户原任务续作和原生下载入库仍待人工验收。建议在使用包含修复的桌面版本后，沿用原任务“继续跟进”，核对下载与归档结果。
