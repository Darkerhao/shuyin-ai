use crate::database::{
    publishing::{PublishInput, PublishQueueItem, PublishRecordInput, PublishTask},
    Database,
};
use tauri::State;

#[tauri::command]
pub fn list_publish_queue(database: State<'_, Database>) -> Result<Vec<PublishQueueItem>, String> {
    database.list_publish_queue()
}

#[tauri::command]
pub fn list_publish_tasks(
    database: State<'_, Database>,
    project_id: String,
) -> Result<Vec<PublishTask>, String> {
    database.list_publish_tasks(&project_id)
}
#[tauri::command]
pub fn save_publish_task(
    database: State<'_, Database>,
    publisher: State<'_, crate::douyin::DouyinPublisher>,
    id: Option<String>,
    input: PublishInput,
) -> Result<PublishTask, String> {
    publisher.ensure_idle()?;
    database.save_publish_task(id.as_deref(), input)
}
#[tauri::command]
pub fn prepare_publish_task(
    app: tauri::AppHandle,
    database: State<'_, Database>,
    publisher: State<'_, crate::douyin::DouyinPublisher>,
    id: String,
) -> Result<PublishTask, String> {
    publisher.ensure_idle()?;
    let task = database.prepare_publish_task(&id)?;
    if let Some(directory) = &task.task_directory {
        crate::completion::notify(
            &app,
            directory,
            crate::completion::Completion::PublishPackage,
        );
    }
    Ok(task)
}
#[tauri::command]
pub fn record_publish_success(
    app: tauri::AppHandle,
    database: State<'_, Database>,
    publisher: State<'_, crate::douyin::DouyinPublisher>,
    id: String,
    input: PublishRecordInput,
) -> Result<PublishTask, String> {
    publisher.ensure_idle()?;
    let task = database.record_publish_success(&id, input)?;
    publisher.release_recorded(&id);
    crate::completion::notify(&app, &id, crate::completion::Completion::PublishRecorded);
    Ok(task)
}
#[tauri::command]
pub fn open_publish_item(
    database: State<'_, Database>,
    id: String,
    item: String,
) -> Result<(), String> {
    database.open_publish_item(&id, &item)
}
