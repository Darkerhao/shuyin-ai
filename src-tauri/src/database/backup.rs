use super::{projects::new_id, Database};
use rusqlite::{
    backup::{Backup, StepResult},
    Connection, OpenFlags,
};
use std::path::{Path, PathBuf};

fn copy_database(source: &Connection, destination: &mut Connection) -> Result<(), String> {
    // A single step completes atomically; busy/locked is reported instead of retried forever.
    match Backup::new(source, destination)
        .map_err(|e| e.to_string())?
        .step(-1)
        .map_err(|e| e.to_string())?
    {
        StepResult::Done => Ok(()),
        _ => Err("数据库正忙，请稍后重试。".into()),
    }
}
fn schema(connection: &Connection) -> Result<Vec<(String, String)>, String> {
    let mut statement = connection
        .prepare("SELECT name,sql FROM sqlite_master WHERE name NOT LIKE 'sqlite_%' ORDER BY name")
        .map_err(|e| e.to_string())?;
    let rows = statement
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .map_err(|e| e.to_string())?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| e.to_string())
}
fn validate_backup(connection: &Connection) -> Result<(), String> {
    let version: u32 = connection
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .map_err(|e| e.to_string())?;
    if !(2..=super::MIGRATIONS.len() as u32).contains(&version) {
        return Err("备份版本不受当前应用支持。".into());
    }
    let check: String = connection
        .query_row("PRAGMA integrity_check", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    if check != "ok" {
        return Err("备份数据库完整性校验失败。".into());
    }
    let expected = Connection::open_in_memory().map_err(|e| e.to_string())?;
    for sql in super::MIGRATIONS.iter().take(version as usize) {
        expected.execute_batch(sql).map_err(|e| e.to_string())?;
    }
    if schema(connection)? != schema(&expected)? {
        return Err("备份数据库结构不匹配。".into());
    }
    let violations: bool = connection
        .prepare("PRAGMA foreign_key_check")
        .map_err(|e| e.to_string())?
        .exists([])
        .map_err(|e| e.to_string())?;
    if violations {
        return Err("备份包含失效的数据关联。".into());
    }
    Ok(())
}
fn backup_to(connection: &Connection, directory: &Path) -> Result<PathBuf, String> {
    if !directory.is_dir() {
        return Err("备份目录不存在。".into());
    }
    let name = format!("shuyin-backup-{}.sqlite3", new_id(connection)?);
    let path = directory.join(name);
    // SQLite creates a consistent snapshot, including committed WAL content, without overwriting.
    connection
        .execute("VACUUM INTO ?1", [path.to_string_lossy().as_ref()])
        .map_err(|e| format!("备份失败：{e}"))?;
    Ok(path)
}
pub(super) fn read_snapshot(path: &Path) -> Result<Connection, String> {
    let source = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|e| e.to_string())?;
    let mut snapshot = Connection::open_in_memory().map_err(|e| e.to_string())?;
    copy_database(&source, &mut snapshot)?;
    validate_backup(&snapshot)?;
    let version: u32 = snapshot
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .map_err(|e| e.to_string())?;
    for (index, sql) in super::MIGRATIONS.iter().enumerate().skip(version as usize) {
        let tx = snapshot.transaction().map_err(|e| e.to_string())?;
        tx.execute_batch(sql).map_err(|e| e.to_string())?;
        tx.pragma_update(None, "user_version", (index + 1) as u32)
            .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;
    }
    snapshot.execute("UPDATE ai_tasks SET status='failed',error_code='interrupted',error_message='备份恢复中断了任务，请手动重试。' WHERE status IN ('pending','running')",[]).map_err(|e|e.to_string())?;
    Ok(snapshot)
}
impl Database {
    pub fn create_backup(&self, directory: &str) -> Result<String, String> {
        self.with_connection(|c| {
            backup_to(c, Path::new(directory)).map(|p| p.to_string_lossy().into_owned())
        })
    }
    pub fn restore_backup(&self, path: &str) -> Result<String, String> {
        let path = std::fs::canonicalize(path).map_err(|e| e.to_string())?;
        if path == std::fs::canonicalize(&self.path).map_err(|e| e.to_string())? {
            return Err("不能用当前数据库恢复自身。".into());
        }
        self.replace_snapshot(&read_snapshot(&path)?)
    }
    pub(super) fn replace_snapshot(&self, snapshot: &Connection) -> Result<String, String> {
        self.with_connection(|c| {
            if c.query_row(
                "SELECT EXISTS(SELECT 1 FROM ai_tasks WHERE status IN ('pending','running'))",
                [],
                |r| r.get::<_, bool>(0),
            )
            .map_err(|e| e.to_string())?
            {
                return Err("有生成任务正在执行，请先取消或等待完成后恢复备份。".into());
            }
            let directory = self.path.parent().unwrap().join("backups");
            std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
            let previous = backup_to(c, &directory)?;
            copy_database(snapshot, c)
                .map_err(|e| format!("恢复失败：{e}。恢复前备份：{}", previous.display()))?;
            Ok(previous.to_string_lossy().into_owned())
        })
    }
}
