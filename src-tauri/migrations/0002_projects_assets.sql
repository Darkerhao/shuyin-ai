CREATE TABLE book_projects (
    id TEXT PRIMARY KEY NOT NULL,
    title TEXT NOT NULL CHECK(length(trim(title)) BETWEEN 1 AND 200),
    author TEXT NOT NULL DEFAULT '',
    publisher TEXT NOT NULL DEFAULT '',
    isbn TEXT NOT NULL DEFAULT '',
    description TEXT NOT NULL DEFAULT '',
    source_notes TEXT NOT NULL DEFAULT '',
    theme TEXT NOT NULL DEFAULT '',
    target_audience TEXT NOT NULL DEFAULT '',
    content_style TEXT NOT NULL DEFAULT '',
    target_duration_seconds INTEGER NOT NULL CHECK(target_duration_seconds BETWEEN 1 AND 86400),
    status TEXT NOT NULL CHECK(status IN ('draft','researching','scripting','material_preparing','ready_to_publish','published','archived')),
    workspace_path TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    archived_at TEXT
);
CREATE INDEX projects_status_updated ON book_projects(status, updated_at DESC);
CREATE TABLE assets (
    id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT NOT NULL REFERENCES book_projects(id) ON DELETE CASCADE,
    asset_type TEXT NOT NULL CHECK(asset_type IN ('video','cover','image','subtitle','document')),
    file_path TEXT NOT NULL,
    original_name TEXT NOT NULL,
    file_size INTEGER NOT NULL,
    modified_at INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    UNIQUE(project_id, file_path)
);
CREATE INDEX assets_project ON assets(project_id);
CREATE TABLE app_settings (key TEXT PRIMARY KEY NOT NULL, value TEXT NOT NULL);
