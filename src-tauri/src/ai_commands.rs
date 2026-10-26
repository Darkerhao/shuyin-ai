use crate::{
    ai::{
        publishing::PublishingDraft, revision, ActiveTasks, AiError, OpenAiCompatible,
        TextProvider, TextRequest, SYSTEM_PROMPT, TIMEOUT,
    },
    credentials::{Credentials, SystemCredentials},
    database::{
        ai::{
            AiTask, GenerationInput, GenerationResult, ProviderConfig, ProviderInput,
            SaveWritingInput, SavedProvider, WritingVersion,
        },
        Database,
    },
};
use tauri::State;

const AUDIO_TOOL: &str = "https://notebooklm.google.com/";

#[tauri::command]
pub fn open_audio_tool() -> Result<(), String> {
    crate::external::open_url(AUDIO_TOOL)
}

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
pub(crate) fn provider_key(
    config: &ProviderConfig,
    vault: &SystemCredentials,
) -> Result<Option<String>, AiError> {
    if !config.input.is_enabled {
        return Err(AiError::new("disabled", "所选模型已停用。"));
    }
    let key = if let Some(reference) = &config.credential_ref {
        Some(
            vault
                .get(reference)
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
    Ok(key)
}

fn request(
    config: ProviderConfig,
    vault: &SystemCredentials,
    system: &str,
    user: String,
) -> Result<TextRequest, AiError> {
    if config.input.model_kind != "text" {
        return Err(AiError::new("configuration", "请选择文本模型。"));
    }
    let key = provider_key(&config, vault)?;
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
    if config.input.model_kind == "image" {
        return Err(AiError::new(
            "configuration",
            "请在封面页生成背景以验证图片模型。",
        ));
    }
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
pub fn apply_generation(
    database: State<'_, Database>,
    id: String,
) -> Result<WritingVersion, String> {
    database.apply_generation(&id)
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
) -> Result<GenerationResult, AiError> {
    let database = database.inner();
    let vault = vault.inner();
    let id = id.as_str();
    run_task(database, active.inner(), id, |mut receiver| async move {
        let prepared=database.prepare_generation(id).map_err(AiError::database)?;
        let model=prepared.provider.input.model_name.clone();
        let system=if prepared.input.revision.is_some() { revision::SYSTEM_PROMPT } else { SYSTEM_PROMPT };
        let req=request(prepared.provider,vault,system,prepared.user)?;
        let provider=OpenAiCompatible::new(TIMEOUT, &req.base_url)?;
        let raw=tokio::select! {
            biased;
            _=receiver.changed()=>return Err(AiError::new("cancelled","生成已取消，原有内容保留。")),
            response=provider.generate(req)=>response?,
        };
        if let (Some(revision),Some(base))=(&prepared.input.revision,&prepared.base) {
            let content=revision::revise(&raw,&base.content,&revision.fields,&prepared.book_title).map_err(|s|AiError::new("invalid_content",&s))?;
            database.finish_revision(id,&prepared.input,content).map(GenerationResult::Preview).map_err(AiError::database)
        } else {
            let draft:PublishingDraft=serde_json::from_str(&raw).map_err(|_|AiError::new("invalid_content","模型输出不符合发布内容结构，原有内容保留。"))?;
            let content=draft.into_content(&prepared.book_title).map_err(|s|AiError::new("invalid_content",&s))?;
            database.finish_generation(id,&prepared.input,content,&model).map(GenerationResult::Version).map_err(AiError::database)
        }
    }).await
}

// All generation types share registration, cancellation status and failure recording.
pub(crate) async fn run_task<T, F, Fut>(
    database: &Database,
    active: &ActiveTasks,
    id: &str,
    execute: F,
) -> Result<T, AiError>
where
    F: FnOnce(tokio::sync::watch::Receiver<bool>) -> Fut,
    Fut: std::future::Future<Output = Result<T, AiError>>,
{
    let (sender, receiver) = tokio::sync::watch::channel(false);
    {
        let mut signals = active
            .0
            .lock()
            .map_err(|_| AiError::new("internal", "任务信号不可用。"))?;
        if signals.contains_key(id) {
            return Err(AiError::new("running", "任务已在运行。"));
        }
        signals.insert(id.to_string(), sender);
    }
    let result = execute(receiver).await;
    if let Ok(mut signals) = active.0.lock() {
        signals.remove(id);
    }
    if let Err(error) = &result {
        database
            .end_generation(
                id,
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
