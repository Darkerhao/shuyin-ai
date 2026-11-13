use super::{
    projects::{get, new_id, NOW},
    Database,
};
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use std::{path::Path, time::UNIX_EPOCH};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Asset {
    pub id: String,
    pub project_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chapter_id: Option<String>,
    pub asset_type: String,
    pub file_path: String,
    pub original_name: String,
    pub file_size: i64,
    pub modified_at: i64,
    pub exists_status: String,
    pub created_at: String,
}
pub(super) fn inspect(path: &Path) -> Result<(i64, i64), String> {
    let metadata = std::fs::metadata(path).map_err(|e| format!("素材文件不可用：{e}"))?;
    if !metadata.is_file() {
        return Err("请选择文件。".into());
    }
    let modified = metadata
        .modified()
        .map_err(|e| e.to_string())?
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs();
    Ok((metadata.len() as i64, modified as i64))
}
pub(super) fn asset_type(path: &Path) -> Result<&'static str, String> {
    match path
        .extension()
        .and_then(|x| x.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "mp4" | "mov" | "mkv" | "avi" | "webm" | "m4v" => Ok("video"),
        "mp3" | "m4a" | "wav" | "aac" | "flac" | "ogg" => Ok("audio"),
        "png" | "jpg" | "jpeg" | "webp" => Ok("image"),
        "srt" | "ass" | "vtt" => Ok("subtitle"),
        "txt" | "md" | "pdf" | "docx" | "epub" => Ok("document"),
        _ => Err("暂不支持此素材格式。请选择视频、音频、图片、字幕或文档。".into()),
    }
}
pub(super) fn open_path(path: &Path) -> Result<(), String> {
    let path = std::fs::canonicalize(path).map_err(|e| format!("路径不可用：{e}"))?;
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        let mut command = std::process::Command::new("explorer.exe");
        command
            .arg(crate::paths::shell_path(&path))
            .creation_flags(0x08000000)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}
pub(super) fn register_existing(
    connection: &Connection,
    project_id: &str,
    path: &Path,
    expected_kind: Option<&str>,
    existing_id: Option<&str>,
    chapter_id: Option<&str>,
) -> Result<String, String> {
    get(connection, project_id)?;
    let scope = if let Some(id) = existing_id {
        let owner: Option<String> = connection
            .query_row(
                "SELECT chapter_id FROM assets WHERE id=?1 AND project_id=?2",
                params![id, project_id],
                |row| row.get(0),
            )
            .map_err(|_| "素材关联不存在。")?;
        if chapter_id.is_some() && owner.as_deref() != chapter_id {
            return Err("素材不属于本次章节。".into());
        }
        owner
    } else {
        chapter_id.map(str::to_owned)
    };
    super::chapters::require_scope(connection, project_id, scope.as_deref())?;
    let path = std::fs::canonicalize(path).map_err(|e| format!("素材文件不可用：{e}"))?;
    let kind = asset_type(&path)?;
    if expected_kind.is_some_and(|value| value != kind) {
        return Err("重新定位的文件必须与原素材类型相同。".into());
    }
    let (size, modified) = inspect(&path)?;
    let original_name = path
        .file_name()
        .map(|value| value.to_string_lossy().into_owned())
        .ok_or("素材文件名无效。")?;
    let id = if let Some(id) = existing_id {
        let original: Option<String> = connection
            .query_row(
                "SELECT asset_type FROM assets WHERE id=?1 AND project_id=?2",
                params![id, project_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        let original = original.ok_or("素材关联不存在。")?;
        if original != kind {
            return Err("重新定位的文件必须与原素材类型相同。".into());
        }
        id.to_owned()
    } else {
        let existing: Option<(String, String)> = connection
            .query_row(
                "SELECT id,asset_type FROM assets WHERE project_id=?1 AND file_path=?2",
                params![project_id, path.to_string_lossy().as_ref()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        if let Some((_id, existing_kind)) = existing {
            if existing_kind != kind {
                return Err("该项目已关联同路径但不同类型的素材。".into());
            }
            return Err("该项目已关联此文件。".into());
        }
        new_id(connection)?
    };
    connection.execute(
        &format!("INSERT INTO assets(id,project_id,asset_type,file_path,original_name,file_size,modified_at,created_at,chapter_id) VALUES(?1,?2,?3,?4,?5,?6,?7,{NOW},?8) ON CONFLICT(id) DO UPDATE SET file_path=excluded.file_path,original_name=excluded.original_name,file_size=excluded.file_size,modified_at=excluded.modified_at"),
        params![id, project_id, kind, path.to_string_lossy().as_ref(), original_name, size, modified,scope],
    ).map_err(|error| match error {
        rusqlite::Error::SqliteFailure(ref code, _) if code.code == rusqlite::ErrorCode::ConstraintViolation => "该项目已关联此文件。".to_owned(),
        _ => error.to_string(),
    })?;
    Ok(id)
}

impl Database {
    pub fn media_asset_path(
        &self,
        project_id: &str,
        asset_id: &str,
    ) -> Result<std::path::PathBuf, String> {
        let kind: String = self.with_connection(|connection| {
            connection.query_row(
                "SELECT asset_type FROM assets WHERE id=?1 AND project_id=?2 AND asset_type IN ('audio','video')",
                params![asset_id, project_id], |row| row.get(0),
            ).map_err(|_| "只能预览本书已登记的音频或视频。".into())
        })?;
        self.video_asset_path(project_id, asset_id, &kind)
    }

    pub fn video_asset_metadata(
        &self,
        project: &str,
        id: &str,
    ) -> Result<crate::media::VideoMetadata, String> {
        crate::media::inspect(&self.video_asset_path(project, id, "video")?)
    }
    pub fn export_video_asset(
        &self,
        project: &str,
        id: &str,
        destination: &str,
    ) -> Result<(), String> {
        use std::{
            fs::{File, OpenOptions},
            io,
        };
        let source = self.video_asset_path(project, id, "video")?;
        let source = source.as_path();
        let metadata = crate::media::inspect(source)?;
        if metadata.duration_seconds <= 0.0
            || !metadata.has_audio
            || metadata.width == 0
            || metadata.height == 0
        {
            return Err("成片缺少有效画面或原声。".into());
        }
        let destination = Path::new(destination);
        if !destination.is_absolute()
            || !source
                .extension()
                .and_then(|s| s.to_str())
                .zip(destination.extension().and_then(|s| s.to_str()))
                .is_some_and(|(source, target)| source.eq_ignore_ascii_case(target))
        {
            return Err("请选择与原成片相同格式的完整保存路径。".into());
        }
        let mut input = File::open(source).map_err(|e| e.to_string())?;
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(destination)
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::AlreadyExists {
                    "目标文件已存在，请选择新的文件名；不会覆盖原文件。".to_owned()
                } else {
                    e.to_string()
                }
            })?;
        if let Err(error) = io::copy(&mut input, &mut output).and_then(|_| output.sync_all()) {
            drop(output);
            let _ = std::fs::remove_file(destination);
            return Err(error.to_string());
        }
        Ok(())
    }

    pub fn list_assets(
        &self,
        project_id: &str,
        chapter_id: Option<&str>,
    ) -> Result<Vec<Asset>, String> {
        self.with_connection(|c| {
            super::chapters::require_scope(c,project_id,chapter_id)?;
            let mut query = c.prepare("SELECT id,project_id,asset_type,file_path,original_name,file_size,modified_at,created_at,chapter_id
                FROM assets
                WHERE project_id=?1 AND (chapter_id IS ?2 OR (?2 IS NOT NULL AND chapter_id IS NULL AND asset_type IN ('image','document')))
                ORDER BY created_at DESC,id").map_err(|e|e.to_string())?;
            let rows = query.query_map(params![project_id,chapter_id], |r| Ok(Asset {
                id: r.get(0)?,
                project_id: r.get(1)?,
                chapter_id: r.get(8)?,
                asset_type: r.get(2)?,
                file_path: r.get(3)?,
                original_name: r.get(4)?,
                file_size: r.get(5)?,
                modified_at: r.get(6)?,
                created_at: r.get(7)?,
                exists_status: String::new(),
            })).map_err(|e|e.to_string())?;
            let mut assets = rows.collect::<rusqlite::Result<Vec<_>>>().map_err(|e|e.to_string())?;
            for asset in &mut assets {
                asset.exists_status = match std::fs::metadata(&asset.file_path) {
                    Ok(metadata) if metadata.is_file() => {
                        asset.file_size = metadata.len() as i64;
                        if let Ok(modified) = metadata.modified() {
                            if let Ok(elapsed) = modified.duration_since(UNIX_EPOCH) {
                                asset.modified_at = elapsed.as_secs() as i64;
                            }
                        }
                        "available"
                    },
                    Ok(_) => "missing",
                    Err(e) if e.kind()==std::io::ErrorKind::NotFound => "missing",
                    Err(_) => "unknown",
                }.into();
            }
            Ok(assets)
        })
    }
    pub fn link_asset(
        &self,
        project_id: &str,
        path: &str,
        asset_id: Option<&str>,
        chapter_id: Option<&str>,
    ) -> Result<String, String> {
        self.with_connection(|connection| {
            if let Some(id) = asset_id {
                if connection.query_row("SELECT EXISTS(SELECT 1 FROM video_jobs WHERE status='running' AND (audio_asset_id=?1 OR source_subtitle_id=?1 OR EXISTS(SELECT 1 FROM json_each(plan_json,'$.scenes') WHERE json_extract(value,'$.image.assetId')=?1)))", [id], |r| r.get::<_, bool>(0)).map_err(|e| e.to_string())? {
                    return Err("素材正在用于视频生成，请先停止任务再重新定位。".into());
                }
            }
            register_existing(connection, project_id, Path::new(path), None, asset_id, chapter_id)
        })
    }
    pub fn unlink_asset(&self, id: &str) -> Result<(), String> {
        self.with_connection(|c| {
            if c.query_row("SELECT EXISTS(SELECT 1 FROM video_jobs WHERE status='running' AND (audio_asset_id=?1 OR source_subtitle_id=?1 OR EXISTS(SELECT 1 FROM json_each(plan_json,'$.scenes') WHERE json_extract(value,'$.image.assetId')=?1)))", [id], |r| r.get::<_, bool>(0)).map_err(|e| e.to_string())? {
                return Err("素材正在用于视频生成，请先停止任务。".into());
            }
            if c.execute(
                "DELETE
                FROM assets
                WHERE id=?1",
                [id],
            )
            .map_err(|e| e.to_string())?
                == 0
            {
                return Err("素材关联不存在。".into());
            }
            Ok(())
        })
    }
    pub fn open_asset(&self, id: &str, directory: bool) -> Result<(), String> {
        let path: String = self.with_connection(|c| {
            c.query_row(
                "SELECT file_path
                FROM assets
                WHERE id=?1",
                [id],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())
        })?;
        // Recheck extension: never allow a crafted backup to launch an executable.
        asset_type(Path::new(&path))?;
        let path = Path::new(&path);
        if directory {
            open_path(path.parent().ok_or("素材目录无效。")?)
        } else {
            inspect(path)?;
            open_path(path)
        }
    }
}
