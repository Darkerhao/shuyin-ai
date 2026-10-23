mod database;

use database::{Database, ProbeSnapshot};
use tauri::Manager;

#[tauri::command]
fn read_probe(database: tauri::State<'_, Database>) -> Result<ProbeSnapshot, String> {
    database.read()
}

#[tauri::command]
fn save_probe(note: String, database: tauri::State<'_, Database>) -> Result<ProbeSnapshot, String> {
    database.save(&note)
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let directory = app.path().app_local_data_dir()?;
            std::fs::create_dir_all(&directory)?;
            // Keep initialization errors available to the UI; never delete a failed database.
            app.manage(Database::open(directory.join("shuyin.sqlite3")));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![read_probe, save_probe])
        .run(tauri::generate_context!())
        .expect("书引 AI 桌面容器启动失败");
}
