CREATE TABLE publish_tasks (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES book_projects(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    description TEXT NOT NULL,
    tags TEXT NOT NULL,
    video_asset_id TEXT REFERENCES assets(id) ON DELETE SET NULL,
    landscape_cover_id TEXT REFERENCES cover_versions(id) ON DELETE SET NULL,
    portrait_cover_id TEXT REFERENCES cover_versions(id) ON DELETE SET NULL,
    title_confirmed INTEGER NOT NULL CHECK(title_confirmed IN (0,1)),
    covers_confirmed INTEGER NOT NULL CHECK(covers_confirmed IN (0,1)),
    status TEXT NOT NULL CHECK(status IN ('draft','ready','published')),
    task_directory TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE INDEX publish_tasks_project ON publish_tasks(project_id, updated_at);
CREATE TABLE publish_records (
    publish_task_id TEXT PRIMARY KEY REFERENCES publish_tasks(id) ON DELETE CASCADE,
    published_at TEXT NOT NULL,
    post_url TEXT NOT NULL,
    notes TEXT NOT NULL,
    created_at TEXT NOT NULL
);
