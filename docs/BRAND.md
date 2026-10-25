# 书引 AI 品牌标识

2026-10-25：标识「引页」将翻开的书与播放键合为一个符号，表达从阅读到视频创作。左侧奶白书页保留阅读的温度，右侧向上翻起的青柠书页形成记忆点，播放键使用真实镂空。采用实色与清晰轮廓，不依赖文字、细线、阴影或动画。

| 颜色 | 色值 | 用途 |
| --- | --- | --- |
| 深墨绿 | `#153D32` | 图标底色、品牌字标 |
| 青柠黄 | `#D5F267` | 翻页、AI 标记 |
| 纸页白 | `#FFF9EC` | 左侧书页 |

## 使用与维护

- 唯一图形源文件为 `assets/app-icon.svg`，512×512，圆角外部透明。应用侧栏直接导入该 SVG，显示为40像素；浏览器标签图标与桌面打包图标都从它生成。
- 标准图标保持完整背景与比例，不拉伸、不旋转、不添加外围装饰。图标最小显示尺寸为16像素；文字组合使用40像素及以上图标。品牌组合为图标、书引字标和 AI 标记，小尺寸仅显示图标。
- 修改源文件后运行 `npm run icons:generate`。脚本复用已锁定的 Tauri CLI，只同步当前桌面配置使用的三张 PNG、ICO、ICNS 与 favicon；其余中间产物保存在忽略的 `output/brand-icons/`。
- 单色使用时，两张书页统一为同一实色，保留中缝与播放键的负空间。青柠黄用于品牌识别，不替代业务按钮的墨绿主题。
- 本次属于样式调整，发布类型为 `style`，默认 patch；不手动调整版本，无数据库、权限或依赖变更。

## 验收标准

1. 侧栏、favicon、Windows ICO、macOS ICNS 与 Linux 使用的 PNG 为同一图形。
2. 16、24、32、40、64、128、256像素下轮廓与播放键可辨认，无裁切或变形。
3. 桌面宽度与窄屏侧栏下品牌入口可见、可聚焦，保留「书引 AI 首页」的无障碍名称。
4. lint、TypeScript/生产构建及应用壳层相关测试通过；安装包内图标与操作系统缓存另作安装验收。

## 本次验证（2026-10-25）

- `npm run icons:generate`：通过，三张桌面 PNG、ICO、ICNS 和 favicon 已同步。Tauri 扫描系统字体时提示 `mstmc.ttf` 字体损坏；图形源没有文字或字体依赖，生成文件已解码并检查。
- `npm run lint`：通过；`npm run build`：TypeScript 检查与生产构建通过。
- `npx vitest run tests/projects.test.ts`：14项通过，覆盖使用应用壳层的书籍和设置流程。
- `git diff --check`：通过。
- Playwright：使用已有隔离内存 IPC 示例数据，执行 `npx --no-install --package @playwright/cli playwright-cli -s=shuyin-logo run-code --filename output/playwright/verify-logo.js`。1440、1120、800、360像素下，图标均为40×40、成功解码、处于侧栏内部；品牌组合无越界，首页入口可聚焦，页面无横向溢出，favicon 内容与源 SVG 一致。页面控制台0错误、0警告。
- Python/Pillow 解码核对：PNG 分别为32、128、256像素；ICO 包含16、24、32、48、64、256像素；ICNS 包含从16到1024像素的标准及 Retina 图像。16–64像素、单色和完整图形已在预览板检查，128/256像素在生成图标中检查。
- 预览板为 `output/branding/logo-presentation.png`，应用实景为 `output/branding/app-{1440,1120,800,360}.png`；均为忽略的本地产物。
- 以上验证对应四项标准中的源码、资源与浏览器界面部分；未重新构建/安装桌面包，未验证 Windows 图标缓存或 macOS/Linux 实机显示。数据库与业务流程没有变更，不需要迁移；本次未运行 Rust、迁移或完整测试套件。

下一步：在下一次桌面打包后确认安装图标、任务栏和桌面快捷方式显示一致。

## Logo 集成后的发布测试回归修复（2026-10-25）

发布流水线在 `prepare` 的前端测试阶段失败：`tests/publishing.test.ts:95` 期望两张封面，却得到 `['', '封面 v1：横版', '封面 v2：竖版']`。本地用以下命令复现了同样的失败：

```powershell
npx vitest run tests/publishing.test.ts -t 'requires choosing saved writing explicitly and previews selected exported covers without a model'
```

根因是测试挂载完整 `App`，随后用 `wrapper.findAll('img')` 查询整个应用。侧栏 Logo 从内联 SVG 改为 `img` 后，装饰图片的空 `alt` 也被当成封面结果。封面显示与发布逻辑正常。

最小修复：将查询限定到 `wrapper.getComponent(PublishingPage)`，仍严格核对两张封面的数量、顺序与完整 `alt`，保留真实 Logo 和完整应用挂载。没有过滤空值、放宽断言或修改产品代码。

修复后上述单项测试通过；`npm test` 的完整前端95项通过，发布脚本27项通过、1项 macOS 专属检查按原设置跳过；`npm run lint`、`npm run build`（包含 TypeScript）和 `git diff --check` 均通过。原始失败用例已恢复通过，严格的封面预览断言保留，满足本次回归修复验收。

没有数据库、依赖、权限或版本变更；发布影响为 `fix`，由后续流水线处理版本。本地修复未提交、推送或重跑远端流水线，本次未运行 Rust 或桌面打包。下一步将修复合入 main 后确认新的 `prepare` 任务通过，再验收后续打包阶段。
