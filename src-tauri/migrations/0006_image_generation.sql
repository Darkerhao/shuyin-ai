ALTER TABLE model_providers ADD COLUMN model_kind TEXT NOT NULL DEFAULT 'text' CHECK(model_kind IN ('text','image'));
DROP INDEX one_default_text_model;
CREATE UNIQUE INDEX one_default_model_per_kind ON model_providers(model_kind) WHERE is_default = 1;
ALTER TABLE ai_tasks ADD COLUMN asset_id TEXT REFERENCES assets(id) ON DELETE SET NULL;
