use super::{
    projects::{get, new_id, NOW},
    Database,
};
use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChapterInput {
    pub title: String,
    pub content: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Chapter {
    pub id: String,
    pub project_id: String,
    #[serde(flatten)]
    pub input: ChapterInput,
    pub target_duration_seconds: Option<i64>,
    pub audio_duration_source: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChapterSummary {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub target_duration_seconds: Option<i64>,
    pub audio_duration_source: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChapterSource {
    pub chapter_id: String,
    pub book_title: String,
    pub author: String,
    pub chapter_title: String,
    pub content: String,
    pub sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_duration_seconds: Option<i64>,
}

pub fn text_hash(text: &str) -> String {
    super::audio::hex_digest(&Sha256::digest(text.as_bytes()))
}

impl ChapterSource {
    pub fn label(&self) -> String {
        format!("《{}》· {}", self.book_title, self.chapter_title)
    }
    pub fn validate(&self, chapter: &str) -> Result<(), String> {
        if self.chapter_id != chapter
            || self.content.trim().is_empty()
            || self.content.chars().count() > 100_000
            || self.sha256 != text_hash(&self.content)
        {
            return Err("章节来源快照无效，已有内容保留。".into());
        }
        Ok(())
    }
    pub fn model_book(&self) -> serde_json::Value {
        serde_json::json!({"title":self.book_title,"author":self.author,"description":"","sourceNotes":self.content})
    }
}

const COLUMNS: &str = "id,project_id,title,content,target_duration_seconds,audio_duration_source,created_at,updated_at";
fn row_chapter(row: &Row<'_>) -> rusqlite::Result<Chapter> {
    Ok(Chapter {
        id: row.get(0)?,
        project_id: row.get(1)?,
        input: ChapterInput {
            title: row.get(2)?,
            content: row.get(3)?,
        },
        target_duration_seconds: row.get(4)?,
        audio_duration_source: row.get(5)?,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
    })
}

pub(super) fn get_chapter(
    connection: &Connection,
    project: &str,
    id: &str,
) -> Result<Chapter, String> {
    connection
        .query_row(
            &format!("SELECT {COLUMNS} FROM book_chapters WHERE id=?1 AND project_id=?2"),
            params![id, project],
            row_chapter,
        )
        .optional()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "章节不存在或不属于当前书籍。".into())
}

pub(super) fn require_scope(
    connection: &Connection,
    project: &str,
    chapter: Option<&str>,
) -> Result<(), String> {
    get(connection, project)?;
    if let Some(chapter) = chapter {
        get_chapter(connection, project, chapter)?;
    }
    Ok(())
}

pub(super) fn snapshot(
    connection: &Connection,
    project: &str,
    chapter: &str,
) -> Result<ChapterSource, String> {
    let book = get(connection, project)?;
    let chapter = get_chapter(connection, project, chapter)?;
    Ok(ChapterSource {
        chapter_id: chapter.id,
        book_title: book.input.title,
        author: book.input.author,
        chapter_title: chapter.input.title,
        sha256: text_hash(&chapter.input.content),
        target_duration_seconds: None,
        content: chapter.input.content,
    })
}

pub(super) fn asset_scope(
    connection: &Connection,
    project: &str,
    id: &str,
    chapter: Option<&str>,
    shared: bool,
) -> Result<(), String> {
    let owner: Option<Option<String>> = connection
        .query_row(
            "SELECT chapter_id FROM assets WHERE id=?1 AND project_id=?2",
            params![id, project],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    let owner = owner.ok_or("素材不存在或不属于当前书籍。")?;
    if owner.as_deref() != chapter && !(shared && owner.is_none()) {
        return Err("素材不属于本次创作章节，请选择对应素材。".into());
    }
    Ok(())
}

impl Database {
    pub fn list_chapters(&self, project: &str) -> Result<Vec<ChapterSummary>, String> {
        self.with_connection(|connection| {
            get(connection, project)?;
            let mut query = connection.prepare("SELECT id,project_id,title,target_duration_seconds,audio_duration_source,created_at,updated_at FROM book_chapters WHERE project_id=?1 ORDER BY created_at,id").map_err(|error| error.to_string())?;
            let rows = query.query_map([project], |row| Ok(ChapterSummary { id:row.get(0)?,project_id:row.get(1)?,title:row.get(2)?,target_duration_seconds:row.get(3)?,audio_duration_source:row.get(4)?,created_at:row.get(5)?,updated_at:row.get(6)? })).map_err(|error| error.to_string())?;
            rows.collect::<rusqlite::Result<Vec<_>>>().map_err(|error| error.to_string())
        })
    }
    pub fn get_chapter(&self, project: &str, id: &str) -> Result<Chapter, String> {
        self.with_connection(|connection| get_chapter(connection, project, id))
    }
    pub fn save_chapter(
        &self,
        project: &str,
        id: Option<&str>,
        mut input: ChapterInput,
    ) -> Result<Chapter, String> {
        input.title = input.title.trim().into();
        input.content = input
            .content
            .trim_start_matches('\u{feff}')
            .replace("\r\n", "\n")
            .replace('\r', "\n")
            .trim()
            .into();
        if input.title.is_empty()
            || input.title.chars().count() > 200
            || input.content.is_empty()
            || input.content.chars().count() > 100_000
            || input.content.contains('\0')
        {
            return Err("章节名称必填且最多200字，正文必填且最多10万字。".into());
        }
        let is_new = id.is_none();
        self.with_connection(|connection| {
            let transaction = connection.transaction().map_err(|error| error.to_string())?;
            get(&transaction, project)?;
            let id = match id { Some(id) => { get_chapter(&transaction,project,id)?; id.to_owned() }, None => new_id(&transaction)? };
            transaction.execute(&format!("INSERT INTO book_chapters(id,project_id,title,content,created_at,updated_at) VALUES(?1,?2,?3,?4,{NOW},{NOW}) ON CONFLICT(id) DO UPDATE SET title=excluded.title,content=excluded.content,updated_at={NOW}"), params![id,project,input.title,input.content]).map_err(|error| error.to_string())?;
            transaction.execute("DELETE FROM editor_drafts WHERE owner_id=?1 AND kind='project'", [format!("{project}:chapter:{id}")]).map_err(|error| error.to_string())?;
            if is_new {
                transaction.execute("DELETE FROM editor_drafts WHERE owner_id=?1 AND kind='project'", [format!("{project}:chapter:new")]).map_err(|error| error.to_string())?;
            }
            transaction.execute(&format!("UPDATE book_projects SET updated_at={NOW} WHERE id=?1"), [project]).map_err(|error| error.to_string())?;
            let saved = get_chapter(&transaction, project, &id)?;
            transaction.commit().map_err(|error| error.to_string())?;
            Ok(saved)
        })
    }
}
