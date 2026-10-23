use crate::database::{Asset, Database, Project, ProjectInput, WorkspaceSettings};
use tauri::State;

#[tauri::command]
pub fn list_projects(
    database: State<'_, Database>,
    search: String,
    status: String,
) -> Result<Vec<Project>, String> {
    database.list_projects(&search, &status)
}
#[tauri::command]
pub fn get_project(database: State<'_, Database>, id: String) -> Result<Project, String> {
    database.get_project(&id)
}
#[tauri::command]
pub fn save_project(
    database: State<'_, Database>,
    id: Option<String>,
    input: ProjectInput,
) -> Result<Project, String> {
    database.save_project(id.as_deref(), input)
}
#[tauri::command]
pub fn delete_project(database: State<'_, Database>, id: String) -> Result<(), String> {
    database.delete_project(&id)
}
#[tauri::command]
pub fn list_assets(
    database: State<'_, Database>,
    project_id: String,
) -> Result<Vec<Asset>, String> {
    database.list_assets(&project_id)
}
#[tauri::command]
pub fn link_asset(
    database: State<'_, Database>,
    project_id: String,
    path: String,
    asset_id: Option<String>,
) -> Result<(), String> {
    database.link_asset(&project_id, &path, asset_id.as_deref())
}
#[tauri::command]
pub fn unlink_asset(database: State<'_, Database>, id: String) -> Result<(), String> {
    database.unlink_asset(&id)
}
#[tauri::command]
pub fn open_asset(
    database: State<'_, Database>,
    id: String,
    directory: bool,
) -> Result<(), String> {
    database.open_asset(&id, directory)
}
#[tauri::command]
pub fn workspace_settings(database: State<'_, Database>) -> Result<WorkspaceSettings, String> {
    database.workspace_settings()
}
#[tauri::command]
pub fn set_workspace_root(
    database: State<'_, Database>,
    path: String,
) -> Result<WorkspaceSettings, String> {
    database.set_workspace_root(&path)
}
#[tauri::command]
pub fn open_project_directory(database: State<'_, Database>, id: String) -> Result<(), String> {
    database.open_project_directory(&id)
}
#[tauri::command]
pub fn relocate_workspace(
    database: State<'_, Database>,
    id: String,
    path: String,
) -> Result<Project, String> {
    database.relocate_workspace(&id, &path)
}
#[tauri::command]
pub fn create_backup(database: State<'_, Database>, directory: String) -> Result<String, String> {
    database.create_backup(&directory)
}
#[tauri::command]
pub fn restore_backup(database: State<'_, Database>, path: String) -> Result<String, String> {
    database.restore_backup(&path)
}
