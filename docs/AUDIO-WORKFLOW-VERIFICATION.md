# 音频制作与成片回填验收（2026-10-26）

## 1. 本轮范围

> 2026-10-26 后续行为调整：最终成片取消竖版 9:16 限制，仅要求 MP4/MOV/M4V 可解析、时长有效、画面尺寸可读取且包含音轨。以下验收记录保留调整前的历史结果。

实现“外部生成音频 → 导入音频 → 剪映生成字幕/背景并导出 → 应用登记最终成片 → 发布检查 → 抖音辅助发布”的本地闭环。

- 「音频制作」页签支持复制现有音频指导、打开固定 Gemini Notebook 地址、导入并打开音频成品。
- 音频支持 MP3、M4A、WAV、AAC、FLAC、OGG；只登记路径，不复制文件。
- 发布任务新增音频引用，最终成片仍单独选择；抖音只使用成片、文字和双封面。
- 最终视频检查限 MP4/MOV/M4V，验证正时长、竖版 9:16（允许 2% 偏差）和音轨。
- 不自动操作 Gemini Notebook，不读取 Cookie，不接管剪映，不自动发布。

## 2. 数据库与任务包

- `0010_audio_workflow.sql` 重建 `assets`、`publish_tasks`、`publish_records`，增加 `audio` 素材类型和可空 `publish_tasks.audio_asset_id`。
- v9 旧项目、视频素材、发布任务和发布记录保留；迁移冲突回滚到 v9。
- 当前 `user_version=10`，数据库备份支持 v2–v10。
- 任务包 manifest 升级为 schema 2，增加可空 `audio` 文件引用。旧包仍可反序列化，但缺少音频的待发布任务不会保持 ready。
- 音频文件纳入完整备份的普通登记素材；发布任务包不复制音频，也不提供音频上传入口。

## 3. 媒体检查实现

- 新增锁定依赖 `mp4parse 0.17.0`。
- 只扫描顶层 MP4 box，读取 `ftyp` 与 `moov`，不遍历完整 `mdat`。
- 读取视频轨道和音频轨道；使用 `tkhd` 取得 `16.16` 定点显示宽高，使用轨道 duration/timescale 取得秒数。
- 不支持格式、损坏容器、无视频轨、无音轨、非正时长或非竖版 9:16 都使发布任务保持草稿。
- 测试素材：`src-tauri/tests/fixtures/portrait.mp4`，1080×1920、含 AAC 音轨、约 1 秒。

## 4. 自动验证

| 命令 | 实际结果 |
| --- | --- |
| `npm run lint` | 通过，0 warning |
| `npm run typecheck` | 通过 |
| `npm test` | 12 个测试文件、132 项通过；发布脚本 30 通过、1 个 macOS 专属跳过；Codex 队列 19 通过 |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked` | 82 通过、1 个需真实系统凭据的测试忽略、0 失败 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | 通过 |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets -- -D warnings` | 通过 |
| `npm run build` | 通过，Vite 产物生成到 `dist` |
| `npm run desktop:build`（已加载 `.tools/env.ps1`） | Release EXE 与 NSIS 安装包生成；随后因未提供 `TAURI_SIGNING_PRIVATE_KEY` 在更新包签名阶段失败 |

桌面构建产物：

- `src-tauri/target/release/shuyin-ai.exe`
- `src-tauri/target/release/bundle/nsis/书引 AI_0.14.0_x64-setup.exe`，5.25 MiB

未配置签名私钥不表示编译或 NSIS 打包失败，但当前安装包不能视为已完成签名发布。

## 5. 覆盖点

- 外部音频入口和音频导入不改变已保存提示词。
- 音频缺失、损坏或任务引用其他书籍时不能生成任务包。
- 非 MP4/MOV、损坏视频、横版、3:4、无音轨和假 `.mp4` 不能通过最终成片检查。
- 音频/视频文件丢失、任务包被修改后 ready 自动退回 draft。
- v9→v10 正常迁移和冲突回滚。
- 完整备份/恢复保留音频、视频、封面、任务包和发布记录。
- 旧待发布任务升级后需补选音频并重新生成任务包；已发布记录不变。

## 6. 尚未执行的人工验收

- 在真实 Windows 安装包中打开 Gemini Notebook、登录并下载音频。
- 使用实际剪映工程生成字幕和背景，导出 9:16 MP4。
- 通过原生文件选择器导入音频和成片。
- 核对页面视觉、窄窗口布局和无障碍键盘操作。
- 用真实成片执行发布检查、抖音上传、预览和人工发布记录。

这些项目需要在对应外部账号、剪映工程和桌面环境中完成，不能由单元测试代替。
