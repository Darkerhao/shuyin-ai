use super::*;
use crate::database::publishing::CollectionChoice;
use serde_json::json;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub id: Option<String>,
    pub name: String,
    pub key: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectionOption {
    pub id: Option<String>,
    pub name: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Collections {
    available: bool,
    loaded: bool,
    options: Vec<CollectionOption>,
    selected: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub run_id: u64,
    pub task_id: String,
    pub status: String,
    pub step: String,
    pub message: String,
    pub waiting_for: Option<String>,
    pub account: Option<Account>,
    pub completed: Vec<String>,
    pub manual: Vec<String>,
    pub collection: Option<CollectionChoice>,
    pub options: Vec<CollectionOption>,
    #[serde(skip)]
    sent_video: bool,
    #[serde(skip)]
    video_key: String,
    #[serde(skip)]
    covers: Vec<CoverSnapshot>,
    #[serde(skip)]
    verified_text: Option<(String, String)>,
    #[serde(skip)]
    account_confirmed: bool,
    #[serde(skip)]
    cover_previews: Vec<(String, serde_json::Value)>,
    #[serde(skip)]
    cover_pending: bool,
    #[serde(skip)]
    collection_authorized: bool,
}

impl Session {
    fn new(run_id: u64, task_id: String) -> Self {
        Self {
            run_id,
            task_id,
            status: "running".into(),
            step: "connect".into(),
            message: "正在连接抖音…".into(),
            waiting_for: None,
            account: None,
            completed: vec![],
            manual: vec![],
            collection: None,
            options: vec![],
            sent_video: false,
            video_key: String::new(),
            covers: vec![],
            verified_text: None,
            account_confirmed: false,
            cover_previews: vec![],
            cover_pending: false,
            collection_authorized: false,
        }
    }
    fn done(&mut self, field: &str, manual: bool) {
        if !self.completed.iter().any(|v| v == field) {
            self.completed.push(field.into());
        }
        if manual && !self.manual.iter().any(|v| v == field) {
            self.manual.push(field.into());
        }
    }
    fn wait(&mut self, reason: &str, message: &str) {
        self.completed.retain(|field| field != reason);
        self.manual.retain(|field| field != reason);
        self.status = "waiting".into();
        self.waiting_for = Some(reason.into());
        self.message = message.into();
    }
    fn reset_page_proofs(&mut self) {
        self.sent_video = false;
        self.video_key.clear();
        self.covers.clear();
        self.verified_text = None;
        self.account_confirmed = false;
        self.completed.clear();
        self.manual.clear();
        self.cover_previews.clear();
        self.cover_pending = false;
        self.collection = None;
        self.options.clear();
        self.collection_authorized = false;
    }
    fn validate_decision(&self, run_id: u64, stopped: bool) -> Result<(), String> {
        if self.run_id != run_id || self.status != "waiting" || stopped {
            return Err("此操作已过期，请重新读取当前进度。".into());
        }
        Ok(())
    }
}

pub(super) fn window_closed(app: &AppHandle, publisher: &DouyinPublisher) {
    publisher.stopped.store(true, Ordering::SeqCst);
    if let Ok(mut confirmed) = publisher.confirmed.lock() {
        *confirmed = None;
    }
    if let Ok(mut task) = publisher.task.lock() {
        *task = None;
    }
    if let Ok(mut active) = publisher.session.lock() {
        if let Some(session) = active.as_mut() {
            session.reset_page_proofs();
            session.status = "stopped".into();
            session.waiting_for = None;
            session.message = "发布窗口已关闭，重新开始后需要核对账号和已有草稿。".into();
            let _ = app.emit_to("main", "douyin-progress", &*session);
        }
    }
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum Decision {
    Account {
        name: String,
    },
    ExistingVideo,
    Processing,
    Page,
    Text,
    Covers,
    Collection {
        name: Option<String>,
        remember: bool,
        manual: bool,
    },
}

fn current(publisher: &DouyinPublisher) -> Result<Option<Session>, String> {
    Ok(publisher.session.lock().map_err(|e| e.to_string())?.clone())
}

fn publish(app: &AppHandle, publisher: &DouyinPublisher, session: &Session) -> Result<(), String> {
    let mut active = publisher.session.lock().map_err(|e| e.to_string())?;
    if active.as_ref().is_some_and(|s| s.run_id != session.run_id) {
        return Err("此运行已经结束，请重新读取进度。".into());
    }
    *active = Some(session.clone());
    let _ = app.emit_to("main", "douyin-progress", session);
    Ok(())
}

#[tauri::command]
pub fn douyin_session(
    window: WebviewWindow,
    publisher: tauri::State<'_, DouyinPublisher>,
) -> Result<Option<Session>, String> {
    local_main(&window)?;
    current(&publisher)
}

#[tauri::command]
pub async fn start_douyin_task(
    app: AppHandle,
    window: WebviewWindow,
    database: tauri::State<'_, Database>,
    publisher: tauri::State<'_, DouyinPublisher>,
    id: String,
) -> Result<Session, String> {
    local_main(&window)?;
    if !cfg!(windows) {
        return Err("应用内抖音辅助发布目前仅支持 Windows，请使用任务包。".into());
    }
    let _operation = match publisher.operation.try_lock() {
        Ok(guard) => guard,
        Err(_) => {
            return current(&publisher)?
                .filter(|s| s.task_id == id)
                .ok_or("另一个任务正在执行。".into())
        }
    };
    if let Some(session) = current(&publisher)? {
        if session.task_id != id && publisher.task.lock().map_err(|e| e.to_string())?.is_some() {
            return Err(
                "另一个任务仍绑定官方页面，请先记录实际结果或关闭发布窗口后处理下一篇。".into(),
            );
        }
        if session.task_id == id
            && ["running", "waiting", "review"].contains(&session.status.as_str())
        {
            open_douyin_window(app, window, publisher.clone()).await?;
            return Ok(session);
        }
    }
    let snapshot = database.publish_snapshot(&id).or_else(|_| {
        database.prepare_publish_task(&id)?;
        database.publish_snapshot(&id)
    })?;
    let mut previous = current(&publisher)?;
    if publisher
        .task
        .lock()
        .map_err(|e| e.to_string())?
        .as_ref()
        .is_some_and(|expected| expected != &snapshot)
    {
        previous = None;
    }
    let run_id = publisher.sequence.fetch_add(1, Ordering::SeqCst) + 1;
    let mut session = previous
        .filter(|s| s.task_id == id)
        .unwrap_or_else(|| Session::new(run_id, id));
    session.run_id = run_id;
    session.status = "running".into();
    session.waiting_for = None;
    publisher.stopped.store(false, Ordering::SeqCst);
    *publisher.task.lock().map_err(|e| e.to_string())? = Some(snapshot.clone());
    *publisher.session.lock().map_err(|e| e.to_string())? = Some(session.clone());
    if let Err(error) = open_douyin_window(app.clone(), window.clone(), publisher.clone()).await {
        session.status = "failed".into();
        session.message = error;
        publish(&app, &publisher, &session)?;
        return Ok(session);
    }
    execute(
        &app,
        &window,
        &database,
        &publisher,
        &snapshot,
        &mut session,
    )
    .await;
    Ok(session)
}

#[tauri::command]
pub async fn continue_douyin_task(
    app: AppHandle,
    window: WebviewWindow,
    database: tauri::State<'_, Database>,
    publisher: tauri::State<'_, DouyinPublisher>,
    run_id: u64,
    decision: Decision,
) -> Result<Session, String> {
    local_main(&window)?;
    let _operation = publisher
        .operation
        .try_lock()
        .map_err(|_| "正在执行，请等待当前操作。")?;
    let mut session = current(&publisher)?.ok_or("没有等待中的辅助任务。")?;
    session.validate_decision(run_id, publisher.stopped.load(Ordering::SeqCst))?;
    let snapshot = database.publish_snapshot(&session.task_id)?;
    running(&publisher, &snapshot)?;
    let publish_window = publish_window(&app)?;
    let page = inspect_douyin_window(app.clone(), window.clone()).await?;
    if !page.online || page.login_visible || !page.authenticated {
        return Err("请先恢复网络并在官方页登录。".into());
    }
    let account = page
        .account
        .as_ref()
        .ok_or("无法识别当前账号，请在官方窗口核对。")?;
    let reason = session.waiting_for.as_deref().unwrap_or_default();
    if reason != "account" && !same_account(session.account.as_ref(), Some(account)) {
        session.reset_page_proofs();
        session.account = Some(account.clone());
        session.wait("account", "账号已变化，请重新核对；不会沿用原账号的合集。");
        publish(&app, &publisher, &session)?;
        return Ok(session);
    }
    match decision {
        Decision::Account { name } if reason == "account" => {
            if name.trim().is_empty() || name.chars().count() > 200 {
                return Err("请填写官方窗口显示的账号昵称。".into());
            }
            if let Some(expected) = session.account.as_ref() {
                if !same_account(Some(expected), Some(account)) {
                    return Err("账号在确认期间已变化，请重新开始核对。".into());
                }
            }
            let mut confirmed = account.clone();
            confirmed.name = name.trim().into();
            session.account = Some(confirmed.clone());
            session.account_confirmed = true;
            *publisher.confirmed.lock().map_err(|e| e.to_string())? = Some(confirmed);
        }
        Decision::ExistingVideo if reason == "video" => {
            let editor = read_video_editor(&publish_window, &publisher, &snapshot).await?;
            session.sent_video = true;
            session.video_key = editor.video_key;
        }
        Decision::Processing if reason == "processing" => {
            let editor = read_video_editor(&publish_window, &publisher, &snapshot).await?;
            if page.uploading || page.processing {
                return Err("平台仍在上传或处理中，请稍后核对。".into());
            }
            verify_video(&session, &editor)?;
            session.done("video", true);
        }
        Decision::Text if reason == "text" => {
            let editor = read_video_editor(&publish_window, &publisher, &snapshot).await?;
            verify_video(&session, &editor)?;
            session.verified_text = Some((editor.title, editor.description));
            session.done("text", true);
        }
        Decision::Covers if reason == "covers" => {
            let editor = read_video_editor(&publish_window, &publisher, &snapshot).await?;
            verify_video(&session, &editor)?;
            if editor.cover_dialog
                || editor.covers.len() != 2
                || editor.covers.iter().any(|c| !c.selected)
            {
                return Err("请先在官方页确认两张封面并关闭弹窗。".into());
            }
            session.covers = editor.covers;
            session.done("covers", true);
            session.cover_previews.clear();
            session.cover_pending = false;
        }
        Decision::Collection {
            name,
            remember,
            manual,
        } if reason == "collection" => {
            let editor = read_video_editor(&publish_window, &publisher, &snapshot).await?;
            verify_video(&session, &editor)?;
            if !manual {
                let options = read_collections(&publish_window).await?;
                if name.is_some()
                    && (!options.loaded
                        || options
                            .options
                            .iter()
                            .filter(|o| Some(&o.name) == name.as_ref())
                            .count()
                            != 1)
                {
                    return Err("合集重名、未加载或已失效，请重新读取。".into());
                }
            }
            let account = session.account.as_ref().ok_or("请先确认账号。")?;
            let choice = CollectionChoice {
                account_id: account.id.clone(),
                account_name: account.name.clone(),
                id: None,
                name,
            };
            running(&publisher, &snapshot)?;
            database.save_douyin_collection(&session.task_id, &choice, remember)?;
            session.collection = Some(choice);
            session.collection_authorized = true;
            session.completed.retain(|s| s != "collection");
            session.manual.retain(|s| s != "collection");
            if manual {
                session.done("collection", true);
            }
        }
        Decision::Page if reason == "page" || reason == "collection" => {}
        _ => return Err("此决定不适用于当前步骤。".into()),
    }
    session.status = "running".into();
    session.waiting_for = None;
    execute(
        &app,
        &window,
        &database,
        &publisher,
        &snapshot,
        &mut session,
    )
    .await;
    Ok(session)
}

fn same_account(a: Option<&Account>, b: Option<&Account>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => match (&a.id, &b.id) {
            (Some(a), Some(b)) => a == b,
            _ => !a.key.is_empty() && a.key != "0" && a.key == b.key,
        },
        _ => false,
    }
}

fn verify_video(session: &Session, editor: &EditorSnapshot) -> Result<(), String> {
    if session.video_key.is_empty() || editor.video_key != session.video_key {
        return Err("网页视频身份无法与当前会话核对，请停止后重新确认视频。".into());
    }
    Ok(())
}

async fn read_video_editor(
    window: &WebviewWindow,
    publisher: &DouyinPublisher,
    snapshot: &PublishSnapshot,
) -> Result<EditorSnapshot, String> {
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        running(publisher, snapshot)?;
        let editor: EditorSnapshot =
            serde_json::from_value(page_action(window, "read", json!({"video":true})).await?)
                .map_err(|_| "无法回读视频预览，请人工检查。")?;
        if editor.video_preview && !editor.video_key.is_empty() {
            return Ok(editor);
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("视频预览尚未就绪，请在官方页打开“预览视频”后重试。".into());
        }
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }
}

async fn read_collections(window: &WebviewWindow) -> Result<Collections, String> {
    serde_json::from_value(page_action(window, "collections", json!({})).await?)
        .map_err(|_| "无法回读合集。".into())
}

async fn execute(
    app: &AppHandle,
    main: &WebviewWindow,
    database: &Database,
    publisher: &DouyinPublisher,
    snapshot: &PublishSnapshot,
    session: &mut Session,
) {
    let result = drive(app, main, database, publisher, snapshot, session).await;
    if let Err(error) = result {
        session.status = if publisher.stopped.load(Ordering::SeqCst) {
            "stopped"
        } else {
            "failed"
        }
        .into();
        session.waiting_for = None;
        session.message = error;
    }
    if publisher.task.lock().is_ok_and(|task| task.is_none()) {
        session.reset_page_proofs();
    }
    let _ = publish(app, publisher, session);
}

fn upload_navigation_finished(page: &DouyinPage) -> bool {
    page.login_visible
        || page.blocked
        || !page.online
        || page.kind == "draft"
        || (page.kind == "upload" && page.upload_ready)
        || (page.kind == "editor" && page.editor_ready)
}

async fn drive(
    app: &AppHandle,
    main: &WebviewWindow,
    database: &Database,
    publisher: &DouyinPublisher,
    snapshot: &PublishSnapshot,
    session: &mut Session,
) -> Result<(), String> {
    running(publisher, snapshot)?;
    session.step = "connect".into();
    session.message = "正在连接抖音，未登录时请在官方窗口扫码…".into();
    publish(app, publisher, session)?;
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(600);
    let page = loop {
        running(publisher, snapshot)?;
        let page = inspect_douyin_window(app.clone(), main.clone()).await?;
        if !page.open {
            return Err("发布窗口已关闭。".into());
        }
        if !page.online {
            session.wait("page", "网络不可用，请恢复网络后继续。");
            return Ok(());
        }
        if page.authenticated && !page.login_visible {
            break page;
        }
        session.account_confirmed = false;
        *publisher.confirmed.lock().map_err(|e| e.to_string())? = None;
        if tokio::time::Instant::now() >= deadline {
            return Err("等待登录超时，请重新开始。".into());
        }
        tokio::time::sleep(std::time::Duration::from_millis(750)).await;
    };
    let account = page
        .account
        .as_ref()
        .ok_or("无法识别账号，请在官方窗口检查。")?;
    if !same_account(session.account.as_ref(), Some(account)) {
        session.reset_page_proofs();
    }
    if !session.account_confirmed {
        let confirmed = publisher
            .confirmed
            .lock()
            .map_err(|e| e.to_string())?
            .clone();
        if same_account(confirmed.as_ref(), Some(account)) {
            session.account = confirmed;
            session.account_confirmed = true;
        } else {
            session.account = Some(account.clone());
            session.wait("account", "请在官方窗口核对账号，并确认使用这个账号继续。");
            return Ok(());
        }
    }
    session.done("connect", false);
    let window = publish_window(app)?;
    if !page.blocked && (page.kind == "home" || (page.kind == "upload" && !page.upload_ready)) {
        running(publisher, snapshot)?;
        session.message = "正在打开发布视频入口，等待上传页面就绪…".into();
        publish(app, publisher, session)?;
        if page.kind == "home" {
            window
                .navigate(
                    "https://creator.douyin.com/creator-micro/content/upload"
                        .parse()
                        .unwrap(),
                )
                .map_err(|e| e.to_string())?;
        }
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(30);
        loop {
            running(publisher, snapshot)?;
            let next = inspect_douyin_window(app.clone(), main.clone()).await?;
            if upload_navigation_finished(&next) {
                break;
            }
            if tokio::time::Instant::now() >= deadline {
                return Err("发布视频入口或上传控件加载超时，请在官方窗口检查后重新开始。".into());
            }
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
        return Box::pin(drive(app, main, database, publisher, snapshot, session)).await;
    }
    if session.cover_pending {
        session.wait(
            "covers",
            "上次封面发送结果尚未确认。请在官方页检查并完成双封面后采用网页结果，不会重复上传。",
        );
        return Ok(());
    }
    let own_cover_dialog =
        !session.cover_previews.is_empty() && page.editor.as_ref().is_some_and(|e| e.cover_dialog);
    if page.kind == "draft" || page.kind == "unknown" || (page.blocked && !own_cover_dialog) {
        session.wait(
            "page",
            "抖音有未完成草稿、弹窗或未知页面，请查看处理后继续；已有内容会保留。",
        );
        return Ok(());
    }
    session.step = "video".into();
    if page.kind == "upload" {
        if session.sent_video {
            session.wait(
                "page",
                "当前会话已发送过视频，请在官方页核对结果，不会重复上传。",
            );
            return Ok(());
        }
        session.message = "正在发送当前任务成片…".into();
        publish(app, publisher, session)?;
        session.sent_video = true; // Reserve before dispatch; timeout cannot prove that dispatch failed.
        publish(app, publisher, session)?;
        verify_snapshot(&database.publish_snapshot(&snapshot.task_id)?, snapshot)?;
        ensure_account(app, main, session).await?;
        set_local_file(
            &window,
            "videoInput",
            None,
            &snapshot.video,
            publisher,
            snapshot,
        )
        .await?;
    } else if !session.sent_video {
        session.wait(
            "video",
            "网页已有视频，无法证明属于本任务。请核对账号和成片后确认；不会重传或清空草稿。",
        );
        return Ok(());
    }
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(1800);
    let editor = loop {
        running(publisher, snapshot)?;
        let page = inspect_douyin_window(app.clone(), main.clone()).await?;
        if !page.online
            || page.login_visible
            || !same_account(session.account.as_ref(), page.account.as_ref())
        {
            session.account_confirmed = false;
            *publisher.confirmed.lock().map_err(|e| e.to_string())? = None;
            session.wait("page", "网络或账号已变化，请检查后继续核对。");
            return Ok(());
        }
        session.message = if page.uploading {
            "视频上传中…"
        } else if page.processing {
            "平台正在处理视频…"
        } else {
            "等待平台确认上传和处理…"
        }
        .into();
        publish(app, publisher, session)?;
        if let Some(editor) = page.editor {
            if editor.cover_dialog && !own_cover_dialog {
                session.wait("page", "请先关闭官方页弹窗后继续。");
                return Ok(());
            }
            if editor.video_preview && !editor.video_key.is_empty() {
                if session.video_key.is_empty() {
                    session.video_key = editor.video_key.clone();
                } else {
                    verify_video(session, &editor)?;
                }
                if !page.uploading
                    && !page.processing
                    && (session.completed.iter().any(|v| v == "video") || page.processed)
                {
                    session.done("video", false);
                    break editor;
                }
                if !page.uploading && !page.processing {
                    session.wait("processing", "预览已可播放，但平台完成信号尚无法自动核实。请核对上传及处理均完成后继续。");
                    return Ok(());
                }
            } else if !page.uploading && !page.processing {
                running(publisher, snapshot)?;
                page_action(&window, "previewVideo", json!({})).await?;
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("等待视频处理超时，已保留页面，不会重新上传。".into());
        }
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    };
    session.step = "text".into();
    session.message = "正在填写并回读发布内容…".into();
    publish(app, publisher, session)?;
    if let Some((title, description)) = &session.verified_text {
        if &editor.title != title || &editor.description != description {
            session.wait(
                "text",
                "网页文案已被修改。可保留并采用网页结果，或停止后人工处理。",
            );
            return Ok(());
        }
    } else {
        let description = [&snapshot.input.description, &snapshot.input.tags]
            .into_iter()
            .filter(|v| !v.is_empty())
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join("\n\n");
        let normalized = |s: &str| s.replace(['\r', '\u{200b}'], "").trim().to_owned();
        if (!editor.title.is_empty() && editor.title != snapshot.input.title)
            || (!editor.description.is_empty()
                && normalized(&editor.description) != normalized(&description))
        {
            session.wait(
                "text",
                "网页已有不同文案。可明确采用网页结果，或停止后人工处理。",
            );
            return Ok(());
        }
        ensure_account(app, main, session).await?;
        running(publisher, snapshot)?;
        page_action(
            &window,
            "fill",
            serde_json::to_value(&snapshot.input).map_err(|e| e.to_string())?,
        )
        .await?;
        let editor = read_editor(&window).await?;
        verify_editor(&editor, snapshot)?;
        session.verified_text = Some((editor.title, editor.description));
        session.done("text", false);
    }
    session.step = "covers".into();
    session.message = "正在设置并核对双封面…".into();
    publish(app, publisher, session)?;
    let editor = read_editor(&window).await?;
    if !session.covers.is_empty() {
        if editor.covers != session.covers {
            session.wait(
                "covers",
                "封面已变化或身份无法核对，请在官方页确认双封面后采用网页结果。",
            );
            return Ok(());
        }
    } else if editor.covers.iter().any(|c| c.selected) && session.cover_previews.is_empty() {
        session.wait(
            "covers",
            "网页已有封面，来源尚未核对。请检查两张封面后采用网页结果；不会覆盖。",
        );
        return Ok(());
    } else {
        for ratio in ["landscape", "portrait"] {
            running(publisher, snapshot)?;
            verify_snapshot(&database.publish_snapshot(&snapshot.task_id)?, snapshot)?;
            ensure_account(app, main, session).await?;
            running(publisher, snapshot)?;
            let has_dialog = read_editor(&window).await?.cover_dialog;
            page_action(
                &window,
                if has_dialog {
                    "switchCover"
                } else {
                    "openCover"
                },
                json!({"ratio":ratio}),
            )
            .await?;
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            running(publisher, snapshot)?;
            let before = page_action(&window, "coverReady", json!({"ratio":ratio})).await?;
            if let Some((_, expected)) = session
                .cover_previews
                .iter()
                .find(|(saved, _)| saved == ratio)
            {
                if before["ready"] != true || &before["previewKey"] != expected {
                    session.wait("covers", "已填写封面的预览身份发生变化，请人工核对双封面。");
                    return Ok(());
                }
                continue;
            }
            session.cover_pending = true;
            publish(app, publisher, session)?;
            set_local_file(
                &window,
                "coverInput",
                Some(ratio),
                if ratio == "landscape" {
                    &snapshot.landscape
                } else {
                    &snapshot.portrait
                },
                publisher,
                snapshot,
            )
            .await?;
            wait_cover(&window, publisher, snapshot, ratio, &before).await?;
            let result = page_action(&window, "coverReady", json!({"ratio":ratio})).await?;
            session
                .cover_previews
                .push((ratio.into(), result["previewKey"].clone()));
            session.cover_pending = false;
            publish(app, publisher, session)?;
        }
        running(publisher, snapshot)?;
        ensure_account(app, main, session).await?;
        running(publisher, snapshot)?;
        page_action(&window, "confirmCover", json!({})).await?;
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(120);
        loop {
            running(publisher, snapshot)?;
            let editor = read_editor(&window).await?;
            if !editor.cover_dialog
                && editor.covers.len() == 2
                && editor.covers.iter().all(|c| c.selected)
            {
                session.covers = editor.covers;
                if session.covers.iter().any(|c| c.preview.is_empty()) {
                    session.wait(
                        "covers",
                        "封面已保存，但预览身份无法自动核对，请在官方页检查两张封面后确认。",
                    );
                    return Ok(());
                }
                session.done("covers", false);
                break;
            }
            if tokio::time::Instant::now() >= deadline {
                return Err("封面保存尚未确认，请检查官方页，重试时会保留已有封面。".into());
            }
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
    }
    session.step = "collection".into();
    session.message = "正在核对目标合集…".into();
    publish(app, publisher, session)?;
    ensure_account(app, main, session).await?;
    if session.collection.is_none() {
        let saved = database.douyin_collection(&session.task_id)?;
        let account = session.account.as_ref().unwrap();
        let had_choice = saved.is_some();
        session.collection = saved.filter(|c| c.account_id.is_some() && c.account_id == account.id);
        if session.collection.is_none() && !had_choice {
            if let Some(id) = account.id.as_ref() {
                session.collection = database.default_douyin_collection(id)?;
                if let Some(choice) = &session.collection {
                    database.save_douyin_collection(&session.task_id, choice, false)?;
                }
            }
        }
    }
    let mut collections = read_collections(&window).await?;
    if !session.manual.iter().any(|s| s == "collection") {
        if session
            .collection
            .as_ref()
            .is_some_and(|c| c.name.is_some())
            && collections.available
            && !session.collection_authorized
        {
            running(publisher, snapshot)?;
            page_action(&window, "openCollections", json!({})).await?;
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            collections = read_collections(&window).await?;
            session.options = collections.options.clone();
            let target = session.collection.as_ref().and_then(|c| c.name.as_ref());
            if !collections.loaded
                || collections
                    .options
                    .iter()
                    .filter(|o| Some(&o.name) == target)
                    .count()
                    != 1
            {
                session.completed.retain(|s| s != "collection");
                session.wait(
                    "collection",
                    "目标合集未加载、重名或已失效，请重新选择，不会重传视频。",
                );
                return Ok(());
            }
        }
        let changed = session
            .collection
            .as_ref()
            .is_some_and(|choice| collections.selected != choice.name);
        if changed && collections.selected.is_some() && !session.collection_authorized {
            session.completed.retain(|s| s != "collection");
            session.wait(
                "collection",
                "网页合集与目标不同，请重新选择或人工确认，已有选择会保留。",
            );
            return Ok(());
        }
        session.completed.retain(|s| s != "collection");
        if let Some(choice) = &session.collection {
            if collections.available && collections.selected == choice.name {
                session.done("collection", false);
            } else if collections.available {
                running(publisher, snapshot)?;
                page_action(&window, "openCollections", json!({})).await?;
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                let loaded = read_collections(&window).await?;
                session.options = loaded.options.clone();
                if loaded.loaded
                    && loaded
                        .options
                        .iter()
                        .filter(|o| Some(&o.name) == choice.name.as_ref())
                        .count()
                        == 1
                {
                    running(publisher, snapshot)?;
                    ensure_account(app, main, session).await?;
                    running(publisher, snapshot)?;
                    page_action(&window, "selectCollection", json!({"name":choice.name})).await?;
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    if read_collections(&window).await?.selected == choice.name {
                        session.done("collection", false);
                    }
                }
            }
        } else if collections.available {
            running(publisher, snapshot)?;
            page_action(&window, "openCollections", json!({})).await?;
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            session.options = read_collections(&window).await?.options;
        }
        if !session.completed.iter().any(|s| s == "collection") {
            session.wait(
                "collection",
                "请选择目标合集或明确不加入；入口无法读取时可在官方页设置并人工确认。",
            );
            return Ok(());
        }
    }
    running(publisher, snapshot)?;
    ensure_account(app, main, session).await?;
    let final_editor = read_video_editor(&window, publisher, snapshot).await?;
    verify_video(session, &final_editor)?;
    if session.verified_text.as_ref()
        != Some(&(final_editor.title.clone(), final_editor.description.clone()))
    {
        session.wait("text", "最终核对发现文案变化，请确认网页结果。");
        return Ok(());
    }
    if final_editor.cover_dialog || final_editor.covers != session.covers {
        session.wait("covers", "最终核对发现封面变化，请确认网页结果。");
        return Ok(());
    }
    if !session.manual.iter().any(|s| s == "collection")
        && read_collections(&window).await?.selected
            != session.collection.as_ref().and_then(|c| c.name.clone())
    {
        session.wait("collection", "最终核对发现合集变化，请重新确认。");
        return Ok(());
    }
    session.collection_authorized = false;
    session.step = "review".into();
    session.status = "review".into();
    session.waiting_for = None;
    session.message = "已准备好，请检查并发布。请在官方页核对预览、声明、可见范围和发布设置，亲自点击发布，再回来记录实际结果。".into();
    Ok(())
}

async fn ensure_account(
    app: &AppHandle,
    main: &WebviewWindow,
    session: &Session,
) -> Result<(), String> {
    let page = inspect_douyin_window(app.clone(), main.clone()).await?;
    if page.login_visible
        || !page.online
        || !page.authenticated
        || !same_account(session.account.as_ref(), page.account.as_ref())
    {
        return Err("账号或网络已变化，已停止写入，请重新核对。".into());
    }
    Ok(())
}

#[tauri::command]
pub async fn stop_douyin_task(
    app: AppHandle,
    window: WebviewWindow,
    publisher: tauri::State<'_, DouyinPublisher>,
    run_id: u64,
) -> Result<Session, String> {
    local_main(&window)?;
    let session = stop_current(&publisher, run_id).await?;
    publish(&app, &publisher, &session)?;
    Ok(session)
}

async fn stop_current(publisher: &DouyinPublisher, run_id: u64) -> Result<Session, String> {
    let session = current(publisher)?
        .filter(|s| s.run_id == run_id)
        .ok_or("停止请求已过期。")?;
    publisher.stopped.store(true, Ordering::SeqCst);
    // Acknowledge only after in-flight host operations have settled.
    let _operation = publisher.operation.lock().await;
    let mut session = current(publisher)?.unwrap_or(session);
    if session.run_id != run_id {
        return Err("运行已变化，请读取当前进度。".into());
    }
    session.status = "stopped".into();
    session.waiting_for = None;
    session.message = "已停止并交由人工接管。网页已开始的上传可能仍在继续，请在官方页检查。".into();
    Ok(session)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upload_navigation_waits_for_controls_and_returns_for_manual_blockers() {
        let mut page: DouyinPage = serde_json::from_value(json!({
            "open": true, "path": "/creator-micro/home", "loginVisible": false,
            "online": true, "kind": "home"
        }))
        .unwrap();
        assert!(!upload_navigation_finished(&page));
        page.kind = "unknown".into();
        assert!(!upload_navigation_finished(&page));
        page.kind = "upload".into();
        assert!(!upload_navigation_finished(&page));
        page.upload_ready = true;
        assert!(upload_navigation_finished(&page));
        page.kind = "editor".into();
        assert!(!upload_navigation_finished(&page));
        page.editor_ready = true;
        assert!(upload_navigation_finished(&page));
        page.kind = "draft".into();
        assert!(upload_navigation_finished(&page));
        page.kind = "unknown".into();
        page.blocked = true;
        assert!(upload_navigation_finished(&page));
        page.blocked = false;
        page.login_visible = true;
        assert!(upload_navigation_finished(&page));
        page.login_visible = false;
        page.online = false;
        assert!(upload_navigation_finished(&page));
    }

    #[test]
    fn changed_fields_require_confirmation_again_without_losing_other_results() {
        let mut session = Session::new(1, "task".into());
        session.done("video", false);
        session.done("text", true);
        session.wait("text", "文案已变化，请重新确认。");
        assert_eq!(session.completed, ["video"]);
        assert!(session.manual.is_empty());
        assert_eq!(session.waiting_for.as_deref(), Some("text"));
    }

    #[test]
    fn old_decisions_and_stopped_runs_cannot_authorize_work() {
        let mut session = Session::new(2, "task".into());
        session.wait("account", "确认账号");
        assert!(session.validate_decision(1, false).is_err());
        assert!(session.validate_decision(2, true).is_err());
        assert!(session.validate_decision(2, false).is_ok());
        session.status = "review".into();
        assert!(session.validate_decision(2, false).is_err());
    }

    #[tokio::test]
    async fn stop_acknowledgement_waits_for_the_operation_and_stale_stop_cannot_cancel_new_work() {
        let publisher = DouyinPublisher::default();
        *publisher.session.lock().unwrap() = Some(Session::new(2, "task".into()));
        assert!(stop_current(&publisher, 1).await.is_err());
        assert!(!publisher.stopped.load(Ordering::SeqCst));
        let operation = publisher.operation.lock().await;
        let stop = stop_current(&publisher, 2);
        tokio::pin!(stop);
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(10), &mut stop)
                .await
                .is_err()
        );
        assert!(publisher.stopped.load(Ordering::SeqCst));
        drop(operation);
        assert_eq!(stop.await.unwrap().status, "stopped");
    }

    #[test]
    fn closing_reset_discards_page_proofs_and_collection_permissions() {
        let mut session = Session::new(1, "task".into());
        session.sent_video = true;
        session.video_key = "old-video".into();
        session.account_confirmed = true;
        session
            .cover_previews
            .push(("landscape".into(), json!(123)));
        session.cover_pending = true;
        session.collection_authorized = true;
        session.options.push(CollectionOption {
            id: None,
            name: "旧账号合集".into(),
        });
        session.done("video", true);
        session.reset_page_proofs();
        assert!(
            !session.sent_video
                && !session.account_confirmed
                && !session.cover_pending
                && !session.collection_authorized
        );
        assert!(
            session.video_key.is_empty()
                && session.completed.is_empty()
                && session.manual.is_empty()
                && session.options.is_empty()
                && session.cover_previews.is_empty()
        );
    }
    #[test]
    fn nickname_alone_never_proves_identity() {
        let a = Account {
            id: None,
            name: "相同昵称".into(),
            key: "123".into(),
        };
        let mut b = a.clone();
        b.key = "456".into();
        assert!(!same_account(Some(&a), Some(&b)));
        b.key = "123".into();
        assert!(same_account(Some(&a), Some(&b)));
        b.key = "0".into();
        assert!(!same_account(Some(&b), Some(&b)));
    }
    #[test]
    fn recovery_requires_the_same_video_and_marks_manual_results() {
        let mut session = Session::new(1, "task".into());
        let mut editor = EditorSnapshot {
            title: "".into(),
            description: "".into(),
            video_preview: true,
            cover_dialog: false,
            covers: vec![],
            video_key: "blob:video".into(),
        };
        assert!(verify_video(&session, &editor).is_err());
        session.video_key = editor.video_key.clone();
        assert!(verify_video(&session, &editor).is_ok());
        editor.video_key = "blob:another".into();
        assert!(verify_video(&session, &editor).is_err());
        session.done("covers", true);
        session.done("covers", true);
        assert_eq!(session.completed, ["covers"]);
        assert_eq!(session.manual, ["covers"]);
    }
}
