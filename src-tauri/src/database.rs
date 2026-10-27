pub mod ai;
mod assets;
mod backup;
pub mod covers;
pub mod drafts;
pub mod full_backup;
pub mod image_generation;
mod projects;
pub mod publishing;
pub use assets::Asset;
pub use projects::{Project, ProjectInput, WorkspaceSettings};

use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;
use std::{path::PathBuf, sync::Mutex, time::Duration};

pub struct Database {
    path: PathBuf,
    connection: Mutex<Result<Connection, String>>,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProbeRecord {
    note: String,
    updated_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProbeSnapshot {
    database_path: String,
    record: Option<ProbeRecord>,
}

fn initialize(path: &PathBuf) -> Result<Connection, String> {
    let mut connection = Connection::open(path).map_err(|error| error.to_string())?;
    connection
        .busy_timeout(Duration::from_secs(3))
        .map_err(|error| error.to_string())?;
    connection
        .pragma_update(None, "foreign_keys", true)
        .map_err(|error| error.to_string())?;
    let version: u32 = connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(|error| error.to_string())?;
    if version as usize > MIGRATIONS.len() {
        return Err(format!(
            "数据库版本 {version} 高于当前支持版本 {}，请使用更新的应用。",
            MIGRATIONS.len()
        ));
    }
    for (index, sql) in MIGRATIONS.iter().enumerate().skip(version as usize) {
        let transaction = connection
            .transaction()
            .map_err(|error| error.to_string())?;
        transaction
            .execute_batch(sql)
            .map_err(|error| error.to_string())?;
        transaction
            .pragma_update(None, "user_version", (index + 1) as u32)
            .map_err(|error| error.to_string())?;
        transaction.commit().map_err(|error| error.to_string())?;
    }
    Ok(connection)
}

pub(super) const MIGRATIONS: &[&str] = &[
    include_str!("../migrations/0001_m0_probe.sql"),
    include_str!("../migrations/0002_projects_assets.sql"),
    include_str!("../migrations/0003_ai_writing.sql"),
    include_str!("../migrations/0004_publishing.sql"),
    include_str!("../migrations/0005_covers.sql"),
    include_str!("../migrations/0006_image_generation.sql"),
    include_str!("../migrations/0007_publish_tasks.sql"),
    include_str!("../migrations/0008_generation_preview.sql"),
    include_str!("../migrations/0009_creator_workflow.sql"),
    include_str!("../migrations/0010_audio_workflow.sql"),
    include_str!("../migrations/0011_douyin_collection.sql"),
];

impl Database {
    pub fn open(path: PathBuf) -> Self {
        let connection = Mutex::new(initialize(&path));
        Self { path, connection }
    }

    pub(super) fn with_connection<T>(
        &self,
        action: impl FnOnce(&mut Connection) -> Result<T, String>,
    ) -> Result<T, String> {
        let mut guard = self.connection.lock().map_err(|error| error.to_string())?;
        action(guard.as_mut().map_err(|error| error.clone())?)
    }

    fn snapshot(&self, connection: &Connection) -> Result<ProbeSnapshot, String> {
        let record = connection
            .query_row(
                "SELECT note, updated_at FROM m0_probe WHERE id = 1",
                [],
                |row| {
                    Ok(ProbeRecord {
                        note: row.get(0)?,
                        updated_at: row.get(1)?,
                    })
                },
            )
            .optional()
            .map_err(|error| error.to_string())?;
        Ok(ProbeSnapshot {
            database_path: self.path.to_string_lossy().into_owned(),
            record,
        })
    }

    pub fn read(&self) -> Result<ProbeSnapshot, String> {
        let guard = self.connection.lock().map_err(|error| error.to_string())?;
        let connection = guard.as_ref().map_err(Clone::clone)?;
        self.snapshot(connection)
    }

    pub fn save(&self, note: &str) -> Result<ProbeSnapshot, String> {
        if note.chars().count() > 2000 {
            return Err("验证文字不能超过 2000 个字符。".into());
        }
        let guard = self.connection.lock().map_err(|error| error.to_string())?;
        let connection = guard.as_ref().map_err(Clone::clone)?;
        connection
            .execute(
                "INSERT INTO m0_probe (id, note, updated_at) VALUES (1, ?1, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
                 ON CONFLICT(id) DO UPDATE SET note = excluded.note, updated_at = excluded.updated_at",
                [note],
            )
            .map_err(|error| error.to_string())?;
        self.snapshot(connection)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrates_and_persists_across_reopening() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("中文 persistence.sqlite3");
        let database = Database::open(path.clone());
        assert!(database.read().unwrap().record.is_none());
        let note = "书引 AI：重启仍在 ' \" ; DROP TABLE m0_probe; --";
        let saved = database.save(note).unwrap().record.unwrap();
        assert_eq!(saved.note, note);
        assert!(saved.updated_at.ends_with('Z'));
        drop(database);
        let reopened = Database::open(path.clone());
        assert_eq!(reopened.read().unwrap().record.unwrap(), saved);
        reopened.save("第二次写入").unwrap();
        drop(reopened);
        assert_eq!(
            Database::open(path.clone())
                .read()
                .unwrap()
                .record
                .unwrap()
                .note,
            "第二次写入"
        );
        let connection = Connection::open(path).unwrap();
        assert_eq!(
            connection
                .query_row("SELECT count(*) FROM m0_probe", [], |row| row
                    .get::<_, u32>(0))
                .unwrap(),
            1
        );
        assert_eq!(
            connection
                .pragma_query_value(None, "user_version", |row| row.get::<_, u32>(0))
                .unwrap(),
            MIGRATIONS.len() as u32
        );
    }

    #[test]
    fn failed_write_preserves_saved_content() {
        let directory = tempfile::tempdir().unwrap();
        let database = Database::open(directory.path().join("test.sqlite3"));
        database.save("保留原内容").unwrap();
        assert!(database.save(&"长".repeat(2001)).is_err());
        assert_eq!(database.read().unwrap().record.unwrap().note, "保留原内容");
        {
            let guard = database.connection.lock().unwrap();
            guard
                .as_ref()
                .unwrap()
                .pragma_update(None, "query_only", true)
                .unwrap();
        }
        assert!(database.save("写入失败").is_err());
        assert_eq!(database.read().unwrap().record.unwrap().note, "保留原内容");
    }

    #[test]
    fn migration_failure_keeps_schema_and_version() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("test.sqlite3");
        let connection = Connection::open(&path).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE m0_probe (original TEXT); INSERT INTO m0_probe VALUES ('keep');",
            )
            .unwrap();
        drop(connection);
        assert!(Database::open(path.clone()).read().is_err());
        let connection = Connection::open(path).unwrap();
        assert_eq!(
            connection
                .pragma_query_value(None, "user_version", |row| row.get::<_, u32>(0))
                .unwrap(),
            0
        );
        assert_eq!(
            connection
                .query_row("SELECT original FROM m0_probe", [], |row| row
                    .get::<_, String>(0))
                .unwrap(),
            "keep"
        );
    }

    #[test]
    fn newer_database_is_rejected_without_resetting() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("test.sqlite3");
        let connection = Connection::open(&path).unwrap();
        let future_version = MIGRATIONS.len() as u32 + 1;
        connection
            .pragma_update(None, "user_version", future_version)
            .unwrap();
        drop(connection);
        assert!(Database::open(path.clone())
            .read()
            .unwrap_err()
            .contains(&format!("版本 {future_version}")));
        assert_eq!(
            Connection::open(path)
                .unwrap()
                .pragma_query_value(None, "user_version", |row| row.get::<_, u32>(0))
                .unwrap(),
            future_version
        );
    }

    #[test]
    fn unavailable_path_returns_initialization_error() {
        let directory = tempfile::tempdir().unwrap();
        let database = Database::open(directory.path().join("missing").join("test.sqlite3"));
        assert!(database.read().is_err());
        assert!(database.save("draft").is_err());
    }
}

#[cfg(test)]
mod m1_tests;

#[cfg(test)]
mod m2_tests;

#[cfg(test)]
mod m3_tests;

#[cfg(test)]
mod m4_tests;

#[cfg(test)]
mod workflow_tests;
