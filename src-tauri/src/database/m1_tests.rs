use super::*;
use std::fs;
fn input(title: &str) -> ProjectInput {
    ProjectInput {
        title: title.into(),
        author: "作者".into(),
        publisher: String::new(),
        isbn: "9780123".into(),
        description: "简介".into(),
        source_notes: "笔记".into(),
        theme: "成长".into(),
        target_audience: "读者".into(),
        content_style: "通俗".into(),
        target_duration_seconds: 180,
        status: "draft".into(),
    }
}

#[test]
fn bound_documents_persist_with_book_and_keep_source_files() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("data.sqlite3");
    let db = Database::open(path.clone());
    let mut sources = Vec::new();
    for extension in ["pdf", "epub", "docx", "txt", "md"] {
        let source = dir.path().join(format!("中文原书.{extension}"));
        fs::write(&source, b"original book").unwrap();
        sources.push(source.to_str().unwrap().to_owned());
    }
    let saved = db
        .save_project_with_documents(None, input("带文件的新书"), &sources)
        .unwrap();
    let assets = db.list_assets(&saved.id).unwrap();
    assert_eq!(assets.len(), 5);
    assert!(assets
        .iter()
        .all(|asset| asset.asset_type == "document" && asset.exists_status == "available"));
    assert_eq!(fs::read_dir(&saved.workspace_path).unwrap().count(), 0);
    drop(db);
    let db = Database::open(path);
    assert_eq!(db.list_assets(&saved.id).unwrap().len(), 5);
    fs::remove_file(&sources[0]).unwrap();
    assert!(db
        .list_assets(&saved.id)
        .unwrap()
        .iter()
        .any(|asset| asset.exists_status == "missing"));
    let replacement = dir.path().join("重新定位.pdf");
    fs::write(&replacement, b"relocated book").unwrap();
    let missing_id = &assets
        .iter()
        .find(|asset| asset.original_name.ends_with(".pdf"))
        .unwrap()
        .id;
    db.link_asset(&saved.id, replacement.to_str().unwrap(), Some(missing_id))
        .unwrap();
    assert!(db
        .list_assets(&saved.id)
        .unwrap()
        .iter()
        .all(|asset| asset.exists_status == "available"));
    db.delete_project(&saved.id).unwrap();
    assert_eq!(fs::read(replacement).unwrap(), b"relocated book");
    for source in &sources[1..] {
        assert_eq!(fs::read(source).unwrap(), b"original book");
    }
}

#[test]
fn bound_documents_roll_back_new_and_existing_books_on_invalid_or_duplicate_sources() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path().join("data.sqlite3"));
    let source = dir.path().join("原书.pdf");
    fs::write(&source, b"original").unwrap();
    let source = source.to_str().unwrap().to_owned();
    let video = dir.path().join("成片.mp4");
    fs::write(&video, b"video").unwrap();
    for invalid in [
        dir.path().join("丢失.pdf").to_str().unwrap().to_owned(),
        video.to_str().unwrap().to_owned(),
        source.clone(),
    ] {
        assert!(db
            .save_project_with_documents(None, input("保存失败"), &[source.clone(), invalid])
            .is_err());
        assert!(db.list_projects("", "").unwrap().is_empty());
        let root = PathBuf::from(db.workspace_settings().unwrap().root).join("projects");
        assert_eq!(fs::read_dir(root).unwrap().count(), 0);
    }
    let book = db
        .save_project_with_documents(None, input("原书名"), std::slice::from_ref(&source))
        .unwrap();
    let extra = dir.path().join("补充.md");
    fs::write(&extra, b"notes").unwrap();
    assert!(db
        .save_project_with_documents(
            Some(&book.id),
            input("不应写入"),
            &[extra.to_str().unwrap().to_owned(), source.clone()]
        )
        .is_err());
    assert_eq!(db.get_project(&book.id).unwrap().input.title, "原书名");
    assert_eq!(db.list_assets(&book.id).unwrap().len(), 1);
    let updated = db
        .save_project_with_documents(
            Some(&book.id),
            input("更新成功"),
            &[extra.to_str().unwrap().to_owned()],
        )
        .unwrap();
    assert_eq!(updated.input.title, "更新成功");
    assert_eq!(updated.workspace_path, book.workspace_path);
    assert_eq!(db.list_assets(&book.id).unwrap().len(), 2);
    assert_eq!(fs::read(source).unwrap(), b"original");
}

#[cfg(target_os = "windows")]
#[test]
fn bound_documents_roll_back_when_source_access_is_denied() {
    use std::{fs::OpenOptions, os::windows::fs::OpenOptionsExt};
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path().join("data.sqlite3"));
    let source = dir.path().join("锁定.pdf");
    fs::write(&source, b"book").unwrap();
    let locked = OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(&source)
        .unwrap();
    assert!(db
        .save_project_with_documents(
            None,
            input("无法访问"),
            &[source.to_str().unwrap().to_owned()]
        )
        .is_err());
    assert!(db.list_projects("", "").unwrap().is_empty());
    drop(locked);
}

#[test]
fn project_directories_use_book_titles_and_keep_existing_files_on_name_collisions() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path().join("data.sqlite3"));
    let first = db.save_project(None, input("  三体  ")).unwrap();
    let workspace = PathBuf::from(&first.workspace_path);
    assert_eq!(workspace.file_name().unwrap(), "三体");
    fs::write(workspace.join("保留.txt"), "原项目文件").unwrap();
    fs::write(workspace.with_file_name("三体 (2)"), "已有同名文件").unwrap();
    let second = db.save_project(None, input("三体")).unwrap();
    assert_eq!(
        PathBuf::from(&second.workspace_path).file_name().unwrap(),
        "三体 (3)"
    );
    assert_ne!(first.id, second.id);
    db.delete_project(&first.id).unwrap();
    let third = db.save_project(None, input("三体")).unwrap();
    assert_eq!(
        PathBuf::from(&third.workspace_path).file_name().unwrap(),
        "三体 (4)"
    );
    assert_eq!(
        fs::read_to_string(workspace.join("保留.txt")).unwrap(),
        "原项目文件"
    );
    drop(db);
    assert_eq!(
        Database::open(dir.path().join("data.sqlite3"))
            .get_project(&second.id)
            .unwrap()
            .workspace_path,
        second.workspace_path
    );
}

#[test]
fn book_titles_make_portable_directory_names_without_changing_the_saved_title() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path().join("data.sqlite3"));
    for (title, expected) in [
        ("《三体》：黑暗森林", "《三体》：黑暗森林"),
        ("书名<>:\"/\\|?*\n结尾. ", "书名__________结尾"),
        ("..", "未命名书籍"),
        ("Con.txt", "_Con.txt"),
        ("LPT1", "_LPT1"),
        ("COM¹", "_COM¹"),
    ] {
        let p = db.save_project(None, input(title)).unwrap();
        let path = PathBuf::from(&p.workspace_path);
        assert_eq!(path.file_name().unwrap(), expected, "{title}");
        assert_eq!(p.input.title, title.trim());
        assert!(path.is_dir());
    }
    let long = "书".repeat(200);
    let p = db.save_project(None, input(&long)).unwrap();
    let name = PathBuf::from(&p.workspace_path)
        .file_name()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    assert!(long.starts_with(&name));
    assert!(name.len() <= 120);
    assert_eq!(p.input.title, long);
}

#[test]
fn editing_legacy_projects_preserves_their_directory_and_asset_references() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path().join("data.sqlite3"));
    let p = db.save_project(None, input("原书名")).unwrap();
    let legacy = PathBuf::from(&p.workspace_path).with_file_name(&p.id);
    fs::create_dir(&legacy).unwrap();
    let p = db
        .relocate_workspace(&p.id, legacy.to_str().unwrap())
        .unwrap();
    let video = legacy.join("视频.mp4");
    fs::write(&video, b"original video").unwrap();
    db.link_asset(&p.id, video.to_str().unwrap(), None).unwrap();
    let edited = db.save_project(Some(&p.id), input("新书名")).unwrap();
    assert_eq!(edited.workspace_path, p.workspace_path);
    assert_eq!(db.list_assets(&p.id).unwrap()[0].exists_status, "available");
    assert_eq!(fs::read(video).unwrap(), b"original video");
}
#[test]
fn upgrade_v1_preserves_probe_and_repeated_open_preserves_projects() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("data.sqlite3");
    let c = Connection::open(&path).unwrap();
    c.execute_batch(include_str!("../../migrations/0001_m0_probe.sql"))
        .unwrap();
    c.execute(
        "INSERT INTO m0_probe VALUES(1,'M0 原记录','2026-10-23T00:00:00Z')",
        [],
    )
    .unwrap();
    c.pragma_update(None, "user_version", 1).unwrap();
    drop(c);
    let db = Database::open(path.clone());
    let saved = db.save_project(None, input("书名 %_' 中文")).unwrap();
    assert_eq!(db.read().unwrap().record.unwrap().note, "M0 原记录");
    drop(db);
    for _ in 0..2 {
        let reopened = Database::open(path.clone());
        assert_eq!(
            reopened.get_project(&saved.id).unwrap().input.source_notes,
            "笔记"
        );
        assert_eq!(reopened.list_projects("%_", "").unwrap().len(), 1);
        assert_eq!(reopened.list_projects("作者", "draft").unwrap().len(), 1);
        assert!(reopened.list_projects("不存在", "").unwrap().is_empty());
        assert_eq!(reopened.read().unwrap().record.unwrap().note, "M0 原记录");
    }
}
#[test]
fn v2_failure_rolls_back_every_business_table_and_keeps_v1() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("data.sqlite3");
    let c = Connection::open(&path).unwrap();
    c.execute_batch(include_str!("../../migrations/0001_m0_probe.sql"))
        .unwrap();
    c.execute_batch("CREATE TABLE assets(original TEXT); INSERT INTO assets VALUES('keep'); PRAGMA user_version=1;").unwrap();
    drop(c);
    assert!(Database::open(path.clone()).read().is_err());
    let c = Connection::open(path).unwrap();
    assert_eq!(
        c.pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
            .unwrap(),
        1
    );
    assert_eq!(
        c.query_row("SELECT original FROM assets", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "keep"
    );
    assert_eq!(
        c.query_row(
            "SELECT count(*) FROM sqlite_master WHERE name='book_projects'",
            [],
            |r| r.get::<_, u32>(0)
        )
        .unwrap(),
        0
    );
}
#[test]
fn edit_archive_unarchive_and_failed_save_keep_data() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path().join("data.sqlite3"));
    let saved = db.save_project(None, input("原书名")).unwrap();
    let mut edited = input("新书名");
    edited.status = "archived".into();
    let archived = db.save_project(Some(&saved.id), edited).unwrap();
    assert!(archived.archived_at.is_some());
    assert_eq!(db.list_projects("", "archived").unwrap().len(), 1);
    let active = db.save_project(Some(&saved.id), input("新书名")).unwrap();
    assert!(active.archived_at.is_none());
    assert!(db.save_project(Some(&saved.id), input("  ")).is_err());
    db.with_connection(|c| {
        c.pragma_update(None, "query_only", true)
            .map_err(|e| e.to_string())
    })
    .unwrap();
    assert!(db.save_project(Some(&saved.id), input("失败内容")).is_err());
    assert!(db.save_project(None, input("创建失败")).is_err());
    assert_eq!(db.get_project(&saved.id).unwrap().input.title, "新书名");
    assert_eq!(
        fs::read_dir(PathBuf::from(active.workspace_path).parent().unwrap())
            .unwrap()
            .count(),
        1
    );
}
#[test]
fn missing_relocate_duplicate_and_delete_never_touch_source() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path().join("data.sqlite3"));
    let project = db.save_project(None, input("素材项目")).unwrap();
    let original = dir.path().join("中文 视频.mp4");
    fs::write(&original, b"source-content").unwrap();
    db.link_asset(&project.id, original.to_str().unwrap(), None)
        .unwrap();
    assert!(db
        .link_asset(&project.id, original.to_str().unwrap(), None)
        .is_err());
    let linked = db.list_assets(&project.id).unwrap().remove(0);
    assert_eq!(linked.exists_status, "available");
    let moved = dir.path().join("移动后.mp4");
    fs::rename(&original, &moved).unwrap();
    assert_eq!(
        db.list_assets(&project.id).unwrap()[0].exists_status,
        "missing"
    );
    assert!(db
        .link_asset(&project.id, original.to_str().unwrap(), Some(&linked.id))
        .is_err());
    let wrong = dir.path().join("错误.png");
    fs::write(&wrong, b"image").unwrap();
    assert!(db
        .link_asset(&project.id, wrong.to_str().unwrap(), Some(&linked.id))
        .is_err());
    db.link_asset(&project.id, moved.to_str().unwrap(), Some(&linked.id))
        .unwrap();
    let relocated = db.list_assets(&project.id).unwrap().remove(0);
    assert_eq!(relocated.id, linked.id);
    assert_eq!(relocated.exists_status, "available");
    db.unlink_asset(&linked.id).unwrap();
    assert_eq!(fs::read(&moved).unwrap(), b"source-content");
    db.link_asset(&project.id, moved.to_str().unwrap(), None)
        .unwrap();
    db.delete_project(&project.id).unwrap();
    assert!(db.get_project(&project.id).is_err());
    assert_eq!(fs::read(&moved).unwrap(), b"source-content");
    assert!(PathBuf::from(project.workspace_path).is_dir());
    db.with_connection(|c| {
        assert_eq!(
            c.query_row("SELECT count(*) FROM assets", [], |r| r.get::<_, u32>(0))
                .unwrap(),
            0
        );
        Ok(())
    })
    .unwrap();
}
#[test]
fn workspace_changes_only_affect_new_projects_and_relocation_only_changes_reference() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path().join("data.sqlite3"));
    let old = db.save_project(None, input("旧项目")).unwrap();
    let root = dir.path().join("新工作目录");
    fs::create_dir(&root).unwrap();
    db.set_workspace_root(root.to_str().unwrap()).unwrap();
    let new = db.save_project(None, input("新项目")).unwrap();
    assert!(PathBuf::from(new.workspace_path).starts_with(fs::canonicalize(&root).unwrap()));
    assert_eq!(
        db.get_project(&old.id).unwrap().workspace_path,
        old.workspace_path
    );
    fs::write(PathBuf::from(&old.workspace_path).join("keep.txt"), "keep").unwrap();
    db.relocate_workspace(&old.id, root.to_str().unwrap())
        .unwrap();
    assert!(PathBuf::from(old.workspace_path).join("keep.txt").exists());
    assert!(db
        .set_workspace_root(dir.path().join("missing").to_str().unwrap())
        .is_err());
    assert_eq!(fs::read_dir(root).unwrap().count(), 1);
}
#[test]
fn backup_restore_roundtrip_preserves_previous_database_and_missing_references() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("data.sqlite3");
    let db = Database::open(path.clone());
    let project = db.save_project(None, input("备份项目")).unwrap();
    let video = dir.path().join("video.mp4");
    fs::write(&video, "video").unwrap();
    db.link_asset(&project.id, video.to_str().unwrap(), None)
        .unwrap();
    let backup = db.create_backup(dir.path().to_str().unwrap()).unwrap();
    let second = db.create_backup(dir.path().to_str().unwrap()).unwrap();
    assert_ne!(backup, second);
    db.delete_project(&project.id).unwrap();
    let later = db.save_project(None, input("恢复前项目")).unwrap();
    fs::remove_file(video).unwrap();
    let previous = db.restore_backup(&backup).unwrap();
    assert_eq!(db.get_project(&project.id).unwrap().input.title, "备份项目");
    assert_eq!(
        db.list_assets(&project.id).unwrap()[0].exists_status,
        "missing"
    );
    assert_eq!(
        Database::open(previous.into())
            .get_project(&later.id)
            .unwrap()
            .input
            .title,
        "恢复前项目"
    );
    drop(db);
    let reopened = Database::open(path);
    assert!(reopened.get_project(&project.id).is_ok());
    reopened.delete_project(&project.id).unwrap();
    reopened
        .with_connection(|c| {
            assert_eq!(
                c.query_row("SELECT count(*) FROM assets", [], |r| r.get::<_, u32>(0))
                    .unwrap(),
                0
            );
            Ok(())
        })
        .unwrap();
}
#[test]
fn invalid_backups_do_not_replace_current_data() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path().join("data.sqlite3"));
    let project = db.save_project(None, input("不可丢失")).unwrap();
    let invalid = dir.path().join("invalid.sqlite3");
    fs::write(&invalid, "not sqlite").unwrap();
    assert!(db.restore_backup(invalid.to_str().unwrap()).is_err());
    assert!(db.restore_backup(db.path.to_str().unwrap()).is_err());
    let backup = db.create_backup(dir.path().to_str().unwrap()).unwrap();
    let c = Connection::open(&backup).unwrap();
    c.execute_batch(
        "CREATE TRIGGER unexpected AFTER INSERT ON assets BEGIN DELETE FROM book_projects; END;",
    )
    .unwrap();
    drop(c);
    assert!(db.restore_backup(&backup).unwrap_err().contains("结构"));
    assert_eq!(db.get_project(&project.id).unwrap().input.title, "不可丢失");
    assert!(!dir.path().join("backups").exists());
}

#[test]
fn every_supported_backup_restores_without_changing_source_and_can_be_undone() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path().join("current.sqlite3"));
    for version in 2..=MIGRATIONS.len() {
        let path = dir.path().join(format!("v{version}.sqlite3"));
        let c = Connection::open(&path).unwrap();
        for sql in MIGRATIONS.iter().take(version) {
            c.execute_batch(sql).unwrap();
        }
        c.pragma_update(None, "user_version", version as u32)
            .unwrap();
        c.execute_batch("INSERT INTO book_projects(id,title,source_notes,target_duration_seconds,status,workspace_path,created_at,updated_at) VALUES('old','旧书','中文资料',600,'draft','missing','original','original'); INSERT INTO assets VALUES('video','old','video','missing.mp4','missing.mp4',10,0,'original');").unwrap();
        drop(c);
        let source = fs::read(&path).unwrap();
        let current = db.save_project(None, input("恢复前项目")).unwrap();
        let previous = db.restore_backup(path.to_str().unwrap()).unwrap();
        assert_eq!(fs::read(&path).unwrap(), source);
        assert_eq!(
            db.get_project("old").unwrap().input.source_notes,
            "中文资料"
        );
        assert_eq!(db.get_project("old").unwrap().updated_at, "original");
        assert_eq!(db.list_assets("old").unwrap()[0].exists_status, "missing");
        assert!(db.get_project(&current.id).is_err());
        db.with_connection(|c| {
            assert_eq!(
                c.pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
                    .unwrap(),
                MIGRATIONS.len() as u32
            );
            assert_eq!(
                c.query_row("PRAGMA integrity_check", [], |r| r.get::<_, String>(0))
                    .unwrap(),
                "ok"
            );
            assert!(!c
                .prepare("PRAGMA foreign_key_check")
                .unwrap()
                .exists([])
                .unwrap());
            Ok(())
        })
        .unwrap();
        db.restore_backup(&previous).unwrap();
        assert_eq!(
            db.get_project(&current.id).unwrap().input.title,
            "恢复前项目"
        );
    }
}
