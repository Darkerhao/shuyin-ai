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
    app: tauri::AppHandle,
    database: State<'_, Database>,
    vault: State<'_, SystemCredentials>,
    active: State<'_, ActiveTasks>,
    id: String,
) -> Result<Asset, AiError> {
    execute_image_task(
        &app,
        database.inner(),
        vault.inner(),
        active.inner(),
        &id,
        "cover",
    )
    .await
}

pub(crate) async fn execute_image_task(
    app: &tauri::AppHandle,
    database: &Database,
    vault: &SystemCredentials,
    active: &ActiveTasks,
    id: &str,
    kind: &str,
) -> Result<Asset, AiError> {
    let asset = run_task(database, active, id, |mut receiver| async move {
        let prepared = database.prepare_image_generation(id, kind).map_err(AiError::database)?;
        let request = ImageRequest {
            key: provider_key(&prepared.provider, vault)?,
            base_url: crate::ai::normalize_base_url(&prepared.provider.input.base_url).map_err(|e|AiError::new("configuration", &e))?,
            model: prepared.provider.input.model_name,
            prompt: prepared.prompt,
            width: prepared.width,
            height: prepared.height,
        };
        let provider = OpenAiCompatible::new(TIMEOUT, &request.base_url)?.observed(database,Some(id),kind);
        let bytes = tokio::select! {
            biased;
            _ = receiver.changed() => return Err(AiError::new("cancelled", "生成已取消，已保存内容保留。")),
            response = provider.generate_image(request) => response?,
        };
        database.finish_image_generation(id, &bytes, kind).map_err(AiError::database)
    }).await?;
    use crate::completion::{notify, Completion};
    notify(
        app,
        id,
        if kind == "cover" {
            Completion::CoverBackground
        } else {
            Completion::Illustration
        },
    );
    Ok(asset)
}

#[tauri::command]
pub fn list_cover_versions(
    database: State<'_, Database>,
    project_id: String,
    chapter_id: Option<String>,
) -> Result<Vec<CoverVersion>, String> {
    database.list_cover_versions(&project_id, chapter_id.as_deref())
}

#[tauri::command]
pub fn save_cover_version(
    app: tauri::AppHandle,
    database: State<'_, Database>,
    project_id: String,
    config: CoverConfig,
    png: Option<Vec<u8>>,
    chapter_id: Option<String>,
) -> Result<CoverVersion, String> {
    let version =
        database.save_cover_version(&project_id, config, png.as_deref(), chapter_id.as_deref())?;
    if png.is_some() {
        crate::completion::cover(&app, &version);
    }
    Ok(version)
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

#[tauri::command]
pub fn read_cover_export(
    database: State<'_, Database>,
    project_id: String,
    id: String,
) -> Result<Response, String> {
    database
        .read_cover_export(&project_id, &id)
        .map(Response::new)
}
#[tauri::command]
pub fn list_cover_presets(
    database: State<'_, Database>,
) -> Result<Vec<crate::database::covers::CoverPreset>, String> {
    database.list_cover_presets()
}
#[tauri::command]
pub fn save_cover_preset(
    database: State<'_, Database>,
    name: String,
    config: CoverConfig,
) -> Result<(), String> {
    database.save_cover_preset(&name, config)
}
#[tauri::command]
pub fn delete_cover_preset(database: State<'_, Database>, id: String) -> Result<(), String> {
    database.delete_cover_preset(&id)
}
