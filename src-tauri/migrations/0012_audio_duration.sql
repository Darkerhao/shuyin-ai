ALTER TABLE book_projects ADD COLUMN audio_duration_source TEXT CHECK(audio_duration_source IN ('ai','manual'));
