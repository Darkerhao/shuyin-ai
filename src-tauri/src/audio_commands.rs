use crate::{
    audio::{AudioAutomation, AudioJobView},
    database::{
        audio::{AudioJob, StartAudioJobInput},
        Database,
    },
};
use tauri::State;

fn view(
    database: &Database,
    automation: &AudioAutomation,
    project_id: &str,
) -> Result<AudioJobView, String> {
    let data = database.audio_job_data(project_id)?;
    Ok(AudioJobView {
        job: data.job,
        active_job: data.active_job,
        bridge: automation.status(),
    })
}

fn view_for_job(
    database: &Database,
    automation: &AudioAutomation,
    job: &AudioJob,
) -> Result<AudioJobView, String> {
    view(database, automation, &job.project_id)
}

#[tauri::command]
pub fn audio_job_snapshot(
    database: State<'_, Database>,
    automation: State<'_, AudioAutomation>,
    project_id: String,
) -> Result<AudioJobView, String> {
    view(database.inner(), automation.inner(), &project_id)
}

#[tauri::command]
pub fn start_audio_job(
    database: State<'_, Database>,
    automation: State<'_, AudioAutomation>,
    input: StartAudioJobInput,
) -> Result<AudioJobView, String> {
    automation.require_connected()?;
    let job = database.start_audio_job(input)?;
    if let Err(error) = automation.open_notebook(Some(&job.id), database.inner()) {
        let _ = database.audio_report_error(&job.id, job.run_id, "browser_launch_failed", &error);
        return Err(error);
    }
    view_for_job(database.inner(), automation.inner(), &job)
}

#[tauri::command]
pub fn control_audio_job(
    database: State<'_, Database>,
    automation: State<'_, AudioAutomation>,
    id: String,
    action: String,
) -> Result<AudioJobView, String> {
    let job = database.control_audio_job(&id, &action)?;
    if action == "open_browser" {
        automation.open_notebook(Some(&id), database.inner())?;
    }
    view_for_job(database.inner(), automation.inner(), &job)
}

#[tauri::command]
pub fn open_audio_extension_setup(automation: State<'_, AudioAutomation>) -> Result<(), String> {
    automation.open_extension_setup()
}
