use super::{
    projects::{get, new_id, NOW},
    Database,
};
use crate::{
    ai::{self, WritingContent, PROMPT_VERSION},
    credentials::Credentials,
};
use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderInput {
    pub name: String,
    pub base_url: String,
    pub model_name: String,
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
        },
        credential_ref: r.get(6)?,
        credential_status: "missing".into(),
    })
}
fn provider(c: &Connection, id: &str) -> Result<ProviderConfig, String> {
    c.query_row("SELECT id,name,base_url,model_name,is_default,is_enabled,credential_ref FROM model_providers WHERE id=?1", [id], row_provider)
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
const VERSION_COLUMNS: &str = "id,project_id,kind,version_no,content_json,source_type,model_name,prompt_version,is_confirmed,created_at";
fn version(c: &Connection, id: &str) -> Result<WritingVersion, String> {
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
    pub kind: String,
    pub content: WritingContent,
    pub based_on: Option<String>,
    pub is_confirmed: bool,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerationInput {
    pub project_id: String,
    pub provider_id: String,
    pub kind: String,
    pub count: usize,
    pub reference_version_id: Option<String>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiTask {
    pub id: String,
    pub kind: String,
    pub status: String,
    pub model_name: String,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub version_id: Option<String>,
    pub created_at: String,
}
const TASK_COLUMNS: &str =
    "id,kind,status,model_name,error_code,error_message,version_id,created_at";
fn row_task(r: &Row<'_>) -> rusqlite::Result<AiTask> {
    Ok(AiTask {
        id: r.get(0)?,
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
}

fn append(
    c: &Connection,
    input: &SaveWritingInput,
    source: &str,
    model: &str,
    prompt: &str,
) -> Result<WritingVersion, String> {
    input.content.validate(&input.kind, None)?;
    get(c, &input.project_id)?;
    let id = new_id(c)?;
    c.execute(&format!("INSERT INTO writing_versions(id,project_id,kind,version_no,content_json,source_type,model_name,prompt_version,is_confirmed,created_at)
        SELECT ?1,?2,?3,COALESCE(MAX(version_no),0)+1,?4,?5,?6,?7,?8,{NOW} FROM writing_versions WHERE project_id=?2 AND kind=?3"),
        params![id,input.project_id,input.kind,serde_json::to_string(&input.content).map_err(|e|e.to_string())?,source,model,prompt,input.is_confirmed]).map_err(|e|e.to_string())?;
    c.execute(
        &format!("UPDATE book_projects SET updated_at={NOW} WHERE id=?1"),
        [&input.project_id],
    )
    .map_err(|e| e.to_string())?;
    version(c, &id)
}
impl Database {
    pub fn list_providers(&self, vault: &impl Credentials) -> Result<Vec<ProviderConfig>, String> {
        self.with_connection(|c| {
            let mut q=c.prepare("SELECT id,name,base_url,model_name,is_default,is_enabled,credential_ref FROM model_providers ORDER BY is_default DESC,name,id").map_err(|e|e.to_string())?;
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
        input.model_name = input.model_name.trim().into();
        if input.name.is_empty()
            || input.name.chars().count() > 100
            || input.model_name.is_empty()
            || input.model_name.len() > 200
            || (input.is_default && !input.is_enabled)
        {
            return Err("请填写服务名称、模型名；默认模型必须启用。".into());
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
            let id=id.map(str::to_string).map(Ok).unwrap_or_else(||new_id(c))?;
            let new_ref=if api_key.is_some(){Some(format!("text-{}",new_id(c)?))}else{old.as_ref().and_then(|p|p.credential_ref.clone())};
            if let (Some(key),Some(reference))=(api_key,new_ref.as_deref()){vault.set(reference,key)?;}
            let saved=(|| {
                let tx=c.transaction().map_err(|e|e.to_string())?;
                if input.is_default {tx.execute("UPDATE model_providers SET is_default=0",[]).map_err(|e|e.to_string())?;}
                tx.execute(&format!("INSERT INTO model_providers(id,name,provider_type,base_url,model_name,is_default,is_enabled,credential_ref,created_at,updated_at)
                    VALUES(?1,?2,'openai-compatible',?3,?4,?5,?6,?7,{NOW},{NOW}) ON CONFLICT(id) DO UPDATE SET name=excluded.name,base_url=excluded.base_url,model_name=excluded.model_name,is_default=excluded.is_default,is_enabled=excluded.is_enabled,credential_ref=excluded.credential_ref,updated_at=excluded.updated_at"),
                    params![id,input.name,input.base_url,input.model_name,input.is_default,input.is_enabled,new_ref]).map_err(|e|e.to_string())?;
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
    ) -> Result<Vec<WritingVersion>, String> {
        ai::validate_kind(kind)?;
        self.with_connection(|c| {get(c,project_id)?;let mut q=c.prepare(&format!("SELECT {VERSION_COLUMNS} FROM writing_versions WHERE project_id=?1 AND kind=?2 ORDER BY version_no DESC")).map_err(|e|e.to_string())?;
            let rows=q.query_map(params![project_id,kind],row_version).map_err(|e|e.to_string())?; rows.collect::<rusqlite::Result<Vec<_>>>().map_err(|e|e.to_string())})
    }
    pub fn save_writing(&self, input: SaveWritingInput) -> Result<WritingVersion, String> {
        self.with_connection(|c| {
            let base = input
                .based_on
                .as_deref()
                .map(|s| version(c, s))
                .transpose()?;
            if base
                .as_ref()
                .is_some_and(|b| b.project_id != input.project_id || b.kind != input.kind)
            {
                return Err("来源版本不属于当前项目或内容类型。".into());
            }
            let source = if base.as_ref().is_some_and(|b| b.source_type != "manual") {
                "mixed"
            } else {
                "manual"
            };
            let tx = c.transaction().map_err(|e| e.to_string())?;
            let saved = append(
                &tx,
                &input,
                source,
                base.as_ref().map_or("", |b| &b.model_name),
                base.as_ref().map_or("", |b| &b.prompt_version),
            )?;
            tx.commit().map_err(|e| e.to_string())?;
            Ok(saved)
        })
    }
    pub fn list_ai_tasks(&self, project_id: &str) -> Result<Vec<AiTask>, String> {
        self.with_connection(|c| {let mut q=c.prepare(&format!("SELECT {TASK_COLUMNS} FROM ai_tasks WHERE project_id=?1 ORDER BY created_at DESC,id DESC LIMIT 30")).map_err(|e|e.to_string())?;
            let rows=q.query_map([project_id],row_task).map_err(|e|e.to_string())?; rows.collect::<rusqlite::Result<Vec<_>>>().map_err(|e|e.to_string())})
    }
    pub fn start_generation(&self, input: GenerationInput) -> Result<String, String> {
        ai::validate_kind(&input.kind)?;
        if !(1..=20).contains(&input.count) {
            return Err("生成数量应为 1–20。".into());
        }
        self.with_connection(|c| {
            let project=get(c,&input.project_id)?;
            if project.input.description.trim().is_empty() && project.input.source_notes.trim().is_empty(){return Err("请先保存书籍简介或读书笔记，再生成内容。".into());}
            let p=provider(c,&input.provider_id)?;
            if !p.input.is_enabled{return Err("所选模型已停用。".into());}
            if let Some(r)=&input.reference_version_id{if version(c,r)?.project_id!=input.project_id{return Err("参考版本不属于当前项目。".into());}}
            let id=new_id(c)?;
            c.execute(&format!("INSERT INTO ai_tasks(id,project_id,kind,provider_id,model_name,prompt_version,status,input_json,created_at) VALUES(?1,?2,?3,?4,?5,?6,'pending',?7,{NOW})"),params![id,input.project_id,input.kind,input.provider_id,p.input.model_name,PROMPT_VERSION,serde_json::to_string(&input).map_err(|e|e.to_string())?]).map_err(|e|e.to_string())?;
            Ok(id)
        })
    }
    pub fn prepare_generation(&self, id: &str) -> Result<PreparedGeneration, String> {
        self.with_connection(|c| {
            let json:String=c.query_row("SELECT input_json FROM ai_tasks WHERE id=?1 AND status='pending'",[id],|r|r.get(0)).map_err(|_|"任务已取消、已运行或不存在。")?;
            let input:GenerationInput=serde_json::from_str(&json).map_err(|_|"任务输入无效。")?;
            let project=get(c,&input.project_id)?;
            let p=provider(c,&input.provider_id)?;
            if !p.input.is_enabled{return Err("所选模型已停用。".into());}
            let reference=input.reference_version_id.as_deref().map(|s|version(c,s)).transpose()?.map(|v|v.content);
            let user=serde_json::json!({"task":input.kind,"count":input.count,"book":project.input,"reference":reference}).to_string();
            c.execute("UPDATE ai_tasks SET status='running',model_name=?2 WHERE id=?1 AND status='pending'",params![id,p.input.model_name]).map_err(|e|e.to_string())?;
            Ok(PreparedGeneration{input,provider:p,user})
        })
    }
    pub fn finish_generation(
        &self,
        id: &str,
        input: &GenerationInput,
        content: WritingContent,
        model: &str,
    ) -> Result<WritingVersion, String> {
        content.validate(&input.kind, Some(input.count))?;
        self.with_connection(|c|{
            let tx=c.transaction().map_err(|e|e.to_string())?;
            let running:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM ai_tasks WHERE id=?1 AND status='running')",[id],|r|r.get(0)).map_err(|e|e.to_string())?;
            if !running{return Err("任务已取消或结束，结果未写入。".into());}
            let saved=append(&tx,&SaveWritingInput{project_id:input.project_id.clone(),kind:input.kind.clone(),content,based_on:None,is_confirmed:false},"ai",model,PROMPT_VERSION)?;
            tx.execute(&format!("UPDATE ai_tasks SET status='succeeded',version_id=?2,completed_at={NOW} WHERE id=?1"),params![id,saved.id]).map_err(|e|e.to_string())?;
            tx.commit().map_err(|e|e.to_string())?;Ok(saved)
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
