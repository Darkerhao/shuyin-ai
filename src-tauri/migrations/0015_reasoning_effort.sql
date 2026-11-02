ALTER TABLE model_providers ADD COLUMN reasoning_effort TEXT
    CHECK(reasoning_effort IS NULL OR (model_kind = 'text' AND reasoning_effort IN ('none', 'minimal', 'low', 'medium', 'high', 'xhigh', 'max')));
