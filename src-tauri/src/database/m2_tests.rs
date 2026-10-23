use super::ai::*;
use super::*;
use crate::{ai::WritingContent, credentials::Credentials};
use std::collections::HashMap;
#[derive(Default)]
struct Vault {
    entries: Mutex<HashMap<String, String>>,
    fail: bool,
}
impl Credentials for Vault {
    fn get(&self, r: &str) -> Result<Option<String>, String> {
        Ok(self.entries.lock().unwrap().get(r).cloned())
    }
    fn set(&self, r: &str, s: &str) -> Result<(), String> {
        if self.fail {
            return Err("vault locked".into());
        }
        self.entries.lock().unwrap().insert(r.into(), s.into());
        Ok(())
    }
    fn delete(&self, r: &str) -> Result<(), String> {
        self.entries.lock().unwrap().remove(r);
        Ok(())
    }
}
fn project(db: &Database) -> Project {
    db.save_project(
        None,
        ProjectInput {
            title: "书名".into(),
            author: "作者".into(),
            publisher: "".into(),
            isbn: "".into(),
            description: "用户资料".into(),
            source_notes: "读书笔记".into(),
            theme: "".into(),
            target_audience: "".into(),
            content_style: "".into(),
            target_duration_seconds: 180,
            status: "draft".into(),
        },
    )
    .unwrap()
}
fn model() -> ProviderInput {
    ProviderInput {
        name: "测试模型".into(),
        base_url: "https://example.com/v1".into(),
        model_name: "test".into(),
        is_default: true,
        is_enabled: true,
    }
}
fn markdown(s: &str) -> WritingContent {
    WritingContent::Markdown { text: s.into() }
}
fn manual(project_id: &str, s: &str) -> SaveWritingInput {
    SaveWritingInput {
        project_id: project_id.into(),
        kind: "outline".into(),
        content: markdown(s),
        based_on: None,
        is_confirmed: true,
    }
}
fn generation(project_id: &str, provider_id: &str) -> GenerationInput {
    GenerationInput {
        project_id: project_id.into(),
        provider_id: provider_id.into(),
        kind: "outline".into(),
        count: 5,
        reference_version_id: None,
    }
}
#[test]
fn v2_upgrade_preserves_data_and_v3_failure_rolls_back() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("data.sqlite3");
    let c = Connection::open(&path).unwrap();
    for sql in MIGRATIONS.iter().take(2) {
        c.execute_batch(sql).unwrap();
    }
    c.execute("INSERT INTO m0_probe VALUES(1,'旧数据','old-time')", [])
        .unwrap();
    c.pragma_update(None, "user_version", 2).unwrap();
    drop(c);
    let db = Database::open(path.clone());
    assert_eq!(db.read().unwrap().record.unwrap().updated_at, "old-time");
    let p = project(&db);
    drop(db);
    assert_eq!(
        Database::open(path).get_project(&p.id).unwrap().input.title,
        "书名"
    );
    let conflict = dir.path().join("conflict.sqlite3");
    let c = Connection::open(&conflict).unwrap();
    for sql in MIGRATIONS.iter().take(2) {
        c.execute_batch(sql).unwrap();
    }
    c.execute_batch("CREATE TABLE writing_versions(original TEXT); INSERT INTO writing_versions VALUES('keep'); PRAGMA user_version=2;").unwrap();
    drop(c);
    assert!(Database::open(conflict.clone()).read().is_err());
    let c = Connection::open(conflict).unwrap();
    assert_eq!(
        c.pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
            .unwrap(),
        2
    );
    assert_eq!(
        c.query_row("SELECT original FROM writing_versions", [], |r| r
            .get::<_, String>(0))
            .unwrap(),
        "keep"
    );
    assert_eq!(
        c.query_row(
            "SELECT count(*) FROM sqlite_master WHERE name='model_providers'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
}
#[test]
fn credentials_never_enter_sqlite_and_failed_saves_preserve_config() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("data.sqlite3");
    let db = Database::open(path.clone());
    let vault = Vault::default();
    let p = db
        .save_provider(
            None,
            model(),
            Some("test-secret-must-not-be-persisted"),
            &vault,
        )
        .unwrap()
        .provider;
    assert_eq!(p.credential_status, "available");
    let public = serde_json::to_string(&db.list_providers(&vault).unwrap()).unwrap();
    assert!(!public.contains("credentialRef"));
    assert!(!public.contains("test-secret-must-not-be-persisted"));
    let mut changed = model();
    changed.base_url = "https://different.example.com/v1".into();
    assert!(db
        .save_provider(Some(&p.id), changed, None, &vault)
        .is_err());
    assert!(db
        .save_provider(
            None,
            model(),
            Some("new"),
            &Vault {
                fail: true,
                ..Default::default()
            }
        )
        .is_err());
    assert_eq!(db.list_providers(&vault).unwrap().len(), 1);
    db.with_connection(|c| {
        c.pragma_update(None, "query_only", true)
            .map_err(|e| e.to_string())
    })
    .unwrap();
    assert!(db
        .save_provider(Some(&p.id), model(), Some("replacement"), &vault)
        .is_err());
    assert_eq!(vault.entries.lock().unwrap().len(), 1);
    assert_eq!(
        vault
            .get(p.credential_ref.as_deref().unwrap())
            .unwrap()
            .unwrap(),
        "test-secret-must-not-be-persisted"
    );
    let bytes = std::fs::read(path).unwrap();
    assert!(!bytes
        .windows(b"test-secret-must-not-be-persisted".len())
        .any(|w| w == b"test-secret-must-not-be-persisted"));
}
#[test]
fn default_model_is_unique_and_deletion_preserves_history() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path().join("data.sqlite3"));
    let vault = Vault::default();
    let first = db
        .save_provider(None, model(), Some("one"), &vault)
        .unwrap()
        .provider;
    let second = db
        .save_provider(None, model(), None, &vault)
        .unwrap()
        .provider;
    assert!(!db.get_provider(&first.id).unwrap().input.is_default);
    assert!(db.get_provider(&second.id).unwrap().input.is_default);
    let p = project(&db);
    let input = generation(&p.id, &first.id);
    let id = db.start_generation(input.clone()).unwrap();
    assert!(db.delete_provider(&first.id, &vault).is_err());
    db.prepare_generation(&id).unwrap();
    db.finish_generation(&id, &input, markdown("生成大纲"), "test")
        .unwrap();
    db.delete_provider(&first.id, &vault).unwrap();
    assert!(vault.entries.lock().unwrap().is_empty());
    assert_eq!(db.list_versions(&p.id, "outline").unwrap().len(), 1);
    assert_eq!(db.list_ai_tasks(&p.id).unwrap()[0].status, "succeeded");
}
#[test]
fn versions_are_append_only_validated_and_persist_after_restart() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("data.sqlite3");
    let db = Database::open(path.clone());
    let p = project(&db);
    let first = db.save_writing(manual(&p.id, "已确认初稿")).unwrap();
    let mut revised = manual(&p.id, "修改后");
    revised.based_on = Some(first.id.clone());
    let second = db.save_writing(revised).unwrap();
    assert_eq!(second.version_no, 2);
    assert!(db.save_writing(manual(&p.id, " ")).is_err());
    let other = project(&db);
    let mut wrong = manual(&other.id, "别的项目");
    wrong.based_on = Some(first.id.clone());
    assert!(db.save_writing(wrong).is_err());
    let backup = db.create_backup(dir.path().to_str().unwrap()).unwrap();
    db.delete_project(&p.id).unwrap();
    db.restore_backup(&backup).unwrap();
    drop(db);
    let reopened = Database::open(path);
    let all = reopened.list_versions(&p.id, "outline").unwrap();
    assert_eq!(all.len(), 2);
    assert_eq!(all[1].content, markdown("已确认初稿"));
    assert!(all[1].is_confirmed);
    assert_eq!(all[0].version_no, 2);
}
#[test]
fn task_success_is_atomic_and_cancelled_or_invalid_results_never_create_versions() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path().join("data.sqlite3"));
    let p = project(&db);
    let model = db
        .save_provider(None, model(), None, &Vault::default())
        .unwrap()
        .provider;
    let input = generation(&p.id, &model.id);
    let task = db.start_generation(input.clone()).unwrap();
    db.prepare_generation(&task).unwrap();
    assert!(db
        .finish_generation(
            &task,
            &input,
            WritingContent::List {
                items: vec!["错误结构".into()]
            },
            "test"
        )
        .is_err());
    db.end_generation(&task, "cancelled", "cancelled", "用户取消")
        .unwrap();
    assert!(db
        .finish_generation(&task, &input, markdown("迟到的结果"), "test")
        .is_err());
    assert!(db.list_versions(&p.id, "outline").unwrap().is_empty());
    let task = db.start_generation(input.clone()).unwrap();
    db.prepare_generation(&task).unwrap();
    db.with_connection(|c|c.execute_batch("CREATE TRIGGER fail_task BEFORE UPDATE ON ai_tasks WHEN NEW.status='succeeded' BEGIN SELECT RAISE(ABORT,'test commit failure'); END;").map_err(|e|e.to_string())).unwrap();
    assert!(db
        .finish_generation(&task, &input, markdown("必须回滚"), "test")
        .is_err());
    assert!(db.list_versions(&p.id, "outline").unwrap().is_empty());
    db.recover_ai_tasks().unwrap();
    assert!(db
        .list_ai_tasks(&p.id)
        .unwrap()
        .iter()
        .any(|t| t.error_code.as_deref() == Some("interrupted")));
}
#[test]
fn v2_backup_is_upgraded_in_memory_and_active_generation_blocks_restore() {
    let dir = tempfile::tempdir().unwrap();
    let old = dir.path().join("old.sqlite3");
    let c = Connection::open(&old).unwrap();
    for sql in MIGRATIONS.iter().take(2) {
        c.execute_batch(sql).unwrap();
    }
    c.execute("INSERT INTO m0_probe VALUES(1,'M1备份','keep')", [])
        .unwrap();
    c.pragma_update(None, "user_version", 2).unwrap();
    drop(c);
    let db = Database::open(dir.path().join("current.sqlite3"));
    let p = project(&db);
    let m = db
        .save_provider(None, model(), None, &Vault::default())
        .unwrap()
        .provider;
    let id = db.start_generation(generation(&p.id, &m.id)).unwrap();
    assert!(db
        .restore_backup(old.to_str().unwrap())
        .unwrap_err()
        .contains("生成任务"));
    assert!(db.get_project(&p.id).is_ok());
    db.end_generation(&id, "cancelled", "cancelled", "取消")
        .unwrap();
    let previous = db.restore_backup(old.to_str().unwrap()).unwrap();
    assert_eq!(db.read().unwrap().record.unwrap().note, "M1备份");
    assert!(db.list_providers(&Vault::default()).unwrap().is_empty());
    assert!(Database::open(previous.into()).get_project(&p.id).is_ok());
    assert_eq!(
        Connection::open(old)
            .unwrap()
            .pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
            .unwrap(),
        2
    );
}
