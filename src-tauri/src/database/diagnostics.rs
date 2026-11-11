use super::{
    projects::{new_id, NOW},
    Database,
};
use rusqlite::{params, Connection, OptionalExtension};
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelCallPage {
    pub rows: Vec<ModelCall>,
    total: u64,
    page: u64,
    page_size: u32,
    running_count: u64,
    retention_days: u32,
}

fn validate_retention_days(days: u32) -> Result<(), String> {
    if ![0, 7, 30, 90].contains(&days) {
        return Err("调用记录仅支持全部保留或保留最近 7、30、90 天。".into());
    }
    Ok(())
}

fn retention_days(connection: &Connection) -> Result<u32, String> {
    let value: Option<String> = connection
        .query_row(
            "SELECT value FROM app_settings WHERE key='model_call_retention_days'",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    let days = value
        .map(|value| serde_json::from_str::<u32>(&value).map_err(|error| error.to_string()))
        .transpose()?
        .unwrap_or(0);
    validate_retention_days(days)?;
    Ok(days)
}

fn clean_expired(connection: &Connection, days: u32) -> Result<usize, String> {
    if days == 0 {
        return Ok(0);
    }
    connection
        .execute(
            "DELETE FROM model_calls WHERE status <> 'running'
             AND started_at < strftime('%Y-%m-%dT%H:%M:%fZ', 'now', ?1)",
            [format!("-{days} days")],
        )
        .map_err(|error| error.to_string())
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
    pub fn list_model_calls(&self, page: u32, page_size: u32) -> Result<ModelCallPage, String> {
        if page == 0 || !(1..=100).contains(&page_size) {
            return Err("页码必须大于 0，每页条数必须为 1 至 100。".into());
        }
        self.with_connection(|connection| {
            let (total, running_count): (u64, u64) = connection
                .query_row(
                    "SELECT COUNT(*), COALESCE(SUM(status='running'), 0) FROM model_calls",
                    [],
                    |row| Ok((row.get::<_, i64>(0)? as u64, row.get::<_, i64>(1)? as u64)),
                )
                .map_err(|error| error.to_string())?;
            let page = u64::from(page).min(total.div_ceil(u64::from(page_size)).max(1));
            let mut query = connection.prepare(
                "SELECT m.id,m.project_id,p.title,m.task_id,m.model,m.stage,m.status,m.started_at,
                        m.elapsed_ms,m.first_response_ms,m.input_tokens,m.output_tokens,m.total_tokens,m.error_code
                 FROM model_calls m LEFT JOIN book_projects p ON p.id=m.project_id
                 ORDER BY m.started_at DESC,m.id DESC LIMIT ?1 OFFSET ?2",
            ).map_err(|error| error.to_string())?;
            let rows = query
                .query_map(params![page_size, (page - 1) as i64 * i64::from(page_size)], |row| {
                    Ok(ModelCall {
                        id: row.get(0)?, project_id: row.get(1)?, title: row.get(2)?,
                        task_id: row.get(3)?, model: row.get(4)?, stage: row.get(5)?,
                        status: row.get(6)?, started_at: row.get(7)?, elapsed_ms: row.get(8)?,
                        first_response_ms: row.get(9)?, input_tokens: row.get(10)?,
                        output_tokens: row.get(11)?, total_tokens: row.get(12)?, error_code: row.get(13)?,
                    })
                })
                .map_err(|error| error.to_string())?
                .collect::<rusqlite::Result<Vec<_>>>()
                .map_err(|error| error.to_string())?;
            Ok(ModelCallPage {
                rows, total, page, page_size, running_count,
                retention_days: retention_days(connection)?,
            })
        })
    }

    pub fn clear_model_calls(&self, confirmed: bool) -> Result<usize, String> {
        if !confirmed {
            return Err("请先确认清除已结束的模型调用记录。".into());
        }
        self.with_connection(|connection| {
            connection
                .execute("DELETE FROM model_calls WHERE status <> 'running'", [])
                .map_err(|error| error.to_string())
        })
    }

    pub fn set_model_call_retention(&self, days: u32, confirmed: bool) -> Result<usize, String> {
        validate_retention_days(days)?;
        if days > 0 && !confirmed {
            return Err("请先确认按保留天数清理模型调用记录。".into());
        }
        self.with_connection(|connection| {
            let transaction = connection
                .transaction()
                .map_err(|error| error.to_string())?;
            transaction
                .execute(
                    "INSERT INTO app_settings(key,value) VALUES('model_call_retention_days',?1)
                 ON CONFLICT(key) DO UPDATE SET value=excluded.value",
                    [days.to_string()],
                )
                .map_err(|error| error.to_string())?;
            let deleted = clean_expired(&transaction, days)?;
            transaction.commit().map_err(|error| error.to_string())?;
            Ok(deleted)
        })
    }

    pub fn cleanup_expired_model_calls(&self) -> Result<usize, String> {
        self.with_connection(|connection| clean_expired(connection, retention_days(connection)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (tempfile::TempDir, Database) {
        let directory = tempfile::tempdir().unwrap();
        let database = Database::open(directory.path().join("database.sqlite3"));
        (directory, database)
    }

    fn insert_call(database: &Database, id: &str, status: &str, age: &str) {
        database.with_connection(|connection| {
            connection.execute(
                "INSERT INTO model_calls(id,model,stage,status,started_at)
                 VALUES(?1,'test-model','connection_test',?2,strftime('%Y-%m-%dT%H:%M:%fZ','now',?3))",
                params![id, status, age],
            ).map_err(|error| error.to_string())?;
            Ok(())
        }).unwrap();
    }

    #[test]
    fn diagnostics_pages_include_all_history_with_stable_order_and_global_running_count() {
        let (directory, database) = fixture();
        database
            .with_connection(|connection| {
                connection.execute_batch(
                "WITH RECURSIVE calls(n) AS (SELECT 1 UNION ALL SELECT n+1 FROM calls WHERE n<245)
                 INSERT INTO model_calls(id,model,stage,status,started_at)
                 SELECT printf('call-%03d',n),'test-model','writing',
                        CASE WHEN n=1 THEN 'running' ELSE 'succeeded' END,'2026-01-01T00:00:00.000Z'
                 FROM calls;",
            ).map_err(|error| error.to_string())
            })
            .unwrap();
        let first = database.list_model_calls(1, 20).unwrap();
        assert_eq!(
            (first.total, first.running_count, first.retention_days),
            (245, 1, 0)
        );
        assert_eq!(first.rows.len(), 20);
        assert_eq!(first.rows[0].id, "call-245");
        let mut ids = Vec::new();
        for page in 1..=13 {
            let result = database.list_model_calls(page, 20).unwrap();
            assert_eq!((result.page, result.page_size), (u64::from(page), 20));
            ids.extend(result.rows.into_iter().map(|row| row.id));
        }
        assert_eq!(ids.len(), 245);
        assert_eq!(
            ids.iter().collect::<std::collections::HashSet<_>>().len(),
            245
        );
        assert_eq!(ids.last().unwrap(), "call-001");
        let last = database.list_model_calls(u32::MAX, 20).unwrap();
        assert_eq!((last.page, last.rows.len()), (13, 5));
        assert_eq!(database.list_model_calls(1, 100).unwrap().rows.len(), 100);
        for (page, size) in [(0, 20), (1, 0), (1, 101), (1, u32::MAX)] {
            assert!(database.list_model_calls(page, size).is_err());
        }
        assert_eq!(database.cleanup_expired_model_calls().unwrap(), 0);
        drop(database);
        let database = Database::open(directory.path().join("database.sqlite3"));
        assert_eq!(database.list_model_calls(1, 20).unwrap().total, 245);
        assert_eq!(database.clear_model_calls(true).unwrap(), 244);
        let reduced = database.list_model_calls(13, 20).unwrap();
        assert_eq!((reduced.page, reduced.total), (1, 1));
        database.recover_model_calls().unwrap();
        database.clear_model_calls(true).unwrap();
        let empty = database.list_model_calls(13, 20).unwrap();
        assert_eq!((empty.page, empty.total, empty.running_count), (1, 0, 0));
        assert!(empty.rows.is_empty());
    }

    #[test]
    fn diagnostics_clear_requires_confirmation_and_preserves_running_calls_and_book_data() {
        let (directory, database) = fixture();
        let project = super::super::m4_tests::project(&database);
        let source = directory.path().join("原书.txt");
        std::fs::write(&source, "保留原始书籍资料").unwrap();
        database
            .link_asset(&project.id, source.to_str().unwrap(), None)
            .unwrap();
        let assets_before =
            serde_json::to_value(database.list_assets(&project.id).unwrap()).unwrap();
        let writing = database.save_writing(serde_json::from_value(serde_json::json!({
            "projectId": project.id, "kind": "publishing", "basedOn": null, "isConfirmed": false,
            "content": {"format":"publishing", "title":"保留文案", "description":"保留内容", "audioPrompt":"原声指导"}
        })).unwrap()).unwrap();
        let writing_before = serde_json::to_value(&writing).unwrap();
        let project_before =
            serde_json::to_value(database.get_project(&project.id).unwrap()).unwrap();
        for status in ["succeeded", "failed", "cancelled", "interrupted", "running"] {
            insert_call(&database, status, status, "-100 days");
        }
        assert!(database.clear_model_calls(false).is_err());
        assert_eq!(database.list_model_calls(1, 20).unwrap().total, 5);
        assert_eq!(database.clear_model_calls(true).unwrap(), 4);
        let remaining = database.list_model_calls(1, 20).unwrap();
        assert_eq!(remaining.rows[0].id, "running");
        assert_eq!(remaining.running_count, 1);
        database
            .finish_model_call(
                "running",
                "succeeded",
                123,
                &crate::ai::diagnostics::Metrics {
                    input_tokens: Some(12),
                    output_tokens: Some(7),
                    total_tokens: Some(19),
                    ..Default::default()
                },
                None,
            )
            .unwrap();
        let finished = database.list_model_calls(1, 20).unwrap();
        assert_eq!(finished.rows[0].total_tokens, Some(19));
        assert_eq!(finished.rows[0].elapsed_ms, Some(123));
        assert_eq!(
            serde_json::to_value(database.get_project(&project.id).unwrap()).unwrap(),
            project_before
        );
        assert_eq!(
            serde_json::to_value(database.list_assets(&project.id).unwrap()).unwrap(),
            assets_before
        );
        assert_eq!(
            serde_json::to_value(&database.list_versions(&project.id, "publishing").unwrap()[0])
                .unwrap(),
            writing_before
        );
        assert_eq!(std::fs::read_to_string(source).unwrap(), "保留原始书籍资料");
    }

    #[test]
    fn diagnostics_retention_cleans_only_expired_finished_calls_and_can_be_disabled() {
        for days in [7, 30, 90] {
            let (_directory, database) = fixture();
            insert_call(&database, "old", "failed", &format!("-{} days", days + 1));
            insert_call(
                &database,
                "recent",
                "succeeded",
                &format!("-{} days", days - 1),
            );
            insert_call(&database, "active", "running", "-365 days");
            assert_eq!(database.cleanup_expired_model_calls().unwrap(), 0);
            assert!(database.set_model_call_retention(days, false).is_err());
            assert_eq!(database.list_model_calls(1, 20).unwrap().total, 3);
            assert!(database.set_model_call_retention(1, true).is_err());
            assert_eq!(database.set_model_call_retention(days, true).unwrap(), 1);
            let result = database.list_model_calls(1, 20).unwrap();
            assert_eq!(
                (result.total, result.retention_days, result.running_count),
                (2, days, 1)
            );
            insert_call(&database, "later-expired", "cancelled", "-365 days");
            assert_eq!(database.cleanup_expired_model_calls().unwrap(), 1);
            database
                .finish_model_call("active", "succeeded", 100, &Default::default(), None)
                .unwrap();
            assert_eq!(database.cleanup_expired_model_calls().unwrap(), 1);
            assert_eq!(
                database.list_model_calls(1, 20).unwrap().rows[0].id,
                "recent"
            );
            assert_eq!(database.set_model_call_retention(0, false).unwrap(), 0);
            insert_call(&database, "keep-history", "interrupted", "-365 days");
            assert_eq!(database.cleanup_expired_model_calls().unwrap(), 0);
            assert_eq!(database.list_model_calls(1, 20).unwrap().total, 2);
        }
    }

    #[test]
    fn diagnostics_retention_survives_restart_and_backup_and_cleans_recovered_calls() {
        let (directory, database) = fixture();
        database.set_model_call_retention(30, true).unwrap();
        insert_call(&database, "old-active", "running", "-31 days");
        insert_call(&database, "recent", "succeeded", "-1 day");
        drop(database);
        let database = Database::open(directory.path().join("database.sqlite3"));
        database.recover_model_calls().unwrap();
        assert_eq!(database.cleanup_expired_model_calls().unwrap(), 1);
        let backup = database
            .create_backup(directory.path().to_str().unwrap())
            .unwrap();
        database.set_model_call_retention(0, false).unwrap();
        database.clear_model_calls(true).unwrap();
        database.restore_backup(&backup).unwrap();
        let restored = database.list_model_calls(1, 20).unwrap();
        assert_eq!((restored.total, restored.retention_days), (1, 30));
        assert_eq!(restored.rows[0].id, "recent");
    }

    #[test]
    fn diagnostics_retention_rolls_back_setting_and_records_when_cleanup_fails() {
        let (_directory, database) = fixture();
        insert_call(&database, "old", "succeeded", "-100 days");
        database
            .with_connection(|connection| {
                connection
                    .execute_batch(
                        "CREATE TRIGGER reject_call_delete BEFORE DELETE ON model_calls
                 BEGIN SELECT RAISE(ABORT, 'cleanup failed'); END;",
                    )
                    .map_err(|error| error.to_string())
            })
            .unwrap();
        assert!(database
            .set_model_call_retention(30, true)
            .unwrap_err()
            .contains("cleanup failed"));
        let unchanged = database.list_model_calls(1, 20).unwrap();
        assert_eq!((unchanged.total, unchanged.retention_days), (1, 0));
    }

    #[test]
    fn diagnostics_v24_migration_preserves_history_and_indexes_both_sort_columns() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("database.sqlite3");
        let connection = Connection::open(&path).unwrap();
        for sql in super::super::MIGRATIONS.iter().take(23) {
            connection.execute_batch(sql).unwrap();
        }
        connection.pragma_update(None, "user_version", 23).unwrap();
        connection.execute("INSERT INTO model_calls(id,model,stage,status,started_at) VALUES('old-call','model','writing','succeeded','2026-01-01T00:00:00.000Z')", []).unwrap();
        drop(connection);
        let database = Database::open(path);
        assert_eq!(
            database.list_model_calls(1, 20).unwrap().rows[0].id,
            "old-call"
        );
        database.with_connection(|connection| {
            let columns: Vec<String> = connection.prepare("PRAGMA index_info(model_calls_recent)").unwrap()
                .query_map([], |row| row.get(2)).unwrap().collect::<rusqlite::Result<_>>().unwrap();
            assert_eq!(columns, vec!["started_at", "id"]);
            let plan: Vec<String> = connection.prepare("EXPLAIN QUERY PLAN SELECT id FROM model_calls ORDER BY started_at DESC,id DESC LIMIT 20 OFFSET 200").unwrap()
                .query_map([], |row| row.get(3)).unwrap().collect::<rusqlite::Result<_>>().unwrap();
            assert!(plan.iter().any(|step| step.contains("model_calls_recent")));
            assert!(plan.iter().all(|step| !step.contains("TEMP B-TREE")));
            Ok(())
        }).unwrap();
    }

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
        assert!(db.list_model_calls(1, 20).unwrap().rows.is_empty());
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
        let rows = serde_json::to_value(db.list_model_calls(1, 20).unwrap().rows).unwrap();
        assert_eq!(rows[0]["status"], "interrupted");
        assert!(rows[0]["elapsedMs"].is_null());
        drop(db);
        let db = Database::open(path);
        assert_eq!(db.list_model_calls(1, 20).unwrap().total, 1);
    }
}
