# 书引 AI 数据库设计文档

应用更新偏好（2026-10-25）：复用既有 `app_settings`，键 `auto_update` 保存字符串 `true` / `false`，未设置默认为开启；通过受限 bool 命令读取/修改，随数据库备份恢复。版本、下载内容与进度属于运行状态，不新增业务表。schema 保持 v9，无迁移脚本。

## 1. 设计原则

- SQLite 作为第一版结构化业务数据主存储。
- 使用迁移脚本维护数据库版本。
- 使用事务保证关联数据一致性。
- 业务主键建议使用 UUID。
- 时间统一保存为 UTC 时间戳或规范 ISO 8601 字符串，并在 UI 转换为本地时间。
- 大型文件存储在文件系统，数据库保存路径与元数据。
- API Key 不存入普通业务表。
- 删除项目不得默认删除原始视频文件。
- 对常用过滤字段建立索引。
- JSON 字段仅用于适合扩展的结构，不用 JSON 取代所有关系表。

## 2. 核心表

> 下列为整体模型设计；M0 实现见第 7 节，M1 实际表及暂缓字段见第 8 节，M2 实际模型及创作版本见第 9 节。

### 2.1 book_projects

| 字段 | 类型 | 说明 |
|---|---|---|
| id | TEXT PK | 项目 ID |
| title | TEXT NOT NULL | 书名 |
| author | TEXT | 作者 |
| publisher | TEXT | 出版社 |
| isbn | TEXT | ISBN |
| description | TEXT | 简介 |
| source_notes | TEXT | 用户资料与笔记 |
| target_audience | TEXT | 目标受众 |
| content_style | TEXT | 内容风格 |
| target_duration_seconds | INTEGER | 目标时长 |
| status | TEXT | 项目状态 |
| workspace_path | TEXT | 工作目录 |
| created_at | TEXT | 创建时间 |
| updated_at | TEXT | 更新时间 |
| archived_at | TEXT | 归档时间，可空 |

### 2.2 script_versions

| 字段 | 类型 | 说明 |
|---|---|---|
| id | TEXT PK | 版本 ID |
| project_id | TEXT FK | 所属项目 |
| version_no | INTEGER | 版本号 |
| outline_json | TEXT | 大纲结构 |
| content_markdown | TEXT | 讲稿 |
| source_type | TEXT | ai / manual / mixed |
| prompt_version | TEXT | Prompt 版本 |
| model_name | TEXT | 模型名称 |
| is_confirmed | INTEGER | 是否确认 |
| created_at | TEXT | 创建时间 |

一个项目可以有多个讲稿版本。建议对 (project_id, version_no) 建立唯一约束。

### 2.3 publish_materials

| 字段 | 类型 | 说明 |
|---|---|---|
| id | TEXT PK | 素材 ID |
| project_id | TEXT FK | 所属项目 |
| material_type | TEXT | title / description / hashtags / cover-copy |
| content | TEXT | 内容 |
| version_no | INTEGER | 版本号 |
| is_selected | INTEGER | 是否选中 |
| is_confirmed | INTEGER | 是否确认 |
| model_name | TEXT | 模型名称 |
| created_at | TEXT | 创建时间 |

建议对项目、素材类型和版本号建立索引。

### 2.4 assets

| 字段 | 类型 | 说明 |
|---|---|---|
| id | TEXT PK | 素材 ID |
| project_id | TEXT FK | 所属项目 |
| asset_type | TEXT | video / cover / image / subtitle / document |
| file_path | TEXT | 路径 |
| original_name | TEXT | 原始文件名 |
| mime_type | TEXT | MIME 类型 |
| file_size | INTEGER | 文件大小 |
| checksum | TEXT | 校验值，可空 |
| width | INTEGER | 宽度 |
| height | INTEGER | 高度 |
| duration_seconds | REAL | 视频时长 |
| exists_status | TEXT | available / missing / unknown |
| created_at | TEXT | 创建时间 |

### 2.5 cover_versions

| 字段 | 类型 | 说明 |
|---|---|---|
| id | TEXT PK | 封面版本 ID |
| project_id | TEXT FK | 所属项目 |
| background_asset_id | TEXT FK | 背景素材，可空 |
| exported_asset_id | TEXT FK | 导出文件，可空 |
| template_id | TEXT | 模板 ID |
| layout_json | TEXT | 可编辑排版配置 |
| version_no | INTEGER | 版本号 |
| is_selected | INTEGER | 是否选中 |
| created_at | TEXT | 创建时间 |

### 2.6 model_providers

| 字段 | 类型 | 说明 |
|---|---|---|
| id | TEXT PK | 配置 ID |
| name | TEXT | 显示名称 |
| provider_type | TEXT | 服务类型 |
| base_url | TEXT | 服务地址 |
| model_name | TEXT | 模型名称 |
| capabilities_json | TEXT | 能力声明 |
| is_default | INTEGER | 是否默认 |
| is_enabled | INTEGER | 是否启用 |
| credential_ref | TEXT | 系统凭据引用，可空 |
| created_at | TEXT | 创建时间 |
| updated_at | TEXT | 更新时间 |

不得保存 API Key 明文。credential_ref 只保存安全凭据引用或标识。

### 2.7 ai_tasks

| 字段 | 类型 | 说明 |
|---|---|---|
| id | TEXT PK | 任务 ID |
| project_id | TEXT FK | 所属项目 |
| task_type | TEXT | 任务类型 |
| provider_id | TEXT FK | 服务配置 |
| model_name | TEXT | 模型名称 |
| status | TEXT | 任务状态 |
| input_json | TEXT | 脱敏后的任务输入 |
| output_json | TEXT | 输出摘要 |
| error_code | TEXT | 错误码 |
| error_message | TEXT | 脱敏错误 |
| started_at | TEXT | 开始时间 |
| completed_at | TEXT | 完成时间 |
| created_at | TEXT | 创建时间 |

禁止将密钥、完整敏感请求头写入任务输入或日志。

### 2.8 publish_tasks

| 字段 | 类型 | 说明 |
|---|---|---|
| id | TEXT PK | 发布任务 ID |
| project_id | TEXT FK | 所属项目 |
| platform | TEXT | douyin 等 |
| video_asset_id | TEXT FK | 视频素材 |
| cover_version_id | TEXT FK | 封面版本 |
| title_material_id | TEXT FK | 标题素材 |
| description_material_id | TEXT FK | 描述素材 |
| status | TEXT | 状态 |
| task_directory | TEXT | 任务包目录 |
| checklist_json | TEXT | 检查结果 |
| created_at | TEXT | 创建时间 |
| updated_at | TEXT | 更新时间 |

### 2.9 publish_records

| 字段 | 类型 | 说明 |
|---|---|---|
| id | TEXT PK | 记录 ID |
| publish_task_id | TEXT FK | 发布任务 |
| platform | TEXT | 发布平台 |
| published_at | TEXT | 实际发布时间 |
| post_url | TEXT | 作品链接 |
| status | TEXT | 状态 |
| views | INTEGER | 播放量 |
| likes | INTEGER | 点赞量 |
| comments | INTEGER | 评论量 |
| favorites | INTEGER | 收藏量 |
| notes | TEXT | 备注 |
| created_at | TEXT | 创建时间 |
| updated_at | TEXT | 更新时间 |

### 2.10 app_settings

| 字段 | 类型 | 说明 |
|---|---|---|
| key | TEXT PK | 设置键 |
| value_json | TEXT | 普通设置 JSON |
| updated_at | TEXT | 更新时间 |

仅保存工作目录、默认模型 ID、封面尺寸和文案偏好等普通配置。不得将密钥写入该表。

## 3. 关系

```text
book_projects
  ├── script_versions
  ├── publish_materials
  ├── assets
  │    └── cover_versions
  ├── ai_tasks
  └── publish_tasks
       └── publish_records

model_providers
  └── ai_tasks

app_settings
  └── 应用级普通设置
```

## 4. 状态值

### 项目状态
- draft
- researching
- scripting
- material_preparing
- ready_to_publish
- published
- archived

项目流程允许跳步，状态不能强迫用户完成所有创作环节。

### 发布任务状态
- draft
- preparing
- ready
- publishing_manual
- published
- failed
- cancelled

第一版只有在用户确认发布成功后才能将任务标记为 published。

### AI 任务状态
- pending
- running
- succeeded
- failed
- cancelled

## 5. 数据一致性与迁移

- 启用 SQLite 外键约束。
- 为常用查询字段建立索引。
- 重要多表操作使用事务。
- 每次结构变更新增迁移脚本。
- 启动时检查并按序执行未执行迁移。
- 迁移失败时明确报错并保留恢复路径。
- 不允许通过删除数据库来“修复”迁移问题。
- 删除项目时使用明确的关联清理策略，不删除外部原始视频。
- 对文件系统操作与数据库写入设计补偿逻辑，避免单侧成功后数据失配。

## 6. 备份与恢复

- 提供手动备份。
- 备份数据库和必要配置。
- 不默认复制原始视频。
- 恢复前先备份当前数据。
- 恢复后校验数据库可读。
- 对路径失效的素材保留记录，并允许用户重新定位。

## 7. M0 技术验证数据库（2026-10-23）

- 文件：`%LOCALAPPDATA%\com.shuyin.desktop\shuyin.sqlite3`。
- 迁移：`src-tauri/migrations/0001_m0_probe.sql`，版本号 `PRAGMA user_version = 1`。
- 表：`m0_probe(id INTEGER PRIMARY KEY CHECK(id = 1), note TEXT NOT NULL, updated_at TEXT NOT NULL)`；文字限制 2000 字符，UTC 时间由 SQLite 生成。
- 仅保存一条可更新的技术验证文字，不保存视频路径、书籍内容或配置密钥。
- 首次建表及版本更新在同一事务中执行；重复启动只读取版本；迁移失败保留原数据库并返回错误；更高版本拒绝打开，不降级或删库。
- M1 应从版本 2 开始添加业务迁移，保留本地已有数据；不要重置版本号或复用 0001。

## 8. M1 实际数据库（2026-10-23）

新增迁移 `0002_projects_assets.sql`，`user_version=2`。按顺序、每版本一个事务执行；版本 1 升级保留 `m0_probe`，结构冲突回滚本次迁移，较高版本拒绝打开。

实际新增表：
- `book_projects`：第 2.1 节字段，加 `theme TEXT NOT NULL`；可选文字用空字符串，目标时长 1–86400 秒；限制状态和非空书名；归档时间随归档/取消归档更新。
- `assets`：仅实现 id、project_id、asset_type、file_path、original_name、file_size、modified_at（Unix 秒）、created_at。`exists_status` 每次读文件系统计算，不把过期状态作为数据库事实。MIME、尺寸、视频时长和 checksum 暂未实现。
- `app_settings(key TEXT PRIMARY KEY, value TEXT NOT NULL)`：目前只有纯文字 `workspace_root`，不提前保存 JSON 配置或密钥。

主键使用 SQLite `randomblob(16)` 生成的 32 位十六进制随机 ID，无新增 UUID 依赖。索引包含项目状态/更新时间、素材项目 ID；同一项目的规范化文件路径唯一。素材外键级联删除只清理数据库关联，不进行文件删除。

M1 当时的备份包含这三张业务表及 M0 验证表，仅接受 v2。M2 后的备份策略见下节。源文件、工作目录内容需要另行保管。恢复前的自动备份位于数据库同级 `backups` 目录；拒绝损坏、未知结构、额外触发器或外键失效的备份。恢复不会搬迁或复制路径指向的素材。

## 9. M2 实际数据库（2026-10-23）

新增 `0003_ai_writing.sql`，`user_version=3`；迁移失败回滚本次版本，保留 v2 数据。新增三表：

- `model_providers`：id、name、provider_type（当前仅 openai-compatible）、base_url、model_name、is_default、is_enabled、credential_ref、created_at、updated_at。部分唯一索引保证至多一个默认模型，默认模型必须启用。credential_ref 为随机系统凭据条目标识，不保存密钥。
- `writing_versions`：id、project_id、kind、version_no、content_json、source_type、model_name、prompt_version、is_confirmed、created_at。kind 为 outline / script / titles / descriptions / hashtags；`(project_id,kind,version_no)` 唯一。大纲和讲稿保存 `{format:"markdown",text}`；候选列表保存 `{format:"list",items}`。与第 2 节早期规划不同，M2 共用一张版本表，不并行创建 script_versions/publish_materials。
- `ai_tasks`：id、project_id、kind、provider_id、model_name、prompt_version、status、input_json、version_id、error_code、error_message、created_at、completed_at。输入仅记录项目/模型/参考版本 ID、类型和数量，不保存密钥、授权头或完整书籍资料；成功关联结果版本，错误为本地固定脱敏信息。

版本只追加，不修改或覆盖历史已确认内容；来源为 ai/manual/mixed。生成成功时，新版本、项目更新时间和任务成功状态在同一事务提交。任务取消后迟到响应不能写版本。应用重启将 pending/running 任务标为 failed/interrupted，不自动重试。

项目删除级联清理创作版本和任务，不删除媒体。模型删除保留历史任务和模型名，provider_id 设 NULL；有未完成任务时拒绝删除模型。系统凭据与 SQLite 不能共享事务：密钥替换先写新随机条目，数据库保存失败时删除新条目，旧条目保留；成功后清理旧条目，清理失败明确返回提示。

备份包含全部上述表但不含密钥。恢复接受 v2/v3：先核验对应原始 schema、完整性与外键，再在内存快照迁移到 v3，自动备份当前数据库后恢复。源备份文件保持原版本；有运行任务时拒绝恢复。备份中的未完成任务恢复为 failed/interrupted；恢复后按当前系统凭据库重新检查密钥，不伪造“已配置”状态。

## 10. 三项结果数据库 v4（2026-10-23）

新增 0004_publishing.sql。事务内同步重建 writing_versions 和 ai_tasks，扩展 kind=publishing，复制旧行后替换，保留任务到旧版本的外键引用、ID、时间戳及旧五类内容。模型表和凭据引用不改动。

publishing 的 content_json 为 `{format:"publishing",title,description,audioPrompt}`。一套结果一个版本，新生成/编辑只追加。标题≤30字，内容≤1000字，提示词≤20000字；AI 原始草案必须有五个完整章节，模板组装后再保存。任务 input_json 仅记录 projectId/providerId，不保存密钥或完整资料。

备份接受 v2/v3/v4，仍在内存中迁移旧备份，原备份不改写；恢复前备份当前数据库。迁移失败回滚，保留旧 schema 和数据，不重置数据库。旧格式只用于保留历史数据，新界面查询 publishing。

## 11. M3 封面版本 v5（2026-10-23）

新增 `0005_covers.sql`，user_version=5。只增加 cover_versions，原七张表及其记录不改写。迁移和版本更新在同一事务内执行。

用户澄清发布封面比例后，新保存/导出仅接受 1440×1080（4:3）与 1080×1440（3:4），本次调整不改变表结构或 schemaVersion。一次导出生成两条独立版本记录。旧尺寸 config_json、exported_path 与文件保留；转换比例只发生在页面恢复编辑后的新草稿，不重写旧行。

实际字段：id、project_id（关联 book_projects，ON DELETE CASCADE）、version_no、config_json、exported_path（配置保存时可空）、created_at。唯一约束 `(project_id, version_no)`；没有更新历史配置的接口。

与第 2.5 节早期草图不同，本轮将模板与背景素材 ID 保存在完整 config_json 中，不重复维护 template_id/background_asset_id 列；不增加导出素材表或 is_selected 状态。JSON 包含 schemaVersion=1，反序列化及保存时校验固定模板、尺寸、文字长度、颜色、字号、对齐与位置。

新版本和项目更新时间事务提交。PNG 只写项目 covers 目录中的唯一新文件；写入或数据库提交失败不会产生成功版本，并尝试清理本次新文件。删除项目只级联删除版本行，保留原图片与所有已导出文件。

备份现在接受 v2–v5，按原 schema 核验并在内存中顺序升级到 v5，不修改源备份。备份包含封面配置与路径，不包含背景/PNG 二进制；外部文件需另行备份。被删除或移动的背景保留引用以便提示恢复，不以删除历史记录修复缺失。

## 12. 可选图片生成 v6（2026-10-24）

`0006_image_generation.sql` 在模型表增加 `model_kind`（text/image，旧行默认 text），将单个默认文本模型索引替换为按用途唯一的默认模型索引；在 ai_tasks 增加可空 `asset_id`，外键指向 assets，移除关联时设 NULL。不新增表，不改写既有封面配置、路径或凭据引用；迁移失败整次回滚并保留 v5。

图片任务使用 `kind=cover`，input_json 仅保存 projectId/providerId/description，不包含 API Key。生成成功时以事务新增 assets、设置任务 asset_id/succeeded 及项目更新时间；实际 PNG 写入项目 covers 子目录中的唯一 `ai-background-{随机ID}.png`。失败清理本次未登记文件；取消后不允许迟到结果写入。背景未自动成为封面历史，用户保存/导出时才追加 cover_versions。

备份接受 v2–v6，继续核验原 schema 后在内存中迁移，不修改源备份；媒体不包含在数据库备份内。v5→v6 数据/凭据引用保留、迁移回滚、旧备份恢复均有 Rust 回归；隔离 Windows 实测 v6 备份恢复后模型、背景引用和封面历史一致，完整性/外键检查通过。

## 13. M4 发布准备 v7（2026-10-24）

新增 `0007_publish_tasks.sql`，事务内增加两张表及索引，旧表及记录不改写：

- `publish_tasks`：id、project_id、title、description、tags、video_asset_id、landscape_cover_id、portrait_cover_id、title_confirmed、covers_confirmed、status、task_directory、created_at、updated_at。标题/内容/标签是发布时的文字快照；不并行新增早期草图中的 publish_materials。视频和封面外键移除时设 NULL，任务仍可重新选素材。
- `publish_records`：publish_task_id（主键兼外键，一项任务一次确认）、published_at、post_url、notes、created_at。仅在明确确认后插入；记录、任务 published 状态和项目状态在同一事务提交。已发布任务固定，另一作品另建任务；不引入运营指标。

当前单一平台为抖音，平台名记录在任务包 manifest 中；未增加可选平台或重复配置。实际任务状态只使用 draft / ready / published。保存修改退回 draft，生成完整且有效的任务包后为 ready；每次查询按素材路径、文件大小/修改时间及任务包文件重查，失效时退回 draft。published 为已确认的历史事实，不因源文件后来移动而撤销。

迁移支持 v1 顺序升级至 v7。备份恢复接受 v2–v7，在内存迁移并提前备份当前库；旧备份文件保持原样。备份不含视频、封面和任务包目录，需另行保管。源视频仅引用；删除项目只级联数据库行，不删除视频、封面或任务包。

## 14. M5 数据验证（2026-10-24）

本轮没有数据库结构变更，schema 保持 v7。新增 v2–v7 逐版本恢复及安全备份回退回归，保留原备份字节、书籍资料、时间戳和缺失素材引用；三项目回归核对文案/封面历史、任务包、任务记录与凭据可用状态。任务包封面内容校验只影响查询时的就绪判定，沿用现有 ready→draft 转换，不新增持久化校验字段。

## 15. 封面素材结果扩展（2026-10-24）

表结构、索引与 `user_version=7` 均不变，无 SQL 迁移。`writing_versions.content_json` 的 publishing 对象新增四个字符串：`coverTitle`（≤30字）、`coverSubtitle`（≤60字）、`coverPromptLandscape` 和 `coverPromptPortrait`（各≤2000字）。新生成必填全部七项，并标注 `publishing-v2`。

旧三项 JSON 读取时由 serde 默认补空字符串，不执行 UPDATE 或批量回填，原始 JSON、ID、版本、时间及引用保持原样。人工编辑允许封面字段为空，因此旧版无需先生成封面才能保存。新保存仍追加版本；不把结果页封面素材重复写入 cover_versions，只有在封面页确认保存/导出才生成封面版本。

备份继续接受 v2–v7，现有快照包含完整 content_json。Rust 回归验证旧三项原始 JSON 在备份恢复后字节不变、新四项在重启后完整保留、超长字段拒绝新增版本，以及三书全流程的新字段随备份保留。升级后的结果由本次新代码读取；旧程序不具备新字段契约，回退需使用升级前的数据库备份。

## 16. 封面 16:9 尺寸扩展（2026-10-25）

保存校验增加 1920×1080，沿用 `cover_versions.config_json` 的宽高字段和 `schemaVersion=1`；数据库仍为 v7，无 SQL 迁移。批量导出追加 4:3、3:4、16:9 三条独立版本，原有行与文件不重写。已有 1920×1080 历史配置可直接恢复编辑，不再转换为 4:3。发布任务的两张封面引用保持原有比例要求。

## 17. 按项调整 v8（2026-10-25）

新增 `0008_generation_preview.sql`，只为 `ai_tasks` 增加可空且校验 JSON 的 `preview_json`，原记录、内容、时间戳及凭据引用不变。迁移与 `user_version=8` 在同一事务内提交；失败保留旧版本。

按项任务仍使用 `kind=publishing`，`input_json` 增加可选 `revision`：`basedOn`、`fields`、`instruction`。不记录原书全文、当前内容快照或密钥。请求使用来源版本，拒绝跨项目/跨类型引用、空字段集合、重复字段及超长要求；Prompt 版本为 `publishing-revision-v1`。

候选生成成功把完整合并结果写入 `preview_json` 并标记 succeeded，不新增 `writing_versions`。采用时事务内追加来源 mixed 的版本、写入实际模型和 Prompt 版本、设置 `version_id` 并清空候选；重复采用返回已关联版本。取消后的迟到响应不能写入候选，采用失败保留候选和旧版本。

备份接受 v2–v8，继续核验原 schema、完整性和外键，在内存中迁移旧备份；源备份不改写。v8 备份包含候选数据，正式 UI 不提供跨重启恢复候选。回退旧程序须使用升级前数据库备份，不对 v8 强制降级。

## 18. 创作草稿、个人样式与发布来源 v9（2026-10-25）

迁移 `0009_creator_workflow.sql` 在同一事务内新增：

- `editor_drafts`：联合主键 `(project_id, kind)`，kind 为 writing/cover，保存 `based_on`、`content_json`、`updated_at`；项目删除级联。来源版本由业务服务核验同项目/类型；允许未完成对象，JSON 上限512 KB。不新增历史版本，显式保存成功清除对应草稿。
- `cover_presets`：id、唯一 name、style_json、created_at，全应用共享；风格只含白名单字段，不携带书籍文字/素材引用。名称重复明确拒绝，应用与删除不改动封面历史。
- `publish_tasks.writing_version_id`：可空外键引用 writing_versions，删除来源时置空；旧任务为空，继续保留原文字快照。新保存校验 publishing 类型与同项目，任务包 manifest 记录来源和实际文本。

`cover_versions.config_json` 的 `schemaVersion=1` 增加 `backgroundX`、`backgroundY`（0–1）和 `backgroundZoom`（1–3）；旧 JSON 不重写，读取缺省0.5/0.5/1。所有比例共用同一草稿裁切值，分别追加导出版本。

v9 当时的 `user_version=9`，支持顺序升级和 v2–v9 数据库备份恢复。兼容迁移不删除旧行；结构冲突回滚整个 v9，保留 v8 数据与版本。回退旧程序需使用升级前数据库备份。

完整备份格式版本为1，与数据库版本独立。目录包含一致性 SQLite 快照、文件清单及 SHA-256，登记的素材/导出封面/当前任务包均复制；可选原始视频。草稿及样式随数据库保留。恢复在内存核验/迁移快照，将文件复制到新目录后重建 `workspace_path`、`file_path`、`exported_path`、`task_directory` 及包内引用，再自动备份当前库并替换。不复制未登记文件、旧的无引用任务包或系统凭据；未包含视频继续引用原路径，缺失时记录保留且任务退回草稿。

回归覆盖 v8→v9 保留/失败回滚、草稿重启、样式白名单、跨书来源拒绝、备份目录搬移、源文件移除后的恢复、遗漏视频、同大小损坏与越界路径拒绝。Windows 真实文件/数据库证据见 `CREATOR-WORKFLOW-VERIFICATION.md`。

## 19. M6 辅助发布（2026-10-26）

M6 当时 schema 为 v9，无迁移或新增持久化状态。`publish_snapshot` 只读取已校验 ready 任务的文字快照、视频引用和包内封面；上传会话与停止标志保留在 Rust 内存，不存网页、账号、Cookie 或浏览器进度。上传/填写完成不改任务状态、不插入发布记录；仅原有人工确认事务写入 published。会话数据不纳入业务备份，旧版数据库备份继续按原校验/恢复路径处理。独立安装版的完整性、外键、备份及升级数据比较见 `M6-VERIFICATION.md`。

M7沿用相同schema：全局任务列表关联book_projects读取书名，逐项使用现有checked校验；不新增队列表或缓存状态。文案来源仍是publish_tasks.writing_version_id，作品链接仍是publish_records.post_url；记录成功仅释放匹配任务的内存快照。备份恢复保留这些已有行，测试与隔离安装模拟记录边界见`M7-VERIFICATION.md`。

## 20. 音频制作与成片回填 v10（2026-10-26）

新增 `0010_audio_workflow.sql`，在同一事务内重建 `assets`、`publish_tasks` 与 `publish_records`：

- `assets.asset_type` 增加 `audio`，原视频、图片、封面、字幕与文档行原样复制。
- `publish_tasks` 增加可空 `audio_asset_id`，同项目音频外键；旧任务该字段为空。其余文字快照、视频、封面、状态、时间戳、任务目录和文案来源不变。
- `publish_records` 原样保留并继续引用重建后的任务。迁移后重新建立 `assets_project` 与 `publish_tasks_project` 索引。

当前 `user_version=10`，数据库备份支持 v2–v10。任务包 manifest 从 schema 1 升至 2，增加可空 `audio` 文件引用；旧任务包仍可读取，但缺少音频的待发布任务不会保持 ready。发布检查使用 `mp4parse 0.17.0` 只读取 MP4 ftyp/moov，不扫描完整 mdat，验证正时长、可读取画面尺寸与音轨，不限制横竖屏比例。音频只登记路径，不复制，不写入发布记录，也不上传抖音。
