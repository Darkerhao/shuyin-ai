ALTER TABLE publish_tasks ADD COLUMN douyin_collection TEXT
    CHECK (douyin_collection IS NULL OR json_valid(douyin_collection));
