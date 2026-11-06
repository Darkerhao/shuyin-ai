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

pub const TRANSCRIPT_PROMPT: &str = "transcript-publishing-v1";

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
    pub selection: StartAutomationInput,
    pub title: String,
    pub author: String,
    pub target_duration_seconds: i64,
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
    pub landscape_task_id: Option<String>,
    pub portrait_task_id: Option<String>,
    pub landscape_cover_id: Option<String>,
    pub portrait_cover_id: Option<String>,
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
    LandscapeImage,
    PortraitImage,
    LandscapeCover,
    PortraitCover,
    Publish,
}
impl Slot {
    fn column(self) -> &'static str {
        match self {
            Self::Audio => "audio_job_id",
            Self::Video => "video_job_id",
            Self::Writing => "writing_task_id",
            Self::LandscapeImage => "landscape_task_id",
            Self::PortraitImage => "portrait_task_id",
            Self::LandscapeCover => "landscape_cover_id",
            Self::PortraitCover => "portrait_cover_id",
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
    pub fn binding(&self, slot: Slot) -> Binding<'_> {
        Binding {
            id: &self.id,
            run_id: self.run_id,
            slot,
        }
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
    connection.query_row("SELECT id,project_id,input_json,status,stage,run_id,error,audio_job_id,video_job_id,writing_task_id,landscape_task_id,portrait_task_id,landscape_cover_id,portrait_cover_id,publish_task_id,created_at,updated_at,publish_started FROM automation_runs WHERE id=?1", [id], |row| {
        let json: String = row.get(2)?;
        let input = serde_json::from_str(&json).map_err(|error| rusqlite::Error::FromSqlConversionFailure(2,rusqlite::types::Type::Text,Box::new(error)))?;
        Ok(AutomationRun { id:row.get(0)?,project_id:row.get(1)?,input,status:row.get(3)?,stage:row.get(4)?,run_id:row.get(5)?,error:row.get(6)?,audio_job_id:row.get(7)?,video_job_id:row.get(8)?,writing_task_id:row.get(9)?,landscape_task_id:row.get(10)?,portrait_task_id:row.get(11)?,landscape_cover_id:row.get(12)?,portrait_cover_id:row.get(13)?,publish_task_id:row.get(14)?,created_at:row.get(15)?,updated_at:row.get(16)?,publish_started:row.get(17)? })
    }).map_err(|error| format!("自动流程不可读取：{error}"))
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
        if !matches!(stage, "landscape_cover" | "portrait_cover") {
            return Err("封面阶段无效。".into());
        }
        self.with_connection(|connection| {
            let column = if stage=="landscape_cover" { "landscape_cover_id" } else { "portrait_cover_id" };
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
        let task = match run.stage.as_str() {
            "writing" => run.writing_task_id.as_deref(),
            "landscape_image" => run.landscape_task_id.as_deref(),
            "portrait_image" => run.portrait_task_id.as_deref(),
            _ => None,
        };
        self.with_connection(|connection| {
            if !matches!(run.status.as_str(),"paused"|"waiting_user") { return Err("请先等待流程结束当前操作。".into()); }
            if let Some(id) = task {
                let changed = connection.execute("UPDATE ai_tasks SET status='pending',error_code=NULL,error_message=NULL,completed_at=NULL WHERE id=?1 AND status IN ('failed','cancelled') AND version_id IS NULL AND asset_id IS NULL",[id]).map_err(|e|e.to_string())?;
                if changed != 1 { return Err("当前生成不需要重试。".into()); }
            } else if matches!(run.stage.as_str(),"video_prepare"|"render") {
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
            if transaction.query_row("SELECT EXISTS(SELECT 1 FROM audio_jobs WHERE status NOT IN ('succeeded','cancelled')) OR EXISTS(SELECT 1 FROM video_jobs WHERE status='running') OR EXISTS(SELECT 1 FROM automation_runs WHERE status NOT IN ('completed','cancelled'))",[],|row|row.get::<_,bool>(0)).map_err(|e|e.to_string())? { return Err("已有未完成的自动、音频或视频任务，请先处理原任务。".into()); }
            let source: String = transaction.query_row("SELECT file_path FROM assets WHERE id=?1 AND project_id=?2 AND asset_type='document'",params![input.source_asset_id,input.project_id],|row|row.get(0)).map_err(|_|"请选择本书已保存的原书文件。")?;
            if !super::audio::supported_source_name(Path::new(&source)) { return Err("自动制作支持 PDF、EPUB、TXT、Markdown 原书。".into()); }
            let (_, source_sha256) = super::audio::hash_file(Path::new(&source))?;
            let text = ai::provider(&transaction,&input.text_provider_id)?;
            let image = ai::provider(&transaction,&input.image_provider_id)?;
            if !text.input.is_enabled || text.input.model_kind != "text" || !image.input.is_enabled || image.input.model_kind != "image" { return Err("请配置可用的文本模型和图片模型。".into()); }
            let id = new_id(&transaction)?;
            let frozen = FrozenInput { title:book.input.title,author:book.input.author,target_duration_seconds:600,source_sha256,text_provider:text.input,image_provider:image.input,selection:input.clone(),account };
            transaction.execute(&format!("INSERT INTO automation_runs(id,request_key,project_id,input_json,status,stage,created_at,updated_at) VALUES(?1,?2,?3,?4,'running','audio',{NOW},{NOW})"),params![id,input.request_key,input.project_id,serde_json::to_string(&frozen).map_err(|e|e.to_string())?]).map_err(|e|e.to_string())?;
            transaction.commit().map_err(|e|e.to_string())?;
            load(connection,&id)
        })
    }
    pub fn verify_automation_input(&self, run: &AutomationRun) -> Result<(), String> {
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
                let current = ai::provider(connection, id)?;
                if serde_json::to_value(&current.input).map_err(|e| e.to_string())?
                    != serde_json::to_value(expected).map_err(|e| e.to_string())?
                {
                    return Err("流程使用的模型配置已变化，请恢复原配置后继续。".into());
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
        self.with_connection(|connection| {
            let id: String = connection
                .query_row(
                    "SELECT version_id FROM ai_tasks WHERE id=?1 AND status='succeeded'",
                    [run.writing_task_id.as_deref().ok_or("流程缺少文案任务。")?],
                    |row| row.get(0),
                )
                .map_err(|_| "流程文案尚未完成。")?;
            ai::version(connection, &id)
        })
    }
    pub fn create_transcript_task(&self, run: &AutomationRun) -> Result<String, String> {
        let video = self.video_job(run.video_job_id.as_deref().ok_or("缺少视频任务。")?)?;
        if video.project_id != run.project_id || video.plan.cues.is_empty() {
            return Err("实际音频字幕尚未就绪。".into());
        }
        let transcript = crate::video::to_srt(&video.plan.cues);
        if transcript.chars().count() > 80_000 {
            return Err(
                "本期字幕超出自动文案输入上限，请在内容创作中处理。完整字幕已保留。".into(),
            );
        }
        self.with_connection(|connection| {
            let transaction = connection.transaction().map_err(|e|e.to_string())?;
            let id = new_id(&transaction)?;
            let user = serde_json::json!({"bookTitle":run.input.title,"author":run.input.author,"transcript":transcript,"planSha256":plan_digest(&video.plan)?});
            transaction.execute(&format!("INSERT INTO ai_tasks(id,project_id,kind,provider_id,model_name,prompt_version,status,input_json,created_at) VALUES(?1,?2,'transcript_publishing',?3,?4,?5,'pending',?6,{NOW})"),params![id,run.project_id,run.input.selection.text_provider_id,run.input.text_provider.model_name,TRANSCRIPT_PROMPT,user.to_string()]).map_err(|e|e.to_string())?;
            bind(&transaction,&run.binding(Slot::Writing),&run.project_id,&id)?;
            transaction.commit().map_err(|e|e.to_string())?;
            Ok(id)
        })
    }
    pub fn verify_automation_video(&self, run: &AutomationRun) -> Result<(), String> {
        let video = self.video_job(run.video_job_id.as_deref().ok_or("缺少视频任务。")?)?;
        let expected: String = self.with_connection(|connection| {
            connection
                .query_row(
                    "SELECT json_extract(input_json,'$.planSha256') FROM ai_tasks WHERE id=?1",
                    [run.writing_task_id.as_deref().ok_or("缺少文案任务。")?],
                    |row| row.get(0),
                )
                .map_err(|e| e.to_string())
        })?;
        if expected != plan_digest(&video.plan)? {
            return Err("字幕或分镜已在生成文案后修改，请停止此流程并重新准备对应文案，避免文案与成片不一致。".into());
        }
        Ok(())
    }
    pub fn prepare_transcript_task(&self, id: &str) -> Result<String, String> {
        self.with_connection(|connection| {
            if connection.query_row("SELECT EXISTS(SELECT 1 FROM automation_runs WHERE writing_task_id=?1 AND status!='running')",[id],|row|row.get::<_,bool>(0)).map_err(|e|e.to_string())? { return Err("自动流程已暂停，文案请求尚未发送。".into()); }
            let input = connection.query_row("SELECT input_json FROM ai_tasks WHERE id=?1 AND kind='transcript_publishing' AND status='pending'",[id],|row|row.get(0)).map_err(|_|"文案任务已执行或结束，不能重复请求。")?;
            connection.execute("UPDATE ai_tasks SET status='running' WHERE id=?1",[id]).map_err(|e|e.to_string())?;
            Ok(input)
        })
    }
    pub fn finish_transcript_task(&self, id: &str, content: WritingContent) -> Result<(), String> {
        self.with_connection(|connection| {
            let transaction = connection.transaction().map_err(|e|e.to_string())?;
            let (project_id,model): (String,String) = transaction.query_row("SELECT project_id,model_name FROM ai_tasks WHERE id=?1 AND kind='transcript_publishing' AND status='running'",[id],|row|Ok((row.get(0)?,row.get(1)?))).map_err(|_|"文案任务已取消或结束。")?;
            let saved = ai::append(&transaction,&ai::SaveWritingInput { project_id,kind:"publishing".into(),content,based_on:None,is_confirmed:false },"ai",&model,TRANSCRIPT_PROMPT)?;
            transaction.execute(&format!("UPDATE ai_tasks SET status='succeeded',version_id=?1,completed_at={NOW} WHERE id=?2"),params![saved.id,id]).map_err(|e|e.to_string())?;
            transaction.commit().map_err(|e|e.to_string())
        })
    }
    pub fn automation_cover_config(&self, run: &AutomationRun) -> Result<CoverConfig, String> {
        let landscape = run.stage == "landscape_cover";
        if !landscape && run.stage != "portrait_cover" {
            return Err("流程不在封面导出阶段。".into());
        }
        let task_id = if landscape {
            &run.landscape_task_id
        } else {
            &run.portrait_task_id
        };
        let (status, _, asset, _) =
            self.automation_ai_result(task_id.as_deref().ok_or("背景任务尚未创建。")?)?;
        if status != "succeeded" {
            return Err("背景尚未生成成功。".into());
        }
        let WritingContent::Publishing {
            cover_title,
            cover_subtitle,
            cover_design,
            cover_prompt_landscape,
            cover_prompt_portrait,
            ..
        } = self.automation_writing(run)?.content
        else {
            return Err("文案类型无效。".into());
        };
        let design = cover_design.ok_or("缺少封面排版配置。")?;
        Ok(CoverConfig {
            schema_version: 1,
            template_id: "spotlight".into(),
            width: if landscape { 1440 } else { 1080 },
            height: if landscape { 1080 } else { 1440 },
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
            background_prompt: if landscape {
                cover_prompt_landscape
            } else {
                cover_prompt_portrait
            },
        })
    }
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
