use crate::{
    ai::{
        publishing::PublishingDraft, ActiveTasks, AiError, OpenAiCompatible, TextProvider,
        TextRequest, SYSTEM_PROMPT, TIMEOUT,
    },
    credentials::{Credentials, SystemCredentials},
    database::{
        ai::{
            AiTask, GenerationInput, ProviderConfig, ProviderInput, SaveWritingInput,
            SavedProvider, WritingVersion,
        },
        Database,
    },
};
use tauri::State;

#[tauri::command]
pub fn list_model_providers(
    database: State<'_, Database>,
    vault: State<'_, SystemCredentials>,
) -> Result<Vec<ProviderConfig>, String> {
    database.list_providers(vault.inner())
}
#[tauri::command]
pub fn save_model_provider(
    database: State<'_, Database>,
    vault: State<'_, SystemCredentials>,
    id: Option<String>,
    input: ProviderInput,
    api_key: Option<String>,
) -> Result<SavedProvider, String> {
    database.save_provider(id.as_deref(), input, api_key.as_deref(), vault.inner())
}
#[tauri::command]
pub fn delete_model_provider(
    database: State<'_, Database>,
    vault: State<'_, SystemCredentials>,
    id: String,
) -> Result<(), String> {
    database.delete_provider(&id, vault.inner())
}
fn request(
    config: ProviderConfig,
    vault: &SystemCredentials,
    system: &str,
    user: String,
) -> Result<TextRequest, AiError> {
    if !config.input.is_enabled {
        return Err(AiError::new("disabled", "所选模型已停用。"));
    }
    let key = if let Some(reference) = config.credential_ref {
        Some(
            vault
                .get(&reference)
                .map_err(|s| AiError::new("credentials", &s))?
                .ok_or_else(|| {
                    AiError::new(
                        "credentials",
                        "系统中未找到此模型密钥，请在模型设置中重新输入。",
                    )
                })?,
        )
    } else {
        None
    };
    Ok(TextRequest {
        base_url: crate::ai::normalize_base_url(&config.input.base_url)
            .map_err(|s| AiError::new("configuration", &s))?,
        model: config.input.model_name,
        key,
        system: system.into(),
        user,
    })
}
#[tauri::command]
pub async fn test_model_connection(
    database: State<'_, Database>,
    vault: State<'_, SystemCredentials>,
    id: String,
) -> Result<String, AiError> {
    let config = database.get_provider(&id).map_err(AiError::database)?;
    let req = request(
        config,
        vault.inner(),
        "Reply with OK only.",
        "Connection test".into(),
    )?;
    OpenAiCompatible::new(TIMEOUT, &req.base_url)?
        .generate(req)
        .await?;
    Ok("连接成功，模型已返回完整文本。".into())
}
#[tauri::command]
pub fn list_writing_versions(
    database: State<'_, Database>,
    project_id: String,
    kind: String,
) -> Result<Vec<WritingVersion>, String> {
    database.list_versions(&project_id, &kind)
}
#[tauri::command]
pub fn save_writing_version(
    database: State<'_, Database>,
    input: SaveWritingInput,
) -> Result<WritingVersion, String> {
    database.save_writing(input)
}
#[tauri::command]
pub fn list_ai_tasks(
    database: State<'_, Database>,
    project_id: String,
) -> Result<Vec<AiTask>, String> {
    database.list_ai_tasks(&project_id)
}
#[tauri::command]
pub fn start_generation(
    database: State<'_, Database>,
    input: GenerationInput,
) -> Result<String, String> {
    database.start_generation(input)
}
#[tauri::command]
pub fn cancel_generation(
    database: State<'_, Database>,
    active: State<'_, ActiveTasks>,
    id: String,
) -> Result<(), String> {
    // Both completion and cancellation check terminal status under the SQLite lock.
    database.end_generation(&id, "cancelled", "cancelled", "用户已取消生成。")?;
    if let Some(sender) = active.0.lock().map_err(|_| "任务信号不可用。")?.get(&id) {
        let _ = sender.send(true);
    }
    Ok(())
}
#[tauri::command]
pub async fn execute_generation(
    database: State<'_, Database>,
    vault: State<'_, SystemCredentials>,
    active: State<'_, ActiveTasks>,
    id: String,
) -> Result<WritingVersion, AiError> {
    let (sender, mut receiver) = tokio::sync::watch::channel(false);
    {
        let mut signals = active
            .0
            .lock()
            .map_err(|_| AiError::new("internal", "任务信号不可用。"))?;
        if signals.contains_key(&id) {
            return Err(AiError::new("running", "任务已在运行。"));
        }
        signals.insert(id.clone(), sender);
    }
    let result=async {
        let prepared=database.prepare_generation(&id).map_err(AiError::database)?;
        let model=prepared.provider.input.model_name.clone();
        let req=request(prepared.provider,vault.inner(),SYSTEM_PROMPT,prepared.user)?;
        let provider=OpenAiCompatible::new(TIMEOUT, &req.base_url)?;
        let raw=tokio::select! {
            biased;
            _=receiver.changed()=>return Err(AiError::new("cancelled","生成已取消，原有内容保留。")),
            response=provider.generate(req)=>response?,
        };
        let draft:PublishingDraft=serde_json::from_str(&raw).map_err(|_|AiError::new("invalid_content","模型输出不符合发布内容结构，原有内容保留。"))?;
        let content=draft.into_content(&prepared.book_title).map_err(|s|AiError::new("invalid_content",&s))?;
        database.finish_generation(&id,&prepared.input,content,&model).map_err(AiError::database)
    }.await;
    if let Ok(mut signals) = active.0.lock() {
        signals.remove(&id);
    }
    if let Err(error) = &result {
        database
            .end_generation(
                &id,
                if error.code == "cancelled" {
                    "cancelled"
                } else {
                    "failed"
                },
                &error.code,
                &error.message,
            )
            .map_err(AiError::database)?;
    }
    result
}
