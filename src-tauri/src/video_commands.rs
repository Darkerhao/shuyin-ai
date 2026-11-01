use crate::{
    ai::{OpenAiCompatible, TextProvider, TIMEOUT},
    ai_commands,
    credentials::SystemCredentials,
    database::{
        video::{StartVideoInput, VideoJob},
        Database,
    },
    video::{self, Cue, Scene, VideoSettings},
    video_process as process,
};
use serde::Deserialize;
use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Manager, State};

#[derive(Default)]
pub struct VideoRuntime(Mutex<Option<(String, Arc<AtomicBool>)>>);
impl VideoRuntime {
    pub fn stop_all(&self) {
        if let Ok(active) = self.0.lock() {
            if let Some((_, flag)) = &*active {
                flag.store(true, Ordering::SeqCst);
            }
        }
    }
}

#[tauri::command]
pub fn video_settings(database: State<'_, Database>) -> Result<VideoSettings, String> {
    database.video_settings()
}
#[tauri::command]
pub fn save_video_settings(
    database: State<'_, Database>,
    settings: VideoSettings,
) -> Result<(), String> {
    database.save_video_settings(&settings)
}
#[tauri::command]
pub fn list_video_jobs(
    database: State<'_, Database>,
    project_id: String,
) -> Result<Vec<VideoJob>, String> {
    database.video_jobs(&project_id)
}

#[tauri::command]
pub fn start_video_job(app: AppHandle, input: StartVideoInput) -> Result<VideoJob, String> {
    if !cfg!(target_os = "windows") {
        return Err("MG 视频首版暂仅支持 Windows。".into());
    }
    let runtime = app.state::<VideoRuntime>();
    let mut active = runtime.0.lock().map_err(|e| e.to_string())?;
    if active.is_some() {
        return Err("已有视频任务正在执行。".into());
    }
    let db = app.state::<Database>();
    let job = db.create_video_job(input)?;
    let job = db.claim_video_job(&job.id, false)?;
    spawn(&app, &mut active, &job, false);
    Ok(job)
}

#[tauri::command]
pub fn continue_video_job(app: AppHandle, id: String, render: bool) -> Result<VideoJob, String> {
    if !cfg!(target_os = "windows") {
        return Err("MG 视频首版暂仅支持 Windows。".into());
    }
    let runtime = app.state::<VideoRuntime>();
    let mut active = runtime.0.lock().map_err(|e| e.to_string())?;
    if active.is_some() {
        return Err("已有视频任务正在执行或停止中。".into());
    }
    let job = app.state::<Database>().claim_video_job(&id, render)?;
    spawn(&app, &mut active, &job, render);
    Ok(job)
}

#[tauri::command]
pub fn cancel_video_job(app: AppHandle, id: String) -> Result<(), String> {
    let runtime = app.state::<VideoRuntime>();
    let active = runtime.0.lock().map_err(|e| e.to_string())?;
    if let Some((active_id, flag)) = &*active {
        if active_id == &id {
            let db = app.state::<Database>();
            let job = db.video_job(&id)?;
            db.end_video_job(
                &id,
                job.run_id,
                "cancelled",
                "用户已停止任务，已保存内容保留。",
            )?;
            flag.store(true, Ordering::SeqCst);
            return Ok(());
        }
    }
    Err("该视频没有正在执行的进程。".into())
}

#[tauri::command]
pub fn save_video_plan(
    database: State<'_, Database>,
    id: String,
    subtitles: String,
    scenes: Vec<Scene>,
) -> Result<(), String> {
    let job = database.video_job(&id)?;
    let cues = video::parse_srt(&subtitles, job.plan.duration)?;
    database.edit_video_plan(&id, cues, scenes)
}

#[tauri::command]
pub fn video_subtitles(database: State<'_, Database>, id: String) -> Result<String, String> {
    Ok(video::to_srt(&database.video_job(&id)?.plan.cues))
}

#[tauri::command]
pub fn export_video_subtitles(database: State<'_, Database>, id: String) -> Result<String, String> {
    database.export_video_subtitles(&id)
}

#[tauri::command]
pub fn media_asset_path(
    app: AppHandle,
    project_id: String,
    asset_id: String,
) -> Result<String, String> {
    let db = app.state::<Database>();
    let asset = db
        .list_assets(&project_id)?
        .into_iter()
        .find(|a| a.id == asset_id && matches!(a.asset_type.as_str(), "audio" | "video"))
        .ok_or("只能预览本书已登记的音频或视频。")?;
    let path = db.video_asset_path(&project_id, &asset_id, &asset.asset_type)?;
    app.asset_protocol_scope()
        .allow_file(&path)
        .map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().into_owned())
}

fn spawn(
    app: &AppHandle,
    active: &mut Option<(String, Arc<AtomicBool>)>,
    job: &VideoJob,
    render: bool,
) {
    let flag = Arc::new(AtomicBool::new(false));
    *active = Some((job.id.clone(), flag.clone()));
    let app = app.clone();
    let job = job.clone();
    tauri::async_runtime::spawn(async move {
        let result = execute(&app, job.clone(), render, flag.clone()).await;
        if let Err(error) = result {
            let status = if flag.load(Ordering::SeqCst) {
                "cancelled"
            } else {
                "failed"
            };
            let _ = app
                .state::<Database>()
                .end_video_job(&job.id, job.run_id, status, &error);
        }
        if let Ok(mut active) = app.state::<VideoRuntime>().0.lock() {
            *active = None;
        }
    });
}

async fn stage(
    settings: &VideoSettings,
    directory: &Path,
    name: &str,
    input: serde_json::Value,
    python: bool,
    flag: Arc<AtomicBool>,
    seconds: u64,
) -> Result<String, String> {
    let input_path = directory.join(format!("{name}-input.json"));
    let output = directory.join(format!("{name}-output.json"));
    process::write_new(
        &input_path,
        &serde_json::to_vec(&input).map_err(|e| e.to_string())?,
    )?;
    let script = directory.join(if python {
        "transcribe.py"
    } else {
        "render.mjs"
    });
    let exe = if python {
        settings.python_path.clone()
    } else {
        settings.node_path.clone()
    };
    tauri::async_runtime::spawn_blocking(move || {
        process::run(
            &exe,
            &script,
            &input_path,
            &output,
            flag,
            Duration::from_secs(seconds),
        )?;
        process::read_small(&output)
    })
    .await
    .map_err(|e| e.to_string())?
}

async fn cancelled(flag: Arc<AtomicBool>) {
    while !flag.load(Ordering::SeqCst) {
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

async fn digest(path: PathBuf) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || process::fingerprint(&path))
        .await
        .map_err(|e| e.to_string())?
}

async fn execute(
    app: &AppHandle,
    mut job: VideoJob,
    render: bool,
    flag: Arc<AtomicBool>,
) -> Result<(), String> {
    let db = app.state::<Database>();
    let settings = db.video_settings()?;
    let audio = db.video_asset_path(
        &job.project_id,
        job.audio_asset_id
            .as_deref()
            .ok_or("源音频关联已被移除。")?,
        "audio",
    )?;
    let hash = digest(audio.clone()).await?;
    let audio_external = process::external_path(&audio);
    let engine_external = process::external_path(Path::new(&settings.engine_root));
    if !job.plan.audio_sha256.is_empty() && job.plan.audio_sha256 != hash {
        return Err("原音频已变化，请新建视频任务，不能沿用旧字幕。".into());
    }
    job.plan.audio_sha256 = hash;
    let project = db.get_project(&job.project_id)?;
    let root = Path::new(&project.workspace_path)
        .join("video")
        .join(&job.id);
    std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    let directory = root.join(format!(
        "run-{}-{}",
        job.run_id,
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_nanos()
    ));
    std::fs::create_dir(&directory).map_err(|e| e.to_string())?;
    process::write_new(
        &directory.join("render.mjs"),
        include_bytes!("../video/render.mjs"),
    )?;
    process::write_new(
        &directory.join("transcribe.py"),
        include_bytes!("../video/transcribe.py"),
    )?;
    let base = serde_json::json!({"action":"probe", "audioPath": audio_external, "engineRoot":engine_external});
    let raw = stage(
        &settings,
        &directory,
        "probe",
        base,
        false,
        flag.clone(),
        60,
    )
    .await?;
    #[derive(Deserialize)]
    struct Probe {
        duration: f64,
    }
    let probe: Probe = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
    if job.plan.duration == 0.0 {
        job.plan.duration = probe.duration;
    }
    if (probe.duration - job.plan.duration).abs() > 0.15 {
        return Err("音频时长与任务不一致。".into());
    }
    if render {
        db.save_video_progress(&job.id, job.run_id, &job.plan, "rendering")?;
        video::validate_cues(&job.plan.cues, job.plan.duration)?;
        video::validate_scenes(&job.plan.scenes, 0, job.plan.cues.len())?;
        let input = serde_json::json!({"action":"render", "audioPath":audio_external, "engineRoot":engine_external, "cacheRoot":process::external_path(&root.join("cache")), "plan":job.plan});
        stage(
            &settings,
            &directory,
            "render",
            input,
            false,
            flag.clone(),
            21600,
        )
        .await?;
        if flag.load(Ordering::SeqCst) {
            return Err("视频任务已停止。".into());
        }
        if digest(audio).await? != job.plan.audio_sha256 {
            return Err("渲染期间源音频发生变化，未登记成片。".into());
        }
        db.save_video_progress(&job.id, job.run_id, &job.plan, "validating")?;
        let subtitle = directory.join("subtitles.srt");
        process::write_new(&subtitle, video::to_srt(&job.plan.cues).as_bytes())?;
        db.finish_video_job(&job.id, job.run_id, &directory.join("video.mp4"), &subtitle)?;
        return Ok(());
    }
    if job.plan.cues.is_empty() {
        db.save_video_progress(&job.id, job.run_id, &job.plan, "transcribing")?;
        job.plan.cues = if let Some(id) = &job.source_subtitle_id {
            let path = db.video_asset_path(&job.project_id, id, "subtitle")?;
            video::parse_srt(&process::read_small(&path)?, job.plan.duration)?
        } else {
            if !Path::new(&settings.model_path).join("model.bin").is_file() {
                return Err(
                    "请先在视频运行环境中选择已下载的 faster-whisper 多语言模型目录。".into(),
                );
            }
            let input = serde_json::json!({"audioPath":audio_external, "modelPath":process::external_path(Path::new(&settings.model_path)), "context":format!("{} {}", project.input.title, project.input.author)});
            let raw = stage(
                &settings,
                &directory,
                "transcription",
                input,
                true,
                flag.clone(),
                7200,
            )
            .await?;
            #[derive(Deserialize)]
            struct Transcript {
                duration: f64,
                cues: Vec<Cue>,
            }
            let transcript: Transcript = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
            if (transcript.duration - job.plan.duration).abs() > 0.15 {
                return Err("转写音频时长不一致。".into());
            }
            video::validate_cues(&transcript.cues, job.plan.duration)?;
            transcript.cues
        };
        if digest(audio.clone()).await? != job.plan.audio_sha256 {
            return Err("转写期间音频发生变化，未采用字幕。".into());
        }
        db.save_video_progress(&job.id, job.run_id, &job.plan, "storyboard")?;
    }
    if flag.load(Ordering::SeqCst) {
        return Err("视频任务已停止。".into());
    }
    let subtitle = directory.join("transcript.srt");
    process::write_new(&subtitle, video::to_srt(&job.plan.cues).as_bytes())?;
    db.register_video_subtitle(&job.id, job.run_id, &subtitle)?;
    let mut first = job.plan.scenes.last().map_or(0, |s| s.last + 1);
    if first > 0 {
        video::validate_scenes(&job.plan.scenes, 0, first)?;
    }
    let config = db.get_provider(
        job.provider_id
            .as_deref()
            .ok_or("分镜模型已被移除，请新建任务。")?,
    )?;
    while first < job.plan.cues.len() {
        if flag.load(Ordering::SeqCst) {
            return Err("视频任务已停止。".into());
        }
        let end = (first + video::BATCH_SIZE).min(job.plan.cues.len());
        let user = serde_json::json!({"book": job.plan.title, "cues": job.plan.cues[first..end].iter().enumerate().map(|(offset,c)| serde_json::json!({"index":first+offset,"text":c.text})).collect::<Vec<_>>()}).to_string();
        let request = ai_commands::request(
            config.clone(),
            app.state::<SystemCredentials>().inner(),
            video::PROMPT,
            user,
        )
        .map_err(|e| e.message)?;
        let provider = OpenAiCompatible::new(TIMEOUT, &request.base_url).map_err(|e| e.message)?;
        let raw = tokio::select! { biased; _ = cancelled(flag.clone()) => return Err("视频任务已停止。".into()), response = provider.generate(request) => response.map_err(|e| e.message)? };
        let scenes: Vec<Scene> = serde_json::from_str(&raw)
            .map_err(|_| "模型分镜不是合法 JSON，已保留字幕和此前分镜。")?;
        video::validate_scenes(&scenes, first, end)?;
        job.plan.scenes.extend(scenes);
        db.save_video_progress(&job.id, job.run_id, &job.plan, "storyboard")?;
        first = end;
    }
    if flag.load(Ordering::SeqCst) {
        return Err("视频任务已停止。".into());
    }
    db.end_video_job(&job.id, job.run_id, "awaiting_confirmation", "")?;
    Ok(())
}
