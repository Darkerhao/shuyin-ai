-- Rebuild the two linked tables together to preserve every existing version/task reference.
CREATE TABLE writing_versions_v4 (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES book_projects(id) ON DELETE CASCADE,
    kind TEXT NOT NULL CHECK(kind IN ('publishing','outline','script','titles','descriptions','hashtags')),
    version_no INTEGER NOT NULL CHECK(version_no > 0),
    content_json TEXT NOT NULL CHECK(json_valid(content_json)),
    source_type TEXT NOT NULL CHECK(source_type IN ('ai','manual','mixed')),
    model_name TEXT NOT NULL,
    prompt_version TEXT NOT NULL,
    is_confirmed INTEGER NOT NULL CHECK(is_confirmed IN (0,1)),
    created_at TEXT NOT NULL,
    UNIQUE(project_id,kind,version_no)
);
INSERT INTO writing_versions_v4 SELECT * FROM writing_versions;
CREATE TABLE ai_tasks_v4 (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES book_projects(id) ON DELETE CASCADE,
    kind TEXT NOT NULL,
    provider_id TEXT REFERENCES model_providers(id) ON DELETE SET NULL,
    model_name TEXT NOT NULL,
    prompt_version TEXT NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('pending','running','succeeded','failed','cancelled')),
    input_json TEXT NOT NULL CHECK(json_valid(input_json)),
    version_id TEXT REFERENCES writing_versions_v4(id) ON DELETE SET NULL,
    error_code TEXT,
    error_message TEXT,
    created_at TEXT NOT NULL,
    completed_at TEXT
);
INSERT INTO ai_tasks_v4 SELECT * FROM ai_tasks;
DROP TABLE ai_tasks;
DROP TABLE writing_versions;
ALTER TABLE writing_versions_v4 RENAME TO writing_versions;
ALTER TABLE ai_tasks_v4 RENAME TO ai_tasks;
CREATE INDEX ai_tasks_project_created ON ai_tasks(project_id,created_at DESC);
