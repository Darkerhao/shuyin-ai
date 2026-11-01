CREATE TABLE video_jobs (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES book_projects(id) ON DELETE CASCADE,
    audio_asset_id TEXT REFERENCES assets(id) ON DELETE SET NULL,
    source_subtitle_id TEXT REFERENCES assets(id) ON DELETE SET NULL,
    provider_id TEXT REFERENCES model_providers(id) ON DELETE SET NULL,
    status TEXT NOT NULL CHECK(status IN ('draft','running','awaiting_confirmation','succeeded','failed','cancelled')),
    stage TEXT NOT NULL,
    run_id INTEGER NOT NULL DEFAULT 0,
    plan_json TEXT NOT NULL,
    error TEXT NOT NULL DEFAULT '',
    subtitle_asset_id TEXT REFERENCES assets(id) ON DELETE SET NULL,
    video_asset_id TEXT REFERENCES assets(id) ON DELETE SET NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE UNIQUE INDEX video_single_running ON video_jobs((1)) WHERE status='running';
CREATE INDEX video_project_history ON video_jobs(project_id,created_at);
