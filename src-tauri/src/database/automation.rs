use super::{
    ai,
    covers::CoverConfig,
    projects::{get, new_id, NOW},
    publishing::PublishInput,
    Database,
};
use crate::ai::WritingContent;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;

pub const CONTENT_CHECK_PROMPT: &str = crate::ai::content_check::VERSION;

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StartAutomationInput {
    pub project_id: String,
    pub source_asset_id: String,
    pub text_provider_id: String,
    pub image_provider_id: String,
    pub format: String,
    pub request_key: String,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FrozenInput {
    #[serde(default)]
    pub workflow_version: u32,
    pub selection: StartAutomationInput,
    pub title: String,
    pub author: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub source_notes: String,
    pub target_duration_seconds: Option<i64>,
    #[serde(default)]
    pub duration_reason: String,
    pub source_sha256: String,
    pub text_provider: ai::ProviderInput,
    pub image_provider: ai::ProviderInput,
    pub account: crate::douyin::flow::Account,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AutomationRun {
    pub id: String,
    pub project_id: String,
    pub input: FrozenInput,
    pub status: String,
    pub stage: String,
    pub run_id: i64,
    pub error: String,
    pub audio_job_id: Option<String>,
    pub video_job_id: Option<String>,
    pub writing_task_id: Option<String>,
    pub duration_task_id: Option<String>,
    pub content_check_task_id: Option<String>,
    pub writing_version_id: Option<String>,
    pub final_writing_version_id: Option<String>,
    pub actual_duration_seconds: Option<f64>,
    pub landscape_task_id: Option<String>,
    pub portrait_task_id: Option<String>,
    pub widescreen_task_id: Option<String>,
    pub landscape_cover_id: Option<String>,
    pub portrait_cover_id: Option<String>,
    pub widescreen_cover_id: Option<String>,
    pub render_plan_sha256: Option<String>,
    pub publish_task_id: Option<String>,
    pub publish_started: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Copy)]
pub enum Slot {
    Audio,
    Video,
    Writing,
    Duration,
    ContentCheck,
    LandscapeImage,
    PortraitImage,
    LandscapeCover,
    PortraitCover,
    WidescreenImage,
    WidescreenCover,
    Publish,
}
impl Slot {
    fn column(self) -> &'static str {
        match self {
            Self::Audio => "audio_job_id",
            Self::Video => "video_job_id",
            Self::Writing => "writing_task_id",
            Self::Duration => "duration_task_id",
            Self::ContentCheck => "content_check_task_id",
            Self::LandscapeImage => "landscape_task_id",
            Self::PortraitImage => "portrait_task_id",
            Self::LandscapeCover => "landscape_cover_id",
            Self::PortraitCover => "portrait_cover_id",
            Self::WidescreenImage => "widescreen_task_id",
            Self::WidescreenCover => "widescreen_cover_id",
            Self::Publish => "publish_task_id",
        }
    }
}
pub struct Binding<'a> {
    pub id: &'a str,
    pub run_id: i64,
    pub slot: Slot,
}
impl AutomationRun {
    pub fn require_current_workflow(&self) -> Result<(), String> {
        if self.input.workflow_version != 2 {
            return Err("此任务使用旧版制作顺序，已有成果已保留。请在原音频、视频或发布详情继续处理，或停止旧流程后重新制作。".into());
        }
        Ok(())
    }

    pub fn child_id(&self, slot: Slot) -> Option<&str> {
        match slot {
            Slot::Audio => &self.audio_job_id,
            Slot::Video => &self.video_job_id,
            Slot::Writing => &self.writing_task_id,
            Slot::Duration => &self.duration_task_id,
            Slot::ContentCheck => &self.content_check_task_id,
            Slot::Publish => &self.publish_task_id,
            Slot::LandscapeImage => &self.landscape_task_id,
            Slot::PortraitImage => &self.portrait_task_id,
            Slot::WidescreenImage => &self.widescreen_task_id,
            Slot::LandscapeCover => &self.landscape_cover_id,
            Slot::PortraitCover => &self.portrait_cover_id,
            Slot::WidescreenCover => &self.widescreen_cover_id,
        }
        .as_deref()
    }

    pub fn binding(&self, slot: Slot) -> Binding<'_> {
        Binding {
            id: &self.id,
            run_id: self.run_id,
            slot,
        }
    }
}

pub struct CoverStep {
    pub image_stage: &'static str,
    pub cover_stage: &'static str,
    pub next: &'static str,
    pub width: u32,
    pub height: u32,
    pub image_slot: Slot,
    pub cover_slot: Slot,
}
pub const COVER_STEPS: [CoverStep; 3] = [
    CoverStep {
        image_stage: "landscape_image",
        cover_stage: "landscape_cover",
        next: "portrait_image",
        width: 1440,
        height: 1080,
        image_slot: Slot::LandscapeImage,
        cover_slot: Slot::LandscapeCover,
    },
    CoverStep {
        image_stage: "portrait_image",
        cover_stage: "portrait_cover",
        next: "widescreen_image",
        width: 1080,
        height: 1440,
        image_slot: Slot::PortraitImage,
        cover_slot: Slot::PortraitCover,
    },
    CoverStep {
        image_stage: "widescreen_image",
        cover_stage: "widescreen_cover",
        next: "storyboard",
        width: 1920,
        height: 1080,
        image_slot: Slot::WidescreenImage,
        cover_slot: Slot::WidescreenCover,
    },
];
pub fn cover_step(stage: &str) -> Option<&'static CoverStep> {
    COVER_STEPS
        .iter()
        .find(|step| stage == step.image_stage || stage == step.cover_stage)
}
impl CoverStep {
    pub fn prompt<'a>(&self, content: &'a WritingContent) -> Result<&'a str, String> {
        let WritingContent::Publishing {
            cover_prompt_landscape,
            cover_prompt_portrait,
            cover_prompt_widescreen,
            ..
        } = content
        else {
            return Err("文案类型无效。".into());
        };
        Ok(match self.image_slot {
            Slot::LandscapeImage => cover_prompt_landscape,
            Slot::PortraitImage => cover_prompt_portrait,
            _ => cover_prompt_widescreen,
        })
    }
}

pub(super) fn bind(
    connection: &Connection,
    binding: &Binding<'_>,
    project_id: &str,
    child: &str,
) -> Result<(), String> {
    let column = binding.slot.column();
    let count = connection.execute(&format!("UPDATE automation_runs SET {column}=?1,updated_at={NOW} WHERE id=?2 AND project_id=?3 AND run_id=?4 AND status='running' AND {column} IS NULL"), params![child,binding.id,project_id,binding.run_id]).map_err(|e| e.to_string())?;
    if count != 1 {
        return Err("自动流程已暂停、结束或该阶段已有任务。".into());
    }
    Ok(())
}

fn load(connection: &Connection, id: &str) -> Result<AutomationRun, String> {
    connection.query_row("SELECT id,project_id,input_json,status,stage,run_id,error,audio_job_id,video_job_id,writing_task_id,landscape_task_id,portrait_task_id,landscape_cover_id,portrait_cover_id,publish_task_id,created_at,updated_at,publish_started,duration_task_id,content_check_task_id,(SELECT version_id FROM ai_tasks WHERE id=automation_runs.writing_task_id AND status='succeeded'),widescreen_task_id,widescreen_cover_id,render_plan_sha256,(SELECT version_id FROM ai_tasks WHERE id=automation_runs.content_check_task_id AND status='succeeded'),(SELECT NULLIF(json_extract(plan_json,'$.duration'),0) FROM video_jobs WHERE id=automation_runs.video_job_id) FROM automation_runs WHERE id=?1", [id], |row| {
        let json: String = row.get(2)?;
        let input = serde_json::from_str(&json).map_err(|error| rusqlite::Error::FromSqlConversionFailure(2,rusqlite::types::Type::Text,Box::new(error)))?;
        Ok(AutomationRun { id:row.get(0)?,project_id:row.get(1)?,input,status:row.get(3)?,stage:row.get(4)?,run_id:row.get(5)?,error:row.get(6)?,audio_job_id:row.get(7)?,video_job_id:row.get(8)?,writing_task_id:row.get(9)?,landscape_task_id:row.get(10)?,portrait_task_id:row.get(11)?,landscape_cover_id:row.get(12)?,portrait_cover_id:row.get(13)?,publish_task_id:row.get(14)?,created_at:row.get(15)?,updated_at:row.get(16)?,publish_started:row.get(17)?,duration_task_id:row.get(18)?,content_check_task_id:row.get(19)?,writing_version_id:row.get(20)?,widescreen_task_id:row.get(21)?,widescreen_cover_id:row.get(22)?,render_plan_sha256:row.get(23)?,final_writing_version_id:row.get(24)?,actual_duration_seconds:row.get(25)? })
    }).map_err(|error| format!("自动流程不可读取：{error}"))
}

pub(super) fn generation_run(
    connection: &Connection,
    task_id: &str,
) -> Result<Option<AutomationRun>, String> {
    let id: Option<String> = connection
        .query_row(
            "SELECT id FROM automation_runs WHERE duration_task_id=?1 OR writing_task_id=?1",
            [task_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    id.map(|id| {
        let run = load(connection, &id)?;
        run.require_current_workflow()?;
        if matches!(run.status.as_str(), "cancelled" | "completed") {
            return Err("自动流程已结束，生成结果未采用。".into());
        }
        Ok(run)
    })
    .transpose()
}

pub(super) fn validate_target(seconds: i64) -> Result<(), String> {
    crate::ai::audio::validate_duration(seconds)?;
    if seconds > 900 {
        return Err("自动制作的目标时长须为1—15分钟，请先调整并保存目标时长。".into());
    }
    Ok(())
}

fn task_version(
    connection: &Connection,
    task_id: Option<&str>,
) -> Result<ai::WritingVersion, String> {
    let id: String = connection
        .query_row(
            "SELECT version_id FROM ai_tasks WHERE id=?1 AND status='succeeded'",
            [task_id.ok_or("流程缺少文案任务。")?],
            |row| row.get(0),
        )
        .map_err(|_| "流程文案尚未完成。")?;
    ai::version(connection, &id)
}

pub(super) fn recover(connection: &Connection) -> Result<(), String> {
    connection.execute(&format!("UPDATE automation_runs SET status='paused',run_id=run_id+1,error='应用已重开或恢复备份。请核对原任务后继续，已有音频和网页上传不会自动重发。',updated_at={NOW} WHERE status IN ('running','waiting_user','awaiting_publish')"), []).map_err(|e| e.to_string())?;
    Ok(())
}

pub(super) fn policy_matches(
    connection: &Connection,
    task_id: &str,
    input: &PublishInput,
) -> Result<bool, String> {
    let frozen: Option<String> = connection
        .query_row(
            "SELECT publish_input_json FROM automation_runs WHERE publish_task_id=?1",
            [task_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?
        .flatten();
    frozen
        .map(|json| {
            serde_json::from_str::<PublishInput>(&json)
                .map(|saved| saved == *input)
                .map_err(|e| e.to_string())
        })
        .unwrap_or(Ok(false))
}

impl Database {
    pub fn fail_automation_cover(&self, id: &str, run_id: i64, stage: &str) -> Result<(), String> {
        if !cover_step(stage).is_some_and(|step| step.cover_stage == stage) {
            return Err("封面阶段无效。".into());
        }
        self.with_connection(|connection| {
            let column = cover_step(stage).ok_or("封面阶段无效。")?.cover_slot.column();
            connection.execute(&format!("UPDATE automation_runs SET status='waiting_user',error='封面排版或导出失败，请检查图片和字体后继续；已生成背景保留，不会重复收费。',updated_at={NOW} WHERE id=?1 AND run_id=?2 AND stage=?3 AND status='running' AND {column} IS NULL"),params![id,run_id,stage]).map_err(|e|e.to_string())?;
            Ok(())
        })
    }
    pub fn mark_automation_upload(&self, run: &AutomationRun) -> Result<(), String> {
        self.with_connection(|connection| {
            let changed = connection.execute(&format!("UPDATE automation_runs SET publish_started=1,updated_at={NOW} WHERE id=?1 AND run_id=?2 AND status='running' AND publish_started=0"),params![run.id,run.run_id]).map_err(|e|e.to_string())?;
            if changed != 1 { return Err("上传已经开始或流程已暂停，请先核对原网页任务。".into()); }
            Ok(())
        })
    }
    pub fn is_automatic_publish(&self, task_id: &str) -> Result<bool, String> {
        self.with_connection(|connection| {
            connection
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM automation_runs WHERE publish_task_id=?1)",
                    [task_id],
                    |row| row.get(0),
                )
                .map_err(|e| e.to_string())
        })
    }
    pub fn next_automation_schedule(&self) -> Result<String, String> {
        self.with_connection(|connection| next_schedule(connection, "now"))
    }
    pub fn retry_automation_generation(&self, run: &AutomationRun) -> Result<(), String> {
        run.require_current_workflow()?;
        let task = match run.stage.as_str() {
            "duration" => run.duration_task_id.as_deref(),
            "writing" => run.writing_task_id.as_deref(),
            "content_check" => run.content_check_task_id.as_deref(),
            "landscape_image" => run.landscape_task_id.as_deref(),
            "portrait_image" => run.portrait_task_id.as_deref(),
            "widescreen_image" => run.widescreen_task_id.as_deref(),
            _ => None,
        };
        self.with_connection(|connection| {
            if !matches!(run.status.as_str(),"paused"|"waiting_user") { return Err("请先等待流程结束当前操作。".into()); }
            if let Some(id) = task {
                let changed = connection.execute("UPDATE ai_tasks SET status='pending',error_code=NULL,error_message=NULL,completed_at=NULL WHERE id=?1 AND status IN ('failed','cancelled') AND version_id IS NULL AND asset_id IS NULL",[id]).map_err(|e|e.to_string())?;
                if changed != 1 { return Err("当前生成不需要重试。".into()); }
            } else if matches!(run.stage.as_str(),"transcript"|"storyboard"|"render") {
                let changed = connection.execute("UPDATE video_jobs SET status=?1,error='' WHERE id=?2 AND status IN ('failed','cancelled')",params![if run.stage=="render" {"awaiting_confirmation"}else{"draft"},run.video_job_id]).map_err(|e|e.to_string())?;
                if changed != 1 { return Err("当前视频不需要重试。".into()); }
            } else { return Err("请在原业务页面处理当前问题后继续。".into()); }
            Ok(())
        })
    }
    pub fn automation_run(&self, id: &str) -> Result<AutomationRun, String> {
        self.with_connection(|connection| load(connection, id))
    }
    pub fn automation_runs(&self) -> Result<Vec<AutomationRun>, String> {
        self.with_connection(|connection| {
            let mut query = connection
                .prepare("SELECT id FROM automation_runs ORDER BY created_at DESC,id DESC LIMIT 30")
                .map_err(|e| e.to_string())?;
            let ids = query
                .query_map([], |row| row.get::<_, String>(0))
                .map_err(|e| e.to_string())?
                .collect::<rusqlite::Result<Vec<_>>>()
                .map_err(|e| e.to_string())?;
            ids.iter().map(|id| load(connection, id)).collect()
        })
    }
    pub fn create_automation(
        &self,
        input: StartAutomationInput,
        account: crate::douyin::flow::Account,
    ) -> Result<AutomationRun, String> {
        if input.request_key.trim().is_empty()
            || input.request_key.len() > 100
            || !matches!(input.format.as_str(), "deep_dive" | "brief")
        {
            return Err("自动任务标识或音频形式无效。".into());
        }
        self.with_connection(|connection| {
            if let Some(id) = connection.query_row("SELECT id FROM automation_runs WHERE request_key=?1",[&input.request_key],|row|row.get::<_,String>(0)).optional().map_err(|e|e.to_string())? {
                let run = load(connection,&id)?;
                if serde_json::to_value(&run.input.selection).map_err(|e|e.to_string())? != serde_json::to_value(&input).map_err(|e|e.to_string())? { return Err("重复启动标识对应其他输入。".into()); }
                return Ok(run);
            }
            let transaction = connection.transaction().map_err(|e|e.to_string())?;
            let book = get(&transaction,&input.project_id)?;
            if book.input.author.trim().is_empty() { return Err("自动制作请先填写作者，避免同名书混淆。".into()); }
            if transaction.query_row("SELECT EXISTS(SELECT 1 FROM ai_tasks WHERE project_id=?1 AND status IN ('pending','running'))",[&input.project_id],|row|row.get::<_,bool>(0)).map_err(|e|e.to_string())? { return Err("本书还有正在生成的内容，请先等待完成或取消，再开始自动制作。".into()); }
            if transaction.query_row("SELECT EXISTS(SELECT 1 FROM audio_jobs WHERE status NOT IN ('succeeded','cancelled')) OR EXISTS(SELECT 1 FROM video_jobs WHERE status='running') OR EXISTS(SELECT 1 FROM automation_runs WHERE status NOT IN ('completed','cancelled'))",[],|row|row.get::<_,bool>(0)).map_err(|e|e.to_string())? { return Err("已有未完成的自动、音频或视频任务，请先处理原任务。".into()); }
            let source: String = transaction.query_row("SELECT file_path FROM assets WHERE id=?1 AND project_id=?2 AND asset_type='document'",params![input.source_asset_id,input.project_id],|row|row.get(0)).map_err(|_|"请选择本书已保存的原书文件。")?;
            if !super::audio::supported_source_name(Path::new(&source)) { return Err("自动制作支持 PDF、EPUB、TXT、Markdown 原书。".into()); }
            let (_, source_sha256) = super::audio::hash_file(Path::new(&source))?;
            let text = ai::provider(&transaction,&input.text_provider_id)?;
            let image = ai::provider(&transaction,&input.image_provider_id)?;
            if !text.input.is_enabled || text.input.model_kind != "text" || !image.input.is_enabled || image.input.model_kind != "image" { return Err("请配置可用的文本模型和图片模型。".into()); }
            let id = new_id(&transaction)?;
            let target_duration_seconds = book.audio_duration_source.as_ref().map(|_| book.input.target_duration_seconds);
            if let Some(seconds) = target_duration_seconds { validate_target(seconds)?; }
            let frozen = FrozenInput { workflow_version:2,title:book.input.title,author:book.input.author,description:book.input.description,source_notes:book.input.source_notes,target_duration_seconds,duration_reason:if target_duration_seconds.is_some() {"使用本书已保存的目标时长".into()} else {String::new()},source_sha256,text_provider:text.input,image_provider:image.input,selection:input.clone(),account };
            transaction.execute(&format!("INSERT INTO automation_runs(id,request_key,project_id,input_json,status,stage,created_at,updated_at) VALUES(?1,?2,?3,?4,'running',?5,{NOW},{NOW})"),params![id,input.request_key,input.project_id,serde_json::to_string(&frozen).map_err(|e|e.to_string())?,if target_duration_seconds.is_some() {"writing"} else {"duration"}]).map_err(|e|e.to_string())?;
            transaction.commit().map_err(|e|e.to_string())?;
            load(connection,&id)
        })
    }
    pub fn verify_automation_input(&self, run: &AutomationRun) -> Result<(), String> {
        run.require_current_workflow()?;
        self.with_connection(|connection| {
            let book = get(connection, &run.project_id)?;
            if book.input.title != run.input.title || book.input.author != run.input.author {
                return Err("书名或作者已变化，请停止本流程并重新开始。".into());
            }
            for (id, expected) in [
                (
                    &run.input.selection.text_provider_id,
                    &run.input.text_provider,
                ),
                (
                    &run.input.selection.image_provider_id,
                    &run.input.image_provider,
                ),
            ] {
                let current = ai::provider(connection, id)?.input;
                if current.model_kind != expected.model_kind
                    || current.base_url != expected.base_url
                    || current.model_name != expected.model_name
                    || current.reasoning_effort != expected.reasoning_effort
                    || current.is_enabled != expected.is_enabled
                {
                    return Err("流程使用的模型调用配置已变化，请恢复原配置后继续。".into());
                }
            }
            Ok(())
        })
    }
    pub fn automation_publish_account(
        &self,
        task_id: &str,
    ) -> Result<Option<crate::douyin::flow::Account>, String> {
        self.with_connection(|connection| {
            let id: Option<String> = connection
                .query_row(
                    "SELECT id FROM automation_runs WHERE publish_task_id=?1",
                    [task_id],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|e| e.to_string())?;
            id.map(|id| load(connection, &id).map(|run| run.input.account))
                .transpose()
        })
    }
    pub fn set_automation_stage(&self, run: &AutomationRun, stage: &str) -> Result<(), String> {
        self.with_connection(|connection| {
            let changed = connection.execute(&format!("UPDATE automation_runs SET stage=?1,error='',updated_at={NOW} WHERE id=?2 AND run_id=?3 AND status='running'"),params![stage,run.id,run.run_id]).map_err(|e|e.to_string())?;
            if changed != 1 { return Err("自动流程已暂停或结束。".into()); }
            Ok(())
        })
    }
    pub fn set_automation_status(
        &self,
        run: &AutomationRun,
        status: &str,
        message: &str,
    ) -> Result<(), String> {
        if !matches!(status, "paused" | "waiting_user" | "awaiting_publish") {
            return Err("自动流程状态无效。".into());
        }
        self.with_connection(|connection| {
            connection.execute(&format!("UPDATE automation_runs SET status=?1,error=?2,updated_at={NOW} WHERE id=?3 AND run_id=?4 AND status='running'"),params![status,message,run.id,run.run_id]).map_err(|e|e.to_string())?;
            Ok(())
        })
    }
    pub fn control_automation(&self, id: &str, action: &str) -> Result<AutomationRun, String> {
        self.with_connection(|connection| {
            let run = load(connection,id)?;
            if matches!(run.status.as_str(),"completed"|"cancelled") { return Err("该流程已经结束。".into()); }
            if action == "resume" { run.require_current_workflow()?; }
            let status = match action { "pause" => "paused", "resume" if run.status != "running" => "running", "cancel" => "cancelled", _ => return Err("当前流程不支持此操作。".into()) };
            connection.execute(&format!("UPDATE automation_runs SET status=?1,run_id=run_id+1,error='',updated_at={NOW} WHERE id=?2"),params![status,id]).map_err(|e|e.to_string())?;
            load(connection,id)
        })
    }
    pub fn recover_automation(&self) -> Result<(), String> {
        self.with_connection(|connection| recover(connection))
    }
    pub fn automation_ai_result(
        &self,
        id: &str,
    ) -> Result<(String, Option<String>, Option<String>, String), String> {
        self.with_connection(|connection|connection.query_row("SELECT status,version_id,asset_id,COALESCE(error_message,'') FROM ai_tasks WHERE id=?1",[id],|row|Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?))).map_err(|e|e.to_string()))
    }
    pub fn automation_writing(&self, run: &AutomationRun) -> Result<ai::WritingVersion, String> {
        self.with_connection(|connection| task_version(connection, run.writing_task_id.as_deref()))
    }
    pub fn automation_publish_writing(
        &self,
        run: &AutomationRun,
    ) -> Result<ai::WritingVersion, String> {
        self.with_connection(|connection| {
            task_version(connection, run.content_check_task_id.as_deref())
        })
    }
    pub fn start_automation_generation(&self, run: &AutomationRun) -> Result<String, String> {
        run.require_current_workflow()?;
        let recommend_duration = run.stage == "duration";
        if !recommend_duration && run.stage != "writing" {
            return Err("流程不在前置创作阶段。".into());
        }
        if !recommend_duration {
            validate_target(
                run.input
                    .target_duration_seconds
                    .ok_or("目标时长尚未确定。")?,
            )?;
        }
        self.start_bound_generation(
            ai::GenerationInput {
                project_id: run.project_id.clone(),
                provider_id: run.input.selection.text_provider_id.clone(),
                revision: None,
                target_duration_seconds: run.input.target_duration_seconds,
                recommend_duration,
            },
            Some(&run.binding(if recommend_duration {
                Slot::Duration
            } else {
                Slot::Writing
            })),
        )
    }
    pub fn create_content_check_task(&self, run: &AutomationRun) -> Result<String, String> {
        run.require_current_workflow()?;
        self.with_connection(|connection| {
            let tx = connection.transaction().map_err(|e|e.to_string())?;
            let current = load(&tx,&run.id)?;
            if current.stage != "content_check" { return Err("流程不在内容校核阶段。".into()); }
            let video = content_video(&tx,&current)?;
            let version = task_version(&tx,current.writing_task_id.as_deref())?;
            let transcript = crate::video::to_srt(&video.plan.cues);
            if transcript.chars().count() > 80_000 { return Err("完整字幕超过8万字，已保留原稿和音频，请处理后再继续。".into()); }
            let user = serde_json::json!({"bookTitle":run.input.title,"author":run.input.author,"sourceVersionId":version.id,"currentContent":version.content,"audioJobId":current.audio_job_id,"audioAssetId":video.audio_asset_id,"videoJobId":video.id,"transcript":transcript,"transcriptSha256":content_digest(&video.plan)?});
            let id = new_id(&tx)?;
            tx.execute(&format!("INSERT INTO ai_tasks(id,project_id,kind,provider_id,model_name,prompt_version,status,input_json,created_at) VALUES(?1,?2,'content_check',?3,?4,?5,'pending',?6,{NOW})"),params![id,run.project_id,run.input.selection.text_provider_id,run.input.text_provider.model_name,CONTENT_CHECK_PROMPT,user.to_string()]).map_err(|e|e.to_string())?;
            bind(&tx,&run.binding(Slot::ContentCheck),&run.project_id,&id)?;
            tx.commit().map_err(|e|e.to_string())?;
            Ok(id)
        })
    }
    pub fn verify_automation_content(&self, run: &AutomationRun) -> Result<(), String> {
        self.with_connection(|c| {
            let current = load(c, &run.id)?;
            verify_content(c, &current)?;
            task_version(c, current.content_check_task_id.as_deref())?;
            Ok(())
        })
    }
    pub fn prepare_automation_render(&self, run: &AutomationRun) -> Result<(), String> {
        self.verify_automation_content(run)?;
        for step in &COVER_STEPS {
            self.read_cover_export(
                &run.project_id,
                run.child_id(step.cover_slot)
                    .ok_or("三尺寸封面尚未全部导出。")?,
            )?;
        }
        self.with_connection(|c| {
            let current=load(c,&run.id)?;
            verify_content(c,&current)?;
            let video=super::video::get(c,current.video_job_id.as_deref().ok_or("缺少视频任务。")?)?;
            crate::video::validate_scenes(&video.plan.scenes,0,video.plan.cues.len())?;
            if video.status != "awaiting_confirmation" { return Err("分镜尚未准备完成。".into()); }
            let count=c.execute(&format!("UPDATE automation_runs SET render_plan_sha256=?1,stage='render',updated_at={NOW} WHERE id=?2 AND run_id=?3 AND status='running' AND stage='storyboard'"),params![plan_digest(&video.plan)?,run.id,run.run_id]).map_err(|e|e.to_string())?;
            if count!=1 { return Err("流程已变化，未开始渲染。".into()); }
            Ok(())
        })
    }
    pub fn verify_automation_video(&self, run: &AutomationRun) -> Result<(), String> {
        self.with_connection(|c| {
            let current = load(c, &run.id)?;
            verify_content(c, &current)?;
            task_version(c, current.content_check_task_id.as_deref())?;
            let video =
                super::video::get(c, current.video_job_id.as_deref().ok_or("缺少视频任务。")?)?;
            if current.render_plan_sha256.as_deref() != Some(plan_digest(&video.plan)?.as_str()) {
                return Err("分镜与本次渲染快照不一致，请停止流程后重新准备成片。".into());
            }
            Ok(())
        })
    }
    pub fn prepare_content_check_task(&self, id: &str) -> Result<String, String> {
        self.with_connection(|c| {
            let run_id:String=c.query_row("SELECT id FROM automation_runs WHERE content_check_task_id=?1",[id],|row|row.get(0)).map_err(|e|e.to_string())?;
            let run=load(c,&run_id)?;
            if run.status!="running" { return Err("自动流程已暂停，校核尚未发送。".into()); }
            verify_content(c,&run)?;
            let input=c.query_row("SELECT input_json FROM ai_tasks WHERE id=?1 AND kind='content_check' AND status='pending'",[id],|row|row.get(0)).map_err(|_|"内容校核已执行，不能重复请求。")?;
            c.execute("UPDATE ai_tasks SET status='running' WHERE id=?1",[id]).map_err(|e|e.to_string())?;
            Ok(input)
        })
    }
    pub fn finish_content_check_task(&self, id: &str, raw: &str) -> Result<(), String> {
        self.with_connection(|connection| {
            let tx=connection.transaction().map_err(|e|e.to_string())?;
            let run_id:String=tx.query_row("SELECT id FROM automation_runs WHERE content_check_task_id=?1 AND status NOT IN ('cancelled','completed')",[id],|row|row.get(0)).map_err(|_|"流程已结束，校核结果未采用。")?;
            let run=load(&tx,&run_id)?;
            verify_content(&tx,&run)?;
            let (model,input):(String,String)=tx.query_row("SELECT model_name,input_json FROM ai_tasks WHERE id=?1 AND kind='content_check' AND status='running'",[id],|row|Ok((row.get(0)?,row.get(1)?))).map_err(|_|"校核任务已取消或结束。")?;
            let input:serde_json::Value=serde_json::from_str(&input).map_err(|e|e.to_string())?;
            let base=ai::version(&tx,input["sourceVersionId"].as_str().ok_or("缺少创作版本。")?)?;
            let content=crate::ai::content_check::apply(raw,&base.content,&run.input.title,run.input.target_duration_seconds.ok_or("缺少目标时长。")?)?;
            let version = if content == base.content { base.id } else {
                ai::append(&tx,&ai::SaveWritingInput { project_id:run.project_id,kind:"publishing".into(),content,based_on:Some(base.id),is_confirmed:false },"ai",&model,CONTENT_CHECK_PROMPT)?.id
            };
            tx.execute(&format!("UPDATE ai_tasks SET status='succeeded',version_id=?1,preview_json=?2,completed_at={NOW} WHERE id=?3"),params![version,raw,id]).map_err(|e|e.to_string())?;
            tx.commit().map_err(|e|e.to_string())
        })
    }
    pub fn automation_cover_config(&self, run: &AutomationRun) -> Result<CoverConfig, String> {
        let step = cover_step(&run.stage)
            .filter(|step| step.cover_stage == run.stage)
            .ok_or("流程不在封面导出阶段。")?;
        self.verify_automation_content(run)?;
        let task_id = run.child_id(step.image_slot);
        let (status, _, asset, _) =
            self.automation_ai_result(task_id.ok_or("背景任务尚未创建。")?)?;
        if status != "succeeded" {
            return Err("背景尚未生成成功。".into());
        }
        let content = self.automation_publish_writing(run)?.content;
        let background_prompt = step.prompt(&content)?.to_owned();
        let WritingContent::Publishing {
            cover_title,
            cover_subtitle,
            cover_design,
            ..
        } = content
        else {
            return Err("文案类型无效。".into());
        };
        let design = cover_design.ok_or("缺少封面排版配置。")?;
        Ok(CoverConfig {
            schema_version: 1,
            template_id: "spotlight".into(),
            width: step.width,
            height: step.height,
            title: cover_title,
            subtitle: cover_subtitle,
            brand: String::new(),
            font_family: "sans".into(),
            title_size: design.title_size,
            subtitle_size: design.subtitle_size,
            align: "center".into(),
            position: "center".into(),
            background_color: design.background_color,
            text_color: design.text_color,
            subtitle_color: Some(design.subtitle_color),
            accent_color: design.accent_color,
            background_asset_id: Some(asset.ok_or("背景素材缺失。")?),
            overlay_opacity: design.overlay_opacity,
            background_x: 0.5,
            background_y: 0.5,
            background_zoom: 1.0,
            background_prompt,
        })
    }
}

fn content_video(c: &Connection, run: &AutomationRun) -> Result<super::video::VideoJob, String> {
    let video = super::video::get(c, run.video_job_id.as_deref().ok_or("缺少视频任务。")?)?;
    let audio = super::audio::job(c, run.audio_job_id.as_deref().ok_or("缺少音频任务。")?)?;
    let base = task_version(c, run.writing_task_id.as_deref())?;
    let WritingContent::Publishing { audio_prompt, .. } = &base.content else {
        return Err("前置创作版本类型无效。".into());
    };
    let actual_prompt: String = c
        .query_row(
            "SELECT json_extract(input_json,'$.prompt') FROM audio_jobs WHERE id=?1",
            [&audio.id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    if audio.writing_version_id.as_deref() != Some(base.id.as_str())
        || &actual_prompt != audio_prompt
        || Some(audio.target_duration_seconds) != run.input.target_duration_seconds
    {
        return Err("实际音频指导或目标与前置方案不一致，请检查原音频任务。".into());
    }
    crate::video::validate_cues(&video.plan.cues, video.plan.duration)?;
    if video.project_id != run.project_id
        || audio.project_id != run.project_id
        || audio.status != "succeeded"
        || video.audio_asset_id.is_none()
        || video.audio_asset_id != audio.asset_id
        || video.plan.audio_sha256.is_empty()
        || Some(&video.plan.audio_sha256) != audio.output_sha256.as_ref()
        || video.plan.duration > 900.0
        || video
            .plan
            .subtitle_review
            .as_ref()
            .is_none_or(|r| r.completed != video.plan.cues.len())
    {
        return Err("实际音频与完整校对字幕尚未通过检查，已有成果保留。".into());
    }
    Ok(video)
}
fn verify_content(c: &Connection, run: &AutomationRun) -> Result<(), String> {
    let video = content_video(c, run)?;
    let json: String = c
        .query_row(
            "SELECT input_json FROM ai_tasks WHERE id=?1",
            [run.content_check_task_id
                .as_deref()
                .ok_or("缺少内容校核任务。")?],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    let input: serde_json::Value = serde_json::from_str(&json).map_err(|e| e.to_string())?;
    let base = task_version(c, run.writing_task_id.as_deref())?;
    if input["transcriptSha256"].as_str() != Some(content_digest(&video.plan)?.as_str())
        || input["sourceVersionId"].as_str() != Some(base.id.as_str())
        || input["audioJobId"].as_str() != run.audio_job_id.as_deref()
        || input["audioAssetId"].as_str() != video.audio_asset_id.as_deref()
        || input["videoJobId"].as_str() != Some(video.id.as_str())
    {
        return Err("音频、字幕或来源版本在校核后变化，请停止流程并重新准备对应材料。".into());
    }
    Ok(())
}
fn content_digest(plan: &crate::video::VideoPlan) -> Result<String, String> {
    let input = serde_json::json!({"audioSha256":plan.audio_sha256,"duration":plan.duration,"cues":plan.cues});
    Ok(
        Sha256::digest(serde_json::to_vec(&input).map_err(|e| e.to_string())?)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect(),
    )
}

fn plan_digest(plan: &crate::video::VideoPlan) -> Result<String, String> {
    Ok(
        Sha256::digest(serde_json::to_vec(plan).map_err(|e| e.to_string())?)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
    )
}

pub(super) fn next_schedule(connection: &Connection, now: &str) -> Result<String, String> {
    connection.query_row("SELECT strftime('%Y-%m-%dT01:11:00+08:00',?1,'+8 hours',CASE WHEN time(?1,'+8 hours') < '01:11:00' THEN '+0 days' ELSE '+1 day' END)",[now],|row|row.get(0)).map_err(|e|e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn next_schedule_uses_beijing_across_day_month_year_boundaries() {
        let connection = Connection::open_in_memory().unwrap();
        for (now, expected) in [
            ("2026-11-07T00:30:00+08:00", "2026-11-07T01:11:00+08:00"),
            ("2026-11-07T01:11:00+08:00", "2026-11-08T01:11:00+08:00"),
            ("2026-11-07T10:00:00+08:00", "2026-11-08T01:11:00+08:00"),
            ("2026-12-31T18:00:00Z", "2027-01-02T01:11:00+08:00"),
            ("2026-11-30T17:10:00Z", "2026-12-01T01:11:00+08:00"),
        ] {
            assert_eq!(next_schedule(&connection, now).unwrap(), expected);
        }
    }
}
