DROP INDEX model_calls_recent;
CREATE INDEX model_calls_recent ON model_calls(started_at DESC, id DESC);
