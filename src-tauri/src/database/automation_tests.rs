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

fn finish_creation(database: &Database, run: &AutomationRun) -> AutomationRun {
    let run = if run.stage == "duration" {
        let id = database.start_automation_generation(run).unwrap();
        database.prepare_generation(&id).unwrap();
        database
            .finish_duration_recommendation(
                &id,
                crate::ai::audio::DurationRecommendation {
                    minutes: 12,
                    reason: "依据本期重点估计".into(),
                },
            )
            .unwrap();
        database.set_automation_stage(run, "writing").unwrap();
        database.automation_run(&run.id).unwrap()
    } else {
        database.automation_run(&run.id).unwrap()
    };
    let id = database.start_automation_generation(&run).unwrap();
    let prepared = database.prepare_generation(&id).unwrap();
    let content: crate::ai::WritingContent = serde_json::from_value(serde_json::json!({"format":"publishing","title":"已保存的创作标题","description":"这期依据原书介绍核心问题","audioPrompt":"本次保存的完整指导，请依据上传原书制作，不补造情节。","tags":"#读书 #主题 #理解","coverTitle":"封面标题","coverSubtitle":"书籍副标题","coverPromptLandscape":"4:3横向书房","coverPromptPortrait":"3:4竖向书架","coverPromptWidescreen":"16:9宽阔空间","coverDesign":{"titleSize":136,"subtitleSize":50,"textColor":"#ffffff","subtitleColor":"#ffffff","accentColor":"#abcdef","backgroundColor":"#123456","overlayOpacity":0.7}})).unwrap();
    database
        .finish_generation(&id, &prepared.input, content, "text")
        .unwrap();
    database.set_automation_stage(&run, "audio").unwrap();
    database.automation_run(&run.id).unwrap()
}

#[test]
fn automation_frontloaded_duration_starts_before_audio() {
    let (_directory, database, input) = setup();
    let run = database.create_automation(input, account()).unwrap();
    assert_eq!(run.stage, "duration");
    let frozen = serde_json::to_value(&run.input).unwrap();
    assert!(frozen["targetDurationSeconds"].is_null());
    assert_eq!(frozen["sourceNotes"], "原始资料");
    assert!(database.start_automatic_audio(&run).is_err());
}

#[test]
fn automation_frontloaded_respects_saved_fifteen_minutes() {
    let (_directory, database, input) = setup();
    database.set_audio_duration(&input.project_id, 900).unwrap();
    let run = database.create_automation(input, account()).unwrap();
    assert_eq!(
        serde_json::to_value(&run.input).unwrap()["targetDurationSeconds"],
        900
    );
    assert_eq!(run.stage, "writing");
    assert!(database.start_automatic_audio(&run).is_err());
}

#[test]
fn automation_frontloaded_rejects_out_of_range_before_creating_tasks() {
    let (_directory, database, input) = setup();
    database
        .set_audio_duration(&input.project_id, 1200)
        .unwrap();
    assert!(database.create_automation(input, account()).is_err());
    assert!(database.automation_runs().unwrap().is_empty());
}

#[test]
fn automation_frontloaded_freezes_notes_and_commits_duration_with_task() {
    let (_directory, database, input) = setup();
    let run = database.create_automation(input, account()).unwrap();
    let id = database.start_automation_generation(&run).unwrap();
    assert!(database.start_automation_generation(&run).is_err());
    let mut changed = database.get_project(&run.project_id).unwrap().input;
    changed.source_notes = "启动后另写的资料".into();
    database
        .save_project(Some(&run.project_id), changed)
        .unwrap();
    let prepared = database.prepare_generation(&id).unwrap();
    let user: serde_json::Value = serde_json::from_str(&prepared.user).unwrap();
    assert_eq!(user["book"]["sourceNotes"], "原始资料");
    assert_eq!(user["durationLimits"]["maxMinutes"], 15);
    assert!(database
        .finish_duration_recommendation(
            &id,
            crate::ai::audio::DurationRecommendation {
                minutes: 20,
                reason: "超出范围".into()
            }
        )
        .is_err());
    assert!(database
        .automation_run(&run.id)
        .unwrap()
        .input
        .target_duration_seconds
        .is_none());
    assert_eq!(database.automation_ai_result(&id).unwrap().0, "running");
    assert!(database
        .get_project(&run.project_id)
        .unwrap()
        .audio_duration_source
        .is_none());
    database.control_automation(&run.id, "pause").unwrap();
    database
        .finish_duration_recommendation(
            &id,
            crate::ai::audio::DurationRecommendation {
                minutes: 13,
                reason: "资料不足，按已知重点估计".into(),
            },
        )
        .unwrap();
    let saved = database.automation_run(&run.id).unwrap();
    assert_eq!(saved.status, "paused");
    assert_eq!(saved.input.target_duration_seconds, Some(780));
    assert_eq!(saved.input.duration_reason, "资料不足，按已知重点估计");
    assert_eq!(
        database
            .get_project(&run.project_id)
            .unwrap()
            .input
            .target_duration_seconds,
        780
    );
    assert_eq!(database.automation_ai_result(&id).unwrap().0, "succeeded");
    assert!(database.set_automation_stage(&run, "writing").is_err());
}

#[test]
fn automation_frontloaded_cancel_rejects_late_result_and_stale_binding() {
    let (_directory, database, input) = setup();
    let run = database.create_automation(input, account()).unwrap();
    let id = database.start_automation_generation(&run).unwrap();
    database.prepare_generation(&id).unwrap();
    database.control_automation(&run.id, "cancel").unwrap();
    assert!(database
        .finish_duration_recommendation(
            &id,
            crate::ai::audio::DurationRecommendation {
                minutes: 12,
                reason: "迟到结果".into()
            }
        )
        .is_err());
    assert!(database
        .automation_run(&run.id)
        .unwrap()
        .input
        .target_duration_seconds
        .is_none());
    assert!(database.start_automation_generation(&run).is_err());
    assert!(database
        .list_versions(&run.project_id, "publishing")
        .unwrap()
        .is_empty());
}

#[test]
fn automation_frontloaded_restart_keeps_creation_and_requires_explicit_retry() {
    let (directory, database, input) = setup();
    let run = database.create_automation(input, account()).unwrap();
    let id = database.start_automation_generation(&run).unwrap();
    database.prepare_generation(&id).unwrap();
    drop(database);
    let database = Database::open(directory.path().join("data.sqlite3"));
    database.recover_ai_tasks().unwrap();
    database.recover_automation().unwrap();
    let paused = database.automation_run(&run.id).unwrap();
    assert_eq!(paused.duration_task_id.as_deref(), Some(id.as_str()));
    assert_eq!(database.automation_ai_result(&id).unwrap().0, "failed");
    assert!(database.prepare_generation(&id).is_err());
    database.retry_automation_generation(&paused).unwrap();
    assert!(database.prepare_generation(&id).is_err());
    let resumed = database.control_automation(&run.id, "resume").unwrap();
    database.prepare_generation(&id).unwrap();
    database
        .finish_duration_recommendation(
            &id,
            crate::ai::audio::DurationRecommendation {
                minutes: 12,
                reason: "恢复原任务".into(),
            },
        )
        .unwrap();
    database.set_automation_stage(&resumed, "writing").unwrap();
    let ready = database.automation_run(&run.id).unwrap();
    let ready = finish_creation(&database, &ready);
    let version = ready.writing_version_id.clone();
    database.control_automation(&run.id, "pause").unwrap();
    let backup = database
        .create_backup(directory.path().to_str().unwrap())
        .unwrap();
    let snapshot = super::backup::read_snapshot(Path::new(&backup)).unwrap();
    let saved: String = snapshot.query_row("SELECT version_id FROM ai_tasks WHERE id=(SELECT writing_task_id FROM automation_runs WHERE id=?1)", [&run.id], |row| row.get(0)).unwrap();
    assert_eq!(Some(saved), version);
    assert_eq!(database.automation_run(&run.id).unwrap().stage, "audio");
}

#[test]
fn automation_frontloaded_blocks_manual_duplicate_generation() {
    let (_directory, database, input) = setup();
    let run = database.create_automation(input, account()).unwrap();
    let request = ai::GenerationInput {
        project_id: run.project_id.clone(),
        provider_id: run.input.selection.text_provider_id.clone(),
        recommend_duration: true,
        target_duration_seconds: None,
        revision: None,
    };
    assert!(database.start_generation(request.clone()).is_err());
    database.control_automation(&run.id, "cancel").unwrap();
    assert!(database.start_generation(request).is_ok());
    let mut next = run.input.selection;
    next.request_key = "next-run".into();
    assert!(database.create_automation(next, account()).is_err());
}

#[test]
fn automation_v21_upgrade_preserves_post_audio_writing_and_publish_markers() {
    let (directory, database, input) = setup();
    let run = database.create_automation(input, account()).unwrap();
    let run = finish_creation(&database, &run);
    freeze_v20_schema(&database, false);
    database.with_connection(|connection| {
        super::migrate(connection)?;
        connection.execute_batch("ALTER TABLE automation_runs DROP COLUMN duration_task_id; ALTER TABLE automation_runs DROP COLUMN content_check_task_id; ALTER TABLE automation_runs DROP COLUMN widescreen_task_id; ALTER TABLE automation_runs DROP COLUMN widescreen_cover_id; ALTER TABLE automation_runs DROP COLUMN render_plan_sha256;").map_err(|e|e.to_string())?;
        connection.execute("UPDATE automation_runs SET writing_task_id=?1,input_json=json_remove(input_json,'$.workflowVersion'),status='awaiting_publish',publish_started=1,publish_handed_off=1",[run.writing_task_id.as_deref().unwrap()]).map_err(|e|e.to_string())?;
        connection.pragma_update(None,"user_version",21).map_err(|e|e.to_string())
    }).unwrap();
    let backup = database
        .create_backup(directory.path().to_str().unwrap())
        .unwrap();
    drop(database);
    let database = Database::open(directory.path().join("data.sqlite3"));
    let saved = database.automation_run(&run.id).unwrap();
    assert_eq!(saved.status, "paused");
    assert_eq!(saved.content_check_task_id, run.writing_task_id);
    assert!(saved.writing_task_id.is_none());
    assert_eq!(
        database.automation_publish_writing(&saved).unwrap().id,
        run.writing_version_id.unwrap()
    );
    assert!(saved.publish_started);
    assert!(database.control_automation(&saved.id, "resume").is_err());
    let snapshot = super::backup::read_snapshot(Path::new(&backup)).unwrap();
    assert_eq!(
        snapshot
            .query_row(
                "SELECT publish_handed_off FROM automation_runs",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
        1
    );
    assert_eq!(
        Connection::open(backup)
            .unwrap()
            .pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
            .unwrap(),
        21
    );
}

#[test]
fn automation_v22_migration_conflict_rolls_back_without_changing_v21() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("conflict.sqlite3");
    let mut connection = Connection::open(&path).unwrap();
    for sql in super::MIGRATIONS.iter().take(21) {
        connection.execute_batch(sql).unwrap();
    }
    connection.pragma_update(None, "user_version", 21).unwrap();
    connection
        .execute_batch("ALTER TABLE automation_runs ADD COLUMN content_check_task_id TEXT;")
        .unwrap();
    assert!(super::migrate(&mut connection).is_err());
    assert_eq!(
        connection
            .pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
            .unwrap(),
        21
    );
    assert!(connection
        .prepare("SELECT duration_task_id FROM automation_runs")
        .is_err());
    assert!(connection
        .prepare("SELECT writing_task_id,publish_started,publish_handed_off FROM automation_runs")
        .is_ok());
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
    assert_eq!(run.input.target_duration_seconds, None);
    assert_eq!(run.input.account.key, account().key);
}

#[test]
fn automatic_audio_uses_saved_creation_and_frozen_duration() {
    let (_directory, database, input) = setup();
    database.set_audio_duration(&input.project_id, 900).unwrap();
    let run = database.create_automation(input, account()).unwrap();
    let run = finish_creation(&database, &run);
    database.set_audio_duration(&run.project_id, 300).unwrap();
    assert!(database
        .start_audio_job(audio::StartAudioJobInput {
            project_id: run.project_id.clone(),
            writing_version_id: run.writing_version_id.clone().unwrap(),
            source: audio::AudioSourceInput::Asset {
                asset_id: run.input.selection.source_asset_id.clone()
            },
            format: "deep_dive".into(),
        })
        .is_err());
    let audio = database.start_automatic_audio(&run).unwrap();
    assert_eq!(audio.writing_version_id, run.writing_version_id);
    assert_eq!(audio.target_duration_seconds, 900);
    assert_eq!(
        database
            .automation_run(&run.id)
            .unwrap()
            .audio_job_id
            .as_deref(),
        Some(audio.id.as_str())
    );
    assert_eq!(
        database
            .list_versions(&run.project_id, "publishing")
            .unwrap()
            .len(),
        1
    );
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
    assert!(frozen.contains(crate::ai::PROMPT_VERSION));
    assert!(frozen.contains(&run.input.source_sha256));
    assert!(frozen.contains("本次保存的完整指导"));
    assert!(database.start_automatic_audio(&run).is_err());
}

#[test]
fn automatic_audio_rejects_changed_source_and_paused_stale_run() {
    let (directory, database, input) = setup();
    let run = database.create_automation(input, account()).unwrap();
    let run = finish_creation(&database, &run);
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
                23
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
    let run = finish_creation(&database, &run);
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
    assert_eq!(upgraded.input.workflow_version, 0);
    assert!(database.control_automation(&run.id, "resume").is_err());
    assert_eq!(
        database.audio_job(&audio.id).unwrap().writing_version_id,
        run.writing_version_id
    );
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

fn transcript_fixture() -> (tempfile::TempDir, Database, AutomationRun) {
    let (directory, database, input) = setup();
    let run = database.create_automation(input, account()).unwrap();
    let run = finish_creation(&database, &run);
    let audio = database.start_automatic_audio(&run).unwrap();
    let path = directory.path().join("original.wav");
    fs::write(&path, b"audio fixture").unwrap();
    let asset = database
        .link_asset(&run.project_id, path.to_str().unwrap(), None)
        .unwrap();
    let hash = super::audio::hash_file(&path).unwrap().1;
    database
        .with_connection(|c| {
            c.execute(
                "UPDATE audio_jobs SET status='succeeded',asset_id=?1,output_sha256=?2 WHERE id=?3",
                rusqlite::params![asset, hash, audio.id],
            )
            .map_err(|e| e.to_string())?;
            Ok(())
        })
        .unwrap();
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
    let run = database.automation_run(&run.id).unwrap();
    let video = database
        .claim_bound_video_job(
            &video.id,
            crate::video::VideoWork::Transcript,
            Some(&run.input.selection.text_provider_id),
            Some(&run),
        )
        .unwrap();
    assert!(database
        .finish_video_transcript(&video.id, video.run_id)
        .is_err());
    let mut plan = video.plan;
    plan.duration = 2.0;
    plan.cues = vec![crate::video::Cue {
        start: 0.0,
        end: 2.0,
        text: "实际声音里的句子".into(),
    }];
    let mut review = crate::video::subtitles::SubtitleReview::new(Default::default());
    review.completed = 1;
    plan.subtitle_review = Some(review);
    database
        .save_video_progress(&video.id, video.run_id, &plan, "proofreading")
        .unwrap();
    let subtitle = directory.path().join("proofread.srt");
    fs::write(&subtitle, crate::video::to_srt(&plan.cues)).unwrap();
    database
        .register_video_subtitle(&video.id, video.run_id, &subtitle)
        .unwrap();
    database
        .finish_video_transcript(&video.id, video.run_id)
        .unwrap();
    let ready = database.video_job(&video.id).unwrap();
    assert_eq!(ready.status, "draft");
    assert_eq!(ready.stage, "transcript_ready");
    assert!(ready.plan.scenes.is_empty());
    database
        .set_automation_stage(&run, "content_check")
        .unwrap();
    let run = database.automation_run(&run.id).unwrap();
    (directory, database, run)
}

#[test]
fn automatic_content_check_preserves_audio_and_does_not_duplicate_consistent_version() {
    let (_dir, db, run) = transcript_fixture();
    let id = db.create_content_check_task(&run).unwrap();
    let input: serde_json::Value =
        serde_json::from_str(&db.prepare_content_check_task(&id).unwrap()).unwrap();
    assert_eq!(
        input["sourceVersionId"],
        run.writing_version_id.clone().unwrap()
    );
    db.finish_content_check_task(
        &id,
        r#"{"verified":true,"reason":"符合实际讲述","changes":{}}"#,
    )
    .unwrap();
    let run = db.automation_run(&run.id).unwrap();
    assert_eq!(run.final_writing_version_id, run.writing_version_id);
    assert_eq!(
        db.list_versions(&run.project_id, "publishing")
            .unwrap()
            .len(),
        1
    );
    db.verify_automation_content(&run).unwrap();
    assert!(db
        .finish_content_check_task(&id, r#"{"verified":true,"reason":"重复","changes":{}}"#)
        .is_err());
    let mut video = db.video_job(run.video_job_id.as_ref().unwrap()).unwrap();
    video.plan.scenes=vec![serde_json::from_value(serde_json::json!({"first":0,"last":0,"kind":"opening","title":"阅读","beats":[{"cue":0,"object":"book","action":"draw","label":"书"}]})).unwrap()];
    db.with_connection(|c| {
        c.execute(
            "UPDATE video_jobs SET plan_json=?1 WHERE id=?2",
            rusqlite::params![serde_json::to_string(&video.plan).unwrap(), video.id],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    })
    .unwrap();
    db.verify_automation_content(&run).unwrap();
    video.plan.cues[0].text = "改写字幕".into();
    db.with_connection(|c| {
        c.execute(
            "UPDATE video_jobs SET plan_json=?1 WHERE id=?2",
            rusqlite::params![serde_json::to_string(&video.plan).unwrap(), video.id],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    })
    .unwrap();
    assert!(db.verify_automation_content(&run).is_err());
    assert!(db
        .claim_video_job(&video.id, crate::video::VideoWork::Prepare, None)
        .is_err());
}

#[test]
fn automatic_content_check_revises_only_needed_fields_and_rejects_late_changes() {
    let (_dir, db, run) = transcript_fixture();
    let id = db.create_content_check_task(&run).unwrap();
    db.prepare_content_check_task(&id).unwrap();
    assert!(db
        .finish_content_check_task(
            &id,
            r#"{"verified":true,"reason":"不允许","changes":{"audioPrompt":"替换音频指导"}}"#
        )
        .is_err());
    assert_eq!(
        db.list_versions(&run.project_id, "publishing")
            .unwrap()
            .len(),
        1
    );
    db.control_automation(&run.id, "pause").unwrap();
    db.finish_content_check_task(
        &id,
        r#"{"verified":true,"reason":"标题应对应实际内容","changes":{"title":"实际声音里的句子"}}"#,
    )
    .unwrap();
    let saved = db.automation_run(&run.id).unwrap();
    assert_eq!(saved.status, "paused");
    let mut expected =
        serde_json::to_value(db.automation_writing(&saved).unwrap().content).unwrap();
    expected["title"] = serde_json::json!("实际声音里的句子");
    assert_eq!(
        serde_json::to_value(db.automation_publish_writing(&saved).unwrap().content).unwrap(),
        expected
    );
    assert_eq!(
        db.audio_job_prompt(run.audio_job_id.as_ref().unwrap())
            .unwrap(),
        expected["audioPrompt"].as_str().unwrap()
    );
    assert_eq!(
        db.list_versions(&run.project_id, "publishing")
            .unwrap()
            .len(),
        2
    );
}

#[test]
fn automatic_content_check_rejects_subtitle_changes_while_request_is_running() {
    let (_dir, db, run) = transcript_fixture();
    let id = db.create_content_check_task(&run).unwrap();
    db.prepare_content_check_task(&id).unwrap();
    db.with_connection(|c|{c.execute("UPDATE video_jobs SET plan_json=json_set(plan_json,'$.cues[0].text','新字幕') WHERE id=?1",[&run.video_job_id]).map_err(|e|e.to_string())?;Ok(())}).unwrap();
    assert!(db
        .finish_content_check_task(&id, r#"{"verified":true,"reason":"之前一致","changes":{}}"#)
        .is_err());
    assert_eq!(
        db.list_versions(&run.project_id, "publishing")
            .unwrap()
            .len(),
        1
    );
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
fn automatic_three_covers_retry_locally_then_freeze_the_same_video_plan() {
    use super::automation::COVER_STEPS;
    let (_dir, db, run) = transcript_fixture();
    let check = db.create_content_check_task(&run).unwrap();
    db.prepare_content_check_task(&check).unwrap();
    db.finish_content_check_task(&check, r#"{"verified":true,"reason":"一致","changes":{}}"#)
        .unwrap();
    for step in &COVER_STEPS {
        let run = db.automation_run(&run.id).unwrap();
        db.set_automation_stage(&run, step.image_stage).unwrap();
        let image = db
            .start_bound_cover_generation(
                image_generation::ImageGenerationInput {
                    project_id: run.project_id.clone(),
                    provider_id: run.input.selection.image_provider_id.clone(),
                    description: step
                        .prompt(&db.automation_publish_writing(&run).unwrap().content)
                        .unwrap()
                        .into(),
                    width: step.width,
                    height: step.height,
                },
                &run.binding(step.image_slot),
            )
            .unwrap();
        if step.width == 1920 {
            db.end_generation(&image, "failed", "timeout", "第三张背景失败")
                .unwrap();
            db.set_automation_status(&run, "waiting_user", "重试当前背景")
                .unwrap();
            let waiting = db.automation_run(&run.id).unwrap();
            assert!(waiting.landscape_cover_id.is_some() && waiting.portrait_cover_id.is_some());
            assert_eq!(db.list_cover_versions(&run.project_id).unwrap().len(), 2);
            db.retry_automation_generation(&waiting).unwrap();
            db.control_automation(&run.id, "resume").unwrap();
        }
        db.prepare_cover_generation(&image).unwrap();
        let mut background = std::io::Cursor::new(Vec::new());
        image::DynamicImage::new_rgb8(512, 512)
            .write_to(&mut background, image::ImageFormat::Png)
            .unwrap();
        db.finish_cover_generation(&image, background.get_ref())
            .unwrap();
        let run = db.automation_run(&run.id).unwrap();
        db.set_automation_stage(&run, step.cover_stage).unwrap();
        let run = db.automation_run(&run.id).unwrap();
        let config = db.automation_cover_config(&run).unwrap();
        assert_eq!((config.width, config.height), (step.width, step.height));
        assert!(db
            .save_bound_cover_version(
                &run.project_id,
                config.clone(),
                Some(b"invalid"),
                Some(&run.binding(step.cover_slot))
            )
            .is_err());
        db.fail_automation_cover(&run.id, run.run_id, step.cover_stage)
            .unwrap();
        let resumed = db.control_automation(&run.id, "resume").unwrap();
        assert_eq!(db.automation_cover_config(&resumed).unwrap(), config);
        assert_eq!(db.automation_ai_result(&image).unwrap().0, "succeeded");
        let mut png = std::io::Cursor::new(Vec::new());
        image::DynamicImage::new_rgb8(step.width, step.height)
            .write_to(&mut png, image::ImageFormat::Png)
            .unwrap();
        db.save_bound_cover_version(
            &run.project_id,
            config,
            Some(png.get_ref()),
            Some(&resumed.binding(step.cover_slot)),
        )
        .unwrap();
        db.set_automation_stage(&resumed, step.next).unwrap();
    }
    let run = db.automation_run(&run.id).unwrap();
    assert_eq!(run.stage, "storyboard");
    assert_eq!(db.list_cover_versions(&run.project_id).unwrap().len(), 3);
    let images: i64 = db
        .with_connection(|c| {
            c.query_row(
                "SELECT count(*) FROM ai_tasks WHERE kind='cover'",
                [],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())
        })
        .unwrap();
    assert_eq!(images, 3);
    let job = db
        .claim_bound_video_job(
            run.video_job_id.as_ref().unwrap(),
            crate::video::VideoWork::Prepare,
            None,
            Some(&run),
        )
        .unwrap();
    let mut plan = job.plan;
    plan.scenes = vec![serde_json::from_value(serde_json::json!({"first":0,"last":0,"kind":"opening","title":"阅读","beats":[{"cue":0,"object":"book","action":"draw","label":"书"}]})).unwrap()];
    db.save_video_progress(&job.id, job.run_id, &plan, "storyboard")
        .unwrap();
    db.end_video_job(&job.id, job.run_id, "awaiting_confirmation", "")
        .unwrap();
    db.verify_automation_content(&run).unwrap();
    db.prepare_automation_render(&run).unwrap();
    db.verify_automation_video(&run).unwrap();
    let ids: i64 = db
        .with_connection(|c| {
            c.query_row("SELECT count(*) FROM video_jobs", [], |r| r.get(0))
                .map_err(|e| e.to_string())
        })
        .unwrap();
    assert_eq!(ids, 1);
    db.with_connection(|c| { c.execute("UPDATE video_jobs SET plan_json=json_set(plan_json,'$.scenes[0].title','变更后的分镜') WHERE id=?1", [&job.id]).map_err(|e|e.to_string())?; Ok(()) }).unwrap();
    db.verify_automation_content(&run).unwrap();
    assert!(db.verify_automation_video(&run).is_err());
}

#[test]
fn automation_v23_upgrades_old_authorization_and_restores_exact_new_artifacts() {
    let (dir, db, input) = setup();
    let run = db.create_automation(input, account()).unwrap();
    let run = finish_creation(&db, &run);
    let audio = db.start_automatic_audio(&run).unwrap();
    db.with_connection(|c| {
        c.execute_batch("ALTER TABLE automation_runs DROP COLUMN widescreen_task_id; ALTER TABLE automation_runs DROP COLUMN widescreen_cover_id; ALTER TABLE automation_runs DROP COLUMN render_plan_sha256; PRAGMA user_version=22;").map_err(|e|e.to_string())?;
        c.execute("UPDATE automation_runs SET input_json=json_set(input_json,'$.workflowVersion',1),publish_started=1,publish_handed_off=1 WHERE id=?1",[&run.id]).map_err(|e|e.to_string())?;
        Ok(())
    }).unwrap();
    let backup = dir.path().join("old-v22.sqlite3");
    db.with_connection(|c| c.backup("main", &backup, None).map_err(|e| e.to_string()))
        .unwrap();
    db.with_connection(super::migrate).unwrap();
    let upgraded = db.automation_run(&run.id).unwrap();
    assert_eq!(upgraded.status, "paused");
    assert_eq!(upgraded.input.workflow_version, 1);
    assert_eq!(upgraded.writing_version_id, run.writing_version_id);
    assert_eq!(upgraded.audio_job_id.as_deref(), Some(audio.id.as_str()));
    assert!(upgraded.publish_started && upgraded.error.contains("双封面授权"));
    assert!(db.control_automation(&run.id, "resume").is_err());
    db.control_audio_job(&audio.id, "cancel").unwrap();
    db.restore_backup(backup.to_str().unwrap()).unwrap();
    let restored = db.automation_run(&run.id).unwrap();
    assert_eq!(restored.writing_version_id, run.writing_version_id);
    assert!(restored.publish_started);
    assert!(db.control_automation(&run.id, "resume").is_err());
    let original = Connection::open(&backup).unwrap();
    assert_eq!(
        original
            .pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
            .unwrap(),
        22
    );
    assert_eq!(
        db.with_connection(|c| c
            .query_row(
                "SELECT publish_handed_off FROM automation_runs WHERE id=?1",
                [&run.id],
                |r| r.get::<_, i64>(0)
            )
            .map_err(|e| e.to_string()))
            .unwrap(),
        1
    );
}

#[test]
fn automation_v23_migration_failure_preserves_v22_schema_and_data() {
    let mut connection = Connection::open_in_memory().unwrap();
    for sql in MIGRATIONS.iter().take(22) {
        connection.execute_batch(sql).unwrap();
    }
    connection.pragma_update(None, "user_version", 22).unwrap();
    connection
        .execute_batch("ALTER TABLE automation_runs ADD COLUMN widescreen_cover_id TEXT;")
        .unwrap();
    assert!(super::migrate(&mut connection).is_err());
    assert_eq!(
        connection
            .pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
            .unwrap(),
        22
    );
    assert!(!connection.query_row("SELECT EXISTS(SELECT 1 FROM pragma_table_info('automation_runs') WHERE name='widescreen_task_id')", [], |r|r.get::<_,bool>(0)).unwrap());
}

#[test]
fn automatic_audio_stop_prevents_late_source_and_generation_actions() {
    let (_directory, database, input) = setup();
    let run = database.create_automation(input, account()).unwrap();
    let run = finish_creation(&database, &run);
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
