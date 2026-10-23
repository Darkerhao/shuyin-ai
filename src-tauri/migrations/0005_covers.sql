CREATE TABLE cover_versions (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES book_projects(id) ON DELETE CASCADE,
    version_no INTEGER NOT NULL CHECK(version_no > 0),
    config_json TEXT NOT NULL,
    exported_path TEXT,
    created_at TEXT NOT NULL,
    UNIQUE(project_id, version_no)
);
