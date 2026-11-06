ALTER TABLE publish_tasks ADD COLUMN allow_download INTEGER NOT NULL DEFAULT 1
    CHECK (allow_download IN (0,1));
ALTER TABLE publish_tasks ADD COLUMN scheduled_at TEXT;
