use crate::database::{
    publishing::{PublishInput, PublishRecordInput, PublishTask},
    Database,
};
use tauri::State;

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
    id: Option<String>,
    input: PublishInput,
) -> Result<PublishTask, String> {
    database.save_publish_task(id.as_deref(), input)
}
#[tauri::command]
pub fn prepare_publish_task(
    database: State<'_, Database>,
    id: String,
) -> Result<PublishTask, String> {
    database.prepare_publish_task(&id)
}
#[tauri::command]
pub fn record_publish_success(
    database: State<'_, Database>,
    id: String,
    input: PublishRecordInput,
) -> Result<PublishTask, String> {
    database.record_publish_success(&id, input)
}
#[tauri::command]
pub fn open_publish_item(
    database: State<'_, Database>,
    id: String,
    item: String,
) -> Result<(), String> {
    database.open_publish_item(&id, &item)
}
