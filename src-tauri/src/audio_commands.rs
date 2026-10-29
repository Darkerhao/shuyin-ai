use crate::{
    audio::{AudioAutomation, AudioJobView},
    database::{
        audio::{AudioJob, StartAudioJobInput},
        Database,
    },
};
use tauri::{AppHandle, State, WebviewWindow};

fn local_main(window: &WebviewWindow) -> Result<(), String> {
    if window.label() != "main" {
        return Err("仅主工作台可以控制音频任务。".into());
    }
    Ok(())
}
fn view(
    app: &AppHandle,
    database: &Database,
    automation: &AudioAutomation,
    project_id: &str,
) -> Result<AudioJobView, String> {
    let data = database.audio_job_data(project_id)?;
    Ok(AudioJobView {
        job: data.job,
        active_job: data.active_job,
        browser: automation.status(app),
    })
}
fn view_for_job(
    app: &AppHandle,
    database: &Database,
    automation: &AudioAutomation,
    job: &AudioJob,
) -> Result<AudioJobView, String> {
    view(app, database, automation, &job.project_id)
}
#[tauri::command]
pub fn audio_job_snapshot(
    app: AppHandle,
    database: State<'_, Database>,
    automation: State<'_, AudioAutomation>,
    project_id: String,
) -> Result<AudioJobView, String> {
    view(&app, &database, &automation, &project_id)
}
#[tauri::command]
pub async fn start_audio_job(
    app: AppHandle,
    window: WebviewWindow,
    database: State<'_, Database>,
    automation: State<'_, AudioAutomation>,
    input: StartAudioJobInput,
) -> Result<AudioJobView, String> {
    local_main(&window)?;
    automation.require_available()?;
    let job = database.start_audio_job(input)?;
    if let Err(error) = automation.open_notebook(&app, &job, "open_browser").await {
        let _ = database.audio_report_error(&job.id, job.run_id, "browser_open_failed", &error);
        return Err(error);
    }
    automation.notify();
    view_for_job(&app, &database, &automation, &job)
}
#[tauri::command]
pub async fn control_audio_job(
    app: AppHandle,
    window: WebviewWindow,
    database: State<'_, Database>,
    automation: State<'_, AudioAutomation>,
    id: String,
    action: String,
) -> Result<AudioJobView, String> {
    local_main(&window)?;
    let job = database.control_audio_job(&id, &action)?;
    if matches!(
        action.as_str(),
        "pause" | "cancel" | "resume" | "refresh_browser" | "reopen_browser"
    ) {
        automation.interrupt(&app);
    }
    if matches!(
        action.as_str(),
        "open_browser" | "resume" | "refresh_browser" | "reopen_browser"
    ) {
        if let Err(error) = automation.open_notebook(&app, &job, &action).await {
            let _ = database.audio_report_error(&job.id, job.run_id, "browser_open_failed", &error);
            return Err(error);
        }
    }
    automation.notify();
    view_for_job(&app, &database, &automation, &job)
}
