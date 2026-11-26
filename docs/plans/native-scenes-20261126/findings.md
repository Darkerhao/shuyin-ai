# 专属主题场景迁移发现

- 当前主题已进入 META.style，42 项共用 themes.json；上一轮改动全部仍在工作区。
- 当前 Scene/Beat 为受控物体目录与 cue 索引，原图像场景和历史 ink 需要继续使用已有路径。
- 源引擎每个主题的 kit.js 不由 boot.js 自动加载，需要添加 META.extras，且 kit 会覆写 Builder.title/text，必须按主题加载。
- pixel 提供 Builder.pixel、pixelText 和 painter 的 hero、sprite、场景绘制。
- manga 提供 grid/panel、chara、prop、scene；台词数据不得直接当可执行代码或 HTML。
- 三维需 META.vendor=['three']、META.webgl=true，runtime/three.js 共享 WebGL renderer。
- 待确认：每个三维 kit 的对象 API、支持物体以及非对应题材的无事实暗示表达方式。
- 决策：新增 26 个主题专属主体，复用 Beat.object，不扩展 Scene/数据库结构。三维无可匹配主题主体时继续原图解，避免强行把普通题材变成天体/医学结构。
- 首轮实际 Chromium 已验证像素画布、漫画分格与五种三维主题，无引擎错误，三维 canvas 回退寻帧一致。截图需在导航完成后设置 1920×1080，否则固定舞台超出默认浏览器视口。
- 像素普通物体复用既有受控几何，经 SVG 解码成低分辨率精灵；专属角色使用原 g.hero/sprite。漫画通过原 P.grid/P.panel 和 P.prop/chara，普通非原生道具复用同一 sketch 函数。
- 三维实际 API：P.space().body/label/cameraTo、P.diorama().city/zone/landmark、P.studio().turn/explode/assemble、P.globe、P.micro().molecule/helix/cell。globe/kit.js 还动态载入相邻 world.js。
- source engine runtime/three.js 共享 WebGL renderer；部分专属 kit 另有全局缓存 renderer。ENGINE.init 会执行两遍，适配器将 GPU 分配推迟到可见页面首次 tick，避免字体探测阶段重复分配。
- 原引擎部分 label/title 使用 innerHTML；适配器将外部文字转义后传入，普通标签使用原 text raw 模式。
- 实际导出发现：pixel/manga/cosmos 六秒导出成功，产品 motor 在引擎 determinism 预检失败（6.28%差异）；宇宙联系表一帧缺主体。HTTP Ego Chromium 同场景冷/暖截图一致。源 tools/lib.mjs 对 META.webgl 无平台判断地传 --use-angle=metal（macOS），FRAMETELL_CHROME_ARGS 的平台后端参数可覆盖。Windows d3d11 对照验证后 motor 预检通过、宇宙缺失帧消失；已接入平台参数选择。
- 新 Vue 端到端 UI 单测需打开 100+项物体下拉，重复触发5秒上限；改为直接触发 Select 公共事件验证状态联动与保存，真实浏览器单独覆盖下拉交互。
- 第三方只使用当前本地 Frametell 0.7.3 源码与 GRAMMAR 进行调用；无升级计划。
