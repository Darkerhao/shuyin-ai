# 音频生成等待期间定时刷新

日期：2026-11-15。影响：fix / patch；不手动升版本。

## 问题与依据

用户反馈生成等待超过一小时，手动点击“刷新网站”后才看到音频完成。当前 `src-tauri/src/audio.rs` 的 `step` 每60秒执行页面观察，但 `reload` 仅出现在手动刷新入口。`audio_page.js` 的生成观察读取当前页面，无法使已经停留在旧状态的网页重新获取远端结果。

这确认了应用缺少自动刷新机制；无法据此确定用户这次音频在服务器上的实际完成时间。本轮没有重放用户的真实远端任务。

## 实现与验收

- 在现有阶段等待记录中保存上次刷新时间，约每3分钟刷新原任务网页；保持单一后台调度器，离开内容创作页也可以继续执行。
- 仅在状态和阶段都为 `generating`、页面观察仍为 `generating` 或 `waiting` 时刷新。刷新前再次核对数据库中的任务、运行代次与页面导航代次。
- 先处理页面观察。已发现成品立即进入原下载流程；登录、人工处理、暂停、取消、失败、完成、上传、提交、下载和归档均不触发定时刷新。
- 调用现有原生 `WebviewWindow::reload`，不显示或聚焦网站窗口，不触发新建笔记本、上传来源或再次生成。
- 刷新后返回短轮询继续观察。刷新时间独立于阶段起始时间，保留原6小时等待上限；刷新调用失败交给原任务错误处理，不无限重试。
- 任务、运行代次或阶段变化时重新初始化等待记录，旧计时不能用于另一个任务。
- 原生导航与页面加载事件维护加载标志，文档加载期间不执行页面观察；观察前先捕获导航代次再检查加载状态。生成阶段只读观察期间发生导航时丢弃失效结果，下轮重新读取任务；暂停/取消不因迟到观察继续推进，稳定页面错误不被忽略。

调度验收通过确定性时钟测试，页面重建后识别成品且零生成点击通过 JSDOM 测试。评审阶段另通过真实 WebView2、IPC和隔离SQLite验证导航失效观察、实际约3分钟自动刷新、原生下载与唯一归档、加载中暂停/取消，以及明确503错误处理；远端网页和音频使用本地夹具，没有进行真实Google账号生成验收。

## 修改范围

- `src-tauri/src/audio.rs`：复用原等待状态增加刷新时间、接入后台调度、3项定时与状态回归，以及原生加载状态与生成观察的导航失效处理。
- `tests/audio-page.test.mjs`：旧页面持续生成、新文档识别完成且不重新点击生成的回归。
- `docs/TECHNICAL-DESIGN.md`、`docs/DEVELOPMENT-PLAN.md` 和本记录：同步实际机制及验证范围。

无数据库结构、迁移、正式业务数据、依赖或权限变更。

## 实际验证

Rust命令先执行 `. ./.tools/env.ps1`。

| 命令 | 结果 |
| --- | --- |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked generation_refresh -- --nocapture` | 新增刷新调度入口尚未实现时3项失败，确认缺少刷新动作会被测试捕获；随后实现并在下述音频套件中全部通过 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked audio -- --nocapture` | 53项通过，254项未被筛选选中 |
| `node --test tests/audio-page.test.mjs tests/audio-package.test.mjs` | 74项通过 |
| `npm run lint` | 通过；npm保留本机既有python/msvs_version配置提示 |
| `npm run check:rust` | Rust格式与全目标Clippy通过 |
| `git diff --check` | 通过 |

以上是初次实现验证；后续独立评审与补充测试如下。没有提交、推送或替换正式安装版。

## 独立评审与修复

用户要求完成后评审测试，以 `e2d9c54968f37a5c331fd47e0483e91ea2986e99` 的工作树差异为范围，分别执行Standards和Spec评审。

- 两轴首轮各报告同一项P2：`reload()`只发起导航，`on_navigation`立即更新代次并唤醒，可能在文档切换时执行观察。浏览器调用错误或旧代次结果进入原错误处理，任务会误转为`waiting_user`，停止自动跟进。
- 在隔离原生窗口中让页面观察暂缓返回，期间发起原生重载，先复现`generating → waiting_user`，错误为`浏览器操作失败：参数错误。(0x80070057)`。这验证宿主导航问题，不声称真实网站必然以同样频率发生。
- 修复为依据原生加载事件等待、生成阶段丢弃导航前的只读观察。Spec复评再指出加载标志检查与代次捕获之间的窗口，随后将代次捕获前移到加载检查之前，不新增状态或请求路径。最终Standards与Spec均0项待修发现。
- 修复后同一导航夹具可接续到`generating → downloading → succeeded`。最终代码还验证加载期间暂停/取消不下载、不归档、不接受迟到推进，明确503新页面仍转入原人工处理路径。

没有新增依赖或临时生产日志。加载标志仅存在会话内，SQLite仍为原schema v25。

## 评审阶段实际测试

| 命令 | 结果 |
| --- | --- |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked` | 305项通过，2项原有忽略（原生凭据测试、需显式Node路径的进程取消测试） |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked audio -- --nocapture` | 最后一次导航代次顺序调整后53项通过 |
| `npx vitest run tests/ai.test.ts tests/automation.test.ts tests/chapters.test.ts --maxWorkers=2` | 3文件、113项通过 |
| `node --test tests/audio-page.test.mjs tests/audio-package.test.mjs` | 74项通过 |
| `npm run build` | TypeScript和Vite生产构建通过，保留既有大分块提示 |
| `npm run lint` | 通过 |
| `npm run check:rust` | 最终Rust格式和全目标Clippy通过 |
| `npm run tauri -- build --debug --no-bundle --config output/playwright/audio-refresh-review/tauri.test.conf.json` | 独立应用标识的Windows debug程序编译通过；配置复用已通过构建的前端，最终重编不重复执行beforeBuildCommand；没有生成安装器 |
| `node output/playwright/audio-refresh-review/native.cjs navigation reproduce` | 修复前原生复现导航使任务转为人工处理；命令以“成功复现”为断言 |
| `node output/playwright/audio-refresh-review/native.cjs navigation pass` | 最终代码通过；同一任务下载并唯一归档48044字节WAV，0次生成点击、1次下载点击 |
| `node output/playwright/audio-refresh-review/native.cjs automatic pass` | 真实等待约3分钟触发一次定时重载，随后同一任务原生下载、唯一归档；无时间快进 |
| `node output/playwright/audio-refresh-review/native.cjs pause pass` | 加载中暂停后等待6.5秒，保持暂停与运行代次，不下载、不归档 |
| `node output/playwright/audio-refresh-review/native.cjs cancel pass` | 加载中取消后等待6.5秒，保持取消与运行代次，不下载、不归档 |
| `node output/playwright/audio-refresh-review/native.cjs page-error pass` | 新页面503仍明确提示网站不可用并等待处理，没有吞掉真实错误 |
| `git diff --check` | 通过 |

原生夹具和JSON结果保存在忽略目录 `output/playwright/audio-refresh-review/`。使用独立的`com.shuyin.audio-refresh-review-20261115`应用标识和SQLite；脚本在写入测试状态前断言数据库路径属于该标识，外部请求拦截为本地网页。样例音频为本地合成的一秒WAV，经Blob地址触发真实原生下载监听与素材事务。最初HTTP下载夹具未触发精确下载、随后音频数据脚本放在成品区域使其超过页面识别范围，分别导致夹具失败；调整为成品区域外构造Blob后闭环通过，未为夹具改动业务识别或下载规则。

原生程序启动在隔离数据目录，测试结束退出该子进程；没有改写正式书库、用户的真实音频文件或正在运行的正式安装程序。Node保留实验SQLite提示，npm保留本机python/msvs_version配置提示；这些提示不作为测试失败。

## 后续验收

原生本地夹具已覆盖实际约3分钟自动刷新及下载归档。后续发布包含修复的正式桌面版本后，仍需用真实Google账号验收一次实际生成，确认官方网页更新、下载与归档。源码改动不会热更新当前已安装进程。
