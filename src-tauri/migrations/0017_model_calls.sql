CREATE TABLE model_calls (
    id TEXT PRIMARY KEY,
    project_id TEXT REFERENCES book_projects(id) ON DELETE SET NULL,
    task_id TEXT,
    model TEXT NOT NULL,
    stage TEXT NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('running','succeeded','failed','cancelled','interrupted')),
    started_at TEXT NOT NULL,
    elapsed_ms INTEGER,
    first_response_ms INTEGER,
    input_tokens INTEGER,
    output_tokens INTEGER,
    total_tokens INTEGER,
    error_code TEXT
);
CREATE INDEX model_calls_recent ON model_calls(started_at DESC);
