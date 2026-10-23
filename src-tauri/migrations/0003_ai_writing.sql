CREATE TABLE model_providers (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL CHECK(length(trim(name)) BETWEEN 1 AND 100),
    provider_type TEXT NOT NULL CHECK(provider_type = 'openai-compatible'),
    base_url TEXT NOT NULL,
    model_name TEXT NOT NULL,
    is_default INTEGER NOT NULL CHECK(is_default IN (0,1)),
    is_enabled INTEGER NOT NULL CHECK(is_enabled IN (0,1)),
    credential_ref TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK(is_default = 0 OR is_enabled = 1)
);
CREATE UNIQUE INDEX one_default_text_model ON model_providers(is_default) WHERE is_default = 1;
CREATE TABLE writing_versions (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES book_projects(id) ON DELETE CASCADE,
    kind TEXT NOT NULL CHECK(kind IN ('outline','script','titles','descriptions','hashtags')),
    version_no INTEGER NOT NULL CHECK(version_no > 0),
    content_json TEXT NOT NULL CHECK(json_valid(content_json)),
    source_type TEXT NOT NULL CHECK(source_type IN ('ai','manual','mixed')),
    model_name TEXT NOT NULL,
    prompt_version TEXT NOT NULL,
    is_confirmed INTEGER NOT NULL CHECK(is_confirmed IN (0,1)),
    created_at TEXT NOT NULL,
    UNIQUE(project_id,kind,version_no)
);
CREATE TABLE ai_tasks (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES book_projects(id) ON DELETE CASCADE,
    kind TEXT NOT NULL,
    provider_id TEXT REFERENCES model_providers(id) ON DELETE SET NULL,
    model_name TEXT NOT NULL,
    prompt_version TEXT NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('pending','running','succeeded','failed','cancelled')),
    input_json TEXT NOT NULL CHECK(json_valid(input_json)),
    version_id TEXT REFERENCES writing_versions(id) ON DELETE SET NULL,
    error_code TEXT,
    error_message TEXT,
    created_at TEXT NOT NULL,
    completed_at TEXT
);
CREATE INDEX ai_tasks_project_created ON ai_tasks(project_id,created_at DESC);
