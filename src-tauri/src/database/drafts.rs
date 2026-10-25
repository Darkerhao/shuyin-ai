use super::{projects::get, projects::NOW, Database};
use rusqlite::{params, OptionalExtension};
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
    if matches!(kind, "writing" | "cover") {
        Ok(())
    } else {
        Err("草稿类型无效。".into())
    }
}

impl Database {
    pub fn read_editor_draft(
        &self,
        project_id: &str,
        kind: &str,
    ) -> Result<Option<EditorDraft>, String> {
        validate_kind(kind)?;
        self.with_connection(|c| {
            get(c, project_id)?;
            let saved: Option<(Option<String>, String, String)> = c.query_row(
                "SELECT based_on,content_json,updated_at FROM editor_drafts WHERE project_id=?1 AND kind=?2",
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
            get(c, project_id)?;
            if let (Some(draft),Some(json)) = (draft,json) {
                if let Some(id) = &draft.based_on {
                    let sql = if kind=="writing" { "SELECT EXISTS(SELECT 1 FROM writing_versions WHERE id=?1 AND project_id=?2 AND kind='publishing')" }
                        else { "SELECT EXISTS(SELECT 1 FROM cover_versions WHERE id=?1 AND project_id=?2)" };
                    if !c.query_row(sql,params![id,project_id],|r|r.get::<_,bool>(0)).map_err(|e|e.to_string())? { return Err("草稿来源版本不属于当前书籍。".into()); }
                }
                c.execute(&format!("INSERT INTO editor_drafts(project_id,kind,based_on,content_json,updated_at) VALUES(?1,?2,?3,?4,{NOW}) ON CONFLICT(project_id,kind) DO UPDATE SET based_on=excluded.based_on,content_json=excluded.content_json,updated_at=excluded.updated_at"),params![project_id,kind,draft.based_on,json]).map_err(|e|e.to_string())?;
            } else {
                c.execute("DELETE FROM editor_drafts WHERE project_id=?1 AND kind=?2",params![project_id,kind]).map_err(|e|e.to_string())?;
            }
            Ok(())
        })
    }
}
