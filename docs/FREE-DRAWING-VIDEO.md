# 自由场景分镜与逐笔绘制

日期：2026-11-17。

## 实现与取舍

用户要求取消物体白名单和每镜最多3个主体，让模型按原声内容设计完整场景。新分镜的每个 beat 提供自由主体名和 `drawing.strokes`，笔画由 `[x,y]` 数值折线组成；全部物体共用1000×300场景坐标，在本地映射到画面安全区域。主体数量按内容安排，程序不另设个数上限。

复用原 Scene/Beat、Provider、SQLite plan_json、任务恢复、单镜修订、Frametell P.path 与 pencil.sequence/mount。模型只提供数值，不执行脚本/SVG/HTML或加载外部文件。标题、字幕与音频仍走原时间轴，标签可为空；同一句多个主体依次绘制，已完成笔画保留到镜尾。新生成必须含笔画，缺笔画不能退回通用图标。旧目录分镜保持读取，并把旧三槽布局扩为按主体数量计算行列；旧图片任务读取保持原路径。

整镜限制为2000笔和12000点，坐标为有限数、在画布内，每笔至少两个不同点。这些限制控制绘制资源，不按主体数计数。新批次最多12条字幕（此前24条），仍按句意尽量切批；自由笔画输出更长，生成耗时与调用次数可能增加。新Prompt为 video-storyboard-v6、video-pencil-style-v2、video-scene-revision-v2。普通手动生成不增加自动收费重试。

编辑与候选对照共用 VideoDrawingPreview 展示整镜布局。自由笔画支持标签、主体名称和字幕锚点编辑；改变画面内容/空间布局用原“重新生成这一镜”。只适用于旧目录的模板构图控件在全自由笔画镜头中隐藏，避免调整无效。

无新依赖、SQL迁移、模型超时或安装版改动。数据仍为schema25，新可选drawing字段随任务/草稿/备份保存。新版本能读取旧任务；旧应用的严格结构解析不认识新字段，因此新计划需使用含此功能的版本。功能影响为feat/minor，未手工升版。

## 根因与真实模型证据

原安装版任务4次请求正常完成后校验失败，没有保存失败原文，无法断言4次均为同一字段。只读同一任务、原模型gpt-6.1-sol/medium和相同首批字幕，受控复现发现：

- 原素描规则：18.85秒返回第6镜的 `object=spear`，目录不含此物体。
- 原书卷规则：35.53秒返回第3镜4个物体，触发3个上限。
- 新自由笔画规则：187.23秒返回前12条字幕的4镜，主体数分别4、3、4、3，笔画数50、30、52、42，全部通过实际Rust结构/内容校验。服务返回输入3261、输出5639 tokens。

三次诊断均通过原Provider调用；诊断记录写到临时测试库，正式任务和原声未改写。临时测试模块已从产品测试入口解除。证据摘要在 `output/storyboard-pencil-diagnosis/{ink-before,classic-before,free-ink}.json`，成功的可编辑计划在 `free-plan.json`。

## 实渲验收

从同一真实原声另存前49.16秒PCM验收片段，使用模型生成的4镜渲染：

- [素描样片](../output/free-drawing-20261117/video.mp4)：49.20秒容器、1920×1080、24fps、H.264/AAC双声道48kHz。
- [30秒同源试片](../output/free-drawing-20261117/preview/video.mp4)：从原声11.46秒起，返回30.01秒容器。
- [书卷手绘对照](../output/free-drawing-20261117/classic/video.mp4)：同一自由笔画计划前12秒，未再次调用模型。
- [成片抽帧](../output/free-drawing-20261117/contact.png)、[6个绘制时刻](../output/free-drawing-20261117/episode/out/snap/free-scene.jpg)：已查看，多人、营寨、旗帜、山地在共享场景中逐笔出现，标题/字幕没有被绘图遮挡。
- [桌面编辑](../output/free-drawing-20261117/editor-desktop.png)、[窄屏编辑](../output/free-drawing-20261117/editor-narrow.png)：1280/390宽度无横向溢出，4主体/50笔完整预览，标签编辑成功；为隔离Vue组件运行验收，不是正式Tauri安装版。
- [浏览器同源证据](../output/free-drawing-20261117/browser-verification.json)：原声12.16/13.16/14.16秒三组整片/试片PNG逐像素相同；跳转45秒后返回仍相同，ENGINE错误0。

素描布局检查为空；媒体检查10 PASS、1 WARN、0失败，-16.1 LUFS、-2.0 dBTP。WARN为没有独立软字幕轨，原有烧录中文字幕已查看。模型画面仍偏简笔线描，人物轮廓和局部空间关系有示意性；本次证明自由绘制/数量/时间链路可运行，不宣称已达到精细人物素描或所有史实表现正确。

## 实际命令与结果

仓库根目录执行；Node使用 `D:/MyWork/software/shuyin-video-node/v24.22.2/node.exe`，媒体验证PATH加入 `D:/MyWork/software/ffmpeg/current/bin`，Rust先执行 `. ./.tools/env.ps1`。

| 命令 | 结果 |
| --- | --- |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked free_drawings -- --nocapture` | 新12主体/任意名称回归修改前失败（drawing字段无法解析），实现后通过 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked --lib -- --quiet` | 337通过、2原有忽略；包含笔画保存/重开/继续与单镜校验 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked reproduce_first_storyboard_batch -- --ignored --nocapture` | 三次受控真实诊断，前两次失败、新协议1通过；临时入口现已移除 |
| `node node_modules/vitest/vitest.mjs run tests/video.test.ts tests/video-scene-revision.test.ts tests/video-storyboard.test.ts tests/video-progress.test.ts --maxWorkers=2` | 4文件、62通过，包含6个自由主体候选预览/采用 |
| `node --test tests/video-render.test.mjs tests/video-audio.test.mjs` | 设置SHUYIN_TEST_FFMPEG，55通过，0跳过 |
| `npm run build` | 类型检查与生产构建通过，保留原大分块/npm本机配置提示 |
| `npm run lint`、`npm run typecheck` | 通过 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check`、`cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets -- -D warnings` | 通过 |
| `python C:/Users/haozi/.agents/skills/ffmpeg-skill/scripts/cut.py <原音频> --start 0 --end 49.16 --accurate -o output/free-drawing-20261117/source.wav --dry-run --json-brief`，随后去掉dry-run执行 | 另存49.16秒，样本精度、时长差0，不覆盖原声 |
| `node src-tauri/video/render.mjs output/free-drawing-20261117/input.json output/free-drawing-20261117/result.json` | 素描真实成片成功，layout-check为空 |
| `node src-tauri/video/render.mjs output/free-drawing-20261117/preview/input.json output/free-drawing-20261117/preview/result.json` | 30秒同源试片成功 |
| `node src-tauri/video/render.mjs output/free-drawing-20261117/classic/input.json output/free-drawing-20261117/classic/result.json` | 12秒书卷手绘对照实渲通过，抽帧已看 |
| `python C:/Users/haozi/.agents/skills/ffmpeg-skill/scripts/look.py output/free-drawing-20261117/video.mp4 --tiles 3x2 -o output/free-drawing-20261117/contact.png --json-brief` | 抽帧成功并已查看 |
| `python C:/Users/haozi/.agents/skills/ffmpeg-skill/scripts/check.py output/free-drawing-20261117/video.mp4 --platform custom --content --lufs -16 --tp -1.5 --json` | 10通过、1软字幕提示、0失败 |
| `ego-browser nodejs -e "await import('file:///D:/MyWork/project/JIAZI-PROJECT/shuyin-ai/output/free-drawing-20261117/browser-parity.mjs')"` | 3组同源与1组跳转截图相同 |
| `git diff --check` | 通过 |

过程错误已处理：初次Rust旧Prompt断言仍要求物体目录，更新为新协议断言；两条Node新断言误计已有HUD进度动作，改为校验主体；并行编译时既有4秒Node子进程超时，顺序全量55通过；Rust重编与真实测试并发导致Windows exe占用，结束后全量337通过；首次样片错误地用全长音频配短计划，时长检查正确拒绝，改为另存验收原声片段；FFprobe不在PATH，加入已安装目录；Ego通过HTTP打开生成页无法加载file引擎，改为原生file地址，同源验证通过。没有放宽校验或关闭规则。

全量Rust337与Node55之后仅补了笔画预算/新生成缺失笔画/经典画风路径断言，`cargo test ... free_drawing -- --nocapture` 最终2通过、`node --test --test-name-pattern='free scene|more than three' tests/video-render.test.mjs` 最终2通过，Clippy复验通过。最后UI隐藏自由场景无效模板控件后重跑生产构建和4文件62项Vue测试通过。官方引擎网页工具本轮不可用；实现依据本机已安装0.7.3源码、README与真实渲染，未新增依赖或宣称升级引擎。

## 验收对应与边界

任意名称/实际笔画、多于3主体、完整场景空间、按字幕逐笔出现、失败保稿和继续、单镜预览采用、双画风、试片同源、工程检查均在上述受控范围通过。未生成12分钟完整长片、未做真实模型跨批全片人物一致性验收、未重新打安装包或替换正式应用、未跨平台验收；未提交或推送。下一步在包含本功能的构建中继续原失败任务，先查看分镜与30秒试片，再决定全片渲染。
