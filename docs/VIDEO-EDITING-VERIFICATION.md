# SRT编辑与分镜出场时机验收（2026-11-03）

## 实现与影响范围

1. 原声SRT继续使用当前输入状态，保存/试片/重建/完整渲染均立即读取最新输入，不增加防抖或另一份字幕草稿。
2. 原内联分镜卡片移到独立组件，编辑同一Scene；缓存分镜JSON快照，字幕输入不触发所有分镜控件和响应式分镜序列化。
3. 收起的物体/动作/字幕锚点选择器按需创建，不再预先挂载上万项隐藏选项。选择仍在Scene中，收起、重新展开和保存不丢选择。
4. 唯一分镜Prompt从v4升级为v5，每镜首主体必须锚定第一条字幕；用开镜已经成立的主体，后文结论/否定仍等原字幕。1.2秒密集出场和12秒长停留继续作为导演建议。
5. 生成结果解析边界拒绝首主体偏晚的整批结果，保留字幕与此前完整分镜，提示继续准备该批；不自动改锚点、不增加重试。通用保存校验保持旧规则，已保存/人工分镜仍可保留有意停顿。

代码文件：

- `src/modules/video/VideoPage.vue`、新增`VideoSceneEditor.vue`：分镜更新隔离、按需控件与缓存。
- `src-tauri/src/video.rs`、`video_commands.rs`：统一生成结果接收与首锚点校验。
- `prompts/video-storyboard-v5.md`：替代v4，未并留旧生成入口。
- `tests/video.test.ts`、`video.rs`内回归：输入工作量、立即保存、折叠保留选择、生成首镜/后续镜/非零批次与后续结论。
- 同步PRD、技术设计、AI规则、使用手册、开发计划，以及三份历史视频文档的当前Prompt链接。

SQLite仍为v15，无表/列/SQL迁移、依赖或权限变更。未改原声音频、字幕时间码、渲染器、已有成片或正式用户数据。按fix/patch判断影响，未改版本、提交、打包或发布。

## 浏览器性能与工作流

脚本：`output/playwright/subtitle-performance/check.mjs`，使用真实Vue/页面/控件，在Chrome无头浏览器中通过Tauri官方mockIPC构造隔离数据；没有写正式数据库或调用远程模型。对照基线读取本轮修改前的Git HEAD页面源码，固定视口1680×1350，关闭热更新，避免中途重新加载影响测量。

输入测量为同步InputEvent到Vue nextTick完成DOM更新的耗时，逐字之间等待一帧；不包含操作系统键盘输入、真实中文输入法或桌面WebView的端到端延迟。合成compositionstart/input/compositionend只验证组合输入状态。以下为本次本机样例，不能承诺其他机器完全相同。

| 场景 | 输入更新中位数 | 单次最大值 | 整个交互期间分镜下拉额外更新 |
| --- | ---: | ---: | ---: |
| 修改前，64条字幕/12镜 | 596.7ms | 624.4ms | 936次 |
| 修改后，64条字幕/12镜 | 2.0ms | 5.0ms | 0次 |
| 修改后，640条字幕/116镜 | 6.4ms | 10.8ms | 0次 |

每组均验证：逐字编辑、组合输入、输入焦点、字幕框滚动位置、未保存离开拦截、点击保存即包含最后一字、分镜数量保留。页面错误和未处理IPC均为0。图片`after-116.png`已打开核对，保留原布局和控件样式。

证据：`output/playwright/subtitle-performance/result-12.json`、`result-116.json`、`before-12.png`、`after-12.png`、`after-116.png`。原116镜基线在90秒初始化等待中超时，未获得该规模输入耗时，不据此计算116镜的改善倍率。排查由此确认收起控件仍全部创建的问题，并通过按需创建修复；640条/116镜修改后验收完整完成。

浏览器实际命令（PowerShell，使用项目已有Node）：

```powershell
# 640条/116镜修改后工作流验收
$env:PERF_MODES = 'after'
& 'D:/MyWork/software/shuyin-video-node/v24.22.2/node.exe' output/playwright/subtitle-performance/check.mjs

# 独立命令会话：64条/12镜前后对照，PERF_MODES未设置
$env:PERF_SCENES = '12'
& 'D:/MyWork/software/shuyin-video-node/v24.22.2/node.exe' output/playwright/subtitle-performance/check.mjs
```

首次脚本试跑修正了模拟IPC直接structuredClone响应式参数的问题；改为与真实IPC一致的JSON序列化后验证。早期热更新使测量页重新加载，最终固定源码并关闭热更新后完成上述证据。没有将这些失败试跑写为成功结果。

## 自动检查

Rust命令先执行`. ./.tools/env.ps1`。

| 命令 | 实际结果 |
| --- | --- |
| `npx vitest run tests/video.test.ts tests/video-storyboard.test.ts tests/video-image.test.ts tests/video-progress.test.ts` | 4个文件，41项通过 |
| `& 'D:/MyWork/software/shuyin-video-node/v24.22.2/node.exe' --test tests/video-render.test.mjs` | 38项通过 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked` | 182通过、2项按原设置忽略（系统凭据与视频子进程专项） |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked generated_storyboards_start_each_subject` | 最后文案调整后定向1项通过 |
| `npm run lint` | 通过 |
| `npm run build` | TypeScript检查及前端生产构建通过 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | 通过 |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets -- -D warnings` | 通过 |
| `git diff --check` | 通过 |

回归先失败后修复：字幕输入仍序列化scenes并更新下拉控件；生成解析仍接受首物体放在较晚字幕；收起区域仍创建物体选择器。修改后相应断言通过，包含立即保存最新输入、人工晚锚点仍可保存、后文check动作仍在后文、展开修改后重新展开并保存。

首轮`npm run test:video`使用机器默认Node24.11.0，被既有Windows运行时版本门拦截，6个入口夹具未产生工程；改用已安装24.22.2后38项全部通过，没有修改版本检查或测试断言。npm既有python/msvs配置提示与前端单包大于500kB提示保留，未调整规则规避。

## 验收边界与下一步

- 已验证页面输入性能、模拟保存/离开保护、分镜接收契约与现有渲染锚点回归。
- 未调用真实远程分镜模型，不能把契约校验通过写成实际内容选择质量已验收。密集出场、长停留仍需试听判断。
- 未对用户截图中的实际音频生成新分镜/视频，未进行原生桌面和安装包验收，未替换安装程序。真实操作系统中文输入法仍需在桌面版本中确认。
- 使用包含本次修改的桌面构建：先编辑一份长SRT确认输入和保存，再核对第1镜的首主体字幕；旧分镜可手动选本镜第一条字幕后保存，用30秒原声试片检查。新生成流程使用v5，仍由用户最终确认完整视频。
