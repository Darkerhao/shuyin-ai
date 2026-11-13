use super::*;
use crate::{
    database::ai::ProviderConfig,
    video::revision::{SceneRevisionInput, PROMPT_VERSION},
};

const KIND: &str = "video_scene_revision";

#[derive(Serialize, Deserialize)]
struct Snapshot {
    input: SceneRevisionInput,
    source: Vec<u8>,
}

pub struct PreparedSceneRevision {
    pub provider: ProviderConfig,
    pub user: String,
}

fn source(job: &VideoJob) -> Result<Vec<u8>, String> {
    Ok(Sha256::digest(serde_json::to_vec(&job.plan).map_err(|e| e.to_string())?).to_vec())
}

fn editable(c: &Connection, input: &SceneRevisionInput) -> Result<VideoJob, String> {
    let job = get(c, &input.job_id)?;
    if matches!(job.status.as_str(), "running" | "succeeded") || job.run_id != input.run_id {
        return Err("视频任务已在执行、已完成或已更新，请重新读取后再调整。".into());
    }
    Ok(job)
}

impl Database {
    pub fn start_video_scene_revision(&self, input: SceneRevisionInput) -> Result<String, String> {
        self.with_connection(|c| {
            let job = editable(c, &input)?;
            input.validate(job.plan.duration, &job.plan.scenes)?;
            let provider = super::super::ai::provider(c, &input.provider_id)?;
            if !provider.input.is_enabled || provider.input.model_kind != "text" { return Err("请选择已启用的文本模型。".into()); }
            let provider_id = input.provider_id.clone();
            let snapshot = serde_json::to_string(&Snapshot { source: source(&job)?, input }).map_err(|e| e.to_string())?;
            if snapshot.len() > 4 * 1024 * 1024 { return Err("分镜编辑快照超过4MB。".into()); }
            let id = projects::new_id(c)?;
            c.execute(&format!("INSERT INTO ai_tasks(id,project_id,kind,provider_id,model_name,prompt_version,status,input_json,created_at,chapter_id) VALUES(?1,?2,?3,?4,?5,?6,'pending',?7,{NOW},?8)"), params![id,job.project_id,KIND,provider_id,provider.input.model_name,PROMPT_VERSION,snapshot,job.chapter_id]).map_err(|e| e.to_string())?;
            Ok(id)
        })
    }

    pub fn prepare_video_scene_revision(&self, id: &str) -> Result<PreparedSceneRevision, String> {
        self.with_connection(|c| {
            let raw: String = c
                .query_row(
                    "SELECT input_json FROM ai_tasks WHERE id=?1 AND kind=?2 AND status='pending'",
                    params![id, KIND],
                    |r| r.get(0),
                )
                .map_err(|_| "单镜任务已取消、已执行或不存在。")?;
            let snapshot: Snapshot = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
            let job = editable(c, &snapshot.input)?;
            if source(&job)? != snapshot.source {
                return Err("正式分镜已变化，请重新生成这一镜。".into());
            }
            let cues = snapshot
                .input
                .validate(job.plan.duration, &job.plan.scenes)?;
            let provider = super::super::ai::provider(c, &snapshot.input.provider_id)?;
            if !provider.input.is_enabled || provider.input.model_kind != "text" {
                return Err("请选择已启用的文本模型。".into());
            }
            let user = snapshot.input.user(&job.plan.title, &cues);
            c.execute(
                "UPDATE ai_tasks SET status='running',model_name=?2 WHERE id=?1",
                params![id, provider.input.model_name],
            )
            .map_err(|e| e.to_string())?;
            Ok(PreparedSceneRevision { provider, user })
        })
    }

    pub fn finish_video_scene_revision(&self, id: &str, raw: &str) -> Result<Scene, String> {
        self.with_connection(|c| {
            let json: String = c
                .query_row(
                    "SELECT input_json FROM ai_tasks WHERE id=?1 AND kind=?2 AND status='running'",
                    params![id, KIND],
                    |r| r.get(0),
                )
                .map_err(|_| "单镜任务已取消或结束，未采用候选。")?;
            let snapshot: Snapshot = serde_json::from_str(&json).map_err(|e| e.to_string())?;
            let job = editable(c, &snapshot.input)?;
            if source(&job)? != snapshot.source {
                return Err("正式分镜已变化，候选已过期。".into());
            }
            let candidate = snapshot.input.candidate(raw)?;
            c.execute(
                &format!("UPDATE ai_tasks SET status='succeeded',completed_at={NOW} WHERE id=?1"),
                [id],
            )
            .map_err(|e| e.to_string())?;
            // Candidate adoption belongs to the existing editor draft; never change the saved plan here.
            Ok(candidate)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::video::revision::tests::{candidate, input};

    fn fixture() -> (tempfile::TempDir, Database, SceneRevisionInput) {
        fixture_for(false)
    }

    fn fixture_for(chapter: bool) -> (tempfile::TempDir, Database, SceneRevisionInput) {
        let (dir, db, mut start) = super::super::tests::fixture();
        if chapter {
            let chapter = db
                .save_chapter(
                    &start.project_id,
                    None,
                    crate::database::chapters::ChapterInput {
                        title: "本章精讲".into(),
                        content: "本章字幕和分镜的来源".into(),
                        target_duration_seconds: Some(600),
                    },
                )
                .unwrap();
            let audio = dir.path().join("chapter-audio.wav");
            std::fs::write(&audio, b"chapter audio fixture").unwrap();
            start.audio_asset_id = db
                .link_asset(
                    &start.project_id,
                    audio.to_str().unwrap(),
                    None,
                    Some(&chapter.id),
                )
                .unwrap();
            start.chapter_id = Some(chapter.id);
        }
        let job = db.create_video_job(start).unwrap();
        let mut active = db
            .claim_video_job(&job.id, VideoWork::Prepare, None)
            .unwrap();
        let mut input = input();
        input.job_id = job.id;
        input.run_id = active.run_id;
        active.plan.duration = 8.0;
        active.plan.cues = crate::video::parse_srt(&input.subtitles, 8.0).unwrap();
        active.plan.scenes = input.scenes.clone();
        db.save_video_progress(&input.job_id, input.run_id, &active.plan, "storyboard")
            .unwrap();
        db.end_video_job(&input.job_id, input.run_id, "awaiting_confirmation", "")
            .unwrap();
        (dir, db, input)
    }

    #[test]
    fn chapter_review_scene_revision_preserves_scope_in_history_and_resume_entries() {
        let (_dir, db, input) = fixture_for(true);
        let job = db.video_job(&input.job_id).unwrap();
        let task = db.start_video_scene_revision(input).unwrap();
        let history = db
            .list_ai_tasks(&job.project_id, job.chapter_id.as_deref())
            .unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].id, task);
        assert_eq!(history[0].chapter_id, job.chapter_id);
        assert!(db.list_ai_tasks(&job.project_id, None).unwrap().is_empty());
        let resume = db.workflow_snapshot().unwrap();
        let running = resume
            .running
            .iter()
            .find(|entry| entry.task_id == task)
            .unwrap();
        assert_eq!(running.chapter_id, job.chapter_id);
        assert_eq!(running.kind, "video");
        assert_eq!(running.stage, KIND);
        assert!(running.title.contains("本章精讲"));
    }

    #[test]
    fn candidate_generation_never_changes_video_plan_or_editor_draft() {
        let (_dir, db, mut input) = fixture();
        db.save_video_draft(
            &input.job_id,
            Some(crate::database::drafts::EditorDraft {
                based_on: Some(input.run_id.to_string()),
                content: serde_json::json!({"unfinished":"保留"}),
                updated_at: String::new(),
            }),
        )
        .unwrap();
        let before = serde_json::to_value(db.video_job(&input.job_id).unwrap()).unwrap();
        input.scenes[0].title = "未保存的邻镜修改".into();
        let id = db.start_video_scene_revision(input.clone()).unwrap();
        let prepared = db.prepare_video_scene_revision(&id).unwrap();
        assert!(prepared.user.contains("未保存的邻镜修改"));
        assert!(db.prepare_video_scene_revision(&id).is_err());
        let scene = db
            .finish_video_scene_revision(&id, &candidate().to_string())
            .unwrap();
        assert_eq!(scene.title, "新的观点");
        assert_eq!(
            serde_json::to_value(db.video_job(&input.job_id).unwrap()).unwrap(),
            before
        );
        assert_eq!(
            db.read_video_draft(&input.job_id).unwrap().unwrap().content["unfinished"],
            "保留"
        );
        let task = db
            .list_ai_tasks(&db.video_job(&input.job_id).unwrap().project_id, None)
            .unwrap();
        assert_eq!(task[0].kind, KIND);
        assert_eq!(task[0].status, "succeeded");
    }

    #[test]
    fn cancellation_and_new_video_run_reject_late_candidates() {
        let (_dir, db, input) = fixture();
        let id = db.start_video_scene_revision(input.clone()).unwrap();
        db.prepare_video_scene_revision(&id).unwrap();
        db.end_generation(&id, "cancelled", "cancelled", "停止")
            .unwrap();
        assert!(db
            .finish_video_scene_revision(&id, &candidate().to_string())
            .is_err());
        let id = db.start_video_scene_revision(input.clone()).unwrap();
        db.prepare_video_scene_revision(&id).unwrap();
        db.claim_video_job(&input.job_id, VideoWork::Prepare, None)
            .unwrap();
        assert!(db
            .finish_video_scene_revision(&id, &candidate().to_string())
            .is_err());
        assert!(db.start_video_scene_revision(input).is_err());
    }

    #[test]
    fn changed_saved_plan_and_invalid_inputs_never_replace_other_scenes() {
        let (_dir, db, input) = fixture();
        for invalid in ["", &"长".repeat(2001)] {
            let mut wrong = input.clone();
            wrong.instruction = invalid.into();
            assert!(db.start_video_scene_revision(wrong).is_err());
        }
        let mut wrong = input.clone();
        wrong.scene_index = 99;
        assert!(db.start_video_scene_revision(wrong).is_err());
        let mut wrong = input.clone();
        wrong.provider_id = "missing".into();
        assert!(db.start_video_scene_revision(wrong).is_err());
        let mut wrong = input.clone();
        wrong.subtitles = "未完成字幕".into();
        assert!(db.start_video_scene_revision(wrong).is_err());
        let id = db.start_video_scene_revision(input.clone()).unwrap();
        db.prepare_video_scene_revision(&id).unwrap();
        let mut scenes = input.scenes.clone();
        scenes[2].title = "别的地方新保存的修改".into();
        db.edit_video_plan(
            &input.job_id,
            crate::video::parse_srt(&input.subtitles, 8.0).unwrap(),
            scenes.clone(),
        )
        .unwrap();
        assert!(db
            .finish_video_scene_revision(&id, &candidate().to_string())
            .is_err());
        assert_eq!(
            serde_json::to_value(db.video_job(&input.job_id).unwrap().plan.scenes).unwrap(),
            serde_json::to_value(scenes).unwrap()
        );
    }
}
