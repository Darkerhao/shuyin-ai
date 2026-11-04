# 原声播放器设计与验收

日期：2026-11-04。状态：已实施，通过前端、浏览器与Windows隔离构建验收。

## 设计与范围

下载完成后在内容创作的任务卡直接试听；其他已导入音频在成品列表试听。视频制作的音频选择区、字幕及分镜试听复用同一个 `AudioPlayer` 组件。当前下载成品不在同一页重复创建播放器。

以用户提供的`modern_audio_preview_player.html`和截图为视觉参考，采用深墨绿玻璃和OLED暗黑两套配色。墨绿底`#10201bf5`、暗黑底`#101418f5`、纸白`#f3f7f5`、祖母绿`#19c58c`、浅绿`#72e9bf`与灰绿`#a9bcb4`；暗黑配色改用柔和青色。边缘使用微光与18px毛玻璃，文字延续项目系统字体，标题18px/窄窗14–16px、辅助文字12px、时间使用等宽数字。中间为全宽80px波形；下方56px圆形播放键置于前后15秒之间，右侧配色、倍速与音量按容器宽度换行。

波形为本地音频8kHz解码后的幅度包络，按可见宽度绘制24–128条柱；已播放部分用绿色渐层，发光指针逐帧读取真实媒体位置。悬停只显示目标时间，点击/拖动与键盘操作继续使用原Slider。文件解码按可见区域触发并串行执行，内存最多缓存4份256数值包络。EQ律动与流体边缘仅表示播放状态，减少动态效果时关闭；没有新字体、播放器库或持久播放设置。

参考HTML通过随机数模拟波形、通过振荡器合成和弦，并写死音质信息；本轮只采用其视觉和控制方式，实际播放本书已下载的原声，不添加合成和弦，不假设无损或48kHz/24bit。旧HTML的时间格式按64进制计算，正式组件按60秒/分钟显示。波形解码只读文件，主窗口CSP补已有本地asset的connect-src，单文件授权与其他权限保持原边界。

## 验收标准

- 下载归档完成后直接显示播放器；点击播放，不自动发声。
- 手动导入音频可就地试听；当前下载结果仅使用一个音频元素。
- 播放/暂停、拖动与键盘进度、前后15秒、倍速、音量和静音均操作真实媒体。
- 波形来自实际文件，悬停显示正确时间，配色切换不打断声音；解码失败仍可播放和寻迹。
- 同页播放另一份音视频时暂停之前的媒体；切换音频、任务、页签及卸载时停止旧播放。
- 视频新建选择与任务原声各自绑定正确素材；单句、分镜试听仍按原时间码停止并联动。
- 缺失文件、加载失败及不支持的音频有明确反馈和显式重试，原稿与业务数据保留。
- 桌面和窄窗口无横向溢出；键盘焦点与减少动态效果有效。

## 验证记录

| 实际命令 | 结果 |
| --- | --- |
| `npm run lint` | 通过，0警告 |
| `npm run typecheck` | 通过；最终桌面构建再次运行同一TypeScript检查 |
| `npx vitest run tests/audio-player.test.ts tests/audio-waveform.test.ts tests/video-subtitles.test.ts tests/video.test.ts tests/ai.test.ts --maxWorkers 2` | 波形阶段133项通过；随后新增音视频互斥/配色保持用例在全量测试中通过 |
| `npx vitest run --maxWorkers 2` | 最终28个测试文件、381项通过；覆盖真实幅度计算、静音波形、同源合并、串行解码、波形失败保留seek、悬停不seek、完整控制、音视频互斥、切换/卸载、任务来源与字幕时间码 |
| `npm run build` | TypeScript及Vite构建通过；最终桌面构建调用同一脚本，JS697.39kB / gzip220.63kB，保留既有500kB分块提示 |
| `node D:/DevCache/npm-cache/_npx/31e32ef8478fbf80/node_modules/@playwright/cli/playwright-cli.js -s=shuyin-player run-code --filename output/playwright/audio-player/setup.js --json` | 加载当前Vue源码、内存Tauri IPC和真实66秒WAV夹具 |
| `node D:/DevCache/npm-cache/_npx/31e32ef8478fbf80/node_modules/@playwright/cli/playwright-cli.js -s=shuyin-player run-code --filename output/playwright/audio-player/check.js --json` | 最终JSON解析为成功；下载状态变化后就地播放器、真实声音时间推进、0.1秒键盘寻迹、拖动/悬停、15秒跳播、倍速/音量/静音、切换互斥/隐藏暂停、导入保稿、两套配色、字幕/分镜来源隔离、失败显式重载及减少动态效果通过；页面异常0 |
| `. ./.tools/env.ps1; npm run desktop:build -- --debug --no-bundle --config output/playwright/audio-player/tauri.native.conf.json` | 最终TypeScript/Vite与Windows Tauri构建通过，未生成或安装发行安装包 |
| `node output/playwright/audio-player/native.cjs` | Windows WebView2 + 真实Tauri IPC + 隔离SQLite通过；实际M4A663.858503秒、21,383,040字节，82条幅度柱/81种高度，悬停时间、播放/暂停、键盘、跳播、倍速、静音、两套配色、单句/分镜及正常重启后播放通过 |
| `git diff --check` | 通过 |

浏览器在内容创作和视频制作各1440/1120/800/375px共8组检查，无页面或播放器横向溢出；键盘焦点为2px实线，减少动态效果时EQ动画为none。真实WAV包络有87种高度，拖动后媒体位置为41秒；第1句实际停在2.816591秒，目标2.8秒，仍在原时间窗末端允许的调度范围内。最终结果见`output/playwright/audio-player/browser-final.json`。

Windows使用独立identifier`com.shuyin.audio-player-20261104`及仓库内工作目录。从用户已下载M4A复制测试输入，原件和副本SHA-256前后相同；没有改写用户数据库。音频成功与视频任务使用明确的隔离数据库夹具，不访问Google、不重新下载、不提交生成，也不冒充真实业务任务成功。原生第1句停止在2.851857秒，原文案JSON保持；schema17、integrity ok、外键异常0、AI任务/模型请求0、页面异常0。正常关闭后重新启动仍可从成功任务卡读取并播放原声，结果见`native-result.json`。

截图已查看：`native-player-detail.png`、`native-player-oled.png`、`native-subtitle-player.png`为Windows实际M4A效果；`player-detail.png`、`player-oled.png`、`writing-1440.png`、`writing-375.png`、`video-1440.png`、`video-375.png`为浏览器布局证据，均位于`output/playwright/audio-player/`。

初次回归发现jsdom缺少ResizeObserver/IntersectionObserver及旧测试只触发click而未切换Reka页签，已按当前库源码补齐测试环境与真实键盘操作；没有添加生产兜底。新增波形mock被resetAllMocks清除导致渲染失败，已修正夹具初始化；原生浏览器验收不使用这些mock。微光伪元素外扩1px曾导致播放器scrollWidth多1px，改为框内绘制后最终8组布局通过。早期的细线播放器验证属于本次调整前结果，不作为最终样式证明。

## 修改文件

- 播放与视觉：新增`src/components/{AudioPlayer,AudioWaveform}.vue`；修改`src/components/AppIcon.vue`、`src/components/ui/slider/Slider.vue`。
- 页面：`src/modules/writing/WritingPage.vue`、`src/modules/video/{VideoPage,VideoSubtitleEditor}.vue`，移除旧音频URL状态、重复打开/播放逻辑和原生音频controls。
- 共享读取：新增`src/services/audioWaveform.ts`、`src/lib/mediaTime.ts`；`src/services/projects.ts`集中mediaSource，`src/services/video.ts`移除重复入口；`src-tauri/tauri.conf.json`仅调整本地asset的CSP。
- 回归：新增`tests/{audio-player,audio-waveform}.test.ts`；更新`tests/{ai,video,video-subtitles}.test.ts`、`tests/ui.ts`。
- 文档：`docs/{PRD,TECHNICAL-DESIGN,DATABASE,AI-RULES,USER-GUIDE,DEVELOPMENT-PLAN,AUDIO-PLAYER-VERIFICATION}.md`。

没有新增依赖、IPC、capability、SQL迁移或版本号改动，未暂存、提交或发布。包含新增就地试听能力，版本影响为feat/minor。

## 验收对应

| 标准 | 证据 |
| --- | --- |
| 下载后就地播放、单个成品不重复播放器 | 浏览器模拟归档状态更新、前端成功任务回归；Windows实际成功夹具/同素材读取 |
| 手动导入和真实播放控制 | 浏览器真实WAV导入保稿及所有控制；Windows实际M4A时间推进/跳播/倍速/静音 |
| 真实波形、悬停/拖动、两套配色 | Web Audio真实文件解码、灰/绿幅度与发光指针截图；两套主题切换保留媒体状态 |
| 音视频互斥与离页停止 | 捕获play事件的前端回归；浏览器另一音频播放/隐藏页签验证 |
| 原声/单句/分镜与来源隔离 | 前端、浏览器及Windows单句句尾停止与第2镜寻迹，改变新任务选择仍保留任务原声 |
| 失败保留、可访问、窄窗/减少动态效果 | 波形失败与媒体读取/播放失败回归、显式重试；8组布局与真实焦点/动画计算样式 |
| 数据保持 | Windows原文案JSON与音频SHA-256、schema/完整性/外键只读核对 |

## 实现依据

- 本仓库Vue3.5.44的组件expose/ref实现和[模板引用文档](https://vuejs.org/guide/essentials/template-refs.html)。
- 本仓库RekaUI2.11.0的SliderRoot/SliderThumb、键盘、指针和useSize源码及[Slider公开API](https://reka-ui.com/docs/components/slider)，复用已有控件，不引入另一套拖动或下拉实现。
- Tauri JS2.12.4的convertFileSrc、Rust2.12.5的asset协议CORS/单文件scope源码及[当前API文档](https://v2.tauri.app/reference/javascript/api/namespacecore/#convertfilesrc)。
- Web Audio的[decodeAudioData](https://developer.mozilla.org/en-US/docs/Web/API/BaseAudioContext/decodeAudioData)只接受完整文件，并按Context采样率重采样；[OfflineAudioContext](https://developer.mozilla.org/en-US/docs/Web/API/OfflineAudioContext/OfflineAudioContext)用于只读低采样率解码，不创建声音输出。未引入外部音频库。

## 数据与运行边界

无数据库迁移，schema17；不修改音频、正式字幕/分镜、文案历史或发布记录。配色和播放设置仅在本组件保留。波形首次需要整文件解码，内存占用与时长、声道有关；8kHz、可见时加载及串行处理减少占用。两小时/多声道文件的首次波形性能需另行实测；此限制不阻止原声播放。

本轮未替换正式安装版、未重跑Google生成与下载、未做macOS/Linux或两小时多声道性能验收。正常主窗口构建需在后续正式打包更新中提供此功能；本次隔离构建不代表正式安装升级通过。下一步使用包含本次改动的正式桌面构建，在真实书籍工作流完成一次下载后的完整试听。
