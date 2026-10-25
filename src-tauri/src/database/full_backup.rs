use super::{
    backup::read_snapshot,
    projects::{create_named_directory, new_id},
    publishing, Database,
};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Component, Path, PathBuf},
};

const PACKAGE_FILES: [&str; 8] = [
    "manifest.json",
    "title.txt",
    "description.txt",
    "tags.txt",
    "video-path.txt",
    "README.txt",
    "cover-landscape.png",
    "cover-portrait.png",
];

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BackupFile {
    original_path: String,
    relative_path: String,
    size: u64,
    sha256: String,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Manifest {
    format: String,
    schema_version: u32,
    include_videos: bool,
    omitted_videos: u64,
    database_sha256: String,
    files: Vec<BackupFile>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupSummary {
    pub manifest_path: String,
    pub file_count: usize,
    pub total_bytes: u64,
    pub omitted_videos: u64,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreResult {
    pub previous_database: String,
    pub workspace_path: String,
}

// Read in bounded chunks so optionally including large videos never loads them into memory.
fn transfer(source: &Path, destination: Option<&Path>) -> Result<(u64, String), String> {
    let mut input =
        File::open(source).map_err(|e| format!("备份文件不可读：{}：{e}", source.display()))?;
    let before = input.metadata().map_err(|e| e.to_string())?;
    if !before.is_file() {
        return Err(format!("不是文件：{}", source.display()));
    }
    let mut output = destination
        .map(|p| OpenOptions::new().write(true).create_new(true).open(p))
        .transpose()
        .map_err(|e| e.to_string())?;
    let mut digest = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    let mut size = 0;
    loop {
        let count = input.read(&mut buffer).map_err(|e| e.to_string())?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
        size += count as u64;
        if let Some(file) = output.as_mut() {
            file.write_all(&buffer[..count])
                .map_err(|e| e.to_string())?;
        }
    }
    if let Some(file) = output {
        file.sync_all().map_err(|e| e.to_string())?;
    }
    let after = input.metadata().map_err(|e| e.to_string())?;
    if size != before.len()
        || before.len() != after.len()
        || before.modified().ok() != after.modified().ok()
    {
        return Err(format!(
            "备份期间文件发生变化，请重试：{}",
            source.display()
        ));
    }
    Ok((
        size,
        digest
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
    ))
}
fn pairs(c: &Connection, sql: &str) -> Result<Vec<(String, String)>, String> {
    let mut q = c.prepare(sql).map_err(|e| e.to_string())?;
    let rows = q
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .map_err(|e| e.to_string())?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| e.to_string())
}
fn package_reference(directory: &str, name: &str) -> String {
    // References retain the source system's separators even when restored on another OS.
    let separator = if directory.contains('\\') { '\\' } else { '/' };
    format!(
        "{}{separator}{name}",
        directory.trim_end_matches(['/', '\\'])
    )
}
fn sources(c: &Connection, include_videos: bool) -> Result<BTreeSet<String>, String> {
    let mut paths = BTreeSet::new();
    for (path, kind) in pairs(c, "SELECT file_path,asset_type FROM assets")? {
        if include_videos || kind != "video" {
            paths.insert(path);
        }
    }
    for (_, path) in pairs(
        c,
        "SELECT id,exported_path FROM cover_versions WHERE exported_path IS NOT NULL",
    )? {
        paths.insert(path);
    }
    for (_, directory) in pairs(
        c,
        "SELECT id,task_directory FROM publish_tasks WHERE task_directory IS NOT NULL",
    )? {
        for name in PACKAGE_FILES {
            paths.insert(package_reference(&directory, name));
        }
    }
    Ok(paths)
}
fn summary(path: &Path, m: &Manifest) -> BackupSummary {
    BackupSummary {
        manifest_path: path.to_string_lossy().into_owned(),
        file_count: m.files.len(),
        total_bytes: m.files.iter().map(|f| f.size).sum(),
        omitted_videos: m.omitted_videos,
    }
}
fn package_file(root: &Path, relative: &str) -> Result<PathBuf, String> {
    let parts: Vec<_> = Path::new(relative).components().collect();
    if parts.len() != 2
        || parts[0] != Component::Normal("files".as_ref())
        || !matches!(parts[1], Component::Normal(_))
        || relative.contains('\\')
    {
        return Err("备份文件路径无效。".into());
    }
    let path = fs::canonicalize(root.join(relative)).map_err(|e| e.to_string())?;
    if !path.starts_with(root) {
        return Err("备份文件不能指向备份目录之外。".into());
    }
    Ok(path)
}
fn read_package(path: &str) -> Result<(PathBuf, Manifest, Connection), String> {
    let path = fs::canonicalize(path).map_err(|e| e.to_string())?;
    let root = path.parent().ok_or("备份位置无效。")?.to_owned();
    let mut bytes = Vec::new();
    File::open(&path)
        .map_err(|e| e.to_string())?
        .take(8 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 8 * 1024 * 1024 {
        return Err("备份清单过大。".into());
    }
    let m: Manifest = serde_json::from_slice(&bytes).map_err(|_| "不是有效的书引 AI 完整备份。")?;
    if m.format != "shuyin-full-backup" || m.schema_version != 1 {
        return Err("完整备份格式或版本不受支持。".into());
    }
    let db_path = fs::canonicalize(root.join("database.sqlite3")).map_err(|e| e.to_string())?;
    if !db_path.starts_with(&root) || transfer(&db_path, None)?.1 != m.database_sha256 {
        return Err("备份数据库校验失败。".into());
    }
    let snapshot = read_snapshot(&db_path)?;
    let mut originals = BTreeSet::new();
    let mut relative = BTreeSet::new();
    for file in &m.files {
        if !originals.insert(file.original_path.clone())
            || !relative.insert(file.relative_path.clone())
        {
            return Err("备份清单包含重复文件。".into());
        }
        let source = package_file(&root, &file.relative_path)?;
        if transfer(&source, None)? != (file.size, file.sha256.clone()) {
            return Err(format!("备份文件校验失败：{}", file.relative_path));
        }
    }
    if originals != sources(&snapshot, m.include_videos)? {
        return Err("备份清单与数据库文件引用不一致。".into());
    }
    Ok((root, m, snapshot))
}
fn cleanup_created(path: &Path, parent: &Path) {
    // The only recursive cleanup targets are newly created direct children of the chosen directory.
    if let Ok(resolved) = fs::canonicalize(path) {
        if resolved.parent() == Some(parent) {
            let _ = fs::remove_dir_all(resolved);
        }
    }
}

impl Database {
    pub fn create_full_backup(
        &self,
        directory: &str,
        include_videos: bool,
    ) -> Result<BackupSummary, String> {
        let parent = fs::canonicalize(directory).map_err(|e| e.to_string())?;
        if !parent.is_dir() {
            return Err("请选择备份目录。".into());
        }
        self.with_connection(|c| {
            let root = parent.join(format!("shuyin-full-backup-{}", new_id(c)?));
            fs::create_dir(&root).map_err(|e| e.to_string())?;
            let result = (|| {
                let db_path = root.join("database.sqlite3");
                c.execute("VACUUM INTO ?1", [db_path.to_string_lossy().as_ref()])
                    .map_err(|e| e.to_string())?;
                let snapshot = read_snapshot(&db_path)?;
                fs::create_dir(root.join("files")).map_err(|e| e.to_string())?;
                let mut files = Vec::new();
                for (index, original_path) in
                    sources(&snapshot, include_videos)?.into_iter().enumerate()
                {
                    let source = Path::new(&original_path);
                    let ext = source
                        .extension()
                        .and_then(|e| e.to_str())
                        .filter(|e| e.len() <= 12 && e.bytes().all(|b| b.is_ascii_alphanumeric()))
                        .unwrap_or("bin");
                    let relative_path = format!("files/{index}.{ext}");
                    let (size, sha256) = transfer(source, Some(&root.join(&relative_path)))?;
                    files.push(BackupFile {
                        original_path,
                        relative_path,
                        size,
                        sha256,
                    });
                }
                let omitted_videos = if include_videos {
                    0
                } else {
                    c.query_row(
                        "SELECT count(*) FROM assets WHERE asset_type='video'",
                        [],
                        |r| r.get::<_, u32>(0).map(u64::from),
                    )
                    .map_err(|e| e.to_string())?
                };
                let m = Manifest {
                    format: "shuyin-full-backup".into(),
                    schema_version: 1,
                    include_videos,
                    omitted_videos,
                    database_sha256: transfer(&db_path, None)?.1,
                    files,
                };
                let path = root.join("manifest.json");
                let mut file = OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&path)
                    .map_err(|e| e.to_string())?;
                file.write_all(&serde_json::to_vec_pretty(&m).map_err(|e| e.to_string())?)
                    .and_then(|_| file.sync_all())
                    .map_err(|e| e.to_string())?;
                Ok(summary(&path, &m))
            })();
            if result.is_err() {
                cleanup_created(&root, &parent);
            }
            result
        })
    }
    pub fn inspect_full_backup(&self, path: &str) -> Result<BackupSummary, String> {
        let (_, m, _) = read_package(path)?;
        Ok(summary(Path::new(path), &m))
    }
    pub fn restore_full_backup(
        &self,
        path: &str,
        directory: &str,
    ) -> Result<RestoreResult, String> {
        let (source, m, snapshot) = read_package(path)?;
        let parent = fs::canonicalize(directory).map_err(|e| e.to_string())?;
        if !parent.is_dir() {
            return Err("请选择恢复目录。".into());
        }
        let root = parent.join(format!("shuyin-restored-{}", new_id(&snapshot)?));
        fs::create_dir(&root).map_err(|e| e.to_string())?;
        let result = (|| {
            fs::create_dir(root.join("files")).map_err(|e| e.to_string())?;
            let mut mapping = BTreeMap::new();
            for file in &m.files {
                mapping.insert(file.original_path.clone(), root.join(&file.relative_path));
            }
            let tasks = pairs(
                &snapshot,
                "SELECT id,task_directory FROM publish_tasks WHERE task_directory IS NOT NULL",
            )?;
            for (id, old) in &tasks {
                let title: String = snapshot
                    .query_row(
                        "SELECT title FROM book_projects WHERE id=(SELECT project_id FROM publish_tasks WHERE id=?1)",
                        [id],
                        |row| row.get(0),
                    )
                    .map_err(|e| e.to_string())?;
                let target =
                    create_named_directory(&root.join("packages"), &format!("{title}-发布任务包"))?;
                for name in PACKAGE_FILES {
                    mapping.insert(package_reference(old, name), target.join(name));
                }
                snapshot
                    .execute(
                        "UPDATE publish_tasks SET task_directory=?1 WHERE id=?2",
                        params![target.to_string_lossy(), id],
                    )
                    .map_err(|e| e.to_string())?;
            }
            for file in &m.files {
                let origin = package_file(&source, &file.relative_path)?;
                let target = &mapping[&file.original_path];
                if transfer(&origin, Some(target))? != (file.size, file.sha256.clone()) {
                    return Err("复制后的文件校验失败。".into());
                }
            }
            for (id, title) in pairs(&snapshot, "SELECT id,title FROM book_projects")? {
                let target = create_named_directory(&root.join("projects"), &title)?;
                snapshot
                    .execute(
                        "UPDATE book_projects SET workspace_path=?1 WHERE id=?2",
                        params![target.to_string_lossy(), id],
                    )
                    .map_err(|e| e.to_string())?;
            }
            for (id, old) in pairs(&snapshot, "SELECT id,file_path FROM assets")? {
                if let Some(target) = mapping.get(&old) {
                    snapshot
                        .execute(
                            "UPDATE assets SET file_path=?1 WHERE id=?2",
                            params![target.to_string_lossy(), id],
                        )
                        .map_err(|e| e.to_string())?;
                }
            }
            for (id, old) in pairs(
                &snapshot,
                "SELECT id,exported_path FROM cover_versions WHERE exported_path IS NOT NULL",
            )? {
                snapshot
                    .execute(
                        "UPDATE cover_versions SET exported_path=?1 WHERE id=?2",
                        params![mapping[&old].to_string_lossy(), id],
                    )
                    .map_err(|e| e.to_string())?;
            }
            snapshot.execute("INSERT INTO app_settings(key,value) VALUES('workspace_root',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value",[root.to_string_lossy().as_ref()]).map_err(|e|e.to_string())?;
            for (id, _) in &tasks {
                publishing::rebase_package(&snapshot, id)?;
            }
            let previous_database = self.replace_snapshot(&snapshot)?;
            Ok(RestoreResult {
                previous_database,
                workspace_path: root.to_string_lossy().into_owned(),
            })
        })();
        if result.is_err() {
            cleanup_created(&root, &parent);
        }
        result
    }
}
