CREATE TABLE assets_v10 (
    id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT NOT NULL REFERENCES book_projects(id) ON DELETE CASCADE,
    asset_type TEXT NOT NULL CHECK(asset_type IN ('video','audio','cover','image','subtitle','document')),
    file_path TEXT NOT NULL,
    original_name TEXT NOT NULL,
    file_size INTEGER NOT NULL,
    modified_at INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    UNIQUE(project_id, file_path)
);
INSERT INTO assets_v10(id,project_id,asset_type,file_path,original_name,file_size,modified_at,created_at)
SELECT id,project_id,asset_type,file_path,original_name,file_size,modified_at,created_at FROM assets;

CREATE TABLE publish_tasks_v10 (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES book_projects(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    description TEXT NOT NULL,
    tags TEXT NOT NULL,
    video_asset_id TEXT REFERENCES assets_v10(id) ON DELETE SET NULL,
    audio_asset_id TEXT REFERENCES assets_v10(id) ON DELETE SET NULL,
    landscape_cover_id TEXT REFERENCES cover_versions(id) ON DELETE SET NULL,
    portrait_cover_id TEXT REFERENCES cover_versions(id) ON DELETE SET NULL,
    title_confirmed INTEGER NOT NULL CHECK(title_confirmed IN (0,1)),
    covers_confirmed INTEGER NOT NULL CHECK(covers_confirmed IN (0,1)),
    status TEXT NOT NULL CHECK(status IN ('draft','ready','published')),
    task_directory TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    writing_version_id TEXT REFERENCES writing_versions(id) ON DELETE SET NULL
);
INSERT INTO publish_tasks_v10(id,project_id,title,description,tags,video_asset_id,audio_asset_id,landscape_cover_id,portrait_cover_id,title_confirmed,covers_confirmed,status,task_directory,created_at,updated_at,writing_version_id)
SELECT id,project_id,title,description,tags,video_asset_id,NULL,landscape_cover_id,portrait_cover_id,title_confirmed,covers_confirmed,status,task_directory,created_at,updated_at,writing_version_id FROM publish_tasks;

CREATE TABLE publish_records_v10 (
    publish_task_id TEXT PRIMARY KEY REFERENCES publish_tasks_v10(id) ON DELETE CASCADE,
    published_at TEXT NOT NULL,
    post_url TEXT NOT NULL,
    notes TEXT NOT NULL,
    created_at TEXT NOT NULL
);
INSERT INTO publish_records_v10(publish_task_id,published_at,post_url,notes,created_at)
SELECT publish_task_id,published_at,post_url,notes,created_at FROM publish_records;

DROP TABLE publish_records;
DROP TABLE publish_tasks;
DROP TABLE assets;
ALTER TABLE assets_v10 RENAME TO assets;
ALTER TABLE publish_tasks_v10 RENAME TO publish_tasks;
ALTER TABLE publish_records_v10 RENAME TO publish_records;

CREATE INDEX assets_project ON assets(project_id);
CREATE INDEX publish_tasks_project ON publish_tasks(project_id, updated_at);
