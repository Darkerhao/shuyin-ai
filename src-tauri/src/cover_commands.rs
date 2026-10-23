use crate::database::{
    covers::{CoverConfig, CoverVersion},
    Database,
};
use crate::{
    ai::{
        images::{ImageProvider, ImageRequest, TIMEOUT},
        ActiveTasks, AiError, OpenAiCompatible,
    },
    ai_commands::{provider_key, run_task},
    credentials::SystemCredentials,
    database::{image_generation::ImageGenerationInput, Asset},
};
use tauri::{ipc::Response, State};

#[tauri::command]
pub fn start_cover_generation(
    database: State<'_, Database>,
    input: ImageGenerationInput,
) -> Result<String, String> {
    database.start_cover_generation(input)
}

#[tauri::command]
pub async fn execute_cover_generation(
    database: State<'_, Database>,
    vault: State<'_, SystemCredentials>,
    active: State<'_, ActiveTasks>,
    id: String,
) -> Result<Asset, AiError> {
    let database = database.inner();
    let vault = vault.inner();
    let id = id.as_str();
    run_task(database, active.inner(), id, |mut receiver| async move {
        let prepared = database.prepare_cover_generation(id).map_err(AiError::database)?;
        let request = ImageRequest {
            key: provider_key(&prepared.provider, vault)?,
            base_url: crate::ai::normalize_base_url(&prepared.provider.input.base_url).map_err(|e|AiError::new("configuration", &e))?,
            model: prepared.provider.input.model_name,
            prompt: prepared.prompt,
        };
        let provider = OpenAiCompatible::new(TIMEOUT, &request.base_url)?;
        let bytes = tokio::select! {
            biased;
            _ = receiver.changed() => return Err(AiError::new("cancelled", "生成已取消，当前封面保留。")),
            response = provider.generate_image(request) => response?,
        };
        database.finish_cover_generation(id, &bytes).map_err(AiError::database)
    }).await
}

#[tauri::command]
pub fn list_cover_versions(
    database: State<'_, Database>,
    project_id: String,
) -> Result<Vec<CoverVersion>, String> {
    database.list_cover_versions(&project_id)
}

#[tauri::command]
pub fn save_cover_version(
    database: State<'_, Database>,
    project_id: String,
    config: CoverConfig,
    png: Option<Vec<u8>>,
) -> Result<CoverVersion, String> {
    database.save_cover_version(&project_id, config, png.as_deref())
}

#[tauri::command]
pub fn read_cover_background(
    database: State<'_, Database>,
    project_id: String,
    asset_id: String,
) -> Result<Response, String> {
    database
        .read_cover_background(&project_id, &asset_id)
        .map(Response::new)
}
