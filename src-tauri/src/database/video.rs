use super::{
    assets,
    projects::{self, NOW},
    Database,
};
use crate::video::{
    validate_cues, validate_scenes, Cue, Scene, VideoPlan, VideoSettings, VideoWork,
};
use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const COLUMNS: &str = "id,project_id,audio_asset_id,source_subtitle_id,provider_id,status,stage,run_id,plan_json,error,subtitle_asset_id,video_asset_id,created_at,updated_at";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoJob {
    pub id: String,
    pub project_id: String,
    pub audio_asset_id: Option<String>,
    pub source_subtitle_id: Option<String>,
    pub provider_id: Option<String>,
    pub status: String,
    pub stage: String,
    pub run_id: i64,
    pub plan: VideoPlan,
    pub error: String,
    pub subtitle_asset_id: Option<String>,
    pub video_asset_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StartVideoInput {
    pub project_id: String,
    pub audio_asset_id: String,
    pub subtitle_asset_id: Option<String>,
    pub provider_id: String,
}

fn row(r: &Row<'_>) -> rusqlite::Result<VideoJob> {
    let raw: String = r.get(8)?;
    let plan = serde_json::from_str(&raw).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(8, rusqlite::types::Type::Text, Box::new(e))
    })?;
    Ok(VideoJob {
        id: r.get(0)?,
        project_id: r.get(1)?,
        audio_asset_id: r.get(2)?,
        source_subtitle_id: r.get(3)?,
        provider_id: r.get(4)?,
        status: r.get(5)?,
        stage: r.get(6)?,
        run_id: r.get(7)?,
        plan,
        error: r.get(9)?,
        subtitle_asset_id: r.get(10)?,
        video_asset_id: r.get(11)?,
        created_at: r.get(12)?,
        updated_at: r.get(13)?,
    })
}

fn get(c: &Connection, id: &str) -> Result<VideoJob, String> {
    c.query_row(
        &format!("SELECT {COLUMNS} FROM video_jobs WHERE id=?1"),
        [id],
        row,
    )
    .map_err(|_| "视频任务不存在。".into())
}

pub(super) fn recover(c: &Connection) -> Result<(), String> {
    c.execute("UPDATE video_jobs SET status='failed',run_id=run_id+1,error='上次执行被中断，请手动重试；已保存字幕和分镜保留。' WHERE status='running'", []).map_err(|e| e.to_string())?;
    Ok(())
}

fn asset_path(c: &Connection, project: &str, id: &str, kind: &str) -> Result<PathBuf, String> {
    let path: String = c
        .query_row(
            "SELECT file_path FROM assets WHERE id=?1 AND project_id=?2 AND asset_type=?3",
            params![id, project, kind],
            |r| r.get(0),
        )
        .map_err(|_| "请选择本书中对应类型的素材。")?;
    let path = std::fs::canonicalize(path).map_err(|_| "素材文件缺失，请先重新定位。")?;
    if !path.is_file() || assets::asset_type(&path)? != kind {
        return Err("素材不是对应类型的可读取文件。".into());
    }
    Ok(path)
}

impl Database {
    pub fn video_settings(&self) -> Result<VideoSettings, String> {
        self.with_connection(|c| {
            let raw: Option<String> = c
                .query_row(
                    "SELECT value FROM app_settings WHERE key='video_runtime'",
                    [],
                    |r| r.get(0),
                )
                .optional()
                .map_err(|e| e.to_string())?;
            raw.map(|s| serde_json::from_str(&s).map_err(|e| e.to_string()))
                .unwrap_or(Ok(VideoSettings::default()))
        })
    }

    pub fn save_video_settings(&self, settings: &VideoSettings) -> Result<(), String> {
        for path in [
            &settings.engine_root,
            &settings.node_path,
            &settings.python_path,
            &settings.model_path,
        ] {
            if !path.is_empty() && (!Path::new(path).is_absolute() || path.len() > 4096) {
                return Err("运行环境必须使用绝对路径。".into());
            }
        }
        self.with_connection(|c| {
            c.execute("INSERT INTO app_settings(key,value) VALUES('video_runtime',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value", [serde_json::to_string(settings).map_err(|e| e.to_string())?]).map_err(|e| e.to_string())?;
            Ok(())
        })
    }

    pub fn video_jobs(&self, project_id: &str) -> Result<Vec<VideoJob>, String> {
        self.with_connection(|c| {
            projects::get(c, project_id)?;
            let mut q = c.prepare(&format!("SELECT {COLUMNS} FROM video_jobs WHERE project_id=?1 ORDER BY created_at DESC,id DESC")).map_err(|e| e.to_string())?;
            let rows = q.query_map([project_id], row).map_err(|e| e.to_string())?;
            rows.collect::<rusqlite::Result<Vec<_>>>().map_err(|e| e.to_string())
        })
    }

    pub fn video_job(&self, id: &str) -> Result<VideoJob, String> {
        self.with_connection(|c| get(c, id))
    }
    pub fn recover_video_jobs(&self) -> Result<(), String> {
        self.with_connection(|c| recover(c))
    }
    pub fn export_video_subtitles(&self, id: &str) -> Result<String, String> {
        self.with_connection(|c| {
            let tx = c.transaction().map_err(|e| e.to_string())?;
            let job = get(&tx, id)?;
            validate_cues(&job.plan.cues, job.plan.duration)?;
            let book = projects::get(&tx, &job.project_id)?;
            let directory = Path::new(&book.workspace_path).join("video").join(id);
            std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
            let path = directory.join(format!("subtitles-{}.srt", projects::new_id(&tx)?));
            crate::video_process::write_new(
                &path,
                crate::video::to_srt(&job.plan.cues).as_bytes(),
            )?;
            let asset =
                assets::register_existing(&tx, &job.project_id, &path, Some("subtitle"), None)?;
            tx.execute(
                "UPDATE video_jobs SET subtitle_asset_id=?1 WHERE id=?2",
                params![asset, id],
            )
            .map_err(|e| e.to_string())?;
            tx.commit().map_err(|e| e.to_string())?;
            Ok(asset)
        })
    }
    pub fn video_asset_path(&self, project: &str, id: &str, kind: &str) -> Result<PathBuf, String> {
        self.with_connection(|c| asset_path(c, project, id, kind))
    }

    pub fn create_video_job(&self, input: StartVideoInput) -> Result<VideoJob, String> {
        self.with_connection(|c| {
            let tx = c.transaction().map_err(|e| e.to_string())?;
            let book = projects::get(&tx, &input.project_id)?;
            asset_path(&tx, &book.id, &input.audio_asset_id, "audio")?;
            if let Some(id) = &input.subtitle_asset_id {
                let path = asset_path(&tx, &book.id, id, "subtitle")?;
                if !path.extension().is_some_and(|x| x.eq_ignore_ascii_case("srt")) { return Err("原声字幕请使用 SRT 格式。".into()); }
            }
            let provider = super::ai::provider(&tx, &input.provider_id)?;
            if provider.input.model_kind != "text" || !provider.input.is_enabled { return Err("请选择已启用的文本模型生成分镜。".into()); }
            let id = projects::new_id(&tx)?;
            let plan = VideoPlan { title: book.input.title, ..Default::default() };
            tx.execute(&format!("INSERT INTO video_jobs(id,project_id,audio_asset_id,source_subtitle_id,provider_id,status,stage,plan_json,created_at,updated_at) VALUES(?1,?2,?3,?4,?5,'draft','preparing',?6,{NOW},{NOW})"), params![id, book.id, input.audio_asset_id, input.subtitle_asset_id, input.provider_id, serde_json::to_string(&plan).map_err(|e| e.to_string())?]).map_err(|e| e.to_string())?;
            let job = get(&tx, &id)?;
            tx.commit().map_err(|e| e.to_string())?;
            Ok(job)
        })
    }

    pub fn claim_video_job(&self, id: &str, work: VideoWork) -> Result<VideoJob, String> {
        self.with_connection(|c| {
            let job = get(c, id)?;
            if matches!(job.status.as_str(), "running" | "succeeded") { return Err("该视频已完成或正在执行。".into()); }
            work.window(&job.plan)?;
            c.execute(&format!("UPDATE video_jobs SET status='running',stage=?1,run_id=run_id+1,error='',updated_at={NOW} WHERE id=?2"), params![work.stage(), id]).map_err(|_| "已有视频任务正在执行，请等待或停止后重试。")?;
            get(c, id)
        })
    }

    pub fn save_video_progress(
        &self,
        id: &str,
        run_id: i64,
        plan: &VideoPlan,
        stage: &str,
    ) -> Result<(), String> {
        self.with_connection(|c| {
            let count = c.execute(&format!("UPDATE video_jobs SET plan_json=?1,stage=?2,updated_at={NOW} WHERE id=?3 AND run_id=?4 AND status='running'"), params![serde_json::to_string(plan).map_err(|e| e.to_string())?, stage, id, run_id]).map_err(|e| e.to_string())?;
            if count != 1 { return Err("视频任务已停止或过期。".into()); }
            Ok(())
        })
    }

    pub fn end_video_job(
        &self,
        id: &str,
        run_id: i64,
        status: &str,
        error: &str,
    ) -> Result<(), String> {
        if !matches!(status, "failed" | "cancelled" | "awaiting_confirmation") {
            return Err("无效的视频状态。".into());
        }
        self.with_connection(|c| {
            c.execute(&format!("UPDATE video_jobs SET status=?1,error=?2,updated_at={NOW} WHERE id=?3 AND run_id=?4 AND status='running'"), params![status, error, id, run_id]).map_err(|e| e.to_string())?;
            Ok(())
        })
    }

    pub fn edit_video_plan(
        &self,
        id: &str,
        cues: Vec<Cue>,
        scenes: Vec<Scene>,
    ) -> Result<(), String> {
        self.with_connection(|c| {
            let mut job = get(c, id)?;
            if matches!(job.status.as_str(), "running" | "succeeded") { return Err("执行中或已完成的视频不能改写。".into()); }
            validate_cues(&cues, job.plan.duration)?;
            if !scenes.is_empty() { validate_scenes(&scenes, 0, cues.len())?; }
            job.plan.cues = cues; job.plan.scenes = scenes;
            c.execute(&format!("UPDATE video_jobs SET plan_json=?1,status='awaiting_confirmation',stage='storyboard',updated_at={NOW} WHERE id=?2"), params![serde_json::to_string(&job.plan).map_err(|e| e.to_string())?, id]).map_err(|e| e.to_string())?;
            Ok(())
        })
    }

    pub fn register_video_subtitle(
        &self,
        id: &str,
        run_id: i64,
        path: &Path,
    ) -> Result<String, String> {
        self.with_connection(|c| {
            let tx = c.transaction().map_err(|e| e.to_string())?;
            let job = get(&tx, id)?;
            if job.run_id != run_id || job.status != "running" {
                return Err("视频任务已过期。".into());
            }
            let asset =
                assets::register_existing(&tx, &job.project_id, path, Some("subtitle"), None)?;
            tx.execute(
                "UPDATE video_jobs SET subtitle_asset_id=?1 WHERE id=?2",
                params![asset, id],
            )
            .map_err(|e| e.to_string())?;
            tx.commit().map_err(|e| e.to_string())?;
            Ok(asset)
        })
    }

    pub fn finish_video_job(
        &self,
        id: &str,
        run_id: i64,
        video: &Path,
        subtitle: &Path,
    ) -> Result<VideoJob, String> {
        let metadata = crate::media::inspect(video)?;
        self.with_connection(|c| {
            let tx = c.transaction().map_err(|e| e.to_string())?;
            let job = get(&tx, id)?;
            if job.status == "succeeded" { return Ok(job); }
            if job.status != "running" || job.run_id != run_id { return Err("视频任务已停止或过期。".into()); }
            if job.stage != "validating" { return Err("只有完整成片校验阶段可以登记视频，试片不能用于发布。".into()); }
            if !metadata.has_audio || metadata.width != 1920 || metadata.height != 1080 || (metadata.duration_seconds - job.plan.duration).abs() > 0.2 { return Err("成片画面、音轨或时长未通过校验。".into()); }
            let video_id = assets::register_existing(&tx, &job.project_id, video, Some("video"), None)?;
            let subtitle_id = assets::register_existing(&tx, &job.project_id, subtitle, Some("subtitle"), None)?;
            tx.execute(&format!("UPDATE video_jobs SET status='succeeded',stage='complete',video_asset_id=?1,subtitle_asset_id=?2,error='',updated_at={NOW} WHERE id=?3"), params![video_id, subtitle_id, id]).map_err(|e| e.to_string())?;
            let result = get(&tx, id)?;
            tx.commit().map_err(|e| e.to_string())?;
            Ok(result)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (tempfile::TempDir, Database, StartVideoInput) {
        let dir = tempfile::tempdir().unwrap();
        let db = Database::open(dir.path().join("data.sqlite3"));
        let book = db.save_project(None, serde_json::from_value(serde_json::json!({"title":"视频测试书", "author":"", "publisher":"", "isbn":"", "description":"", "sourceNotes":"", "theme":"", "targetAudience":"", "contentStyle":"", "targetDurationSeconds":600,"status":"draft"})).unwrap()).unwrap();
        let path = dir.path().join("audio.m4a");
        std::fs::write(&path, b"source").unwrap();
        let audio = db
            .link_asset(&book.id, path.to_str().unwrap(), None)
            .unwrap();
        db.with_connection(|c| {
            c.execute("INSERT INTO model_providers(id,name,provider_type,base_url,model_name,is_default,is_enabled,model_kind,created_at,updated_at) VALUES('model','test','openai-compatible','http://localhost:1234/v1','test',1,1,'text','now','now')", []).unwrap();
            Ok(())
        }).unwrap();
        (
            dir,
            db,
            StartVideoInput {
                project_id: book.id,
                audio_asset_id: audio,
                subtitle_asset_id: None,
                provider_id: "model".into(),
            },
        )
    }

    #[test]
    fn video_job_rejects_foreign_assets_and_runs_only_once() {
        let (_dir, db, input) = fixture();
        let mut bad = StartVideoInput {
            project_id: input.project_id.clone(),
            audio_asset_id: "foreign".into(),
            subtitle_asset_id: None,
            provider_id: "model".into(),
        };
        assert!(db.create_video_job(bad).is_err());
        bad = StartVideoInput {
            project_id: input.project_id.clone(),
            audio_asset_id: input.audio_asset_id.clone(),
            subtitle_asset_id: None,
            provider_id: "model".into(),
        };
        let a = db.create_video_job(input).unwrap();
        let b = db.create_video_job(bad).unwrap();
        let running = db.claim_video_job(&a.id, VideoWork::Prepare).unwrap();
        assert!(db.claim_video_job(&a.id, VideoWork::Prepare).is_err());
        assert!(db.claim_video_job(&b.id, VideoWork::Prepare).is_err());
        assert!(db.delete_project(&a.project_id).is_err());
        assert!(db
            .unlink_asset(a.audio_asset_id.as_deref().unwrap())
            .is_err());
        db.end_video_job(&a.id, running.run_id, "cancelled", "stop")
            .unwrap();
        assert!(db
            .save_video_progress(&a.id, running.run_id, &a.plan, "storyboard")
            .is_err());
        assert!(db.claim_video_job(&b.id, VideoWork::Prepare).is_ok());
    }

    #[test]
    fn video_recovery_keeps_subtitles_but_invalidates_old_workers() {
        let (dir, db, input) = fixture();
        let job = db.create_video_job(input).unwrap();
        let job = db.claim_video_job(&job.id, VideoWork::Prepare).unwrap();
        let mut plan = job.plan.clone();
        plan.duration = 5.0;
        plan.cues = vec![Cue {
            start: 0.5,
            end: 2.0,
            text: "保存的原声".into(),
        }];
        db.save_video_progress(&job.id, job.run_id, &plan, "storyboard")
            .unwrap();
        let backup = db.create_backup(dir.path().to_str().unwrap()).unwrap();
        assert!(db.restore_backup(&backup).is_err());
        drop(db);
        let db = Database::open(dir.path().join("data.sqlite3"));
        db.recover_video_jobs().unwrap();
        let recovered = db.video_job(&job.id).unwrap();
        assert_eq!(recovered.status, "failed");
        assert_eq!(recovered.plan.cues[0].text, "保存的原声");
        assert!(db
            .save_video_progress(&job.id, job.run_id, &plan, "storyboard")
            .is_err());
        db.restore_backup(&backup).unwrap();
        assert_eq!(db.video_job(&job.id).unwrap().status, "failed");
    }

    #[test]
    fn legacy_storyboards_remain_readable_and_only_explicitly_upgrade_the_scenes() {
        let (_dir, db, input) = fixture();
        let job = db.create_video_job(input).unwrap();
        let old = serde_json::json!({"title":"旧视频", "audioSha256":"preserved", "duration":5.0, "cues":[{"start":0.5,"end":4.0,"text":"保留的字幕"}], "scenes":[{"first":0,"last":0,"kind":"book","title":"旧图解","labels":["书籍"]}]});
        db.with_connection(|c| {
            c.execute(
                "UPDATE video_jobs SET plan_json=?1,status='awaiting_confirmation' WHERE id=?2",
                params![old.to_string(), job.id],
            )
            .unwrap();
            Ok(())
        })
        .unwrap();
        let saved = db.video_job(&job.id).unwrap();
        assert_eq!(saved.plan.scenes[0].labels[0], "书籍");
        assert!(db
            .claim_video_job(&job.id, VideoWork::Render)
            .unwrap_err()
            .contains("重新生成分镜"));
        assert_eq!(
            db.video_job(&job.id).unwrap().status,
            "awaiting_confirmation"
        );
        db.edit_video_plan(&job.id, saved.plan.cues, vec![])
            .unwrap();
        let upgraded = db.video_job(&job.id).unwrap();
        assert_eq!(upgraded.plan.cues[0].text, "保留的字幕");
        assert_eq!(upgraded.plan.audio_sha256, "preserved");
        assert!(upgraded.plan.scenes.is_empty());
        assert!(db.claim_video_job(&job.id, VideoWork::Prepare).is_ok());
    }

    #[test]
    fn scene_layers_round_trip_and_backup_without_rewriting_old_plan_json() {
        let (dir, db, input) = fixture();
        let job = db.create_video_job(input).unwrap();
        let old = serde_json::json!({"title":"原书", "audioSha256":"source", "duration":5.0, "cues":[{"start":0.5,"end":4.0,"text":"寻找方向"}], "scenes":[{"first":0,"last":0,"kind":"focus","title":"方向","beats":[{"cue":0,"object":"book","action":"draw","label":"方向"}]}]}).to_string();
        db.with_connection(|c| {
            c.execute(
                "UPDATE video_jobs SET plan_json=?1,status='awaiting_confirmation' WHERE id=?2",
                params![old, job.id],
            )
            .unwrap();
            Ok(())
        })
        .unwrap();
        let saved = db.video_job(&job.id).unwrap();
        let decoded = serde_json::to_value(&saved.plan).unwrap();
        assert_eq!(decoded["scenes"][0]["environment"], "plain");
        assert_eq!(decoded["scenes"][0]["framing"], "reference");
        db.with_connection(|c| {
            let json: String = c
                .query_row(
                    "SELECT plan_json FROM video_jobs WHERE id=?1",
                    [&job.id],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(
                json, old,
                "Reading defaults must not rewrite historical JSON."
            );
            Ok(())
        })
        .unwrap();
        let scenes: Vec<Scene> = serde_json::from_value(serde_json::json!([{"first":0,"last":0,"kind":"focus","title":"方向","environment":"path","framing":"detail","beats":[{"cue":0,"object":"compass","action":"draw","label":"方向"}]}])).unwrap();
        db.edit_video_plan(&job.id, saved.plan.cues.clone(), scenes)
            .unwrap();
        let expected = serde_json::to_value(db.video_job(&job.id).unwrap().plan).unwrap();
        let backup = db.create_backup(dir.path().to_str().unwrap()).unwrap();
        db.edit_video_plan(&job.id, saved.plan.cues, vec![])
            .unwrap();
        db.restore_backup(&backup).unwrap();
        let recovered = serde_json::to_value(db.video_job(&job.id).unwrap().plan).unwrap();
        assert_eq!(recovered, expected);
        assert_eq!(recovered["audioSha256"], "source");
        assert_eq!(recovered["scenes"][0]["environment"], "path");
        assert_eq!(recovered["scenes"][0]["framing"], "detail");
        assert_eq!(recovered["scenes"][0]["beats"][0]["object"], "compass");
    }

    #[test]
    fn preview_returns_to_confirmation_without_registering_assets_and_edits_invalidate_it() {
        use crate::video::{PreviewRequest, VideoWork};
        let (dir, db, input) = fixture();
        let job = db.create_video_job(input).unwrap();
        let job = db.claim_video_job(&job.id, VideoWork::Prepare).unwrap();
        let video = rendered_video(dir.path());
        let duration = crate::media::inspect(&video).unwrap().duration_seconds;
        let plan: VideoPlan = serde_json::from_value(serde_json::json!({
            "title":"原书", "audioSha256":"preserved", "duration":duration,
            "cues":[{"start":0,"end":duration,"text":"原声"}],
            "scenes":[{"first":0,"last":0,"kind":"focus","title":"书籍","beats":[{"cue":0,"object":"book","action":"draw","label":"原书"}]}]
        })).unwrap();
        db.save_video_progress(&job.id, job.run_id, &plan, "storyboard")
            .unwrap();
        db.end_video_job(&job.id, job.run_id, "awaiting_confirmation", "")
            .unwrap();
        let original = serde_json::to_string(&db.video_job(&job.id).unwrap().plan).unwrap();
        let preview = db
            .claim_video_job(
                &job.id,
                VideoWork::Preview(PreviewRequest {
                    first_scene: 0,
                    seconds: 30,
                }),
            )
            .unwrap();
        assert_eq!(preview.stage, "previewing");
        assert_eq!(preview.status, "running");
        assert!(db
            .unlink_asset(preview.audio_asset_id.as_deref().unwrap())
            .is_err());
        let srt = dir.path().join("preview.srt");
        std::fs::write(&srt, crate::video::to_srt(&plan.cues)).unwrap();
        assert!(
            db.finish_video_job(&job.id, preview.run_id, &video, &srt)
                .is_err(),
            "A valid preview MP4 still must not be accepted as a completed film."
        );
        db.end_video_job(&job.id, preview.run_id, "awaiting_confirmation", "")
            .unwrap();
        let done = db.video_job(&job.id).unwrap();
        assert_eq!(done.stage, "previewing");
        assert_eq!(done.status, "awaiting_confirmation");
        assert_eq!(serde_json::to_string(&done.plan).unwrap(), original);
        assert!(done.video_asset_id.is_none());
        assert_eq!(db.list_assets(&job.project_id).unwrap().len(), 1);
        db.edit_video_plan(&job.id, done.plan.cues, done.plan.scenes)
            .unwrap();
        assert_eq!(db.video_job(&job.id).unwrap().stage, "storyboard");
    }

    fn rendered_video(directory: &Path) -> PathBuf {
        let mut bytes = include_bytes!("../../tests/fixtures/portrait.mp4").to_vec();
        let start = bytes.windows(4).position(|w| w == b"tkhd").unwrap() - 4;
        let size = u32::from_be_bytes(bytes[start..start + 4].try_into().unwrap()) as usize;
        bytes[start + size - 8..start + size - 4].copy_from_slice(&(1920u32 << 16).to_be_bytes());
        bytes[start + size - 4..start + size].copy_from_slice(&(1080u32 << 16).to_be_bytes());
        let video = directory.join("rendered.mp4");
        std::fs::write(&video, bytes).unwrap();
        video
    }

    #[test]
    fn video_completion_registers_both_files_atomically_and_is_idempotent() {
        let (dir, db, input) = fixture();
        let job = db.create_video_job(input).unwrap();
        let job = db.claim_video_job(&job.id, VideoWork::Prepare).unwrap();
        let video = rendered_video(dir.path());
        let subtitle = dir.path().join("subtitles.srt");
        let mut plan = job.plan.clone();
        plan.duration = crate::media::inspect(&video).unwrap().duration_seconds;
        db.save_video_progress(&job.id, job.run_id, &plan, "validating")
            .unwrap();
        assert!(db
            .finish_video_job(&job.id, job.run_id, &video, &subtitle)
            .is_err());
        assert_eq!(db.list_assets(&job.project_id).unwrap().len(), 1);
        std::fs::write(&subtitle, "1\n00:00:00,000 --> 00:00:00,500\n原声字幕\n").unwrap();
        let completed = db
            .finish_video_job(&job.id, job.run_id, &video, &subtitle)
            .unwrap();
        assert_eq!(completed.status, "succeeded");
        assert_eq!(db.list_assets(&job.project_id).unwrap().len(), 3);
        assert_eq!(
            db.finish_video_job(&job.id, job.run_id, &video, &subtitle)
                .unwrap()
                .video_asset_id,
            completed.video_asset_id
        );
        assert_eq!(db.list_assets(&job.project_id).unwrap().len(), 3);
        let restore = dir.path().join("restored");
        std::fs::create_dir(&restore).unwrap();
        let backup = db
            .create_full_backup(dir.path().to_str().unwrap(), true)
            .unwrap();
        db.restore_full_backup(&backup.manifest_path, restore.to_str().unwrap())
            .unwrap();
        assert!(db
            .list_assets(&job.project_id)
            .unwrap()
            .iter()
            .all(|a| a.exists_status == "available"));
        assert_eq!(
            db.video_job(&job.id).unwrap().video_asset_id,
            completed.video_asset_id
        );
    }

    #[test]
    fn schema_thirteen_upgrades_without_losing_existing_data() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("old.sqlite3");
        let connection = Connection::open(&path).unwrap();
        for sql in super::super::MIGRATIONS.iter().take(13) {
            connection.execute_batch(sql).unwrap();
        }
        connection.pragma_update(None, "user_version", 13).unwrap();
        connection
            .execute(
                "INSERT INTO app_settings(key,value) VALUES('preserved','yes')",
                [],
            )
            .unwrap();
        drop(connection);
        let db = Database::open(path);
        db.with_connection(|c| {
            assert_eq!(
                c.pragma_query_value(None, "user_version", |r| r.get::<_, i32>(0))
                    .unwrap(),
                14
            );
            assert_eq!(
                c.query_row(
                    "SELECT value FROM app_settings WHERE key='preserved'",
                    [],
                    |r| r.get::<_, String>(0)
                )
                .unwrap(),
                "yes"
            );
            assert!(!c
                .prepare("PRAGMA foreign_key_check")
                .unwrap()
                .exists([])
                .unwrap());
            Ok(())
        })
        .unwrap();
    }
}
