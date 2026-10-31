use crate::database::audio::AudioJob;
#[cfg(windows)]
use crate::database::{
    audio::{AudioExecutionJob, AudioObservation},
    Database,
};
use serde::Serialize;
use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex,
    },
    time::Duration,
};
#[cfg(windows)]
use tauri::WebviewWindow;
use tauri::{AppHandle, Manager};

#[cfg(windows)]
#[path = "audio_download.rs"]
mod download;
const LABEL: &str = "audio-notebook";
const HOME: &str = "https://notebook.google.com/";

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioBrowserStatus {
    pub supported: bool,
    pub open: bool,
    pub url: Option<String>,
    pub message: Option<String>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioJobView {
    pub job: Option<AudioJob>,
    pub active_job: Option<AudioJob>,
    pub browser: AudioBrowserStatus,
}
#[cfg(windows)]
#[derive(Clone)]
struct DownloadIntent {
    job_id: String,
    run_id: i64,
    url: String,
    path: std::path::PathBuf,
    started: bool,
    epoch: u64,
    armed_at: std::time::Instant,
    navigation_id: Option<u64>,
    redirects: Vec<String>,
}
#[cfg(windows)]
impl DownloadIntent {
    fn allow_navigation(&mut self, url: &str, navigation_id: u64, redirected: bool) -> bool {
        if !download_url(url) {
            return false;
        }
        match self.navigation_id {
            None if !redirected && self.url == url => {
                self.navigation_id = Some(navigation_id);
            }
            Some(expected) if redirected && expected == navigation_id => {
                self.redirects.push(url.to_owned());
            }
            _ => return false,
        }
        true
    }

    fn matches_url(&self, url: &str) -> bool {
        self.url == url || self.redirects.iter().any(|redirect| redirect == url)
    }
}
pub struct AudioAutomation {
    operation: tokio::sync::Mutex<()>,
    wake: tokio::sync::Notify,
    epoch: AtomicU64,
    window_task: Mutex<Option<String>>,
    #[cfg(windows)]
    waiting: Mutex<Option<(String, i64, String, std::time::Instant)>>,
    #[cfg(windows)]
    download: Mutex<Option<DownloadIntent>>,
    instance: Option<InstanceGuard>,
    message: Option<String>,
}
impl AudioAutomation {
    pub fn initialize(_app: &AppHandle) -> Self {
        let (instance, message) = if !cfg!(windows) {
            (
                None,
                Some("应用内自动音频目前仅在 Windows 支持，请使用手动音频导入。".into()),
            )
        } else {
            match acquire_instance() {
                Ok(Some(instance)) => (Some(instance), None),
                Ok(None) => (None, Some("另一应用实例正在管理音频任务。".into())),
                Err(message) => (None, Some(message)),
            }
        };
        Self {
            operation: tokio::sync::Mutex::new(()),
            wake: tokio::sync::Notify::new(),
            epoch: AtomicU64::new(0),
            window_task: Mutex::new(None),
            #[cfg(windows)]
            download: Mutex::new(None),
            #[cfg(windows)]
            waiting: Mutex::new(None),
            instance,
            message,
        }
    }
    pub fn status(&self, app: &AppHandle) -> AudioBrowserStatus {
        let window = app.get_webview_window(LABEL);
        AudioBrowserStatus {
            supported: cfg!(windows) && self.instance.is_some(),
            open: window.is_some(),
            url: window
                .and_then(|window| window.url().ok())
                .map(|url| url.to_string()),
            message: self.message.clone(),
        }
    }
    pub fn require_available(&self) -> Result<(), String> {
        if !cfg!(windows) {
            return Err("应用内自动音频目前仅在 Windows 支持，请使用手动音频导入。".into());
        }
        if self.instance.is_none() {
            return Err(self
                .message
                .clone()
                .unwrap_or_else(|| "音频任务不可用。".into()));
        }
        Ok(())
    }
    pub fn notify(&self) {
        self.wake.notify_one();
    }
    pub fn start(&self, app: AppHandle) {
        if self.require_available().is_err() {
            return;
        }
        tauri::async_runtime::spawn(async move {
            loop {
                let automation = app.state::<AudioAutomation>();
                let delay = automation.tick(&app).await;
                tokio::select! { _ = automation.wake.notified() => {}, _ = tokio::time::sleep(delay) => {} }
            }
        });
    }
    pub async fn open_notebook(
        &self,
        app: &AppHandle,
        job: &AudioJob,
        action: &str,
    ) -> Result<(), String> {
        self.require_available()?;
        let _operation = self.operation.lock().await;
        let same = self
            .window_task
            .lock()
            .map_err(|e| e.to_string())?
            .as_deref()
            == Some(&job.id);
        let url = job
            .notebook_url
            .as_deref()
            .unwrap_or(HOME)
            .parse::<tauri::Url>()
            .map_err(|e| e.to_string())?;
        if !notebook_url(&url) {
            return Err("音频任务的网站地址无效。".into());
        }
        if let Some(window) = app.get_webview_window(LABEL) {
            if !same {
                self.interrupt(app);
                window
                    .set_title(&format!("音频制作 · {} · 书引 AI", job.project_title))
                    .map_err(|e| e.to_string())?;
                window.navigate(url).map_err(|e| e.to_string())?;
                *self.window_task.lock().map_err(|e| e.to_string())? = Some(job.id.clone());
            } else if window.url().is_ok_and(|current| {
                needs_navigation(&current, &url, job.notebook_url.is_some(), action)
            }) {
                window.navigate(url).map_err(|e| e.to_string())?;
            } else if action == "refresh_browser" {
                window.reload().map_err(|e| e.to_string())?;
            }
            window.show().map_err(|e| e.to_string())?;
            window.unminimize().map_err(|e| e.to_string())?;
            return window.set_focus().map_err(|e| e.to_string());
        }
        let directory = app
            .path()
            .app_local_data_dir()
            .map_err(|e| e.to_string())?
            .join("audio-webview");
        std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
        let handle = app.clone();
        let created =
            tauri::WebviewWindowBuilder::new(app, LABEL, tauri::WebviewUrl::External(url))
                .title(format!("音频制作 · {} · 书引 AI", job.project_title))
                .inner_size(1200.0, 850.0)
                .min_inner_size(900.0, 650.0)
                .data_directory(directory)
                .disable_drag_drop_handler()
                .on_navigation(move |url| {
                    let state = handle.state::<AudioAutomation>();
                    // The native handler verifies the download's NavigationId and redirects.
                    #[cfg(windows)]
                    if state.download.lock().ok().is_some_and(|pending| {
                        pending.as_ref().is_some_and(|intent| {
                            intent.armed_at.elapsed() < Duration::from_secs(15)
                        })
                    }) {
                        return true;
                    }
                    state.epoch.fetch_add(1, Ordering::SeqCst);
                    state.notify();
                    allowed_url(url)
                })
                .on_new_window(|_, _| tauri::webview::NewWindowResponse::Deny)
                .build()
                .map_err(|e| format!("无法打开应用内音频网站：{e}"))?;
        #[cfg(windows)]
        if let Err(error) = download::attach(&created, app.clone()).await {
            let _ = created.destroy();
            return Err(error);
        }
        let handle = app.clone();
        created.on_window_event(move |event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                if let Some(window) = handle.get_webview_window(LABEL) {
                    let _ = window.hide();
                }
            }
        });
        *self.window_task.lock().map_err(|e| e.to_string())? = Some(job.id.clone());
        self.notify();
        Ok(())
    }
    pub fn interrupt(&self, app: &AppHandle) {
        self.epoch.fetch_add(1, Ordering::SeqCst);
        if let Some(window) = app.get_webview_window(LABEL) {
            let _ = window.eval("globalThis.__shuyinAudioRun = null");
            #[cfg(windows)]
            download::cancel(&window);
        }
        #[cfg(windows)]
        if let Ok(mut intent) = self.download.lock() {
            *intent = None;
        }
    }
    #[cfg(windows)]
    pub async fn confirm_notebook(
        &self,
        app: &AppHandle,
        database: &Database,
        job: &AudioJob,
        confirmed_url: &str,
    ) -> Result<AudioJob, String> {
        self.require_available()?;
        let _operation = self.operation.lock().await;
        let window = app
            .get_webview_window(LABEL)
            .ok_or("请先打开原任务的空白笔记本。")?;
        if self
            .window_task
            .lock()
            .map_err(|e| e.to_string())?
            .as_deref()
            != Some(&job.id)
        {
            return Err("当前网站不属于此音频任务。".into());
        }
        let url = window.url().map_err(|e| e.to_string())?;
        if !notebook_url(&url) || url.as_str() != confirmed_url {
            return Err("网站地址已变化，请查看当前网站后重新确认。".into());
        }
        let epoch = self.epoch.load(Ordering::SeqCst);
        let result = crate::webview_host::call(
            &window,
            "Runtime.evaluate",
            serde_json::json!({"expression": format!("(() => {{ {}; return {{ url: location.href, empty: globalThis.ShuyinAudioPage.canRecoverEmptyNotebook() }}; }})()", include_str!("audio_page.js")), "returnByValue": true}),
        ).await?;
        let value = &result["result"]["value"];
        if self.epoch.load(Ordering::SeqCst) != epoch
            || value["url"].as_str() != Some(confirmed_url)
        {
            return Err("网站已变化，请核对后重新确认。".into());
        }
        if value["empty"].as_bool() != Some(true) {
            return Err("未确认当前笔记本为空。必须明确显示 0 个来源且没有音频生成结果，才能恢复添加来源；已有内容请在网站核对或手动导入音频。".into());
        }
        let saved = database.confirm_audio_notebook(&job.id, job.run_id, confirmed_url)?;
        self.interrupt(app);
        Ok(saved)
    }
    #[cfg(not(windows))]
    async fn tick(&self, _app: &AppHandle) -> Duration {
        Duration::from_secs(60)
    }
    #[cfg(windows)]
    async fn tick(&self, app: &AppHandle) -> Duration {
        let _operation = self.operation.lock().await;
        let database = app.state::<Database>();
        let Ok(Some(job)) = database.audio_execution_job() else {
            return Duration::from_secs(60);
        };
        if matches!(
            job.status.as_str(),
            "paused" | "waiting_user" | "failed" | "succeeded" | "cancelled"
        ) {
            return Duration::from_secs(60);
        }
        match self.step(app, &database, &job).await {
            Ok(delay) => Duration::from_secs(delay),
            Err(message) => {
                if database
                    .audio_step_failed(&job.id, job.run_id, &message)
                    .is_ok()
                {
                    self.interrupt(app);
                }
                Duration::from_secs(60)
            }
        }
    }
    #[cfg(windows)]
    async fn step(
        &self,
        app: &AppHandle,
        database: &Database,
        job: &AudioExecutionJob,
    ) -> Result<u64, String> {
        {
            let mut waiting = self.waiting.lock().map_err(|e| e.to_string())?;
            if !waiting.as_ref().is_some_and(|(id, run, stage, _)| {
                *id == job.id && *run == job.run_id && *stage == job.stage
            }) {
                *waiting = Some((
                    job.id.clone(),
                    job.run_id,
                    job.stage.clone(),
                    std::time::Instant::now(),
                ));
            }
            let limit = match job.stage.as_str() {
                "generating" => 6 * 60 * 60,
                "source_processing" => 15 * 60,
                "downloading" | "importing" => 30 * 60,
                _ => 120,
            };
            if job.status != "waiting_login"
                && waiting
                    .as_ref()
                    .is_some_and(|(_, _, _, start)| start.elapsed() > Duration::from_secs(limit))
            {
                return Err("当前阶段等待超时。请查看网站；页面空白或报错时点击“刷新网站”或“重新打开”，核对后继续跟进；不会重新提交生成。".into());
            }
            if job.status == "waiting_login" {
                *waiting = None;
            }
        }
        if matches!(job.stage.as_str(), "downloading" | "importing") {
            if let Some(intent) = self.download.lock().map_err(|e| e.to_string())?.clone() {
                if intent.job_id == job.id && intent.run_id == job.run_id {
                    if !intent.started && intent.armed_at.elapsed() > Duration::from_secs(15) {
                        return Err("网站未启动本任务的精确下载，请核对后重试下载。".into());
                    }
                    return Ok(5);
                }
            }
        }
        if matches!(job.stage.as_str(), "importing" | "verify_remote")
            && database.retry_audio_import(&job.id, job.run_id)?.is_some()
        {
            return Ok(60);
        }
        let window = app
            .get_webview_window(LABEL)
            .ok_or("任务网站已关闭，请点击继续跟进重新打开。")?;
        let url = window.url().map_err(|e| e.to_string())?;
        // WebView2 starts at about:blank before its first navigation commits.
        // The existing stage timeout bounds this wait if navigation fails.
        if url.as_str() == "about:blank" {
            return Ok(2);
        }
        if !allowed_url(&url) {
            return Err(
                "网站加载失败或离开了官方网站。请检查网络后点击“重新打开”，恢复后继续跟进。".into(),
            );
        }
        if url.host_str() == Some("accounts.google.com") {
            database.observe_audio_job(&job.id, job.run_id, &AudioObservation::LoginRequired)?;
            return Ok(10);
        }
        if url
            .query_pairs()
            .any(|(key, value)| key == "location" && value == "unsupported")
        {
            return Err("Notebook 网站提示当前网络所在地区不受支持。请检查网络和服务可用地区后点击“重新打开”，恢复后继续跟进。".into());
        }
        if self
            .window_task
            .lock()
            .map_err(|e| e.to_string())?
            .as_deref()
            != Some(job.id.as_str())
        {
            return Err("当前网站不属于正在执行的音频任务。".into());
        }
        if job.stage == "verify_remote" && job.notebook_url.is_none() {
            return Err("原任务未保存笔记本地址。请查看原任务的空白笔记本，再点“继续跟进”确认关联；不会自动重新创建或提交生成。".into());
        }
        let epoch = self.epoch.load(Ordering::SeqCst);
        let intercept = job.stage == "adding_source"
            && matches!(
                job.source,
                crate::database::audio::AudioSourceData::Asset { .. }
            );
        if intercept {
            crate::webview_host::call(
                &window,
                "Page.setInterceptFileChooserDialog",
                serde_json::json!({"enabled":true}),
            )
            .await?;
        }
        let result = evaluate(&window, job, serde_json::json!({})).await;
        if intercept {
            let _ = crate::webview_host::call(
                &window,
                "Page.setInterceptFileChooserDialog",
                serde_json::json!({"enabled":false}),
            )
            .await;
        }
        // Creating a notebook may navigate before Runtime.evaluate can return.
        // The page-side creation marker must still match before its URL is adopted.
        if job.stage == "creating_notebook" && self.epoch.load(Ordering::SeqCst) != epoch {
            self.require_current(database, job, self.epoch.load(Ordering::SeqCst))?;
            if window
                .url()
                .is_ok_and(|url| notebook_url(&url) && url.path().starts_with("/notebook/"))
            {
                return Ok(2);
            }
        }
        let result = result?;
        self.require_current(database, job, epoch)?;
        let kind = result["kind"]
            .as_str()
            .ok_or("网页没有返回有效的任务观察。")?
            .to_owned();
        match kind.as_str() {
            "file_input_ready" => {
                let path = database.audio_source_snapshot(&job.id, job.run_id)?;
                self.require_current(database, job, epoch)?;
                database.observe_audio_job(
                    &job.id,
                    job.run_id,
                    &AudioObservation::SourceSubmitted,
                )?;
                set_source_file(&window, job, &path).await?;
            }
            "download_target" => {
                let url = result["downloadUrl"]
                    .as_str()
                    .ok_or("没有找到可精确关联的音频下载地址。")?
                    .to_owned();
                if !download_url(&url) {
                    return Err("网站提供的音频下载地址不受支持。".into());
                }
                let path = database.audio_download_path(&job.id, job.run_id)?;
                if path.exists() {
                    return Err("本次下载缓存已存在，请核对后继续重试下载。".into());
                }
                *self.download.lock().map_err(|e| e.to_string())? = Some(DownloadIntent {
                    job_id: job.id.clone(),
                    run_id: job.run_id,
                    url: url.clone(),
                    path,
                    started: false,
                    epoch,
                    armed_at: std::time::Instant::now(),
                    navigation_id: None,
                    redirects: Vec::new(),
                });
                let mut request = job.clone();
                request.stage = "begin_download".into();
                evaluate(&window, &request, serde_json::json!({"downloadUrl":url})).await?;
            }
            "waiting" | "navigating" | "source_processing" => {
                database.audio_checked(&job.id, job.run_id)?;
            }
            _ => {
                let observation: AudioObservation =
                    serde_json::from_value(result).map_err(|e| format!("网页任务观察无效：{e}"))?;
                database.observe_audio_job(&job.id, job.run_id, &observation)?;
            }
        }
        Ok(
            if kind == "generating" || (job.stage == "generating" && kind == "waiting") {
                60
            } else if matches!(kind.as_str(), "source_processing" | "waiting")
                && job.stage == "source_processing"
            {
                15
            } else {
                2
            },
        )
    }
    #[cfg(windows)]
    fn require_current(
        &self,
        database: &Database,
        job: &AudioExecutionJob,
        epoch: u64,
    ) -> Result<(), String> {
        let current = database.audio_execution_job()?.ok_or("音频任务已结束。")?;
        if self.epoch.load(Ordering::SeqCst) != epoch
            || current.id != job.id
            || current.run_id != job.run_id
            || matches!(
                current.status.as_str(),
                "paused" | "waiting_user" | "failed" | "cancelled" | "succeeded"
            )
        {
            return Err("网页或任务已变化，本次操作结果已失效，请核对后继续。".into());
        }
        Ok(())
    }
}
fn needs_navigation(
    current: &tauri::Url,
    target: &tauri::Url,
    recorded: bool,
    action: &str,
) -> bool {
    action == "reopen_browser"
        || current.as_str() == "about:blank"
        || (action == "refresh_browser"
            && (!allowed_url(current)
                || current
                    .query_pairs()
                    .any(|(key, value)| key == "location" && value == "unsupported")))
        || (recorded
            && current.host_str() != Some("accounts.google.com")
            && !(notebook_url(current)
                && current.path().trim_end_matches('/') == target.path().trim_end_matches('/')))
}
fn notebook_url(url: &tauri::Url) -> bool {
    url.scheme() == "https"
        && matches!(
            url.host_str(),
            Some(
                "notebook.google.com"
                    | "notebooklm.google.com"
                    | "notebook.google"
                    | "notebooklm.google"
            )
        )
        && url.port_or_known_default() == Some(443)
        && url.username().is_empty()
        && url.password().is_none()
}
fn allowed_url(url: &tauri::Url) -> bool {
    notebook_url(url)
        || (url.scheme() == "https"
            && url.host_str() == Some("accounts.google.com")
            && url.port_or_known_default() == Some(443)
            && url.username().is_empty()
            && url.password().is_none())
}
#[cfg(windows)]
fn download_url(url: &str) -> bool {
    tauri::Url::parse(url).is_ok_and(|url| {
        matches!(url.scheme(), "https" | "blob")
            && url.username().is_empty()
            && url.password().is_none()
    })
}
#[cfg(windows)]
async fn evaluate(
    window: &WebviewWindow,
    job: &AudioExecutionJob,
    context: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if !notebook_url(&window.url().map_err(|e| e.to_string())?) {
        return Err("当前不是 Notebook 页面。".into());
    }
    let token =
        serde_json::to_string(&format!("{}:{}", job.id, job.run_id)).map_err(|e| e.to_string())?;
    let script = format!("(async () => {{ globalThis.__shuyinAudioRun = {token}; {}; return await globalThis.ShuyinAudioPage.runStep({}, {}); }})()", include_str!("audio_page.js"), serde_json::to_string(job).map_err(|e| e.to_string())?, context);
    let result = crate::webview_host::call(
        window,
        "Runtime.evaluate",
        serde_json::json!({"expression":script,"awaitPromise":true,"returnByValue":true}),
    )
    .await?;
    page_result(result)
}
#[cfg(any(windows, test))]
fn page_result(result: serde_json::Value) -> Result<serde_json::Value, String> {
    if result.get("exceptionDetails").is_some() {
        return Err(result["exceptionDetails"]["exception"]["description"]
            .as_str()
            .unwrap_or("网页任务执行失败，请核对网站后继续。")
            .lines()
            .next()
            .unwrap_or_default()
            .to_owned());
    }
    let value = result["result"]
        .get("value")
        .cloned()
        .ok_or("网页没有返回可解析的任务结果。")?;
    if value["kind"] == "page_error" {
        let message = value["message"]
            .as_str()
            .filter(|text| !text.trim().is_empty())
            .unwrap_or("网页任务执行失败，请核对网站后继续。");
        let code = value["code"].as_str().unwrap_or("page_action_failed");
        return Err(format!("[{code}] {message}"));
    }
    Ok(value)
}
#[cfg(windows)]
async fn set_source_file(
    window: &WebviewWindow,
    job: &AudioExecutionJob,
    path: &std::path::Path,
) -> Result<(), String> {
    let marker =
        serde_json::to_string(&format!("{}:{}", job.id, job.run_id)).map_err(|e| e.to_string())?;
    let expression = format!("(() => {{ const inputs = [...document.querySelectorAll('input[type=file]')].filter(i => i.getAttribute('data-shuyin-audio-source') === {marker}); if (inputs.length !== 1) throw Error('原书上传控件已变化'); return inputs[0]; }})()");
    let result = crate::webview_host::call(
        window,
        "Runtime.evaluate",
        serde_json::json!({"expression":expression}),
    )
    .await?;
    let id = result["result"]["objectId"]
        .as_str()
        .filter(|_| result["result"]["subtype"] == "node")
        .ok_or("未找到当前任务唯一的文件上传控件。")?;
    let selected = crate::webview_host::call(
        window,
        "DOM.setFileInputFiles",
        serde_json::json!({"objectId":id,"files":[crate::paths::shell_path(path)]}),
    )
    .await;
    let _ = crate::webview_host::call(
        window,
        "Runtime.releaseObject",
        serde_json::json!({"objectId":id}),
    )
    .await;
    selected.map(|_| ())
}
struct InstanceGuard(#[cfg(windows)] usize);
#[cfg(windows)]
impl Drop for InstanceGuard {
    fn drop(&mut self) {
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(self.0 as _);
        }
    }
}
#[cfg(windows)]
fn acquire_instance() -> Result<Option<InstanceGuard>, String> {
    use windows_sys::Win32::{
        Foundation::{GetLastError, ERROR_ALREADY_EXISTS},
        System::Threading::CreateMutexW,
    };
    let name: Vec<u16> = (if cfg!(debug_assertions) {
        "Local\\ShuyinAI-AudioBridge-Dev"
    } else {
        "Local\\ShuyinAI-AudioBridge"
    })
    .encode_utf16()
    .chain(Some(0))
    .collect();
    let handle = unsafe { CreateMutexW(std::ptr::null(), 0, name.as_ptr()) };
    if handle.is_null() {
        return Err("无法取得音频任务实例锁。".into());
    }
    if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(handle);
        }
        return Ok(None);
    }
    Ok(Some(InstanceGuard(handle as usize)))
}
#[cfg(not(windows))]
fn acquire_instance() -> Result<Option<InstanceGuard>, String> {
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(windows)]
    #[test]
    fn download_redirects_require_the_original_navigation_and_server_redirect_flag() {
        let original = "https://lh3.googleusercontent.com/notebooklm/audio";
        let redirected = "https://lh3.google.com/redirected-audio";
        let mut intent = DownloadIntent {
            job_id: "job".into(),
            run_id: 1,
            url: original.into(),
            path: "audio.partial".into(),
            started: false,
            epoch: 1,
            armed_at: std::time::Instant::now(),
            navigation_id: None,
            redirects: Vec::new(),
        };
        assert!(!intent.allow_navigation(redirected, 42, true));
        assert!(!intent.allow_navigation("https://example.com/unrelated", 42, false));
        assert!(intent.allow_navigation(original, 42, false));
        assert!(!intent.allow_navigation(redirected, 99, true));
        assert!(!intent.allow_navigation(redirected, 42, false));
        assert!(!intent.allow_navigation("http://example.com/audio", 42, true));
        assert!(intent.allow_navigation(redirected, 42, true));
        assert!(intent.matches_url(original));
        assert!(intent.matches_url(redirected));
        assert!(!intent.matches_url("https://example.com/unobserved"));
        assert!(intent.allow_navigation("https://media.example.com/final", 42, true));
        assert!(intent.matches_url("https://media.example.com/final"));
    }

    #[test]
    fn page_errors_preserve_the_code_and_complete_message() {
        let result = serde_json::json!({"result":{"value":{
            "kind":"page_error",
            "code":"ambiguous_control",
            "message":"来源弹窗中有多个上传入口。\n请核对后继续。"
        }}});
        assert_eq!(
            page_result(result),
            Err("[ambiguous_control] 来源弹窗中有多个上传入口。\n请核对后继续。".into())
        );
    }

    #[test]
    fn page_results_preserve_observations_and_report_protocol_exceptions() {
        let observation = serde_json::json!({"kind":"file_input_ready"});
        assert_eq!(
            page_result(serde_json::json!({"result":{"value":observation}})),
            Ok(observation)
        );
        assert_eq!(
            page_result(serde_json::json!({"exceptionDetails":{}})),
            Err("网页任务执行失败，请核对网站后继续。".into())
        );
    }

    #[test]
    fn recovery_navigation_preserves_login_and_official_notebook_redirects() {
        let target = "https://notebook.google.com/notebook/task".parse().unwrap();
        for action in ["open_browser", "resume", "refresh_browser"] {
            for current in [
                "https://accounts.google.com/signin",
                "https://notebook.google/notebook/task",
            ] {
                assert!(!needs_navigation(
                    &current.parse().unwrap(),
                    &target,
                    true,
                    action
                ));
            }
        }
        assert!(needs_navigation(
            &"about:blank".parse().unwrap(),
            &target,
            true,
            "refresh_browser"
        ));
        assert!(needs_navigation(
            &"https://notebook.google/notebook/other".parse().unwrap(),
            &target,
            true,
            "refresh_browser"
        ));
        for current in [
            "chrome-error://chromewebdata/",
            "https://notebook.google/?location=unsupported",
        ] {
            assert!(needs_navigation(
                &current.parse().unwrap(),
                &target,
                false,
                "refresh_browser"
            ));
        }
        assert!(needs_navigation(
            &"https://accounts.google.com/signin".parse().unwrap(),
            &target,
            true,
            "reopen_browser"
        ));
    }
    #[test]
    fn only_exact_official_navigation_is_allowed() {
        for url in [
            HOME,
            "https://notebooklm.google.com/notebook/a",
            "https://notebooklm.google/?location=unsupported",
            "https://notebook.google/?location=unsupported",
            "https://accounts.google.com/signin",
        ] {
            assert!(allowed_url(&url.parse().unwrap()));
        }
        for url in [
            "http://notebook.google.com",
            "https://notebook.google.com.evil.test",
            "https://evil.test/?notebook.google.com",
            "https://notebook.google.com:444",
            "https://user@notebook.google.com",
            "https://notebook.google.evil.test",
            "https://notebooklm.google:444",
            "https://user@notebook.google",
            "about:blank",
        ] {
            assert!(!allowed_url(&url.parse().unwrap()));
        }
    }
}
