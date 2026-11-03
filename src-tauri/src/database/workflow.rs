use super::Database;
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResumeTask {
    pub project_id: String,
    pub title: String,
    pub kind: String,
    pub task_id: String,
    pub status: String,
    pub stage: String,
    pub updated_at: String,
    pub running: bool,
}

#[derive(Serialize)]
pub struct WorkflowSnapshot {
    pub resume: Vec<ResumeTask>,
    pub running: Vec<ResumeTask>,
}

impl Database {
    pub fn workflow_snapshot(&self) -> Result<WorkflowSnapshot, String> {
        self.with_connection(|c| {
            // Recent activity determines what the creator was doing; active work always wins.
            let mut q = c.prepare("SELECT p.id,p.title,t.kind,t.id,t.status,t.stage,t.updated,t.running FROM book_projects p JOIN (
                SELECT project_id,'video' kind,id,status,stage,MAX(updated_at,COALESCE(json_extract(editor_draft_json,'$.updatedAt'),updated_at)) updated,status='running' running FROM video_jobs
                UNION ALL SELECT project_id,'audio',id,status,stage,updated_at,status IN ('created','preparing','generating','downloading','importing') FROM audio_jobs
                UNION ALL SELECT project_id,'publishing',id,status,'',updated_at,0 FROM publish_tasks
                UNION ALL SELECT project_id,CASE WHEN kind='video_scene_revision' OR kind='video_illustration' THEN 'video' WHEN kind='cover' THEN 'covers' ELSE 'writing' END,id,status,kind,created_at,1 FROM ai_tasks WHERE status IN ('pending','running')
                UNION ALL SELECT project_id,CASE kind WHEN 'cover' THEN 'covers' ELSE 'writing' END,'','draft','',updated_at,0 FROM editor_drafts
                UNION ALL SELECT project_id,'writing',id,'saved','',created_at,0 FROM writing_versions
                UNION ALL SELECT project_id,'covers',id,'saved','',created_at,0 FROM cover_versions
            ) t ON t.project_id=p.id WHERE p.status!='archived' ORDER BY t.running DESC,t.updated DESC,t.id DESC").map_err(|e|e.to_string())?;
            let rows = q.query_map([], |r| Ok(ResumeTask { project_id:r.get(0)?,title:r.get(1)?,kind:r.get(2)?,task_id:r.get(3)?,status:r.get(4)?,stage:r.get(5)?,updated_at:r.get(6)?,running:r.get(7)? })).map_err(|e|e.to_string())?;
            let mut resume = BTreeMap::new();
            let mut running = Vec::new();
            for row in rows {
                let row = row.map_err(|e|e.to_string())?;
                if !resume.contains_key(&row.project_id) {
                    resume.insert(row.project_id.clone(), row.clone());
                }
                if row.running { running.push(row); }
            }
            Ok(WorkflowSnapshot { resume:resume.into_values().collect(),running })
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::video::VideoWork;
    #[test]
    fn workflow_uses_real_video_task_and_prefers_running_over_newer_drafts() {
        let (_dir, db, input) = crate::database::video::tests::fixture();
        let project = input.project_id.clone();
        let job = db.create_video_job(input).unwrap();
        db.with_connection(|c|{c.execute("INSERT INTO editor_drafts(project_id,kind,content_json,updated_at) VALUES(?1,'writing','{}','9999')",[&project]).unwrap();Ok(())}).unwrap();
        assert_eq!(db.workflow_snapshot().unwrap().resume[0].kind, "writing");
        db.claim_video_job(&job.id, VideoWork::Prepare, None)
            .unwrap();
        let snapshot = db.workflow_snapshot().unwrap();
        assert_eq!(snapshot.running.len(), 1);
        assert_eq!(snapshot.resume[0].task_id, job.id);
        assert_eq!(snapshot.resume[0].stage, "preparing");
        db.end_video_job(&job.id, 1, "awaiting_confirmation", "")
            .unwrap();
        assert!(db.workflow_snapshot().unwrap().running.is_empty());
    }
}
