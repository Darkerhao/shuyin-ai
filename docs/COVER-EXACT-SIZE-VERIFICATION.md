# 封面背景准确尺寸修复（2026-11-14）

## 用户要求与根因

用户要求图片服务请求、返回背景的实际像素、背景文件名及对应封面输出使用相同的比例和尺寸：

| 比例 | 请求 size / 实际背景像素 / 封面输出 | PNG 背景命名示例 |
| --- | --- | --- |
| 4:3 | 1440×1080 | 三国演义-4比3-1440x1080-背景-v1.png |
| 3:4 | 1080×1440 | 三国演义-3比4-1080x1440-背景-v1.png |
| 16:9 | 1920×1080 | 三国演义-16比9-1920x1080-背景-v1.png |

原 `src-tauri/src/ai/images.rs` 仅按横竖方向把 `size` 替换为 1536x1024 / 1024x1536；返回图片只做格式和像素上限检查。背景保存正确地读取真实像素命名，但素材比例因此不同于目标封面比例。

## 实现与范围

- 统一图片 Provider 直接用请求的 width/height 组成 `size`，删除按方向替换尺寸的分支。
- 在既有图片完整解码之后核对实际宽高；不一致返回 `image_size_mismatch`，说明实际/请求尺寸，不返回可保存的图片结果。
- 原背景命名函数继续读取真实像素和实际 PNG/JPEG/WebP 格式；通过准确尺寸校验后，文件名比例、尺寸自然与该次请求及封面输出一致。
- 原封面工作区、候选采用、Canvas 排版、保存事务及版本避让共用原路径，不增加配置、依赖或重复状态。
- 页面说明、PRD、技术设计和用户指南同步为准确尺寸请求。
- 共用接口的视频插画同样原样请求其已有目标 1920×1080；未增加图片调用次数。
- 不自动改名或转换旧背景。用户主动共用已有背景、本地图片或历史图片时，仍按各目标画布裁切；该次生成只保证原请求所属尺寸。

无数据库结构或迁移、Command、权限或版本号变更；属于 fix / patch。未修改工作区已有的流程优化 Rust 改动及其文档内容。

修改文件：`src-tauri/src/ai/images.rs`、`src-tauri/src/ai/tests.rs`、`src-tauri/src/database/m3_tests.rs`、`src/modules/covers/CoversPage.vue`、`docs/PRD.md`、`docs/TECHNICAL-DESIGN.md`、`docs/USER-GUIDE.md`、`docs/DEVELOPMENT-PLAN.md`、本验收记录。

## 回归过程

先改测试再改生产逻辑，执行：

```powershell
. ./.tools/env.ps1
cargo test --manifest-path src-tauri/Cargo.toml --locked image_contract_ -- --nocapture
```

修复前 0 通过、2 失败：

1. 实际 HTTP 请求 `size=1536x1024`，期望 `1440x1080`。
2. 请求 1440×1080、返回 1536×1024 时仍得到成功图片结果。

修复后两项通过，覆盖三种准确请求及 PNG/JPEG/WebP 原字节返回；拒绝三种旧方向尺寸映射和相同比例但分辨率不同的 720×540 返回。HTTP 400、取消、超时和无效图片继续沿用原错误路径。数据库命名测试改为三种完整目标尺寸，核对文件名、磁盘路径及保存字节。

## 实际验证

Rust 命令均在当前 PowerShell 进程先加载 `. ./.tools/env.ps1`。

| 命令 | 结果 |
| --- | --- |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked image_contract_ -- --nocapture` | 修复前2失败，修复后2通过 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked database::m3_tests:: -- --nocapture` | 17通过 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked` | 302通过、2项原有忽略 |
| `npx vitest run tests/coverRenderer.test.ts tests/covers.test.ts --maxWorkers=2` | 68通过 |
| `npm run lint` | 通过 |
| `npm run build` | TypeScript检查及Vite生产构建通过 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | 通过 |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets -- -D warnings` | 首次回归测试写法未通过；修正后通过 |
| `git diff --check` | 通过 |

构建保留既有大于500kB的分块提示，以及本机npm的python/msvs配置提示。

全量Rust的2项忽略分别为原生凭据存储核验及需要额外Node路径配置的子进程终止核验。本轮未改变其忽略条件。首次Clippy检查指出新回归测试的 `.err().expect()` 写法触发 `clippy::err_expect`；改为 `let Err(error) = ... else { panic!(...) }`，既保留明确的失败信号，也避免失败时打印整张图片的字节，不关闭规则。

测试写法修正后重新执行两项HTTP契约测试、fmt和Clippy，全部通过。最终自查确认只有一条准确尺寸请求路径，尺寸不符在Provider返回前失败，不进入保存素材的调用；未新增重试、重复命名或持久状态。

## 独立评审与测试复跑（2026-11-14）

用户追加要求完成后进行评审测试。使用 code-review 的 Standards / Spec 两路独立只读评审，固定基线为 `6d813df1ddc57bdba01a1c9663e708f50420f151`，比较本次封面准确尺寸的工作区改动；不纳入已有流程优化代码或同文件中无关的文档段落。

| 评审方向 | 结论 | 核对结果 |
| --- | --- | --- |
| Standards 工程规范 | PASS，0项待修发现 | 删除旧尺寸映射，直接复用宽高，无新增依赖、配置或重复状态；尺寸校验在素材保存之前完成，错误状态和消息沿原任务链记录；命名、事务及版本避让复用原实现。未发现需要新抽象的代码气味。 |
| Spec 需求符合性 | PASS，0项待修发现 | 三组准确尺寸请求、返回实际像素及命名一致；错误回传页面，不产生新候选或替换草稿；自动任务沿用原有限重试，视频插画共用接口的影响已记录，未新增工作流或调用次数。 |

工程规范评审独立执行 `cargo test --manifest-path src-tauri/Cargo.toml --locked image_contract_ -- --nocapture`，2项通过；另执行 `cargo test --manifest-path src-tauri/Cargo.toml --locked generated_background_filenames_use_actual_dimensions_and_format -- --nocapture`，1项通过（覆盖三种尺寸及三种格式）。需求评审独立执行上述两项HTTP契约回归，2项通过。两路均未调用付费模型或操作正式数据。

主线程本轮重新执行：

| 命令 | 本轮结果 |
| --- | --- |
| `npx vitest run --maxWorkers=2` | 全量37个测试文件、536项通过 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked` | 全量302项通过、2项原有忽略 |
| `npm run lint` | 通过 |
| `npm run build` | TypeScript检查和Vite生产构建通过 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | 通过 |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets -- -D warnings` | 通过 |
| `git diff --check` | 通过 |

本轮全量前端与Rust测试日志位于本机忽略目录 `output/cover-exact-size-review/vitest.log`、`output/cover-exact-size-review/rust-tests.log`。未额外运行 `npm test` 中的视频FFmpeg、发布脚本及队列套件；本轮没有这些模块的改动。复评无待修问题，仅将评审与复跑结果补入本记录和开发计划，未新增生产代码或数据库变更。

## 验收对应与限制

| 验收点 | 证据 |
| --- | --- |
| 4:3、3:4、16:9 请求使用准确宽高 | 本机 HTTP 契约测试逐个检查完整请求 JSON |
| 实际图片尺寸与请求一致 | 完整解码校验及4种尺寸不符回归 |
| 文件名比例、尺寸对应实际背景和目标输出 | 三种尺寸的素材保存及封面导出命名回归 |
| 失败保留背景、草稿和历史 | 原封面失败/取消前端回归、保存失败与旧文件恢复Rust回归 |
| 正式数据安全 | 测试使用临时数据库、本机HTTP服务和模拟IPC，不操作正式书库 |

本轮使用 [OpenAI Images Generate 官方文档](https://developers.openai.com/api/reference/resources/images/methods/generate) 核对 `size` 字段。2026-11-14读取的当前文档允许 GPT Image 2/2.5 自定义 `WIDTHxHEIGHT`，同时要求宽高为16的倍数。1080不符合此约束，按用户指定尺寸原样请求不代表官方接口或其兼容服务一定支持；服务拒绝时明确失败，不静默替换尺寸。普通手动生成不自动重试，自动制作保留既有的有限模型重试策略。

未调用收费图片服务，未验证所选第三方模型的真实尺寸支持、画面质量或费用；未构建安装包或替换正式桌面版本。下一步使用包含本次修改的桌面版本，分别生成三种背景并核对文件属性与导出封面。
