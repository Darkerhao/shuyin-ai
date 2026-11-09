# 计划收尾验收（2026-11-09）

状态：进行中。用户已授权从现有书库选三种体裁建立副本，调用现有服务制作三份音频、九张封面和视频，上传当前抖音账号草稿并停在最终发布前。P4官方自动提交尚未交付，不以本次网页验收替代。

## 最新用户修订：取消实际音频时长上限

用户在验收过程中要求取消音频时间限制。自动流程的900秒门槛、视频字幕/渲染入口的7200秒门槛均已移除，目标只用于规划；自动推荐不再发送15分钟限制。删去重复validate_target，沿用统一目标输入验证，推荐Prompt更新为audio-duration-v3，UI和现行计划已同步。无数据库迁移、依赖、权限或手工版本号变更。

对应回归先在旧代码复现：20分钟目标被拒、推荐仍被限制、3小时时间轴校核与Node媒体入口失败。修复后`npm run test:rust`262通过/2既有忽略，Node视频与音频脚本85通过；lint、fmt/Clippy和包含TypeScript/Vite的release构建通过。真实已归档1099.72秒音频由新渲染入口成功读取，三个小时用例为受控元数据/字幕回归，不冒充3小时完整实片验收。

实际命令：`cargo test --manifest-path src-tauri/Cargo.toml --locked automation`（改前30通过/3失败），`node --test --test-name-pattern='probe accepts long audio' tests/video-render.test.mjs`（改前失败）；修改后`npm run test:rust`、`node --test tests/video-render.test.mjs tests/audio-page.test.mjs tests/audio-package.test.mjs`、`npm run lint`、`npm run check:rust`、`npm run desktop:build -- --no-bundle --config output/playwright/acceptance-20261109/tauri-candidate.json`均通过。Node使用已有24.22.2，Rust先加载`. ./.tools/env.ps1`。

真实媒体检查：`node src-tauri/video/render.mjs output/playwright/acceptance-20261109/long-audio-probe-input.json output/playwright/acceptance-20261109/long-audio-probe-result.json`返回duration1099.72/engineVersion0.7.3。新候选`shuyin-unlimited.exe`重开保留当前音频、精确文案/视频/背景ID并续作，未覆盖正式安装程序。以下早期15分钟暂停记录为用户修订前的历史。

## 当前证据

- 源码基线dev/5765b74；用户已将正式安装版更新至0.55.0，实际数据库schema23。测试通过情况来自本轮执行，日志位于`output/acceptance-20261109/`。
- `npm test`：447项前端、39项视频、78项发布/音频脚本通过，1项macOS专用跳过；19项任务队列通过。视频测试使用现有Node24.22.2。
- `npm run test:rust`：260通过、2项既有忽略；`npm run lint`、`npm run check:rust`、`npm run desktop:build -- --no-bundle`通过，包含TypeScript/Vite。大分块及npm既有配置提示保留。
- 原始安装进程与重开进程读取了不同业务快照，出现《法治的细节》ID、封面及历史发布记录差异；两份库和逐表差异已保留。用户批准恢复10:44快照后，16张表核对与两次启动通过：18本书、132封面、12发布记录；仅备份元信息按操作更新。历史视图差异根因仍未确定，恢复不算根因修复。

## 真实书籍

| 书籍 | 体裁 | 状态 |
| --- | --- | --- |
| 显微镜下的大明 | 历史 | 流程960c00cf8152896c107903a80d7618dd已到真实awaiting_publish并暂存离开；有故障恢复/人工网页核对，属于辅助通过 |
| 文城 | 小说 | 副本673aaef742208ba8dd413ce338d53764已创建，保存600秒目标，待开始 |
| 人生由我 | 传记 | 副本b350ce2e5e6431141f40f024d535741f已创建，保存600秒目标，待开始 |

第一册项目`9013e77a8c9de25c6cf98a150fb71caa`、流程`3700184b771d39588e0a9076f66799b8`、创作版本`8389841e936dae3f2285303b5750ac4b`、音频任务`6643dc65a25653ba0813b99750343289`。前置指导1901字，真实生成前逐字回读相同；发生过上传和音频配置恢复操作，不能算正常零中途操作通过。

上述首轮仅保留作故障恢复证据：真实转写得到482条字幕，发现验收输入的书名后缀被固定结尾朗读。已停止该轮后续制作，未产生图片；三个副本改为真实书名，通过精确项目ID区分。修复候选从同一项目启动新流程，新版本44a0e4cabed3bc79a31e83cfb635fbff、音频c5ff5a8d00a374d2715f4803a813bc19，1831字指导与版本绑定，自动上传和提交无需人工操作。保留额外一次首轮音频及模型费用，计数不冒充只有三次音频尝试。

该15分钟目标流程进一步自动完成下载/归档/窗口关闭，实际1099.72秒（18:19），不能进入既有15分钟制作上限。已保存为`book1-duration-boundary.json`并停止后续ASR；代码在字幕校对后才检查超限，本轮未等待该晚期自动暂停，不将人工停止记为自动处理通过。最终三本流程改用600秒目标和NotebookLM既有短时长选项；保留额外两份首册音频及所有调用，原声不裁切不变速。

音频25992397字节，AAC立体声/44100Hz/256kbps，实际807.079秒，目标900秒。现有FFmpeg的probe和只读响度测量退出0：-20.42 LUFS、-4.01 dBTP、LRA5.10；声音偏轻，未做归一化或其他音频变更。初次媒体检查因当前PATH没有ffprobe退出127，改用已有D盘FFmpeg后通过，没有安装新依赖。

首次下载等待由调试工具接管导致；依据既有原生下载验证约定恢复`Browser.setDownloadBehavior=default`后，同一成品原生下载/素材归档成功且窗口自动关闭。未修改下载产品代码，也未再次生成音频。该恢复操作单独记录，不能冒充无介入通过。

## 上传故障与最小修复

真实NotebookLM新建原笔记本后，原书上传报`file_input_missing`。当前官方上传器在首次`focus`或`mouseover`时创建隐藏文件输入；原生网页失焦时`focus()`不触发，输入不存在。

同一原笔记本、同一当前网页中，关闭Playwright默认焦点模拟后复现：`document.hasFocus()=false`、旧代码返回错误、输入数0。将`upload.focus()`替换为`mouseover`事件后，同样失焦条件返回`file_input_ready`、唯一输入正确标记。继续保留现有来源弹窗和输入唯一性校验，没有增加状态、依赖或重试。

`node --test --test-name-pattern='native window is unfocused' tests/audio-page.test.mjs`修复前失败；修复后`node --test tests/audio-page.test.mjs tests/audio-package.test.mjs`45通过。`npm run lint`及修复候选release构建通过。当前官方脚本、真实前后结果和回归日志保存于`output/playwright/acceptance-20261109/`。

修复候选为该目录的`shuyin-acceptance.exe`，沿用原identifier和系统凭据；未覆盖`D:/书引 AI/shuyin-ai.exe`。构建使用临时配置显示0.55.0，项目版本文件未更改，未打包发布或提交Git。

## 待完成

第二、三本真实制作与网页准备、原生拖入、最终全部成果持久化仍待完成。第一册已保存官方草稿；当前网页显示“你还有上次未发布的视频，是否继续编辑？”，应用拒绝在已有草稿上开始下一册自动流程。已询问是否允许在保留本地完整材料的前提下替换本轮第一条验收草稿，尚未得到答复，不清除或覆盖该草稿。原安装0.55.0仍含已复现的失焦上传错误及旧时长门槛；目前修复运行于独立EXE候选，未生成/发布正式升级包。P4及历史数据视图差异分别保留未完成结论。

## 第一册实际成果与官网检查

最终音频345.12秒，162条校对字幕、35个分镜；三张AI背景生成并实际导出1440×1080、1080×1440、1920×1080PNG。竖版请求一次HTTP524后单独重试成功，横版ID及文件保持；没有重做成功尺寸。已查看三张图片，文字完整、尺寸正确；横版主标题最后一字单独换行，作为排版待优化项保留。

视频30575168字节、345.125秒、1920×1080、24fps、8283帧、H.264 High/yuv420p/8bit和AAC双声道。`probe.py`和`check.py --platform custom --aspect 16:9 --no-loudness`退出0；格式检查无失败，只有无独立可切换字幕轨提示，视频画面已烧录中文字幕，另有SRT素材。`look.py --tiles 3x2 --width 1920`抽帧已视觉检查，标题/图形/字幕清楚且不越界；没有进行完整事实听审。原声测量-20.82LUFS/-4.42dBTP，偏轻，未修改原声。

抖音实际视频元数据345.125秒/1920×1080，播放时间从0推进到1.496秒、无媒体错误；平台未给出机器可核实的完成信号，通过原人工处理确认继续。真实标题/正文/标签、两封面、无合集、禁止保存、2026-11-10 01:11均由原流程填写并严格回读；最终封面指纹变化经目视核对后确认，平台显示封面检测通过。发布会话manual=[video,covers]，不能计作零中途操作成功。

发布任务`1d1f3b8f98e603818badbcac23915300`为ready，自动流程为awaiting_publish，实际发布记录0。已点击“暂存离开”，没有点击“发布”。官方页面的同时发布到红果选项仍按账号原有状态显示，AI自主声明尚未选择，最终提交前需用户确认内容。

`python -X utf8 output/playwright/acceptance-20261109/verify_snapshot.py <本轮ready快照> 960c00cf8152896c107903a80d7618dd output/playwright/acceptance-20261109/book1-verified.json`实际通过：原书/音频摘要、创建与校核版本、音频指导、唯一视频任务、完整字幕、三PNG头尺寸、发布精确关联、未来01:11、禁止下载及零发布记录均核对。快照路径在`book1-ready.json`，逐项结果在`book1-verified.json`。

与获批恢复基线逐行比较，原18本书、120素材、13音频、12视频、132封面、26文案、16发布任务、12发布记录、11草稿、9模型及原自动流程全部保留无更改；只新增本轮验收记录。证据为`original-data-preservation.json`。
