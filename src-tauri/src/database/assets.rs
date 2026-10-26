use super::{
    projects::{get, new_id, NOW},
    Database,
};
use rusqlite::{params, OptionalExtension};
use serde::Serialize;
use std::{path::Path, time::UNIX_EPOCH};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Asset {
    pub id: String,
    pub project_id: String,
    pub asset_type: String,
    pub file_path: String,
    pub original_name: String,
    pub file_size: i64,
    pub modified_at: i64,
    pub exists_status: String,
    pub created_at: String,
}
fn inspect(path: &Path) -> Result<(i64, i64), String> {
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
fn asset_type(path: &Path) -> Result<&'static str, String> {
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
#[cfg(target_os = "windows")]
fn shell_path(path: &Path) -> String {
    let path = path.to_string_lossy();
    if let Some(unc) = path.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{unc}")
    } else {
        path.strip_prefix(r"\\?\").unwrap_or(&path).to_owned()
    }
}

pub(super) fn open_path(path: &Path) -> Result<(), String> {
    let path = std::fs::canonicalize(path).map_err(|e| format!("路径不可用：{e}"))?;
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        let mut command = std::process::Command::new("explorer.exe");
        command
            .arg(shell_path(&path))
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
impl Database {
    pub fn list_assets(&self, project_id: &str) -> Result<Vec<Asset>, String> {
        self.with_connection(|c| {
            get(c,project_id)?;
            let mut query = c.prepare("SELECT id,project_id,asset_type,file_path,original_name,file_size,modified_at,created_at
                FROM assets
                WHERE project_id=?1
                ORDER BY created_at DESC,id").map_err(|e|e.to_string())?;
            let rows = query.query_map([project_id], |r| Ok(Asset {
                id: r.get(0)?,
                project_id: r.get(1)?,
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
    ) -> Result<String, String> {
        let path = std::fs::canonicalize(path).map_err(|e| format!("素材文件不可用：{e}"))?;
        let kind = asset_type(&path)?;
        let (size, modified) = inspect(&path)?;
        self.with_connection(|c| {
            get(c,project_id)?;
            let id = if let Some(id)=asset_id {
                let original: Option<String> = c.query_row("SELECT asset_type
                FROM assets
                WHERE id=?1 AND project_id=?2",params![id,project_id],|r|r.get(0)).optional().map_err(|e|e.to_string())?;
                let original=original.ok_or("素材关联不存在。")?;
                if original!=kind { return Err("重新定位的文件必须与原素材类型相同。".into()); }
                id.to_owned()
            } else { new_id(c)? };
            c.execute(&format!("INSERT INTO assets(id,project_id,asset_type,file_path,original_name,file_size,modified_at,created_at) VALUES(?1,?2,?3,?4,?5,?6,?7,{NOW})
                ON CONFLICT(id) DO UPDATE SET
                file_path=excluded.file_path,original_name=excluded.original_name,file_size=excluded.file_size,modified_at=excluded.modified_at"),params![id,project_id,kind,path.to_string_lossy().as_ref(),path.file_name().unwrap().to_string_lossy().as_ref(),size,modified]).map_err(|e|match e { rusqlite::Error::SqliteFailure(ref code,_) if code.code==rusqlite::ErrorCode::ConstraintViolation => "该项目已关联此文件。".to_owned(), _=>e.to_string() })?;
            Ok(id)
        })
    }
    pub fn unlink_asset(&self, id: &str) -> Result<(), String> {
        self.with_connection(|c| {
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

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::*;
    #[test]
    fn shell_paths_preserve_unicode_spaces_and_network_roots() {
        assert_eq!(
            shell_path(Path::new(r"\\?\D:\书籍 素材\视频.mp4")),
            r"D:\书籍 素材\视频.mp4"
        );
        assert_eq!(
            shell_path(Path::new(r"\\?\UNC\server\share\视频.mp4")),
            r"\\server\share\视频.mp4"
        );
    }
}
