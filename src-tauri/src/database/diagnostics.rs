use super::{
    projects::{new_id, NOW},
    Database,
};
use rusqlite::params;
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelCall {
    id: String,
    project_id: Option<String>,
    title: Option<String>,
    task_id: Option<String>,
    model: String,
    stage: String,
    status: String,
    started_at: String,
    elapsed_ms: Option<i64>,
    first_response_ms: Option<i64>,
    input_tokens: Option<i64>,
    output_tokens: Option<i64>,
    total_tokens: Option<i64>,
    error_code: Option<String>,
}

impl Database {
    pub fn begin_model_call(
        &self,
        task: Option<&str>,
        model: &str,
        stage: &str,
    ) -> Result<String, String> {
        self.with_connection(|c|{
            let id=new_id(c)?;
            c.execute(&format!("INSERT INTO model_calls(id,project_id,task_id,model,stage,status,started_at) VALUES(?1,COALESCE((SELECT project_id FROM ai_tasks WHERE id=?2),(SELECT project_id FROM video_jobs WHERE id=?2)),?2,?3,?4,'running',{NOW})"),params![id,task,model,stage]).map_err(|e|e.to_string())?;
            Ok(id)
        })
    }
    pub fn finish_model_call(
        &self,
        id: &str,
        status: &str,
        elapsed: u64,
        metrics: &crate::ai::diagnostics::Metrics,
        error: Option<&str>,
    ) -> Result<(), String> {
        self.with_connection(|c|{
            c.execute("UPDATE model_calls SET status=?2,elapsed_ms=?3,first_response_ms=?4,input_tokens=?5,output_tokens=?6,total_tokens=?7,error_code=?8 WHERE id=?1 AND status='running'",params![id,status,elapsed as i64,metrics.first_response_ms.map(|n|n as i64),metrics.input_tokens.map(|n|n as i64),metrics.output_tokens.map(|n|n as i64),metrics.total_tokens.map(|n|n as i64),error]).map_err(|e|e.to_string())?;Ok(())
        })
    }
    pub fn recover_model_calls(&self) -> Result<(), String> {
        self.with_connection(|c|{c.execute("UPDATE model_calls SET status='interrupted',error_code='interrupted' WHERE status='running'",[]).map_err(|e|e.to_string())?;Ok(())})
    }
    pub fn list_model_calls(&self) -> Result<Vec<ModelCall>, String> {
        self.with_connection(|c|{
            let mut q=c.prepare("SELECT m.id,m.project_id,p.title,m.task_id,m.model,m.stage,m.status,m.started_at,m.elapsed_ms,m.first_response_ms,m.input_tokens,m.output_tokens,m.total_tokens,m.error_code FROM model_calls m LEFT JOIN book_projects p ON p.id=m.project_id ORDER BY m.started_at DESC,m.id DESC LIMIT 200").map_err(|e|e.to_string())?;
            let result=q.query_map([],|r|Ok(ModelCall{id:r.get(0)?,project_id:r.get(1)?,title:r.get(2)?,task_id:r.get(3)?,model:r.get(4)?,stage:r.get(5)?,status:r.get(6)?,started_at:r.get(7)?,elapsed_ms:r.get(8)?,first_response_ms:r.get(9)?,input_tokens:r.get(10)?,output_tokens:r.get(11)?,total_tokens:r.get(12)?,error_code:r.get(13)?})).map_err(|e|e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string());
            result
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn diagnostics_migrate_from_v16_and_survive_backup_restore_with_interrupted_calls() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("database.sqlite3");
        let c = rusqlite::Connection::open(&path).unwrap();
        for sql in super::super::MIGRATIONS.iter().take(16) {
            c.execute_batch(sql).unwrap();
        }
        c.pragma_update(None, "user_version", 16).unwrap();
        drop(c);
        let db = Database::open(path.clone());
        assert!(db.list_model_calls().unwrap().is_empty());
        let id = db
            .begin_model_call(None, "model", "connection_test")
            .unwrap();
        let backup = db.create_backup(dir.path().to_str().unwrap()).unwrap();
        db.finish_model_call(
            &id,
            "succeeded",
            123,
            &crate::ai::diagnostics::Metrics::default(),
            None,
        )
        .unwrap();
        db.restore_backup(&backup).unwrap();
        let rows = serde_json::to_value(db.list_model_calls().unwrap()).unwrap();
        assert_eq!(rows[0]["status"], "interrupted");
        assert!(rows[0]["elapsedMs"].is_null());
        drop(db);
        let db = Database::open(path);
        assert_eq!(db.list_model_calls().unwrap().len(), 1);
    }
}
