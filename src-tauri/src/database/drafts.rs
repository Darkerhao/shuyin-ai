use super::{projects::get, projects::NOW, Database};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EditorDraft {
    pub based_on: Option<String>,
    pub content: Value,
    #[serde(default)]
    pub updated_at: String,
}

fn validate_kind(kind: &str) -> Result<(), String> {
    if matches!(kind, "writing" | "cover" | "project" | "publishing") {
        Ok(())
    } else {
        Err("草稿类型无效。".into())
    }
}

// New-book drafts have no project yet. Publishing drafts belong to a specific task,
// or to the one unsaved publishing form for a book.
fn project_for_owner(c: &Connection, owner: &str, kind: &str) -> Result<Option<String>, String> {
    if kind == "project" && owner == "new" {
        return Ok(None);
    }
    if let Some((project, chapter)) = owner.split_once(":chapter:") {
        get(c, project)?;
        if kind == "project" && chapter == "new" {
            return Ok(Some(project.into()));
        }
        let chapter = if kind == "publishing" {
            chapter
                .strip_suffix(":new")
                .ok_or("章节发布草稿所有者无效。")?
        } else {
            chapter
        };
        super::chapters::get_chapter(c, project, chapter)?;
        return Ok(Some(project.into()));
    }
    let project_id = if kind == "publishing" {
        let (project, task) = owner.split_once(':').ok_or("发布草稿所有者无效。")?;
        if task != "new" && !c.query_row(
            "SELECT EXISTS(SELECT 1 FROM publish_tasks WHERE id=?1 AND project_id=?2 AND status!='published')",
            params![task, project], |r| r.get::<_, bool>(0),
        ).map_err(|e| e.to_string())? {
            return Err("发布任务不存在、已发布或不属于当前书籍。".into());
        }
        project
    } else {
        owner
    };
    get(c, project_id)?;
    Ok(Some(project_id.into()))
}

impl Database {
    pub fn read_editor_draft(
        &self,
        project_id: &str,
        kind: &str,
    ) -> Result<Option<EditorDraft>, String> {
        validate_kind(kind)?;
        self.with_connection(|c| {
            project_for_owner(c, project_id, kind)?;
            let saved: Option<(Option<String>, String, String)> = c.query_row(
                "SELECT based_on,content_json,updated_at FROM editor_drafts WHERE owner_id=?1 AND kind=?2",
                params![project_id,kind], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?)),
            ).optional().map_err(|e|e.to_string())?;
            saved.map(|(based_on,json,updated_at)| Ok(EditorDraft { based_on,content:serde_json::from_str(&json).map_err(|e|e.to_string())?,updated_at })).transpose()
        })
    }

    pub fn save_editor_draft(
        &self,
        project_id: &str,
        kind: &str,
        draft: Option<EditorDraft>,
    ) -> Result<(), String> {
        validate_kind(kind)?;
        // Incomplete edits are valid drafts; final version validation runs only on explicit save.
        let json = draft
            .as_ref()
            .map(|d| serde_json::to_string(&d.content))
            .transpose()
            .map_err(|e| e.to_string())?;
        if draft.as_ref().is_some_and(|d| !d.content.is_object())
            || json.as_ref().is_some_and(|s| s.len() > 512 * 1024)
        {
            return Err("草稿格式无效或超过 512 KB。".into());
        }
        self.with_connection(|c| {
            let book = project_for_owner(c, project_id, kind)?;
            if let (Some(draft),Some(json)) = (draft,json) {
                if let Some(id) = draft.based_on.as_ref().filter(|_| matches!(kind, "writing" | "cover")) {
                    let chapter = project_id.split_once(":chapter:").map(|(_,chapter)| chapter);
                    let sql = if kind=="writing" { "SELECT EXISTS(SELECT 1 FROM writing_versions WHERE id=?1 AND project_id=?2 AND chapter_id IS ?3 AND kind='publishing')" }
                        else { "SELECT EXISTS(SELECT 1 FROM cover_versions WHERE id=?1 AND project_id=?2 AND chapter_id IS ?3)" };
                    if !c.query_row(sql,params![id,book,chapter],|r|r.get::<_,bool>(0)).map_err(|e|e.to_string())? { return Err("草稿来源版本不属于当前书籍与章节。".into()); }
                }
                c.execute(&format!("INSERT INTO editor_drafts(owner_id,project_id,kind,based_on,content_json,updated_at) VALUES(?1,?2,?3,?4,?5,{NOW}) ON CONFLICT(owner_id,kind) DO UPDATE SET based_on=excluded.based_on,content_json=excluded.content_json,updated_at=excluded.updated_at"),params![project_id,book,kind,draft.based_on,json]).map_err(|e|e.to_string())?;
            } else {
                c.execute("DELETE FROM editor_drafts WHERE owner_id=?1 AND kind=?2",params![project_id,kind]).map_err(|e|e.to_string())?;
            }
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::{m4_tests, MIGRATIONS};

    fn draft(content: Value) -> Option<EditorDraft> {
        Some(EditorDraft {
            based_on: None,
            content,
            updated_at: String::new(),
        })
    }

    #[test]
    fn editor_owners_migration_preserves_old_drafts_and_reopens() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("v18.sqlite3");
        let connection = Connection::open(&path).unwrap();
        for sql in MIGRATIONS.iter().take(18) {
            connection.execute_batch(sql).unwrap();
        }
        connection.pragma_update(None, "user_version", 18).unwrap();
        connection.execute("INSERT INTO book_projects(id,title,author,publisher,isbn,description,source_notes,theme,target_audience,content_style,target_duration_seconds,status,workspace_path,created_at,updated_at) VALUES('old','旧书','','','','','','','','',600,'draft','','now','now')", []).unwrap();
        connection.execute("INSERT INTO editor_drafts(project_id,kind,based_on,content_json,updated_at) VALUES('old','writing',NULL,'{\"title\":\"原草稿\"}','before')", []).unwrap();
        drop(connection);
        for _ in 0..2 {
            let db = Database::open(path.clone());
            let value = db.read_editor_draft("old", "writing").unwrap().unwrap();
            assert_eq!(value.content["title"], "原草稿");
            assert_eq!(value.updated_at, "before");
            assert!(db
                .with_connection(|c| Ok(!c
                    .prepare("PRAGMA foreign_key_check")
                    .unwrap()
                    .exists([])
                    .unwrap()))
                .unwrap());
        }
    }

    #[test]
    fn incomplete_forms_are_isolated_and_survive_backup_without_business_writes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("drafts.sqlite3");
        let db = Database::open(path.clone());
        let book = m4_tests::project(&db);
        let input = m4_tests::input(&db, &book);
        let first = db.save_publish_task(None, input.clone()).unwrap();
        let second = db.save_publish_task(None, input).unwrap();
        let owners = [
            ("new".to_owned(), "project"),
            (book.id.clone(), "project"),
            (format!("{}:new", book.id), "publishing"),
            (format!("{}:{}", book.id, first.id), "publishing"),
            (format!("{}:{}", book.id, second.id), "publishing"),
        ];
        for (index, (owner, kind)) in owners.iter().enumerate() {
            db.save_editor_draft(owner, kind, draft(serde_json::json!({"title":"","scheduledAt":"","marker":index,"sourcePaths":["D:/尚未绑定.pdf"]}))).unwrap();
        }
        assert_eq!(db.list_publish_tasks(&book.id, None).unwrap().len(), 2);
        assert_eq!(
            db.get_project(&book.id).unwrap().input.title,
            book.input.title
        );
        let backup = db.create_backup(dir.path().to_str().unwrap()).unwrap();
        drop(db);
        let db = Database::open(path);
        db.restore_backup(&backup).unwrap();
        for (index, (owner, kind)) in owners.iter().enumerate() {
            assert_eq!(
                db.read_editor_draft(owner, kind).unwrap().unwrap().content["marker"],
                index
            );
        }
        let foreign = m4_tests::project(&db);
        assert!(db
            .save_editor_draft(
                &format!("{}:{}", foreign.id, first.id),
                "publishing",
                draft(serde_json::json!({}))
            )
            .is_err());
        db.delete_project(&book.id).unwrap();
        assert_eq!(
            db.with_connection(|c| c
                .query_row(
                    "SELECT count(*) FROM editor_drafts WHERE project_id=?1",
                    [&book.id],
                    |r| r.get::<_, u32>(0)
                )
                .map_err(|e| e.to_string()))
                .unwrap(),
            0
        );
    }

    #[test]
    fn formal_save_clears_only_its_draft_and_failed_save_preserves_it() {
        let dir = tempfile::tempdir().unwrap();
        let db = Database::open(dir.path().join("save.sqlite3"));
        let book = m4_tests::project(&db);
        db.save_editor_draft(
            &book.id,
            "project",
            draft(serde_json::json!({"title":"暂存"})),
        )
        .unwrap();
        let mut invalid = book.input.clone();
        invalid.title.clear();
        assert!(db.save_project(Some(&book.id), invalid).is_err());
        assert!(db.read_editor_draft(&book.id, "project").unwrap().is_some());
        db.save_project(Some(&book.id), book.input.clone()).unwrap();
        assert!(db.read_editor_draft(&book.id, "project").unwrap().is_none());
        let input = m4_tests::input(&db, &book);
        let task = db.save_publish_task(None, input.clone()).unwrap();
        let owner = format!("{}:{}", book.id, task.id);
        let new_owner = format!("{}:new", book.id);
        for key in [&owner, &new_owner] {
            db.save_editor_draft(
                key,
                "publishing",
                draft(serde_json::json!({"scheduledAt":""})),
            )
            .unwrap();
        }
        let mut invalid = input.clone();
        invalid.scheduled_at = Some(String::new());
        assert!(db.save_publish_task(Some(&task.id), invalid).is_err());
        assert!(db
            .read_editor_draft(&owner, "publishing")
            .unwrap()
            .is_some());
        db.save_publish_task(Some(&task.id), input).unwrap();
        assert!(db
            .read_editor_draft(&owner, "publishing")
            .unwrap()
            .is_none());
        assert!(db
            .read_editor_draft(&new_owner, "publishing")
            .unwrap()
            .is_some());
        db.with_connection(|c| {
            c.execute(
                "UPDATE publish_tasks SET status='published' WHERE id=?1",
                [&task.id],
            )
            .unwrap();
            Ok(())
        })
        .unwrap();
        assert!(db
            .save_editor_draft(&owner, "publishing", draft(serde_json::json!({})))
            .is_err());
    }
}
