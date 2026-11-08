ALTER TABLE automation_runs ADD COLUMN duration_task_id TEXT REFERENCES ai_tasks(id);
ALTER TABLE automation_runs ADD COLUMN content_check_task_id TEXT REFERENCES ai_tasks(id);
UPDATE automation_runs SET content_check_task_id=writing_task_id,writing_task_id=NULL;
UPDATE automation_runs SET input_json=json_set(input_json,'$.workflowVersion',0);
UPDATE automation_runs SET status='paused',run_id=run_id+1,
    error='此任务使用旧版制作顺序，已有成果已保留。请在原音频、视频或发布详情继续处理，或停止旧流程后重新制作。'
WHERE status IN ('running','paused','waiting_user','awaiting_publish');
