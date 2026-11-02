use crate::{
    ai::{OpenAiCompatible, TextProvider, TIMEOUT},
    ai_commands,
    credentials::SystemCredentials,
    database::{
        video::{StartVideoInput, VideoJob},
        Database,
    },
    video::{self, Cue, PreviewRequest, Scene, VideoProgress, VideoSettings, VideoWork},
    video_process as process,
};
use serde::{Deserialize, Serialize};
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
pub struct VideoRuntime(Mutex<Option<ActiveVideo>>);

struct ActiveVideo {
    id: String,
    run_id: i64,
    flag: Arc<AtomicBool>,
    started_at: u64,
    directory: Option<PathBuf>,
}

#[derive(Serialize)]
pub struct VideoJobView {
    #[serde(flatten)]
    job: VideoJob,
    progress: Option<VideoProgress>,
    preview: Option<VideoPreview>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoPreview {
    start: f64,
    end: f64,
    signature: String,
    scenes: Vec<usize>,
}

fn preview_signature(plan: &video::VideoPlan) -> Result<String, String> {
    use sha2::{Digest, Sha256};
    let mut hash = Sha256::new();
    hash.update(serde_json::to_vec(plan).map_err(|e| e.to_string())?);
    hash.update(include_bytes!("../video/render.mjs"));
    hash.update(include_bytes!("../video/illustrations.js"));
    hash.update(include_bytes!("../video/visual-catalog.json"));
    Ok(hash
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

fn preview_directory(database: &Database, job: &VideoJob) -> Result<PathBuf, String> {
    Ok(
        Path::new(&database.get_project(&job.project_id)?.workspace_path)
            .join("video")
            .join(&job.id)
            .join(format!("preview-{}", job.run_id)),
    )
}

fn cached_preview(database: &Database, job: &VideoJob) -> Option<VideoPreview> {
    if job.status != "awaiting_confirmation" || job.stage != "previewing" {
        return None;
    }
    let directory = preview_directory(database, job).ok()?;
    let output: serde_json::Value =
        serde_json::from_str(&process::read_small(&directory.join("render-output.json")).ok()?)
            .ok()?;
    let preview: VideoPreview = serde_json::from_value(output.get("preview")?.clone()).ok()?;
    if database
        .video_image_snapshots(&job.project_id, &job.plan.scenes)
        .is_err()
        || preview.signature != preview_signature(&job.plan).ok()?
        || !preview.start.is_finite()
        || !preview.end.is_finite()
        || preview.start < 0.0
        || preview.end > job.plan.duration
        || preview.end <= preview.start
        || preview.end - preview.start > 60.0
        || preview.scenes.len() > 12
        || !directory.join("video.mp4").is_file()
        || preview.scenes.iter().any(|i| {
            *i >= job.plan.scenes.len() || !directory.join(format!("scene-{i}.png")).is_file()
        })
    {
        return None;
    }
    Some(preview)
}

fn active_progress(active: &ActiveVideo, job: &VideoJob) -> Option<VideoProgress> {
    if job.status != "running" || active.id != job.id || active.run_id != job.run_id {
        return None;
    }
    let mut progress = active
        .directory
        .as_ref()
        .and_then(|dir| process::read_small(&dir.join("progress.json")).ok())
        .and_then(|raw| serde_json::from_str::<VideoProgress>(&raw).ok())
        .filter(|progress| progress.stage == job.stage)
        .unwrap_or_default();
    progress.started_at = active.started_at;
    progress.stage.clone_from(&job.stage);
    Some(progress)
}

impl VideoRuntime {
    pub fn stop_all(&self) {
        if let Ok(active) = self.0.lock() {
            if let Some(active) = &*active {
                active.flag.store(true, Ordering::SeqCst);
            }
        }
    }
}

#[tauri::command]
pub fn start_video_illustration(
    database: State<'_, Database>,
    input: crate::database::image_generation::ImageGenerationInput,
) -> Result<String, String> {
    database.start_video_illustration(input)
}
#[tauri::command]
pub async fn execute_video_illustration(
    database: State<'_, Database>,
    vault: State<'_, SystemCredentials>,
    active: State<'_, crate::ai::ActiveTasks>,
    id: String,
) -> Result<crate::database::Asset, crate::ai::AiError> {
    crate::cover_commands::execute_image_task(
        database.inner(),
        vault.inner(),
        active.inner(),
        &id,
        "video_illustration",
    )
    .await
}
#[tauri::command]
pub fn import_video_image(
    database: State<'_, Database>,
    project_id: String,
    path: String,
) -> Result<String, String> {
    database.import_video_image(&project_id, &path)
}

#[tauri::command]
pub fn read_video_image(
    database: State<'_, Database>,
    project_id: String,
    asset_id: String,
) -> Result<tauri::ipc::Response, String> {
    database
        .read_video_image(&project_id, &asset_id)
        .map(tauri::ipc::Response::new)
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
    runtime: State<'_, VideoRuntime>,
    project_id: String,
) -> Result<Vec<VideoJobView>, String> {
    let jobs = database.video_jobs(&project_id)?;
    let active = runtime.0.lock().map_err(|e| e.to_string())?;
    Ok(jobs
        .into_iter()
        .map(|job| VideoJobView {
            preview: cached_preview(&database, &job),
            progress: active
                .as_ref()
                .and_then(|active| active_progress(active, &job)),
            job,
        })
        .collect())
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
    let job = db.claim_video_job(&job.id, VideoWork::Prepare, None)?;
    spawn(&app, &mut active, &job, VideoWork::Prepare);
    Ok(job)
}

#[tauri::command]
pub fn continue_video_job(
    app: AppHandle,
    id: String,
    render: bool,
    provider_id: Option<String>,
) -> Result<VideoJob, String> {
    if !cfg!(target_os = "windows") {
        return Err("MG 视频首版暂仅支持 Windows。".into());
    }
    let runtime = app.state::<VideoRuntime>();
    let mut active = runtime.0.lock().map_err(|e| e.to_string())?;
    if active.is_some() {
        return Err("已有视频任务正在执行或停止中。".into());
    }
    if !render && provider_id.is_none() {
        return Err("请选择已启用的文本模型生成分镜。".into());
    }
    let work = if render {
        VideoWork::Render
    } else {
        VideoWork::Prepare
    };
    let job = app
        .state::<Database>()
        .claim_video_job(&id, work, provider_id.as_deref())?;
    spawn(&app, &mut active, &job, work);
    Ok(job)
}

#[tauri::command]
pub fn preview_video_job(
    app: AppHandle,
    id: String,
    request: PreviewRequest,
) -> Result<VideoJob, String> {
    if !cfg!(target_os = "windows") {
        return Err("MG 视频首版暂仅支持 Windows。".into());
    }
    let runtime = app.state::<VideoRuntime>();
    let mut active = runtime.0.lock().map_err(|e| e.to_string())?;
    if active.is_some() {
        return Err("已有视频任务正在执行或停止中。".into());
    }
    let work = VideoWork::Preview(request);
    let job = app.state::<Database>().claim_video_job(&id, work, None)?;
    spawn(&app, &mut active, &job, work);
    Ok(job)
}

#[tauri::command]
pub fn cancel_video_job(app: AppHandle, id: String) -> Result<(), String> {
    let runtime = app.state::<VideoRuntime>();
    let active = runtime.0.lock().map_err(|e| e.to_string())?;
    if let Some(active) = &*active {
        if active.id == id {
            let db = app.state::<Database>();
            let job = db.video_job(&id)?;
            db.end_video_job(
                &id,
                job.run_id,
                "cancelled",
                "用户已停止任务，已保存内容保留。",
            )?;
            active.flag.store(true, Ordering::SeqCst);
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

#[tauri::command]
pub fn video_preview_media(
    app: AppHandle,
    project_id: String,
    id: String,
    run_id: i64,
    scene: Option<usize>,
) -> Result<String, String> {
    let db = app.state::<Database>();
    let job = db.video_job(&id)?;
    if job.project_id != project_id || job.run_id != run_id {
        return Err("试片不属于本书或已失效，请重新预览。".into());
    }
    let preview = cached_preview(&db, &job).ok_or("试片已失效或文件缺失，请重新预览。")?;
    let name = match scene {
        Some(index) if preview.scenes.contains(&index) => format!("scene-{index}.png"),
        Some(_) => return Err("该分镜没有可用的试片缩略图。".into()),
        None => "video.mp4".into(),
    };
    let directory =
        std::fs::canonicalize(preview_directory(&db, &job)?).map_err(|e| e.to_string())?;
    let path = std::fs::canonicalize(directory.join(name)).map_err(|e| e.to_string())?;
    if !path.starts_with(&directory) {
        return Err("试片文件不在本次渲染目录中。".into());
    }
    app.asset_protocol_scope()
        .allow_file(&path)
        .map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().into_owned())
}

fn spawn(app: &AppHandle, active: &mut Option<ActiveVideo>, job: &VideoJob, work: VideoWork) {
    let flag = Arc::new(AtomicBool::new(false));
    *active = Some(ActiveVideo {
        id: job.id.clone(),
        run_id: job.run_id,
        flag: flag.clone(),
        started_at: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64,
        directory: None,
    });
    let app = app.clone();
    let job = job.clone();
    tauri::async_runtime::spawn(async move {
        let result = execute(&app, job.clone(), work, flag.clone()).await;
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
    work: VideoWork,
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
    let directory = if matches!(work, VideoWork::Preview(_)) {
        root.join(format!("preview-{}", job.run_id))
    } else {
        root.join(format!(
            "run-{}-{}",
            job.run_id,
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|e| e.to_string())?
                .as_nanos()
        ))
    };
    std::fs::create_dir(&directory).map_err(|e| e.to_string())?;
    if let Some(active) = app
        .state::<VideoRuntime>()
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .as_mut()
    {
        if active.id == job.id && active.run_id == job.run_id {
            active.directory = Some(directory.clone());
        }
    }
    process::write_new(
        &directory.join("render.mjs"),
        include_bytes!("../video/render.mjs"),
    )?;
    process::write_new(
        &directory.join("illustrations.js"),
        include_bytes!("../video/illustrations.js"),
    )?;
    process::write_new(
        &directory.join("visual-catalog.json"),
        include_bytes!("../video/visual-catalog.json"),
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
    if !matches!(work, VideoWork::Prepare) {
        let window = work.window(&job.plan)?;
        let snapshots = db.video_image_snapshots(&job.project_id, &job.plan.scenes)?;
        let mut images = serde_json::Map::new();
        for (id, snapshot) in snapshots {
            let path = directory.join(format!("image-{}.{}", snapshot.sha256, snapshot.extension));
            if !path.exists() {
                process::write_new(&path, &snapshot.bytes)?;
            }
            images.insert(id, serde_json::json!({"path":process::external_path(&path), "sha256":snapshot.sha256, "extension":snapshot.extension}));
        }
        db.save_video_progress(&job.id, job.run_id, &job.plan, work.stage())?;
        let input = serde_json::json!({"action":if window.is_some() {"preview"} else {"render"}, "window":window, "signature":preview_signature(&job.plan)?, "audioPath":audio_external, "engineRoot":engine_external, "cacheRoot":process::external_path(&root.join("cache")), "plan":job.plan, "images":images});
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
        db.video_image_snapshots(&job.project_id, &job.plan.scenes)?;
        if let Some(window) = window {
            let metadata = crate::media::inspect(&directory.join("video.mp4"))?;
            if !metadata.has_audio
                || metadata.width != 1920
                || metadata.height != 1080
                || (metadata.duration_seconds - (window.end - window.start)).abs() > 0.2
            {
                return Err("试片尺寸、时长或音轨未通过校验。".into());
            }
            db.end_video_job(&job.id, job.run_id, "awaiting_confirmation", "")?;
            return Ok(());
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
        let end = video::storyboard_batch_end(&job.plan.cues, first);
        let user = video::storyboard_input(&job.plan, first, end);
        let request = ai_commands::request(
            config.clone(),
            app.state::<SystemCredentials>().inner(),
            &video::storyboard_prompt(),
            user,
        )
        .map_err(|e| e.message)?;
        let provider = OpenAiCompatible::new(TIMEOUT, &request.base_url).map_err(|e| e.message)?;
        let raw = tokio::select! { biased; _ = cancelled(flag.clone()) => return Err("视频任务已停止。".into()), response = provider.generate(request) => response.map_err(|e| e.message)? };
        let scenes: Vec<Scene> = serde_json::from_str(&raw)
            .map_err(|_| "模型分镜不是合法 JSON，已保留字幕和此前分镜。")?;
        video::validate_scenes(&scenes, first, end)?;
        if scenes.iter().any(|scene| scene.image.is_some()) {
            return Err("分镜模型只能选择手绘目录；关键图片须由用户选用并确认来源。".into());
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(directory: &Path) -> (ActiveVideo, VideoJob) {
        (
            ActiveVideo {
                id: "job".into(),
                run_id: 2,
                flag: Arc::new(AtomicBool::new(false)),
                started_at: 1234,
                directory: Some(directory.to_path_buf()),
            },
            VideoJob {
                id: "job".into(),
                project_id: "book".into(),
                audio_asset_id: None,
                source_subtitle_id: None,
                provider_id: None,
                status: "running".into(),
                stage: "transcribing".into(),
                run_id: 2,
                plan: Default::default(),
                error: String::new(),
                subtitle_asset_id: None,
                video_asset_id: None,
                created_at: String::new(),
                updated_at: String::new(),
            },
        )
    }

    #[test]
    fn progress_is_scoped_to_the_active_job_run_and_stage() {
        let directory = tempfile::tempdir().unwrap();
        let (active, mut job) = fixture(directory.path());
        std::fs::write(directory.path().join("progress.json"), r#"{"stage":"transcribing","phase":"recognizing","startedAt":99,"completed":30,"total":100,"cueCount":3,"preview":[{"start":27,"end":30,"text":"preview"}]}"#).unwrap();
        let progress = active_progress(&active, &job).unwrap();
        assert_eq!(progress.started_at, 1234);
        assert_eq!(progress.completed, 30.0);
        assert_eq!(progress.preview[0].text, "preview");
        job.stage = "storyboard".into();
        let progress = active_progress(&active, &job).unwrap();
        assert_eq!(progress.started_at, 1234);
        assert!(progress.preview.is_empty());
        assert!(progress.phase.is_empty());
        job.run_id = 3;
        assert!(active_progress(&active, &job).is_none());
        job.run_id = 2;
        job.id = "other-job".into();
        assert!(active_progress(&active, &job).is_none());
        job.id = "job".into();
        job.status = "cancelled".into();
        assert!(active_progress(&active, &job).is_none());
    }

    #[test]
    fn missing_or_invalid_telemetry_keeps_the_job_and_run_clock_available() {
        let directory = tempfile::tempdir().unwrap();
        let (active, job) = fixture(directory.path());
        assert_eq!(active_progress(&active, &job).unwrap().started_at, 1234);
        std::fs::write(directory.path().join("progress.json"), "incomplete").unwrap();
        let progress = active_progress(&active, &job).unwrap();
        assert_eq!(progress.started_at, 1234);
        assert_eq!(progress.completed, 0.0);
        assert!(progress.preview.is_empty());
    }

    #[test]
    fn rendering_frame_counts_are_exposed_in_the_existing_job_view() {
        let directory = tempfile::tempdir().unwrap();
        let (active, mut job) = fixture(directory.path());
        job.stage = "rendering".into();
        std::fs::write(
            directory.path().join("progress.json"),
            r#"{"stage":"rendering","phase":"rendering_frames","completed":144,"total":480}"#,
        )
        .unwrap();
        let view = serde_json::to_value(VideoJobView {
            progress: active_progress(&active, &job),
            preview: None,
            job,
        })
        .unwrap();
        assert_eq!(view["id"], "job");
        assert_eq!(view["progress"]["startedAt"], 1234);
        assert_eq!(view["progress"]["phase"], "rendering_frames");
        assert_eq!(view["progress"]["completed"], 144.0);
        assert_eq!(view["progress"]["total"], 480.0);
        assert_eq!(view["progress"]["preview"], serde_json::json!([]));
    }

    #[test]
    fn subtitle_encoding_telemetry_keeps_its_units_speed_and_timestamp() {
        let directory = tempfile::tempdir().unwrap();
        let (active, mut job) = fixture(directory.path());
        job.stage = "rendering".into();
        std::fs::write(
            directory.path().join("progress.json"),
            r#"{"stage":"rendering","phase":"rendering_subtitles","completed":41.5,"total":100,"unit":"seconds","speed":1.25,"updatedAt":60000}"#,
        )
        .unwrap();
        let progress = serde_json::to_value(active_progress(&active, &job).unwrap()).unwrap();
        assert_eq!(progress["startedAt"], 1234);
        assert_eq!(progress["completed"], 41.5);
        assert_eq!(progress["unit"], "seconds");
        assert_eq!(progress["speed"], 1.25);
        assert_eq!(progress["updatedAt"], 60000);
    }
}
