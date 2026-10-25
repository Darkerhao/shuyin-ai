use crate::database::Database;
use rusqlite::OptionalExtension;

impl Database {
    fn auto_update(&self) -> Result<bool, String> {
        self.with_connection(|connection| {
            let value: Option<String> = connection
                .query_row(
                    "SELECT value FROM app_settings WHERE key='auto_update'",
                    [],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|error| error.to_string())?;
            Ok(value.as_deref() != Some("false"))
        })
    }

    fn save_auto_update(&self, enabled: bool) -> Result<(), String> {
        self.with_connection(|connection| {
            connection
                .execute(
                    "INSERT INTO app_settings(key,value) VALUES('auto_update',?1)
                     ON CONFLICT(key) DO UPDATE SET value=excluded.value",
                    [if enabled { "true" } else { "false" }],
                )
                .map_err(|error| error.to_string())?;
            Ok(())
        })
    }
}

#[tauri::command]
pub fn update_settings(database: tauri::State<'_, Database>) -> Result<bool, String> {
    database.auto_update()
}

#[tauri::command]
pub fn set_auto_update(enabled: bool, database: tauri::State<'_, Database>) -> Result<(), String> {
    database.save_auto_update(enabled)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn automatic_update_preference_survives_restart_and_backup_restore() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("test.sqlite3");
        let database = Database::open(path.clone());
        assert!(database.auto_update().unwrap());
        database.save_auto_update(false).unwrap();
        drop(database);
        let database = Database::open(path);
        assert!(!database.auto_update().unwrap());
        let backup = database
            .create_backup(directory.path().to_str().unwrap())
            .unwrap();
        database.save_auto_update(true).unwrap();
        assert!(database.auto_update().unwrap());
        database.restore_backup(&backup).unwrap();
        assert!(!database.auto_update().unwrap());
    }
}
