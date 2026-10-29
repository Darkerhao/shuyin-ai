#[cfg(any(target_os = "windows", test))]
use super::{assets, projects::safe_book_name};
use super::{
    projects::{get, new_id, NOW},
    Database,
};
use crate::{ai::WritingContent, database::ai};
use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
#[cfg(any(target_os = "windows", test))]
use std::{
    fs::OpenOptions,
    io::{BufWriter, Seek, SeekFrom, Write},
};
use std::{
    fs::{self, File},
    io::{BufReader, Read},
    path::{Path, PathBuf},
};

const JOB_COLUMNS: &str = "j.id,p.title,j.project_id,j.writing_version_id,j.input_json,j.status,j.stage,j.run_id,j.error_code,j.error_message,j.notebook_url,j.remote_audio_ref,j.output_size,j.output_sha256,j.asset_id,j.submitted_at,j.last_checked_at,j.created_at,j.updated_at";
const ACTIVE_JOB_QUERY: &str = "SELECT j.id,p.title,j.project_id,j.writing_version_id,j.input_json,j.status,j.stage,j.run_id,j.error_code,j.error_message,j.notebook_url,j.remote_audio_ref,j.output_size,j.output_sha256,j.asset_id,j.submitted_at,j.last_checked_at,j.created_at,j.updated_at\nFROM audio_jobs j JOIN book_projects p ON p.id=j.project_id\nWHERE j.status NOT IN ('succeeded','cancelled')\nORDER BY j.updated_at DESC,j.created_at DESC LIMIT 1";

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StartAudioJobInput {
    pub project_id: String,
    pub writing_version_id: String,
    pub source: AudioSourceInput,
    pub format: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum AudioSourceInput {
    Asset { asset_id: String },
    Text,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AudioJob {
    pub id: String,
    pub project_id: String,
    pub project_title: String,
    pub writing_version_id: Option<String>,
    pub status: String,
    pub stage: String,
    pub run_id: i64,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub notebook_url: Option<String>,
    pub remote_audio_ref: Option<String>,
    pub output_size: Option<i64>,
    pub output_sha256: Option<String>,
    pub asset_id: Option<String>,
    pub format: String,
    pub source_label: String,
    pub target_duration_seconds: i64,
    pub submitted_at: Option<String>,
    pub last_checked_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioJobData {
    pub job: Option<AudioJob>,
    pub active_job: Option<AudioJob>,
}

#[cfg(any(target_os = "windows", test))]
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum AudioObservation {
    LoginRequired,
    LoginReady,
    NotebookCreated {
        notebook_url: String,
    },
    SourceSubmitted,
    SourceReady,
    AudioConfigured,
    GenerationSubmitted,
    Generating,
    GenerationComplete {
        #[serde(default)]
        download_url: Option<String>,
        #[serde(default)]
        suggested_filename: Option<String>,
    },
    PageChanged {
        message: String,
    },
}

#[cfg(any(target_os = "windows", test))]
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioExecutionJob {
    pub id: String,
    pub run_id: i64,
    pub status: String,
    pub stage: String,
    pub project_title: String,
    pub prompt: String,
    pub target_duration_seconds: i64,
    pub format: String,
    pub notebook_url: Option<String>,
    pub source: AudioSourceData,
}

#[cfg(any(target_os = "windows", test))]
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AudioSourceData {
    Text {
        text: String,
    },
    Asset {
        name: String,
        size: i64,
        sha256: String,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct FrozenAudioInput {
    prompt_version: String,
    prompt: String,
    target_duration_seconds: i64,
    format: String,
    source: FrozenSource,
}

impl FrozenAudioInput {
    fn source_label(&self) -> String {
        match &self.source {
            FrozenSource::Asset { name, .. } => name.clone(),
            FrozenSource::Text { .. } => "书籍资料文本".into(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum FrozenSource {
    Asset {
        asset_id: String,
        name: String,
        size: i64,
        sha256: String,
    },
    Text {
        text: String,
    },
}

struct PreparedSource {
    frozen: FrozenSource,
    path: Option<PathBuf>,
}

fn parse_input(json: &str) -> Result<FrozenAudioInput, String> {
    serde_json::from_str(json).map_err(|_| "音频任务快照损坏。".into())
}

fn row_job(row: &Row<'_>) -> rusqlite::Result<AudioJob> {
    let input_json: String = row.get(4)?;
    let input = parse_input(&input_json).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            4,
            rusqlite::types::Type::Text,
            Box::new(std::io::Error::other(error)),
        )
    })?;
    Ok(AudioJob {
        id: row.get(0)?,
        project_title: row.get(1)?,
        project_id: row.get(2)?,
        writing_version_id: row.get(3)?,
        status: row.get(5)?,
        stage: row.get(6)?,
        run_id: row.get(7)?,
        error_code: row.get(8)?,
        error_message: row.get(9)?,
        notebook_url: row.get(10)?,
        remote_audio_ref: row.get(11)?,
        output_size: row.get(12)?,
        output_sha256: row.get(13)?,
        asset_id: row.get(14)?,
        format: input.format.clone(),
        source_label: input.source_label(),
        target_duration_seconds: input.target_duration_seconds,
        submitted_at: row.get(15)?,
        last_checked_at: row.get(16)?,
        created_at: row.get(17)?,
        updated_at: row.get(18)?,
    })
}

fn job(connection: &Connection, id: &str) -> Result<AudioJob, String> {
    connection.query_row(
        &format!("SELECT {JOB_COLUMNS} FROM audio_jobs j JOIN book_projects p ON p.id=j.project_id WHERE j.id=?1"),
        [id], row_job,
    ).optional().map_err(|error| error.to_string())?.ok_or_else(|| "音频任务不存在。".into())
}

fn latest_job(connection: &Connection, project_id: &str) -> Result<Option<AudioJob>, String> {
    connection.query_row(
        &format!("SELECT {JOB_COLUMNS} FROM audio_jobs j JOIN book_projects p ON p.id=j.project_id WHERE j.project_id=?1 ORDER BY j.created_at DESC,j.updated_at DESC LIMIT 1"),
        [project_id], row_job,
    ).optional().map_err(|error| error.to_string())
}

fn active_job(connection: &Connection) -> Result<Option<AudioJob>, String> {
    connection
        .query_row(ACTIVE_JOB_QUERY, [], row_job)
        .optional()
        .map_err(|error| error.to_string())
}

fn require_run(job: &AudioJob, run_id: i64) -> Result<(), String> {
    if job.run_id != run_id {
        return Err("这次浏览器动作已过期，请刷新任务后重试。".into());
    }
    Ok(())
}

#[cfg(any(target_os = "windows", test))]
fn validate_notebook_url(value: &str) -> bool {
    tauri::Url::parse(value).is_ok_and(|url| {
        url.scheme() == "https"
            && matches!(
                url.host_str(),
                Some("notebook.google.com" | "notebooklm.google.com")
            )
            && url.username().is_empty()
            && url.password().is_none()
            && url.port_or_known_default() == Some(443)
            && url.path().strip_prefix("/notebook/").is_some_and(|id| {
                !id.is_empty()
                    && id
                        .bytes()
                        .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
            })
    })
}

fn valid_format(value: &str) -> bool {
    matches!(value, "deep_dive" | "brief")
}

fn supported_source_name(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str(),
        "pdf" | "epub" | "txt" | "md" | "docx"
    )
}

fn hash_file(path: &Path) -> Result<(i64, String), String> {
    let metadata = fs::metadata(path).map_err(|error| format!("来源文件不可用：{error}"))?;
    if !metadata.is_file() || metadata.len() == 0 {
        return Err("来源文件为空或不可读取。".into());
    }
    let mut file = BufReader::new(File::open(path).map_err(|error| error.to_string())?);
    let mut hasher = Sha256::new();
    let mut total = 0u64;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer).map_err(|error| error.to_string())?;
        if read == 0 {
            break;
        }
        total += read as u64;
        hasher.update(&buffer[..read]);
    }
    Ok((total as i64, hex_digest(&hasher.finalize())))
}

#[cfg(any(target_os = "windows", test))]
fn audio_format(path: &Path) -> Result<&'static str, String> {
    let metadata = fs::metadata(path).map_err(|error| format!("下载的音频不可用：{error}"))?;
    if !metadata.is_file() || metadata.len() < 256 {
        return Err("下载的音频为空或不完整。".into());
    }
    let mut file = File::open(path).map_err(|error| error.to_string())?;
    let mut header = [0u8; 4096];
    let read = file.read(&mut header).map_err(|error| error.to_string())?;
    let bytes = &header[..read];
    if bytes.starts_with(b"ID3")
        || (bytes.len() >= 2 && bytes[0] == 0xff && bytes[1] & 0xe0 == 0xe0 && bytes[1] & 0x06 != 0)
    {
        return Ok("mp3");
    }
    if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WAVE" {
        return Ok("wav");
    }
    if bytes.starts_with(b"fLaC") {
        return Ok("flac");
    }
    if bytes.starts_with(b"OggS") {
        if bytes.windows(8).any(|window| window == b"OpusHead")
            || bytes.windows(6).any(|window| window == b"vorbis")
        {
            return Ok("ogg");
        }
        return Err("OGG 文件缺少音频编码信息。".into());
    }
    if bytes.len() >= 2 && bytes[0] == 0xff && bytes[1] & 0xf6 == 0xf0 {
        return Ok("aac");
    }
    if bytes.len() >= 12 && &bytes[4..8] == b"ftyp" {
        file.seek(SeekFrom::Start(0))
            .map_err(|error| error.to_string())?;
        let context =
            mp4parse::read_mp4(&mut file).map_err(|_| "M4A 音频无法解析，请重新下载。")?;
        let has_audio = context
            .tracks
            .iter()
            .any(|track| track.track_type == mp4parse::TrackType::Audio);
        let has_video = context
            .tracks
            .iter()
            .any(|track| track.track_type == mp4parse::TrackType::Video);
        if has_audio && !has_video {
            return Ok("m4a");
        }
        return Err("下载内容不是可用的 M4A 音频。".into());
    }
    Err("下载内容不是可识别的音频文件。".into())
}

fn hex_digest(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(any(target_os = "windows", test))]
fn checked_directory(path: &Path) -> Result<(), String> {
    if !path.is_dir() {
        return Err("项目工作目录已丢失，请重新指定目录。".into());
    }
    Ok(())
}

#[cfg(any(target_os = "windows", test))]
fn copy_verified(source: &Path, destination: &Path) -> Result<(i64, String), String> {
    let (source_size, source_hash) = hash_file(source)?;
    if destination.exists() {
        let (existing_size, existing_hash) = hash_file(destination)?;
        if existing_size == source_size && existing_hash == source_hash {
            return Ok((existing_size, existing_hash));
        }
        return Err("目标音频已存在且内容不同，不会覆盖已有文件。".into());
    }
    let partial = destination.with_extension(format!(
        "{}.partial",
        destination
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("audio")
    ));
    if partial.exists() {
        fs::remove_file(&partial).map_err(|error| format!("无法清理旧的临时音频：{error}"))?;
    }
    let mut reader = BufReader::new(File::open(source).map_err(|error| error.to_string())?);
    let output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&partial)
        .map_err(|error| format!("无法创建音频临时文件：{error}"))?;
    let mut writer = BufWriter::new(output);
    let mut hasher = Sha256::new();
    let mut total = 0u64;
    let mut buffer = [0u8; 64 * 1024];
    let result = (|| {
        loop {
            let read = reader
                .read(&mut buffer)
                .map_err(|error| error.to_string())?;
            if read == 0 {
                break;
            }
            writer
                .write_all(&buffer[..read])
                .map_err(|error| error.to_string())?;
            hasher.update(&buffer[..read]);
            total += read as u64;
        }
        writer.flush().map_err(|error| error.to_string())?;
        writer
            .get_ref()
            .sync_all()
            .map_err(|error| error.to_string())?;
        let hash = hex_digest(&hasher.finalize());
        if total as i64 != source_size || hash != source_hash {
            return Err("复制后的音频大小或 SHA-256 不一致。".into());
        }
        drop(writer);
        if destination.exists() {
            return Err("目标音频在复制期间出现，不会覆盖已有文件。".into());
        }
        fs::rename(&partial, destination).map_err(|error| format!("无法保存最终音频：{error}"))?;
        Ok((total as i64, hash))
    })();
    if result.is_err() {
        let _ = fs::remove_file(&partial);
    }
    result
}

impl Database {
    pub fn audio_job_data(&self, project_id: &str) -> Result<AudioJobData, String> {
        self.with_connection(|connection| {
            get(connection, project_id)?;
            Ok(AudioJobData {
                job: latest_job(connection, project_id)?,
                active_job: active_job(connection)?,
            })
        })
    }

    pub fn start_audio_job(&self, input: StartAudioJobInput) -> Result<AudioJob, String> {
        if !valid_format(&input.format) {
            return Err("音频形式只能是深入探索或简报。".into());
        }
        if let Some(existing) = self.with_connection(|connection| active_job(connection))? {
            if existing.project_id == input.project_id {
                return Ok(existing);
            }
            return Err(format!(
                "当前已有《{}》的音频任务。请先完成或停止跟进。",
                existing.project_title
            ));
        }
        let (project, version, source) = self.with_connection(|connection| {
            let project = get(connection, &input.project_id)?;
            let version = ai::version(connection, &input.writing_version_id)?;
            if version.project_id != project.id || version.kind != "publishing" {
                return Err("音频任务必须绑定当前书籍的发布文案版本。".into());
            }
            let WritingContent::Publishing { audio_prompt, .. } = &version.content else { return Err("所选版本不是发布文案。".into()); };
            if audio_prompt.trim().is_empty() { return Err("所选版本的音频生成提示词为空。".into()); }
            let source = match &input.source {
                AudioSourceInput::Text => {
                    if project.input.source_notes.trim().is_empty() { return Err("本书还没有可用的资料文本。".into()); }
                    if project.input.source_notes.chars().count() > 200_000 { return Err("书籍资料文本过长，请精简后再试。".into()); }
                    PreparedSource { frozen: FrozenSource::Text { text: project.input.source_notes.clone() }, path: None }
                }
                AudioSourceInput::Asset { asset_id } => {
                    let row: Option<(String, String, String)> = connection.query_row(
                        "SELECT asset_type,file_path,original_name FROM assets WHERE id=?1 AND project_id=?2",
                        params![asset_id, project.id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                    ).optional().map_err(|error| error.to_string())?;
                    let (kind, path, name) = row.ok_or("所选原书文件不属于当前书籍。")?;
                    if kind != "document" { return Err("原书来源必须是已登记的文档素材。".into()); }
                    let path = fs::canonicalize(&path).map_err(|error| format!("原书文件不可用：{error}"))?;
                    if !supported_source_name(&path) { return Err("原书来源只支持 PDF、EPUB、TXT 或 Markdown 文件。".into()); }
                    PreparedSource { frozen: FrozenSource::Asset { asset_id: asset_id.clone(), name, size: 0, sha256: String::new() }, path: Some(path) }
                }
            };
            Ok((project, version, source))
        })?;
        let (size, sha256) = match (&input.source, &source.frozen, &source.path) {
            (
                AudioSourceInput::Asset { asset_id },
                FrozenSource::Asset { name, .. },
                Some(path),
            ) => {
                let (size, sha256) = hash_file(path)?;
                let (current_kind, current_size, current_name): (String, i64, String) = self.with_connection(|connection| connection.query_row(
                    "SELECT asset_type,file_size,original_name FROM assets WHERE id=?1 AND project_id=?2",
                    params![asset_id, project.id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                ).map_err(|error| error.to_string()))?;
                if current_kind != "document" || current_size != size || current_name != *name {
                    return Err("原书文件在创建任务时发生变化，请重新选择。".into());
                }
                (size, sha256)
            }
            (AudioSourceInput::Text, FrozenSource::Text { .. }, None) => (0, String::new()),
            _ => return Err("来源快照无效。".into()),
        };
        let frozen_source = match source.frozen {
            FrozenSource::Asset { asset_id, name, .. } => FrozenSource::Asset {
                asset_id,
                name,
                size,
                sha256,
            },
            text @ FrozenSource::Text { .. } => text,
        };
        let input_json = serde_json::to_string(&FrozenAudioInput {
            prompt_version: version.prompt_version,
            prompt: match version.content {
                WritingContent::Publishing { audio_prompt, .. } => audio_prompt,
                _ => unreachable!(),
            },
            target_duration_seconds: project.input.target_duration_seconds,
            format: input.format,
            source: frozen_source,
        })
        .map_err(|error| error.to_string())?;
        self.with_connection(|connection| {
            if let Some(existing) = active_job(connection)? {
                if existing.project_id == input.project_id { return Ok(existing); }
                return Err(format!("当前已有《{}》的音频任务。请先完成或停止跟进。", existing.project_title));
            }
            let id = new_id(connection)?;
            connection.execute(
                &format!("INSERT INTO audio_jobs(id,project_id,writing_version_id,input_json,status,stage,run_id,created_at,updated_at) VALUES(?1,?2,?3,?4,'created','opening_site',1,{NOW},{NOW})"),
                params![id, input.project_id, input.writing_version_id, input_json],
            ).map_err(|error| match error {
                rusqlite::Error::SqliteFailure(ref code, _) if code.code == rusqlite::ErrorCode::ConstraintViolation => "当前已有未完成的音频任务。请先完成或停止跟进。".to_owned(),
                _ => error.to_string(),
            })?;
            job(connection, &id)
        })
    }

    pub fn control_audio_job(&self, id: &str, action: &str) -> Result<AudioJob, String> {
        self.with_connection(|connection| {
            let current = job(connection, id)?;
            match action {
                "pause" => {
                    if matches!(current.status.as_str(), "succeeded" | "cancelled") { return Err("已完成或已停止的任务不能暂停。".into()); }
                    connection.execute(&format!("UPDATE audio_jobs SET status='paused',run_id=run_id+1,error_code=NULL,error_message=NULL,updated_at={NOW} WHERE id=?1"), [id]).map_err(|error| error.to_string())?;
                }
                "resume" => {
                    if !matches!(current.status.as_str(), "paused" | "waiting_user") { return Err("只有已暂停或等待人工核对的任务可以继续。".into()); }
                    let stage = match current.stage.as_str() {
                        "awaiting_extension" | "opening_site" => "opening_site",
                        "download_pending" => "downloading",
                        "creating_notebook" if current.notebook_url.is_none() => "verify_remote",
                        "submitting" => "verify_remote",
                        other => other,
                    };
                    let status = match stage {
                        "generating" => "generating",
                        "download_pending" | "downloading" => "downloading",
                        "importing" => "importing",
                        "verify_remote" | "login" | "creating_notebook" | "adding_source" | "source_processing" | "configuring_audio" | "submitting" => "preparing",
                        _ => "created",
                    };
                    connection.execute(&format!("UPDATE audio_jobs SET status=?2,stage=?3,run_id=run_id+1,error_code=NULL,error_message=NULL,updated_at={NOW} WHERE id=?1"), params![id, status, stage]).map_err(|error| error.to_string())?;
                }
                "cancel" => {
                    if current.status == "succeeded" { return Err("音频已经保存，不能取消。".into()); }
                    if current.status != "cancelled" {
                        connection.execute(&format!("UPDATE audio_jobs SET status='cancelled',run_id=run_id+1,error_code='tracking_stopped',error_message='已停止本地跟进，不等于取消 Google 生成。',updated_at={NOW} WHERE id=?1"), [id]).map_err(|error| error.to_string())?;
                    }
                }
                "open_browser" => {}
                _ => return Err("不支持的音频任务操作。".into()),
            }
            job(connection, id)
        })
    }

    pub fn recover_audio_jobs(&self) -> Result<(), String> {
        self.with_connection(|connection| {
            connection.execute(
                &format!("UPDATE audio_jobs SET status='waiting_user',stage=CASE WHEN stage IN ('opening_site','awaiting_extension','login') AND notebook_url IS NULL AND submitted_at IS NULL THEN 'opening_site' ELSE 'verify_remote' END,run_id=run_id+1,error_code='verify_remote',error_message='应用重启后请查看网站并继续跟进；已开始的远端任务会先核对。',updated_at={NOW} WHERE status IN ('created','waiting_login','preparing','generating','downloading','importing')"),
                [],
            ).map_err(|error| error.to_string())?;
            Ok(())
        })
    }

    #[cfg(any(target_os = "windows", test))]
    pub fn audio_execution_job(&self) -> Result<Option<AudioExecutionJob>, String> {
        self.with_connection(|connection| {
            let Some(current) = active_job(connection)? else {
                return Ok(None);
            };
            let input_json: String = connection
                .query_row(
                    "SELECT input_json FROM audio_jobs WHERE id=?1",
                    [&current.id],
                    |row| row.get(0),
                )
                .map_err(|error| error.to_string())?;
            let input = parse_input(&input_json)?;
            let source = match input.source {
                FrozenSource::Text { text } => AudioSourceData::Text { text },
                FrozenSource::Asset {
                    name, size, sha256, ..
                } => AudioSourceData::Asset { name, size, sha256 },
            };
            Ok(Some(AudioExecutionJob {
                id: current.id,
                run_id: current.run_id,
                status: current.status,
                stage: current.stage,
                project_title: current.project_title,
                prompt: input.prompt,
                target_duration_seconds: input.target_duration_seconds,
                format: input.format,
                notebook_url: current.notebook_url,
                source,
            }))
        })
    }

    #[cfg(any(target_os = "windows", test))]
    pub fn audio_cache_directory(&self, job_id: &str) -> Result<PathBuf, String> {
        self.with_connection(|connection| {
            let current = job(connection, job_id)?;
            let project = get(connection, &current.project_id)?;
            let workspace = PathBuf::from(project.workspace_path);
            checked_directory(&workspace)?;
            let cache = workspace.join(".audio-jobs").join(job_id);
            fs::create_dir_all(&cache).map_err(|e| e.to_string())?;
            let cache = cache.canonicalize().map_err(|e| e.to_string())?;
            if !cache.starts_with(workspace.canonicalize().map_err(|e| e.to_string())?) {
                return Err("音频任务缓存必须位于本书目录。".into());
            }
            Ok(cache)
        })
    }

    #[cfg(any(target_os = "windows", test))]
    pub fn audio_download_path(&self, job_id: &str, run_id: i64) -> Result<PathBuf, String> {
        self.with_connection(|connection| {
            let current = job(connection, job_id)?;
            require_run(&current, run_id)
        })?;
        let directory = self.audio_cache_directory(job_id)?.join(run_id.to_string());
        fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
        Ok(directory.join("download.partial"))
    }

    #[cfg(any(target_os = "windows", test))]
    pub fn audio_source_snapshot(&self, job_id: &str, run_id: i64) -> Result<PathBuf, String> {
        let (path, name, size, sha256) = self.with_connection(|connection| {
            let current = job(connection, job_id)?;
            require_run(&current, run_id)?;
            if current.status != "preparing" || current.stage != "adding_source" {
                return Err("当前任务不能上传来源。".into());
            }
            let input_json: String = connection.query_row("SELECT input_json FROM audio_jobs WHERE id=?1", [job_id], |row| row.get(0)).map_err(|e| e.to_string())?;
            let FrozenSource::Asset { asset_id, name, size, sha256 } = parse_input(&input_json)?.source else {
                return Err("资料文本不使用文件上传。".into());
            };
            let path: String = connection.query_row("SELECT file_path FROM assets WHERE id=?1 AND project_id=?2 AND asset_type='document'", params![asset_id, current.project_id], |row| row.get(0))
                .optional().map_err(|e| e.to_string())?.ok_or("原书来源已从本书移除。")?;
            Ok((PathBuf::from(path), name, size, sha256))
        })?;
        if hash_file(&path)? != (size, sha256.clone()) {
            return Err("原书文件大小或 SHA-256 与任务快照不一致，请停止后重新选择。".into());
        }
        let directory = self
            .audio_cache_directory(job_id)?
            .join(run_id.to_string())
            .join("source");
        fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
        let name = Path::new(&name).file_name().ok_or("原书文件名无效。")?;
        let destination = directory.join(name);
        let copied = copy_verified(&path, &destination)?;
        if copied != (size, sha256) {
            let _ = fs::remove_file(&destination);
            return Err("原书在制作上传快照时发生变化，已停止上传。".into());
        }
        self.with_connection(|connection| require_run(&job(connection, job_id)?, run_id))?;
        Ok(destination)
    }

    #[cfg(any(target_os = "windows", test))]
    pub fn retry_audio_import(
        &self,
        job_id: &str,
        run_id: i64,
    ) -> Result<Option<AudioJob>, String> {
        let current = self.with_connection(|connection| job(connection, job_id))?;
        require_run(&current, run_id)?;
        if current.output_sha256.is_none() || current.asset_id.is_some() {
            return Ok(None);
        }
        let path = self.audio_cache_directory(job_id)?.join("complete.audio");
        if !path.is_file() {
            return Err("已确认的音频缓存缺失，请重新下载原音频。".into());
        }
        if hash_file(&path)?
            != (
                current.output_size.ok_or("音频缓存缺少大小记录。")?,
                current.output_sha256.unwrap(),
            )
        {
            return Err("已确认的音频缓存与 SHA-256 记录不一致。".into());
        }
        self.with_connection(|connection| {
            let latest = job(connection, job_id)?;
            require_run(&latest, run_id)?;
            if !matches!(latest.status.as_str(), "preparing" | "importing") {
                return Err("当前任务不能重试音频归档。".into());
            }
            self.set_job(connection, job_id, "importing", "importing", None, None)
        })?;
        Ok(Some(self.complete_audio_download(
            job_id,
            run_id,
            path.to_str().ok_or("音频缓存路径无效。")?,
        )?))
    }

    #[cfg(target_os = "windows")]
    pub fn audio_checked(&self, id: &str, run_id: i64) -> Result<(), String> {
        self.with_connection(|connection| {
            require_run(&job(connection, id)?, run_id)?;
            self.touch_job(connection, id)
        })
    }

    #[cfg(target_os = "windows")]
    pub fn audio_step_failed(&self, id: &str, run_id: i64, message: &str) -> Result<(), String> {
        self.with_connection(|connection| {
            let current = job(connection, id)?;
            require_run(&current, run_id)?;
            if matches!(
                current.status.as_str(),
                "paused" | "cancelled" | "succeeded" | "failed"
            ) {
                return Ok(());
            }
            let stage = if matches!(
                current.stage.as_str(),
                "creating_notebook" | "adding_source" | "submitting"
            ) {
                "verify_remote"
            } else {
                &current.stage
            };
            self.set_job(
                connection,
                id,
                "waiting_user",
                stage,
                Some("page_action_failed"),
                Some(message),
            )
        })
    }

    fn set_job(
        &self,
        connection: &Connection,
        id: &str,
        status: &str,
        stage: &str,
        error_code: Option<&str>,
        error_message: Option<&str>,
    ) -> Result<(), String> {
        connection.execute(
            &format!("UPDATE audio_jobs SET status=?2,stage=?3,error_code=?4,error_message=?5,last_checked_at={NOW},updated_at={NOW} WHERE id=?1"),
            params![id, status, stage, error_code, error_message],
        ).map_err(|error| error.to_string())?;
        Ok(())
    }

    #[cfg(any(target_os = "windows", test))]
    fn touch_job(&self, connection: &Connection, id: &str) -> Result<(), String> {
        connection
            .execute(
                &format!(
                    "UPDATE audio_jobs SET last_checked_at={NOW},updated_at={NOW} WHERE id=?1"
                ),
                [id],
            )
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    #[cfg(any(target_os = "windows", test))]
    pub fn observe_audio_job(
        &self,
        job_id: &str,
        run_id: i64,
        observation: &AudioObservation,
    ) -> Result<AudioJob, String> {
        self.with_connection(|connection| {
            let current = job(connection, job_id)?;
            require_run(&current, run_id)?;
            if matches!(current.status.as_str(), "paused" | "failed" | "succeeded" | "cancelled") { return Err("音频任务当前不接受页面操作结果。".into()); }
            match observation {
                AudioObservation::LoginRequired => {
                    if current.status == "waiting_login" {
                        self.touch_job(connection, job_id)?;
                        return job(connection, job_id);
                    }
                    if !matches!(current.status.as_str(), "created" | "preparing" | "generating" | "downloading" | "importing") {
                        return Err("当前阶段不能切换为等待登录。".into());
                    }
                    self.set_job(
                        connection,
                        job_id,
                        "waiting_login",
                        &current.stage,
                        Some("login_required"),
                        Some("请在应用内网站完成 Google 登录，登录完成后会自动继续。"),
                    )?;
                }
                AudioObservation::LoginReady => {
                    if current.status == "preparing" {
                        self.touch_job(connection, job_id)?;
                        return job(connection, job_id);
                    }
                    if !matches!(current.status.as_str(), "created" | "waiting_login") {
                        return Err("当前阶段不能恢复 Notebook 任务。".into());
                    }
                    let stage = match current.stage.as_str() {
                        "awaiting_extension" | "opening_site" => "opening_site",
                        "download_pending" => "downloading",
                        "creating_notebook" if current.notebook_url.is_none() => "verify_remote",
                        "submitting" => "verify_remote",
                        other => other,
                    };
                    let status = match stage {
                        "generating" => "generating",
                        "downloading" => "downloading",
                        "importing" => "importing",
                        _ => "preparing",
                    };
                    let stage = if stage == "opening_site" { "creating_notebook" } else { stage };
                    self.set_job(connection, job_id, status, stage, None, None)?;
                }
                AudioObservation::NotebookCreated { notebook_url } => {
                    if !validate_notebook_url(notebook_url) { return Err("Notebook 地址不是受支持的 HTTPS 站点。".into()); }
                    if current.stage == "adding_source" && current.notebook_url.as_deref() == Some(notebook_url) { self.touch_job(connection, job_id)?; return job(connection, job_id); }
                    if current.status != "preparing" || current.stage != "creating_notebook" { return Err("当前阶段不能登记新笔记本。".into()); }
                    connection.execute(
                        &format!("UPDATE audio_jobs SET stage='adding_source',notebook_url=?2,error_code=NULL,error_message=NULL,last_checked_at={NOW},updated_at={NOW} WHERE id=?1"),
                        params![job_id, notebook_url],
                    ).map_err(|error| error.to_string())?;
                }
                AudioObservation::SourceSubmitted => {
                    if current.stage == "source_processing" { self.touch_job(connection, job_id)?; return job(connection, job_id); }
                    if current.status != "preparing" || current.stage != "adding_source" { return Err("当前阶段不能提交来源。".into()); }
                    self.set_job(connection, job_id, "preparing", "source_processing", None, None)?;
                }
                AudioObservation::SourceReady => {
                    if current.stage == "configuring_audio" { self.touch_job(connection, job_id)?; return job(connection, job_id); }
                    if current.status != "preparing" || current.stage != "source_processing" { return Err("当前阶段不能确认来源就绪。".into()); }
                    self.set_job(connection, job_id, "preparing", "configuring_audio", None, None)?;
                }
                AudioObservation::AudioConfigured => {
                    if current.stage == "submitting" { self.touch_job(connection, job_id)?; return job(connection, job_id); }
                    if current.status != "preparing" || current.stage != "configuring_audio" { return Err("当前阶段不能确认音频设置。".into()); }
                    self.set_job(connection, job_id, "preparing", "submitting", None, None)?;
                }
                AudioObservation::GenerationSubmitted => {
                    if current.status == "generating" && current.stage == "generating" { self.touch_job(connection, job_id)?; return job(connection, job_id); }
                    if current.status != "preparing" || current.stage != "submitting" { return Err("当前阶段不能提交音频生成。".into()); }
                    connection.execute(
                        &format!("UPDATE audio_jobs SET status='generating',stage='generating',submitted_at=coalesce(submitted_at,{NOW}),error_code=NULL,error_message=NULL,last_checked_at={NOW},updated_at={NOW} WHERE id=?1"),
                        [job_id],
                    ).map_err(|error| error.to_string())?;
                }
                AudioObservation::Generating => {
                    if !matches!(current.status.as_str(), "preparing" | "generating") || !matches!(current.stage.as_str(), "verify_remote" | "submitting" | "generating") { return Err("当前阶段不能登记生成中状态。".into()); }
                    connection.execute(
                        &format!("UPDATE audio_jobs SET status='generating',stage='generating',submitted_at=coalesce(submitted_at,{NOW}),error_code=NULL,error_message=NULL,last_checked_at={NOW},updated_at={NOW} WHERE id=?1"),
                        [job_id],
                    ).map_err(|error| error.to_string())?;
                }
                AudioObservation::GenerationComplete { .. } => {
                    if current.status == "downloading" && current.stage == "downloading" { self.touch_job(connection, job_id)?; return job(connection, job_id); }
                    if current.status != "generating" && !(current.status == "preparing" && current.stage == "verify_remote") { return Err("当前阶段不能登记音频生成完成。".into()); }
                    self.set_job(connection, job_id, "downloading", "downloading", None, None)?;
                }
                AudioObservation::PageChanged { message } => {
                    self.set_job(connection, job_id, "waiting_user", &current.stage, Some("page_changed"), Some(if message.trim().is_empty() { "页面状态无法确认，请在应用内网站核对后继续。" } else { message.trim() }))?;
                }
            }
            job(connection, job_id)
        })
    }

    #[cfg(any(target_os = "windows", test))]
    pub fn audio_download_started(&self, job_id: &str, run_id: i64) -> Result<AudioJob, String> {
        self.with_connection(|connection| {
            let current = job(connection, job_id)?;
            require_run(&current, run_id)?;
            if current.status != "downloading" { return Err("当前任务还没有进入下载阶段。".into()); }
            connection.execute(&format!("UPDATE audio_jobs SET stage='downloading',last_checked_at={NOW},updated_at={NOW} WHERE id=?1"), [job_id]).map_err(|error| error.to_string())?;
            job(connection, job_id)
        })
    }

    pub fn audio_report_error(
        &self,
        job_id: &str,
        run_id: i64,
        code: &str,
        message: &str,
    ) -> Result<AudioJob, String> {
        self.with_connection(|connection| {
            let current = job(connection, job_id)?;
            require_run(&current, run_id)?;
            if current.status == "succeeded" {
                return Ok(current);
            }
            if current.status == "cancelled" {
                return Err("任务已停止跟进，迟到错误不会改变状态。".into());
            }
            let status = if code.starts_with("site_") || code.starts_with("quota_") {
                "failed"
            } else {
                "waiting_user"
            };
            self.set_job(
                connection,
                job_id,
                status,
                &current.stage,
                Some(code),
                Some(if message.trim().is_empty() {
                    "网页自动化需要人工核对。"
                } else {
                    message.trim()
                }),
            )?;
            job(connection, job_id)
        })
    }

    #[cfg(any(target_os = "windows", test))]
    pub fn complete_audio_download(
        &self,
        job_id: &str,
        run_id: i64,
        path: &str,
    ) -> Result<AudioJob, String> {
        let (current, session_file, project_workspace, title) =
            self.with_connection(|connection| {
                let current = job(connection, job_id)?;
                require_run(&current, run_id)?;
                if current.status == "succeeded" && current.asset_id.is_some() {
                    return Ok((current, PathBuf::new(), PathBuf::new(), String::new()));
                }
                if !matches!(current.status.as_str(), "downloading" | "importing") {
                    return Err("当前任务还没有进入音频归档阶段。".into());
                }
                let project = get(connection, &current.project_id)?;
                let session_file = PathBuf::from(path)
                    .canonicalize()
                    .map_err(|error| format!("下载的音频路径不可用：{error}"))?;
                let cache = PathBuf::from(&project.workspace_path)
                    .join(".audio-jobs")
                    .join(job_id);
                let expected = cache.join(run_id.to_string()).join("download.partial");
                let completed = cache.join("complete.audio");
                let exact_download = expected.canonicalize().ok().as_ref() == Some(&session_file);
                let confirmed_cache = completed.canonicalize().ok().as_ref() == Some(&session_file)
                    && current.output_size.is_some()
                    && current.output_sha256.is_some()
                    && hash_file(&session_file)?
                        == (
                            current.output_size.unwrap(),
                            current.output_sha256.clone().unwrap(),
                        );
                if !exact_download && !confirmed_cache {
                    return Err("下载文件不属于当前音频任务的精确缓存路径。".into());
                }
                if !session_file.is_file() {
                    return Err("下载内容不是文件。".into());
                }
                Ok((
                    current,
                    session_file,
                    PathBuf::from(project.workspace_path),
                    project.input.title,
                ))
            })?;
        if current.status == "succeeded" {
            return Ok(current);
        }
        if let Err(error) = audio_format(&session_file) {
            let _ = self.audio_report_error(job_id, run_id, "import_failed", &error);
            return Err(error);
        }
        let cache_file = self.audio_cache_directory(job_id)?.join("complete.audio");
        let (verified_size, verified_hash) = copy_verified(&session_file, &cache_file)?;
        let session_file = cache_file;
        self.with_connection(|connection| {
            let latest = job(connection, job_id)?;
            require_run(&latest, run_id)?;
            if !matches!(latest.status.as_str(), "downloading" | "importing") {
                return Err("音频任务状态在归档期间发生变化。".into());
            }
            connection
                .execute(
                    "UPDATE audio_jobs SET output_size=?2,output_sha256=?3 WHERE id=?1",
                    params![job_id, verified_size, verified_hash],
                )
                .map_err(|e| e.to_string())?;
            self.set_job(connection, job_id, "importing", "importing", None, None)
        })?;
        let stored = (|| -> Result<(i64, String, PathBuf), String> {
            let extension = audio_format(&session_file)?;
            checked_directory(&project_workspace)?;
            let audio_dir = project_workspace.join("audio");
            fs::create_dir_all(&audio_dir).map_err(|error| format!("无法创建音频目录：{error}"))?;
            let destination = audio_dir.join(format!(
                "{}-音频-{job_id}.{extension}",
                safe_book_name(&title)
            ));
            let (size, sha256) = copy_verified(&session_file, &destination)?;
            Ok((size, sha256, destination))
        })();
        let (size, sha256, destination) = match stored {
            Ok(value) => value,
            Err(error) => {
                let _ = self.audio_report_error(job_id, run_id, "import_failed", &error);
                return Err(error);
            }
        };
        let result = self.with_connection(|connection| {
            let transaction = connection.transaction().map_err(|error| error.to_string())?;
            let latest = job(&transaction, job_id)?;
            require_run(&latest, run_id)?;
            if latest.status == "succeeded" && latest.asset_id.is_some() { return Ok(latest); }
            if !matches!(latest.status.as_str(), "downloading" | "importing") { return Err("音频任务状态在最终归档时发生变化。".into()); }
            let asset_id = assets::register_existing(&transaction, &latest.project_id, &destination, Some("audio"), None)?;
            transaction.execute(
                &format!("UPDATE audio_jobs SET status='succeeded',stage='completed',asset_id=?2,output_size=?3,output_sha256=?4,error_code=NULL,error_message=NULL,last_checked_at={NOW},updated_at={NOW} WHERE id=?1"),
                params![job_id, asset_id, size, sha256],
            ).map_err(|error| error.to_string())?;
            let completed = job(&transaction, job_id)?;
            transaction.commit().map_err(|error| error.to_string())?;
            Ok(completed)
        });
        if result.is_err() {
            let _ = self.audio_report_error(
                job_id,
                run_id,
                "import_failed",
                "音频文件已保留，但素材登记事务失败，可重试归档。",
            );
        }
        result
    }
}

#[cfg(test)]
#[path = "audio_tests.rs"]
mod audio_tests;
