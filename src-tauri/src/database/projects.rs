use super::Database;
use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
};

const COLUMNS: &str = "id,title,author,publisher,isbn,description,source_notes,theme,target_audience,content_style,target_duration_seconds,status,workspace_path,created_at,updated_at,archived_at";
pub(super) const NOW: &str = "strftime('%Y-%m-%dT%H:%M:%fZ','now')";

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectInput {
    pub title: String,
    pub author: String,
    pub publisher: String,
    pub isbn: String,
    pub description: String,
    pub source_notes: String,
    pub theme: String,
    pub target_audience: String,
    pub content_style: String,
    pub target_duration_seconds: i64,
    pub status: String,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    #[serde(flatten)]
    pub input: ProjectInput,
    pub workspace_path: String,
    pub created_at: String,
    pub updated_at: String,
    pub archived_at: Option<String>,
    pub audio_duration_source: Option<String>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSettings {
    pub root: String,
    pub database_path: String,
}
fn row_project(row: &Row<'_>) -> rusqlite::Result<Project> {
    Ok(Project {
        id: row.get(0)?,
        input: ProjectInput {
            title: row.get(1)?,
            author: row.get(2)?,
            publisher: row.get(3)?,
            isbn: row.get(4)?,
            description: row.get(5)?,
            source_notes: row.get(6)?,
            theme: row.get(7)?,
            target_audience: row.get(8)?,
            content_style: row.get(9)?,
            target_duration_seconds: row.get(10)?,
            status: row.get(11)?,
        },
        workspace_path: row.get(12)?,
        created_at: row.get(13)?,
        updated_at: row.get(14)?,
        archived_at: row.get(15)?,
        audio_duration_source: row.get(16)?,
    })
}
pub(super) fn get(connection: &Connection, id: &str) -> Result<Project, String> {
    connection
        .query_row(
            &format!(
                "SELECT {COLUMNS},audio_duration_source
                FROM book_projects
                WHERE id=?1"
            ),
            [id],
            row_project,
        )
        .optional()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "项目不存在或已删除。".into())
}
pub(super) fn new_id(connection: &Connection) -> Result<String, String> {
    connection
        .query_row("SELECT lower(hex(randomblob(16)))", [], |r| r.get(0))
        .map_err(|e| e.to_string())
}

pub(super) fn create_named_directory(parent: &Path, title: &str) -> Result<PathBuf, String> {
    let mut name: String = title
        .trim()
        .chars()
        .map(|ch| {
            if ch.is_control() || matches!(ch, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*')
            {
                '_'
            } else {
                ch
            }
        })
        .collect();
    // Windows device names remain reserved even when followed by an extension.
    let stem = name
        .split('.')
        .next()
        .unwrap_or("")
        .trim_end()
        .to_ascii_uppercase();
    if matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || stem
            .strip_prefix("COM")
            .or_else(|| stem.strip_prefix("LPT"))
            .is_some_and(|suffix| {
                matches!(
                    suffix,
                    "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
                )
            })
    {
        name.insert(0, '_');
    }
    // Leave room for duplicate suffixes and files nested below the project on every platform.
    while name.len() > 120 {
        name.pop();
    }
    let name = name.trim_end_matches([' ', '.']);
    let name = if name.is_empty() {
        "未命名书籍"
    } else {
        name
    };
    fs::create_dir_all(parent).map_err(|e| format!("无法创建工作目录：{e}"))?;
    let mut index = 1u64;
    loop {
        let path = parent.join(if index == 1 {
            name.to_owned()
        } else {
            format!("{name} ({index})")
        });
        // Reserve the directory atomically; never reuse an existing file or folder.
        match fs::create_dir(&path) {
            Ok(()) => return Ok(path),
            Err(error) if error.kind() == ErrorKind::AlreadyExists => index += 1,
            Err(error) => return Err(format!("无法创建目录 {}：{error}", path.display())),
        }
    }
}

fn valid_status(status: &str) -> bool {
    matches!(
        status,
        "draft"
            | "researching"
            | "scripting"
            | "material_preparing"
            | "ready_to_publish"
            | "published"
            | "archived"
    )
}
fn validate(input: &ProjectInput) -> Result<(), String> {
    if input.title.trim().is_empty() || input.title.chars().count() > 200 {
        return Err("书名必填，且不能超过 200 字。".into());
    }
    if !(1..=86400).contains(&input.target_duration_seconds) || !valid_status(&input.status) {
        return Err("项目状态或目标时长无效。".into());
    }
    Ok(())
}
impl Database {
    pub fn list_projects(&self, search: &str, status: &str) -> Result<Vec<Project>, String> {
        if !status.is_empty() && !valid_status(status) {
            return Err("筛选状态无效。".into());
        }
        self.with_connection(|c| {
            let mut query = c.prepare(&format!("SELECT {COLUMNS},audio_duration_source
                FROM book_projects
                WHERE (?1='' OR instr(lower(title),lower(?1))>0 OR instr(lower(author),lower(?1))>0 OR instr(isbn,?1)>0) AND (?2='' OR status=?2)
                ORDER BY updated_at DESC,id" )).map_err(|e|e.to_string())?;
            let rows = query.query_map(params![search.trim(),status], row_project).map_err(|e|e.to_string())?;
            rows.collect::<rusqlite::Result<Vec<_>>>().map_err(|e|e.to_string())
        })
    }
    pub fn get_project(&self, id: &str) -> Result<Project, String> {
        self.with_connection(|c| get(c, id))
    }
    fn workspace_root(&self, c: &Connection) -> Result<PathBuf, String> {
        let root: Option<String> = c
            .query_row(
                "SELECT value
                FROM app_settings
                WHERE key='workspace_root'",
                [],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        Ok(root
            .map(PathBuf::from)
            .unwrap_or_else(|| self.path.parent().unwrap().join("workspace")))
    }
    pub fn workspace_settings(&self) -> Result<WorkspaceSettings, String> {
        self.with_connection(|c| {
            Ok(WorkspaceSettings {
                root: self.workspace_root(c)?.to_string_lossy().into(),
                database_path: self.path.to_string_lossy().into(),
            })
        })
    }
    pub fn set_workspace_root(&self, path: &str) -> Result<WorkspaceSettings, String> {
        let path = std::fs::canonicalize(path).map_err(|e| format!("工作目录不可用：{e}"))?;
        if !path.is_dir() {
            return Err("请选择文件夹。".into());
        }
        self.with_connection(|c| {
            // Test actual write access before persisting the setting.
            let probe = path.join(format!(".shuyin-{}", new_id(c)?));
            let file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&probe)
                .map_err(|e| format!("工作目录不可写：{e}"))?;
            drop(file);
            std::fs::remove_file(probe).map_err(|e| e.to_string())?;
            c.execute(
                "INSERT INTO app_settings(key,value) VALUES('workspace_root',?1)
                ON CONFLICT(key) DO UPDATE SET
                value=excluded.value",
                [path.to_string_lossy().as_ref()],
            )
            .map_err(|e| e.to_string())?;
            Ok(())
        })?;
        self.workspace_settings()
    }
    pub fn save_project(&self, id: Option<&str>, input: ProjectInput) -> Result<Project, String> {
        validate(&input)?;
        self.with_connection(|c| {
            let existing = id.map(|id|get(c,id)).transpose()?;
            let id = match id { Some(id) => id.to_owned(), None => new_id(c)? };
            let workspace = match &existing {
                Some(p) => PathBuf::from(&p.workspace_path),
                None => create_named_directory(&self.workspace_root(c)?.join("projects"), &input.title)?,
            };
            let result = c.execute(&format!("INSERT INTO book_projects ({COLUMNS}) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,{NOW},{NOW},CASE WHEN ?12='archived' THEN {NOW} END)
               
                ON CONFLICT(id) DO UPDATE SET
                title=excluded.title,author=excluded.author,publisher=excluded.publisher,isbn=excluded.isbn,description=excluded.description,source_notes=excluded.source_notes,theme=excluded.theme,target_audience=excluded.target_audience,content_style=excluded.content_style,target_duration_seconds=excluded.target_duration_seconds,status=excluded.status,updated_at=excluded.updated_at,archived_at=CASE WHEN excluded.status='archived' THEN coalesce(book_projects.archived_at,excluded.archived_at) END"),
                params![id,input.title.trim(),input.author,input.publisher,input.isbn,input.description,input.source_notes,input.theme,input.target_audience,input.content_style,input.target_duration_seconds,input.status,workspace.to_string_lossy().as_ref()]);
            if let Err(error) = result {
                // Only remove the empty directory created by this failed operation.
                if existing.is_none() { let _ = std::fs::remove_dir(&workspace); }
                return Err(error.to_string());
            }
            get(c,&id)
        })
    }
    pub fn delete_project(&self, id: &str) -> Result<(), String> {
        self.with_connection(|c| {
            get(c, id)?;
            c.execute(
                "DELETE
                FROM book_projects
                WHERE id=?1",
                [id],
            )
            .map_err(|e| e.to_string())?;
            Ok(()) // ON DELETE CASCADE removes references, never files.
        })
    }
    pub fn open_project_directory(&self, id: &str) -> Result<(), String> {
        let p = self.get_project(id)?;
        let path = Path::new(&p.workspace_path);
        if !path.is_dir() {
            return Err("项目工作目录已丢失，请重新指定目录。".into());
        }
        super::assets::open_path(path)
    }
    pub fn relocate_workspace(&self, id: &str, path: &str) -> Result<Project, String> {
        let path = std::fs::canonicalize(path).map_err(|e| e.to_string())?;
        if !path.is_dir() {
            return Err("请选择文件夹。".into());
        }
        self.with_connection(|c| {
            get(c, id)?;
            c.execute(
                &format!(
                    "UPDATE book_projects SET workspace_path=?1,updated_at={NOW}
                WHERE id=?2"
                ),
                params![path.to_string_lossy().as_ref(), id],
            )
            .map_err(|e| e.to_string())?;
            get(c, id)
        })
    }
}
