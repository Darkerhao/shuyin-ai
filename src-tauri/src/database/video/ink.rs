use super::*;
use crate::{
    database::image_generation::ImageGenerationInput,
    video::{ink, SceneImage},
};

pub(crate) fn bound_job(
    c: &Connection,
    task: &str,
    input: &str,
) -> Result<(VideoJob, usize), String> {
    let value: serde_json::Value = serde_json::from_str(input).map_err(|e| e.to_string())?;
    let binding = &value["videoBinding"];
    let job = get(
        c,
        binding["jobId"].as_str().ok_or("水墨任务缺少视频引用。")?,
    )?;
    let index = binding["sceneIndex"].as_u64().ok_or("水墨任务缺少镜号。")? as usize;
    if job.status != "running"
        || Some(job.run_id) != binding["runId"].as_i64()
        || job
            .plan
            .scenes
            .get(index)
            .and_then(|s| s.painting.as_ref())
            .and_then(|p| p.task_id.as_deref())
            != Some(task)
    {
        return Err("视频任务已停止或画面已变化，未采用迟到的画作。".into());
    }
    Ok((job, index))
}

impl Database {
    pub(crate) fn prepare_video_painting(
        &self,
        id: &str,
        run_id: i64,
        index: usize,
    ) -> Result<String, String> {
        self.with_connection(|c| {
            let tx = c.transaction().map_err(|e|e.to_string())?;
            let mut job = get(&tx,id)?;
            if job.status != "running" || job.run_id != run_id { return Err("视频任务已停止。".into()); }
            let scene = job.plan.scenes.get(index).ok_or("水墨分镜不存在。")?;
            let painting = scene.painting.as_ref().ok_or("水墨分镜缺少画面描述。")?;
            painting.validate()?;
            if scene.image.is_some() { return Err("此镜已有画作，不必重新生成。".into()); }
            let provider_id = job.plan.image_provider_id.as_deref().ok_or("请选择水墨图片模型。")?;
            let provider = super::super::ai::provider(&tx,provider_id)?;
            if !provider.input.is_enabled || provider.input.model_kind != "image" { return Err("水墨图片模型已停用或不存在，请在模型设置中恢复。".into()); }
            let task = painting.task_id.clone().unwrap_or(projects::new_id(&tx)?);
            let mut input = serde_json::to_value(ImageGenerationInput {
                project_id:job.project_id.clone(),chapter_id:job.chapter_id.clone(),provider_id:provider_id.into(),
                description:ink::image_description(&job.plan,index),width:1920,height:1080,
            }).map_err(|e|e.to_string())?;
            input["videoBinding"] = serde_json::json!({"jobId":id,"runId":run_id,"sceneIndex":index});
            if painting.task_id.is_some() {
                let status: String = tx.query_row("SELECT status FROM ai_tasks WHERE id=?1 AND kind='video_ink' AND project_id=?2 AND chapter_id IS ?3",params![task,job.project_id,job.chapter_id],|row|row.get(0)).map_err(|_|"水墨画作任务不存在。")?;
                if !matches!(status.as_str(),"pending"|"failed"|"cancelled") { return Err("画作任务仍在执行或已完成，请重新读取视频。".into()); }
                tx.execute("UPDATE ai_tasks SET status='pending',input_json=?2,error_code=NULL,error_message=NULL,completed_at=NULL WHERE id=?1",params![task,input.to_string()]).map_err(|e|e.to_string())?;
            } else {
                tx.execute(&format!("INSERT INTO ai_tasks(id,project_id,chapter_id,kind,provider_id,model_name,prompt_version,status,input_json,created_at) VALUES(?1,?2,?3,'video_ink',?4,?5,?6,'pending',?7,{NOW})"),params![task,job.project_id,job.chapter_id,provider_id,provider.input.model_name,ink::IMAGE_VERSION,input.to_string()]).map_err(|e|e.to_string())?;
            }
            job.plan.scenes[index].painting.as_mut().unwrap().task_id = Some(task.clone());
            tx.execute(&format!("UPDATE video_jobs SET plan_json=?2,stage='painting',updated_at={NOW} WHERE id=?1"),params![id,serde_json::to_string(&job.plan).map_err(|e|e.to_string())?]).map_err(|e|e.to_string())?;
            tx.commit().map_err(|e|e.to_string())?;
            Ok(task)
        })
    }
}

pub(crate) fn attach(
    c: &Connection,
    task: &str,
    input: &str,
    asset: &str,
    model: &str,
    bytes: &[u8],
) -> Result<(), String> {
    let (mut job, index) = bound_job(c, task, input)?;
    job.plan.scenes[index].image = Some(SceneImage {
        asset_id: asset.into(),
        source: format!("AI水墨画作 · {model}"),
        rights: "AI生成的叙事画作，非史料或真实人物复原；使用前须核对模型服务条款。".into(),
        creator: String::new(),
        fit: "cover".into(),
        x: 0.5,
        y: 0.5,
        sha256: Sha256::digest(bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
    });
    c.execute(
        &format!("UPDATE video_jobs SET plan_json=?2,updated_at={NOW} WHERE id=?1"),
        params![
            job.id,
            serde_json::to_string(&job.plan).map_err(|e| e.to_string())?
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn fixture() -> (tempfile::TempDir, Database, VideoJob) {
        let (dir, db, mut input) = super::super::tests::fixture();
        db.with_connection(|c| {
            c.execute("INSERT INTO model_providers(id,name,provider_type,base_url,model_name,is_default,is_enabled,model_kind,created_at,updated_at) VALUES('ink-image','test','openai-compatible','http://localhost/v1','ink-model',0,1,'image','now','now')",[]).unwrap();
            Ok(())
        }).unwrap();
        input.theme = Some("ink".into());
        input.image_provider_id = Some("ink-image".into());
        let job = db.create_video_job(input).unwrap();
        let mut job = db
            .claim_video_job(&job.id, VideoWork::Prepare, None)
            .unwrap();
        job.plan.duration = 8.0;
        job.plan.cues = serde_json::from_value(
            json!([{"start":0,"end":4,"text":"江岸读书"},{"start":4,"end":8,"text":"远山渐隐"}]),
        )
        .unwrap();
        job.plan.scenes = serde_json::from_value(json!([
            {"first":0,"last":0,"kind":"opening","title":"江岸","painting":{"description":"近景水墨皴擦细致描绘江边书案的木纹和书卷纸边；中景读书人的侧影保持清楚，远景层层淡墨山峰衬托开阔水面，留出自然呼吸的空间，主体动作与山水关系明确而连贯，并强调前后层次与留白韵律。","motion":"push_in"}},
            {"first":1,"last":1,"kind":"closing","title":"远山","painting":{"description":"近景延续书卷与江岸的细密笔触，中景水面以淡墨自然流向远方，远景群山逐层淡去，保留细致山石纹理和宣纸颗粒，不添加旁白之外的情节，并留出空间表现墨色深浅变化以及景深的自然转折。","motion":"pan_left"}}
        ])).unwrap();
        db.save_video_progress(&job.id, job.run_id, &job.plan, "storyboard")
            .unwrap();
        (dir, db, job)
    }

    #[test]
    fn ink_images_attach_atomically_and_resume_without_regenerating_finished_scenes() {
        let (_dir, db, job) = fixture();
        assert!(VideoWork::Render.window(&job.plan).is_err());
        let task = db.prepare_video_painting(&job.id, job.run_id, 0).unwrap();
        let prepared = db.prepare_image_generation(&task, "video_ink").unwrap();
        assert!(prepared.prompt.contains("皴擦"));
        assert!(prepared.prompt.contains("江岸读书"));
        let bytes = crate::ai::images::test_image(image::ImageFormat::Png, 1920, 1080);
        let asset = db
            .finish_image_generation(&task, &bytes, "video_ink")
            .unwrap();
        let saved = db.video_job(&job.id).unwrap();
        assert_eq!(
            saved.plan.scenes[0].image.as_ref().unwrap().asset_id,
            asset.id
        );
        assert_eq!(
            saved.plan.scenes[0]
                .painting
                .as_ref()
                .unwrap()
                .task_id
                .as_deref(),
            Some(task.as_str())
        );
        assert!(db.prepare_video_painting(&job.id, job.run_id, 0).is_err());
        let second = db.prepare_video_painting(&job.id, job.run_id, 1).unwrap();
        db.prepare_image_generation(&second, "video_ink").unwrap();
        db.end_generation(&second, "failed", "timeout", "测试超时")
            .unwrap();
        db.end_video_job(&job.id, job.run_id, "failed", "画作失败")
            .unwrap();
        let resumed = db
            .claim_video_job(&job.id, VideoWork::Prepare, None)
            .unwrap();
        assert_eq!(
            db.prepare_video_painting(&job.id, resumed.run_id, 1)
                .unwrap(),
            second
        );
        db.prepare_image_generation(&second, "video_ink").unwrap();
        db.finish_image_generation(&second, &bytes, "video_ink")
            .unwrap();
        let saved = db.video_job(&job.id).unwrap();
        assert_eq!(
            saved.plan.scenes[0].image.as_ref().unwrap().asset_id,
            asset.id
        );
        assert!(VideoWork::Render.window(&saved.plan).is_ok());
        assert_eq!(
            db.video_image_snapshots(&saved.project_id, &saved.plan.scenes, None)
                .unwrap()
                .len(),
            2
        );
    }

    #[test]
    fn saving_camera_edits_keeps_the_failed_painting_task_for_resume() {
        let (_dir, db, job) = fixture();
        let task = db.prepare_video_painting(&job.id, job.run_id, 0).unwrap();
        db.end_generation(&task, "failed", "timeout", "超时")
            .unwrap();
        db.end_video_job(&job.id, job.run_id, "failed", "超时")
            .unwrap();
        let mut scenes = db.video_job(&job.id).unwrap().plan.scenes;
        scenes[0].painting.as_mut().unwrap().motion = "pull_out".into();
        db.edit_video_plan(&job.id, job.plan.cues, scenes).unwrap();
        let resumed = db
            .claim_video_job(&job.id, VideoWork::Prepare, None)
            .unwrap();
        assert_eq!(
            db.prepare_video_painting(&job.id, resumed.run_id, 0)
                .unwrap(),
            task
        );
    }

    #[test]
    fn stopping_video_rejects_late_paintings_without_registering_assets() {
        let (_dir, db, job) = fixture();
        let task = db.prepare_video_painting(&job.id, job.run_id, 0).unwrap();
        db.prepare_image_generation(&task, "video_ink").unwrap();
        db.end_video_job(&job.id, job.run_id, "cancelled", "停止")
            .unwrap();
        let bytes = crate::ai::images::test_image(image::ImageFormat::Png, 1920, 1080);
        assert!(db
            .finish_image_generation(&task, &bytes, "video_ink")
            .is_err());
        assert!(db.video_job(&job.id).unwrap().plan.scenes[0]
            .image
            .is_none());
    }

    #[tokio::test]
    async fn ink_http_contract_saves_each_image_and_retries_only_the_failed_scene() {
        use crate::{
            ai::ActiveTasks, cover_commands::execute_image_task, credentials::SystemCredentials,
        };
        use base64::{engine::general_purpose::STANDARD, Engine};
        let (_dir, db, job) = fixture();
        let active = ActiveTasks::default();
        let vault = SystemCredentials("com.shuyin.ink-contract".into());
        let bytes = crate::ai::images::test_image(image::ImageFormat::Png, 1920, 1080);
        let response = json!({"data":[{"b64_json":STANDARD.encode(&bytes)}]}).to_string();
        let mut tasks = Vec::new();
        for (index, success) in [(0, true), (1, false), (1, true)] {
            let (url, server) = crate::ai::tests::server(
                if success { 200 } else { 500 },
                if success {
                    &response
                } else {
                    "fixture timeout"
                },
                std::time::Duration::ZERO,
            );
            db.with_connection(|c| {
                c.execute(
                    "UPDATE model_providers SET base_url=?1 WHERE id='ink-image'",
                    [url],
                )
                .map_err(|e| e.to_string())?;
                Ok(())
            })
            .unwrap();
            let current = db.video_job(&job.id).unwrap();
            let task = db
                .prepare_video_painting(&job.id, current.run_id, index)
                .unwrap();
            let result =
                execute_image_task(None, &db, &vault, &active, &task, "video_ink", None).await;
            let received = server.join().unwrap();
            assert!(received.starts_with("POST /v1/images/generations HTTP/1.1"));
            let body: serde_json::Value =
                serde_json::from_str(received.split("\r\n\r\n").nth(1).unwrap()).unwrap();
            assert_eq!(body["model"], "ink-model");
            assert_eq!(body["size"], "1920x1080");
            assert_eq!(body["n"], 1);
            assert!(body["prompt"].as_str().unwrap().contains(if index == 0 {
                "江岸读书"
            } else {
                "远山渐隐"
            }));
            if success {
                assert!(result.is_ok());
            } else {
                assert!(result.is_err());
                db.end_video_job(&job.id, current.run_id, "failed", "图片服务失败")
                    .unwrap();
                db.claim_video_job(&job.id, VideoWork::Prepare, None)
                    .unwrap();
            }
            tasks.push(task);
        }
        assert_ne!(tasks[0], tasks[1]);
        assert_eq!(tasks[1], tasks[2]);
        let saved = db.video_job(&job.id).unwrap();
        assert_eq!(
            saved
                .plan
                .scenes
                .iter()
                .filter(|scene| scene.image.is_some())
                .count(),
            2
        );
        assert!(VideoWork::Render.window(&saved.plan).is_ok());
        let calls = db.list_model_calls(1, 20).unwrap();
        assert_eq!(calls.rows.len(), 3);
        let calls = serde_json::to_value(calls.rows).unwrap();
        assert!(calls
            .as_array()
            .unwrap()
            .iter()
            .any(|call| call["status"] == "failed"));
        assert!(calls
            .as_array()
            .unwrap()
            .iter()
            .any(|call| call["status"] == "succeeded"));
    }

    #[test]
    fn ink_restart_preserves_finished_art_and_rebinds_only_the_interrupted_task() {
        let (dir, db, job) = fixture();
        let first = db.prepare_video_painting(&job.id, job.run_id, 0).unwrap();
        db.prepare_image_generation(&first, "video_ink").unwrap();
        let bytes = crate::ai::images::test_image(image::ImageFormat::Png, 1920, 1080);
        let art = db
            .finish_image_generation(&first, &bytes, "video_ink")
            .unwrap();
        let pending = db.prepare_video_painting(&job.id, job.run_id, 1).unwrap();
        db.prepare_image_generation(&pending, "video_ink").unwrap();
        drop(db);
        let db = Database::open(dir.path().join("data.sqlite3"));
        db.recover_ai_tasks().unwrap();
        db.recover_video_jobs().unwrap();
        let resumed = db
            .claim_video_job(&job.id, VideoWork::Prepare, None)
            .unwrap();
        assert_eq!(
            resumed.plan.scenes[0].image.as_ref().unwrap().asset_id,
            art.id
        );
        assert_eq!(
            db.read_video_image(&job.project_id, &art.id).unwrap(),
            bytes
        );
        assert_eq!(
            db.prepare_video_painting(&job.id, resumed.run_id, 1)
                .unwrap(),
            pending
        );
        db.prepare_image_generation(&pending, "video_ink").unwrap();
        db.finish_image_generation(&pending, &bytes, "video_ink")
            .unwrap();
        db.end_video_job(&job.id, resumed.run_id, "awaiting_confirmation", "")
            .unwrap();
        let saved = db.video_job(&job.id).unwrap();
        let mut changed = saved.plan.scenes.clone();
        changed[1]
            .painting
            .as_mut()
            .unwrap()
            .description
            .push_str("近景山石纹理更加清楚。");
        db.edit_video_plan(&job.id, saved.plan.cues.clone(), changed)
            .unwrap();
        let edited = db.video_job(&job.id).unwrap();
        assert_eq!(
            edited.plan.scenes[0].image.as_ref().unwrap().asset_id,
            art.id
        );
        assert!(edited.plan.scenes[1].image.is_none());
        assert!(edited.plan.scenes[1]
            .painting
            .as_ref()
            .unwrap()
            .task_id
            .is_none());
        assert!(VideoWork::Render.window(&edited.plan).is_err());
    }

    #[test]
    fn changing_the_transcript_invalidates_ink_directions_and_every_derived_painting() {
        let (_dir, db, job) = fixture();
        let claimed = db.video_job(&job.id).unwrap();
        let bytes = crate::ai::images::test_image(image::ImageFormat::Png, 1920, 1080);
        for index in 0..2 {
            let task = db
                .prepare_video_painting(&job.id, job.run_id, index)
                .unwrap();
            db.prepare_image_generation(&task, "video_ink").unwrap();
            db.finish_image_generation(&task, &bytes, "video_ink")
                .unwrap();
        }
        let rendered = db.video_job(&job.id).unwrap();
        db.end_video_job(&job.id, job.run_id, "awaiting_confirmation", "")
            .unwrap();
        let edited_cues = rendered
            .plan
            .cues
            .iter()
            .cloned()
            .map(|mut cue| {
                cue.text.push('。');
                cue
            })
            .collect();
        db.edit_video_plan(&job.id, edited_cues, rendered.plan.scenes)
            .unwrap();
        let current = db.video_job(&job.id).unwrap();
        assert!(current.plan.scenes.is_empty());
        assert!(VideoWork::Render.window(&current.plan).is_err());
        assert_eq!(claimed.plan.scenes.len(), 2);
    }
}
