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

> 下列业务表属于后续里程碑的设计，M0 尚未创建。M0 的实际数据库仅包含技术验证表 `m0_probe`，见第 7 节。

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
