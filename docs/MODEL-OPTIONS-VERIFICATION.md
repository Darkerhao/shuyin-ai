# 模型地址与 Model ID 复用选项验收（2026-11-12）

## 实现范围

- 模型设置的服务地址与模型名称共用 `ModelOptionInput`，支持筛选、选择、回车新增、编辑和删除候选；输入内容后用方向键进入列表会保留当前筛选。
- 回车只保存候选项，不提交模型配置；选择候选项只回填当前表单，「保存模型」仍使用原命令和原校验。
- 两组候选分别保存在 `app_settings.model_options_base_url` 和 `app_settings.model_options_model_name`。首次读取从现有模型配置导入，用户删除为空后不会再次导入。
- 服务地址复用现有 URL 安全校验；模型名称与模型配置共用同一规范化函数，去首尾空白并按 200 字节上限校验。维护候选不修改 `model_providers`、API Key 或历史创作。
- 复用 Reka UI Popover 和现有输入、按钮、图标及语义颜色，无新增依赖、表结构、SQL 迁移或网络权限。

## 验证结果

| 检查 | 结果 |
| --- | --- |
| `npx vitest run tests/model-options.test.ts tests/ai.test.ts` | 2 个文件、106 项通过 |
| `npx vitest run --maxWorkers=2` | 36 个文件、526 项通过 |
| `npm run lint` | 通过，无警告 |
| `npm run build` | TypeScript 与 Vite 生产构建通过；保留既有大于 500 kB 分包警告 |
| `. ./.tools/env.ps1; npm run test:rust` | 288 项通过，2 项既有环境测试忽略 |
| `. ./.tools/env.ps1; npm run check:rust` | fmt 与 Clippy 通过 |
| `git diff --check` | 通过 |

真实浏览器在 `1366×900` 下通过模拟 Tauri IPC 验证：回车新增地址和模型、离开页面后重新读取、编辑候选、删除候选；页面未产生未处理 IPC 或运行时错误，维护候选期间 `save_model_provider` 调用数为 0。深色与浅色弹层均有实底、描边、阴影，宽度与输入框一致且位于视口内。截图记录：

- `output/model-options-20261112/dark-options-fixed.png`
- `output/model-options-20261112/light-options-fixed.png`
- `output/model-options-20261112/review-keyboard-filter.png`

键盘复审在 Chromium 中输入 `read` 后按方向键，弹层只保留 `reading-model` 一项，焦点也落在该匹配项。数据库回归覆盖首次导入、空列表不在重启后重导、去重、非法地址/模型名不覆盖原值、数据库备份恢复以及候选维护不改变模型凭据。

## 独立评审

按 Standards 与 Spec 两轴独立评审本轮改动。Spec 轴未发现缺失、范围蔓延或行为偏差；Standards 轴首轮发现一项键盘筛选失效和一项模型名称校验重复。修复后新增“输入筛选值后按方向键仍只定位匹配项”的回归，并让候选与正式模型配置共用 `normalize_model_name`。两轴复审均为 PASS，无剩余 findings。

## 环境边界

- 默认并行 worker 的 `npm test` 在当前机器上会使既有 `updates` 与 `voice-reminders` 用例超过 5 秒；相同用例单独重跑 90 项全部通过，限制为 2 个 worker 后完整 526 项通过。本次未修改这些无关测试或超时配置。
- `npm run test:video` 的失败来自本机 Node `24.11.0` 的已知 Windows 中文路径保护，且部分临时目录夹具未生成；与模型选项无关。
- `npm run test:queue` 有 1 项既有的“缺失前序报告后重跑后继任务”断言失败，其余 18 项通过；本次未改动队列脚本。
- `npm run test:release` 已执行到大量通过断言，但进程未自行退出，随后中止；未将其记为通过。
- 未用正式数据库、系统凭据库或已安装桌面程序做人工验收；真实 Tauri IPC 与安装版验证仍可后续补齐。
