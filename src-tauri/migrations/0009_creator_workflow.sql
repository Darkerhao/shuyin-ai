CREATE TABLE editor_drafts (
    project_id TEXT NOT NULL REFERENCES book_projects(id) ON DELETE CASCADE,
    kind TEXT NOT NULL CHECK(kind IN ('writing','cover')),
    based_on TEXT,
    content_json TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    PRIMARY KEY(project_id, kind)
);
CREATE TABLE cover_presets (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    style_json TEXT NOT NULL,
    created_at TEXT NOT NULL
);
ALTER TABLE publish_tasks ADD COLUMN writing_version_id TEXT REFERENCES writing_versions(id) ON DELETE SET NULL;
