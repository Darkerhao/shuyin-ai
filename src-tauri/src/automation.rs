use crate::{
    ai::{
        publishing::TranscriptDraft, ActiveTasks, AiError, OpenAiCompatible, TextProvider,
        WritingContent, TIMEOUT,
    },
    ai_commands,
    audio::AudioAutomation,
    credentials::SystemCredentials,
    database::{
        automation::{AutomationRun, Slot, StartAutomationInput},
        image_generation::ImageGenerationInput,
        publishing::PublishInput,
        video::StartVideoInput,
        Database,
    },
    douyin::{self, DouyinPublisher},
    video_commands,
};
use std::sync::Mutex;
use tauri::{AppHandle, Manager, State, WebviewWindow};

#[derive(Default)]
pub struct AutomationRuntime(Mutex<Option<String>>);

fn local_main(window: &WebviewWindow) -> Result<(), String> {
    if window.label() != "main" {
        return Err("仅主工作台可以控制自动流程。".into());
    }
    Ok(())
}

#[tauri::command]
pub fn automation_snapshot(database: State<'_, Database>) -> Result<Vec<AutomationRun>, String> {
    database.automation_runs()
}

#[tauri::command]
pub async fn start_automation(
    app: AppHandle,
    window: WebviewWindow,
    input: StartAutomationInput,
) -> Result<AutomationRun, String> {
    local_main(&window)?;
    if app
        .state::<AutomationRuntime>()
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .is_some()
    {
        return Err("上一个自动流程仍在结束当前阶段，请稍后再开始。".into());
    }
    app.state::<AudioAutomation>().require_available()?;
    let account = douyin::flow::require_automation_account(&app.state::<DouyinPublisher>())?;
    let database = app.state::<Database>();
    for id in [&input.text_provider_id, &input.image_provider_id] {
        ai_commands::provider_key(
            &database.get_provider(id)?,
            &app.state::<SystemCredentials>(),
        )
        .map_err(|e| e.message)?;
    }
    let settings = database.video_settings()?;
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let database = handle.state::<Database>();
        let report = crate::video::environment::inspect(&database, settings, true)?;
        report.require_ready()?;
        database.save_video_settings(&report.settings)
    })
    .await
    .map_err(|e| e.to_string())??;
    let run = database.create_automation(input, account)?;
    if run.status == "running" {
        launch_or_pause(&app, &run)?;
    }
    Ok(run)
}

#[tauri::command]
pub async fn control_automation(
    app: AppHandle,
    window: WebviewWindow,
    id: String,
    action: String,
) -> Result<AutomationRun, String> {
    local_main(&window)?;
    let database = app.state::<Database>();
    if action == "resume" || action == "retry" {
        if app
            .state::<AutomationRuntime>()
            .0
            .lock()
            .map_err(|e| e.to_string())?
            .is_some()
        {
            return Err("当前阶段仍在结束，请稍后继续。已完成结果会保留。".into());
        }
        let run = database.automation_run(&id)?;
        if action == "retry" {
            database.retry_automation_generation(&run)?;
        }
        if run.stage == "publishing" {
            if let Some(task_id) = &run.publish_task_id {
                if database.renew_automatic_schedule(task_id)? {
                    database.prepare_publish_task(task_id)?;
                }
            }
        }
        if action == "resume" && run.status == "paused" {
            if let Some(id) = &run.audio_job_id {
                let audio = database.audio_job(id)?;
                if audio.status == "paused" {
                    let audio = database.control_audio_job(id, "resume")?;
                    let runtime = app.state::<AudioAutomation>();
                    runtime.open_notebook(&app, &audio, "resume").await?;
                    runtime.notify();
                }
            }
            if let Some(id) = &run.video_job_id {
                if database.video_job(id)?.status == "cancelled"
                    && matches!(run.stage.as_str(), "video_prepare" | "render")
                {
                    database.retry_automation_generation(&run)?;
                }
            }
        }
        let run = database.control_automation(&id, "resume")?;
        launch_or_pause(&app, &run)?;
        return Ok(run);
    }
    let run = database.control_automation(&id, &action)?;
    if let Some(id) = &run.audio_job_id {
        let job = database.audio_job(id)?;
        if !matches!(job.status.as_str(), "succeeded" | "cancelled")
            && (action == "cancel" || job.status != "paused")
        {
            database.control_audio_job(
                id,
                if action == "cancel" {
                    "cancel"
                } else {
                    "pause"
                },
            )?;
            let audio = app.state::<AudioAutomation>();
            audio.interrupt(&app);
            audio.notify();
        }
    }
    if let Some(id) = &run.video_job_id {
        if database.video_job(id)?.status == "running" {
            video_commands::cancel_video_job(app.clone(), id.clone())?;
        }
    }
    if action == "cancel" {
        for id in [
            &run.writing_task_id,
            &run.landscape_task_id,
            &run.portrait_task_id,
        ]
        .into_iter()
        .flatten()
        {
            database.end_generation(
                id,
                "cancelled",
                "cancelled",
                "自动流程已停止，已有成果保留。",
            )?;
            if let Some(sender) = app
                .state::<ActiveTasks>()
                .0
                .lock()
                .map_err(|e| e.to_string())?
                .get(id)
            {
                let _ = sender.send(true);
            }
        }
    }
    if run.stage == "publishing" {
        if let Some(session) = douyin::flow::automation_session(&app.state::<DouyinPublisher>())? {
            if Some(&session.task_id) == run.publish_task_id.as_ref() {
                douyin::flow::stop_douyin_task(
                    app.clone(),
                    window,
                    app.state::<DouyinPublisher>(),
                    session.run_id,
                )
                .await?;
            }
        }
    }
    Ok(run)
}

#[tauri::command]
pub fn automation_cover(
    database: State<'_, Database>,
    id: String,
) -> Result<crate::database::covers::CoverConfig, String> {
    let run = database.automation_run(&id)?;
    if run.status != "running" {
        return Err("自动流程已暂停。".into());
    }
    database.automation_cover_config(&run)
}

#[tauri::command]
pub fn complete_automation_cover(
    database: State<'_, Database>,
    id: String,
    run_id: i64,
    stage: String,
    png: Vec<u8>,
) -> Result<(), String> {
    let run = database.automation_run(&id)?;
    if run.run_id != run_id || run.status != "running" {
        return Err("封面结果已过期，原有成果保留。".into());
    }
    let (slot, existing) = match stage.as_str() {
        "landscape_cover" => (Slot::LandscapeCover, &run.landscape_cover_id),
        "portrait_cover" => (Slot::PortraitCover, &run.portrait_cover_id),
        _ => return Err("封面阶段无效。".into()),
    };
    if existing.is_some() {
        return Ok(());
    }
    if run.stage != stage {
        return Err("封面阶段已变化。".into());
    }
    let config = database.automation_cover_config(&run)?;
    database.save_bound_cover_version(
        &run.project_id,
        config,
        Some(&png),
        Some(&run.binding(slot)),
    )?;
    Ok(())
}

#[tauri::command]
pub fn fail_automation_cover(
    database: State<'_, Database>,
    id: String,
    run_id: i64,
    stage: String,
) -> Result<(), String> {
    database.fail_automation_cover(&id, run_id, &stage)
}

fn launch_or_pause(app: &AppHandle, run: &AutomationRun) -> Result<(), String> {
    if let Err(error) = launch(app, &run.id) {
        app.state::<Database>()
            .set_automation_status(run, "paused", &error)?;
        return Err(error);
    }
    Ok(())
}

fn launch(app: &AppHandle, id: &str) -> Result<(), String> {
    let runtime = app.state::<AutomationRuntime>();
    let mut active = runtime.0.lock().map_err(|e| e.to_string())?;
    if let Some(existing) = &*active {
        return if existing == id {
            Ok(())
        } else {
            Err("已有自动流程正在运行。".into())
        };
    }
    *active = Some(id.into());
    let handle = app.clone();
    let id = id.to_owned();
    tauri::async_runtime::spawn(async move {
        drive(&handle, &id).await;
        if let Ok(mut active) = handle.state::<AutomationRuntime>().0.lock() {
            *active = None;
        }
    });
    Ok(())
}

async fn drive(app: &AppHandle, id: &str) {
    loop {
        let database = app.state::<Database>();
        let Ok(run) = database.automation_run(id) else {
            return;
        };
        if run.status != "running" {
            return;
        }
        let result = match database.verify_automation_input(&run) {
            Ok(()) => advance(app, &run).await,
            Err(error) => Err(error),
        };
        if let Err(error) = result {
            let _ = database.set_automation_status(&run, "waiting_user", &error);
            return;
        }
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
}

async fn advance(app: &AppHandle, run: &AutomationRun) -> Result<(), String> {
    let database = app.state::<Database>();
    match run.stage.as_str() {
        "audio" => {
            let audio = app.state::<AudioAutomation>();
            let job = match &run.audio_job_id {
                Some(id) => database.audio_job(id)?,
                None => {
                    let job = database.start_automatic_audio(run)?;
                    audio.open_notebook(app, &job, "open_browser").await?;
                    audio.notify();
                    return Ok(());
                }
            };
            if job.status == "succeeded" {
                if job.asset_id.is_none() {
                    return Err("音频已完成但尚未归档素材。".into());
                }
                database.set_automation_stage(run, "video_prepare")?;
            } else if matches!(
                job.status.as_str(),
                "failed" | "paused" | "waiting_user" | "cancelled"
            ) {
                return Err(job
                    .error_message
                    .unwrap_or_else(|| "音频需要核对，请打开原音频任务处理后继续。".into()));
            }
        }
        "video_prepare" => {
            let job = match &run.video_job_id {
                Some(id) => database.video_job(id)?,
                None => {
                    let audio =
                        database.audio_job(run.audio_job_id.as_deref().ok_or("缺少音频任务。")?)?;
                    database.create_bound_video_job(
                        StartVideoInput {
                            project_id: run.project_id.clone(),
                            audio_asset_id: audio.asset_id.ok_or("音频尚未归档。")?,
                            subtitle_asset_id: None,
                            provider_id: run.input.selection.text_provider_id.clone(),
                        },
                        Some(&run.binding(Slot::Video)),
                    )?
                }
            };
            match job.status.as_str() {
                "draft" => {
                    video_commands::continue_bound_video_job(
                        app.clone(),
                        job.id,
                        false,
                        Some(run.input.selection.text_provider_id.clone()),
                        Some(run),
                    )?;
                }
                "awaiting_confirmation" | "succeeded" => {
                    crate::video::validate_cues(&job.plan.cues, job.plan.duration)?;
                    crate::video::validate_scenes(&job.plan.scenes, 0, job.plan.cues.len())?;
                    if job.plan.duration > 900.0 {
                        return Err("实际音频超过自动流程15分钟上限，音频和字幕已保留，请调整节目后重新制作。".into());
                    }
                    database.set_automation_stage(run, "writing")?;
                }
                "failed" | "cancelled" => {
                    return Err(format!(
                        "视频准备未完成：{}。可进入视频页处理，或明确重试当前生成。",
                        job.error
                    ))
                }
                _ => {}
            }
        }
        "writing" => {
            let id = match &run.writing_task_id {
                Some(id) => id.clone(),
                None => database.create_transcript_task(run)?,
            };
            let (status, _, _, error) = database.automation_ai_result(&id)?;
            if status == "succeeded" {
                database.set_automation_stage(run, "landscape_image")?;
            } else if status == "pending" {
                generate_transcript(app, run, &id)
                    .await
                    .map_err(|e| e.message)?;
            } else if status != "running" {
                return Err(format!(
                    "文案生成未完成：{error}。如需再次调用模型，请明确重试当前生成。"
                ));
            }
        }
        "landscape_image" | "portrait_image" => {
            let landscape = run.stage == "landscape_image";
            let existing = if landscape {
                &run.landscape_task_id
            } else {
                &run.portrait_task_id
            };
            let id = match existing {
                Some(id) => id.clone(),
                None => {
                    let WritingContent::Publishing {
                        cover_prompt_landscape,
                        cover_prompt_portrait,
                        ..
                    } = database.automation_writing(run)?.content
                    else {
                        return Err("文案类型无效。".into());
                    };
                    database.start_bound_cover_generation(
                        ImageGenerationInput {
                            project_id: run.project_id.clone(),
                            provider_id: run.input.selection.image_provider_id.clone(),
                            description: if landscape {
                                cover_prompt_landscape
                            } else {
                                cover_prompt_portrait
                            },
                            width: if landscape { 1440 } else { 1080 },
                            height: if landscape { 1080 } else { 1440 },
                        },
                        &run.binding(if landscape {
                            Slot::LandscapeImage
                        } else {
                            Slot::PortraitImage
                        }),
                    )?
                }
            };
            let (status, _, _, error) = database.automation_ai_result(&id)?;
            if status == "succeeded" {
                database.set_automation_stage(
                    run,
                    if landscape {
                        "landscape_cover"
                    } else {
                        "portrait_cover"
                    },
                )?;
            } else if status == "pending" {
                crate::cover_commands::execute_image_task(
                    &database,
                    &app.state::<SystemCredentials>(),
                    &app.state::<ActiveTasks>(),
                    &id,
                    "cover",
                )
                .await
                .map_err(|e| e.message)?;
            } else if status != "running" {
                return Err(format!(
                    "背景生成未完成：{error}。已完成背景保留，再次调用需明确重试。"
                ));
            }
        }
        "landscape_cover" => {
            if run.landscape_cover_id.is_some() {
                database.set_automation_stage(run, "portrait_image")?;
            }
        }
        "portrait_cover" => {
            if run.portrait_cover_id.is_some() {
                database.set_automation_stage(run, "render")?;
            }
        }
        "render" => {
            database.verify_automation_video(run)?;
            let job = database.video_job(run.video_job_id.as_deref().ok_or("缺少视频任务。")?)?;
            match job.status.as_str() {
                "awaiting_confirmation" => {
                    video_commands::continue_bound_video_job(
                        app.clone(),
                        job.id,
                        true,
                        None,
                        Some(run),
                    )?;
                }
                "succeeded" => {
                    database.set_automation_stage(run, "publishing")?;
                }
                "failed" | "cancelled" => {
                    return Err(format!(
                        "视频渲染未完成：{}。请在视频页处理或明确重试。",
                        job.error
                    ))
                }
                _ => {}
            }
        }
        "publishing" => {
            database.verify_automation_video(run)?;
            if run.publish_started {
                let session = douyin::flow::automation_session(&app.state::<DouyinPublisher>())?;
                let mut session = session.filter(|session|Some(&session.task_id)==run.publish_task_id.as_ref()).ok_or("上次上传已经开始，但网页会话已丢失。请在发布详情核对原草稿或已发布作品，不会自动再次上传。")?;
                if session.status == "failed" {
                    let window = app.get_webview_window("main").ok_or("工作台窗口已关闭。")?;
                    session = douyin::flow::start_douyin_task(
                        app.clone(),
                        window,
                        database.clone(),
                        app.state::<DouyinPublisher>(),
                        session.task_id.clone(),
                    )
                    .await?;
                }
                if session.status == "review" {
                    database.set_automation_status(
                        run,
                        "awaiting_publish",
                        "发布材料已准备好，请在抖音窗口完成最终发布。实际发布后可记录结果。",
                    )?;
                    return Ok(());
                }
                if session.status != "running" {
                    return Err(format!("请在发布详情继续处理原会话：{}", session.message));
                }
                return Ok(());
            }
            let id = match &run.publish_task_id {
                Some(id) => id.clone(),
                None => {
                    let job =
                        database.video_job(run.video_job_id.as_deref().ok_or("缺少视频任务。")?)?;
                    let version = database.automation_writing(run)?;
                    let WritingContent::Publishing {
                        title,
                        description,
                        tags,
                        ..
                    } = version.content
                    else {
                        return Err("文案类型无效。".into());
                    };
                    let task = database.save_bound_publish_task(
                        None,
                        PublishInput {
                            project_id: run.project_id.clone(),
                            writing_version_id: Some(version.id),
                            title,
                            description,
                            tags,
                            audio_asset_id: job.audio_asset_id,
                            video_asset_id: job.video_asset_id,
                            landscape_cover_id: run.landscape_cover_id.clone(),
                            portrait_cover_id: run.portrait_cover_id.clone(),
                            title_confirmed: false,
                            covers_confirmed: false,
                            allow_download: false,
                            scheduled_at: Some(database.next_automation_schedule()?),
                        },
                        Some(&run.binding(Slot::Publish)),
                    )?;
                    task.id
                }
            };
            let window = app.get_webview_window("main").ok_or("工作台窗口已关闭。")?;
            database.mark_automation_upload(run)?;
            let session = douyin::flow::start_douyin_task(
                app.clone(),
                window,
                database.clone(),
                app.state::<DouyinPublisher>(),
                id,
            )
            .await?;
            if session.status == "review" {
                database.set_automation_status(run,"awaiting_publish","视频、文案、双封面、禁止保存和定时设置已填写。请在抖音窗口检查并点击发布；平台若要求声明，请按实际内容确认。")?;
            } else {
                return Err(session.message);
            }
        }
        _ => return Err("未知自动流程阶段。".into()),
    }
    Ok(())
}

async fn generate_transcript(
    app: &AppHandle,
    run: &AutomationRun,
    id: &str,
) -> Result<(), AiError> {
    let database = app.state::<Database>();
    let database = database.inner();
    let active = app.state::<ActiveTasks>();
    ai_commands::run_task(database,&active,id,|mut receiver| async move {
        let user = database.prepare_transcript_task(id).map_err(AiError::database)?;
        let request = ai_commands::request(database.get_provider(&run.input.selection.text_provider_id).map_err(AiError::database)?,&app.state::<SystemCredentials>(),include_str!("../../prompts/transcript-publishing-v1.md"),user)?;
        let provider = OpenAiCompatible::new(TIMEOUT,&request.base_url)?.observed(database,Some(id),"transcript_publishing");
        let raw = tokio::select! { biased; _=receiver.changed()=>return Err(AiError::new("cancelled","自动文案已取消。")), response=provider.generate(request)=>response? };
        let audio_prompt = database.audio_job_prompt(run.audio_job_id.as_deref().ok_or_else(||AiError::database("缺少音频任务。".into()))?).map_err(AiError::database)?;
        let draft: TranscriptDraft = serde_json::from_str(&raw).map_err(|_|AiError::new("invalid_content","字幕文案输出结构无效，已有音频和字幕保留。"))?;
        let content = draft.into_content(audio_prompt).map_err(|error|AiError::new("invalid_content",&error))?;
        database.finish_transcript_task(id,content).map_err(AiError::database)
    }).await
}
