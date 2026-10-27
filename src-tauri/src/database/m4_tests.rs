use super::{
    covers::CoverConfig,
    publishing::{PublishInput, PublishRecordInput},
    *,
};
use std::{fs, path::Path};

pub(super) const PORTRAIT_VIDEO: &[u8] = include_bytes!("../../tests/fixtures/portrait.mp4");

fn landscape_video() -> Vec<u8> {
    let mut bytes = PORTRAIT_VIDEO.to_vec();
    let marker = bytes
        .windows(4)
        .position(|window| window == b"tkhd")
        .expect("fixture contains a video track header");
    let start = marker - 4;
    let size = u32::from_be_bytes(bytes[start..start + 4].try_into().unwrap()) as usize;
    let end = start + size;
    bytes[end - 8..end - 4].copy_from_slice(&(1920u32 << 16).to_be_bytes());
    bytes[end - 4..end].copy_from_slice(&(1080u32 << 16).to_be_bytes());
    bytes
}

pub(super) fn project(db: &Database) -> Project {
    db.save_project(None,serde_json::from_value(serde_json::json!({"title":"中文书籍","author":"作者","publisher":"","isbn":"","description":"","sourceNotes":"原始资料","theme":"","targetAudience":"","contentStyle":"","targetDurationSeconds":600,"status":"draft"})).unwrap()).unwrap()
}
fn cover(db: &Database, id: &str, width: u32, height: u32) -> String {
    let config:CoverConfig = serde_json::from_value(serde_json::json!({"schemaVersion":1,"templateId":"classic","width":width,"height":height,"title":"中文标题","subtitle":"读书","brand":"甲子","fontFamily":"sans","titleSize":112,"subtitleSize":44,"align":"left","position":"center","backgroundColor":"#ffffff","textColor":"#24362f","accentColor":"#9a5938","backgroundAssetId":null,"overlayOpacity":0.45})).unwrap();
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, width, height);
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(&vec![180; (width * height * 3) as usize])
            .unwrap();
    }
    db.save_cover_version(id, config, Some(&bytes)).unwrap().id
}
pub(super) fn input(db: &Database, project: &Project) -> PublishInput {
    let path = Path::new(&project.workspace_path).join("中文 视频.mp4");
    fs::write(&path, PORTRAIT_VIDEO).unwrap();
    let audio = Path::new(&project.workspace_path).join("中文 音频.mp3");
    fs::write(&audio, b"original audio").unwrap();
    PublishInput {
        writing_version_id: None,
        project_id: project.id.clone(),
        title: "中文发布标题".into(),
        description: "第一行\n第二行：保留换行".into(),
        tags: "#读书 #中文".into(),
        video_asset_id: Some(
            db.link_asset(&project.id, path.to_str().unwrap(), None)
                .unwrap(),
        ),
        audio_asset_id: Some(
            db.link_asset(&project.id, audio.to_str().unwrap(), None)
                .unwrap(),
        ),
        landscape_cover_id: Some(cover(db, &project.id, 1440, 1080)),
        portrait_cover_id: Some(cover(db, &project.id, 1080, 1440)),
        title_confirmed: true,
        covers_confirmed: true,
    }
}
fn record() -> PublishRecordInput {
    PublishRecordInput {
        confirmed: true,
        published_at: "2026-10-24T02:00:00Z".into(),
        post_url: "https://www.douyin.com/video/123".into(),
        notes: "用户确认".into(),
    }
}

#[test]
fn publish_queue_rechecks_files_and_keeps_each_books_task_and_record() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path().join("db.sqlite3"));
    let first_book = project(&db);
    let mut second_book_input = first_book.input.clone();
    second_book_input.title = "另一本书".into();
    let second_book = db.save_project(None, second_book_input).unwrap();
    let first = db.save_publish_task(None, input(&db, &first_book)).unwrap();
    db.prepare_publish_task(&first.id).unwrap();
    let second = db
        .save_publish_task(None, input(&db, &second_book))
        .unwrap();
    db.prepare_publish_task(&second.id).unwrap();
    db.record_publish_success(&second.id, record()).unwrap();
    let queue = db.list_publish_queue().unwrap();
    assert_eq!(queue.len(), 2);
    let item = queue.iter().find(|item| item.task.id == second.id).unwrap();
    assert_eq!(item.project_title, "另一本书");
    assert_eq!(item.task.input.project_id, second_book.id);
    assert_eq!(item.task.status, "published");
    assert_eq!(
        item.task.record.as_ref().unwrap().post_url,
        record().post_url
    );
    let manifest = db.publish_snapshot(&first.id).unwrap();
    fs::remove_file(&manifest.landscape).unwrap();
    let queue = db.list_publish_queue().unwrap();
    assert_eq!(
        queue
            .iter()
            .find(|item| item.task.id == first.id)
            .unwrap()
            .task
            .status,
        "draft"
    );
    assert_eq!(
        queue
            .iter()
            .find(|item| item.task.id == second.id)
            .unwrap()
            .task
            .status,
        "published"
    );
    let backup = db.create_backup(dir.path().to_str().unwrap()).unwrap();
    db.restore_backup(&backup).unwrap();
    assert_eq!(
        serde_json::to_value(db.list_publish_queue().unwrap()).unwrap(),
        serde_json::to_value(queue).unwrap()
    );
}

#[test]
fn upload_snapshot_uses_selected_task_and_package_covers() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path().join("db.sqlite3"));
    let p = project(&db);
    let i = input(&db, &p);
    let task = db.save_publish_task(None, i.clone()).unwrap();
    assert!(db.publish_snapshot(&task.id).is_err());
    let ready = db.prepare_publish_task(&task.id).unwrap();
    let snapshot = db.publish_snapshot(&task.id).unwrap();
    assert_eq!(snapshot.task_id, task.id);
    assert_eq!(snapshot.input, i);
    assert_eq!(snapshot.project_title, p.input.title);
    let pack = Path::new(ready.task_directory.as_ref().unwrap());
    assert_eq!(
        Path::new(&snapshot.landscape),
        pack.join("cover-landscape.png")
    );
    assert_eq!(
        Path::new(&snapshot.portrait),
        pack.join("cover-portrait.png")
    );
    assert_eq!(fs::read(&snapshot.video).unwrap(), PORTRAIT_VIDEO);

    let other = project(&db);
    let mut different = input(&db, &other);
    different.title = "另一本书的发布标题".into();
    let second = db.save_publish_task(None, different).unwrap();
    db.prepare_publish_task(&second.id).unwrap();
    assert_eq!(db.publish_snapshot(&task.id).unwrap(), snapshot);
    assert_ne!(
        db.publish_snapshot(&second.id).unwrap().video,
        snapshot.video
    );

    fs::write(pack.join("cover-landscape.png"), b"damaged").unwrap();
    assert!(db.publish_snapshot(&task.id).is_err());
    assert_eq!(db.list_publish_tasks(&p.id).unwrap()[0].status, "draft");
    db.prepare_publish_task(&task.id).unwrap();
    db.record_publish_success(&task.id, record()).unwrap();
    assert!(db.publish_snapshot(&task.id).is_err());
    assert_eq!(fs::read(&snapshot.video).unwrap(), PORTRAIT_VIDEO);
}

#[test]
fn incomplete_and_foreign_materials_cannot_be_ready() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path().join("db.sqlite3"));
    let p = project(&db);
    let mut i = input(&db, &p);
    i.title_confirmed = false;
    i.covers_confirmed = false;
    let t = db.save_publish_task(None, i.clone()).unwrap();
    assert!(db.prepare_publish_task(&t.id).unwrap_err().contains("确认"));
    assert_eq!(db.list_publish_tasks(&p.id).unwrap()[0].status, "draft");
    i.title_confirmed = true;
    i.covers_confirmed = true;
    let other = project(&db);
    let mut foreign = i.clone();
    foreign.project_id = other.id;
    assert!(db.save_publish_task(None, foreign).is_err());
    let mut wrong = i.clone();
    wrong.landscape_cover_id = i.portrait_cover_id.clone();
    assert!(db.save_publish_task(None, wrong).is_err());
    i.title = "长".repeat(31);
    assert!(db.save_publish_task(Some(&t.id), i).is_err());
    assert_eq!(
        db.list_publish_tasks(&p.id).unwrap()[0].input.title,
        t.input.title
    );
}

#[test]
fn audio_and_final_video_checks_are_required() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path().join("db.sqlite3"));
    let p = project(&db);
    let original = input(&db, &p);
    let mut missing_audio = original.clone();
    missing_audio.audio_asset_id = None;
    let task = db.save_publish_task(None, missing_audio).unwrap();
    assert!(db
        .prepare_publish_task(&task.id)
        .unwrap_err()
        .contains("音频成品"));
    let bad = Path::new(&p.workspace_path).join("不是成片.mp4");
    fs::write(&bad, b"not a video").unwrap();
    let mut bad_video = original;
    bad_video.video_asset_id = Some(db.link_asset(&p.id, bad.to_str().unwrap(), None).unwrap());
    let task = db.save_publish_task(None, bad_video).unwrap();
    assert!(db
        .prepare_publish_task(&task.id)
        .unwrap_err()
        .contains("最终成片"));
}

#[test]
fn non_portrait_final_video_with_audio_can_be_ready() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path().join("db.sqlite3"));
    let p = project(&db);
    let mut publish = input(&db, &p);
    let landscape = Path::new(&p.workspace_path).join("横版 成片.mp4");
    fs::write(&landscape, landscape_video()).unwrap();
    let metadata = crate::media::inspect(&landscape).unwrap();
    assert_eq!((metadata.width, metadata.height), (1920, 1080));
    assert!(metadata.has_audio);
    publish.video_asset_id = Some(
        db.link_asset(&p.id, landscape.to_str().unwrap(), None)
            .unwrap(),
    );
    let task = db.save_publish_task(None, publish).unwrap();
    let ready = db.prepare_publish_task(&task.id).unwrap();
    assert_eq!(ready.status, "ready");
    assert!(
        ready
            .checklist
            .iter()
            .find(|item| item.key == "video")
            .unwrap()
            .passed
    );
}

#[test]
fn package_is_complete_unique_and_preserves_sources_and_unicode() {
    let dir = tempfile::tempdir().unwrap();
    let dbpath = dir.path().join("db.sqlite3");
    let db = Database::open(dbpath.clone());
    let p = project(&db);
    let i = input(&db, &p);
    let t = db.save_publish_task(None, i.clone()).unwrap();
    let ready = db.prepare_publish_task(&t.id).unwrap();
    assert_eq!(ready.status, "ready");
    assert!(ready.checklist.iter().all(|c| c.passed));
    let pack = PathBuf::from(ready.task_directory.unwrap());
    assert_eq!(pack.file_name().unwrap(), "中文书籍-发布任务包");
    assert_eq!(fs::read_dir(&pack).unwrap().count(), 8);
    assert_eq!(
        fs::read_to_string(pack.join("description.txt")).unwrap(),
        i.description
    );
    assert_eq!(fs::read_to_string(pack.join("tags.txt")).unwrap(), i.tags);
    let video = fs::read_to_string(pack.join("video-path.txt")).unwrap();
    assert_eq!(fs::read(&video).unwrap(), PORTRAIT_VIDEO);
    assert!(!pack.join("中文 视频.mp4").exists());
    for (name, w, h) in [
        ("cover-landscape.png", 1440, 1080),
        ("cover-portrait.png", 1080, 1440),
    ] {
        let image = image::open(pack.join(name)).unwrap();
        assert_eq!((image.width(), image.height()), (w, h));
    }
    let next = db.prepare_publish_task(&t.id).unwrap();
    assert_eq!(
        Path::new(next.task_directory.as_ref().unwrap())
            .file_name()
            .unwrap(),
        "中文书籍-发布任务包 (2)"
    );
    assert_ne!(next.task_directory.as_deref(), pack.to_str());
    assert!(pack.is_dir());
    drop(db);
    let db = Database::open(dbpath);
    assert_eq!(db.list_publish_tasks(&p.id).unwrap()[0].status, "ready");
    db.delete_project(&p.id).unwrap();
    assert!(pack.is_dir());
    assert_eq!(fs::read(video).unwrap(), PORTRAIT_VIDEO);
}

#[test]
fn missing_relocated_changed_or_unlinked_materials_invalidate_ready_packages() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path().join("db.sqlite3"));
    let p = project(&db);
    let i = input(&db, &p);
    let task = db.save_publish_task(None, i.clone()).unwrap();
    let ready = db.prepare_publish_task(&task.id).unwrap();
    let pack = PathBuf::from(ready.task_directory.unwrap());
    let source = Path::new(&p.workspace_path).join("中文 视频.mp4");
    let moved = Path::new(&p.workspace_path).join("移动.mp4");
    fs::rename(&source, &moved).unwrap();
    assert_eq!(db.list_publish_tasks(&p.id).unwrap()[0].status, "draft");
    assert!(db.prepare_publish_task(&task.id).is_err());
    db.link_asset(&p.id, moved.to_str().unwrap(), i.video_asset_id.as_deref())
        .unwrap();
    assert_eq!(db.list_publish_tasks(&p.id).unwrap()[0].status, "draft");
    let ready = db.prepare_publish_task(&task.id).unwrap();
    assert_eq!(ready.status, "ready");
    assert!(fs::read_to_string(
        Path::new(ready.task_directory.as_ref().unwrap()).join("video-path.txt")
    )
    .unwrap()
    .ends_with("移动.mp4"));
    fs::write(
        Path::new(ready.task_directory.as_ref().unwrap()).join("title.txt"),
        "外部编辑",
    )
    .unwrap();
    assert_eq!(db.list_publish_tasks(&p.id).unwrap()[0].status, "draft");
    db.prepare_publish_task(&task.id).unwrap();
    db.unlink_asset(i.video_asset_id.as_ref().unwrap()).unwrap();
    let current = db.list_publish_tasks(&p.id).unwrap().remove(0);
    assert!(current.input.video_asset_id.is_none());
    assert_eq!(current.status, "draft");
    assert!(pack.is_dir());
    assert_eq!(fs::read(moved).unwrap(), PORTRAIT_VIDEO);
}

#[test]
fn package_io_and_commit_failure_keep_draft_sources_and_previous_package() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path().join("db.sqlite3"));
    let p = project(&db);
    let i = input(&db, &p);
    let task = db.save_publish_task(None, i.clone()).unwrap();
    let ready = db.prepare_publish_task(&task.id).unwrap();
    let previous = ready.task_directory.unwrap();
    db.with_connection(|c|{ c.execute_batch("CREATE TRIGGER fail_package BEFORE UPDATE ON publish_tasks WHEN NEW.status='ready' BEGIN SELECT RAISE(ABORT,'write rejected'); END;").unwrap(); Ok(()) }).unwrap();
    assert!(db
        .prepare_publish_task(&task.id)
        .unwrap_err()
        .contains("write rejected"));
    assert_eq!(
        fs::read_dir(Path::new(&p.workspace_path).join("publish"))
            .unwrap()
            .count(),
        1
    );
    assert!(Path::new(&previous).exists());
    assert_eq!(db.list_publish_tasks(&p.id).unwrap()[0].status, "ready");
    let mut edit = i;
    edit.title = "修改后".into();
    db.save_publish_task(Some(&task.id), edit).unwrap();
    assert_eq!(db.list_publish_tasks(&p.id).unwrap()[0].status, "draft");
    let source = Path::new(&p.workspace_path).join("中文 视频.mp4");
    assert_eq!(fs::read(source).unwrap(), PORTRAIT_VIDEO);
    let p2 = project(&db);
    let i2 = input(&db, &p2);
    let t2 = db.save_publish_task(None, i2).unwrap();
    fs::write(
        Path::new(&p2.workspace_path).join("publish"),
        b"existing file",
    )
    .unwrap();
    assert!(db.prepare_publish_task(&t2.id).is_err());
    assert_eq!(
        fs::read(Path::new(&p2.workspace_path).join("publish")).unwrap(),
        b"existing file"
    );
}

#[test]
fn corrupt_cover_is_rejected_without_ready_or_lost_draft() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path().join("db.sqlite3"));
    let p = project(&db);
    let i = input(&db, &p);
    let t = db.save_publish_task(None, i).unwrap();
    let covers = db.list_cover_versions(&p.id).unwrap();
    fs::write(covers[0].exported_path.as_ref().unwrap(), b"invalid PNG").unwrap();
    assert!(db.prepare_publish_task(&t.id).is_err());
    assert_eq!(db.list_publish_tasks(&p.id).unwrap()[0].status, "draft");
    assert_eq!(
        db.list_publish_tasks(&p.id).unwrap()[0].input.title,
        t.input.title
    );
}

#[test]
fn changed_package_cover_with_same_size_cannot_remain_ready() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path().join("db.sqlite3"));
    let p = project(&db);
    let i = input(&db, &p);
    let task = db.save_publish_task(None, i).unwrap();
    for name in ["cover-landscape.png", "cover-portrait.png"] {
        let ready = db.prepare_publish_task(&task.id).unwrap();
        let copy = Path::new(ready.task_directory.as_ref().unwrap()).join(name);
        let original = fs::read(&copy).unwrap();
        let mut corrupt = original.clone();
        corrupt[0] ^= 0xff;
        fs::write(&copy, &corrupt).unwrap();
        assert_eq!(fs::metadata(&copy).unwrap().len(), original.len() as u64);
        let current = db.list_publish_tasks(&p.id).unwrap().remove(0);
        assert_eq!(current.status, "draft");
        assert!(
            !current
                .checklist
                .iter()
                .find(|c| c.key == "package")
                .unwrap()
                .passed
        );
        assert!(db.record_publish_success(&task.id, record()).is_err());
        assert_eq!(fs::read(&copy).unwrap(), corrupt);
        assert_eq!(db.list_cover_versions(&p.id).unwrap().len(), 2);
    }
    assert_eq!(db.prepare_publish_task(&task.id).unwrap().status, "ready");
}

#[test]
fn success_requires_confirmation_and_record_task_project_commit_together() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path().join("db.sqlite3"));
    let p = project(&db);
    let i = input(&db, &p);
    let t = db.save_publish_task(None, i.clone()).unwrap();
    assert!(db.record_publish_success(&t.id, record()).is_err());
    db.prepare_publish_task(&t.id).unwrap();
    let mut denied = record();
    denied.confirmed = false;
    assert!(db.record_publish_success(&t.id, denied).is_err());
    let mut invalid = record();
    invalid.post_url = "javascript:alert(1)".into();
    assert!(db.record_publish_success(&t.id, invalid).is_err());
    let mut invalid = record();
    invalid.published_at = "invalid".into();
    assert!(db.record_publish_success(&t.id, invalid).is_err());
    db.with_connection(|c|{ c.execute_batch("CREATE TRIGGER fail_record BEFORE UPDATE ON publish_tasks WHEN NEW.status='published' BEGIN SELECT RAISE(ABORT,'transaction rejected'); END;").unwrap(); Ok(()) }).unwrap();
    assert!(db.record_publish_success(&t.id, record()).is_err());
    assert!(db.list_publish_tasks(&p.id).unwrap()[0].record.is_none());
    assert_eq!(db.get_project(&p.id).unwrap().input.status, "draft");
    db.with_connection(|c| {
        c.execute_batch("DROP TRIGGER fail_record").unwrap();
        Ok(())
    })
    .unwrap();
    let done = db.record_publish_success(&t.id, record()).unwrap();
    assert_eq!(done.status, "published");
    assert_eq!(done.record.unwrap().notes, "用户确认");
    assert_eq!(db.get_project(&p.id).unwrap().input.status, "published");
    assert!(db.record_publish_success(&t.id, record()).is_err());
    assert!(db.save_publish_task(Some(&t.id), i).is_err());
    let backup = db.create_backup(dir.path().to_str().unwrap()).unwrap();
    db.delete_project(&p.id).unwrap();
    db.restore_backup(&backup).unwrap();
    let restored = db.list_publish_tasks(&p.id).unwrap();
    assert_eq!(restored[0].status, "published");
    assert_eq!(
        restored[0].record.as_ref().unwrap().post_url,
        record().post_url
    );
}

#[test]
fn v7_migration_old_backup_and_failure_preserve_v6_data() {
    let dir = tempfile::tempdir().unwrap();
    let old = dir.path().join("v6.sqlite3");
    let c = Connection::open(&old).unwrap();
    for sql in MIGRATIONS.iter().take(6) {
        c.execute_batch(sql).unwrap();
    }
    c.execute_batch("INSERT INTO m0_probe VALUES(1,'历史内容','原时间'); PRAGMA user_version=6;")
        .unwrap();
    drop(c);
    let before = fs::read(&old).unwrap();
    let current = dir.path().join("current.sqlite3");
    fs::copy(&old, &current).unwrap();
    let db = Database::open(current.clone());
    assert_eq!(db.read().unwrap().record.unwrap().updated_at, "原时间");
    let p = project(&db);
    let i = input(&db, &p);
    db.save_publish_task(None, i).unwrap();
    db.restore_backup(old.to_str().unwrap()).unwrap();
    assert!(db.list_projects("", "").unwrap().is_empty());
    assert_eq!(fs::read(&old).unwrap(), before);
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
    drop(db);
    let conflict = dir.path().join("conflict.sqlite3");
    fs::copy(&old, &conflict).unwrap();
    let c = Connection::open(&conflict).unwrap();
    c.execute_batch(
        "CREATE TABLE publish_records(original TEXT); INSERT INTO publish_records VALUES('keep');",
    )
    .unwrap();
    drop(c);
    assert!(Database::open(conflict.clone()).read().is_err());
    let c = Connection::open(conflict).unwrap();
    assert_eq!(
        c.pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
            .unwrap(),
        6
    );
    assert_eq!(
        c.query_row("SELECT original FROM publish_records", [], |r| r
            .get::<_, String>(0))
            .unwrap(),
        "keep"
    );
    assert!(!c
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='publish_tasks')",
            [],
            |r| r.get::<_, bool>(0)
        )
        .unwrap());
}

#[test]
fn three_book_workflows_keep_writing_covers_packages_and_credentials_after_restore() {
    use super::{
        ai::{GenerationInput, SaveWritingInput},
        m2_tests::{model, Vault},
    };
    use crate::ai::WritingContent;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("books.sqlite3");
    let db = Database::open(path.clone());
    let vault = Vault::default();
    let provider = db
        .save_provider(None, model(), Some("M5-test-only-secret"), &vault)
        .unwrap()
        .provider;
    let mut expected = Vec::new();
    for title in ["论语", "西游记", "朝花夕拾"] {
        let mut p = project(&db);
        p.input.title = title.into();
        let p = db.save_project(Some(&p.id), p.input).unwrap();
        let generation = GenerationInput {
            project_id: p.id.clone(),
            provider_id: provider.id.clone(),
            revision: None,
            target_duration_seconds: None,
            recommend_duration: false,
        };
        let task = db.start_generation(generation.clone()).unwrap();
        db.prepare_generation(&task).unwrap();
        let content = WritingContent::Publishing {
            title: format!("一起读《{title}》"),
            description: format!("读《{title}》\n从原书出发。"),
            audio_prompt: format!("请以《{title}》原书为来源。"),
            cover_title: format!("一起读《{title}》"),
            cover_subtitle: "从书中看见生活".into(),
            cover_prompt_landscape: format!("横版 4:3，《{title}》的主题，无字背景。"),
            cover_prompt_portrait: format!("竖版 3:4，《{title}》的主题，纵向构图，无字背景。"),
            cover_design: None,
        };
        let first = db
            .finish_generation(&task, &generation, content.clone(), "test")
            .unwrap();
        db.save_writing(SaveWritingInput {
            project_id: p.id.clone(),
            kind: "publishing".into(),
            content,
            based_on: Some(first.id),
            is_confirmed: true,
        })
        .unwrap();
        let failed = db.start_generation(generation).unwrap();
        db.prepare_generation(&failed).unwrap();
        db.end_generation(&failed, "failed", "timeout", "模型请求超时")
            .unwrap();
        let writing = db.list_versions(&p.id, "publishing").unwrap();
        assert_eq!(writing.len(), 2);
        let mut i = input(&db, &p);
        if let WritingContent::Publishing {
            title, description, ..
        } = &writing[0].content
        {
            i.title = title.clone();
            i.description = description.clone();
        }
        let publish = db.save_publish_task(None, i).unwrap();
        db.prepare_publish_task(&publish.id).unwrap();
        expected.push((
            p.id.clone(),
            serde_json::to_value(writing).unwrap(),
            serde_json::to_value(db.list_cover_versions(&p.id).unwrap()).unwrap(),
            serde_json::to_value(db.list_publish_tasks(&p.id).unwrap()).unwrap(),
        ));
    }
    let backup = db.create_backup(dir.path().to_str().unwrap()).unwrap();
    assert!(!fs::read(&backup)
        .unwrap()
        .windows(b"M5-test-only-secret".len())
        .any(|s| s == b"M5-test-only-secret"));
    for (id, ..) in &expected {
        db.delete_project(id).unwrap();
    }
    let recovery = db.restore_backup(&backup).unwrap();
    assert!(Path::new(&recovery).is_file());
    drop(db);
    let db = Database::open(path);
    assert_eq!(
        db.list_providers(&vault).unwrap()[0].credential_status,
        "available"
    );
    for (id, writing, covers, tasks) in expected {
        assert_eq!(
            serde_json::to_value(db.list_versions(&id, "publishing").unwrap()).unwrap(),
            writing
        );
        assert_eq!(
            serde_json::to_value(db.list_cover_versions(&id).unwrap()).unwrap(),
            covers
        );
        assert_eq!(
            serde_json::to_value(db.list_publish_tasks(&id).unwrap()).unwrap(),
            tasks
        );
        assert_eq!(db.list_ai_tasks(&id).unwrap().len(), 2);
        let video = db
            .list_assets(&id)
            .unwrap()
            .into_iter()
            .find(|asset| asset.asset_type == "video")
            .unwrap();
        assert_eq!(fs::read(&video.file_path).unwrap(), PORTRAIT_VIDEO);
    }
}
