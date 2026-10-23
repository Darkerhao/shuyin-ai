# 书引 AI 数据库设计文档

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
