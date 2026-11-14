use super::{
    projects::{get, new_id, NOW},
    Database,
};
use crate::{
    ai::{
        self,
        revision::{self, PublishingField, RevisionInput},
        WritingContent, PROMPT_VERSION,
    },
    credentials::Credentials,
};
use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ModelOptionField {
    BaseUrl,
    ModelName,
}
impl ModelOptionField {
    fn storage(self) -> (&'static str, &'static str) {
        match self {
            Self::BaseUrl => ("model_options_base_url", "base_url"),
            Self::ModelName => ("model_options_model_name", "model_name"),
        }
    }
    fn normalize(self, value: &str) -> Result<String, String> {
        match self {
            Self::BaseUrl => ai::normalize_base_url(value),
            Self::ModelName => normalize_model_name(value),
        }
    }
}

fn write_model_options(c: &Connection, key: &str, values: &[String]) -> Result<(), String> {
    let json = serde_json::to_string(values).map_err(|e| e.to_string())?;
    c.execute(
        "INSERT INTO app_settings(key,value) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        params![key, json],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn normalize_model_name(value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() || value.len() > 200 {
        return Err("模型 ID 不能为空且不能超过 200 字节。".into());
    }
    Ok(value.into())
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderInput {
    pub model_kind: String,
    pub name: String,
    pub base_url: String,
    pub model_name: String,
    #[serde(default)]
    pub reasoning_effort: Option<String>,
    pub is_default: bool,
    pub is_enabled: bool,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConfig {
    pub id: String,
    #[serde(flatten)]
    pub input: ProviderInput,
    pub credential_status: String,
    #[serde(skip)]
    pub credential_ref: Option<String>,
}
fn row_provider(r: &Row<'_>) -> rusqlite::Result<ProviderConfig> {
    Ok(ProviderConfig {
        id: r.get(0)?,
        input: ProviderInput {
            name: r.get(1)?,
            base_url: r.get(2)?,
            model_name: r.get(3)?,
            is_default: r.get(4)?,
            is_enabled: r.get(5)?,
            model_kind: r.get(7)?,
            reasoning_effort: r.get(8)?,
        },
        credential_ref: r.get(6)?,
        credential_status: "missing".into(),
    })
}
pub(super) fn provider(c: &Connection, id: &str) -> Result<ProviderConfig, String> {
    c.query_row("SELECT id,name,base_url,model_name,is_default,is_enabled,credential_ref,model_kind,reasoning_effort FROM model_providers WHERE id=?1", [id], row_provider)
        .optional().map_err(|e|e.to_string())?.ok_or_else(||"模型配置不存在。".into())
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedProvider {
    pub provider: ProviderConfig,
    pub warning: Option<String>,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WritingVersion {
    pub id: String,
    pub project_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chapter_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_snapshot: Option<super::chapters::ChapterSource>,
    pub kind: String,
    pub version_no: i64,
    pub content: WritingContent,
    pub source_type: String,
    pub model_name: String,
    pub prompt_version: String,
    pub is_confirmed: bool,
    pub created_at: String,
}
fn row_version(r: &Row<'_>) -> rusqlite::Result<WritingVersion> {
    let json: String = r.get(4)?;
    let content = serde_json::from_str(&json).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(4, rusqlite::types::Type::Text, Box::new(e))
    })?;
    Ok(WritingVersion {
        id: r.get(0)?,
        project_id: r.get(1)?,
        chapter_id: r.get(10)?,
        source_snapshot: r
            .get::<_, Option<String>>(11)?
            .map(|json| serde_json::from_str(&json))
            .transpose()
            .map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    11,
                    rusqlite::types::Type::Text,
                    Box::new(error),
                )
            })?,
        kind: r.get(2)?,
        version_no: r.get(3)?,
        content,
        source_type: r.get(5)?,
        model_name: r.get(6)?,
        prompt_version: r.get(7)?,
        is_confirmed: r.get(8)?,
        created_at: r.get(9)?,
    })
}
const VERSION_COLUMNS: &str = "id,project_id,kind,version_no,content_json,source_type,model_name,prompt_version,is_confirmed,created_at,chapter_id,source_snapshot_json";
pub(super) fn version(c: &Connection, id: &str) -> Result<WritingVersion, String> {
    c.query_row(
        &format!("SELECT {VERSION_COLUMNS} FROM writing_versions WHERE id=?1"),
        [id],
        row_version,
    )
    .map_err(|_| "创作版本不存在。".into())
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveWritingInput {
    pub project_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chapter_id: Option<String>,
    pub kind: String,
    pub content: WritingContent,
    pub based_on: Option<String>,
    pub is_confirmed: bool,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerationInput {
    pub project_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chapter_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_snapshot: Option<super::chapters::ChapterSource>,
    pub provider_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<RevisionInput>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerationPreview {
    pub task_id: String,
    pub fields: Vec<PublishingField>,
    pub before: WritingContent,
    pub content: WritingContent,
}
#[derive(Serialize)]
#[serde(untagged)]
pub enum GenerationResult {
    Version(WritingVersion),
    Preview(GenerationPreview),
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiTask {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chapter_id: Option<String>,
    pub kind: String,
    pub status: String,
    pub model_name: String,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub version_id: Option<String>,
    pub created_at: String,
}
const TASK_COLUMNS: &str =
    "id,kind,status,model_name,error_code,error_message,version_id,created_at,chapter_id";
fn row_task(r: &Row<'_>) -> rusqlite::Result<AiTask> {
    Ok(AiTask {
        id: r.get(0)?,
        chapter_id: r.get(8)?,
        kind: r.get(1)?,
        status: r.get(2)?,
        model_name: r.get(3)?,
        error_code: r.get(4)?,
        error_message: r.get(5)?,
        version_id: r.get(6)?,
        created_at: r.get(7)?,
    })
}
pub struct PreparedGeneration {
    pub input: GenerationInput,
    pub provider: ProviderConfig,
    pub user: String,
    pub book_title: String,
    pub chapter_title: Option<String>,
    pub base: Option<WritingVersion>,
}

fn revision_base(
    c: &Connection,
    input: &GenerationInput,
) -> Result<Option<WritingVersion>, String> {
    input
        .revision
        .as_ref()
        .map(|revision| {
            revision.validate()?;
            let base = version(c, &revision.based_on)?;
            if base.project_id != input.project_id
                || base.chapter_id != input.chapter_id
                || base.kind != "publishing"
            {
                return Err("调整来源不属于当前书籍的发布内容。".into());
            }
            Ok(base)
        })
        .transpose()
}

pub(super) fn append(
    c: &Connection,
    input: &SaveWritingInput,
    source: &str,
    model: &str,
    prompt: &str,
    source_snapshot: Option<&super::chapters::ChapterSource>,
) -> Result<WritingVersion, String> {
    input.content.validate(&input.kind, None)?;
    super::chapters::require_scope(c, &input.project_id, input.chapter_id.as_deref())?;
    let inherited = input
        .based_on
        .as_deref()
        .map(|id| version(c, id))
        .transpose()?;
    if inherited.as_ref().is_some_and(|base| {
        base.project_id != input.project_id || base.chapter_id != input.chapter_id
    }) {
        return Err("创作来源不属于本次书籍与章节。".into());
    }
    let snapshot = if let Some(source) = source_snapshot {
        Some(source.clone())
    } else if let Some(base) = inherited {
        base.source_snapshot
    } else if let Some(chapter) = &input.chapter_id {
        Some(super::chapters::snapshot(c, &input.project_id, chapter)?)
    } else {
        None
    };
    if let Some(chapter) = input.chapter_id.as_deref() {
        snapshot
            .as_ref()
            .ok_or("章节版本缺少来源快照。")?
            .validate(chapter)?;
    }
    let id = new_id(c)?;
    c.execute(&format!("INSERT INTO writing_versions(id,project_id,kind,version_no,content_json,source_type,model_name,prompt_version,is_confirmed,created_at,chapter_id,source_snapshot_json)
        SELECT ?1,?2,?3,COALESCE(MAX(version_no),0)+1,?4,?5,?6,?7,?8,{NOW},?9,?10 FROM writing_versions WHERE project_id=?2 AND kind=?3"),
        params![id,input.project_id,input.kind,serde_json::to_string(&input.content).map_err(|e|e.to_string())?,source,model,prompt,input.is_confirmed,input.chapter_id,snapshot.as_ref().map(serde_json::to_string).transpose().map_err(|error| error.to_string())?]).map_err(|e|e.to_string())?;
    c.execute(
        &format!("UPDATE book_projects SET updated_at={NOW} WHERE id=?1"),
        [&input.project_id],
    )
    .map_err(|e| e.to_string())?;
    version(c, &id)
}
impl Database {
    pub fn model_field_options(&self, field: ModelOptionField) -> Result<Vec<String>, String> {
        self.with_connection(|c| {
            let (key, column) = field.storage();
            let stored: Option<String> = c
                .query_row("SELECT value FROM app_settings WHERE key=?1", [key], |r| {
                    r.get(0)
                })
                .optional()
                .map_err(|e| e.to_string())?;
            if let Some(json) = stored {
                return serde_json::from_str(&json).map_err(|e| e.to_string());
            }
            // Import existing values once; removed suggestions must not reappear on reload.
            let mut query = c
                .prepare(&format!(
                    "SELECT DISTINCT {column} FROM model_providers ORDER BY {column}"
                ))
                .map_err(|e| e.to_string())?;
            let values = query
                .query_map([], |r| r.get::<_, String>(0))
                .map_err(|e| e.to_string())?
                .collect::<rusqlite::Result<Vec<_>>>()
                .map_err(|e| e.to_string())?;
            write_model_options(c, key, &values)?;
            Ok(values)
        })
    }

    pub fn save_model_field_options(
        &self,
        field: ModelOptionField,
        values: Vec<String>,
    ) -> Result<Vec<String>, String> {
        let mut normalized = Vec::new();
        for value in values {
            let value = field.normalize(&value)?;
            if !normalized.contains(&value) {
                normalized.push(value);
            }
        }
        self.with_connection(|c| write_model_options(c, field.storage().0, &normalized))?;
        Ok(normalized)
    }

    pub fn list_providers(&self, vault: &impl Credentials) -> Result<Vec<ProviderConfig>, String> {
        self.with_connection(|c| {
            let mut q=c.prepare("SELECT id,name,base_url,model_name,is_default,is_enabled,credential_ref,model_kind,reasoning_effort FROM model_providers ORDER BY is_default DESC,name,id").map_err(|e|e.to_string())?;
            let rows=q.query_map([],row_provider).map_err(|e|e.to_string())?;
            rows.map(|r| {let mut p=r.map_err(|e|e.to_string())?; p.credential_status=match p.credential_ref.as_deref().map(|s|vault.get(s)) {
                Some(Ok(Some(_)))=>"available", Some(Err(_))=>"unavailable", _=>"missing"
            }.into(); Ok(p)}).collect()
        })
    }
    pub fn get_provider(&self, id: &str) -> Result<ProviderConfig, String> {
        self.with_connection(|c| provider(c, id))
    }
    pub fn save_provider(
        &self,
        id: Option<&str>,
        mut input: ProviderInput,
        api_key: Option<&str>,
        vault: &impl Credentials,
    ) -> Result<SavedProvider, String> {
        input.base_url = ai::normalize_base_url(&input.base_url)?;
        input.name = input.name.trim().into();
        input.model_name = normalize_model_name(&input.model_name)
            .map_err(|_| "请填写服务名称、模型名；默认模型必须启用。")?;
        if !matches!(input.model_kind.as_str(), "text" | "image")
            || input.name.is_empty()
            || input.name.chars().count() > 100
            || (input.is_default && !input.is_enabled)
        {
            return Err("请填写服务名称、模型名；默认模型必须启用。".into());
        }
        if input.model_kind == "image" {
            input.reasoning_effort = None;
        }
        if input.reasoning_effort.as_deref().is_some_and(|effort| {
            !matches!(
                effort,
                "none" | "minimal" | "low" | "medium" | "high" | "xhigh" | "max"
            )
        }) {
            return Err("不支持的推理强度，请选择服务默认或列表中的档位。".into());
        }
        if api_key
            .is_some_and(|s| s.trim().is_empty() || s.len() > 4096 || s.contains(['\r', '\n']))
        {
            return Err("API Key 不能为空、换行或超过 4096 字节。".into());
        }
        self.with_connection(|c| {
            let old=id.map(|s|provider(c,s)).transpose()?;
            if old.as_ref().is_some_and(|p|p.input.base_url!=input.base_url && p.credential_ref.is_some()) && api_key.is_none() {
                return Err("更换服务地址时请重新输入 API Key，避免将旧密钥发送到新地址。".into());
            }
            if old.as_ref().is_some_and(|p| p.input.model_kind != input.model_kind) && c.query_row("SELECT EXISTS(SELECT 1 FROM ai_tasks WHERE provider_id=?1 AND status IN ('pending','running'))", [id], |r| r.get::<_, bool>(0)).map_err(|e|e.to_string())? {
                return Err("模型有未完成任务，暂不能更改用途。".into());
            }
            let id=id.map(str::to_string).map(Ok).unwrap_or_else(||new_id(c))?;
            let new_ref=if api_key.is_some(){Some(format!("text-{}",new_id(c)?))}else{old.as_ref().and_then(|p|p.credential_ref.clone())};
            if let (Some(key),Some(reference))=(api_key,new_ref.as_deref()){vault.set(reference,key)?;}
            let saved=(|| {
                let tx=c.transaction().map_err(|e|e.to_string())?;
                if input.is_default {tx.execute("UPDATE model_providers SET is_default=0 WHERE model_kind=?1",[&input.model_kind]).map_err(|e|e.to_string())?;}
                tx.execute(&format!("INSERT INTO model_providers(id,name,provider_type,base_url,model_name,is_default,is_enabled,credential_ref,created_at,updated_at,model_kind,reasoning_effort)
                    VALUES(?1,?2,'openai-compatible',?3,?4,?5,?6,?7,{NOW},{NOW},?8,?9) ON CONFLICT(id) DO UPDATE SET name=excluded.name,base_url=excluded.base_url,model_name=excluded.model_name,is_default=excluded.is_default,is_enabled=excluded.is_enabled,credential_ref=excluded.credential_ref,updated_at=excluded.updated_at,model_kind=excluded.model_kind,reasoning_effort=excluded.reasoning_effort"),
                    params![id,input.name,input.base_url,input.model_name,input.is_default,input.is_enabled,new_ref,input.model_kind,input.reasoning_effort]).map_err(|e|e.to_string())?;
                let saved=provider(&tx,&id)?; tx.commit().map_err(|e|e.to_string())?; Ok::<_,String>(saved)
            })();
            if saved.is_err() && api_key.is_some() {if let Some(r)=&new_ref {vault.delete(r)?;}}
            let mut provider=saved?;
            provider.credential_status=match new_ref.as_deref().map(|r|vault.get(r)) { Some(Ok(Some(_)))=>"available",Some(Err(_))=>"unavailable",_=>"missing" }.into();
            let warning=if api_key.is_some(){old.and_then(|p|p.credential_ref).and_then(|r|vault.delete(&r).err()).map(|_|"配置已保存，但旧凭据清理失败；可在系统凭据管理器中清理旧 text- 条目。".into())}else{None};
            Ok(SavedProvider{provider,warning})
        })
    }
    pub fn delete_provider(&self, id: &str, vault: &impl Credentials) -> Result<(), String> {
        self.with_connection(|c| {
            let old=provider(c,id)?;
            if c.query_row("SELECT EXISTS(SELECT 1 FROM ai_tasks WHERE provider_id=?1 AND status IN ('pending','running'))",[id],|r|r.get::<_,bool>(0)).map_err(|e|e.to_string())? {return Err("此模型仍有生成任务，请先取消或等待完成。".into());}
            let tx=c.transaction().map_err(|e|e.to_string())?;
            tx.execute("DELETE FROM model_providers WHERE id=?1",[id]).map_err(|e|e.to_string())?;
            if let Some(r)=old.credential_ref {vault.delete(&r)?;}
            tx.commit().map_err(|e|e.to_string())
        })
    }
    pub fn list_versions(
        &self,
        project_id: &str,
        kind: &str,
        chapter_id: Option<&str>,
    ) -> Result<Vec<WritingVersion>, String> {
        ai::validate_kind(kind)?;
        self.with_connection(|c| {super::chapters::require_scope(c,project_id,chapter_id)?;let mut q=c.prepare(&format!("SELECT {VERSION_COLUMNS} FROM writing_versions WHERE project_id=?1 AND kind=?2 AND chapter_id IS ?3 ORDER BY version_no DESC")).map_err(|e|e.to_string())?;
            let rows=q.query_map(params![project_id,kind,chapter_id],row_version).map_err(|e|e.to_string())?; rows.collect::<rusqlite::Result<Vec<_>>>().map_err(|e|e.to_string())})
    }
    pub fn save_writing(&self, input: SaveWritingInput) -> Result<WritingVersion, String> {
        self.with_connection(|c| {
            let base = input
                .based_on
                .as_deref()
                .map(|s| version(c, s))
                .transpose()?;
            if base.as_ref().is_some_and(|b| {
                b.project_id != input.project_id
                    || b.chapter_id != input.chapter_id
                    || b.kind != input.kind
            }) {
                return Err("来源版本不属于当前项目或内容类型。".into());
            }
            let source = if base.as_ref().is_some_and(|b| b.source_type != "manual") {
                "mixed"
            } else {
                "manual"
            };
            let snapshot = base.as_ref().and_then(|base| base.source_snapshot.clone());
            let tx = c.transaction().map_err(|e| e.to_string())?;
            let saved = append(
                &tx,
                &input,
                source,
                base.as_ref().map_or("", |b| &b.model_name),
                base.as_ref().map_or("", |b| &b.prompt_version),
                snapshot.as_ref(),
            )?;
            tx.commit().map_err(|e| e.to_string())?;
            Ok(saved)
        })
    }
    pub fn list_ai_tasks(
        &self,
        project_id: &str,
        chapter_id: Option<&str>,
    ) -> Result<Vec<AiTask>, String> {
        self.with_connection(|c| {super::chapters::require_scope(c,project_id,chapter_id)?;let mut q=c.prepare(&format!("SELECT {TASK_COLUMNS} FROM ai_tasks WHERE project_id=?1 AND chapter_id IS ?2 ORDER BY created_at DESC,id DESC LIMIT 30")).map_err(|e|e.to_string())?;
            let rows=q.query_map(params![project_id,chapter_id],row_task).map_err(|e|e.to_string())?; rows.collect::<rusqlite::Result<Vec<_>>>().map_err(|e|e.to_string())})
    }
    pub fn start_generation(&self, input: GenerationInput) -> Result<String, String> {
        self.start_bound_generation(input, None)
    }
    pub(super) fn start_bound_generation(
        &self,
        mut input: GenerationInput,
        binding: Option<&super::automation::Binding<'_>>,
    ) -> Result<String, String> {
        self.with_connection(|connection| {
            let transaction = connection.transaction().map_err(|e| e.to_string())?;
            let c = &transaction;
            if binding.is_none() && c.query_row("SELECT EXISTS(SELECT 1 FROM automation_runs WHERE project_id=?1 AND json_extract(input_json,'$.workflowVersion')=2 AND status NOT IN ('completed','cancelled'))",[&input.project_id],|row|row.get::<_,bool>(0)).map_err(|e|e.to_string())? { return Err("本书正在自动制作，请查看本次创作方案，或停止流程后手动生成。".into()); }
            super::chapters::require_scope(c,&input.project_id,input.chapter_id.as_deref())?;
            let base = revision_base(c, &input)?;
            if let Some(base) = base {
                input.source_snapshot = base.source_snapshot;
            } else if binding.is_none() {
                input.source_snapshot = input.chapter_id.as_deref().map(|chapter| super::chapters::snapshot(c,&input.project_id,chapter)).transpose()?;
            }
            if let Some(chapter) = &input.chapter_id {
                input.source_snapshot.as_ref().ok_or("章节任务缺少来源快照。")?.validate(chapter)?;
            }
            let p=provider(c,&input.provider_id)?;
            if !p.input.is_enabled || p.input.model_kind != "text" {return Err("请选择已启用的文本模型。".into());}
            let id=new_id(c)?;
            let prompt = if input.revision.is_some() { revision::PROMPT_VERSION } else { PROMPT_VERSION };
            c.execute(&format!("INSERT INTO ai_tasks(id,project_id,kind,provider_id,model_name,prompt_version,status,input_json,created_at,chapter_id) VALUES(?1,?2,?3,?4,?5,?6,'pending',?7,{NOW},?8)"),params![id,input.project_id,"publishing",input.provider_id,p.input.model_name,prompt,serde_json::to_string(&input).map_err(|e|e.to_string())?,input.chapter_id]).map_err(|e|e.to_string())?;
            if let Some(binding) = binding { super::automation::bind(c, binding, &input.project_id, &id)?; }
            transaction.commit().map_err(|e| e.to_string())?;
            Ok(id)
        })
    }
    pub fn prepare_generation(&self, id: &str) -> Result<PreparedGeneration, String> {
        self.with_connection(|c| {
            let (json,prompt):(String,String)=c.query_row("SELECT input_json,prompt_version FROM ai_tasks WHERE id=?1 AND kind='publishing' AND status='pending'",[id],|r|Ok((r.get(0)?,r.get(1)?))).map_err(|_|"任务已取消、已运行或不存在。")?;
            if prompt.starts_with("audio-duration-") { return Err("时长推荐已取消，请直接生成内容。".into()); }
            let input:GenerationInput=serde_json::from_str(&json).map_err(|_|"任务输入无效。")?;
            let project=get(c,&input.project_id)?;
            let mut p=provider(c,&input.provider_id)?;
            if !p.input.is_enabled || p.input.model_kind != "text" {return Err("请选择已启用的文本模型。".into());}
            let base = revision_base(c, &input)?;
            let mut user=serde_json::json!({"book":{"title":project.input.title,"author":project.input.author,"description":project.input.description,"sourceNotes":project.input.source_notes}});
            let mut book_title = project.input.title;
            let chapter_title = input.source_snapshot.as_ref().map(|source| source.chapter_title.clone());
            if let Some(run) = super::automation::generation_run(c, id)? {
                if run.status != "running" { return Err("自动流程已暂停，生成请求尚未发送。".into()); }
                user["book"] = serde_json::json!({"title":run.input.title,"author":run.input.author,"description":run.input.description,"sourceNotes":run.input.source_notes});
                book_title = run.input.title;
                p.input = run.input.text_provider;
            }
            if let Some(source) = &input.source_snapshot {
                source.validate(input.chapter_id.as_deref().ok_or("章节范围缺失。")?)?;
                user["book"] = source.model_book();
                user["sourceScope"] = serde_json::json!({"kind":"chapter","chapterId":source.chapter_id,"chapterTitle":source.chapter_title});
                book_title = source.book_title.clone();
            } else { user["sourceScope"] = serde_json::json!({"kind":"book"}); }
            user["audio"] = serde_json::json!({"seriesName": if chapter_title.is_some() {"读懂一章"} else {"读懂一本书"}});
            if let Some(base) = &base {
                user["currentContent"] = serde_json::to_value(&base.content).map_err(|e|e.to_string())?;
                user["revision"] = serde_json::to_value(&input.revision).map_err(|e|e.to_string())?;
            }
            c.execute("UPDATE ai_tasks SET status='running',model_name=?2 WHERE id=?1 AND status='pending'",params![id,p.input.model_name]).map_err(|e|e.to_string())?;
            Ok(PreparedGeneration{input,provider:p,user:user.to_string(),book_title,chapter_title,base})
        })
    }
    pub fn finish_generation(
        &self,
        id: &str,
        input: &GenerationInput,
        content: WritingContent,
        model: &str,
    ) -> Result<WritingVersion, String> {
        content.validate("publishing", None)?;
        self.with_connection(|c|{
            let tx=c.transaction().map_err(|e|e.to_string())?;
            let running:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM ai_tasks WHERE id=?1 AND status='running')",[id],|r|r.get(0)).map_err(|e|e.to_string())?;
            if !running{return Err("任务已取消或结束，结果未写入。".into());}
            super::automation::generation_run(&tx, id)?;
            let saved=append(&tx,&SaveWritingInput{project_id:input.project_id.clone(),chapter_id:input.chapter_id.clone(),kind:"publishing".into(),content,based_on:None,is_confirmed:false},"ai",model,PROMPT_VERSION,input.source_snapshot.as_ref())?;
            tx.execute(&format!("UPDATE ai_tasks SET status='succeeded',version_id=?2,completed_at={NOW} WHERE id=?1"),params![id,saved.id]).map_err(|e|e.to_string())?;
            tx.commit().map_err(|e|e.to_string())?;Ok(saved)
        })
    }
    pub fn finish_revision(
        &self,
        id: &str,
        input: &GenerationInput,
        content: WritingContent,
    ) -> Result<GenerationPreview, String> {
        content.validate("publishing", None)?;
        self.with_connection(|c| {
            let base = revision_base(c, input)?.ok_or("调整任务缺少来源版本。")?;
            let changed = c.execute(&format!("UPDATE ai_tasks SET status='succeeded',preview_json=?2,completed_at={NOW} WHERE id=?1 AND status='running'"),params![id,serde_json::to_string(&content).map_err(|e|e.to_string())?]).map_err(|e|e.to_string())?;
            if changed == 0 { return Err("任务已取消或结束，候选结果未保存。".into()); }
            Ok(GenerationPreview { task_id:id.into(), fields:input.revision.as_ref().unwrap().fields.clone(), before:base.content, content })
        })
    }
    pub fn apply_generation(&self, id: &str) -> Result<WritingVersion, String> {
        self.with_connection(|c| {
            let tx = c.transaction().map_err(|e|e.to_string())?;
            let (input, preview, saved, model, prompt): (String,Option<String>,Option<String>,String,String) = tx.query_row(
                "SELECT input_json,preview_json,version_id,model_name,prompt_version FROM ai_tasks WHERE id=?1 AND kind='publishing' AND status='succeeded'",[id],|r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?))
            ).map_err(|_|"没有可采用的候选结果。")?;
            let input:GenerationInput=serde_json::from_str(&input).map_err(|_|"调整任务无效。")?;
            let base=revision_base(&tx,&input)?.ok_or("此任务不是按项调整。")?;
            if let Some(saved)=saved { return version(&tx,&saved); }
            let content:WritingContent=serde_json::from_str(&preview.ok_or("候选结果已不可用。")?).map_err(|_|"候选结果无效。")?;
            let saved=append(&tx,&SaveWritingInput {
        chapter_id: input.chapter_id,
 project_id:input.project_id, kind:"publishing".into(),content,based_on:Some(base.id),is_confirmed:false },"mixed",&model,&prompt, input.source_snapshot.as_ref())?;
            tx.execute("UPDATE ai_tasks SET version_id=?2,preview_json=NULL WHERE id=?1",params![id,saved.id]).map_err(|e|e.to_string())?;
            tx.commit().map_err(|e|e.to_string())?;
            Ok(saved)
        })
    }
    pub fn end_generation(
        &self,
        id: &str,
        status: &str,
        code: &str,
        message: &str,
    ) -> Result<(), String> {
        self.with_connection(|c|{c.execute(&format!("UPDATE ai_tasks SET status=?2,error_code=?3,error_message=?4,completed_at={NOW} WHERE id=?1 AND status IN ('pending','running')"),params![id,status,code,message]).map_err(|e|e.to_string())?;Ok(())})
    }
    pub fn recover_ai_tasks(&self) -> Result<(), String> {
        self.with_connection(|c|{c.execute(&format!("UPDATE ai_tasks SET status='failed',error_code='interrupted',error_message='应用退出或数据库恢复中断了生成，请手动重试。',completed_at={NOW} WHERE status IN ('pending','running')"),[]).map_err(|e|e.to_string())?;Ok(())})
    }
}
