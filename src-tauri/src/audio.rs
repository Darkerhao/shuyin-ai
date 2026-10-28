use crate::database::{
    audio::{AudioJob, BridgeObservation},
    Database,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::Mutex,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Manager};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const EXTENSION_ID: &str = "nplahghodghepogenmdekjafojiljohf";
const NOTEBOOK_HOME: &str = "https://notebook.google.com/";
const CONNECTED_FOR: Duration = Duration::from_secs(5 * 60);
const MAX_BRIDGE_MESSAGE: usize = 4 * 1024 * 1024;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioBridgeStatus {
    pub supported: bool,
    pub connected: bool,
    pub last_seen_seconds_ago: Option<u64>,
    pub host_name: String,
    pub message: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioJobView {
    pub job: Option<AudioJob>,
    pub active_job: Option<AudioJob>,
    pub bridge: AudioBridgeStatus,
}

pub struct AudioAutomation {
    supported: bool,
    host_name: String,
    pipe_name: String,
    last_seen: Mutex<Option<Instant>>,
    message: Mutex<Option<String>>,
    verified_sources: Mutex<HashSet<(String, i64)>>,
    chrome_path: Option<PathBuf>,
    extension_dir: Option<PathBuf>,
    instance: Option<InstanceGuard>,
}

impl AudioAutomation {
    pub fn initialize(app: &AppHandle) -> Self {
        #[cfg(target_os = "windows")]
        {
            Self::initialize_windows(app)
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = app;
            Self {
                supported: false,
                host_name: host_name(),
                pipe_name: String::new(),
                last_seen: Mutex::new(None),
                message: Mutex::new(Some("自动音频目前仅支持 Windows + Chrome。".into())),
                verified_sources: Mutex::new(HashSet::new()),
                chrome_path: None,
                extension_dir: None,
                instance: None,
            }
        }
    }

    pub fn start(&self, app: AppHandle) {
        if !self.supported || self.instance.is_none() {
            return;
        }
        let app_for_errors = app.clone();
        let pipe = self.pipe_name.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(error) = run_pipe_server(app, pipe).await {
                if let Some(state) = app_for_errors.try_state::<AudioAutomation>() {
                    state.set_message(format!("Chrome 扩展桥接不可用：{error}"));
                }
            }
        });
    }

    fn set_message(&self, value: impl Into<String>) {
        if let Ok(mut message) = self.message.lock() {
            *message = Some(value.into());
        }
    }

    fn touch(&self) {
        if let Ok(mut last_seen) = self.last_seen.lock() {
            *last_seen = Some(Instant::now());
        }
    }

    fn message_text(&self) -> Option<String> {
        self.message.lock().ok().and_then(|value| value.clone())
    }

    pub fn status(&self) -> AudioBridgeStatus {
        let last_seen = self.last_seen.lock().ok().and_then(|value| *value);
        AudioBridgeStatus {
            supported: self.supported,
            connected: last_seen.is_some_and(|value| value.elapsed() < CONNECTED_FOR),
            last_seen_seconds_ago: last_seen.map(|value| value.elapsed().as_secs()),
            host_name: self.host_name.clone(),
            message: self.message_text(),
        }
    }

    pub fn require_connected(&self) -> Result<(), String> {
        if !self.supported {
            return Err(self
                .message_text()
                .unwrap_or_else(|| "自动音频目前仅支持 Windows + Chrome。".into()));
        }
        if self.instance.is_none() {
            return Err(self
                .message_text()
                .unwrap_or_else(|| "另一实例正在管理音频任务。".into()));
        }
        if self.chrome_path.is_none() {
            return Err("未检测到 Google Chrome，请先安装 Chrome。".into());
        }
        if let Some(message) = self.message_text().filter(|value| value.contains("注册")) {
            return Err(message);
        }
        if !self.status().connected {
            return Err("Chrome 扩展尚未连接。请先安装并启用书引 AI Chrome 扩展。".into());
        }
        Ok(())
    }

    pub fn source_verified(&self, job_id: &str, run_id: i64) -> bool {
        self.verified_sources
            .lock()
            .ok()
            .is_some_and(|verified| verified.contains(&(job_id.to_owned(), run_id)))
    }

    pub fn mark_source_verified(&self, job_id: &str, run_id: i64) {
        if let Ok(mut verified) = self.verified_sources.lock() {
            verified.insert((job_id.to_owned(), run_id));
        }
    }

    pub fn open_notebook(&self, job_id: Option<&str>, database: &Database) -> Result<(), String> {
        let url = match job_id {
            Some(id) => database
                .control_audio_job(id, "open_browser")?
                .notebook_url
                .unwrap_or_else(|| NOTEBOOK_HOME.into()),
            None => NOTEBOOK_HOME.into(),
        };
        open_chrome(&url, &self.chrome_path)
    }

    pub fn open_extension_setup(&self) -> Result<(), String> {
        if !self.supported {
            return Err("自动音频目前仅支持 Windows + Chrome。".into());
        }
        let extension = self
            .extension_dir
            .as_ref()
            .ok_or("应用未找到 Chrome 扩展目录。")?;
        if !extension.is_dir() {
            return Err(format!("Chrome 扩展目录不存在：{}", extension.display()));
        }
        open_folder(extension)?;
        open_chrome("chrome://extensions/", &self.chrome_path)
    }

    #[cfg(target_os = "windows")]
    fn initialize_windows(app: &AppHandle) -> Self {
        let host_name = host_name();
        let chrome_path = find_chrome();
        let extension_dir = find_extension_dir(app);
        let instance = match acquire_instance() {
            Ok(Some(guard)) => guard,
            Ok(None) => {
                return Self {
                    supported: true,
                    host_name,
                    pipe_name: String::new(),
                    last_seen: Mutex::new(None),
                    message: Mutex::new(Some("另一实例正在管理音频任务。".into())),
                    verified_sources: Mutex::new(HashSet::new()),
                    chrome_path,
                    extension_dir,
                    instance: None,
                };
            }
            Err(error) => {
                return Self {
                    supported: true,
                    host_name,
                    pipe_name: String::new(),
                    last_seen: Mutex::new(None),
                    message: Mutex::new(Some(format!("音频桥接初始化失败：{error}"))),
                    verified_sources: Mutex::new(HashSet::new()),
                    chrome_path,
                    extension_dir,
                    instance: None,
                };
            }
        };
        let pipe = pipe_name();
        let host_manifest = app.path().app_local_data_dir().ok().map(|directory| {
            directory
                .join("audio-bridge")
                .join("hosts")
                .join(format!("{host_name}.json"))
        });
        let helper_path = find_helper_path();
        let mut message = None;
        if let (Some(manifest), Some(helper)) = (&host_manifest, &helper_path) {
            if !helper.is_file() {
                message = Some("未找到浏览器桥接程序，请重新安装应用。".into());
            } else if let Err(error) = register_host(manifest, helper, &host_name) {
                message = Some(format!("Chrome 集成注册失败：{error}"));
            }
        } else {
            message = Some("Chrome 集成路径初始化失败。".into());
        }
        if let Ok(directory) = app.path().app_local_data_dir() {
            let bridge_dir = directory.join("audio-bridge");
            let _ = std::fs::create_dir_all(&bridge_dir);
            let pipe_file = bridge_dir.join(if cfg!(debug_assertions) {
                "pipe-dev.txt"
            } else {
                "pipe.txt"
            });
            if let Err(error) = std::fs::write(&pipe_file, &pipe) {
                message = Some(format!("无法写入浏览器管道信息：{error}"));
            }
        }
        Self {
            supported: true,
            host_name,
            pipe_name: pipe,
            last_seen: Mutex::new(None),
            message: Mutex::new(message),
            verified_sources: Mutex::new(HashSet::new()),
            chrome_path,
            extension_dir,
            instance: Some(instance),
        }
    }
}

fn host_name() -> String {
    if cfg!(debug_assertions) {
        "com.shuyin.desktop.audio.dev".into()
    } else {
        "com.shuyin.desktop.audio".into()
    }
}

#[cfg(target_os = "windows")]
fn pipe_name() -> String {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!(r"\\.\pipe\shuyin-audio-{}-{stamp}", std::process::id())
}
#[cfg(not(target_os = "windows"))]
fn pipe_name() -> String {
    String::new()
}

#[derive(Debug, Deserialize)]
#[serde(
    tag = "action",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
enum BridgeRequest {
    Poll {
        request_id: Option<String>,
    },
    SourceChunk {
        request_id: Option<String>,
        job_id: String,
        run_id: i64,
        offset: i64,
    },
    Observe {
        request_id: Option<String>,
        job_id: String,
        run_id: i64,
        observation: BridgeObservation,
    },
    DownloadStarted {
        request_id: Option<String>,
        job_id: String,
        run_id: i64,
        download_id: i64,
    },
    DownloadComplete {
        request_id: Option<String>,
        job_id: String,
        run_id: i64,
        path: String,
    },
    ReportError {
        request_id: Option<String>,
        job_id: String,
        run_id: i64,
        code: String,
        message: String,
    },
}

impl BridgeRequest {
    fn request_id(&self) -> Option<String> {
        match self {
            Self::Poll { request_id }
            | Self::SourceChunk { request_id, .. }
            | Self::Observe { request_id, .. }
            | Self::DownloadStarted { request_id, .. }
            | Self::DownloadComplete { request_id, .. }
            | Self::ReportError { request_id, .. } => request_id.clone(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BridgeFailure {
    code: String,
    message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SourceChunkPayload {
    data: String,
    next_offset: i64,
    total_size: i64,
    name: String,
    sha256: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BridgeResponse {
    ok: bool,
    request_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    job: Option<serde_json::Value>,
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    source_chunk: Option<SourceChunkPayload>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<BridgeFailure>,
}

impl BridgeResponse {
    fn failure(request_id: Option<String>, code: &str, message: impl Into<String>) -> Self {
        Self {
            ok: false,
            request_id,
            job: None,
            source_chunk: None,
            error: Some(BridgeFailure {
                code: code.into(),
                message: message.into(),
            }),
        }
    }
}

async fn read_frame<R: tokio::io::AsyncRead + Unpin>(
    reader: &mut R,
) -> Result<Option<Vec<u8>>, String> {
    let mut length = [0u8; 4];
    match reader.read_exact(&mut length).await {
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(error) => return Err(error.to_string()),
    }
    let length = u32::from_le_bytes(length) as usize;
    if length == 0 || length > MAX_BRIDGE_MESSAGE {
        return Err("浏览器桥接消息大小无效。".into());
    }
    let mut payload = vec![0u8; length];
    reader
        .read_exact(&mut payload)
        .await
        .map_err(|error| error.to_string())?;
    Ok(Some(payload))
}

async fn write_frame<W: tokio::io::AsyncWrite + Unpin>(
    writer: &mut W,
    payload: &[u8],
) -> Result<(), String> {
    if payload.len() > MAX_BRIDGE_MESSAGE {
        return Err("浏览器桥接响应过大。".into());
    }
    writer
        .write_all(&(payload.len() as u32).to_le_bytes())
        .await
        .map_err(|error| error.to_string())?;
    writer
        .write_all(payload)
        .await
        .map_err(|error| error.to_string())?;
    writer.flush().await.map_err(|error| error.to_string())
}

async fn process_request(app: &AppHandle, request: BridgeRequest) -> BridgeResponse {
    let request_id = request.request_id();
    let Some(state) = app.try_state::<AudioAutomation>() else {
        return BridgeResponse::failure(request_id, "bridge_unavailable", "音频桥接状态不可用。");
    };
    state.touch();
    let database = app.state::<Database>();
    match request {
        BridgeRequest::Poll { .. } => match database.audio_bridge_job() {
            Ok(job) => BridgeResponse {
                ok: true,
                request_id,
                job: job.and_then(|value| serde_json::to_value(value).ok()),
                source_chunk: None,
                error: None,
            },
            Err(message) => BridgeResponse::failure(request_id, "request_failed", message),
        },
        BridgeRequest::SourceChunk {
            job_id,
            run_id,
            offset,
            ..
        } => {
            let verify = !state.source_verified(&job_id, run_id);
            match database.audio_source_chunk(&job_id, run_id, offset, verify) {
                Ok(chunk) => {
                    if verify {
                        state.mark_source_verified(&job_id, run_id);
                    }
                    BridgeResponse {
                        ok: true,
                        request_id,
                        job: None,
                        source_chunk: Some(SourceChunkPayload {
                            data: STANDARD.encode(chunk.bytes),
                            next_offset: chunk.next_offset,
                            total_size: chunk.total_size,
                            name: chunk.name,
                            sha256: chunk.sha256,
                        }),
                        error: None,
                    }
                }
                Err(error) => BridgeResponse::failure(request_id, "source_chunk", error),
            }
        }
        BridgeRequest::Observe {
            job_id,
            run_id,
            observation,
            ..
        } => match database.observe_audio_job(&job_id, run_id, &observation) {
            Ok(job) => BridgeResponse {
                ok: true,
                request_id,
                job: serde_json::to_value(job).ok(),
                source_chunk: None,
                error: None,
            },
            Err(message) => BridgeResponse::failure(request_id, "observe", message),
        },
        BridgeRequest::DownloadStarted {
            job_id,
            run_id,
            download_id,
            ..
        } => {
            if download_id <= 0 {
                BridgeResponse::failure(request_id, "download", "浏览器下载编号无效。")
            } else {
                match database.audio_download_started(&job_id, run_id) {
                    Ok(job) => BridgeResponse {
                        ok: true,
                        request_id,
                        job: serde_json::to_value(job).ok(),
                        source_chunk: None,
                        error: None,
                    },
                    Err(message) => BridgeResponse::failure(request_id, "download", message),
                }
            }
        }
        BridgeRequest::DownloadComplete {
            job_id,
            run_id,
            path,
            ..
        } => match database.complete_audio_download(&job_id, run_id, &path) {
            Ok(job) => BridgeResponse {
                ok: true,
                request_id,
                job: serde_json::to_value(job).ok(),
                source_chunk: None,
                error: None,
            },
            Err(message) => BridgeResponse::failure(request_id, "import", message),
        },
        BridgeRequest::ReportError {
            job_id,
            run_id,
            code,
            message,
            ..
        } => match database.audio_report_error(&job_id, run_id, &code, &message) {
            Ok(job) => BridgeResponse {
                ok: true,
                request_id,
                job: serde_json::to_value(job).ok(),
                source_chunk: None,
                error: None,
            },
            Err(error) => BridgeResponse::failure(request_id, "report_error", error),
        },
    }
}

#[cfg(target_os = "windows")]
async fn run_pipe_server(app: AppHandle, pipe_name: String) -> Result<(), String> {
    use tokio::net::windows::named_pipe::ServerOptions;
    loop {
        let server = ServerOptions::new()
            .create(&pipe_name)
            .map_err(|error| error.to_string())?;
        server.connect().await.map_err(|error| error.to_string())?;
        let request_app = app.clone();
        tauri::async_runtime::spawn(async move {
            let _ = handle_pipe_connection(request_app, server).await;
        });
    }
}

#[cfg(not(target_os = "windows"))]
async fn run_pipe_server(_app: AppHandle, _pipe_name: String) -> Result<(), String> {
    Ok(())
}

#[cfg(target_os = "windows")]
async fn handle_pipe_connection(
    app: AppHandle,
    mut server: tokio::net::windows::named_pipe::NamedPipeServer,
) -> Result<(), String> {
    while let Some(payload) = read_frame(&mut server).await? {
        let response = match serde_json::from_slice::<BridgeRequest>(&payload) {
            Ok(request) => process_request(&app, request).await,
            Err(error) => BridgeResponse::failure(
                None,
                "invalid_request",
                format!("浏览器桥接请求无效：{error}"),
            ),
        };
        let bytes = serde_json::to_vec(&response).map_err(|error| error.to_string())?;
        write_frame(&mut server, &bytes).await?;
    }
    Ok(())
}

#[cfg(target_os = "windows")]
struct InstanceGuard(usize);

#[cfg(target_os = "windows")]
impl Drop for InstanceGuard {
    fn drop(&mut self) {
        use windows_sys::Win32::Foundation::CloseHandle;
        unsafe {
            CloseHandle(self.0 as *mut std::ffi::c_void);
        }
    }
}

#[cfg(target_os = "windows")]
fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(target_os = "windows")]
fn acquire_instance() -> Result<Option<InstanceGuard>, String> {
    use windows_sys::Win32::{
        Foundation::{GetLastError, ERROR_ALREADY_EXISTS},
        System::Threading::CreateMutexW,
    };
    let name = wide(if cfg!(debug_assertions) {
        r"Local\ShuyinAI-AudioBridge-Dev"
    } else {
        r"Local\ShuyinAI-AudioBridge"
    });
    let handle = unsafe { CreateMutexW(std::ptr::null(), 0, name.as_ptr()) };
    if handle.is_null() {
        return Err(format!("无法创建音频实例互斥量：{}", unsafe {
            GetLastError()
        }));
    }
    if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
        use windows_sys::Win32::Foundation::CloseHandle;
        unsafe {
            CloseHandle(handle);
        }
        return Ok(None);
    }
    Ok(Some(InstanceGuard(handle as usize)))
}

#[cfg(target_os = "windows")]
fn find_chrome() -> Option<PathBuf> {
    let mut paths = Vec::new();
    if let Ok(root) = std::env::var("ProgramFiles") {
        paths.push(PathBuf::from(root).join(r"Google\Chrome\Application\chrome.exe"));
    }
    if let Ok(root) = std::env::var("ProgramFiles(x86)") {
        paths.push(PathBuf::from(root).join(r"Google\Chrome\Application\chrome.exe"));
    }
    if let Ok(root) = std::env::var("LOCALAPPDATA") {
        paths.push(PathBuf::from(root).join(r"Google\Chrome\Application\chrome.exe"));
    }
    paths.into_iter().find(|path| path.is_file())
}

#[cfg(target_os = "windows")]
fn find_extension_dir(app: &AppHandle) -> Option<PathBuf> {
    let resource = app
        .path()
        .resource_dir()
        .ok()
        .map(|path| path.join("browser-extension"));
    if resource.as_ref().is_some_and(|path| path.is_dir()) {
        return resource;
    }
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()?
        .join("browser-extension");
    source.is_dir().then_some(source)
}

#[cfg(target_os = "windows")]
fn find_helper_path() -> Option<PathBuf> {
    if let Ok(executable) = std::env::current_exe() {
        if let Some(path) = executable
            .parent()
            .map(|directory| directory.join("browser-bridge.exe"))
        {
            if path.is_file() {
                return Some(path);
            }
        }
    }
    let dev = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("binaries/browser-bridge-x86_64-pc-windows-msvc.exe");
    dev.is_file().then_some(dev)
}

#[cfg(target_os = "windows")]
fn write_registry(host_name: &str, manifest: &Path) -> Result<(), String> {
    use windows_sys::Win32::{
        Foundation::ERROR_SUCCESS,
        System::Registry::{
            RegCloseKey, RegCreateKeyExW, RegSetValueExW, HKEY_CURRENT_USER, KEY_WRITE,
            REG_OPTION_NON_VOLATILE, REG_SZ,
        },
    };
    let subkey = wide(&format!(
        r"Software\Google\Chrome\NativeMessagingHosts\{host_name}"
    ));
    let value = wide(&manifest.to_string_lossy());
    let value_name = wide("");
    let mut key = std::ptr::null_mut();
    let mut disposition = 0u32;
    let status = unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            subkey.as_ptr(),
            0,
            std::ptr::null(),
            REG_OPTION_NON_VOLATILE,
            KEY_WRITE,
            std::ptr::null(),
            &mut key,
            &mut disposition,
        )
    };
    if status != ERROR_SUCCESS {
        return Err(format!("无法创建 Chrome 注册表项：{status}"));
    }
    let bytes = value.as_ptr() as *const u8;
    let length = (value.len() * std::mem::size_of::<u16>()) as u32;
    let status = unsafe { RegSetValueExW(key, value_name.as_ptr(), 0, REG_SZ, bytes, length) };
    unsafe {
        RegCloseKey(key);
    }
    if status != ERROR_SUCCESS {
        return Err(format!("无法写入 Chrome 注册表值：{status}"));
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn register_host(manifest: &Path, helper: &Path, host_name: &str) -> Result<(), String> {
    if let Some(parent) = manifest.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let helper = std::fs::canonicalize(helper).unwrap_or_else(|_| helper.to_path_buf());
    let body = serde_json::json!({
        "name": host_name,
        "description": "Book Voice AI audio automation bridge",
        "path": helper,
        "type": "stdio",
        "allowed_origins": [format!("chrome-extension://{EXTENSION_ID}/")]
    });
    let bytes = serde_json::to_vec_pretty(&body).map_err(|error| error.to_string())?;
    let temporary = manifest.with_extension("json.tmp");
    std::fs::write(&temporary, bytes).map_err(|error| error.to_string())?;
    if manifest.exists() {
        std::fs::remove_file(manifest).map_err(|error| error.to_string())?;
    }
    std::fs::rename(&temporary, manifest).map_err(|error| error.to_string())?;
    write_registry(host_name, manifest)
}

#[cfg(target_os = "windows")]
fn open_chrome(url: &str, chrome: &Option<PathBuf>) -> Result<(), String> {
    if !(url.starts_with("https://") || url == "chrome://extensions/") {
        return Err("只允许打开固定 HTTPS 页面或扩展管理页。".into());
    }
    use std::os::windows::process::CommandExt;
    let chrome = chrome
        .as_ref()
        .ok_or("未检测到 Google Chrome，请先安装 Chrome。")?;
    std::process::Command::new(chrome)
        .arg("--new-window")
        .arg(url)
        .creation_flags(0x08000000)
        .spawn()
        .map_err(|error| format!("无法打开 Chrome：{error}"))?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn open_folder(path: &Path) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    std::process::Command::new("explorer.exe")
        .arg(path)
        .creation_flags(0x08000000)
        .spawn()
        .map_err(|error| format!("无法打开扩展目录：{error}"))?;
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn open_chrome(_url: &str, _chrome: &Option<PathBuf>) -> Result<(), String> {
    Err("自动音频目前仅支持 Windows + Chrome。".into())
}

#[cfg(not(target_os = "windows"))]
fn open_folder(_path: &Path) -> Result<(), String> {
    Err("自动音频目前仅支持 Windows + Chrome。".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_chunk_response_is_flat_for_the_extension() {
        let response = BridgeResponse {
            ok: true,
            request_id: Some("r1".into()),
            job: None,
            source_chunk: Some(SourceChunkPayload {
                data: "AA==".into(),
                next_offset: 1,
                total_size: 2,
                name: "书.pdf".into(),
                sha256: "a".repeat(64),
            }),
            error: None,
        };
        let value = serde_json::to_value(response).unwrap();
        assert_eq!(value["data"], "AA==");
        assert_eq!(value["nextOffset"], 1);
        assert_eq!(value["totalSize"], 2);
        assert_eq!(value["name"], "书.pdf");
        assert!(value.get("sourceChunk").is_none());
    }

    #[test]
    fn generation_complete_does_not_require_a_download_url() {
        let observation: BridgeObservation = serde_json::from_value(serde_json::json!({
            "kind": "generation_complete"
        }))
        .unwrap();
        assert!(matches!(
            observation,
            BridgeObservation::GenerationComplete {
                download_url: None,
                ..
            }
        ));
    }
}
