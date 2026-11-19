use super::*;
use crate::{
    database::ai::ProviderConfig,
    video::revision::{parse_candidate, SceneRevisionInput, PROMPT_VERSION},
};

const KIND: &str = "video_scene_revision";

#[derive(Serialize, Deserialize)]
struct Snapshot {
    job_id: String,
    run_id: i64,
    provider_id: String,
    scene: Scene,
    user: String,
    source: Vec<u8>,
}

impl Snapshot {
    fn current_job(&self, c: &Connection) -> Result<VideoJob, String> {
        let job = editable(c, &self.job_id, self.run_id)?;
        if source(&job)? != self.source {
            return Err("正式分镜已变化，请重新生成这一镜。".into());
        }
        Ok(job)
    }
}

pub struct PreparedSceneRevision {
    pub theme: String,
    pub provider: ProviderConfig,
    pub user: String,
    pub pencil: bool,
}

fn source(job: &VideoJob) -> Result<Vec<u8>, String> {
    Ok(Sha256::digest(serde_json::to_vec(&job.plan).map_err(|e| e.to_string())?).to_vec())
}

fn editable(c: &Connection, job_id: &str, run_id: i64) -> Result<VideoJob, String> {
    let job = get(c, job_id)?;
    if matches!(job.status.as_str(), "running" | "succeeded") || job.run_id != run_id {
        return Err("视频任务已在执行、已完成或已更新，请重新读取后再调整。".into());
    }
    Ok(job)
}

impl Database {
    pub fn start_video_scene_revision(&self, input: SceneRevisionInput) -> Result<String, String> {
        self.with_connection(|c| {
            let job = editable(c, &input.job_id, input.run_id)?;
            let cues = input.validate(&job.plan)?;
            let provider = super::super::ai::provider(c, &input.provider_id)?;
            if !provider.input.is_enabled || provider.input.model_kind != "text" { return Err("请选择已启用的文本模型。".into()); }
            let snapshot = Snapshot {
                source: source(&job)?, user: input.user(&job.plan, &cues),
                job_id: input.job_id, run_id: input.run_id, provider_id: input.provider_id, scene: input.scene,
            };
            let raw = serde_json::to_string(&snapshot).map_err(|e| e.to_string())?;
            if raw.len() > 4 * 1024 * 1024 { return Err("分镜编辑快照超过4MB。".into()); }
            let id = projects::new_id(c)?;
            c.execute(&format!("INSERT INTO ai_tasks(id,project_id,kind,provider_id,model_name,prompt_version,status,input_json,created_at,chapter_id) VALUES(?1,?2,?3,?4,?5,?6,'pending',?7,{NOW},?8)"), params![id,job.project_id,KIND,snapshot.provider_id,provider.input.model_name,PROMPT_VERSION,raw,job.chapter_id]).map_err(|e| e.to_string())?;
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
            let job = snapshot.current_job(c)?;
            let provider = super::super::ai::provider(c, &snapshot.provider_id)?;
            if !provider.input.is_enabled || provider.input.model_kind != "text" {
                return Err("请选择已启用的文本模型。".into());
            }
            c.execute(
                "UPDATE ai_tasks SET status='running',model_name=?2 WHERE id=?1",
                params![id, provider.input.model_name],
            )
            .map_err(|e| e.to_string())?;
            Ok(PreparedSceneRevision {
                theme: job.plan.theme,
                provider,
                user: snapshot.user,
                pencil: snapshot.scene.artwork.is_some(),
            })
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
            let job = snapshot.current_job(c)?;
            let mut scene = snapshot.scene;
            let candidate = if let Some(original) = &scene.artwork {
                let artwork = crate::video::pencil::parse_artwork(
                    raw,
                    scene.first,
                    scene.last,
                    &job.plan
                        .pencil
                        .as_ref()
                        .ok_or("任务缺少冻结人物设定。")?
                        .characters,
                )?;
                artwork.validate_cast(
                    &original
                        .character_ids()
                        .map(str::to_owned)
                        .collect::<Vec<_>>(),
                )?;
                scene.artwork = Some(artwork);
                scene
            } else {
                parse_candidate(&scene, raw)?
            };
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
    use crate::video::revision::tests::{candidate, input, plan};

    #[test]
    fn pencil_revision_keeps_frozen_heads_and_candidate_does_not_change_saved_artwork() {
        use crate::video::pencil::{Artwork, Character, PencilPlan};
        let (_dir, db, mut input) = fixture();
        let mut running = db
            .claim_video_job(&input.job_id, VideoWork::Prepare, None)
            .unwrap();
        let head = serde_json::json!({"mask":[],"strokes":[{"p":[[20,20],[80,80]],"w":1.2,"a":0.6}],"shades":[]});
        let character:Character=serde_json::from_value(serde_json::json!({"id":"reader","name":"读者","description":"方巾长脸","heads":{"front":head,"side":head}})).unwrap();
        let mut other = character.clone();
        other.id = "other".into();
        other.description = "只在邻镜出现的人".into();
        let artwork:Artwork=serde_json::from_value(serde_json::json!({"design":"原画稿","layers":[{"cue":1,"name":"人物","mask":[],"strokes":[{"p":[[100,300],[150,500]],"w":1.2,"a":0.6}],"shades":[],"texts":[],"portrait":{"characterId":"reader","view":"side","x":150,"y":120,"scale":0.8,"flip":false}}]})).unwrap();
        running.plan.pencil = Some(PencilPlan {
            characters: vec![character, other],
            pending: vec![],
        });
        running.plan.scenes[1].beats.clear();
        running.plan.scenes[1].artwork = Some(artwork.clone());
        input.scene = running.plan.scenes[1].clone();
        input.run_id = running.run_id;
        db.save_video_progress(&input.job_id, input.run_id, &running.plan, "storyboard")
            .unwrap();
        db.end_video_job(&input.job_id, input.run_id, "awaiting_confirmation", "")
            .unwrap();
        let id = db.start_video_scene_revision(input.clone()).unwrap();
        let prepared = db.prepare_video_scene_revision(&id).unwrap();
        assert!(prepared.pencil);
        assert!(prepared.user.contains("方巾长脸"));
        let user: serde_json::Value = serde_json::from_str(&prepared.user).unwrap();
        assert_eq!(user["scene"]["characters"], serde_json::json!(["reader"]));
        assert!(!prepared.user.contains("只在邻镜出现的人"));
        assert!(!prepared.user.contains("\"heads\""));
        let mut revised = artwork;
        revised.design = "新画稿".into();
        let candidate = db
            .finish_video_scene_revision(&id, &serde_json::to_string(&revised).unwrap())
            .unwrap();
        assert_eq!(candidate.artwork.unwrap().design, "新画稿");
        let saved = db.video_job(&input.job_id).unwrap();
        assert_eq!(
            saved.plan.scenes[1].artwork.as_ref().unwrap().design,
            "原画稿"
        );
        assert_eq!(
            serde_json::to_value(&saved.plan.pencil).unwrap(),
            serde_json::to_value(running.plan.pencil).unwrap()
        );
        let mut changed = saved.plan.cues.clone();
        changed[0].text = "校正台词".into();
        assert!(db
            .edit_video_plan(&input.job_id, changed.clone(), saved.plan.scenes.clone())
            .unwrap_err()
            .contains("已有画稿保留"));
        assert!(db.video_job(&input.job_id).unwrap().plan.pencil.is_some());
        db.edit_video_plan(&input.job_id, changed, vec![]).unwrap();
        let rebuilt = db.video_job(&input.job_id).unwrap();
        assert!(rebuilt.plan.scenes.is_empty());
        assert!(rebuilt.plan.pencil.is_none());
    }

    #[test]
    fn reading_unmodified_srt_preserves_saved_precision_and_allows_pencil_export() {
        use crate::video::{parse_srt, to_srt};
        let (_dir, db, input) = fixture();
        let mut running = db
            .claim_video_job(&input.job_id, VideoWork::Prepare, None)
            .unwrap();
        running.plan.theme = "ink".into();
        running.plan.pencil = Some(Default::default());
        running.plan.cues[0].start = 1.139_999;
        db.save_video_progress(&running.id, running.run_id, &running.plan, "storyboard")
            .unwrap();
        db.end_video_job(&running.id, running.run_id, "awaiting_confirmation", "")
            .unwrap();
        let saved = db.video_job(&running.id).unwrap().plan;
        let srt = to_srt(&saved.cues);
        let parsed = parse_srt(&srt, saved.duration).unwrap();
        assert_ne!(parsed, saved.cues);
        db.edit_video_plan(&running.id, parsed.clone(), saved.scenes.clone())
            .unwrap();
        let reloaded = db.video_job(&running.id).unwrap().plan;
        assert_eq!(reloaded.cues, saved.cues);
        assert!(reloaded.pencil.is_some());
        VideoWork::Render.window(&reloaded).unwrap();
        let mut changed = parsed;
        changed[0].start = 1.141;
        assert!(db
            .edit_video_plan(&running.id, changed, saved.scenes)
            .unwrap_err()
            .contains("精绘字幕已改变"));
    }

    #[test]
    fn single_pencil_revision_uses_only_its_context_even_when_the_storyboard_exceeds_four_mib() {
        use crate::video::to_srt;
        let (_dir, db, mut input) = fixture();
        let mut running = db
            .claim_video_job(&input.job_id, VideoWork::Prepare, None)
            .unwrap();
        let plan = &mut running.plan;
        plan.theme = "ink".into();
        plan.duration = 4800.0;
        plan.pencil = Some(Default::default());
        plan.cues = (0..240)
            .map(|i| Cue {
                start: (i * 20) as f64,
                end: (i * 20 + 15) as f64,
                text: "本段字幕".into(),
            })
            .collect();
        plan.scenes = (0..240).map(|i| serde_json::from_value(serde_json::json!({
            "first":i,"last":i,"kind":"focus","title":"书页",
            "artwork":{"design":"线".repeat(5900), "layers":[{
                "cue":i,"name":"书页","mask":[],"strokes":[{"p":[[20,20],[80,80]],"w":1.2,"a":0.6}],"shades":[],"texts":[]
            }]}
        })).unwrap()).collect();
        assert!(serde_json::to_vec(&plan.scenes).unwrap().len() > 4 * 1024 * 1024);
        VideoWork::Render.window(plan).unwrap();
        db.save_video_progress(&running.id, running.run_id, plan, "storyboard")
            .unwrap();
        db.end_video_job(&running.id, running.run_id, "awaiting_confirmation", "")
            .unwrap();
        input.run_id = running.run_id;
        input.scene_index = 120;
        input.scene = plan.scenes[input.scene_index].clone();
        input.previous_scene = None;
        input.next_scene = None;
        input.subtitles = to_srt(&plan.cues);
        let id = db.start_video_scene_revision(input.clone()).unwrap();
        db.with_connection(|c| {
            let bytes: i64 = c
                .query_row(
                    "SELECT length(CAST(input_json AS BLOB)) FROM ai_tasks WHERE id=?1",
                    [&id],
                    |row| row.get(0),
                )
                .unwrap();
            assert!(bytes < 100_000);
            Ok(())
        })
        .unwrap();
        let prepared = db.prepare_video_scene_revision(&id).unwrap();
        let user: serde_json::Value = serde_json::from_str(&prepared.user).unwrap();
        assert_eq!(user["scene"]["characters"], serde_json::json!([]));
        assert_eq!(user["characters"], serde_json::json!([]));
        assert_eq!(user["cues"].as_array().unwrap().len(), 1);
        let mut candidate = input.scene.artwork.unwrap();
        candidate.design = "只重画该镜".into();
        assert!(db
            .finish_video_scene_revision(&id, &serde_json::to_string(&candidate).unwrap())
            .is_ok());
        assert_ne!(
            db.video_job(&running.id).unwrap().plan.scenes[120]
                .artwork
                .as_ref()
                .unwrap()
                .design,
            candidate.design
        );
    }

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
        let fixture = plan(&input);
        active.plan.duration = fixture.duration;
        active.plan.cues = fixture.cues;
        active.plan.scenes = fixture.scenes;
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
        input.previous_scene.as_mut().unwrap().title = "未保存的邻镜修改".into();
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
        let mut scenes = db.video_job(&input.job_id).unwrap().plan.scenes;
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
