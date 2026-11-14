use super::chapters::ChapterInput;
use super::*;
use rusqlite::params;

fn chapter_input(title: &str, content: &str) -> ChapterInput {
    ChapterInput {
        title: title.into(),
        content: content.into(),
    }
}

#[test]
fn chapter_review_media_preview_resolves_exact_audio_and_video_but_rejects_invalid_assets() {
    let directory = tempfile::tempdir().unwrap();
    let database = Database::open(directory.path().join("preview.sqlite3"));
    let book = m4_tests::project(&database);
    let other = m4_tests::project(&database);
    let chapter = database
        .save_chapter(&book.id, None, chapter_input("第一章", "本章正文"))
        .unwrap();
    for scope in [None, Some(chapter.id.as_str())] {
        for extension in ["wav", "mp4", "txt"] {
            let path = directory
                .path()
                .join(format!("{}.{extension}", scope.unwrap_or("book")));
            std::fs::write(&path, b"local preview fixture").unwrap();
            let id = database
                .link_asset(&book.id, path.to_str().unwrap(), None, scope)
                .unwrap();
            assert!(database.media_asset_path(&other.id, &id).is_err());
            if extension == "txt" {
                assert!(database.media_asset_path(&book.id, &id).is_err());
                continue;
            }
            assert_eq!(
                database.media_asset_path(&book.id, &id).unwrap(),
                std::fs::canonicalize(&path).unwrap()
            );
            std::fs::remove_file(&path).unwrap();
            assert!(database.media_asset_path(&book.id, &id).is_err());
            std::fs::create_dir(&path).unwrap();
            assert!(database.media_asset_path(&book.id, &id).is_err());
        }
    }
    assert!(database.media_asset_path(&book.id, "unknown").is_err());
}

#[test]
fn chapter_review_manual_edits_keep_original_source_without_duration_constraints() {
    let directory = tempfile::tempdir().unwrap();
    let database = Database::open(directory.path().join("manual-audio.sqlite3"));
    let book = m4_tests::project(&database);
    let chapter = database
        .save_chapter(
            &book.id,
            None,
            chapter_input("第一章", "制作旧方案时的原文"),
        )
        .unwrap();
    database
        .seed_legacy_duration(&book.id, 300, Some(&chapter.id))
        .unwrap();
    let original = writing(&database, &book.id, &chapter.id);
    database
        .seed_legacy_duration(&book.id, 900, Some(&chapter.id))
        .unwrap();
    database
        .save_chapter(
            &book.id,
            Some(&chapter.id),
            chapter_input("第一章新名称", "后来的正文，不得混入旧指导"),
        )
        .unwrap();
    for field in ["title", "audioPrompt"] {
        let mut content = serde_json::to_value(&original.content).unwrap();
        content[field] = serde_json::json!(if field == "title" {
            "只修改标题"
        } else {
            "手动修改的章节精讲指导"
        });
        let saved=database.save_writing(serde_json::from_value(serde_json::json!({
            "projectId":book.id,"chapterId":chapter.id,"kind":"publishing","content":content,"basedOn":original.id,"isConfirmed":false
        })).unwrap()).unwrap();
        let snapshot = saved.source_snapshot.as_ref().unwrap();
        assert_eq!(snapshot.content, "制作旧方案时的原文");
        assert_eq!(snapshot.chapter_title, "第一章");
        assert_eq!(
            snapshot.target_duration_seconds, None,
            "manual {field} edit"
        );
        let audio = database
            .start_audio_job(audio::StartAudioJobInput {
                project_id: book.id.clone(),
                writing_version_id: saved.id.clone(),
                source: audio::AudioSourceInput::Chapter {
                    chapter_id: chapter.id.clone(),
                },
                format: "deep_dive".into(),
            })
            .unwrap();
        assert!(serde_json::to_value(&audio)
            .unwrap()
            .get("targetDurationSeconds")
            .is_none());
        assert_eq!(
            database.audio_job_prompt(&audio.id).unwrap(),
            content["audioPrompt"].as_str().unwrap()
        );
        database.control_audio_job(&audio.id, "cancel").unwrap();
    }
    assert_eq!(
        database
            .list_versions(&book.id, "publishing", Some(&chapter.id))
            .unwrap()
            .last()
            .unwrap()
            .source_snapshot
            .as_ref()
            .unwrap()
            .target_duration_seconds,
        None
    );
}

#[test]
fn chapter_save_draft_and_restart_keep_each_chapter_separate_from_book_notes() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("chapters.sqlite3");
    let database = Database::open(path.clone());
    database
        .set_workspace_root(directory.path().to_str().unwrap())
        .unwrap();
    let book = super::m4_tests::project(&database);
    let new_owner = format!("{}:chapter:new", book.id);
    database
        .save_editor_draft(
            &new_owner,
            "project",
            Some(super::drafts::EditorDraft {
                based_on: None,
                content: serde_json::json!({"title":"","content":"未完成正文"}),
                updated_at: String::new(),
            }),
        )
        .unwrap();
    assert!(database
        .save_chapter(&book.id, None, chapter_input("", "正文"))
        .is_err());
    assert!(database
        .read_editor_draft(&new_owner, "project")
        .unwrap()
        .is_some());
    let first = database
        .save_chapter(
            &book.id,
            None,
            chapter_input(" 第一章 ", "\u{feff}第一段\r\n第二段。"),
        )
        .unwrap();
    let second = database
        .save_chapter(
            &book.id,
            None,
            chapter_input("第二章", "仅属于第二章的内容。"),
        )
        .unwrap();
    assert!(database
        .read_editor_draft(&new_owner, "project")
        .unwrap()
        .is_none());
    assert_eq!(database.list_chapters(&book.id).unwrap().len(), 2);
    let other = super::m4_tests::project(&database);
    assert!(database.get_chapter(&other.id, &first.id).is_err());
    assert!(database
        .save_chapter(
            &other.id,
            Some(&first.id),
            chapter_input("错误覆盖", "不应写入")
        )
        .is_err());
    drop(database);
    let reopened = Database::open(path);
    assert_eq!(
        reopened
            .get_chapter(&book.id, &first.id)
            .unwrap()
            .input
            .content,
        "第一段\n第二段。"
    );
    assert_eq!(
        reopened
            .get_chapter(&book.id, &second.id)
            .unwrap()
            .input
            .content,
        "仅属于第二章的内容。"
    );
    assert_eq!(
        reopened.get_project(&book.id).unwrap().input.source_notes,
        "原始资料"
    );
}

#[test]
fn chapter_generation_freezes_the_selected_text_and_duration_only_updates_that_chapter() {
    let directory = tempfile::tempdir().unwrap();
    let database = Database::open(directory.path().join("chapters.sqlite3"));
    database
        .set_workspace_root(directory.path().to_str().unwrap())
        .unwrap();
    let book = super::m4_tests::project(&database);
    let first = database
        .save_chapter(
            &book.id,
            None,
            chapter_input("第一章", "冻结的第一章正文。"),
        )
        .unwrap();
    let second = database
        .save_chapter(
            &book.id,
            None,
            chapter_input("第二章", "第二章正文不能混入。"),
        )
        .unwrap();
    let provider = database
        .save_provider(
            None,
            super::m2_tests::model(),
            None,
            &super::m2_tests::Vault::default(),
        )
        .unwrap()
        .provider;
    let request = database.start_generation(serde_json::from_value(serde_json::json!({"projectId":book.id,"chapterId":first.id,"providerId":provider.id})).unwrap()).unwrap();
    database
        .save_chapter(
            &book.id,
            Some(&first.id),
            chapter_input("第一章", "编辑后的第一章正文。"),
        )
        .unwrap();
    let prepared = database.prepare_generation(&request).unwrap();
    let sent: serde_json::Value = serde_json::from_str(&prepared.user).unwrap();
    assert_eq!(sent["book"]["sourceNotes"], "冻结的第一章正文。");
    assert!(!prepared.user.contains("原始资料"));
    assert!(!prepared.user.contains("第二章正文不能混入"));
    assert_eq!(sent["audio"], serde_json::json!({"seriesName":"读懂一章"}));
    assert!(prepared
        .input
        .source_snapshot
        .as_ref()
        .unwrap()
        .target_duration_seconds
        .is_none());
    for chapter in [&first.id, &second.id] {
        assert!(database
            .get_chapter(&book.id, chapter)
            .unwrap()
            .target_duration_seconds
            .is_none());
    }
    database
        .finish_generation(&request, &prepared.input, m2_tests::publishing(), "text")
        .unwrap();
    assert_eq!(
        database
            .list_versions(&book.id, "publishing", Some(&first.id))
            .unwrap()
            .len(),
        1
    );
    assert!(database
        .list_versions(&book.id, "publishing", Some(&second.id))
        .unwrap()
        .is_empty());
}

#[test]
fn chapter_migration_preserves_books_and_normalizes_the_existing_source() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("chapters.sqlite3");
    let connection = Connection::open(&path).unwrap();
    for migration in &MIGRATIONS[..24] {
        connection.execute_batch(migration).unwrap();
    }
    connection.pragma_update(None, "user_version", 24).unwrap();
    connection.execute("INSERT INTO book_projects(id,title,source_notes,target_duration_seconds,status,workspace_path,created_at,updated_at) VALUES('book','原书','原有整书笔记',600,'draft',?1,'before','before')", [directory.path().to_string_lossy().as_ref()]).unwrap();
    connection.execute("INSERT INTO assets(id,project_id,asset_type,file_path,original_name,file_size,modified_at,created_at) VALUES('source','book','document','old.txt','原书.txt',10,1,'before')", []).unwrap();
    connection.execute("INSERT INTO automation_runs(id,request_key,project_id,input_json,status,stage,created_at,updated_at) VALUES('old-run','old-request','book',?1,'cancelled','audio','before','before')", [r#"{"workflowVersion":2,"selection":{"projectId":"book","sourceAssetId":"source"},"sourceSha256":"original-hash"}"#]).unwrap();
    drop(connection);

    let database = Database::open(path);
    assert_eq!(
        database.get_project("book").unwrap().input.source_notes,
        "原有整书笔记"
    );
    database
        .with_connection(|connection| {
            assert_eq!(
                connection
                    .pragma_query_value(None, "user_version", |row| row.get::<_, u32>(0))
                    .unwrap(),
                25
            );
            let input: String = connection
                .query_row(
                    "SELECT input_json FROM automation_runs WHERE id='old-run'",
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            let input: serde_json::Value = serde_json::from_str(&input).unwrap();
            assert_eq!(
                input["selection"]["source"],
                serde_json::json!({"kind":"asset","assetId":"source"})
            );
            assert!(input["selection"].get("sourceAssetId").is_none());
            assert_eq!(input["sourceSha256"], "original-hash");
            for table in [
                "writing_versions",
                "ai_tasks",
                "audio_jobs",
                "video_jobs",
                "automation_runs",
                "cover_versions",
                "publish_tasks",
                "assets",
            ] {
                let sql = "SELECT count(*) FROM pragma_table_info(?1) WHERE name='chapter_id'";
                assert_eq!(
                    connection
                        .query_row(sql, params![table], |row| row.get::<_, u32>(0))
                        .unwrap(),
                    1,
                    "{table}"
                );
            }
            assert_eq!(
                connection
                    .query_row(
                        "SELECT chapter_id FROM assets WHERE id='source'",
                        [],
                        |row| row.get::<_, Option<String>>(0)
                    )
                    .unwrap(),
                None
            );
            Ok(())
        })
        .unwrap();
    let backup = std::fs::read_dir(directory.path().join("chapter-upgrade-backups"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let original = Connection::open(backup).unwrap();
    assert_eq!(
        original
            .pragma_query_value(None, "user_version", |row| row.get::<_, u32>(0))
            .unwrap(),
        24
    );
    let source: String = original.query_row("SELECT json_extract(input_json,'$.selection.sourceAssetId') FROM automation_runs WHERE id='old-run'", [], |row| row.get(0)).unwrap();
    assert_eq!(source, "source");
}

fn writing(database: &Database, project: &str, chapter: &str) -> ai::WritingVersion {
    database.save_writing(serde_json::from_value(serde_json::json!({
        "projectId": project, "chapterId": chapter, "kind":"publishing", "basedOn": null, "isConfirmed": false,
        "content": {"format":"publishing","title":"本章精讲","description":"详细解读本章的概念和论证。","audioPrompt":"根据本章正文细致解读，不引入其他章节。"}
    })).unwrap()).unwrap()
}

#[test]
fn chapter_versions_and_audio_keep_original_text_after_the_chapter_is_edited() {
    let directory = tempfile::tempdir().unwrap();
    let database = Database::open(directory.path().join("chapters.sqlite3"));
    let book = m4_tests::project(&database);
    let first = database
        .save_chapter(
            &book.id,
            None,
            chapter_input("第一章", "第一章最初的完整正文。"),
        )
        .unwrap();
    let second = database
        .save_chapter(
            &book.id,
            None,
            chapter_input("第二章", "另一章不能用于本次音频。"),
        )
        .unwrap();
    database
        .seed_legacy_duration(&book.id, 300, Some(&first.id))
        .unwrap();
    let version = writing(&database, &book.id, &first.id);
    let original = version.source_snapshot.clone().unwrap();
    database
        .save_chapter(
            &book.id,
            Some(&first.id),
            chapter_input("第一章新名称", "第一章编辑后的正文。"),
        )
        .unwrap();
    database
        .seed_legacy_duration(&book.id, 900, Some(&first.id))
        .unwrap();
    assert!(database
        .start_audio_job(audio::StartAudioJobInput {
            project_id: book.id.clone(),
            writing_version_id: version.id.clone(),
            source: audio::AudioSourceInput::Chapter {
                chapter_id: second.id.clone()
            },
            format: "deep_dive".into(),
        })
        .is_err());
    let job = database
        .start_audio_job(audio::StartAudioJobInput {
            project_id: book.id.clone(),
            writing_version_id: version.id.clone(),
            source: audio::AudioSourceInput::Chapter {
                chapter_id: first.id.clone(),
            },
            format: "deep_dive".into(),
        })
        .unwrap();
    assert_eq!(job.chapter_id.as_deref(), Some(first.id.as_str()));
    assert!(serde_json::to_value(&job)
        .unwrap()
        .get("targetDurationSeconds")
        .is_none());
    assert!(job.source_label.contains("第一章") && !job.source_label.contains("新名称"));
    let frozen: serde_json::Value = database
        .with_connection(|c| {
            let json: String = c
                .query_row(
                    "SELECT input_json FROM audio_jobs WHERE id=?1",
                    [&job.id],
                    |row| row.get(0),
                )
                .unwrap();
            Ok(serde_json::from_str(&json).unwrap())
        })
        .unwrap();
    assert_eq!(frozen["source"]["text"], original.content);
    assert_eq!(frozen["sourceSha256"], original.sha256);
    database.control_audio_job(&job.id, "cancel").unwrap();
    let provider = database
        .save_provider(None, m2_tests::model(), None, &m2_tests::Vault::default())
        .unwrap()
        .provider;
    for field in ["title", "audioPrompt"] {
        let task = database.start_generation(serde_json::from_value(serde_json::json!({
            "projectId":book.id,"chapterId":first.id,"providerId":provider.id,"targetDurationSeconds":900,
            "revision":{"basedOn":version.id,"fields":[field],"instruction":"细化这一项"}
        })).unwrap()).unwrap();
        let prepared = database.prepare_generation(&task).unwrap();
        assert_eq!(
            prepared.input.source_snapshot.as_ref().unwrap().content,
            original.content
        );
        assert_eq!(
            serde_json::to_value(&prepared.input)
                .unwrap()
                .get("targetDurationSeconds"),
            None
        );
        database
            .finish_revision(&task, &prepared.input, version.content.clone())
            .unwrap();
        let adopted = database.apply_generation(&task).unwrap();
        assert_eq!(
            adopted.source_snapshot.unwrap().target_duration_seconds,
            None
        );
        assert_eq!(database.apply_generation(&task).unwrap().id, adopted.id);
    }
    assert!(database
        .list_versions(&book.id, "publishing", None)
        .unwrap()
        .is_empty());
    assert!(database
        .list_versions(&book.id, "publishing", Some(&second.id))
        .unwrap()
        .is_empty());
}

#[test]
fn chapter_covers_video_publishing_and_drafts_are_independent_and_survive_full_restore() {
    let directory = tempfile::tempdir().unwrap();
    let database = Database::open(directory.path().join("chapters.sqlite3"));
    database
        .set_workspace_root(directory.path().to_str().unwrap())
        .unwrap();
    let book = m4_tests::project(&database);
    let mut saved = Vec::new();
    for (name, text) in [
        ("第一章", "第一章的论点和细节"),
        ("第二章", "第二章的论点和细节"),
    ] {
        let chapter = database
            .save_chapter(&book.id, None, chapter_input(name, text))
            .unwrap();
        database
            .seed_legacy_duration(&book.id, 300, Some(&chapter.id))
            .unwrap();
        let version = writing(&database, &book.id, &chapter.id);
        let mut input = m4_tests::scoped_input(&database, &book, Some(&chapter.id));
        input.writing_version_id = Some(version.id.clone());
        let wide = m4_tests::scoped_cover(&database, &book.id, 1920, 1080, Some(&chapter.id));
        let owner = format!("{}:chapter:{}:new", book.id, chapter.id);
        database.save_editor_draft(&owner,"publishing",Some(drafts::EditorDraft {based_on:None,content:serde_json::json!({"chapterId":chapter.id,"title":"本章尚未提交的发布草稿"}),updated_at:String::new()})).unwrap();
        let task = database.save_publish_task(None, input.clone()).unwrap();
        assert!(database
            .read_editor_draft(&owner, "publishing")
            .unwrap()
            .is_none());
        let ready = database.prepare_publish_task(&task.id).unwrap();
        assert_eq!(ready.status, "ready");
        assert_eq!(
            database
                .publish_snapshot(&task.id)
                .unwrap()
                .input
                .chapter_id,
            Some(chapter.id.clone())
        );
        assert_eq!(
            database
                .list_cover_versions(&book.id, Some(&chapter.id))
                .unwrap()
                .len(),
            3
        );
        assert!(database.read_cover_export(&book.id, &wide).unwrap().len() > 100);
        for kind in ["writing", "cover"] {
            database
                .save_editor_draft(
                    &format!("{}:chapter:{}", book.id, chapter.id),
                    kind,
                    Some(drafts::EditorDraft {
                        based_on: None,
                        content: serde_json::json!({"marker":name}),
                        updated_at: String::new(),
                    }),
                )
                .unwrap();
        }
        saved.push((chapter, version, input, task.id));
    }
    for field in [
        "writingVersionId",
        "audioAssetId",
        "videoAssetId",
        "landscapeCoverId",
        "portraitCoverId",
    ] {
        let mut cross = serde_json::to_value(&saved[0].2).unwrap();
        cross[field] = serde_json::to_value(&saved[1].2).unwrap()[field].clone();
        assert!(
            database
                .save_publish_task(None, serde_json::from_value(cross).unwrap())
                .is_err(),
            "{field}"
        );
    }
    assert!(database
        .list_publish_tasks(&book.id, None)
        .unwrap()
        .is_empty());
    assert!(database
        .list_cover_versions(&book.id, None)
        .unwrap()
        .is_empty());
    assert_eq!(
        database.project_progress(&book.id, None).unwrap()[4].status,
        "未开始"
    );
    assert_eq!(
        database
            .project_progress(&book.id, Some(&saved[0].0.id))
            .unwrap()[4]
            .status,
        "已就绪"
    );
    database
        .record_publish_success(
            &saved[0].3,
            publishing::PublishRecordInput {
                confirmed: true,
                published_at: "2026-11-13T01:00:00Z".into(),
                post_url: "https://www.douyin.com/video/123".into(),
                notes: "隔离测试的人工记录，未调用平台".into(),
            },
        )
        .unwrap();
    assert_eq!(
        database.get_project(&book.id).unwrap().input.status,
        "draft"
    );
    assert_eq!(
        database
            .list_publish_tasks(&book.id, Some(&saved[1].0.id))
            .unwrap()[0]
            .status,
        "ready"
    );
    let resume = database.workflow_snapshot().unwrap().resume;
    for (chapter, _, _, _) in &saved {
        assert!(resume
            .iter()
            .any(|task| task.chapter_id.as_ref() == Some(&chapter.id)
                && task.title.contains(&chapter.input.title)));
    }
    let backup = database
        .create_full_backup(directory.path().to_str().unwrap(), true)
        .unwrap();
    let original_bytes = std::fs::read(&backup.manifest_path).unwrap();
    let restored = Database::open(directory.path().join("restored.sqlite3"));
    let restored_workspace = directory.path().join("restored-workspace");
    std::fs::create_dir(&restored_workspace).unwrap();
    restored
        .restore_full_backup(&backup.manifest_path, restored_workspace.to_str().unwrap())
        .unwrap();
    assert_eq!(
        std::fs::read(&backup.manifest_path).unwrap(),
        original_bytes
    );
    for (chapter, version, input, task_id) in saved {
        assert_eq!(
            restored
                .get_chapter(&book.id, &chapter.id)
                .unwrap()
                .input
                .content,
            chapter.input.content
        );
        let history = restored
            .list_versions(&book.id, "publishing", Some(&chapter.id))
            .unwrap();
        assert_eq!(history[0].id, version.id);
        assert_eq!(history[0].source_snapshot, version.source_snapshot);
        let task = restored
            .list_publish_tasks(&book.id, Some(&chapter.id))
            .unwrap()
            .remove(0);
        assert_eq!(task.id, task_id);
        assert_eq!(task.input, input);
        for kind in ["writing", "cover"] {
            assert_eq!(
                restored
                    .read_editor_draft(&format!("{}:chapter:{}", book.id, chapter.id), kind)
                    .unwrap()
                    .unwrap()
                    .content["marker"],
                chapter.input.title
            );
        }
        assert_eq!(
            restored
                .list_cover_versions(&book.id, Some(&chapter.id))
                .unwrap()
                .len(),
            3
        );
        assert!(restored
            .list_assets(&book.id, Some(&chapter.id))
            .unwrap()
            .iter()
            .all(|asset| asset.exists_status == "available"));
    }
    restored
        .with_connection(|c| {
            assert!(!c
                .prepare("PRAGMA foreign_key_check")
                .unwrap()
                .exists([])
                .unwrap());
            Ok(())
        })
        .unwrap();
}
