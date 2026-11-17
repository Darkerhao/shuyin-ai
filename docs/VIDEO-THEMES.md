# 视频画风：铅笔素描与书卷手绘

## 当前规则（2026-11-17）

用户明确“水墨画”指铅笔素描、一笔一笔画出来的感觉。新本地 `theme=ink` 统一显示为「铅笔素描」：白纸、单色细线、轮廓和细节先后绘制、排线明暗、跟随笔尖。`theme=classic` 保留书卷手绘的原配色和讲解构图。两种画风在生成前独立选择；任务继续冻结原键，切换画风通过原音频/字幕制作新版。

两种画风的新视频均不调用逐镜图片模型，共用字幕、单镜、保存恢复、试片及成片。旧 `imageProviderId` 画作显示为「旧版水墨画」，保留已存图片与读取，不改作素描、不继续补图。源码和本轮验证见[铅笔素描](PENCIL-VIDEO.md)。

## 前序水墨方向与验收（历史记录）

## 待实施修订：两种画风独立可选与本地绘制

用户最新要求在生成分镜前明确选择“水墨画”或“书卷手绘（原风格）”。继续使用ink/classic字段，分别保留分镜导演规则与绘图表现，共用原声、时间轴、单镜编辑、保存恢复和渲染；水墨视频取消逐镜图片生成。选择器移到生成操作附近，当前任务显示冻结画风，第02阶段显示“水墨分镜/手绘分镜”。更换画风使用原SRT制作独立新版，不改写已有任务。另纳入Frametell官方版本核查、隔离升级和双画风回归，详见[双画风本地绘制与引擎升级计划](INK-STROKE-VIDEO-PLAN.md)。上述均为计划，下文保留当前实现和历史验证事实。

实施状态：上述双画风选择与本地水墨已接入当前源码。水墨新任务使用 `META.style=ink-wash`、项目色板/纸纹和 `ink-style.js`，不需要图片模型；旧图片水墨仅保留历史读取。0.8.0外部引擎候选尚未安装，未作为本次功能依赖。

## 1. 需求与范围

视频制作提供两种主题，由用户在创建任务时选择：

- `书卷手绘`：保留原有配色、线稿、环境和物体几何。
- `水墨宣纸`：依照实际字幕创建详细艺术分镜，为每镜用所选图片模型生成一张完整水墨画，并沿原声时间轴进行细线显影、墨色层次与轻运镜。

新任务默认 `水墨宣纸` 并启用逐镜画作；用户须选图片模型，调用张数等于分镜数量，可能产生模型费用。`书卷手绘` 不调用逐镜图片模型。旧计划缺 `imageProviderId` 时保持旧分镜和渲染，不自动升级或发起付费图片请求。可用原字幕制作新版切换风格。

逐镜水墨实现与验收见[水墨视频闭环](INK-VIDEO.md)。入口见 [VideoPage.vue](../src/modules/video/VideoPage.vue)、[render.mjs](../src-tauri/video/render.mjs)、[ink-painting.js](../src-tauri/video/ink-painting.js)、[illustrations.js](../src-tauri/video/illustrations.js)。

## 2. 数据与兼容

- `VideoPlan.theme` 为可选 JSON 字段，值为 `classic` 或 `ink`。
- `VideoPlan.imageProviderId` 标识新逐镜水墨任务；`Scene.painting` 存储画面描述、有限运镜和可恢复图片 taskId，`Scene.image` 冻结生成图及内容摘要。
- 旧 `plan_json` 缺少该字段时反序列化为 `classic`，并在重新保存经典主题时继续省略该字段。
- 新建任务的 `StartVideoInput.theme` 缺省为 `ink`；手动页面显式传入选中的主题。
- 自动制作 `selection.detailedInk` 采用显式可选值，旧 JSON 缺字段默认 false；创建视频时冻结图片模型 ID。
- 描述、图片 taskId 和图像资产复用 plan_json、ai_tasks、assets；无新 SQLite 结构/迁移，schema 仍为 v25。
- 预览签名、渲染缓存键和自动流程的计划摘要继续包含完整 plan，因此主题变化会使旧试片失效。
- 已完成的成片不重写；旧任务只读保持原主题。

## 3. 渲染实现

`render.mjs` 将主题写入每个页面的 `theme` 和 `META.theme`：

- 经典主题走原 `environment`、`sketch` 和颜色表；原 85 个参考物体的归一化源码摘要仍由 `test:video` 校验。
- 旧水墨计划沿用原低饱和矢量分镜；新逐镜画作独立生成图片，不把调色滤镜当作完整水墨画。
- `video_ink` 子任务沿用既有图像 Provider，画作在单一 SQLite 事务中登记素材并回写 plan；失败后重试同一个镜头，重启后成功画作不重做。
- `ink-painting.js` 从同一冻结图片中逐时显影笔画，并按确定性运镜状态渲染，预览/成片共用 cue 时钟。
- 字幕仍由引擎固定层负责；水墨主题使用暖纸色半透明底和更薄的边角，经典主题字幕样式不变。
- 仅画面描述变化时当前镜头图片失效；字幕或时间码变化时清空水墨分镜，按新内容重建。原声、字幕时间码、HUD 字幕层、响度与发布流程不变。

## 4. 页面交互

手动视频页要求选择图片模型并说明按镜费用；分镜区展示描述、画作完成状态及运镜，用户修改描述后只重做本镜。未完成画作时不提供试片或成片。自动制作开始授权说明调用费用随镜头数变化；旧自动流程不追加图像请求。

## 5. 历史双主题验证（2026-11-15）

工作目录为仓库根目录。Node 使用 `D:/MyWork/software/shuyin-video-node/v24.22.2/node.exe`，媒体工具使用本机 `D:/MyWork/software/ffmpeg/current/bin`。

| 实际命令 | 结果 |
| --- | --- |
| `npm run test:video` | 47 项中 41 通过、6 项既有音频集成测试跳过、0 失败 |
| `npx vitest run` | 38 文件 548 项通过 |
| `npx vitest run tests/video.test.ts` | 最终按钮文案调整后 36 项通过 |
| `npx vitest run tests/automation.test.ts tests/automation-tasks.test.ts tests/video.test.ts` | 3 文件 62 项通过 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked video` | 64 项通过、1 项既有进程树专项忽略 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked automation` | 52 项通过 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked --lib -- --quiet` | 320 项通过、2 项既有忽略 |
| `npm run lint` | 通过，未关闭规则 |
| `npm run build` | 类型检查与 Vite 构建通过；保留既有大于 500 kB 分块提醒 |
| `cargo fmt --check` / `cargo clippy --locked --all-targets -- -D warnings` | 通过 |
| `git diff --check` | 通过 |
| 真实 Frametell 0.7.3 隔离样片 | 同一 12 秒静音夹具分别以 `ink`、`classic` 渲染；两份均 1920×1080、24fps、288 帧、12.0 秒、H.264 + AAC 双声道 |
| `probe.py` + `check.py --platform custom --no-loudness` | 两份 probe 通过；check 0 失败，2 个预期提醒（夹具音轨静音、无独立软字幕轨） |
| `look.py --tiles 3x2` / `--at 8` | 两份联系表及 8 秒帧已查看；水墨主题为暖纸、低饱和墨色和淡墨层，经典主题保留蓝色水纹、橙色进度线等原配色 |
| `layout-check.json` | 两份均为空数组，无布局错误 |

样片在忽略目录 `output/theme-verify/`：`video.mp4` 为水墨，`classic/video.mp4` 为经典；联系表分别为 `video_sheet.png` 与 `classic/video_sheet.png`。样片使用静音 WAV 和两条测试字幕，仅验证渲染与界面链路，不代表真实内容质量。

## 新逐镜水墨验证

当前实现命令、真实引擎样片及未覆盖边界见[水墨视频闭环](INK-VIDEO.md)。本页以下 2026-11-15 的调色样片仅作兼容/历史记录，不证明新模型画质。

## 6. 未验证边界（原双主题记录）

- 未用真实书籍音频、真实长片和实际模型生成内容审听水墨主题；只用 12 秒夹具验证编码与画面差异。
- 未运行完整 `npm test` 的音频、发布和队列套件；本轮运行的是与改动相关的视频 Node、视频页面、Rust video、lint 和类型检查。
- 未制作安装包、未替换正式安装版、未做 macOS/Linux 实机验收。
- 水墨主题是现有矢量分镜的视觉主题，不是每镜 AI 水墨插画；后者需要另立图片生成/素材审核切片。

## 7. 原双主题历史结论

以下结论针对 2026-11-15 的调色/矢量水墨实现，不代表新逐镜水墨画作已通过。当前闭环、测试命令和质量边界见[水墨视频实现与验证](INK-VIDEO.md)。

上述遗留的“后续再增加每镜插画”建议已被本轮完整实现取代；实际第三方模型画质及安装版运行仍须按 `INK-VIDEO.md` 的边界验收。
