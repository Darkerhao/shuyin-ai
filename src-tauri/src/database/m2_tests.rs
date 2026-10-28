use super::ai::*;
use super::*;
use crate::{ai::WritingContent, credentials::Credentials};
use rusqlite::params;
use std::collections::HashMap;
#[derive(Default)]
pub(super) struct Vault {
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
pub(super) fn model() -> ProviderInput {
    ProviderInput {
        model_kind: "text".into(),
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
        revision: None,
        target_duration_seconds: None,
        recommend_duration: false,
    }
}
#[test]
fn duration_is_saved_independently_and_generation_snapshots_the_selected_value() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("duration.sqlite3");
    let db = Database::open(path.clone());
    let p = project(&db);
    let provider = db
        .save_provider(None, model(), None, &Vault::default())
        .unwrap()
        .provider;
    let base = db
        .save_writing(SaveWritingInput {
            project_id: p.id.clone(),
            kind: "publishing".into(),
            content: publishing(),
            based_on: None,
            is_confirmed: false,
        })
        .unwrap();
    db.set_audio_duration(&p.id, 1500).unwrap();
    let task = db
        .start_generation(generation(&p.id, &provider.id))
        .unwrap();
    db.set_audio_duration(&p.id, 1800).unwrap();
    let prepared = db.prepare_generation(&task).unwrap();
    assert_eq!(prepared.input.target_duration_seconds, Some(1500));
    let user: serde_json::Value = serde_json::from_str(&prepared.user).unwrap();
    assert_eq!(user["audio"]["duration"], "25分钟");
    assert_eq!(user["audio"]["spokenCharacters"], 5500);
    assert_eq!(user["book"]["sourceNotes"], p.input.source_notes);
    assert!(db.set_audio_duration(&p.id, 61).is_err());
    assert!(db.set_audio_duration("missing", 600).is_err());
    let mut invalid = generation(&p.id, &provider.id);
    invalid.target_duration_seconds = Some(0);
    assert!(db.start_generation(invalid).is_err());
    db.end_generation(&task, "cancelled", "cancelled", "cancelled")
        .unwrap();
    let backup = db.create_backup(dir.path().to_str().unwrap()).unwrap();
    db.set_audio_duration(&p.id, 600).unwrap();
    db.restore_backup(&backup).unwrap();
    drop(db);
    let db = Database::open(path);
    let restored = db.get_project(&p.id).unwrap();
    assert_eq!(restored.input.target_duration_seconds, 1800);
    assert_eq!(restored.audio_duration_source.as_deref(), Some("manual"));
    assert_eq!(restored.input.source_notes, p.input.source_notes);
    assert_eq!(
        db.list_versions(&p.id, "publishing").unwrap()[0].id,
        base.id
    );
}

#[test]
fn recommended_duration_is_atomic_and_cancelled_or_invalid_results_do_not_change_it() {
    use crate::ai::audio::DurationRecommendation;
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path().join("duration.sqlite3"));
    let p = project(&db);
    let provider = db
        .save_provider(None, model(), None, &Vault::default())
        .unwrap()
        .provider;
    let mut input = generation(&p.id, &provider.id);
    input.recommend_duration = true;
    let task = db.start_generation(input.clone()).unwrap();
    let prepared = db.prepare_generation(&task).unwrap();
    let user: serde_json::Value = serde_json::from_str(&prepared.user).unwrap();
    assert!(user.get("audio").is_none());
    db.end_generation(&task, "cancelled", "cancelled", "cancelled")
        .unwrap();
    let result = || DurationRecommendation {
        minutes: 25,
        reason: "按现有资料估计。".into(),
    };
    assert!(db.finish_duration_recommendation(&task, result()).is_err());
    assert_eq!(
        db.get_project(&p.id).unwrap().input.target_duration_seconds,
        p.input.target_duration_seconds
    );
    let task = db.start_generation(input.clone()).unwrap();
    db.prepare_generation(&task).unwrap();
    assert!(db
        .finish_duration_recommendation(
            &task,
            DurationRecommendation {
                minutes: 0,
                reason: "无效".into()
            }
        )
        .is_err());
    db.with_connection(|c| { c.execute_batch("CREATE TRIGGER fail_duration BEFORE UPDATE ON ai_tasks WHEN NEW.status='succeeded' BEGIN SELECT RAISE(ABORT,'test failure'); END;").unwrap(); Ok(()) }).unwrap();
    assert!(db.finish_duration_recommendation(&task, result()).is_err());
    assert_eq!(
        db.get_project(&p.id).unwrap().input.target_duration_seconds,
        p.input.target_duration_seconds
    );
    assert!(db
        .get_project(&p.id)
        .unwrap()
        .audio_duration_source
        .is_none());
    db.with_connection(|c| {
        c.execute_batch("DROP TRIGGER fail_duration").unwrap();
        Ok(())
    })
    .unwrap();
    db.finish_duration_recommendation(&task, result()).unwrap();
    assert_eq!(
        db.get_project(&p.id).unwrap().input.target_duration_seconds,
        1500
    );
    assert_eq!(
        db.get_project(&p.id)
            .unwrap()
            .audio_duration_source
            .as_deref(),
        Some("ai")
    );
    assert!(db.list_versions(&p.id, "publishing").unwrap().is_empty());
    assert!(db.finish_duration_recommendation(&task, result()).is_err());
    input.revision = Some(crate::ai::revision::RevisionInput {
        based_on: "missing".into(),
        fields: vec![],
        instruction: String::new(),
    });
    assert!(db.start_generation(input).is_err());
}

#[test]
fn v12_duration_migration_preserves_old_data_and_rolls_back_conflicts() {
    let dir = tempfile::tempdir().unwrap();
    for conflict in [false, true] {
        let path = dir.path().join(format!("v11-{conflict}.sqlite3"));
        let c = Connection::open(&path).unwrap();
        for sql in MIGRATIONS.iter().take(11) {
            c.execute_batch(sql).unwrap();
        }
        c.execute_batch("PRAGMA user_version=11; INSERT INTO book_projects(id,title,source_notes,target_duration_seconds,status,workspace_path,created_at,updated_at) VALUES('p','保留旧书','保留资料',900,'draft','D:/books','original','original'); INSERT INTO writing_versions(id,project_id,kind,version_no,content_json,source_type,model_name,prompt_version,is_confirmed,created_at) VALUES('v','p','publishing',1,'{\"format\":\"publishing\",\"title\":\"旧标题\",\"description\":\"旧内容\",\"audioPrompt\":\"8—10分钟旧指导\"}','ai','model','publishing-v1',0,'original');").unwrap();
        if conflict {
            c.execute_batch("ALTER TABLE book_projects ADD COLUMN audio_duration_source TEXT")
                .unwrap();
        }
        drop(c);
        let db = Database::open(path.clone());
        if conflict {
            assert!(db.read().is_err());
        } else {
            let p = db.get_project("p").unwrap();
            assert_eq!(p.input.target_duration_seconds, 900);
            assert!(p.audio_duration_source.is_none());
            assert_eq!(p.updated_at, "original");
            assert_eq!(db.list_versions("p", "publishing").unwrap()[0].id, "v");
        }
        drop(db);
        let c = Connection::open(&path).unwrap();
        assert_eq!(
            c.pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
                .unwrap(),
            if conflict {
                11
            } else {
                MIGRATIONS.len() as u32
            }
        );
        assert_eq!(
            c.query_row(
                "SELECT source_notes FROM book_projects WHERE id='p'",
                [],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
            "保留资料"
        );
        assert_eq!(c.query_row("SELECT json_extract(content_json,'$.audioPrompt') FROM writing_versions WHERE id='v'",[],|r|r.get::<_,String>(0)).unwrap(),"8—10分钟旧指导");
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
    db.finish_generation(&id, &input, publishing(), "test")
        .unwrap();
    db.delete_provider(&first.id, &vault).unwrap();
    assert!(vault.entries.lock().unwrap().is_empty());
    assert_eq!(db.list_versions(&p.id, "publishing").unwrap().len(), 1);
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
        .finish_generation(&task, &input, publishing(), "test")
        .is_err());
    assert!(db.list_versions(&p.id, "publishing").unwrap().is_empty());
    let task = db.start_generation(input.clone()).unwrap();
    db.prepare_generation(&task).unwrap();
    db.with_connection(|c|c.execute_batch("CREATE TRIGGER fail_task BEFORE UPDATE ON ai_tasks WHEN NEW.status='succeeded' BEGIN SELECT RAISE(ABORT,'test commit failure'); END;").map_err(|e|e.to_string())).unwrap();
    assert!(db
        .finish_generation(&task, &input, publishing(), "test")
        .is_err());
    assert!(db.list_versions(&p.id, "publishing").unwrap().is_empty());
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

fn publishing() -> WritingContent {
    WritingContent::Publishing {
        title: "发布标题".into(),
        description: "发布内容".into(),
        audio_prompt: "音频提示词".into(),
        cover_title: "换了几辈子，还是放不下".into(),
        cover_subtitle: "《生死疲劳》里的执念".into(),
        cover_prompt_landscape: "横版 4:3，田野横向展开\n中央留白，无文字。".into(),
        cover_prompt_portrait: "竖版 3:4，田垄向远方延伸\n中央留白，无文字。".into(),
        cover_design: Some(serde_json::from_value(serde_json::json!({"titleSize":148,"subtitleSize":54,"textColor":"#ffe45c","subtitleColor":"#ffffff","accentColor":"#ff7547","backgroundColor":"#141414","overlayOpacity":0.75})).unwrap()),
    }
}

#[test]
fn cover_materials_and_legacy_publishing_survive_save_backup_and_restart() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("data.sqlite3");
    let db = Database::open(path.clone());
    let p = project(&db);
    let legacy_json = r#"{"format":"publishing","title":"旧标题","description":"旧内容","audioPrompt":"旧提示词"}"#;
    db.with_connection(|c| {
        c.execute("INSERT INTO writing_versions(id,project_id,kind,version_no,content_json,source_type,model_name,prompt_version,is_confirmed,created_at) VALUES('old',?1,'publishing',1,?2,'ai','test','publishing-v1',1,'old')", params![p.id, legacy_json]).unwrap();
        Ok(())
    }).unwrap();
    let old = db.list_versions(&p.id, "publishing").unwrap().remove(0);
    let WritingContent::Publishing {
        cover_title,
        cover_subtitle,
        cover_prompt_landscape,
        cover_prompt_portrait,
        ..
    } = &old.content
    else {
        panic!()
    };
    assert!([
        cover_title,
        cover_subtitle,
        cover_prompt_landscape,
        cover_prompt_portrait
    ]
    .iter()
    .all(|value| value.is_empty()));
    let input = SaveWritingInput {
        project_id: p.id.clone(),
        kind: "publishing".into(),
        content: old.content.clone(),
        based_on: Some(old.id),
        is_confirmed: false,
    };
    db.save_writing(input).unwrap();
    let mut input = SaveWritingInput {
        project_id: p.id.clone(),
        kind: "publishing".into(),
        content: publishing(),
        based_on: None,
        is_confirmed: false,
    };
    db.save_writing(SaveWritingInput {
        project_id: input.project_id.clone(),
        kind: input.kind.clone(),
        content: input.content.clone(),
        based_on: None,
        is_confirmed: false,
    })
    .unwrap();
    if let WritingContent::Publishing {
        cover_prompt_portrait,
        ..
    } = &mut input.content
    {
        *cover_prompt_portrait = "字".repeat(2001);
    }
    assert!(db.save_writing(input).is_err());
    assert_eq!(db.list_versions(&p.id, "publishing").unwrap().len(), 3);
    let backup = db.create_backup(dir.path().to_str().unwrap()).unwrap();
    db.delete_project(&p.id).unwrap();
    db.restore_backup(&backup).unwrap();
    drop(db);
    let reopened = Database::open(path);
    let versions = reopened.list_versions(&p.id, "publishing").unwrap();
    assert_eq!(versions.len(), 3);
    assert_eq!(versions[0].content, publishing());
    assert_eq!(versions[2].content, old.content);
    reopened
        .with_connection(|c| {
            let unchanged: String = c
                .query_row(
                    "SELECT content_json FROM writing_versions WHERE id='old'",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(unchanged, legacy_json);
            assert_eq!(
                c.pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
                    .unwrap(),
                MIGRATIONS.len() as u32
            );
            Ok(())
        })
        .unwrap();
}

#[test]
fn v3_upgrade_preserves_old_versions_task_links_and_backup_and_allows_title_only_generation() {
    let dir = tempfile::tempdir().unwrap();
    let old = dir.path().join("v3.sqlite3");
    let c = Connection::open(&old).unwrap();
    c.pragma_update(None, "foreign_keys", true).unwrap();
    for sql in MIGRATIONS.iter().take(3) {
        c.execute_batch(sql).unwrap();
    }
    c.execute_batch("INSERT INTO book_projects(id,title,target_duration_seconds,status,workspace_path,created_at,updated_at) VALUES('p','旧书',600,'draft','D:/books','old','old');
    INSERT INTO model_providers VALUES('m','模型','openai-compatible','https://example.com/v1','model',1,1,'reference','old','old');
    INSERT INTO writing_versions VALUES('v','p','outline',1,'{\"format\":\"markdown\",\"text\":\"旧稿\"}','ai','model','writing-v1',1,'old');
    INSERT INTO ai_tasks(id,project_id,kind,provider_id,model_name,prompt_version,status,input_json,version_id,created_at,completed_at) VALUES('t','p','outline','m','model','writing-v1','succeeded','{}','v','old','old');
    PRAGMA user_version=3;").unwrap();
    drop(c);
    let current = dir.path().join("current.sqlite3");
    std::fs::copy(&old, &current).unwrap();
    let db = Database::open(current.clone());
    assert_eq!(
        db.list_ai_tasks("p").unwrap()[0].version_id.as_deref(),
        Some("v")
    );
    assert_eq!(
        db.list_versions("p", "outline").unwrap()[0].content,
        markdown("旧稿")
    );
    let new = db
        .start_generation(GenerationInput {
            project_id: "p".into(),
            provider_id: "m".into(),
            revision: None,
            target_duration_seconds: None,
            recommend_duration: false,
        })
        .unwrap();
    db.prepare_generation(&new).unwrap();
    db.finish_generation(
        &new,
        &GenerationInput {
            project_id: "p".into(),
            provider_id: "m".into(),
            revision: None,
            target_duration_seconds: None,
            recommend_duration: false,
        },
        publishing(),
        "model",
    )
    .unwrap();
    assert_eq!(db.list_versions("p", "publishing").unwrap().len(), 1);
    let v4 = db.create_backup(dir.path().to_str().unwrap()).unwrap();
    db.restore_backup(old.to_str().unwrap()).unwrap();
    assert_eq!(
        db.list_ai_tasks("p").unwrap()[0].version_id.as_deref(),
        Some("v")
    );
    db.restore_backup(&v4).unwrap();
    assert_eq!(db.list_versions("p", "publishing").unwrap().len(), 1);
    let c = Connection::open(current).unwrap();
    assert!(!c
        .prepare("PRAGMA foreign_key_check")
        .unwrap()
        .exists([])
        .unwrap());
    assert_eq!(
        c.pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
            .unwrap(),
        MIGRATIONS.len() as u32
    );
    assert_eq!(
        Connection::open(old)
            .unwrap()
            .pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
            .unwrap(),
        3
    );
}
#[test]
fn v4_migration_failure_rolls_back_without_dropping_old_task_links() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("conflict.sqlite3");
    let c = Connection::open(&path).unwrap();
    for sql in MIGRATIONS.iter().take(3) {
        c.execute_batch(sql).unwrap();
    }
    c.execute_batch("CREATE TABLE ai_tasks_v4(original TEXT); INSERT INTO ai_tasks_v4 VALUES('keep'); PRAGMA user_version=3;").unwrap();
    drop(c);
    assert!(Database::open(path.clone()).read().is_err());
    let c = Connection::open(path).unwrap();
    assert_eq!(
        c.pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
            .unwrap(),
        3
    );
    assert_eq!(
        c.query_row("SELECT original FROM ai_tasks_v4", [], |r| r
            .get::<_, String>(0))
            .unwrap(),
        "keep"
    );
    assert!(c.prepare("SELECT * FROM writing_versions").is_ok());
    assert!(c.prepare("SELECT * FROM writing_versions_v4").is_err());
}

#[test]
fn revision_preview_requires_adoption_and_adoption_is_atomic_and_idempotent() {
    use crate::ai::revision::{self, PublishingField, RevisionInput};
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path().join("data.sqlite3"));
    let project = project(&db);
    let provider = db
        .save_provider(None, model(), None, &Vault::default())
        .unwrap()
        .provider;
    let base = db
        .save_writing(SaveWritingInput {
            project_id: project.id.clone(),
            kind: "publishing".into(),
            content: publishing(),
            based_on: None,
            is_confirmed: false,
        })
        .unwrap();
    let mut input = generation(&project.id, &provider.id);
    input.revision = Some(RevisionInput {
        based_on: base.id.clone(),
        fields: vec![PublishingField::Title],
        instruction: "标题更简洁".into(),
    });
    let id = db.start_generation(input.clone()).unwrap();
    let prepared = db.prepare_generation(&id).unwrap();
    assert!(prepared.user.contains("标题更简洁"));
    let content = revision::revise(
        r#"{"title":"调整后的标题"}"#,
        &base.content,
        &input.revision.as_ref().unwrap().fields,
        "书名",
        600,
    )
    .unwrap();
    let preview = db.finish_revision(&id, &input, content.clone()).unwrap();
    assert_eq!(preview.before, base.content);
    assert_eq!(
        db.list_versions(&project.id, "publishing").unwrap().len(),
        1
    );
    // A failed database write must retain the candidate so adoption can be retried.
    db.with_connection(|c| {
        c.pragma_update(None, "query_only", true)
            .map_err(|e| e.to_string())
    })
    .unwrap();
    assert!(db.apply_generation(&id).is_err());
    db.with_connection(|c| {
        c.pragma_update(None, "query_only", false)
            .map_err(|e| e.to_string())
    })
    .unwrap();
    let backup = db.create_backup(dir.path().to_str().unwrap()).unwrap();
    db.restore_backup(&backup).unwrap();
    let adopted = db.apply_generation(&id).unwrap();
    assert_eq!(adopted.content, content);
    assert_eq!(adopted.source_type, "mixed");
    assert_eq!(adopted.prompt_version, revision::PROMPT_VERSION);
    assert_eq!(db.apply_generation(&id).unwrap().id, adopted.id);
    assert_eq!(
        db.list_versions(&project.id, "publishing").unwrap().len(),
        2
    );
    assert_eq!(
        db.list_ai_tasks(&project.id).unwrap()[0]
            .version_id
            .as_deref(),
        Some(adopted.id.as_str())
    );
    assert_eq!(
        db.list_versions(&project.id, "publishing").unwrap()[1].content,
        base.content
    );
}

#[test]
fn revision_rejects_foreign_sources_duplicate_fields_and_late_cancelled_responses() {
    use crate::ai::revision::{PublishingField, RevisionInput};
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path().join("data.sqlite3"));
    let p = project(&db);
    let other = project(&db);
    let provider = db
        .save_provider(None, model(), None, &Vault::default())
        .unwrap()
        .provider;
    let base = db
        .save_writing(SaveWritingInput {
            project_id: p.id.clone(),
            kind: "publishing".into(),
            content: publishing(),
            based_on: None,
            is_confirmed: false,
        })
        .unwrap();
    let mut input = generation(&other.id, &provider.id);
    input.revision = Some(RevisionInput {
        based_on: base.id.clone(),
        fields: vec![PublishingField::Title],
        instruction: String::new(),
    });
    assert!(db.start_generation(input.clone()).is_err());
    input.project_id = p.id.clone();
    input
        .revision
        .as_mut()
        .unwrap()
        .fields
        .push(PublishingField::Title);
    assert!(db.start_generation(input.clone()).is_err());
    input.revision.as_mut().unwrap().fields.pop();
    let id = db.start_generation(input.clone()).unwrap();
    db.prepare_generation(&id).unwrap();
    db.end_generation(&id, "cancelled", "cancelled", "用户取消")
        .unwrap();
    assert!(db.finish_revision(&id, &input, publishing()).is_err());
    assert!(db.apply_generation(&id).is_err());
    assert_eq!(db.list_versions(&p.id, "publishing").unwrap().len(), 1);
}

#[test]
fn v8_migration_conflict_keeps_v7_data_and_version() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("v7.sqlite3");
    let c = Connection::open(&path).unwrap();
    for sql in MIGRATIONS.iter().take(7) {
        c.execute_batch(sql).unwrap();
    }
    c.execute_batch("INSERT INTO m0_probe VALUES(1,'保留旧数据','原时间'); ALTER TABLE ai_tasks ADD COLUMN preview_json TEXT; PRAGMA user_version=7;").unwrap();
    drop(c);
    assert!(Database::open(path.clone()).read().is_err());
    let c = Connection::open(path).unwrap();
    assert_eq!(
        c.pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
            .unwrap(),
        7
    );
    assert_eq!(
        c.query_row("SELECT note FROM m0_probe", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "保留旧数据"
    );
}
