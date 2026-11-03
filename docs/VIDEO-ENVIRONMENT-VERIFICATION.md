# 视频环境检查与桌面恢复验收（2026-11-03）

## 实现

- 在现有视频运行环境配置中增加“一键检查环境”，按新转写/已有字幕用途检查七项依赖。显示实际版本、使用路径、可用/需处理/不需要状态，缺项提供重新选择和修复说明。
- 保留已有四项配置。只为空项查找PATH、附近独立工程和本地虚拟环境，用户主动填入检测路径并保存；检查不自动更改已保存设置。改变配置或用途后清除旧报告。
- 与渲染共用Node版本保护和Frametell版本识别，读取引擎真实FFmpeg/浏览器/字体位置。执行FFmpeg版本命令、启动Chrome/Edge并加载字体；新转写用途还检查Python依赖和CPU/int8模型离线加载。没有下载、付费服务或音频识别请求。
- 复用隐藏子进程、Windows Job Object、45秒阶段截止及临时输出，检查结束清理仅本次创建的目录。已有字幕不启动Python。

改动文件：

- `src/modules/video/VideoEnvironmentCheck.vue`、`VideoPage.vue`、`src/services/video.ts`、`src/types/video.ts`。
- `src-tauri/src/video/environment.rs`、`video.rs`、`video_commands.rs`、`lib.rs`、`permissions/video.toml`。
- `src-tauri/video/runtime.mjs`、`render.mjs`、`transcribe.py`。
- `tests/video-environment.test.ts`、`video-render.test.mjs`、`test_video_transcribe.py`，以及environment.rs内测试。
- PRD、技术设计、数据库、AI规则、使用手册、开发计划、草稿补验及本文。

无数据库迁移，schema16；无新增npm/Cargo/Python依赖，未改变四项设置结构。新增一条限定主窗口的检查命令。属于feat/minor，未手动改版本、提交、打包安装器或发布。没有修改正式任务和正式用户配置。

## 验收标准与证据

| 标准 | 实际证据 |
| --- | --- |
| 原草稿在真实桌面重开可恢复 | 469条字幕、72镜、933.46秒隔离长片任务在真实WebView2/IPC/SQLite中编辑后暂存；强制结束精确测试进程，再启动恢复字幕、镜头标题、修改项筛选。正式计划未变化，任务切换不串稿。 |
| 正常关闭保留编辑位置 | 发送Windows WM_CLOSE，经应用原关闭监听退出；重新启动SRT光标位置为18。 |
| 逐句试听仍对应原声 | 真实长音频副本可播放，首句末暂停于2.48119秒；单句恢复只改文字。并非全片人工听审。 |
| 一键检查与故障处理 | 空配置自动找到引擎及PATH Node24.11.0，正确报告Node缺陷；主动填入路径，改用24.22.2后复查通过。 |
| 已有字幕跳过语音依赖 | 将Python设为不存在路径，已有字幕用途仍为5项可用、2项不需要；切到新转写时正确显示Python/模型需处理。 |
| 完整环境可用 | 改用实际Python及模型后7项可用，修复说明可展开；配置改变后旧报告消失。 |
| 检查与配置保存独立 | 四轮检查后回读video_settings仍等于检查前设置；最后点击“保存运行环境”才写入选择。 |
| 界面与操作 | 原生WebView2在1440/1120/800px无检查区内部横向溢出，截图已查看；验证过程中页面脚本错误为0。 |
| 渲染闭环 | 使用同一隔离任务生成30秒原声试片，实际1920×1080，原生播放器成功播放；plan保持相同，不新增正式素材。验证了新共用runtime.mjs随运行目录复制并被实际渲染加载。 |

本机探测版本：Node24.22.2、Frametell0.7.3、FFmpeg6.1.1、Chrome156.0.8078.79、Python3.12.4、faster-whisper1.2.1、av16.1.0。霞鹜文楷、Caveat、JetBrains Mono、Fredoka的所用字体样式已在浏览器中加载；small模型从本地目录以CPU/int8加载，local_files_only开启。

## 实际命令与结果

Node脚本验证在本进程将`D:\MyWork\software\shuyin-video-node\v24.22.2`置于PATH首位，不修改机器默认Node或项目engines。Rust命令先执行`. ./.tools/env.ps1`。

| 命令 | 结果 |
| --- | --- |
| `npm test` | 前端323项、视频39项、发布78项、队列19项通过；1项macOS专属检查跳过。 |
| `python -B -m unittest discover -s tests -p test_video_transcribe.py` | 6项通过，包括离线模型探测、不执行识别、错误依赖和缺少模型文件。 |
| `npm run lint` | 通过。 |
| `npm run typecheck` | 通过。 |
| `npm run build`（由Tauri构建调用） | 通过；主包约615kB，仍有既有体积提示。 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked` | 188项通过，2项原有原生凭据/进程树专项忽略。 |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets -- -D warnings` | 通过。 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | 通过。 |
| `npm run tauri -- build --debug --no-bundle --config output/playwright/video-runtime/tauri.verify.conf.json` | 前后两次独立标识调试EXE构建成功，嵌入生产前端；不是NSIS安装包。 |
| `node output/playwright/video-runtime/native.mjs seed`、`restart`、`final` | 分别完成编辑暂存、强退恢复/切换/试听、正常关闭后光标恢复。 |
| `node output/playwright/video-runtime/environment.mjs` | 四种环境场景、修复入口、三种宽度、检查不自动保存、显式保存回读通过。 |
| `node output/playwright/video-runtime/preview.mjs` | 30秒真实试片及原生播放通过。 |
| `git diff --check` | 通过。 |

原生启动/关闭使用`output/playwright/video-runtime/app.ps1 Start/Force/Close`。每次操作校验精确EXE和PID；正常关闭按PID定位窗口发送WM_CLOSE，不向其他窗口发送操作。首次验证脚本遇到测试任务日期标签和窗口标题变化，改为明确标签及PID定位后重跑通过；不是产品数据丢失或关闭错误。

使用独立应用标识`com.shuyin.video-runtime-verify`。Tauri现有路径逻辑将测试SQLite放在C盘专用目录；D盘output内存放音频副本、工作区、WebView数据目录设置、截图和验证EXE。测试只复制既有长片输入及源音频，不改源文件，不写正式库。Playwright通过CDP连接真实WebView2，没有使用mockIPC。

证据目录：`output/playwright/video-runtime/`，主要文件为`native-{seed,restart,final}.json`、`environment-result.json`、`preview-result.json`、`environment-{1440,1120,800}.png`、`native-restored.png`。每轮应用检查创建的`.video-check-*`目录均已结束并清理。

## 边界与下一步

未执行NSIS安装/升级、正式用户环境修改、macOS/Linux实机验证或整段15分钟视频的全量渲染与逐句听审。本轮完成的是隔离Windows调试构建的真实桌面验证及30秒试片。环境探测只查有限位置，其他目录中的安装需要手动选择；通过依赖检查也不能保证任意音频、显卡或长片任务成功。

下一业务切片建议实施“单镜重新生成”：填写修改要求、预览后采用，只替换选中的一镜。CI检查前移及开发Node版本统一仍为单独工程事项，本轮没有修改发布工作流。
