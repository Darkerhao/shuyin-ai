mod ai;
mod ai_commands;
mod audio;
mod audio_commands;
mod automation;
mod commands;
mod completion;
mod cover_commands;
mod credentials;
mod database;
mod douyin;
mod media;
mod note_import;
#[cfg(target_os = "windows")]
mod paths;
mod publish_commands;
mod updates;
mod video;
mod video_commands;
mod video_process;
mod webview_host;

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
                database.recover_model_calls()?;
                database.recover_audio_jobs()?;
                database.recover_video_jobs()?;
                database.recover_automation()?;
            }
            app.manage(database);
            let cleanup_app = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let mut interval = tokio::time::interval(std::time::Duration::from_secs(3600));
                interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
                loop {
                    // The first tick runs at startup; later ticks also cover long-running sessions.
                    interval.tick().await;
                    let handle = cleanup_app.clone();
                    let result = tauri::async_runtime::spawn_blocking(move || {
                        handle.state::<Database>().cleanup_expired_model_calls()
                    })
                    .await
                    .map_err(|error| error.to_string())
                    .and_then(|result| result);
                    if let Err(error) = result {
                        eprintln!("模型调用记录自动清理失败：{error}");
                    }
                }
            });
            app.manage(video_commands::VideoRuntime::default());
            app.manage(audio::AudioAutomation::initialize(app.handle()));
            app.state::<audio::AudioAutomation>()
                .start(app.handle().clone());
            app.manage(credentials::SystemCredentials(
                app.config().identifier.clone(),
            ));
            app.manage(ai::ActiveTasks::default());
            app.manage(douyin::DouyinPublisher::default());
            app.manage(automation::AutomationRuntime::default());
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == "main" && matches!(event, tauri::WindowEvent::Destroyed) {
                window
                    .app_handle()
                    .state::<video_commands::VideoRuntime>()
                    .stop_all();
                if let Some(audio) = window.app_handle().get_webview_window("audio-notebook") {
                    let _ = audio.destroy();
                }
                if let Some(publishing) = window.app_handle().get_webview_window("douyin-publish") {
                    let _ = publishing.destroy();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            read_probe,
            automation::automation_snapshot,
            automation::start_automation,
            automation::control_automation,
            automation::automation_cover,
            automation::complete_automation_cover,
            automation::fail_automation_cover,
            douyin::flow::prepare_automation_account,
            douyin::flow::confirm_automation_account,
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
            commands::workflow_snapshot,
            commands::project_progress,
            commands::interface_preferences,
            commands::set_sidebar_prompt,
            commands::export_video_asset,
            commands::video_asset_metadata,
            commands::last_backup,
            commands::storage_usage,
            commands::preview_cache_cleanup,
            commands::clean_video_cache,
            commands::list_model_calls,
            commands::clear_model_calls,
            commands::set_model_call_retention,
            video_commands::read_video_draft,
            video_commands::save_video_draft,
            video_commands::original_video_cues,
            video_commands::check_video_environment,
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
            ai_commands::set_audio_duration,
            audio_commands::audio_job_snapshot,
            audio_commands::start_audio_job,
            audio_commands::control_audio_job,
            video_commands::read_video_image,
            video_commands::import_video_image,
            video_commands::start_video_illustration,
            video_commands::start_video_scene_revision,
            video_commands::execute_video_scene_revision,
            video_commands::execute_video_illustration,
            video_commands::video_settings,
            video_commands::save_video_settings,
            video_commands::list_video_jobs,
            video_commands::start_video_job,
            video_commands::continue_video_job,
            video_commands::cancel_video_job,
            video_commands::save_video_plan,
            video_commands::video_subtitles,
            video_commands::export_video_subtitles,
            video_commands::media_asset_path,
            video_commands::preview_video_job,
            video_commands::video_preview_media
        ])
        .run(tauri::generate_context!())
        .expect("书引 AI 桌面容器启动失败");
}
