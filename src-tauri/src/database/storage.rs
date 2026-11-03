use super::{projects, Database};
use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupRecord {
    pub kind: String,
    pub path: String,
    pub completed_at: String,
}
pub(super) fn record_backup(c: &Connection, kind: &str, path: &Path) -> Result<(), String> {
    let completed_at = c
        .query_row(&format!("SELECT {}", projects::NOW), [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    let record = BackupRecord {
        kind: kind.into(),
        path: path.to_string_lossy().into_owned(),
        completed_at,
    };
    c.execute("INSERT INTO app_settings(key,value) VALUES('last_successful_backup',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value",[serde_json::to_string(&record).map_err(|e|e.to_string())?]).map_err(|e|e.to_string())?;
    Ok(())
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageUsage {
    pub project_id: String,
    pub title: String,
    pub bytes: u64,
    pub reclaimable_bytes: u64,
    pub files: usize,
    pub blocked: bool,
    pub error: String,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CachePreview {
    pub project_id: String,
    pub token: String,
    pub bytes: u64,
    pub files: Vec<String>,
}
#[derive(Debug, PartialEq, Serialize)]
struct FileStamp {
    path: PathBuf,
    size: u64,
    modified: u128,
}

fn is_link(meta: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        meta.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        meta.file_type().is_symlink()
    }
}
fn walk(root: &Path, files: &mut Vec<FileStamp>) -> Result<(), String> {
    let meta = match fs::symlink_metadata(root) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e.to_string()),
    };
    if is_link(&meta) {
        return Ok(());
    }
    if meta.is_file() {
        files.push(FileStamp {
            path: root.to_path_buf(),
            size: meta.len(),
            modified: meta
                .modified()
                .map_err(|e| e.to_string())?
                .duration_since(UNIX_EPOCH)
                .map_err(|e| e.to_string())?
                .as_nanos(),
        });
    } else if meta.is_dir() {
        for entry in fs::read_dir(root).map_err(|e| e.to_string())? {
            walk(&entry.map_err(|e| e.to_string())?.path(), files)?;
        }
    }
    Ok(())
}
fn running(c: &Connection, id: &str) -> Result<bool, String> {
    c.query_row(
        "SELECT EXISTS(SELECT 1 FROM video_jobs WHERE project_id=?1 AND status='running')",
        [id],
        |r| r.get(0),
    )
    .map_err(|e| e.to_string())
}
fn snapshot(c: &Connection, id: &str) -> Result<(PathBuf, Vec<FileStamp>, String), String> {
    if running(c, id)? {
        return Err("该书的视频任务正在运行，请停止或完成后再清理。".into());
    }
    let book = projects::get(c, id)?;
    let root = PathBuf::from(book.workspace_path);
    if is_link(&fs::symlink_metadata(&root).map_err(|e| e.to_string())?) {
        return Err("工作目录是链接，不能自动清理。".into());
    }
    let root = fs::canonicalize(root).map_err(|e| e.to_string())?;
    let mut q=c.prepare("SELECT file_path FROM assets UNION SELECT exported_path FROM cover_versions WHERE exported_path IS NOT NULL").map_err(|e|e.to_string())?;
    let protected = q
        .query_map([], |r| r.get::<_, String>(0))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    let protected: BTreeSet<PathBuf> = protected
        .into_iter()
        .filter_map(|p| fs::canonicalize(p).ok())
        .collect();
    let mut q = c
        .prepare("SELECT task_directory FROM publish_tasks WHERE task_directory IS NOT NULL")
        .map_err(|e| e.to_string())?;
    let packages = q
        .query_map([], |r| r.get::<_, String>(0))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    let packages: Vec<_> = packages
        .into_iter()
        .filter_map(|p| fs::canonicalize(p).ok())
        .collect();
    let mut q = c
        .prepare("SELECT id FROM video_jobs WHERE project_id=?1")
        .map_err(|e| e.to_string())?;
    let jobs = q
        .query_map([id], |r| r.get::<_, String>(0))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    let mut all = Vec::new();
    walk(&root, &mut all)?;
    let files: Vec<_> = all
        .into_iter()
        .filter(|f| {
            if protected.contains(&f.path) || packages.iter().any(|p| f.path.starts_with(p)) {
                return false;
            }
            let Ok(relative) = f.path.strip_prefix(root.join("video")) else {
                return false;
            };
            let parts: Vec<_> = relative.components().collect();
            if parts.len() < 3 || !jobs.iter().any(|id| parts[0].as_os_str() == id.as_str()) {
                return false;
            }
            let folder = parts[1].as_os_str().to_string_lossy();
            folder == "cache"
                || folder
                    .strip_prefix("preview-")
                    .is_some_and(|s| s.parse::<u64>().is_ok())
        })
        .collect();
    let mut files = files;
    files.sort_by(|a, b| a.path.cmp(&b.path));
    let token = Sha256::digest(serde_json::to_vec(&files).map_err(|e| e.to_string())?)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    Ok((root, files, token))
}
impl Database {
    pub fn last_backup(&self) -> Result<Option<BackupRecord>, String> {
        self.with_connection(|c| {
            let raw: Option<String> = c
                .query_row(
                    "SELECT value FROM app_settings WHERE key='last_successful_backup'",
                    [],
                    |r| r.get(0),
                )
                .optional()
                .map_err(|e| e.to_string())?;
            raw.map(|s| serde_json::from_str(&s).map_err(|e| e.to_string()))
                .transpose()
        })
    }
    pub fn storage_usage(&self) -> Result<Vec<StorageUsage>, String> {
        self.with_connection(|c| {
            let mut q = c
                .prepare("SELECT id,title,workspace_path FROM book_projects ORDER BY title,id")
                .map_err(|e| e.to_string())?;
            let books = q
                .query_map([], |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                    ))
                })
                .map_err(|e| e.to_string())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?;
            Ok(books
                .into_iter()
                .map(|(id, title, path)| {
                    let mut all = Vec::new();
                    let error = walk(Path::new(&path), &mut all).err().unwrap_or_default();
                    let mut usage = StorageUsage {
                        project_id: id.clone(),
                        title,
                        bytes: all.iter().map(|f| f.size).sum(),
                        reclaimable_bytes: 0,
                        files: 0,
                        blocked: running(c, &id).unwrap_or(true),
                        error,
                    };
                    match snapshot(c, &id) {
                        Ok((_, files, _)) => {
                            usage.reclaimable_bytes = files.iter().map(|f| f.size).sum();
                            usage.files = files.len();
                        }
                        Err(e) => usage.error = e,
                    }
                    usage
                })
                .collect())
        })
    }
    pub fn preview_cache_cleanup(&self, id: &str) -> Result<CachePreview, String> {
        self.with_connection(|c| {
            let (root, files, token) = snapshot(c, id)?;
            Ok(CachePreview {
                project_id: id.into(),
                token,
                bytes: files.iter().map(|f| f.size).sum(),
                files: files
                    .iter()
                    .map(|f| {
                        f.path
                            .strip_prefix(&root)
                            .unwrap()
                            .to_string_lossy()
                            .into_owned()
                    })
                    .collect(),
            })
        })
    }
    pub fn clean_video_cache(&self, id: &str, token: &str, confirmed: bool) -> Result<u64, String> {
        if !confirmed {
            return Err("清理前必须预览并确认。".into());
        }
        self.with_connection(|c| {
            let (root, files, current) = snapshot(c, id)?;
            if current != token {
                return Err("缓存或素材引用已变化，请重新预览后确认。".into());
            }
            let mut bytes = 0;
            for file in files {
                // Revalidate each resolved target under the exact workspace. Never recurse into junctions.
                if !file.path.starts_with(&root)
                    || fs::canonicalize(&file.path).map_err(|e| e.to_string())? != file.path
                {
                    return Err("缓存路径已变化，已停止清理。".into());
                }
                let mut parent = file.path.parent();
                while let Some(path) = parent {
                    if path == root {
                        break;
                    }
                    if is_link(&fs::symlink_metadata(path).map_err(|e| e.to_string())?) {
                        return Err("缓存目录变为链接，已停止清理。".into());
                    }
                    parent = path.parent();
                }
                fs::remove_file(&file.path)
                    .map_err(|e| format!("已清理{bytes}字节，部分缓存删除失败：{e}"))?;
                bytes += file.size;
            }
            Ok(bytes)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::video::VideoWork;
    fn put(path: &Path, bytes: &[u8]) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }
    #[test]
    fn cache_cleanup_requires_current_preview_and_keeps_registered_files_and_run_outputs() {
        let (_dir, db, input) = crate::database::video::tests::fixture();
        let project = input.project_id.clone();
        let job = db.create_video_job(input).unwrap();
        let root = PathBuf::from(db.get_project(&project).unwrap().workspace_path);
        let base = root.join("video").join(&job.id);
        let cache = base.join("cache/key/chunk.mp4");
        put(&cache, b"cached frames");
        let preview = base.join("preview-1/video.mp4");
        put(&preview, b"preview");
        let source = base.join("cache/key/registered.srt");
        put(&source, b"registered");
        db.link_asset(&project, source.to_str().unwrap(), None)
            .unwrap();
        let run = base.join("run-1-123/video.mp4");
        put(&run, b"finished video");
        let old = db.preview_cache_cleanup(&project).unwrap();
        assert_eq!(old.files.len(), 2);
        assert!(db.clean_video_cache(&project, &old.token, false).is_err());
        put(&cache, b"updated cached frames");
        assert!(db.clean_video_cache(&project, &old.token, true).is_err());
        let current = db.preview_cache_cleanup(&project).unwrap();
        db.claim_video_job(&job.id, VideoWork::Prepare, None)
            .unwrap();
        assert!(db
            .clean_video_cache(&project, &current.token, true)
            .is_err());
        db.end_video_job(&job.id, 1, "failed", "stop").unwrap();
        assert_eq!(
            db.clean_video_cache(&project, &current.token, true)
                .unwrap(),
            current.bytes
        );
        assert!(!cache.exists() && !preview.exists());
        assert!(source.exists() && run.exists());
    }
    #[cfg(windows)]
    #[test]
    fn cache_scan_does_not_follow_external_junctions_or_remove_their_targets() {
        use std::os::windows::fs::symlink_dir;
        let (dir, db, input) = crate::database::video::tests::fixture();
        let project = input.project_id.clone();
        let job = db.create_video_job(input).unwrap();
        let cache = PathBuf::from(db.get_project(&project).unwrap().workspace_path)
            .join("video")
            .join(job.id)
            .join("cache");
        let external = dir.path().join("outside");
        put(&external.join("keep.mp4"), b"keep");
        fs::create_dir_all(&cache).unwrap();
        // Directory symlinks require developer mode; junctions work without elevation.
        let link = cache.join("engine");
        if symlink_dir(&external, &link).is_err() {
            let result=std::process::Command::new("powershell").args(["-NoProfile","-Command","New-Item -ItemType Junction -Path $env:SHUYIN_TEST_LINK -Target $env:SHUYIN_TEST_TARGET | Out-Null"]).env("SHUYIN_TEST_LINK",&link).env("SHUYIN_TEST_TARGET",&external).output().unwrap();
            assert!(result.status.success());
        }
        put(&cache.join("own.bin"), b"delete");
        let preview = db.preview_cache_cleanup(&project).unwrap();
        assert_eq!(preview.files.len(), 1);
        db.clean_video_cache(&project, &preview.token, true)
            .unwrap();
        assert_eq!(fs::read(external.join("keep.mp4")).unwrap(), b"keep");
        fs::remove_dir(link).unwrap();
    }
    #[test]
    fn successful_backup_record_survives_reopen_and_failed_backup_keeps_the_last_success() {
        let (dir, db, _) = crate::database::video::tests::fixture();
        assert!(db.last_backup().unwrap().is_none());
        let path = db.create_backup(dir.path().to_str().unwrap()).unwrap();
        let record = db.last_backup().unwrap().unwrap();
        assert_eq!(record.path, path);
        assert!(db
            .create_backup(dir.path().join("missing").to_str().unwrap())
            .is_err());
        assert_eq!(
            db.last_backup().unwrap().unwrap().completed_at,
            record.completed_at
        );
        drop(db);
        let reopened = Database::open(dir.path().join("data.sqlite3"));
        assert_eq!(reopened.last_backup().unwrap().unwrap().path, path);
    }
}
