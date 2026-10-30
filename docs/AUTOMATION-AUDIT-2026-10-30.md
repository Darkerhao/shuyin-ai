# 音频与抖音自动化复核（2026-10-30）

源码与用户安装版版本字段均为 0.23.0。本轮完成本地回归、官方未登录页面识别和安装版状态观察；真实账号登录后的端到端验收尚未完成，不能宣称两条流程已经完整通过。

## 现场证据

用户反馈音频打开网页后不继续，显示登录、空白或错误页。Computer Use 观察安装版 `D:/书引 AI/shuyin-ai.exe`：当前书籍曾被《大明王朝1566》的旧音频任务阻塞，状态为等待人工核对远端任务，提示“原任务的笔记本身份无法确认，请核对网站；不会重新创建或提交生成”。这证实了现场阻塞，尚不能还原最初失败原因。随后任务卡不再显示，本轮没有点击停止、恢复或删除任务，不能认定已恢复。

独立浏览器打开 Notebook 官方入口后进入 `/trynow`，运行产品脚本得到 `login_required`；抖音上传入口实际显示登录页，检测结果为 `loginVisible=true`、`authenticated=false`、`kind=login`。独立浏览器不等于应用内 WebView 会话。没有读取或复制 Cookie，没有提交真实生成、上传或发布。

只读查询默认路径数据库得到 schema v7 且无 audio_jobs，与当前界面不符，因此不作为当前任务证据，也未修改该文件。已请求用户在应用内完成登录并保留故障窗口。

## 修复及影响

三个新增回归在修复前均失败，修复后通过；不将这些适配器缺陷冒充此次现场故障的全部根因。

| 问题 | 最小修复 |
| --- | --- |
| CSS 隐藏容器里的新建按钮参与匹配，错误报告多个入口 | 音频可见性检测排除祖先 display:none |
| 来源名称子元素被当作整行，漏掉同一列表行进度 | 优先读取最近的 li / role=listitem |
| 抖音只读取没有子元素的节点，文字旁有 SVG 就漏读状态 | 读取含直接文本节点的可见元素 |

修改文件：`src-tauri/src/audio_page.js`、`src-tauri/src/douyin_page.js`、`tests/audio-page.test.mjs`、`tests/douyin-editor.test.ts`、本报告及开发计划。无新依赖、权限、数据库结构或迁移；按 fix/patch 处理，未修改版本、安装版、提交或发布。

## 实际命令与结果

| 命令 | 结果 |
| --- | --- |
| `node --test --test-name-pattern='CSS-hidden\|row processing' tests/audio-page.test.mjs` | 修复前 2 项失败 |
| `npx vitest run tests/douyin-editor.test.ts` | 修复前新增项失败，17 项原测试通过 |
| `node --test tests/audio-page.test.mjs` | 修复后 31 项通过 |
| `npx vitest run tests/douyin-editor.test.ts tests/douyin.test.ts tests/publishing.test.ts` | 修复后 46 项通过 |
| `npx vitest run` | 最终 223 项通过，14 文件 |
| `npm run test:rust` | 最终 125 项通过，1 项既有凭据测试忽略；含迁移、重启、恢复和归档回归 |
| `npm run lint` | 通过 |
| `npm run build` | 首次清理 dist 被沙箱拒绝；正常权限重跑通过，含 TypeScript 检查，保留约 516 kB 分块提示 |
| `npm run check:rust` | fmt / 全目标 Clippy 通过 |
| `npm test` | 基线前端 222 通过；页面/打包/发布脚本 66 通过、1 跳过；队列 18 通过、1 失败，整体未通过 |
| `npm run test:queue` | 正常权限复测仍 18 通过、1 失败：tests/codex-queue.test.mjs:118 缺前驱报告未重跑前驱及后继；与本轮适配器修改无关，未扩展修改范围 |

Rust 命令先加载 `. ./.tools/env.ps1`，TEMP/TMP/SQLITE_TMPDIR 指向 `output/flow-test-temp`；Cargo 构建锁需正常权限执行。

## 验收与下一步

- 音频登录识别、创建、来源上传契约、提示词回读、生成去重、下载定位、暂停/恢复和归档：本地回归覆盖。实际生成及下载尚未验收。
- 抖音任务准备、账号确认、上传状态、文案、封面/合集及停止恢复：本地回归通过，登录后最新官方页面尚未验收。
- 需在应用内完成 Google 登录，再以选定来源实际走完创建、上传、配置、一次生成、下载、登记和重启核对。
- 抖音需登录并明确测试任务，走完上传、处理、文案、双封面、合集和人工发布前检查。最终发布仍由用户操作。
- 修复目前仅在源码中；安装升级、macOS/Linux 未运行。
