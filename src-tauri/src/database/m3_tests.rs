use super::covers::CoverConfig;
use super::*;
use super::{
    image_generation::ImageGenerationInput,
    m2_tests::{model, Vault},
};
use crate::ai::images::{test_jpeg, test_png};
use std::fs;

fn image_model(db: &Database) -> ai::ProviderConfig {
    let mut input = model();
    input.model_kind = "image".into();
    input.is_default = true;
    db.save_provider(None, input, None, &Vault::default())
        .unwrap()
        .provider
}
fn image_task(db: &Database, project_id: &str, provider_id: &str) -> String {
    db.start_cover_generation(ImageGenerationInput {
        project_id: project_id.into(),
        provider_id: provider_id.into(),
        description: "安静的水墨树林".into(),
    })
    .unwrap()
}

#[test]
fn image_and_text_defaults_are_independent_and_wrong_task_types_are_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path().join("db.sqlite3"));
    let p = project(&db);
    let vault = Vault::default();
    let text = db
        .save_provider(None, model(), None, &vault)
        .unwrap()
        .provider;
    let image = image_model(&db);
    assert!(db.get_provider(&text.id).unwrap().input.is_default);
    assert!(db.get_provider(&image.id).unwrap().input.is_default);
    assert!(db
        .start_generation(ai::GenerationInput {
            project_id: p.id.clone(),
            provider_id: image.id.clone(),
            revision: None,
            target_duration_seconds: None,
            recommend_duration: false,
        })
        .is_err());
    assert!(db
        .start_cover_generation(ImageGenerationInput {
            project_id: p.id.clone(),
            provider_id: text.id,
            description: "树林".into()
        })
        .is_err());
    let task = image_task(&db, &p.id, &image.id);
    assert!(db.prepare_generation(&task).is_err());
    let mut changed = image.input;
    changed.model_kind = "text".into();
    assert!(db
        .save_provider(Some(&image.id), changed, None, &vault)
        .is_err());
    assert!(db.delete_provider(&image.id, &vault).is_err());
    let prepared = db.prepare_cover_generation(&task).unwrap();
    assert!(prepared.prompt.contains("安静的水墨树林"));
    assert!(prepared.prompt.contains(&p.input.title));
    db.end_generation(&task, "cancelled", "cancelled", "取消")
        .unwrap();
    assert!(db.finish_cover_generation(&task, &test_png()).is_err());
    assert!(db.list_assets(&p.id).unwrap().is_empty());
}

#[test]
fn generated_background_persists_with_history_and_backup_without_overwriting_files() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("db.sqlite3");
    let db = Database::open(path.clone());
    let p = project(&db);
    let image = image_model(&db);
    let bytes = test_jpeg();
    let task = image_task(&db, &p.id, &image.id);
    db.prepare_cover_generation(&task).unwrap();
    let asset = db.finish_cover_generation(&task, &bytes).unwrap();
    assert_eq!(asset.original_name, "M3测试书-1比1-4x4-背景-v1.jpg");
    assert_eq!(
        PathBuf::from(&asset.file_path).file_name().unwrap(),
        asset.original_name.as_str()
    );
    assert_eq!(db.read_cover_background(&p.id, &asset.id).unwrap(), bytes);
    assert!(db.finish_cover_generation(&task, &bytes).is_err());
    let mut draft = config();
    draft.background_asset_id = Some(asset.id.clone());
    db.save_cover_version(&p.id, draft.clone(), Some(&png()))
        .unwrap();
    let backup = db.create_backup(dir.path().to_str().unwrap()).unwrap();
    let next = image_task(&db, &p.id, &image.id);
    db.prepare_cover_generation(&next).unwrap();
    let second = db.finish_cover_generation(&next, &bytes).unwrap();
    assert_eq!(second.original_name, "M3测试书-1比1-4x4-背景-v2.jpg");
    assert_ne!(asset.file_path, second.file_path);
    db.restore_backup(&backup).unwrap();
    drop(db);
    let db = Database::open(path);
    assert_eq!(db.list_cover_versions(&p.id).unwrap()[0].config, draft);
    assert_eq!(db.list_assets(&p.id).unwrap().len(), 1);
    assert_eq!(fs::read(asset.file_path).unwrap(), bytes);
    assert_eq!(db.list_ai_tasks(&p.id).unwrap()[0].status, "succeeded");
    db.with_connection(|c| {
        assert!(!c
            .prepare("PRAGMA foreign_key_check")
            .unwrap()
            .exists([])
            .unwrap());
        Ok(())
    })
    .unwrap();
    let next = image_task(&db, &p.id, &image.id);
    db.prepare_cover_generation(&next).unwrap();
    let third = db.finish_cover_generation(&next, &bytes).unwrap();
    assert_eq!(third.original_name, "M3测试书-1比1-4x4-背景-v3.jpg");
    assert_eq!(fs::read(second.file_path).unwrap(), bytes);
}

#[test]
fn generated_background_filenames_use_actual_dimensions_and_format() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path().join("db.sqlite3"));
    let p = project(&db);
    let provider = image_model(&db);
    for (format, extension) in [
        (image::ImageFormat::Png, "png"),
        (image::ImageFormat::Jpeg, "jpg"),
        (image::ImageFormat::WebP, "webp"),
    ] {
        let image = image::RgbImage::from_pixel(12, 8, image::Rgb([60, 90, 120]));
        let mut bytes = std::io::Cursor::new(Vec::new());
        image.write_to(&mut bytes, format).unwrap();
        let task = image_task(&db, &p.id, &provider.id);
        db.prepare_cover_generation(&task).unwrap();
        let asset = db.finish_cover_generation(&task, bytes.get_ref()).unwrap();
        assert_eq!(
            asset.original_name,
            format!("M3测试书-3比2-12x8-背景-v1.{extension}")
        );
        assert_eq!(
            db.read_cover_background(&p.id, &asset.id).unwrap(),
            *bytes.get_ref()
        );
    }
}

#[test]
fn image_write_failure_invalid_response_and_database_failure_preserve_old_cover() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path().join("db.sqlite3"));
    let p = project(&db);
    let image = image_model(&db);
    db.save_cover_version(&p.id, config(), None).unwrap();
    let task = image_task(&db, &p.id, &image.id);
    db.prepare_cover_generation(&task).unwrap();
    assert!(db.finish_cover_generation(&task, b"invalid").is_err());
    let covers = PathBuf::from(&p.workspace_path).join("covers");
    fs::create_dir_all(covers.parent().unwrap()).unwrap();
    fs::write(&covers, b"keep").unwrap();
    assert!(db.finish_cover_generation(&task, &test_png()).is_err());
    assert_eq!(fs::read(&covers).unwrap(), b"keep");
    fs::remove_file(&covers).unwrap();
    db.with_connection(|c| { c.execute_batch("CREATE TRIGGER fail_image BEFORE INSERT ON assets BEGIN SELECT RAISE(ABORT,'test failure'); END;").unwrap(); Ok(()) }).unwrap();
    assert!(db.finish_cover_generation(&task, &test_png()).is_err());
    assert_eq!(fs::read_dir(covers).unwrap().count(), 0);
    assert!(db.list_assets(&p.id).unwrap().is_empty());
    assert_eq!(db.list_cover_versions(&p.id).unwrap().len(), 1);
    db.recover_ai_tasks().unwrap();
    assert_eq!(db.list_ai_tasks(&p.id).unwrap()[0].status, "failed");
}

#[test]
fn v6_upgrade_and_v5_backup_keep_existing_models_credentials_and_covers() {
    let dir = tempfile::tempdir().unwrap();
    let old = dir.path().join("v5.sqlite3");
    let c = Connection::open(&old).unwrap();
    for sql in MIGRATIONS.iter().take(5) {
        c.execute_batch(sql).unwrap();
    }
    c.execute_batch("PRAGMA user_version=5; INSERT INTO model_providers VALUES('old','old','openai-compatible','https://example.com/v1','text',1,1,'keep-credential','created','updated');").unwrap();
    // Build the historical fixture with its own schema rather than current application APIs.
    c.execute("INSERT INTO book_projects(id,title,source_notes,target_duration_seconds,status,workspace_path,created_at,updated_at) VALUES('p','M3测试书','保留笔记',600,'draft',?1,'created','updated')",[dir.path().to_string_lossy().as_ref()]).unwrap();
    c.execute(
        "INSERT INTO cover_versions VALUES('cover','p',1,?1,NULL,'created')",
        [serde_json::to_string(&config()).unwrap()],
    )
    .unwrap();
    drop(c);
    let original = fs::read(&old).unwrap();
    let current = dir.path().join("current.sqlite3");
    fs::copy(&old, &current).unwrap();
    let db = Database::open(current);
    let model = db.get_provider("old").unwrap();
    assert_eq!(model.input.model_kind, "text");
    assert_eq!(model.credential_ref.as_deref(), Some("keep-credential"));
    assert!(model.input.is_default);
    assert_eq!(db.list_cover_versions("p").unwrap()[0].id, "cover");
    image_model(&db);
    db.restore_backup(old.to_str().unwrap()).unwrap();
    assert_eq!(fs::read(&old).unwrap(), original);
    assert_eq!(
        db.get_provider("old").unwrap().credential_ref.as_deref(),
        Some("keep-credential")
    );
    assert_eq!(db.list_cover_versions("p").unwrap()[0].config, config());
}

#[test]
fn v6_migration_failure_rolls_back_new_columns_and_keeps_v5() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("db.sqlite3");
    let c = Connection::open(&path).unwrap();
    for sql in MIGRATIONS.iter().take(5) {
        c.execute_batch(sql).unwrap();
    }
    c.execute_batch("PRAGMA user_version=5; CREATE INDEX one_default_model_per_kind ON model_providers(name); INSERT INTO m0_probe VALUES(1,'keep','time');").unwrap();
    drop(c);
    assert!(Database::open(path.clone()).read().is_err());
    let c = Connection::open(path).unwrap();
    assert_eq!(
        c.pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
            .unwrap(),
        5
    );
    assert_eq!(
        c.query_row("SELECT note FROM m0_probe", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "keep"
    );
    assert!(c.prepare("SELECT model_kind FROM model_providers").is_err());
}

fn config() -> CoverConfig {
    serde_json::from_value(serde_json::json!({
        "schemaVersion":1,"templateId":"classic","width":1080,"height":1440,
        "title":"中文封面","subtitle":"十分钟读懂一本书","brand":"甲子 · 书引",
        "fontFamily":"serif","titleSize":112,"subtitleSize":44,"align":"left","position":"center",
        "backgroundColor":"#f4efdf","textColor":"#24362f","accentColor":"#9a5938",
        "backgroundAssetId":null,"overlayOpacity":0.45
    }))
    .unwrap()
}
fn project(db: &Database) -> Project {
    db.save_project(
        None,
        serde_json::from_value(serde_json::json!({
            "title":"M3测试书","author":"","publisher":"","isbn":"","description":"",
            "sourceNotes":"保留笔记","theme":"","targetAudience":"","contentStyle":"",
            "targetDurationSeconds":600,"status":"draft"
        }))
        .unwrap(),
    )
    .unwrap()
}
// Envelope fixture for file/transaction tests. Real Canvas PNG decoding is covered by desktop QA.
fn png() -> Vec<u8> {
    let mut bytes = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
    bytes.extend(1080u32.to_be_bytes());
    bytes.extend(1440u32.to_be_bytes());
    bytes.extend([0; 9]);
    bytes.extend(b"\0\0\0\0IEND\xaeB`\x82");
    bytes
}
#[test]
fn covers_export_and_restore_all_three_supported_sizes() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path().join("db.sqlite3"));
    let p = project(&db);
    for (width, height) in [(1440, 1080), (1080, 1440), (1920, 1080)] {
        let mut draft = config();
        draft.width = width;
        draft.height = height;
        let mut bytes = png();
        bytes[16..20].copy_from_slice(&width.to_be_bytes());
        bytes[20..24].copy_from_slice(&height.to_be_bytes());
        let saved = db.save_cover_version(&p.id, draft, Some(&bytes)).unwrap();
        let path = saved.exported_path.unwrap();
        let ratio = match (width, height) {
            (1440, 1080) => "4比3",
            (1080, 1440) => "3比4",
            _ => "16比9",
        };
        assert_eq!(
            PathBuf::from(&path).file_name().unwrap(),
            format!(
                "M3测试书-{ratio}-{width}x{height}-v{}.png",
                saved.version_no
            )
            .as_str()
        );
        assert_eq!(fs::read(path).unwrap(), bytes);
    }
    for (width, height) in [(1080, 1920), (1280, 720), (1080, 1080), (1440, 1440)] {
        let mut draft = config();
        draft.width = width;
        draft.height = height;
        assert!(db.save_cover_version(&p.id, draft, None).is_err());
    }
    drop(db);
    let db = Database::open(dir.path().join("db.sqlite3"));
    let versions = db.list_cover_versions(&p.id).unwrap();
    assert_eq!(versions.len(), 3);
    assert_eq!(
        (versions[0].config.width, versions[0].config.height),
        (1920, 1080)
    );
}

#[test]
fn cover_filenames_use_current_book_title_and_skip_existing_versions() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path().join("db.sqlite3"));
    let p = project(&db);
    let covers = PathBuf::from(&p.workspace_path).join("covers");
    fs::create_dir_all(&covers).unwrap();
    let occupied = covers.join("M3测试书-3比4-1080x1440-v1.png");
    fs::write(&occupied, b"existing cover").unwrap();
    fs::create_dir(covers.join("M3测试书-3比4-1080x1440-v2.png")).unwrap();
    let saved = db
        .save_cover_version(&p.id, config(), Some(&png()))
        .unwrap();
    assert_eq!(saved.version_no, 3);
    assert_eq!(
        PathBuf::from(saved.exported_path.unwrap())
            .file_name()
            .unwrap(),
        "M3测试书-3比4-1080x1440-v3.png"
    );
    assert_eq!(fs::read(occupied).unwrap(), b"existing cover");
    assert_eq!(db.read_cover_export(&p.id, &saved.id).unwrap(), png());

    let long_title = "书".repeat(200);
    let shortened_title = "书".repeat(40);
    for (title, title_part) in [
        ("围城", "围城"),
        ("书名<>:\"/\\|?*\n结尾. ", "书名__________结尾"),
        ("Con.txt", "_Con.txt"),
        ("..", "未命名书籍"),
        (long_title.as_str(), shortened_title.as_str()),
    ] {
        let mut input = p.input.clone();
        input.title = title.into();
        db.save_project(Some(&p.id), input).unwrap();
        let saved = db
            .save_cover_version(&p.id, config(), Some(&png()))
            .unwrap();
        let path = PathBuf::from(saved.exported_path.unwrap());
        assert_eq!(
            path.file_name().unwrap(),
            format!("{title_part}-3比4-1080x1440-v{}.png", saved.version_no).as_str()
        );
        assert_eq!(path.parent().unwrap(), covers);
        assert_eq!(db.read_cover_export(&p.id, &saved.id).unwrap(), png());
    }
}

#[test]
fn legacy_random_cover_paths_remain_readable_after_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("db.sqlite3");
    let db = Database::open(db_path.clone());
    let p = project(&db);
    let saved = db
        .save_cover_version(&p.id, config(), Some(&png()))
        .unwrap();
    let original = PathBuf::from(saved.exported_path.unwrap());
    let legacy = original.with_file_name("cover-1080x1440-v1-78c71cba81e04bbaa09c1345c18f39e0.png");
    fs::rename(original, &legacy).unwrap();
    db.with_connection(|c| {
        c.execute(
            "UPDATE cover_versions SET exported_path=?1 WHERE id=?2",
            rusqlite::params![legacy.to_str().unwrap(), saved.id],
        )
        .unwrap();
        Ok(())
    })
    .unwrap();
    drop(db);
    let db = Database::open(db_path);
    assert_eq!(db.read_cover_export(&p.id, &saved.id).unwrap(), png());
    let next = db
        .save_cover_version(&p.id, config(), Some(&png()))
        .unwrap();
    assert_eq!(
        PathBuf::from(next.exported_path.unwrap())
            .file_name()
            .unwrap(),
        "M3测试书-3比4-1080x1440-v2.png"
    );
    assert_eq!(fs::read(legacy).unwrap(), png());
}

#[test]
fn v5_migration_preserves_v4_and_restores_old_backup_without_rewriting_it() {
    let dir = tempfile::tempdir().unwrap();
    let old = dir.path().join("v4.sqlite3");
    let c = Connection::open(&old).unwrap();
    for sql in MIGRATIONS.iter().take(4) {
        c.execute_batch(sql).unwrap();
    }
    c.execute_batch("INSERT INTO m0_probe VALUES(1,'原文','原时间'); PRAGMA user_version=4;")
        .unwrap();
    drop(c);
    let original = fs::read(&old).unwrap();
    let current = dir.path().join("current.sqlite3");
    fs::copy(&old, &current).unwrap();
    let db = Database::open(current.clone());
    assert_eq!(db.read().unwrap().record.unwrap().updated_at, "原时间");
    let p = project(&db);
    db.save_cover_version(&p.id, config(), None).unwrap();
    let v5 = db.create_backup(dir.path().to_str().unwrap()).unwrap();
    let before = db.restore_backup(old.to_str().unwrap()).unwrap();
    assert!(PathBuf::from(before).exists());
    assert_eq!(fs::read(old).unwrap(), original);
    assert!(db.list_projects("", "").unwrap().is_empty());
    db.restore_backup(&v5).unwrap();
    assert_eq!(db.list_cover_versions(&p.id).unwrap()[0].config, config());
    drop(db);
    let db = Database::open(current);
    assert_eq!(db.list_cover_versions(&p.id).unwrap().len(), 1);
    db.with_connection(|c| {
        assert_eq!(
            c.pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
                .unwrap(),
            MIGRATIONS.len() as u32
        );
        assert!(!c
            .prepare("PRAGMA foreign_key_check")
            .unwrap()
            .exists([])
            .unwrap());
        Ok(())
    })
    .unwrap();
}
#[test]
fn v5_conflict_rolls_back_without_resetting_existing_data() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("conflict.sqlite3");
    let c = Connection::open(&path).unwrap();
    for sql in MIGRATIONS.iter().take(4) {
        c.execute_batch(sql).unwrap();
    }
    c.execute_batch("CREATE TABLE cover_versions(original TEXT); INSERT INTO cover_versions VALUES('keep'); PRAGMA user_version=4;").unwrap();
    drop(c);
    assert!(Database::open(path.clone()).read().is_err());
    let c = Connection::open(path).unwrap();
    assert_eq!(
        c.pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
            .unwrap(),
        4
    );
    assert_eq!(
        c.query_row("SELECT original FROM cover_versions", [], |r| r
            .get::<_, String>(0))
            .unwrap(),
        "keep"
    );
}
#[test]
fn cover_versions_exports_and_sources_survive_reopen_and_project_delete() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("db.sqlite3");
    let db = Database::open(path.clone());
    let p = project(&db);
    let source = dir.path().join("背景.png");
    fs::write(&source, png()).unwrap();
    let asset_id = db
        .link_asset(&p.id, source.to_str().unwrap(), None)
        .unwrap();
    let mut first = config();
    first.background_asset_id = Some(asset_id);
    let v1 = db
        .save_cover_version(&p.id, first.clone(), Some(&png()))
        .unwrap();
    let mut second = config();
    second.title = "修订后".into();
    let v2 = db
        .save_cover_version(&p.id, second.clone(), Some(&png()))
        .unwrap();
    assert_ne!(v1.exported_path, v2.exported_path);
    assert_eq!(v2.version_no, 2);
    drop(db);
    let db = Database::open(path);
    let all = db.list_cover_versions(&p.id).unwrap();
    assert_eq!(all[0].config, second);
    assert_eq!(all[1].config, first);
    db.delete_project(&p.id).unwrap();
    for file in [
        v1.exported_path.unwrap(),
        v2.exported_path.unwrap(),
        source.to_string_lossy().into_owned(),
    ] {
        assert_eq!(fs::read(file).unwrap(), png());
    }
    db.with_connection(|c| {
        assert_eq!(
            c.query_row("SELECT count(*) FROM cover_versions", [], |r| r
                .get::<_, u32>(0))
                .unwrap(),
            0
        );
        Ok(())
    })
    .unwrap();
}
#[test]
fn failed_exports_keep_saved_versions_and_remove_only_new_uncommitted_file() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path().join("db.sqlite3"));
    let p = project(&db);
    let saved = db
        .save_cover_version(&p.id, config(), Some(&png()))
        .unwrap();
    let mut invalid = config();
    invalid.width = 1920;
    assert!(db.save_cover_version(&p.id, invalid, Some(&png())).is_err());
    let mut wrong_size = png();
    wrong_size[19] = 0;
    assert!(db
        .save_cover_version(&p.id, config(), Some(&wrong_size))
        .is_err());
    db.with_connection(|c| { c.execute_batch("CREATE TRIGGER fail_cover BEFORE INSERT ON cover_versions BEGIN SELECT RAISE(ABORT,'test rollback'); END;").unwrap(); Ok(()) }).unwrap();
    assert!(db
        .save_cover_version(&p.id, config(), Some(&png()))
        .is_err());
    assert_eq!(db.list_cover_versions(&p.id).unwrap().len(), 1);
    assert_eq!(fs::read(saved.exported_path.unwrap()).unwrap(), png());
    assert_eq!(
        fs::read_dir(PathBuf::from(&p.workspace_path).join("covers"))
            .unwrap()
            .count(),
        1
    );
    let another = project(&db);
    let blocked = PathBuf::from(&another.workspace_path).join("covers");
    fs::write(blocked, b"existing file").unwrap();
    assert!(db
        .save_cover_version(&another.id, config(), Some(&png()))
        .is_err());
}
#[test]
fn backgrounds_check_project_format_size_and_missing_file_without_losing_config() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path().join("db.sqlite3"));
    let p = project(&db);
    let other = project(&db);
    let source = dir.path().join("背景.png");
    fs::write(&source, png()).unwrap();
    let id = db
        .link_asset(&p.id, source.to_str().unwrap(), None)
        .unwrap();
    assert_eq!(db.read_cover_background(&p.id, &id).unwrap(), png());
    assert!(db.read_cover_background(&other.id, &id).is_err());
    let mut draft = config();
    draft.background_asset_id = Some(id.clone());
    assert!(db
        .save_cover_version(&other.id, draft.clone(), None)
        .is_err());
    fs::write(&source, b"not an image").unwrap();
    assert!(db.read_cover_background(&p.id, &id).is_err());
    fs::File::create(&source)
        .unwrap()
        .set_len(15 * 1024 * 1024 + 1)
        .unwrap();
    assert!(db.read_cover_background(&p.id, &id).is_err());
    fs::remove_file(&source).unwrap();
    assert!(db.read_cover_background(&p.id, &id).is_err());
    db.save_cover_version(&p.id, draft.clone(), None).unwrap();
    assert_eq!(db.list_cover_versions(&p.id).unwrap()[0].config, draft);
}
