CREATE TABLE book_chapters (
    id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT NOT NULL REFERENCES book_projects(id) ON DELETE CASCADE,
    title TEXT NOT NULL CHECK(length(trim(title)) BETWEEN 1 AND 200),
    content TEXT NOT NULL CHECK(length(trim(content)) BETWEEN 1 AND 100000),
    target_duration_seconds INTEGER CHECK(target_duration_seconds IS NULL OR
        (target_duration_seconds BETWEEN 60 AND 86400 AND target_duration_seconds % 60 = 0)),
    audio_duration_source TEXT CHECK(audio_duration_source IS NULL OR audio_duration_source IN ('ai','manual')),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK((target_duration_seconds IS NULL) = (audio_duration_source IS NULL))
);
CREATE INDEX chapters_project_updated ON book_chapters(project_id, updated_at DESC, id);

ALTER TABLE writing_versions ADD COLUMN chapter_id TEXT REFERENCES book_chapters(id);
ALTER TABLE writing_versions ADD COLUMN source_snapshot_json TEXT CHECK(source_snapshot_json IS NULL OR json_valid(source_snapshot_json));
ALTER TABLE ai_tasks ADD COLUMN chapter_id TEXT REFERENCES book_chapters(id);
ALTER TABLE audio_jobs ADD COLUMN chapter_id TEXT REFERENCES book_chapters(id);
ALTER TABLE video_jobs ADD COLUMN chapter_id TEXT REFERENCES book_chapters(id);
ALTER TABLE automation_runs ADD COLUMN chapter_id TEXT REFERENCES book_chapters(id);
ALTER TABLE cover_versions ADD COLUMN chapter_id TEXT REFERENCES book_chapters(id);
ALTER TABLE publish_tasks ADD COLUMN chapter_id TEXT REFERENCES book_chapters(id);
ALTER TABLE assets ADD COLUMN chapter_id TEXT REFERENCES book_chapters(id);

UPDATE automation_runs
SET input_json = json_remove(json_set(input_json,
    '$.selection.source', json_object('kind','asset','assetId',json_extract(input_json,'$.selection.sourceAssetId'))), '$.selection.sourceAssetId')
WHERE json_valid(input_json) AND json_type(input_json,'$.selection.sourceAssetId') = 'text';
