CREATE TABLE audio_jobs (
    id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT NOT NULL REFERENCES book_projects(id) ON DELETE CASCADE,
    writing_version_id TEXT REFERENCES writing_versions(id) ON DELETE SET NULL,
    input_json TEXT NOT NULL CHECK(json_valid(input_json)),
    status TEXT NOT NULL CHECK(status IN (
        'created','waiting_login','preparing','generating','downloading',
        'importing','succeeded','waiting_user','paused','failed','cancelled'
    )),
    stage TEXT NOT NULL,
    run_id INTEGER NOT NULL CHECK(run_id > 0),
    error_code TEXT,
    error_message TEXT,
    notebook_url TEXT,
    remote_audio_ref TEXT,
    output_size INTEGER CHECK(output_size IS NULL OR output_size > 0),
    output_sha256 TEXT CHECK(output_sha256 IS NULL OR length(output_sha256) = 64),
    asset_id TEXT REFERENCES assets(id) ON DELETE SET NULL,
    submitted_at TEXT,
    last_checked_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX audio_jobs_project_updated
ON audio_jobs(project_id, updated_at DESC);

CREATE UNIQUE INDEX one_active_audio_job
ON audio_jobs((1))
WHERE status NOT IN ('succeeded','cancelled');
