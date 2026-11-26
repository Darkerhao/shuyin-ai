# 音频驱动水墨画视频（2026-11-16）

> 2026-11-26：水墨宣纸已从新制作入口下线，新任务统一使用书卷手绘。本文仅记录历史实现、旧任务恢复与历史验证，当前主题规则见[视频主题](VIDEO-THEMES.md)。

## 目标与验收

按实际原声字幕设计细致、连贯的中国水墨画面，用同一视频任务逐镜生成、保存、显影与运镜，再合入原声与字幕。新水墨视频使用图片模型制作真正画作；不能只靠矢量简笔画调色代替。画作是依旁白的艺术表达，不承诺史料级人物面貌或建筑复原。

1. 分镜输入使用完整实际字幕时间轴和相邻上下文，每镜保存具体主体、环境、前中后景、材质细节与构图描述；不补造旁白没有讲过的情节。
2. 统一水墨画作提示词，要求80—1200字可独立绘制的画面描述、真实毛笔笔意、皴擦、浓淡渗化、宣纸肌理和精细局部；不生成文字、框架或简笔图标。
3. 每镜只生成一张画作，任务ID、描述和素材引用保存在既有 plan_json；逐镜串行保存，失败与重启从未完成处恢复，不重做成功图片。分镜已经完整时，补画作只依赖冻结的图片模型，不再要求文本模型仍可用。
4. 水墨画作占据画面主体，逐步显影与轻运镜由同一原声时钟控制；字幕固定，预览与整片同一时间画面一致，任意跳转确定。
5. 手动入口选择图片模型并明确逐镜费用，自动制作复用已选择图片模型，启动说明覆盖逐镜图片调用。经典视频与已有水墨调色任务保留原计划，新版显式创建。修改音频字幕内容或时间码后，旧水墨分镜与画作失效，不能沿用来渲染新音频。
6. 所有画作完成后才进入确认/自动渲染；失败不降级为简笔画，不改音频和字幕时间码。

## 实施顺序

- 先打通水墨分镜协议及校验、图片任务引用与恢复，再接手动/自动执行。
- 接入同源显影/运镜，保留现有图片快照、解码、来源、摘要与成片归档。
- 更新页面模型选择、画面描述/进度和费用说明，完成公共接口回归与真实引擎样片。

复用视频任务、ai_tasks、ImageProvider、素材与备份，无新SQL迁移、依赖、供应商或第二套队列。plan 中 imageProviderId 标记新水墨画作流程；旧计划缺字段继续保留既有行为。

## 验证记录

2026-11-16 接续未提交实现并完成本地验证。Git 基线为 `dev/24baa86`，版本仍为0.66.1；未提交、推送或替换正式安装。未对正式数据库或付费模型发起操作。

### 实现与影响范围

- 文本模型以实际字幕及邻镜上下文生成 `painting.description/motion`，通过完整范围、字段、80—1200字及运镜枚举校验。经典分镜仍使用原有物体目录。
- 每镜的 `video_ink` 任务与当前视频ID/运行代次/镜号绑定；图片完整解码、准确1920×1080尺寸校验、素材登记和镜头引用同事务完成。停止拒绝迟到结果，失败/重启保留已完成画作；仅改运镜时也保留原失败任务ID。
- 原声、cue时间码、字幕处理、试片和完整渲染复用原流程。图片未齐时，页面、Rust渲染入口和自动流程均拒绝渲染；描述改变只解除本镜画作，字幕内容或时间码变化清空依赖的水墨分镜。
- 手动页显示逐镜费用与图片模型，任务显示实际模型和已保存张数；自动创建冻结 `detailedInk` 和图片模型。旧自动JSON缺字段默认不启用逐镜图片，旧视频缺theme仍为classic。
- 无新增表、列、SQL迁移、依赖或权限，schema保持25。图片及任务继续随现有数据库/完整备份保存；本轮未另做原生安装升级。

主要文件：`src-tauri/src/video.rs`、`src-tauri/src/video/ink.rs`、`src-tauri/src/video_commands.rs`、`src-tauri/src/database/video.rs`、`src-tauri/src/database/video/ink.rs`、`src-tauri/src/database/image_generation.rs`、`src-tauri/src/cover_commands.rs`、`src-tauri/src/automation.rs`及原自动流程数据库模块；`src-tauri/video/{render.mjs,ink-painting.js,illustrations.js}`；视频页、分镜/图片/进度组件、AutomationSetup、服务与类型。相关Rust/Node/Vue测试、六份主文档、双主题说明和三份规划记录同步更新。

### 本轮根因修复

显影层原本每帧清空可见Canvas，却只补画新增笔触。使用上轮真实渲染快照运行新增回归，第二次渲染同一帧得到0条笔触，期望12655条，确证旧笔触丢失。现由离屏Canvas累计、可见Canvas按当前运镜绘制；回退重置后按相同时钟重建。未改独立Frametell源码或放宽确定性检查。

本机Chromium读取冻结的file图片像素会产生Canvas `SecurityError`，故保留宿主校验后按资产ID一次嵌入的方式；没有关闭浏览器跨源保护。长片的图片解码和内存开销仍需实测。

### 实际命令与结果

下列命令从仓库根目录执行；`node` 指 `D:/MyWork/software/shuyin-video-node/v24.22.2/node.exe`，Rust先执行 `. ./.tools/env.ps1`。

| 实际命令 | 结果 |
| --- | --- |
| `node node_modules/vitest/vitest.mjs run --maxWorkers=2` | 38文件、550项通过；最终文案/模型进度展示调整前的全量检查 |
| `node node_modules/vitest/vitest.mjs run tests/video.test.ts tests/automation.test.ts tests/automation-tasks.test.ts --maxWorkers=2` | 最终相关3文件、64项通过 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked --lib -- --quiet` | 328通过、2原有忽略；整书/章节×经典/水墨四组合、图片HTTP契约、失败/停止/重启及编辑恢复通过 |
| `node --test tests/video-render.test.mjs tests/video-audio.test.mjs` | 设置 `SHUYIN_TEST_FFMPEG=D:/MyWork/software/ffmpeg/current/bin/ffmpeg.exe`，50通过、0跳过 |
| `npm run lint` | 通过 |
| `npm run build` | TypeScript和生产构建通过，保留原有大分块与本机npm配置提示 |
| `npm run check:rust` | fmt及所有target的Clippy通过，警告视为错误 |
| `node src-tauri/video/render.mjs output/ink-video/continued-20261116/full/input.json output/ink-video/continued-20261116/full/result.json` | 真实Frametell0.7.3完整绘制、字幕、音量处理与合原声通过 |
| `node src-tauri/video/render.mjs output/ink-video/continued-20261116/preview/input.json output/ink-video/continued-20261116/preview/result.json` | 同源4秒编译窗口渲染通过；这是渲染器夹具，页面仍只提供30/60秒试片 |
| `node output/ink-video/continued-20261116/verify-parity.mjs` | 源时间10.340998秒与试片2秒PNG字节相同；6组重复、顺序和回退截图一致 |
| `python <ffmpeg-skill>/scripts/probe.py <video.mp4> --json` | 完整片容器17.900秒、视频17.833333秒/428帧；试片容器4.011秒、视频4.000秒/96帧；均1920×1080、24fps、H.264/yuv420p、AAC双声道48kHz |
| `python <ffmpeg-skill>/scripts/check.py <full/video.mp4> --platform custom --content --lufs -16 --tp -1.5 --json` | 10项PASS、1项WARN、0失败；-16.6LUFS/-1.9dBTP，无黑屏/冻结异常；WARN为无独立软字幕轨，画面内字幕已查看 |
| `python <ffmpeg-skill>/scripts/look.py <full/video.mp4> --tiles 3x2 -o <full/contact.png>` | 六格图已查看，画作细节及四条中文字幕可见 |
| `ego-browser nodejs -e "await import('file:///.../round-01.mjs')"` 及后续02—07 | 隔离页面1440/1120/800/390×深浅8布局无横溢；模型/1—2张进度、描述修改后仅本镜失效、试片/成片门禁及原任务继续通过；未知IPC为0，空间已关闭 |
| `git diff --check`、`git diff --cached --check`及本地Markdown链接检查 | 空白/暂存检查通过；14份Markdown、272个本地链接通过，冲突标记0 |

初次浏览器导航在load等待中超时，实际页面已提交并正常可用；同一页面接续完成。编辑用例首轮因隔离假IPC未序列化Vue Proxy而报structuredClone错误，只修正夹具参数序列化后通过，没有修改产品逻辑规避。媒体脚本首次误将 `--json` 当作带文件名参数，按实际帮助改为标准输出重定向后重跑通过；首次参数错误不计验证成功。

### 证据与验收对应

样片目录 `output/ink-video/continued-20261116/` 保留full/preview输入、MP4、probe、技术检查、联系表、渲染日志及parity-result.json；页面目录 `output/playwright/ink-continued-20261116/` 保留7轮脚本、8组布局、交互JSON及深浅截图。样片复用本地SAPI测试原声和Cleveland Museum of Art的CC0画作（1979.27.2.18），不是AI生成画质示例，也不是实际书籍节目。

Frametell布局检查无ERROR；完整片两条、试片一条 `sparse` WARN保留。实际 `tools/checks.mjs` 面积统计明确排除宽高都超过内容区97%的满幅元素，因此本次全幅图片/Canvas不计入主体面积；实图已查看，没有删除检查或改阈值。任务专用Vite进程72180和14396端口已停止，原应用未操作。

| 验收项 | 本地结果与证据边界 |
| --- | --- |
| 实际字幕驱动细致分镜及画作协议 | Rust协议校验、图片HTTP请求正文和自动四组合通过；真实模型的内容准确度未验 |
| 逐镜保存，失败/暂停/重启不重做已成功画作 | SQLite事务、HTTP失败续作、迟到拒绝及重开测试通过 |
| 改描述只重做本镜，改字幕使旧派生画面失效 | Rust与页面隔离交互通过；改运镜保留原失败任务ID |
| 画作未齐不能试片/成片，自动流程等待全部画作 | Rust公共入口、自动整书/章节与页面门禁通过 |
| 原声时钟、显影/运镜、试片与整片一致 | 原声/音量回归50项、真实引擎样片、同一源时间PNG及重复/回退检查通过 |
| 经典与历史主题、旧自动任务保留 | 旧JSON默认、经典物体目录摘要及未启用detailedInk的自动流程测试通过 |

### 未验证与下一步

未调用真实付费文本/图片模型、NotebookLM或抖音账号；未验真实整书/章节画面叙事、跨镜人物一致性、无文字/水印和实际费用。当前样片不能证明这些质量要求。未实测长片多图解码/内存、原生Tauri完整生成、正式安装升级或macOS/Linux；未运行无关的release/queue套件。本轮交付为实现及上述本地契约/引擎/页面验证。

下一步应使用明确授权的一段真实书籍音频，在含本功能的新桌面构建中生成少量水墨镜头，核对图片服务准确尺寸、内容、费用和听感，再扩展长片测试。正式安装版不会因为工作区源码改变而自动更新。
