CREATE TABLE automation_runs (
    id TEXT PRIMARY KEY,
    request_key TEXT NOT NULL UNIQUE,
    project_id TEXT NOT NULL REFERENCES book_projects(id) ON DELETE CASCADE,
    input_json TEXT NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('running','paused','waiting_user','awaiting_publish','completed','cancelled')),
    stage TEXT NOT NULL,
    run_id INTEGER NOT NULL DEFAULT 1,
    error TEXT NOT NULL DEFAULT '',
    audio_job_id TEXT REFERENCES audio_jobs(id),
    video_job_id TEXT REFERENCES video_jobs(id),
    writing_task_id TEXT REFERENCES ai_tasks(id),
    landscape_task_id TEXT REFERENCES ai_tasks(id),
    portrait_task_id TEXT REFERENCES ai_tasks(id),
    landscape_cover_id TEXT REFERENCES cover_versions(id),
    portrait_cover_id TEXT REFERENCES cover_versions(id),
    publish_task_id TEXT REFERENCES publish_tasks(id),
    publish_input_json TEXT,
    publish_started INTEGER NOT NULL DEFAULT 0 CHECK(publish_started IN (0,1)),
    publish_handed_off INTEGER NOT NULL DEFAULT 0 CHECK(publish_handed_off IN (0,1)),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE UNIQUE INDEX automation_single_active ON automation_runs((1))
WHERE status IN ('running','paused','waiting_user','awaiting_publish');
CREATE INDEX automation_project ON automation_runs(project_id,created_at);
