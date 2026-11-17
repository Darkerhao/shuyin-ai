# 铅笔素描逐笔视频与绘图引擎计划

## 当前定义与实施（2026-11-17）

用户明确此前“水墨画”指铅笔素描逐笔成形。当前方案已收敛为：原声音频/字幕 → 铅笔素描或书卷手绘 → 原结构化分镜 → 同一 Frametell 路径与时间轴 → 试片/成片。内部沿用 ink/classic；使用单一 pencil-style.js / pencil-look.json 替换前序尚未提交的本地毛笔与墨块，不增加第三种 theme、绘图依赖或图片请求。

本轮实现、实际验证和边界见[铅笔素描验收](PENCIL-VIDEO.md)。下文保留前序水墨方向与引擎调查作历史记录，其中“毛笔/淡墨/水墨分镜”等名称不再描述新素描画风。引擎升级、真实书籍长片、安装版与跨平台仍单独验收。

## 前序双画风与引擎调查（历史记录）

日期：2026-11-17。状态：本地风格链和新任务零生图已实施；完整业务、长片和安装版仍待验收。

本次实现基于当前外部 Frametell 快照，没有安装或切换未发布的0.8.0。新增项目自己的 `ink-look.json`、`ink-style.js` 和水墨分镜规则；无图片模型的 `theme=ink` 现在使用原生 `ink-wash` style + `META.look` + 本地逐笔路径。`theme=classic` 仍使用原 `study` 与原物体绘制。新任务不接受 `imageProviderId`，旧图片任务保留读取。

## 1. 结论与目标

可行。保留可独立选择的 **水墨画** 和 **书卷手绘（原风格）**，沿用现有 `theme=ink/classic` 字段、同一绘图引擎和任务工作流。风格同时影响分镜导演规则、构图与本地画笔效果，不止改变颜色。

以项目已经接入、声明为 Frametell 0.7.3 的本机快照为当前基线，将官方v0.8.0固定标签源码作为唯一升级候选，隔离验证后决定是否采用（详见4.4）；在现有逐笔绘制模块中补强水墨笔触、器物细节和空间构图。新水墨视频根据实际字幕生成结构化分镜，再由本地代码直接画出内容；取消每镜生成图片、从图片提取轮廓和显现原图的制作步骤。依赖升级与本项目水墨画笔增强分别验收，不能将引擎换版本当作水墨画质已经提升。

唯一推荐流程：

```text
原声音频 + 原字幕时间轴
  → 选择画风：水墨画 / 书卷手绘（原风格）
  → 共同叙事约束 + 所选画风规则，生成主体 / 场景 / 动作 / 字幕锚点
  → 校验并保存现有分镜 JSON
  → 现有绘图引擎逐笔画线、皴擦、铺淡墨与局部细节
  → 同一渲染器输出试片 / 整片
  → 现有 FFmpeg 合入原声、字幕、响度处理并归档
```

新水墨视频的创建、续作、单镜调整、试片与成片，图片模型调用次数均为 **0**。原风格也以本地绘制制作分镜，不因新增水墨能力增加图片请求。已保存分镜后的绘画与视频合成在本地完成；文本分镜、字幕校对仍可能产生文本模型费用，本地编码仍需要计算时间。

当前已验证的是本地视频渲染链和短片夹具；真实书籍内容、自动全流程、15分钟连续片、安装版和跨平台仍未验证。

本轮仅制定计划与验证可复用基础，没有生成新样片、修改产品代码或发起模型请求。

### 1.1 两种画风各自保留的能力

| 选择项 | 分镜规则 | 本地绘制 | 典型用途 |
| --- | --- | --- | --- |
| 水墨画（`ink`） | 以具体场景和主体关系组织镜头，重视前中后景、疏密留白、局部细节与连贯意象 | 毛笔提按、飞白、皴擦、淡墨、细线和宣纸层次 | 文学、历史叙事及适合写意场景的读书内容 |
| 书卷手绘（原风格，`classic`） | 保留现有物体/动作/对照/路径的讲解方式与短标签 | 原有配色、几何、线稿、标记和动作 | 概念、关系、观点和结构化讲解，也可继续用于原有读书内容 |

两种画风均能独立选择、生成整批分镜、重新生成单镜、编辑、试片及成片。水墨能力不会替换原风格；同一段音频可分别制作两个版本进行比较。新建默认值沿用当前水墨选择；旧计划缺少theme时仍按classic读取，不新增第三种“自动风格”或全局强制转换。

### 1.2 截图位置与交互安排

用户截图红框为第02阶段“细致水墨画”，顶部同时显示“画作模型、已保存0/0张”。它反映当前图片制作路径，应随新流程一起调整：

1. 在“生成字幕与分镜”按钮上方、以及用原字幕制作新版的入口旁，就地展示 **分镜画风：○ 水墨画　○ 书卷手绘（原风格）**。两项使用原生单选语义和现有样式，不增加组件依赖；保留清楚的选中、键盘和焦点反馈。不再只把画风藏在可折叠运行设置里。
2. 使用现有待新建任务的theme值作为唯一选择状态。选择不会发起模型调用；点击生成才将画风冻结进本次计划。已有任务旁显示“本次画风”，其值来自已保存plan，不从新任务选择器推断。
3. 进度条第02项按本任务theme显示 **水墨分镜 / 手绘分镜**；后续统一为“确认分镜 → 画面渲染 → 成片归档”。进度条继续表示执行阶段，选项放在生成入口，不把进度步骤改成能改变运行任务的按钮。
4. 新本地任务把“画作模型、已保存N张”改为“本次画风：水墨画/书卷手绘 · 本地逐笔绘制”，分镜阶段按已覆盖字幕和已保存镜头显示真实进度；不显示尚不存在的画作张数。
5. 当前任务生成、重试和单镜修订始终沿用其冻结画风。更换画风通过“用原字幕制作新版”，清楚标出目标画风，原声与已有版本保留。
6. 当前remake按钮仅对succeeded任务展示（`VideoPage.vue:732`）。实现时扩展至已停止/失败/待确认、且音频与已保存SRT齐全的手动任务，避免旧缺图任务无路可走；运行任务先停止。自动任务仍遵循原活动流程互斥规则，不绕开全局绑定。
7. 自动制作开始区提供同样两个选择，启动后从冻结输入显示实际画风。页面切换和应用重启恢复当前任务画风；修改下一版本选项不能改变当前任务的阶段标题和成果。

截图用于说明交互需求。截图安装版显示v0.67.1，本次源码package.json为0.67.0，不能仅凭截图认为本地源码就是当前运行安装版；本轮不操作或更新安装版。

## 2. 当前为何需要生成图片

当前实现为了获得较细致的完整画作，选用了先生成成画、再逐步显影的路线。图片依赖是应用这一制作路径引入的，并非现有绘图引擎的要求。

| 位置 | 当前实际行为 | 计划处理 |
| --- | --- | --- |
| [VideoPage.vue](../src/modules/video/VideoPage.vue:213) | 新建水墨任务要求图片模型并传入 `imageProviderId` | 新建水墨只要求原有文本模型与本地视频环境 |
| [database/video.rs](../src-tauri/src/database/video.rs:318) | 后端同样要求手动水墨任务提供图片模型 | 从创建入口移除逐镜图片要求 |
| [video_commands.rs](../src-tauri/src/video_commands.rs:840) | 有图片模型时切换到画作描述协议，随后按镜执行 `video_ink` 图片任务 | 使用可信绘图目录的分镜协议，结束后直接进入分镜确认 |
| [ink-painting.js](../src-tauri/video/ink-painting.js:39) | 从源图读取像素、采样轮廓，并随时间显示原图 | 新水墨不进入该路径 |
| [AutomationSetup.vue](../src/components/AutomationSetup.vue:45) / [automation.rs](../src-tauri/src/automation.rs:410) | 新自动水墨通过 `detailedInk` 给视频任务绑定图片模型 | 新自动视频使用与手动相同的本地绘制方式 |

此前隔离样片的 [ink-film.js](../output/ink-sketch-sample-20261115/ink-film.js) 也存在加载参考图、检测边缘的步骤；其路径提取方法不作为新方案的绘画来源。新样片须直接使用本地受信任几何与笔触，不依赖 AI 图片或参考位图。

## 3. 已有工具能复用什么

已检查实际安装目录 `D:/MyWork/project/JIAZI-PROJECT/whiteboard-teacher-video/engine` 的 package.json、README 与 runtime 源码，声明版本为0.7.3。追加官方源码比较发现该目录包含混合版本文件，不能仅凭package.json把它视为纯0.7.3，详见4.4。

| 已有能力 | 当前来源 | 复用方式 |
| --- | --- | --- |
| 逐笔路径、直线、曲线与图层 | [illustrations.js](../src-tauri/video/illustrations.js:75)；引擎 `runtime/core.js:103`、`:113`、`:219` | 延续 `P.path/line/curve/layer` 及原动作时钟 |
| 稳定随机种子和按路径长度显现 | 引擎 `runtime/core.js:90`、`:139`、`:191` | 笔触构建一次；重复取帧和回退时保持相同形状 |
| 本地纸纹、水彩填充和排线 | 引擎 `runtime/main.js:6`、`runtime/core.js:120`、`:214` | 复用已有纸墨基础，补充水墨绘画需要的局部层次 |
| 93种物体、7种环境、13种构图类别、6种动作 | [visual-catalog.json](../src-tauri/video/visual-catalog.json) | 保留单一目录，按样片实际缺口补少量素材 |
| 实际字幕对应主体出场时间 | [video.rs](../src-tauri/src/video.rs:48)、[render.mjs](../src-tauri/video/render.mjs:59) | 保留每镜 first/last 与 beat.cue |
| 试听、试片、整片、缓存、归档与响度 | [video_commands.rs](../src-tauri/src/video_commands.rs)、[render.mjs](../src-tauri/video/render.mjs) | 在原工作流内替换画面制作方式 |
| 单镜重新生成与人工修改 | [video/revision.rs](../src-tauri/src/video/revision.rs)、[VideoSceneEditor.vue](../src/modules/video/VideoSceneEditor.vue) | 本地水墨继续编辑主体、场景、景别、动作和字幕位置 |

93种物体已有可识别线稿，但部分对象如山、树仅包含少量几何。现有水墨主题主要调整颜色、透明度和底色；直接取消图片调用不能自动获得细致画面，需要同步补强绘画表达。

## 4. 绘画实现方式

### 4.1 在同一绘图模块补足笔墨

在 `illustrations.js` 内增加职责明确的小型水墨笔触函数，继续由现有 `draw(P)` 调用，不另建绘图服务、插件系统或渲染队列。

- **提按粗细**：对本地受信任中心路径在构建时做有限采样，形成起收较细、中段可变化的笔宽轮廓。用原逐笔中心路径的进度控制 SVG 遮罩，令墨线沿落笔方向出现。
- **枯湿浓淡**：在同一笔画内用固定种子的细缝、少量叠笔和局部透明度表现飞白与墨色；淡墨填充继续使用引擎已有水彩能力。避免把任意模糊当成纸上晕染。
- **细节与皴擦**：书页边、木纹、衣褶、枝叶、山石和水纹由确定的本地曲线构成，跟随对应主体的时间与图层出现。细节不是新增叙事主体。
- **稳定时间**：笔压轮廓、细纹与随机变化在镜头构建时生成，取帧只更新显现程度。引擎 `liveRough` 会逐帧清空并重建动态几何，不将全部静态细节放入该循环。

这里仍使用当前 SVG、Rough.js 与引擎时钟。新增笔触只负责现有绘图 API 尚不具备的毛笔表现；复用已有路径显现、图层和纸纹，不复制完整引擎。

### 4.2 让画面有具体内容和空间

先细化样片真正需要的常用对象：书卷、书案、人物背影、门窗、树木、山石、河岸。其他对象继续使用原目录，不一次性重画全部93种素材。

水墨构图以完整场景为主：先建立主要轮廓，再铺淡墨，随后出现细节和重点浓墨。前中后景的层级、留白与景别共同表达字幕中的主体关系。人物、器物或环境没有在当前旁白成立时，不因为“画面丰富”提前加入。

复用原 `environment/framing/kind` 决定局部布局；水墨画面扩大主体区域，压缩现有大标题、横线与标签占用。字幕和必要标题保持在固定层。书卷主题继续使用原绘制几何，水墨细化逻辑不改变其参考图形。

保留每镜最多3个叙事主体。该上限限制画面要表达的对象数，不限制每个对象内部的笔画数量。

### 4.3 文本模型负责分镜，本地程序负责画笔

两种风格共用 `first/last/kind/environment/framing/beats` 协议，以目录中的可信对象与动作表达实际内容。保留一份共同的原声/时间/事实/目录约束；按theme加载一份短小的水墨或原手绘规则。原手绘的导演规则独立保留，水墨规则可强调场景空间及局部细节，不复制整套Prompt或另建任务执行器。

整批生成和单镜修订都由后端读取已保存 `job.plan.theme`。当前 `storyboard_prompt()`、`storyboard_input()` 和 `video::revision::prompt()` 没有画风参数，修订链 `PreparedSceneRevision` 只返回provider/user；实现时须把同一已保存画风贯穿这两条调用链。单镜候选不得返回或改变theme，来源摘要继续覆盖完整计划，防止另一版本的候选串用。手动新建、自动新建、失败续作、单镜调整、原字幕制作新版均走相同规则组合入口。

模型不输出代码、任意 SVG、图片路径或上万条坐标。程序根据已验证的对象、景别、动作与字幕锚点执行绘制，图形细节由本地素材决定。无需新增“自由绘画指令语言”。

对于目录暂时无法准确表达的具体场景，优先使用旁白支持的主体近景或中性意象，并在样片复核时补真实缺失素材；不得以任意山水替代整段内容，也不转入图片生成。

取舍是将逐镜图片费用转为一次性的本地笔触和素材建设工作。画面细致程度、可表现题材与人物动作丰富度取决于受信任素材库；先交付适合读书讲解的水墨线描与写意场景，通过真实样片判断效果，不能承诺仅靠93种现有线稿画出任意复杂情节。

### 4.4 绘图引擎升级：固定0.8.0源码候选，先隔离验证

2026-11-16已直接查询官方GitHub/registry元数据、CHANGELOG、绘图API与实际源码，核实如下：

| 对象 | 当前核实事实 | 本次安排 |
| --- | --- | --- |
| 本机引擎 | package声明0.7.3；core/live/main内容等于上游0.8.0，engine/boot/render/project仍等于上游0.7.3/0.7.4 | 保留完整本机快照，不原地覆盖或拼接升级 |
| 官方最新GitHub Release | [v0.7.4](https://api.github.com/repos/sitor-ai/frametell/releases/latest)，非预发布 | [0.7.3→0.7.4比较](https://github.com/sitor-ai/frametell/compare/v0.7.3...v0.7.4)中engine仅改包版本，对当前绘图没有实质增益，不安排这一步升版 |
| 官方源码升级候选 | [v0.8.0标签](https://api.github.com/repos/sitor-ai/frametell/tags?per_page=20)，固定提交 `36b15a8fd4094dc3274a6cd4158c87bd0bdc95c9`；[CHANGELOG](https://github.com/sitor-ai/frametell/blob/v0.8.0/CHANGELOG.md)列出该版本 | 获取完整固定源码快照，只将其称为候选；其[GitHub Release条目](https://api.github.com/repos/sitor-ai/frametell/releases/tags/v0.8.0)查询404，不声称已有正式安装资源 |
| Rough.js | 本机安装和锁文件均4.6.6；[官方registry](https://registry.npmjs.org/roughjs)的latest亦为4.6.6 | 维持4.6.6，无版本升级需求 |

Frametell的engine在官方manifest中为private，npm的[该包元数据](https://registry.npmjs.org/@frametell%2fengine)返回404；不使用 `npm install @frametell/engine@latest` 作为计划升级方式。来源及文件内容比较见[引擎升级研究记录](../output/ink-stroke-plan/engine-upgrade-research.md)。

0.8.0主要增加对象编辑覆盖、竖屏相关能力和部分绘图API修复，现有横屏视频不必接入完整Studio或新增竖屏模式。[0.8.0绘图API](https://github.com/sitor-ai/frametell/blob/v0.8.0/packages/agent/prompts/API.md)与[core实现](https://github.com/sitor-ai/frametell/blob/v0.8.0/packages/engine/runtime/core.js)仍以固定笔宽、粗糙线条和填充为基础，不包含现成的毛笔提按/水墨模拟。部分绘图修复本机已存在，不能重复算作升级收益。本次升级价值是得到来源清楚、版本完整的快照并验证新API兼容性；水墨效果仍按4.1—4.3实现。

实施步骤：

1. 记录当前引擎和所用依赖锁文件的摘要，保留本机完整目录与其匹配的应用基线。在独立目录取得上述固定SHA的完整0.8.0引擎，不改用户现有外部工程及未跟踪素材。
2. 阅读并核对目标版本的Builder、图层、Proxy调用、`linkProject/engineModulesDir`、boot、取帧/字幕、lint和渲染输出契约；本轮仍验证1920×1080、24fps，不因升级引入新视频规格或上游图片生成Agent。
3. 在隔离应用适配中更新 `runtime.mjs` 的精确0.7.3门禁、`VideoEnvironmentCheck.vue` 等版本说明、环境检查与测试夹具。最终采用的版本须精确固定，不改为接受任意 `>=0.7.3`，也不通过篡改第三方package版本绕过校验。
4. 用一份引擎快照指纹统一参与Node缓存与Rust试片签名：内容包括实际版本、关键绘图/渲染文件及所用依赖锁文件摘要，在已有环境/probe边界计算并复用，不逐帧扫描或新增数据库表。当前Node缓存只包含引擎路径，Rust签名只包含plan和本项目代码；同目录换文件或切换引擎后都须拒绝旧试片与缓存。
5. 用同一音频/SRT分别验证两个画风的真实引擎运行、原声锚点、中文字体/路径、重复取帧/回退与预览整片PNG一致性，再验证连续片段耗时、内存和零图片请求。保留原手绘参考几何，不能只跑mock。
6. 通过后将本次使用引擎统一切到已验证0.8.0快照，只有一个执行器和一份所选引擎配置；旧目录仅为工程回退备份。若回退，恢复与旧引擎匹配的应用版本门禁/适配及旧路径，再使缓存失效，不能声称在仅接受0.8.0的应用中只改路径即可回退。

若0.8.0隔离验收不能通过，则不切换引擎；双画风分镜与本地笔触建设继续在当前已核实API上完成，不回到图片生成。这是同一升级流程的验收门槛，不长期维护两套渲染实现。

## 5. 工作流、历史任务与费用边界

### 5.1 新任务

- 手动新建水墨移除图片模型必填、逐镜张数和补图进度，使用原有“字幕与分镜 → 确认 → 试片/成片”流程。
- 新水墨计划不保存 `imageProviderId`，新分镜不得携带 `painting` 或 `image`。服务端与渲染编译边界均检查，不能仅通过隐藏页面控件控制制作路径。
- 单镜调整重新生成本镜结构化分镜；本地重画不调用图片模型。修改字幕和时间码后按既有规则保存、重建分镜并使旧试片失效。
- 自动制作的视频阶段与手动共用同一执行器，不为自动水墨建立第二套绘制协议或图片任务。
- 新建的theme在整个任务中固定：整批分镜、单镜修订、进度文案、预览和成片取同一已保存值，classic与ink各自可用。不同画风或不同引擎构建不能复用同一份试片或渲染缓存。
- 继续复用 `video_jobs.plan_json`、草稿、run_id、渲染签名与缓存。计划不新增SQLite表、列、迁移、权限或绘图依赖种类，schema维持25；若核实后升级已有引擎，其版本与锁文件影响独立记录。

### 5.2 已有任务与清理

- 已保存音频、字幕、图片和成片保留，不批量清库、改旧计划或覆盖原视频。
- 已生成全部图片的旧任务只读取已有素材进行查看和本地渲染；编辑不能触发新图片请求。
- 缺少画作的旧任务停止补图，提示通过现有“用原字幕制作新版”进入本地水墨流程，复用原音频/SRT。属于旧自动流程的任务，先停止旧流程再制作新版，避免活动流程与新手动任务争用。
- 新自动请求不再创建逐镜画作配置；旧自动冻结 JSON 中的 `detailedInk`、旧视频计划中的 `imageProviderId` 仅保留必要读取，不再作为新增视频图片请求的许可。自动流程 `selection.imageProviderId` 继续用于原封面。旧自动流程若已绑定缺图任务，明确等待处理，不能静默继续收费。
- 删除旧水墨的图片创建/补画循环、专用生成 Prompt、绑定写回与失效重试生产逻辑；仅保留读取历史计划、历史任务展示及已有素材渲染所需代码。图片 Provider 其他用途按原模块管理。

### 5.3 封面是独立费用范围

当前整书/章节完整自动流程仍会制作三比例封面，其图片模型配置还用于封面背景。此次计划取消的是**水墨视频逐镜图片生成**，不能因此直接删除自动流程的封面图片模型配置，也不能声称整个发布流程零图片调用。

在验证中分别统计 `video_ink`、普通视频插画及封面请求：新水墨视频不得借用任何图片用途绕过零生图要求；原封面不因视频镜头数增加请求。封面改为本地绘制属于另一个业务范围，本计划不扩大处理。

## 6. 分阶段实施

| 阶段 | 具体工作 | 交付与通过条件 |
| --- | --- | --- |
| E0：引擎版本与兼容基线 | 已核查官方Release/标签与本机混合快照；以完整v0.8.0固定SHA作唯一升级候选，在独立目录验证API/缓存/原声输出 | 来源及门禁适配清楚；原目录与匹配应用基线可回退。未通过不切换，双画风功能可在当前引擎完成 |
| P0：双画风验证画面 | 在确定的引擎绘制链内，用同一段30—45秒音频分别制作水墨、原手绘短片；水墨约3个对应场景，补足笔压、淡墨和细节 | 两份MP4、同时间对照取帧、绘制耗时与资源记录。两种画风可辨且忠于旁白；水墨无源图片或图片模型请求 |
| P1：双画风分镜与取消生图 | 生成入口并排单选、冻结theme，贯通公共约束与独立画风规则；调整手动/自动创建并删除逐镜图片生产步骤 | 两种画风均能单独生成；进度第02阶段随任务画风显示，新水墨无图片模型也能完成 |
| P2：打通编辑与内容 | 完成同画风单镜修订、字幕变更、草稿、续作、新版切换、缓存；按样片内容补少量素材，更新状态及费用说明 | 编辑/保存/重启保持画风；同一音频可有两种独立版本，改一份不影响另一份，整书/章节归属准确 |
| P3：验收与文档 | 完成工程检查、组件/后端/绘制回归、真实引擎双画风短片与至少一段15分钟以上水墨连续片段，更新主文档与使用说明 | 零生图调用、时钟一致、引擎版本与画风无缓存串用；原风格回归和性能记录可复核 |

P0的目的就是先验证程序绘制的细致程度与本机成本；不能用“技术可以画”替代实际观看，也不能通过恢复生图来满足样片效果。15分钟是连续渲染验收用例，不是产品时长上限。

## 7. 预计影响文件

| 范围 | 预计修改 |
| --- | --- |
| 绘制与编译 | `src-tauri/video/illustrations.js`、`render.mjs`、`visual-catalog.json`、新增 `ink-style.js`/`ink-look.json`；`ink-painting.js` 限定为读取历史图片 |
| 引擎适配 | 按选定版本调整 `src-tauri/video/runtime.mjs`、`src-tauri/src/video/environment.rs`、`video_commands.rs`、`VideoEnvironmentCheck.vue` 及环境/渲染测试；升级在独立外部目录验证，不修改正在使用的引擎 |
| 后端与数据边界 | `src-tauri/src/video.rs`、`video_commands.rs`、`database/video.rs`、`automation.rs`、`database/automation.rs`；清理 `video/ink.rs`、`database/video/ink.rs` 与图片生成模块中的专用生产接线 |
| 分镜 Prompt | 公共叙事约束 + `prompts/video-ink-style-v1.md` 水墨规则 + 原手绘规则，整批/单镜复用同一组合入口；Prompt按实际改动升版本；移除未再使用的水墨图片生成生产入口 |
| 页面与类型 | `src/modules/video/VideoPage.vue`、`VideoSceneEditor.vue`、`VideoJobProgress.vue`、必要的 `VideoImageEditor.vue` 接线、`src/components/AutomationSetup.vue`、`src/services/video.ts`、`automation.ts`、`src/types/video.ts` |
| 回归与文档 | 现有视频、自动流程、单镜修订和图片调用边界测试；PRD、TECHNICAL-DESIGN、DATABASE、AI-RULES、DEVELOPMENT-PLAN、INK-VIDEO、VIDEO-THEMES、视频运行说明 |

实际实现按调用链收敛修改范围，不为满足列表机械改动每个文件。版本影响根据最终用户能力判断；本轮计划不修改版本或依赖。

## 8. 验收标准

1. **零图片请求**：没有可用图片模型时，手动新水墨可准备分镜、试片、成片；新水墨自动视频阶段不发起 `video_ink` 或其他图片请求。封面调用单独核对。
2. **画面直接来自绘制**：新水墨没有生成图、下载图、参考位图提线或整幅原图显现；纸纹由本地几何/噪声实现。画面可观察到沿路径落笔、粗细变化、淡墨与局部细纹。
3. **内容对应**：样片至少覆盖3种旁白实际需要的构图；首主体从本镜第一条字幕开始，新对象/语义动作不早于对应台词。连续镜头不能只是换标题和同一张通用山水。
4. **原声一致**：保留原声音轨内容与字幕时间码；画面改变不裁切、变速或重新配音。继续沿用现有响度规则。
5. **试片一致**：同一原声时间的试片与成片PNG一致；重复取帧、倒退、直接跳转和切镜后仍保持笔画及纹理一致。
6. **恢复与编辑**：停止/失败/重启保留字幕和已保存分镜；单镜修订不重做全片；更改字幕后旧试片和不匹配分镜不能用于最终成片。
7. **历史与归属**：已有图片/成片不丢失；缺图旧任务不再生成图片；整书与同书两章分开验证，经典主题参考几何保持。
8. **性能可复核**：记录短片与连续片段的实际渲染时长、初始化时间、单镜节点量及峰值内存；重复取帧不增加几何节点，连续渲染无崩溃或错误归档。样片阶段依据实测约束细节密度，不承诺固定提速倍数。
9. **双画风选择**：生成入口直接展示水墨画/书卷手绘，两项均可键盘操作；两个版本分别保存、生成、编辑、试片、成片和恢复，原风格没有被移除或强制转成水墨。
10. **分镜规则独立**：同一字幕分别生成时，服务端实际请求包含各自的画风规则；整批、自动续作和单镜修订保持同一theme；非法画风或模型尝试改变theme不能落库。
11. **进度与当前任务一致**：第02阶段分别显示水墨分镜/手绘分镜，不再由图片模型字段决定；修改下一版本选项不改变当前任务进度，新的本地任务不显示画作模型/0张或等待补画。
12. **引擎升级有依据**：核对官方稳定版本、源码与实际API；采用新引擎前，两种画风及已有原声合成均通过真实运行验证。引擎变更使旧试片/缓存失效，失败可切回保留目录，不能只放宽版本校验或改第三方版本号。

## 9. 验证安排与本轮实际结果

实施后执行与最终变更对应的检查，建议命令：

```powershell
npm run lint
npm run typecheck
npx vitest run tests/video.test.ts tests/video-storyboard.test.ts tests/video-scene-revision.test.ts tests/video-progress.test.ts tests/video-image.test.ts tests/video-environment.test.ts tests/automation.test.ts
$env:SHUYIN_TEST_FFMPEG = 'D:/MyWork/software/ffmpeg/current/bin/ffmpeg.exe'
& 'D:/MyWork/software/shuyin-video-node/v24.22.2/node.exe' --test tests/video-render.test.mjs tests/video-audio.test.mjs
. ./.tools/env.ps1
cargo test --manifest-path src-tauri/Cargo.toml --locked --lib -- --quiet
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets -- -D warnings
npm run build
git diff --check
```

另用真实Frametell执行试片/整片，验证同时间PNG、重复/倒退取帧；通过现有 FFmpeg probe/check/look 工具检查MP4、音轨与字幕并观看取帧。在隔离数据库与模型契约服务中明确记录新水墨图片请求为0，不用模拟画面替代真实引擎视觉验收。

**初版计划阶段已执行（同日）**：

| 命令 | 结果 |
| --- | --- |
| `git status --short` / `git branch --show-current` / `git log -3 --format='%h %s'` | 调查起点为干净工作区，main/7b555c3；未拉取、切换、提交或推送 |
| `python C:/Users/haozi/.agents/skills/planning-with-files-zh/scripts/session-catchup.py D:/MyWork/project/JIAZI-PROJECT/shuyin-ai` | 退出0，无额外待恢复报告 |
| `& 'D:/MyWork/software/shuyin-video-node/v24.22.2/node.exe' --version` | v24.22.2 |
| `& 'D:/MyWork/software/shuyin-video-node/v24.22.2/node.exe' --test '--test-name-pattern=the saved theme reaches\|every catalog object\|original audio timing\|a truncated preview\|all six light environments' tests/video-render.test.mjs` | 5项通过、0失败，验证现有主题、目录、时钟、环境与截断试片基础 |
| PowerShell here-string 经 `python -` 执行文档检查 | 3份Markdown、91个本地链接、P0—P3阶段、8项验收编号与编码/冲突检查通过 |
| `git diff --check` | 通过 |

上述5项绘制测试只证明现有能力可复用，不证明本计划的新笔墨效果、零生图流程、长片或安装版已经通过。

**双画风与升级补充阶段实际执行**：

- 读取并核对手动/自动theme入口、进度组件、整批与单镜Prompt、当前版本门禁和缓存签名；未重跑上一阶段5项测试，未将其计为本次重跑结果。
- `python 'D:/MyWork/project/JIAZI-PROJECT/shuyin-ai/.agents/skills/ui-ux-pro-max/scripts/search.py' 'radio group selected state keyboard' --domain ux -n 3`：命中3条控件语义、键盘及选中反馈规则；补充Vue状态查询用于选择器沿用原theme状态。
- Python标准库 `urllib.request` 查询上文官方GitHub/registry接口、标签API/源码并与本机LF归一化内容比较，完成Release/标签/npm包状态与混合快照核实。registry/Release的404为对应资源不存在的事实，不推断其他分发渠道。
- 当前未安装或切换引擎、未运行0.8.0渲染、未更新产品依赖锁或版本；新双画风及升级性能/画质均待后续真实验收。
- PowerShell here-string 经 `python -` 执行文档检查：7份Markdown编码/冲突检查、4份功能文档100个本地链接及引用源码行号、E0/P0—P3和12项验收检查通过；新增计划的尾空白检查通过。`git diff --check` 通过。

本段初版计划记录当时的只读状态；产品实现和实际命令结果见下方“2026-11-17实施结果”。官方0.8.0仍未安装，不能把引擎候选研究当作升级完成。

## 10. 2026-11-17 实施结果

### 已实现

- `render.mjs` 对无图片模型的水墨计划写入 `META.style='ink-wash'`、项目 `META.look` 和 `ink-style.js`；`ink-look.json`保存项目色板及线条参数，`ink-style.js`使用 Frametell Builder 的路径/曲线/图层/`at/dur`绘制书卷、人物、树木、山石和纸纤维。
- `illustrations.js`继续负责原 `classic`，水墨只在本地对象有专属细描时替换对应对象，未知对象仍走原目录；没有图片路径、Canvas像素采样或图像网络请求。预览签名加入新的look与笔触源码。
- 新视频创建拒绝 `imageProviderId`；整批分镜和单镜修订都读取保存的 `theme`，水墨规则位于 `video-ink-style-v1.md`。旧 `imageProviderId`/`video_ink`记录只保留历史读取，准备旧任务提示制作本地新版。自动视频不再绑定逐镜图片任务，封面图片模型仍独立保留。
- 页面在生成操作附近提供两个可见单选项；进度第02阶段按任务显示水墨分镜/手绘分镜；新任务不显示画作模型或0/0张。原字幕制作新版可用于切换画风。

### 实际验证

| 命令/产物 | 实际结果 |
| --- | --- |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked --lib -- --quiet` | 331通过、2原有忽略 |
| `cargo check ... --lib` / `cargo fmt ...` / `cargo clippy ... -D warnings` | 全部通过 |
| `npm exec vitest run tests/video.test.ts tests/video-progress.test.ts tests/automation.test.ts tests/video-scene-revision.test.ts tests/video-storyboard.test.ts -- --maxWorkers=2` | 5文件79通过、无未处理错误 |
| `node tests/video-render.test.mjs` | 44通过、0失败 |
| `npm run lint` / `npm run typecheck` | 通过；npm保留既有用户配置警告 |
| `output/shuyin-ink-style-20261117-v2/ink/video.mp4` | 17.90秒、1920×1080、AAC双声道、constant VFR；Frametell layout-check为空 |
| `output/shuyin-ink-style-20261117-v2/classic/video.mp4` | 17.90秒、1920×1080、AAC双声道、constant VFR；Frametell layout-check为空 |
| FFmpeg `check.py --platform custom --no-loudness` 两份 | 0失败；字幕轨为预期WARN（字幕已烧录画面，未封装软字幕轨） |
| `look.py --tiles 3x2`及人工查看 | 两种风格联系表已查看；水墨呈现纸色、淡墨和本地细描，原风格保留几何/彩色线稿 |

样片使用4条夹具字幕、3镜、同一17.8秒本地SAPI音频，`INK_IMAGES={}`且输入无`imageProviderId`；用于验证风格链、时间轴和输出，不代表真实书籍内容画质或长片性能。`npm test`编排首次受默认Node24.11路径守卫/更新页时序影响而中止，已用Node24.22.2分别跑完Vitest全量、video、release和queue套件；自动全流程、15分钟连续片、安装版和跨平台仍待后续。
