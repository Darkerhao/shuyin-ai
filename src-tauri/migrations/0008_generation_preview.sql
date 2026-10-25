ALTER TABLE ai_tasks ADD COLUMN preview_json TEXT
    CHECK(preview_json IS NULL OR json_valid(preview_json));
