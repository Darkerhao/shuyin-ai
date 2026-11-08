use super::{
    automation::*,
    m2_tests::{model, Vault},
    m4_tests, *,
};
use crate::douyin::flow::Account;
use std::{fs, path::Path};

fn setup() -> (tempfile::TempDir, Database, StartAutomationInput) {
    let directory = tempfile::tempdir().unwrap();
    let database = Database::open(directory.path().join("data.sqlite3"));
    let project = m4_tests::project(&database);
    let source = directory.path().join("原书.txt");
    fs::write(&source, "这是真实原书测试资料。").unwrap();
    let source_asset_id = database
        .link_asset(&project.id, source.to_str().unwrap(), None)
        .unwrap();
    let text = database
        .save_provider(None, model(), None, &Vault::default())
        .unwrap()
        .provider;
    let mut image = model();
    image.model_kind = "image".into();
    image.name = "测试图片".into();
    let image = database
        .save_provider(None, image, None, &Vault::default())
        .unwrap()
        .provider;
    (
        directory,
        database,
        StartAutomationInput {
            project_id: project.id,
            source_asset_id,
            text_provider_id: text.id,
            image_provider_id: image.id,
            format: "deep_dive".into(),
            request_key: "start-one".into(),
        },
    )
}
fn account() -> Account {
    Account {
        id: None,
        name: "测试账号".into(),
        key: "window-account-identity".into(),
    }
}

#[test]
fn automation_start_is_idempotent_and_exclusive() {
    let (_directory, database, input) = setup();
    let run = database
        .create_automation(input.clone(), account())
        .unwrap();
    assert_eq!(
        database
            .create_automation(input.clone(), account())
            .unwrap()
            .id,
        run.id
    );
    let mut other = input.clone();
    other.request_key = "second".into();
    assert!(database.create_automation(other, account()).is_err());
    let mut changed = input;
    changed.format = "brief".into();
    assert!(database.create_automation(changed, account()).is_err());
    assert_eq!(database.automation_runs().unwrap().len(), 1);
    assert_eq!(run.input.target_duration_seconds, 600);
    assert_eq!(run.input.account.key, account().key);
}

#[test]
fn automatic_audio_uses_real_source_without_fake_writing_version() {
    let (_directory, database, input) = setup();
    let run = database.create_automation(input, account()).unwrap();
    let audio = database.start_automatic_audio(&run).unwrap();
    assert!(audio.writing_version_id.is_none());
    assert_eq!(audio.target_duration_seconds, 600);
    assert_eq!(
        database
            .automation_run(&run.id)
            .unwrap()
            .audio_job_id
            .as_deref(),
        Some(audio.id.as_str())
    );
    assert!(database
        .list_versions(&run.project_id, "publishing")
        .unwrap()
        .is_empty());
    let frozen: String = database
        .with_connection(|connection| {
            connection
                .query_row(
                    "SELECT input_json FROM audio_jobs WHERE id=?1",
                    [&audio.id],
                    |row| row.get(0),
                )
                .map_err(|e| e.to_string())
        })
        .unwrap();
    assert!(frozen.contains("automatic-audio-v1"));
    assert!(frozen.contains(&run.input.source_sha256));
    assert!(frozen.contains("仅依据上传的原书"));
    assert!(database.start_automatic_audio(&run).is_err());
}

#[test]
fn automatic_audio_rejects_changed_source_and_paused_stale_run() {
    let (directory, database, input) = setup();
    let run = database.create_automation(input, account()).unwrap();
    fs::write(directory.path().join("原书.txt"), "完全不同的内容").unwrap();
    assert!(database.start_automatic_audio(&run).is_err());
    assert!(database
        .automation_run(&run.id)
        .unwrap()
        .audio_job_id
        .is_none());
    fs::write(directory.path().join("原书.txt"), "这是真实原书测试资料。").unwrap();
    database.control_automation(&run.id, "pause").unwrap();
    assert!(database.start_automatic_audio(&run).is_err());
    let count: i64 = database
        .with_connection(|connection| {
            connection
                .query_row("SELECT count(*) FROM audio_jobs", [], |row| row.get(0))
                .map_err(|e| e.to_string())
        })
        .unwrap();
    assert_eq!(count, 0);
}

#[test]
fn automatic_image_binding_rolls_back_after_pause_and_preserves_single_task() {
    let (_directory, database, input) = setup();
    let run = database.create_automation(input, account()).unwrap();
    let image = || image_generation::ImageGenerationInput {
        project_id: run.project_id.clone(),
        provider_id: run.input.selection.image_provider_id.clone(),
        description: "4:3原书背景".into(),
        width: 1440,
        height: 1080,
    };
    database.control_automation(&run.id, "pause").unwrap();
    assert!(database
        .start_bound_cover_generation(image(), &run.binding(Slot::LandscapeImage))
        .is_err());
    assert!(database.list_ai_tasks(&run.project_id).unwrap().is_empty());
    let resumed = database.control_automation(&run.id, "resume").unwrap();
    let id = database
        .start_bound_cover_generation(image(), &resumed.binding(Slot::LandscapeImage))
        .unwrap();
    assert!(database
        .start_bound_cover_generation(image(), &resumed.binding(Slot::LandscapeImage))
        .is_err());
    assert_eq!(database.list_ai_tasks(&run.project_id).unwrap().len(), 1);
    assert_eq!(
        database
            .automation_run(&run.id)
            .unwrap()
            .landscape_task_id
            .as_deref(),
        Some(id.as_str())
    );
}

#[test]
fn automation_publish_policy_never_sets_manual_confirmation_and_invalidates_on_edit() {
    let (_directory, database, input) = setup();
    let run = database.create_automation(input, account()).unwrap();
    let project = database.get_project(&run.project_id).unwrap();
    let mut publish = m4_tests::input(&database, &project);
    publish.title_confirmed = false;
    publish.covers_confirmed = false;
    publish.allow_download = false;
    publish.scheduled_at = Some(database.next_automation_schedule().unwrap());
    let task = database
        .save_bound_publish_task(None, publish.clone(), Some(&run.binding(Slot::Publish)))
        .unwrap();
    assert!(!task.input.title_confirmed && !task.input.covers_confirmed);
    assert!(task
        .checklist
        .iter()
        .filter(|item| ["title", "covers"].contains(&item.key))
        .all(|item| item.passed));
    let ready = database.prepare_publish_task(&task.id).unwrap();
    assert_eq!(ready.status, "ready");
    let snapshot = database.publish_snapshot(&task.id).unwrap();
    assert!(!snapshot.input.allow_download);
    assert!(snapshot
        .input
        .scheduled_at
        .unwrap()
        .ends_with("T01:11:00+08:00"));
    publish.title = "编辑后需要重新核对".into();
    let changed = database.save_publish_task(Some(&task.id), publish).unwrap();
    assert!(
        !changed
            .checklist
            .iter()
            .find(|item| item.key == "title")
            .unwrap()
            .passed
    );
    assert!(database.prepare_publish_task(&task.id).is_err());
}

#[test]
fn automation_restart_and_backup_keep_upload_intent_without_replay() {
    let (directory, database, input) = setup();
    let run = database.create_automation(input, account()).unwrap();
    database.mark_automation_upload(&run).unwrap();
    assert!(database.mark_automation_upload(&run).is_err());
    let backup = database
        .create_backup(directory.path().to_str().unwrap())
        .unwrap();
    assert!(database.restore_backup(&backup).is_err());
    database.control_automation(&run.id, "pause").unwrap();
    database.restore_backup(&backup).unwrap();
    let restored = database.automation_run(&run.id).unwrap();
    assert_eq!(restored.status, "paused");
    assert!(restored.publish_started);
    assert!(restored.run_id > run.run_id);
    database.control_automation(&run.id, "resume").unwrap();
    database.recover_automation().unwrap();
    assert_eq!(database.automation_run(&run.id).unwrap().status, "paused");
}

#[test]
fn automation_model_changes_are_detected_before_next_step() {
    let (_directory, database, input) = setup();
    let run = database.create_automation(input, account()).unwrap();
    database.verify_automation_input(&run).unwrap();
    let mut model = run.input.text_provider.clone();
    model.model_name = "another-model".into();
    database
        .save_provider(
            Some(&run.input.selection.text_provider_id),
            model,
            None,
            &Vault::default(),
        )
        .unwrap();
    assert!(database.verify_automation_input(&run).is_err());
}

#[test]
fn automation_model_display_changes_do_not_interrupt_the_frozen_provider() {
    let (_directory, database, input) = setup();
    let run = database.create_automation(input, account()).unwrap();
    let mut renamed = run.input.text_provider.clone();
    renamed.name = "已整理的模型名称".into();
    database
        .save_provider(
            Some(&run.input.selection.text_provider_id),
            renamed,
            None,
            &Vault::default(),
        )
        .unwrap();
    assert!(database.verify_automation_input(&run).is_ok());

    let mut default = model();
    default.name = "另一任务的默认文本模型".into();
    database
        .save_provider(None, default, None, &Vault::default())
        .unwrap();
    assert!(
        !database
            .get_provider(&run.input.selection.text_provider_id)
            .unwrap()
            .input
            .is_default
    );
    assert!(database.verify_automation_input(&run).is_ok());

    let mut changed = database
        .get_provider(&run.input.selection.text_provider_id)
        .unwrap()
        .input;
    changed.base_url = "https://different.example/v1".into();
    database
        .save_provider(
            Some(&run.input.selection.text_provider_id),
            changed,
            None,
            &Vault::default(),
        )
        .unwrap();
    assert!(database.verify_automation_input(&run).is_err());
}

#[test]
fn automation_v19_upgrade_keeps_previous_data_and_manual_defaults() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("v19.sqlite3");
    let connection = Connection::open(&path).unwrap();
    for sql in MIGRATIONS.iter().take(19) {
        connection.execute_batch(sql).unwrap();
    }
    connection.pragma_update(None, "user_version", 19).unwrap();
    connection
        .execute(
            "INSERT INTO app_settings(key,value) VALUES('old-setting','keep')",
            [],
        )
        .unwrap();
    drop(connection);
    let database = Database::open(path);
    database
        .with_connection(|connection| {
            assert_eq!(
                connection
                    .pragma_query_value(None, "user_version", |row| row.get::<_, u32>(0))
                    .unwrap(),
                21
            );
            assert_eq!(
                connection
                    .query_row(
                        "SELECT value FROM app_settings WHERE key='old-setting'",
                        [],
                        |row| row.get::<_, String>(0)
                    )
                    .unwrap(),
                "keep"
            );
            assert_eq!(
                connection
                    .query_row("SELECT count(*) FROM automation_runs", [], |row| row
                        .get::<_, i64>(0))
                    .unwrap(),
                0
            );
            Ok(())
        })
        .unwrap();
}

fn freeze_v20_schema(database: &Database, missing_markers: bool) {
    database
        .with_connection(|connection| {
            let tx = connection.transaction().unwrap();
            tx.execute_batch("DROP INDEX automation_single_active; DROP INDEX automation_project; ALTER TABLE automation_runs RENAME TO automation_previous;").unwrap();
            let sql = MIGRATIONS[19]
                .lines()
                .filter(|line| !missing_markers || !(line.trim_start().starts_with("publish_started ") || line.trim_start().starts_with("publish_handed_off ")))
                .collect::<Vec<_>>()
                .join("\n");
            tx.execute_batch(&sql).unwrap();
            let columns = "id,request_key,project_id,input_json,status,stage,run_id,error,audio_job_id,video_job_id,writing_task_id,landscape_task_id,portrait_task_id,landscape_cover_id,portrait_cover_id,publish_task_id,publish_input_json,created_at,updated_at";
            tx.execute_batch(&format!("INSERT INTO automation_runs({columns}) SELECT {columns} FROM automation_previous;")).unwrap();
            if !missing_markers {
                tx.execute_batch("UPDATE automation_runs SET publish_started=(SELECT publish_started FROM automation_previous WHERE id=automation_runs.id),publish_handed_off=(SELECT publish_handed_off FROM automation_previous WHERE id=automation_runs.id);").unwrap();
            }
            tx.execute_batch("DROP TABLE automation_previous; PRAGMA user_version=20;").unwrap();
            tx.commit().unwrap();
            Ok(())
        })
        .unwrap();
}

#[test]
fn automation_legacy_v20_reopens_and_restores_without_losing_the_run() {
    let (directory, database, input) = setup();
    let run = database.create_automation(input, account()).unwrap();
    let audio = database.start_automatic_audio(&run).unwrap();
    freeze_v20_schema(&database, true);
    let backup = database
        .create_backup(directory.path().to_str().unwrap())
        .unwrap();
    drop(database);

    let database = Database::open(directory.path().join("data.sqlite3"));
    let upgraded = database.automation_run(&run.id).unwrap();
    assert_eq!(upgraded.audio_job_id.as_deref(), Some(audio.id.as_str()));
    assert_eq!(upgraded.input.source_sha256, run.input.source_sha256);
    assert!(!upgraded.publish_started);
    let snapshot = super::backup::read_snapshot(Path::new(&backup)).unwrap();
    assert_eq!(
        snapshot
            .query_row("SELECT id FROM automation_runs", [], |r| r
                .get::<_, String>(0))
            .unwrap(),
        run.id
    );
    assert_eq!(
        snapshot
            .query_row("SELECT status FROM automation_runs", [], |r| r
                .get::<_, String>(0))
            .unwrap(),
        "paused"
    );
    let source = Connection::open(&backup).unwrap();
    assert_eq!(
        source
            .pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
            .unwrap(),
        20
    );
    assert!(source
        .prepare("SELECT publish_started FROM automation_runs")
        .is_err());
    database
        .create_backup(directory.path().to_str().unwrap())
        .and_then(|path| super::backup::read_snapshot(Path::new(&path)).map(|_| ()))
        .unwrap();
}

#[test]
fn automation_v20_upgrade_keeps_existing_upload_and_handoff_markers() {
    let (directory, database, input) = setup();
    let run = database.create_automation(input, account()).unwrap();
    database.mark_automation_upload(&run).unwrap();
    database
        .with_connection(|c| {
            c.execute("UPDATE automation_runs SET publish_handed_off=1", [])
                .map(|_| ())
                .map_err(|e| e.to_string())
        })
        .unwrap();
    freeze_v20_schema(&database, false);
    let backup = database
        .create_backup(directory.path().to_str().unwrap())
        .unwrap();
    drop(database);

    for _ in 0..2 {
        let database = Database::open(directory.path().join("data.sqlite3"));
        assert!(database.automation_run(&run.id).unwrap().publish_started);
        database
            .with_connection(|c| {
                assert_eq!(
                    c.query_row("SELECT publish_handed_off FROM automation_runs", [], |r| {
                        r.get::<_, i64>(0)
                    })
                    .unwrap(),
                    1
                );
                Ok(())
            })
            .unwrap();
    }
    let snapshot = super::backup::read_snapshot(Path::new(&backup)).unwrap();
    assert_eq!(
        snapshot
            .query_row(
                "SELECT publish_started,publish_handed_off FROM automation_runs",
                [],
                |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?))
            )
            .unwrap(),
        (1, 1)
    );
}

#[test]
fn automation_legacy_v20_failed_upgrade_keeps_the_original_schema_and_run() {
    let (directory, database, input) = setup();
    let run = database.create_automation(input, account()).unwrap();
    freeze_v20_schema(&database, true);
    database
        .with_connection(|c| {
            c.execute_batch("CREATE TABLE automation_runs_previous (original TEXT);")
                .map_err(|e| e.to_string())
        })
        .unwrap();
    drop(database);
    let path = directory.path().join("data.sqlite3");
    assert!(Database::open(path.clone()).read().is_err());
    let original = Connection::open(path).unwrap();
    assert_eq!(
        original
            .pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
            .unwrap(),
        20
    );
    assert_eq!(
        original
            .query_row("SELECT id FROM automation_runs", [], |r| r
                .get::<_, String>(0))
            .unwrap(),
        run.id
    );
    assert!(original
        .prepare("SELECT publish_started FROM automation_runs")
        .is_err());
    assert_eq!(original.query_row("SELECT count(*) FROM sqlite_master WHERE name IN ('automation_single_active','automation_project')", [], |r| r.get::<_, i64>(0)).unwrap(), 2);
}

#[test]
fn automation_legacy_v20_backup_still_rejects_unexpected_schema_changes() {
    let (directory, database, input) = setup();
    database.create_automation(input, account()).unwrap();
    freeze_v20_schema(&database, true);
    database
        .with_connection(|c| {
            c.execute_batch("ALTER TABLE automation_runs ADD COLUMN unexpected TEXT;")
                .map_err(|e| e.to_string())
        })
        .unwrap();
    let backup = database
        .create_backup(directory.path().to_str().unwrap())
        .unwrap();
    assert!(super::backup::read_snapshot(Path::new(&backup))
        .err()
        .unwrap()
        .contains("结构不匹配"));
}

#[test]
fn automation_late_cover_failure_does_not_pause_the_next_size_or_generation() {
    let (_directory, database, input) = setup();
    let run = database.create_automation(input, account()).unwrap();
    database
        .set_automation_stage(&run, "portrait_cover")
        .unwrap();
    database
        .fail_automation_cover(&run.id, run.run_id, "landscape_cover")
        .unwrap();
    assert_eq!(database.automation_run(&run.id).unwrap().status, "running");
    database.control_automation(&run.id, "pause").unwrap();
    let resumed = database.control_automation(&run.id, "resume").unwrap();
    database
        .fail_automation_cover(&run.id, run.run_id, "portrait_cover")
        .unwrap();
    assert_eq!(database.automation_run(&run.id).unwrap().status, "running");
    database
        .fail_automation_cover(&run.id, resumed.run_id, "portrait_cover")
        .unwrap();
    assert_eq!(
        database.automation_run(&run.id).unwrap().status,
        "waiting_user"
    );
}

#[test]
fn automatic_video_keeps_archived_audio_hash_and_frozen_prompt() {
    let (directory, database, input) = setup();
    let run = database.create_automation(input, account()).unwrap();
    let audio = database.start_automatic_audio(&run).unwrap();
    let path = directory.path().join("original.wav");
    fs::write(&path, b"audio fixture").unwrap();
    let asset = database
        .link_asset(&run.project_id, path.to_str().unwrap(), None)
        .unwrap();
    let expected = "a".repeat(64);
    database.with_connection(|connection| {
        connection.execute("UPDATE audio_jobs SET status='succeeded',asset_id=?1,output_sha256=?2,input_json=json_set(input_json,'$.prompt','旧版本实际使用的冻结指导') WHERE id=?3",rusqlite::params![asset,expected,audio.id]).map_err(|e|e.to_string())?;
        Ok(())
    }).unwrap();
    let run = database.automation_run(&run.id).unwrap();
    let video = database
        .create_bound_video_job(
            video::StartVideoInput {
                project_id: run.project_id.clone(),
                audio_asset_id: asset,
                subtitle_asset_id: None,
                provider_id: run.input.selection.text_provider_id.clone(),
            },
            Some(&run.binding(Slot::Video)),
        )
        .unwrap();
    assert_eq!(video.plan.audio_sha256, expected);
    assert_eq!(
        database.audio_job_prompt(&audio.id).unwrap(),
        "旧版本实际使用的冻结指导"
    );
    let mut plan = video.plan;
    plan.duration = 2.0;
    plan.cues = vec![crate::video::Cue {
        start: 0.0,
        end: 2.0,
        text: "实际声音里的句子".into(),
    }];
    database
        .with_connection(|connection| {
            connection
                .execute(
                    "UPDATE video_jobs SET plan_json=?1 WHERE id=?2",
                    rusqlite::params![serde_json::to_string(&plan).unwrap(), video.id],
                )
                .map_err(|e| e.to_string())?;
            Ok(())
        })
        .unwrap();
    let run = database.automation_run(&run.id).unwrap();
    database.create_transcript_task(&run).unwrap();
    let run = database.automation_run(&run.id).unwrap();
    database.verify_automation_video(&run).unwrap();
    plan.cues[0].text = "被改写的新字幕".into();
    database
        .with_connection(|connection| {
            connection
                .execute(
                    "UPDATE video_jobs SET plan_json=?1 WHERE id=?2",
                    rusqlite::params![serde_json::to_string(&plan).unwrap(), video.id],
                )
                .map_err(|e| e.to_string())?;
            Ok(())
        })
        .unwrap();
    assert!(database.verify_automation_video(&run).is_err());
}

#[test]
fn automatic_schedule_renews_only_before_hand_off_and_preserves_same_task() {
    let (_directory, database, input) = setup();
    let run = database.create_automation(input, account()).unwrap();
    let project = database.get_project(&run.project_id).unwrap();
    let mut input = m4_tests::input(&database, &project);
    input.title_confirmed = false;
    input.covers_confirmed = false;
    input.allow_download = false;
    input.scheduled_at = Some(database.next_automation_schedule().unwrap());
    let task = database
        .save_bound_publish_task(None, input, Some(&run.binding(Slot::Publish)))
        .unwrap();
    let expire = || {
        database.with_connection(|connection| {
        let expired = "2000-01-01T01:11:00+08:00";
        connection.execute("UPDATE publish_tasks SET scheduled_at=?1 WHERE id=?2",rusqlite::params![expired,task.id]).map_err(|e|e.to_string())?;
        connection.execute("UPDATE automation_runs SET publish_input_json=json_set(publish_input_json,'$.scheduledAt',?1) WHERE id=?2",rusqlite::params![expired,run.id]).map_err(|e|e.to_string())?;
        Ok(())
    }).unwrap()
    };
    expire();
    assert!(database.renew_automatic_schedule(&task.id).unwrap());
    let prepared = database.prepare_publish_task(&task.id).unwrap();
    assert_eq!(prepared.id, task.id);
    assert!(!prepared.input.allow_download);
    assert!(prepared
        .input
        .scheduled_at
        .unwrap()
        .ends_with("T01:11:00+08:00"));
    assert!(database.publish_snapshot(&task.id).is_ok());
    database.hand_off_automatic_publish(&task.id).unwrap();
    expire();
    assert!(!database.renew_automatic_schedule(&task.id).unwrap());
    assert!(database.publish_snapshot(&task.id).is_err());
}

#[test]
fn automatic_audio_stop_prevents_late_source_and_generation_actions() {
    let (_directory, database, input) = setup();
    let run = database.create_automation(input, account()).unwrap();
    let audio = database.start_automatic_audio(&run).unwrap();
    let waiting = database
        .observe_audio_job(
            &audio.id,
            audio.run_id,
            &audio::AudioObservation::LoginReady,
        )
        .unwrap();
    assert_eq!(waiting.stage, "creating_notebook");
    database.control_automation(&run.id, "cancel").unwrap();
    database.control_audio_job(&audio.id, "cancel").unwrap();
    assert!(database
        .observe_audio_job(
            &audio.id,
            audio.run_id,
            &audio::AudioObservation::NotebookCreated {
                notebook_url: "https://notebooklm.google.com/notebook/original".into()
            }
        )
        .is_err());
    assert!(database
        .observe_audio_job(
            &audio.id,
            audio.run_id,
            &audio::AudioObservation::SourceSubmitted
        )
        .is_err());
    assert!(database
        .observe_audio_job(
            &audio.id,
            audio.run_id,
            &audio::AudioObservation::GenerationSubmitted
        )
        .is_err());
    let stopped = database.audio_job(&audio.id).unwrap();
    assert_eq!(stopped.status, "cancelled");
    assert!(stopped.submitted_at.is_none());
}
