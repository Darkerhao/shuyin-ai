use super::{
    drafts::EditorDraft,
    m4_tests::{input, project},
    *,
};
use std::{fs, path::Path};

fn writing(db: &Database, id: &str, title: &str) -> ai::WritingVersion {
    db.save_writing(serde_json::from_value(serde_json::json!({"projectId":id,"kind":"publishing","content":{"format":"publishing","title":title,"description":"保留发布内容","audioPrompt":"原有音频指导"},"basedOn":null,"isConfirmed":false})).unwrap()).unwrap()
}

#[test]
fn draft_restarts_and_overwrites_only_the_same_editor_without_adding_history() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("data.sqlite3");
    let db = Database::open(path.clone());
    let p = project(&db);
    let original = writing(&db, &p.id, "正式标题");
    let foreign = project(&db);
    let other = writing(&db, &foreign.id, "其他书");
    let edit = serde_json::json!({"format":"publishing","title":"","description":"未完成的草稿"});
    for text in [serde_json::json!({"title":"临时"}), edit.clone()] {
        db.save_editor_draft(
            &p.id,
            "writing",
            Some(EditorDraft {
                based_on: Some(original.id.clone()),
                content: text,
                updated_at: String::new(),
            }),
        )
        .unwrap();
    }
    db.save_editor_draft(
        &p.id,
        "cover",
        Some(EditorDraft {
            based_on: None,
            content: serde_json::json!({"title":"未完成的封面"}),
            updated_at: String::new(),
        }),
    )
    .unwrap();
    assert!(db
        .save_editor_draft(
            &p.id,
            "writing",
            Some(EditorDraft {
                based_on: Some(other.id),
                content: edit.clone(),
                updated_at: String::new()
            })
        )
        .is_err());
    assert!(db.save_editor_draft(&p.id, "unknown", None).is_err());
    assert_eq!(db.list_versions(&p.id, "publishing").unwrap().len(), 1);
    assert!(db.list_cover_versions(&p.id).unwrap().is_empty());
    drop(db);
    let db = Database::open(path);
    assert_eq!(
        db.read_editor_draft(&p.id, "writing")
            .unwrap()
            .unwrap()
            .content,
        edit
    );
    assert_eq!(
        db.read_editor_draft(&p.id, "cover")
            .unwrap()
            .unwrap()
            .content["title"],
        "未完成的封面"
    );
    let backup = db.create_backup(dir.path().to_str().unwrap()).unwrap();
    db.save_editor_draft(&p.id, "writing", None).unwrap();
    assert!(db.read_editor_draft(&p.id, "writing").unwrap().is_none());
    db.restore_backup(&backup).unwrap();
    assert_eq!(
        db.read_editor_draft(&p.id, "writing")
            .unwrap()
            .unwrap()
            .content,
        edit
    );
    db.delete_project(&p.id).unwrap();
    assert_eq!(
        db.with_connection(|c| c
            .query_row("SELECT count(*) FROM editor_drafts", [], |r| r
                .get::<_, u32>(0))
            .map_err(|e| e.to_string()))
            .unwrap(),
        0
    );
}

#[test]
fn style_presets_contain_only_reusable_style_and_old_covers_default_to_center() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path().join("data.sqlite3"));
    let p = project(&db);
    let i = input(&db, &p);
    let mut config = db.list_cover_versions(&p.id).unwrap().remove(0).config;
    assert_eq!(
        (
            config.background_x,
            config.background_y,
            config.background_zoom
        ),
        (0.5, 0.5, 1.0)
    );
    config.brand = "我的品牌".into();
    config.background_x = 0.1;
    config.background_zoom = 2.0;
    db.save_cover_preset("个人风格", config.clone()).unwrap();
    assert!(db.save_cover_preset("个人风格", config.clone()).is_err());
    let presets = db.list_cover_presets().unwrap();
    let style = &presets[0].style;
    assert_eq!(style["brand"], "我的品牌");
    for key in [
        "title",
        "subtitle",
        "backgroundAssetId",
        "backgroundX",
        "backgroundY",
        "backgroundZoom",
        "width",
        "height",
    ] {
        assert!(style.get(key).is_none());
    }
    let saved = db.save_cover_version(&p.id, config.clone(), None).unwrap();
    assert_eq!(saved.config.background_zoom, 2.0);
    config.background_x = 1.01;
    assert!(db.save_cover_version(&p.id, config, None).is_err());
    assert_eq!(db.list_cover_versions(&p.id).unwrap().len(), 3);
    assert!(db
        .read_cover_export(&p.id, i.landscape_cover_id.as_deref().unwrap())
        .is_ok());
    let other = project(&db);
    assert!(db
        .read_cover_export(&other.id, i.landscape_cover_id.as_deref().unwrap())
        .is_err());
    let backup = db.create_backup(dir.path().to_str().unwrap()).unwrap();
    db.delete_cover_preset(&presets[0].id).unwrap();
    db.restore_backup(&backup).unwrap();
    assert_eq!(
        db.list_cover_presets().unwrap()[0].style["brand"],
        "我的品牌"
    );
}

#[test]
fn publish_tasks_preserve_historical_writing_source_and_reject_other_books() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path().join("data.sqlite3"));
    let p = project(&db);
    let old = writing(&db, &p.id, "旧标题");
    writing(&db, &p.id, "新标题");
    let mut i = input(&db, &p);
    i.writing_version_id = Some(old.id.clone());
    i.title = "旧标题的人工编辑".into();
    let task = db.save_publish_task(None, i.clone()).unwrap();
    assert_eq!(task.input.writing_version_id, Some(old.id.clone()));
    let other = project(&db);
    i.writing_version_id = Some(writing(&db, &other.id, "其他书").id);
    assert!(db.save_publish_task(Some(&task.id), i).is_err());
    assert_eq!(
        db.list_publish_tasks(&p.id).unwrap()[0]
            .input
            .writing_version_id,
        Some(old.id)
    );
    let ready = db.prepare_publish_task(&task.id).unwrap();
    let manifest: serde_json::Value = serde_json::from_slice(
        &fs::read(Path::new(ready.task_directory.as_deref().unwrap()).join("manifest.json"))
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        manifest["input"]["writingVersionId"],
        ready.input.writing_version_id.unwrap()
    );
}

#[test]
fn v8_upgrade_preserves_old_rows_and_rolls_back_a_conflicting_v9_schema() {
    let dir = tempfile::tempdir().unwrap();
    for conflict in [false, true] {
        let path = dir.path().join(format!("{conflict}.sqlite3"));
        let c = Connection::open(&path).unwrap();
        for sql in MIGRATIONS.iter().take(8) {
            c.execute_batch(sql).unwrap();
        }
        c.execute_batch("INSERT INTO book_projects(id,title,source_notes,target_duration_seconds,status,workspace_path,created_at,updated_at) VALUES('old','旧书','原始笔记',600,'draft','missing','original','original'); PRAGMA user_version=8;").unwrap();
        if conflict {
            c.execute_batch("CREATE TABLE cover_presets(original TEXT)")
                .unwrap();
        }
        drop(c);
        let db = Database::open(path.clone());
        if conflict {
            assert!(db.read().is_err());
            let c = Connection::open(path).unwrap();
            assert_eq!(
                c.pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
                    .unwrap(),
                8
            );
            assert!(c.prepare("SELECT * FROM editor_drafts").is_err());
        } else {
            assert_eq!(
                db.get_project("old").unwrap().input.source_notes,
                "原始笔记"
            );
            assert!(db.list_cover_presets().unwrap().is_empty());
            assert!(db.read_editor_draft("old", "writing").unwrap().is_none());
        }
    }
}

#[test]
fn full_backup_moves_and_restores_media_packages_drafts_styles_and_video_into_new_paths() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path().join("data.sqlite3"));
    let p = project(&db);
    let duplicate = project(&db);
    let text = writing(&db, &p.id, "备份文案");
    let mut i = input(&db, &p);
    i.writing_version_id = Some(text.id.clone());
    let task = db.save_publish_task(None, i).unwrap();
    db.prepare_publish_task(&task.id).unwrap();
    let image = Path::new(&p.workspace_path).join("原始背景.png");
    fs::write(&image, crate::ai::images::test_png()).unwrap();
    let asset = db.link_asset(&p.id, image.to_str().unwrap(), None).unwrap();
    let mut config = db.list_cover_versions(&p.id).unwrap()[0].config.clone();
    config.background_asset_id = Some(asset.clone());
    config.background_zoom = 1.75;
    config.background_x = 0.2;
    db.save_cover_preset("备份样式", config.clone()).unwrap();
    db.save_editor_draft(
        &p.id,
        "cover",
        Some(EditorDraft {
            based_on: None,
            content: serde_json::to_value(&config).unwrap(),
            updated_at: String::new(),
        }),
    )
    .unwrap();
    let backup = db
        .create_full_backup(dir.path().to_str().unwrap(), true)
        .unwrap();
    assert_eq!(backup.omitted_videos, 0);
    let moved = dir.path().join("搬移后的备份");
    fs::rename(Path::new(&backup.manifest_path).parent().unwrap(), &moved).unwrap();
    let manifest = moved.join("manifest.json");
    assert_eq!(
        db.inspect_full_backup(manifest.to_str().unwrap())
            .unwrap()
            .file_count,
        backup.file_count
    );
    let old = fs::canonicalize(&p.workspace_path).unwrap();
    assert!(old.starts_with(fs::canonicalize(dir.path()).unwrap()));
    fs::remove_dir_all(old).unwrap();
    db.delete_project(&p.id).unwrap();
    let current = project(&db);
    let result = db
        .restore_full_backup(manifest.to_str().unwrap(), dir.path().to_str().unwrap())
        .unwrap();
    let restored = db.get_project(&p.id).unwrap();
    let duplicate = db.get_project(&duplicate.id).unwrap();
    let mut names: Vec<_> = [&restored.workspace_path, &duplicate.workspace_path]
        .into_iter()
        .map(|path| {
            Path::new(path)
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    assert_eq!(names, ["中文书籍", "中文书籍 (2)"]);
    assert_ne!(restored.workspace_path, p.workspace_path);
    assert!(Path::new(&restored.workspace_path).is_dir());
    assert!(Path::new(&result.previous_database).is_file());
    let files = db.list_assets(&p.id).unwrap();
    assert!(files.iter().all(|a| a.exists_status == "available"));
    assert_eq!(
        fs::read(
            &files
                .iter()
                .find(|a| a.asset_type == "video")
                .unwrap()
                .file_path
        )
        .unwrap(),
        b"original video"
    );
    assert_eq!(
        db.read_cover_background(&p.id, &asset).unwrap(),
        crate::ai::images::test_png()
    );
    assert_eq!(
        db.read_editor_draft(&p.id, "cover")
            .unwrap()
            .unwrap()
            .content,
        serde_json::to_value(config).unwrap()
    );
    assert_eq!(db.list_cover_presets().unwrap()[0].name, "备份样式");
    let restored = db.list_publish_tasks(&p.id).unwrap().remove(0);
    assert_eq!(
        Path::new(restored.task_directory.as_ref().unwrap())
            .file_name()
            .unwrap(),
        "中文书籍-发布任务包"
    );
    assert_eq!(restored.status, "ready");
    assert!(restored.checklist.iter().all(|c| c.passed));
    assert_eq!(restored.input.writing_version_id, Some(text.id));
    db.restore_backup(&result.previous_database).unwrap();
    assert!(db.get_project(&current.id).is_ok());
}

#[test]
fn excluded_video_keeps_original_reference_and_missing_video_downgrades_ready_task() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path().join("data.sqlite3"));
    let p = project(&db);
    let i = input(&db, &p);
    let task = db.save_publish_task(None, i.clone()).unwrap();
    db.prepare_publish_task(&task.id).unwrap();
    let video = db.list_assets(&p.id).unwrap()[0].file_path.clone();
    let backup = db
        .create_full_backup(dir.path().to_str().unwrap(), false)
        .unwrap();
    assert_eq!(backup.omitted_videos, 1);
    fs::remove_file(&video).unwrap();
    db.restore_full_backup(&backup.manifest_path, dir.path().to_str().unwrap())
        .unwrap();
    assert_eq!(db.list_assets(&p.id).unwrap()[0].file_path, video);
    assert_eq!(db.list_assets(&p.id).unwrap()[0].exists_status, "missing");
    assert_eq!(db.list_publish_tasks(&p.id).unwrap()[0].status, "draft");
}

#[test]
fn full_backup_rejects_missing_files_same_size_corruption_and_traversal_before_replacing_data() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path().join("data.sqlite3"));
    let p = project(&db);
    let i = input(&db, &p);
    let backup = db
        .create_full_backup(dir.path().to_str().unwrap(), false)
        .unwrap();
    let path = Path::new(&backup.manifest_path);
    let original = fs::read(path).unwrap();
    let mut m: serde_json::Value = serde_json::from_slice(&original).unwrap();
    let file = path
        .parent()
        .unwrap()
        .join(m["files"][0]["relativePath"].as_str().unwrap());
    let mut bytes = fs::read(&file).unwrap();
    bytes[0] ^= 1;
    fs::write(&file, &bytes).unwrap();
    assert!(db
        .restore_full_backup(path.to_str().unwrap(), dir.path().to_str().unwrap())
        .unwrap_err()
        .contains("校验失败"));
    assert_eq!(db.get_project(&p.id).unwrap().input.title, p.input.title);
    bytes[0] ^= 1;
    fs::write(&file, bytes).unwrap();
    m["files"][0]["relativePath"] = serde_json::json!("../outside.png");
    fs::write(path, serde_json::to_vec(&m).unwrap()).unwrap();
    assert!(db
        .inspect_full_backup(path.to_str().unwrap())
        .unwrap_err()
        .contains("路径无效"));
    fs::write(path, original).unwrap();
    let cover = db
        .list_cover_versions(&p.id)
        .unwrap()
        .into_iter()
        .find(|c| Some(c.id.clone()) == i.landscape_cover_id)
        .unwrap();
    fs::remove_file(cover.exported_path.unwrap()).unwrap();
    assert!(db
        .create_full_backup(dir.path().to_str().unwrap(), false)
        .is_err());
    assert_eq!(
        db.get_project(&p.id).unwrap().input.source_notes,
        "原始资料"
    );
}
