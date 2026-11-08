ALTER TABLE automation_runs ADD COLUMN widescreen_task_id TEXT REFERENCES ai_tasks(id);
ALTER TABLE automation_runs ADD COLUMN widescreen_cover_id TEXT REFERENCES cover_versions(id);
ALTER TABLE automation_runs ADD COLUMN render_plan_sha256 TEXT;
UPDATE automation_runs SET status='paused',run_id=run_id+1,
    error='本流程使用旧版制作顺序和双封面授权，成果已保留。请在原任务详情继续，或停止后以三尺寸配置重新制作。'
WHERE status IN ('running','paused','waiting_user','awaiting_publish');
