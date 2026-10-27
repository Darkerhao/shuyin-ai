mod ai;
mod ai_commands;
mod commands;
mod cover_commands;
mod credentials;
mod database;
mod douyin;
mod douyin_webview;
mod external;
mod media;
mod note_import;
mod publish_commands;
mod updates;

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
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            let directory = app.path().app_local_data_dir()?;
            std::fs::create_dir_all(&directory)?;
            // Keep initialization errors available to the UI; never delete a failed database.
            let database = Database::open(directory.join("shuyin.sqlite3"));
            // Surface initialization errors through commands as before.
            if database.read().is_ok() {
                database.recover_ai_tasks()?;
            }
            app.manage(database);
            app.manage(credentials::SystemCredentials(
                app.config().identifier.clone(),
            ));
            app.manage(ai::ActiveTasks::default());
            app.manage(douyin::DouyinPublisher::default());
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == "main" && matches!(event, tauri::WindowEvent::Destroyed) {
                if let Some(publishing) = window.app_handle().get_webview_window("douyin-publish") {
                    let _ = publishing.destroy();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            read_probe,
            save_probe,
            updates::update_settings,
            updates::set_auto_update,
            commands::list_projects,
            commands::get_project,
            commands::save_project,
            commands::delete_project,
            commands::list_assets,
            commands::link_asset,
            commands::unlink_asset,
            commands::open_asset,
            commands::workspace_settings,
            commands::set_workspace_root,
            commands::open_project_directory,
            commands::relocate_workspace,
            commands::create_backup,
            commands::restore_backup,
            commands::read_editor_draft,
            commands::save_editor_draft,
            note_import::import_reading_notes,
            commands::create_full_backup,
            commands::inspect_full_backup,
            commands::restore_full_backup,
            publish_commands::list_publish_tasks,
            publish_commands::list_publish_queue,
            publish_commands::save_publish_task,
            publish_commands::prepare_publish_task,
            publish_commands::record_publish_success,
            publish_commands::open_publish_item,
            douyin::open_douyin_window,
            douyin::flow::start_douyin_task,
            douyin::flow::continue_douyin_task,
            douyin::flow::douyin_session,
            douyin::flow::stop_douyin_task,
            cover_commands::list_cover_versions,
            cover_commands::save_cover_version,
            cover_commands::read_cover_background,
            cover_commands::read_cover_export,
            cover_commands::list_cover_presets,
            cover_commands::save_cover_preset,
            cover_commands::delete_cover_preset,
            cover_commands::start_cover_generation,
            cover_commands::execute_cover_generation,
            ai_commands::list_model_providers,
            ai_commands::save_model_provider,
            ai_commands::delete_model_provider,
            ai_commands::test_model_connection,
            ai_commands::list_writing_versions,
            ai_commands::save_writing_version,
            ai_commands::list_ai_tasks,
            ai_commands::start_generation,
            ai_commands::execute_generation,
            ai_commands::apply_generation,
            ai_commands::cancel_generation,
            ai_commands::open_audio_tool,
            ai_commands::set_audio_duration
        ])
        .run(tauri::generate_context!())
        .expect("书引 AI 桌面容器启动失败");
}
