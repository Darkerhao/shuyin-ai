use crate::database::{publishing::PublishSnapshot, Database};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tauri::{webview::NewWindowResponse, AppHandle, Emitter, Manager, WebviewUrl, WebviewWindow};

const LABEL: &str = "douyin-publish";
const PORTAL: &str = "https://creator.douyin.com/";

#[derive(Default)]
pub struct DouyinPublisher {
    opening: Mutex<()>,
    operation: tokio::sync::Mutex<()>,
    task: Mutex<Option<PublishSnapshot>>,
    stopped: AtomicBool,
}

impl DouyinPublisher {
    pub fn release_recorded(&self, id: &str) {
        if let Ok(mut task) = self.task.lock() {
            if task.as_ref().is_some_and(|current| current.task_id == id) {
                self.stopped.store(true, Ordering::SeqCst);
                *task = None;
            }
        }
    }
}

fn allowed_url(url: &tauri::Url) -> bool {
    url.scheme() == "https"
        && url.host_str() == Some("creator.douyin.com")
        && url.port_or_known_default() == Some(443)
        && url.username().is_empty()
        && url.password().is_none()
}

fn local_main(window: &WebviewWindow) -> Result<(), String> {
    if window.label() != "main" {
        return Err("仅书引 AI 主窗口可以控制发布窗口。".into());
    }
    Ok(())
}

#[tauri::command]
pub async fn open_douyin_window(
    app: AppHandle,
    window: WebviewWindow,
    publisher: tauri::State<'_, DouyinPublisher>,
) -> Result<(), String> {
    local_main(&window)?;
    if !cfg!(windows) {
        return Err("应用内抖音发布目前仅在 Windows 验证。".into());
    }
    let _opening = publisher.opening.lock().map_err(|e| e.to_string())?;
    if let Some(existing) = app.get_webview_window(LABEL) {
        existing.show().map_err(|e| e.to_string())?;
        existing.unminimize().map_err(|e| e.to_string())?;
        return existing.set_focus().map_err(|e| e.to_string());
    }
    let directory = app
        .path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())?
        .join("douyin-webview");
    std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    let created = tauri::WebviewWindowBuilder::new(
        &app,
        LABEL,
        WebviewUrl::External(PORTAL.parse().unwrap()),
    )
    .title("抖音发布 · 书引 AI")
    .inner_size(1200.0, 850.0)
    .min_inner_size(900.0, 650.0)
    .data_directory(directory)
    .disable_drag_drop_handler()
    .on_navigation(allowed_url)
    .on_new_window(|_, _| NewWindowResponse::Deny)
    .build()
    .map_err(|e| format!("无法打开抖音发布窗口：{e}"))?;
    let handle = app.clone();
    created.on_window_event(move |event| {
        if matches!(event, tauri::WindowEvent::Destroyed) {
            let publisher = handle.state::<DouyinPublisher>();
            publisher.stopped.store(true, Ordering::SeqCst);
            if let Ok(mut task) = publisher.task.lock() {
                *task = None;
            };
        }
    });
    Ok(())
}

#[derive(Debug, Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DouyinPage {
    #[serde(default)]
    pub supported: bool,
    pub open: bool,
    pub path: String,
    pub login_visible: bool,
    pub online: bool,
    #[serde(default)]
    pub editor_ready: bool,
    #[serde(default)]
    pub editor: Option<EditorSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EditorSnapshot {
    pub title: String,
    pub description: String,
    pub video_preview: bool,
    pub cover_dialog: bool,
    pub covers: Vec<CoverSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CoverSnapshot {
    pub ratio: String,
    pub selected: bool,
    pub preview: String,
}

async fn evaluate(window: &WebviewWindow, script: String) -> Result<String, String> {
    if !allowed_url(&window.url().map_err(|e| e.to_string())?) {
        return Err("当前不是抖音创作者中心，已停止页面操作。".into());
    }
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let sender = Mutex::new(Some(sender));
    window
        .eval_with_callback(script, move |value| {
            if let Ok(mut sender) = sender.lock() {
                if let Some(sender) = sender.take() {
                    let _ = sender.send(value);
                }
            }
        })
        .map_err(|_| "无法读取抖音页面，请检查发布窗口。")?;
    tokio::time::timeout(std::time::Duration::from_secs(10), receiver)
        .await
        .map_err(|_| "抖音页面响应超时，请检查网络和发布窗口。")?
        .map_err(|_| "发布窗口已关闭。".into())
}

async fn page_action(
    window: &WebviewWindow,
    action: &str,
    payload: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let action = serde_json::to_string(action).map_err(|e| e.to_string())?;
    let script = format!(
        "({})({}, {})",
        include_str!("douyin_editor.js"),
        action,
        payload
    );
    let result = evaluate(window, script).await?;
    let value: serde_json::Value =
        serde_json::from_str(&result).map_err(|_| "抖音页面尚未就绪，请重新检查。")?;
    if value.get("ok").and_then(|ok| ok.as_bool()) == Some(false) {
        return Err(value["error"]
            .as_str()
            .unwrap_or("页面拒绝了当前操作，请人工接管。")
            .into());
    }
    Ok(value)
}

async fn page_element(
    window: &WebviewWindow,
    action: &str,
    ratio: Option<&str>,
) -> Result<String, String> {
    use crate::douyin_webview::call;
    use serde_json::json;
    let action_json = serde_json::to_string(action).map_err(|e| e.to_string())?;
    let payload = ratio.map_or_else(
        || "{}".to_owned(),
        |value| json!({"ratio":value}).to_string(),
    );
    let script = format!("(() => {{ const input = ({})({}, {}); if (!(input instanceof HTMLInputElement)) throw new Error(input.error || '上传控件无效。'); return input; }})()", include_str!("douyin_editor.js"), action_json, payload);
    let result = call(window, "Runtime.evaluate", json!({"expression":script})).await?;
    result["result"]["objectId"]
        .as_str()
        .filter(|_| result["result"]["subtype"] == "node")
        .map(str::to_owned)
        .ok_or_else(|| {
            result["exceptionDetails"]["exception"]["description"]
                .as_str()
                .unwrap_or("页面控件不唯一或状态不适合继续，请人工检查。")
                .lines()
                .next()
                .unwrap_or_default()
                .to_owned()
        })
}

async fn set_local_file(
    window: &WebviewWindow,
    action: &str,
    ratio: Option<&str>,
    path: &str,
    publisher: &DouyinPublisher,
    snapshot: &PublishSnapshot,
) -> Result<(), String> {
    use crate::douyin_webview::call;
    use serde_json::json;
    let object_id = page_element(window, action, ratio).await?;
    if action == "videoInput" && !publisher.stopped.load(Ordering::SeqCst) {
        // The target is known; reserve the task before sending the file, since a timeout is ambiguous.
        *publisher.task.lock().map_err(|e| e.to_string())? = Some(snapshot.clone());
    }
    let selected = match running(publisher, snapshot) {
        Ok(()) => {
            call(
                window,
                "DOM.setFileInputFiles",
                json!({"objectId":object_id,"files":[path]}),
            )
            .await
        }
        Err(error) => Err(error),
    };
    let _ = call(
        window,
        "Runtime.releaseObject",
        json!({"objectId":object_id}),
    )
    .await;
    selected.map(|_| ())
}

fn verify_snapshot(current: &PublishSnapshot, expected: &PublishSnapshot) -> Result<(), String> {
    if current != expected {
        return Err("发布任务或素材已变化，请检查任务包并重新开始。".into());
    }
    Ok(())
}

fn publish_window(app: &AppHandle) -> Result<WebviewWindow, String> {
    let window = app
        .get_webview_window(LABEL)
        .ok_or("请先打开抖音验证窗口。")?;
    if !allowed_url(&window.url().map_err(|e| e.to_string())?) {
        return Err("发布窗口已离开官方页面。请检查网页并人工接管。".into());
    }
    Ok(window)
}

#[tauri::command]
pub async fn upload_douyin_video(
    app: AppHandle,
    window: WebviewWindow,
    database: tauri::State<'_, Database>,
    publisher: tauri::State<'_, DouyinPublisher>,
    id: String,
    account_confirmed: bool,
) -> Result<(), String> {
    local_main(&window)?;
    if !account_confirmed {
        return Err("请先在官方页面核对发布账号并确认。".into());
    }
    let _operation = publisher
        .operation
        .try_lock()
        .map_err(|_| "正在操作，请勿重复点击。")?;
    let snapshot = database.publish_snapshot(&id)?;
    {
        let active = publisher.task.lock().map_err(|e| e.to_string())?;
        if active.is_some() {
            return Err("已有上传任务，请检查官方页面并人工接管，不能重复上传。".into());
        }
    }
    let publish_window = app
        .get_webview_window(LABEL)
        .ok_or("请先打开抖音窗口并登录。")?;
    if !allowed_url(&publish_window.url().map_err(|e| e.to_string())?) {
        return Err("当前不是抖音创作者中心。".into());
    }
    publisher.stopped.store(false, Ordering::SeqCst);
    set_local_file(
        &publish_window,
        "videoInput",
        None,
        &snapshot.video,
        &publisher,
        &snapshot,
    )
    .await?;
    progress(&app, &id, "视频已交给抖音，正在等待上传和处理完成…");
    let result = async {
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(1800);
        loop {
            running(&publisher, &snapshot)?;
            let page = inspect_douyin_window(app.clone(), window.clone()).await?;
            if !page.open || !page.online || page.login_visible {
                return Err("窗口、网络或登录状态已变化，请检查官方页面并人工接管。".into());
            }
            if let Some(editor) = page.editor {
                if editor.video_preview {
                    break;
                }
                if editor.cover_dialog {
                    return Err("网页出现弹窗，请人工处理后检查页面继续。".into());
                }
                page_action(&publish_window, "previewVideo", serde_json::json!({})).await?;
            }
            if tokio::time::Instant::now() >= deadline {
                return Err("视频等待超时，请检查抖音页面；不会重新上传。".into());
            }
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
        verify_snapshot(&database.publish_snapshot(&id)?, &snapshot)?;
        fill_task(&app, &publish_window, &database, &publisher, &snapshot)
            .await
            .map(|_| ())
    }
    .await;
    publisher.stopped.store(true, Ordering::SeqCst);
    result
}

#[tauri::command]
pub async fn inspect_douyin_window(
    app: AppHandle,
    window: WebviewWindow,
) -> Result<DouyinPage, String> {
    local_main(&window)?;
    let Some(publish_window) = app.get_webview_window(LABEL) else {
        return Ok(DouyinPage {
            supported: cfg!(windows),
            open: false,
            path: String::new(),
            login_visible: false,
            online: false,
            editor_ready: false,
            editor: None,
        });
    };
    let result = evaluate(&publish_window, include_str!("douyin_page.js").into()).await?;
    let mut page: DouyinPage =
        serde_json::from_str(&result).map_err(|_| "页面尚未就绪，请稍后重新检查。")?;
    page.supported = cfg!(windows);
    if page.path == "/creator-micro/content/post/video" && page.editor_ready && !page.login_visible
    {
        page.editor = Some(read_editor(&publish_window).await?);
    }
    Ok(page)
}

async fn read_editor(window: &WebviewWindow) -> Result<EditorSnapshot, String> {
    serde_json::from_value(page_action(window, "read", serde_json::json!({})).await?)
        .map_err(|_| "无法回读编辑器，请人工检查。".into())
}

fn running(publisher: &DouyinPublisher, snapshot: &PublishSnapshot) -> Result<(), String> {
    if publisher.stopped.load(Ordering::SeqCst) {
        return Err("已停止自动填写，请在官方页面继续处理。".into());
    }
    let active = publisher.task.lock().map_err(|e| e.to_string())?;
    verify_snapshot(active.as_ref().ok_or("发布窗口已关闭。")?, snapshot)
}

fn progress(app: &AppHandle, id: &str, message: &str) {
    let _ = app.emit_to(
        "main",
        "douyin-progress",
        serde_json::json!({"taskId":id,"message":message}),
    );
}

async fn wait_cover(
    window: &WebviewWindow,
    publisher: &DouyinPublisher,
    snapshot: &PublishSnapshot,
    ratio: &str,
    before: &serde_json::Value,
) -> Result<(), String> {
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(120);
    loop {
        running(publisher, snapshot)?;
        let current = page_action(window, "coverReady", serde_json::json!({"ratio":ratio})).await?;
        if current["ready"] == true && current["previewKey"] != before["previewKey"] {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("封面处理超时，请在官方窗口检查。不会再次上传。".into());
        }
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }
}

#[tauri::command]
pub async fn complete_douyin_task(
    app: AppHandle,
    window: WebviewWindow,
    database: tauri::State<'_, Database>,
    publisher: tauri::State<'_, DouyinPublisher>,
    id: String,
    existing_video_confirmed: bool,
) -> Result<EditorSnapshot, String> {
    local_main(&window)?;
    let _operation = publisher
        .operation
        .try_lock()
        .map_err(|_| "正在操作，请勿重复点击。")?;
    let snapshot = database.publish_snapshot(&id)?;
    {
        let active = publisher.task.lock().map_err(|e| e.to_string())?;
        if let Some(expected) = active.as_ref() {
            verify_snapshot(&snapshot, expected)?;
        } else if !existing_video_confirmed {
            return Err("重开后请先核对官方网页已有视频和账号，再确认继续。".into());
        }
    }
    if !existing_video_confirmed {
        return Err("请先核对官方网页已上传的视频和账号。".into());
    }
    let publish_window = publish_window(&app)?;
    let editor = read_editor(&publish_window).await?;
    if editor.cover_dialog {
        return Err("请在官方页关闭弹窗，确认视频上传处理完成。".into());
    }
    if editor.covers.iter().any(|cover| cover.selected) {
        return Err("网页已有封面，请人工接管，避免覆盖。".into());
    }
    *publisher.task.lock().map_err(|e| e.to_string())? = Some(snapshot.clone());
    publisher.stopped.store(false, Ordering::SeqCst);
    let result = fill_task(&app, &publish_window, &database, &publisher, &snapshot).await;
    publisher.stopped.store(true, Ordering::SeqCst);
    result
}

async fn fill_task(
    app: &AppHandle,
    publish_window: &WebviewWindow,
    database: &Database,
    publisher: &DouyinPublisher,
    snapshot: &PublishSnapshot,
) -> Result<EditorSnapshot, String> {
    let id = &snapshot.task_id;
    let mut editor = read_editor(publish_window).await?;
    if editor.cover_dialog || editor.covers.iter().any(|c| c.selected) {
        return Err("网页已有封面或弹窗，请人工接管，避免覆盖。".into());
    }
    if !editor.video_preview {
        running(publisher, snapshot)?;
        page_action(publish_window, "previewVideo", serde_json::json!({})).await?;
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        editor = read_editor(publish_window).await?;
    }
    if !editor.video_preview {
        return Err("视频仍在上传或处理中，请稍后检查。".into());
    }
    progress(app, id, "正在填写并核对标题和发布内容…");
    running(publisher, snapshot)?;
    page_action(
        publish_window,
        "fill",
        serde_json::to_value(&snapshot.input).map_err(|e| e.to_string())?,
    )
    .await?;
    for ratio in ["landscape", "portrait"] {
        running(publisher, snapshot)?;
        verify_snapshot(&database.publish_snapshot(id)?, snapshot)?;
        progress(
            app,
            id,
            if ratio == "landscape" {
                "正在设置横版 4:3 封面…"
            } else {
                "正在设置竖版 3:4 封面…"
            },
        );
        let action = if ratio == "landscape" {
            "openCover"
        } else {
            "switchCover"
        };
        page_action(publish_window, action, serde_json::json!({"ratio":ratio})).await?;
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        running(publisher, snapshot)?;
        let before = page_action(
            publish_window,
            "coverReady",
            serde_json::json!({"ratio":ratio}),
        )
        .await?;
        let path = if ratio == "landscape" {
            &snapshot.landscape
        } else {
            &snapshot.portrait
        };
        set_local_file(
            publish_window,
            "coverInput",
            Some(ratio),
            path,
            publisher,
            snapshot,
        )
        .await?;
        wait_cover(publish_window, publisher, snapshot, ratio, &before).await?;
    }
    running(publisher, snapshot)?;
    verify_snapshot(&database.publish_snapshot(id)?, snapshot)?;
    page_action(publish_window, "confirmCover", serde_json::json!({})).await?;
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(120);
    loop {
        running(publisher, snapshot)?;
        let editor = read_editor(publish_window).await?;
        if !editor.cover_dialog
            && editor.covers.len() == 2
            && editor.covers.iter().all(|c| c.selected)
        {
            verify_editor(&editor, snapshot)?;
            return Ok(editor);
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("封面保存尚未确认，请在官方窗口检查并人工接管。".into());
        }
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }
}

fn verify_editor(editor: &EditorSnapshot, snapshot: &PublishSnapshot) -> Result<(), String> {
    let description = [&snapshot.input.description, &snapshot.input.tags]
        .into_iter()
        .filter(|s| !s.is_empty())
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join("\n\n");
    let normalized = |value: &str| {
        value
            .replace('\u{200b}', "")
            .replace("\r\n", "\n")
            .trim()
            .to_owned()
    };
    if editor.title != snapshot.input.title
        || normalized(&editor.description) != normalized(&description)
    {
        return Err("文案回读与任务不一致，请人工检查。".into());
    }
    Ok(())
}

#[tauri::command]
pub fn stop_douyin_task(
    window: WebviewWindow,
    publisher: tauri::State<'_, DouyinPublisher>,
) -> Result<(), String> {
    local_main(&window)?;
    publisher.stopped.store(true, Ordering::SeqCst);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn navigation_is_restricted_to_the_official_https_origin() {
        for url in [
            PORTAL,
            "https://creator.douyin.com/creator-micro/content/upload",
        ] {
            assert!(allowed_url(&url.parse().unwrap()));
        }
        for url in [
            "http://creator.douyin.com/",
            "https://creator.douyin.com.evil.test/",
            "https://evil.test/?next=https://creator.douyin.com/",
            "https://creator.douyin.com:444/",
            "https://user@creator.douyin.com/",
            "file:///C:/private.txt",
            "https://tauri.localhost/",
        ] {
            assert!(!allowed_url(&url.parse().unwrap()), "{url}");
        }
    }

    fn snapshot() -> PublishSnapshot {
        PublishSnapshot {
            task_id: "task".into(),
            project_title: "一本书".into(),
            task_directory: "package".into(),
            video: "video.mp4".into(),
            landscape: "landscape.png".into(),
            portrait: "portrait.png".into(),
            input: crate::database::publishing::PublishInput {
                writing_version_id: None,
                project_id: "book".into(),
                title: "发布标题".into(),
                description: "第一行\r\n第二行".into(),
                tags: "#读书".into(),
                video_asset_id: Some("video".into()),
                audio_asset_id: Some("audio".into()),
                landscape_cover_id: Some("landscape".into()),
                portrait_cover_id: Some("portrait".into()),
                title_confirmed: true,
                covers_confirmed: true,
            },
        }
    }

    #[test]
    fn stopped_closed_and_changed_sessions_cannot_continue() {
        let publisher = DouyinPublisher::default();
        let original = snapshot();
        assert!(running(&publisher, &original).is_err());
        *publisher.task.lock().unwrap() = Some(original.clone());
        assert!(running(&publisher, &original).is_ok());
        let mut changed = original.clone();
        changed.input.title = "另一个标题".into();
        assert!(running(&publisher, &changed).is_err());
        publisher.stopped.store(true, Ordering::SeqCst);
        assert!(running(&publisher, &original).is_err());
        assert_eq!(publisher.task.lock().unwrap().as_ref().unwrap(), &original);
    }

    #[test]
    fn only_the_recorded_task_releases_the_active_upload() {
        let publisher = DouyinPublisher::default();
        *publisher.task.lock().unwrap() = Some(snapshot());
        publisher.release_recorded("another-task");
        assert!(publisher.task.lock().unwrap().is_some());
        assert!(!publisher.stopped.load(Ordering::SeqCst));
        publisher.release_recorded("task");
        assert!(publisher.task.lock().unwrap().is_none());
        assert!(publisher.stopped.load(Ordering::SeqCst));
    }

    #[test]
    fn readback_accepts_editor_line_endings_and_rejects_different_text() {
        let original = snapshot();
        let mut editor = EditorSnapshot {
            title: original.input.title.clone(),
            description: "第一行\n第二行\n\n#读书\u{200b}".into(),
            video_preview: true,
            cover_dialog: false,
            covers: vec![],
        };
        assert!(verify_editor(&editor, &original).is_ok());
        editor.description = "用户改动".into();
        assert!(verify_editor(&editor, &original).is_err());
    }
}
