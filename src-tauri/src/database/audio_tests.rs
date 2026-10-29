use super::*;
use crate::database::{
    ai::WritingVersion,
    projects::{Project, ProjectInput},
    Database,
};
use rusqlite::Connection;
use std::{fs, path::Path};

#[test]
fn notebook_identity_survives_naming_failure_and_restart() {
    let directory = tempfile::tempdir().unwrap();
    let database = Database::open(directory.path().join("data.sqlite3"));
    let project = sample_project(&database, "命名恢复", "原书资料");
    let writing = sample_writing(&database, &project, "完整指导");
    let original = start_text_job(&database, &project, &writing);
    database
        .observe_audio_job(&original.id, original.run_id, &AudioObservation::LoginReady)
        .unwrap();
    let identified: AudioObservation = serde_json::from_value(serde_json::json!({
        "kind": "notebook_identified", "notebookUrl": "https://notebook.google.com/notebook/original"
    })).unwrap();
    let saved = database
        .observe_audio_job(&original.id, original.run_id, &identified)
        .unwrap();
    assert_eq!(saved.stage, "creating_notebook");
    assert!(saved.notebook_url.is_some());
    database
        .audio_step_failed(&original.id, original.run_id, "标题尚未加载")
        .unwrap();
    let resumed = database.control_audio_job(&original.id, "resume").unwrap();
    assert_eq!(resumed.stage, "creating_notebook");
    assert_eq!(resumed.notebook_url, saved.notebook_url);
    database.recover_audio_jobs().unwrap();
    let restarted = database.control_audio_job(&original.id, "resume").unwrap();
    assert_eq!(restarted.stage, "creating_notebook");
    assert_eq!(restarted.notebook_url, saved.notebook_url);
    assert!(database
        .observe_audio_job(
            &original.id,
            restarted.run_id,
            &AudioObservation::GenerationSubmitted
        )
        .is_err());
    let named = database
        .observe_audio_job(
            &original.id,
            restarted.run_id,
            &AudioObservation::NotebookCreated {
                notebook_url: saved.notebook_url.unwrap(),
            },
        )
        .unwrap();
    assert_eq!(named.stage, "adding_source");
    assert!(named.submitted_at.is_none());
}

#[test]
fn manual_empty_notebook_confirmation_recovers_only_the_original_unsubmitted_job() {
    let directory = tempfile::tempdir().unwrap();
    let database = Database::open(directory.path().join("data.sqlite3"));
    let project = sample_project(&database, "旧任务恢复", "原书资料");
    let writing = sample_writing(&database, &project, "完整指导");
    let original = start_text_job(&database, &project, &writing);
    database
        .observe_audio_job(&original.id, original.run_id, &AudioObservation::LoginReady)
        .unwrap();
    database
        .audio_step_failed(&original.id, original.run_id, "命名失败")
        .unwrap();
    let url = "https://notebook.google.com/notebook/confirmed";
    assert!(database
        .confirm_audio_notebook(&original.id, original.run_id + 1, url)
        .is_err());
    assert!(database
        .confirm_audio_notebook(
            &original.id,
            original.run_id,
            "https://example.com/notebook/other"
        )
        .is_err());
    let recovered = database
        .confirm_audio_notebook(&original.id, original.run_id, url)
        .unwrap();
    assert_eq!(recovered.id, original.id);
    assert_eq!(recovered.writing_version_id, original.writing_version_id);
    assert_eq!(recovered.created_at, original.created_at);
    assert_eq!(recovered.stage, "creating_notebook");
    assert_eq!(recovered.status, "preparing");
    assert_eq!(recovered.notebook_url.as_deref(), Some(url));
    assert!(recovered.submitted_at.is_none());
    assert!(database
        .observe_audio_job(
            &original.id,
            original.run_id,
            &AudioObservation::NotebookCreated {
                notebook_url: url.into()
            }
        )
        .is_err());
    database.control_audio_job(&original.id, "pause").unwrap();
    assert!(database
        .confirm_audio_notebook(&original.id, recovered.run_id + 1, url)
        .is_err());
    let resumed = database.control_audio_job(&original.id, "resume").unwrap();
    assert!(database
        .observe_audio_job(
            &original.id,
            resumed.run_id,
            &AudioObservation::NotebookCreated {
                notebook_url: "https://notebook.google.com/notebook/other".into()
            }
        )
        .is_err());
    database.control_audio_job(&original.id, "cancel").unwrap();
    let next = start_text_job(&database, &project, &writing);
    database
        .observe_audio_job(&next.id, next.run_id, &AudioObservation::LoginReady)
        .unwrap();
    database
        .audio_step_failed(&next.id, next.run_id, "命名失败")
        .unwrap();
    database
        .with_connection(|connection| {
            connection
                .execute(
                    "UPDATE audio_jobs SET submitted_at=created_at WHERE id=?1",
                    [&next.id],
                )
                .map_err(|e| e.to_string())?;
            Ok(())
        })
        .unwrap();
    assert!(database
        .confirm_audio_notebook(&next.id, next.run_id, url)
        .is_err());
}

#[test]
fn browser_recovery_invalidates_old_actions_and_retains_the_original_job() {
    for action in ["refresh_browser", "reopen_browser"] {
        let directory = tempfile::tempdir().unwrap();
        let database = Database::open(directory.path().join("data.sqlite3"));
        let project = sample_project(&database, "恢复测试", "原书资料");
        let writing = sample_writing(&database, &project, "完整指导");
        let original = start_text_job(&database, &project, &writing);
        let paused = database.control_audio_job(&original.id, action).unwrap();
        assert_eq!(paused.status, "paused");
        assert_eq!(paused.stage, "opening_site");
        assert_eq!(paused.run_id, original.run_id + 1);
        assert_eq!(paused.writing_version_id, original.writing_version_id);
        assert_eq!(paused.created_at, original.created_at);
        assert!(database
            .observe_audio_job(&original.id, original.run_id, &AudioObservation::LoginReady)
            .is_err());
        let resumed = database.control_audio_job(&original.id, "resume").unwrap();
        assert_eq!(resumed.stage, "opening_site");
        assert_eq!(resumed.id, original.id);
        assert!(resumed.submitted_at.is_none());
    }
}

#[test]
fn recovery_never_resubmits_an_uncertain_generation_or_source_upload() {
    for stage in ["adding_source", "submitting"] {
        let directory = tempfile::tempdir().unwrap();
        let database = Database::open(directory.path().join("data.sqlite3"));
        let project = sample_project(&database, "提交恢复", "原书资料");
        let writing = sample_writing(&database, &project, "完整指导");
        let original = start_text_job(&database, &project, &writing);
        database.with_connection(|connection| {
            connection.execute("UPDATE audio_jobs SET status='preparing',stage=?2,notebook_url='https://notebook.google/notebook/task' WHERE id=?1", params![original.id, stage]).map_err(|e| e.to_string())?;
            Ok(())
        }).unwrap();
        database
            .control_audio_job(&original.id, "refresh_browser")
            .unwrap();
        let resumed = database.control_audio_job(&original.id, "resume").unwrap();
        assert_eq!(resumed.stage, "verify_remote");
        assert_eq!(
            resumed.notebook_url.as_deref(),
            Some("https://notebook.google/notebook/task")
        );
        assert!(database
            .observe_audio_job(
                &original.id,
                resumed.run_id,
                &AudioObservation::GenerationSubmitted
            )
            .is_err());
    }
}

#[test]
fn stopped_jobs_can_resume_but_cannot_take_over_another_active_job() {
    let directory = tempfile::tempdir().unwrap();
    let database = Database::open(directory.path().join("data.sqlite3"));
    let project = sample_project(&database, "停止恢复", "原书资料");
    let writing = sample_writing(&database, &project, "完整指导");
    let original = start_text_job(&database, &project, &writing);
    database.control_audio_job(&original.id, "cancel").unwrap();
    assert_eq!(
        database
            .control_audio_job(&original.id, "refresh_browser")
            .unwrap()
            .status,
        "cancelled"
    );
    let resumed = database.control_audio_job(&original.id, "resume").unwrap();
    assert_eq!(resumed.id, original.id);
    database.control_audio_job(&original.id, "cancel").unwrap();
    let other = start_text_job(&database, &project, &writing);
    for action in [
        "resume",
        "open_browser",
        "refresh_browser",
        "reopen_browser",
        "cancel",
    ] {
        let error = database
            .control_audio_job(&original.id, action)
            .unwrap_err();
        assert!(error.contains("另一个音频任务"));
    }
    assert_eq!(
        database.audio_execution_job().unwrap().unwrap().id,
        other.id
    );
}

#[test]
fn stopped_task_survives_restart_and_resumes_download_without_regenerating() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("data.sqlite3");
    let database = Database::open(path.clone());
    let project = sample_project(&database, "停止后继续", "原书资料");
    let writing = sample_writing(&database, &project, "完整指导");
    let original = start_text_job(&database, &project, &writing);
    advance_to_download(&database, &original);
    let stopped = database.control_audio_job(&original.id, "cancel").unwrap();
    drop(database);

    let database = Database::open(path);
    database.recover_audio_jobs().unwrap();
    let snapshot = database.audio_job_data(&project.id).unwrap();
    assert!(snapshot.active_job.is_none());
    assert_eq!(snapshot.job.unwrap().status, "cancelled");
    let resumed = database.control_audio_job(&original.id, "resume").unwrap();
    assert_eq!(resumed.id, original.id);
    assert_eq!(resumed.status, "downloading");
    assert_eq!(resumed.stage, "downloading");
    assert_eq!(resumed.notebook_url, stopped.notebook_url);
    assert_eq!(resumed.submitted_at, stopped.submitted_at);
    assert!(resumed.submitted_at.is_some());
    assert!(resumed.run_id > stopped.run_id);
    let execution = database.audio_execution_job().unwrap().unwrap();
    assert_eq!(execution.prompt, "完整指导");
    assert!(matches!(execution.source, AudioSourceData::Text { ref text } if text == "原书资料"));
    assert!(database
        .observe_audio_job(
            &resumed.id,
            resumed.run_id,
            &AudioObservation::GenerationSubmitted
        )
        .is_err());
    assert!(database
        .audio_download_started(&original.id, original.run_id)
        .is_err());

    database
        .audio_download_started(&resumed.id, resumed.run_id)
        .unwrap();
    let download = database
        .audio_download_path(&resumed.id, resumed.run_id)
        .unwrap();
    fs::create_dir_all(download.parent().unwrap()).unwrap();
    fs::write(&download, wav_bytes()).unwrap();
    let completed = database
        .complete_audio_download(&resumed.id, resumed.run_id, download.to_str().unwrap())
        .unwrap();
    assert_eq!(completed.id, original.id);
    assert_eq!(completed.status, "succeeded");
    assert_eq!(
        database
            .list_assets(&project.id)
            .unwrap()
            .iter()
            .filter(|asset| asset.asset_type == "audio")
            .count(),
        1
    );
}

#[test]
fn official_notebook_domains_can_be_saved_and_reopened() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("data.sqlite3");
    let database = Database::open(path.clone());
    let project = sample_project(&database, "网站跳转", "本地资料");
    let writing = sample_writing(&database, &project, "完整指导");
    for host in [
        "notebook.google.com",
        "notebooklm.google.com",
        "notebook.google",
        "notebooklm.google",
    ] {
        let job = start_text_job(&database, &project, &writing);
        database
            .observe_audio_job(&job.id, job.run_id, &AudioObservation::LoginReady)
            .unwrap();
        let url = format!("https://{host}/notebook/task");
        database
            .observe_audio_job(
                &job.id,
                job.run_id,
                &AudioObservation::NotebookCreated {
                    notebook_url: url.clone(),
                },
            )
            .unwrap();
        let reopened = Database::open(path.clone());
        let saved = reopened.audio_job_data(&project.id).unwrap().job.unwrap();
        assert_eq!(saved.notebook_url.as_deref(), Some(url.as_str()));
        database.control_audio_job(&job.id, "cancel").unwrap();
    }
    for url in [
        "https://notebook.google.evil.test/notebook/task",
        "https://notebooklm.google:444/notebook/task",
        "https://user@notebook.google/notebook/task",
        "https://notebook.google/?location=unsupported",
    ] {
        assert!(!validate_notebook_url(url));
    }
}

#[test]
fn bound_book_document_is_available_to_audio_without_linking_again() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("data.sqlite3");
    let database = Database::open(path.clone());
    let source = directory.path().join("绑定原书.txt");
    fs::write(&source, b"book content").unwrap();
    let project = sample_project(&database, "绑定来源", "");
    let project = database
        .save_project_with_documents(
            Some(&project.id),
            project.input,
            &[source.to_str().unwrap().to_owned()],
        )
        .unwrap();
    let writing = sample_writing(&database, &project, "完整指导");
    drop(database);
    let database = Database::open(path);
    let assets = database.list_assets(&project.id).unwrap();
    let job = database
        .start_audio_job(StartAudioJobInput {
            project_id: project.id,
            writing_version_id: writing.id,
            source: AudioSourceInput::Asset {
                asset_id: assets[0].id.clone(),
            },
            format: "deep_dive".into(),
        })
        .unwrap();
    assert_eq!(job.source_label, "绑定原书.txt");
    let job = database
        .observe_audio_job(&job.id, job.run_id, &AudioObservation::LoginReady)
        .unwrap();
    let job = database
        .observe_audio_job(
            &job.id,
            job.run_id,
            &AudioObservation::NotebookCreated {
                notebook_url: "https://notebook.google.com/notebook/source".into(),
            },
        )
        .unwrap();
    let snapshot = database.audio_source_snapshot(&job.id, job.run_id).unwrap();
    assert_eq!(fs::read(snapshot).unwrap(), b"book content");
}

#[test]
fn start_audio_job_accepts_frontend_source_json() {
    for source in [
        serde_json::json!({ "kind": "asset", "assetId": "document-1" }),
        serde_json::json!({ "kind": "text" }),
    ] {
        let payload = serde_json::json!({
            "projectId": "project-1",
            "writingVersionId": "version-1",
            "source": source,
            "format": "deep_dive"
        });
        let input: StartAudioJobInput = serde_json::from_value(payload.clone()).unwrap();
        assert_eq!(serde_json::to_value(input).unwrap(), payload);
    }
    assert!(
        serde_json::from_value::<AudioSourceInput>(serde_json::json!({
            "kind": "asset"
        }))
        .is_err()
    );
}

#[test]
fn native_page_observation_accepts_the_actual_javascript_contract() {
    let observation: AudioObservation = serde_json::from_value(serde_json::json!({
        "kind": "notebook_created",
        "notebookUrl": "https://notebook.google.com/notebook/task"
    }))
    .unwrap();
    assert_eq!(
        observation,
        AudioObservation::NotebookCreated {
            notebook_url: "https://notebook.google.com/notebook/task".into()
        }
    );
}

fn sample_project(database: &Database, title: &str, notes: &str) -> Project {
    database
        .save_project(
            None,
            ProjectInput {
                title: title.into(),
                author: String::new(),
                publisher: String::new(),
                isbn: String::new(),
                description: String::new(),
                source_notes: notes.into(),
                theme: String::new(),
                target_audience: String::new(),
                content_style: String::new(),
                target_duration_seconds: 1_500,
                status: "draft".into(),
            },
        )
        .unwrap()
}

fn sample_writing(database: &Database, project: &Project, prompt: &str) -> WritingVersion {
    database
        .save_writing(
            serde_json::from_value(serde_json::json!({
                "projectId": project.id,
                "kind": "publishing",
                "content": {
                    "format": "publishing",
                    "title": "发布标题",
                    "description": "发布内容",
                    "audioPrompt": prompt
                },
                "basedOn": null,
                "isConfirmed": false
            }))
            .unwrap(),
        )
        .unwrap()
}

fn start_text_job(database: &Database, project: &Project, writing: &WritingVersion) -> AudioJob {
    database
        .start_audio_job(StartAudioJobInput {
            project_id: project.id.clone(),
            writing_version_id: writing.id.clone(),
            source: AudioSourceInput::Text,
            format: "deep_dive".into(),
        })
        .unwrap()
}

fn advance_to_download(database: &Database, job: &AudioJob) {
    let id = &job.id;
    let run = job.run_id;
    database
        .observe_audio_job(id, run, &AudioObservation::LoginReady)
        .unwrap();
    database
        .observe_audio_job(
            id,
            run,
            &AudioObservation::NotebookCreated {
                notebook_url: "https://notebook.google.com/notebook/task".into(),
            },
        )
        .unwrap();
    database
        .observe_audio_job(id, run, &AudioObservation::SourceSubmitted)
        .unwrap();
    database
        .observe_audio_job(id, run, &AudioObservation::SourceReady)
        .unwrap();
    database
        .observe_audio_job(id, run, &AudioObservation::AudioConfigured)
        .unwrap();
    database
        .observe_audio_job(id, run, &AudioObservation::GenerationSubmitted)
        .unwrap();
    database
        .observe_audio_job(id, run, &AudioObservation::Generating)
        .unwrap();
    database
        .observe_audio_job(
            id,
            run,
            &AudioObservation::GenerationComplete {
                download_url: None,
                suggested_filename: None,
            },
        )
        .unwrap();
    assert_eq!(
        database
            .audio_job_data(&job.project_id)
            .unwrap()
            .job
            .unwrap()
            .stage,
        "downloading"
    );
}

#[test]
fn download_started_requires_current_run_and_download_phase() {
    let directory = tempfile::tempdir().unwrap();
    let database = Database::open(directory.path().join("data.sqlite3"));
    let project = sample_project(&database, "下载任务", "资料");
    let writing = sample_writing(&database, &project, "完整指导");
    let job = start_text_job(&database, &project, &writing);

    assert!(database
        .audio_download_started(&job.id, job.run_id)
        .unwrap_err()
        .contains("还没有进入下载阶段"));
    advance_to_download(&database, &job);
    assert!(database
        .audio_download_started(&job.id, job.run_id + 1)
        .is_err());

    let started = database
        .audio_download_started(&job.id, job.run_id)
        .unwrap();
    assert_eq!(started.status, "downloading");
    assert_eq!(started.stage, "downloading");
    assert!(started.last_checked_at.is_some());
    assert!(started.asset_id.is_none());

    database.control_audio_job(&job.id, "cancel").unwrap();
    assert!(database
        .audio_download_started(&job.id, job.run_id)
        .is_err());
}

fn wav_bytes() -> Vec<u8> {
    let mut bytes = Vec::new();
    let data = vec![0u8; 1_600];
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&((36 + data.len()) as u32).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&8_000u32.to_le_bytes());
    bytes.extend_from_slice(&16_000u32.to_le_bytes());
    bytes.extend_from_slice(&2u16.to_le_bytes());
    bytes.extend_from_slice(&16u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&(data.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&data);
    bytes
}

#[test]
fn migrates_v12_database_to_v13_without_losing_projects() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("v12.sqlite3");
    {
        let connection = Connection::open(&path).unwrap();
        for sql in crate::database::MIGRATIONS.iter().take(12) {
            connection.execute_batch(sql).unwrap();
        }
        connection
            .pragma_update(None, "user_version", 12u32)
            .unwrap();
        connection
            .execute(
                "INSERT INTO book_projects(id,title,author,publisher,isbn,description,source_notes,theme,target_audience,content_style,target_duration_seconds,status,workspace_path,created_at,updated_at) VALUES('p1','旧书','','','','','旧资料','','','',600,'draft','D:/old','2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
                [],
            )
            .unwrap();
    }
    let database = Database::open(path.clone());
    assert_eq!(database.get_project("p1").unwrap().input.title, "旧书");
    let connection = Connection::open(path).unwrap();
    assert_eq!(
        connection
            .pragma_query_value(None, "user_version", |row| row.get::<_, u32>(0))
            .unwrap(),
        13
    );
    assert_eq!(
        connection
            .query_row("SELECT count(*) FROM audio_jobs", [], |row| row
                .get::<_, u32>(0))
            .unwrap(),
        0
    );
}

#[test]
fn freezes_prompt_source_and_duration_and_rejects_a_second_book() {
    let directory = tempfile::tempdir().unwrap();
    let database = Database::open(directory.path().join("data.sqlite3"));
    let project = sample_project(&database, "书一", "第一版资料");
    let writing = sample_writing(&database, &project, "第一版完整指导");
    let job = start_text_job(&database, &project, &writing);
    assert_eq!(job.id, start_text_job(&database, &project, &writing).id);
    let bridge = database.audio_execution_job().unwrap().unwrap();
    assert_eq!(bridge.prompt, "第一版完整指导");
    assert_eq!(bridge.target_duration_seconds, 1_500);
    assert!(matches!(bridge.source, AudioSourceData::Text { ref text } if text == "第一版资料"));

    let project_file = project.workspace_path.clone();
    let input: ProjectInput = serde_json::from_value(serde_json::json!({
        "title": "书一", "author": "", "publisher": "", "isbn": "", "description": "",
        "sourceNotes": "后来改写", "theme": "", "targetAudience": "", "contentStyle": "",
        "targetDurationSeconds": 900, "status": "draft"
    }))
    .unwrap();
    database.save_project(Some(&project.id), input).unwrap();
    assert_eq!(
        database.audio_execution_job().unwrap().unwrap().prompt,
        "第一版完整指导"
    );
    assert_eq!(
        database.get_project(&project.id).unwrap().workspace_path,
        project_file
    );

    let other = sample_project(&database, "书二", "其他资料");
    let other_writing = sample_writing(&database, &other, "其他指导");
    let error = database
        .start_audio_job(StartAudioJobInput {
            project_id: other.id,
            writing_version_id: other_writing.id,
            source: AudioSourceInput::Text,
            format: "deep_dive".into(),
        })
        .unwrap_err();
    assert!(error.contains("书一"));
    database.control_audio_job(&job.id, "cancel").unwrap();
    assert!(database
        .audio_job_data(&project.id)
        .unwrap()
        .active_job
        .is_none());
}

#[test]
fn source_snapshot_verifies_size_and_hash_and_rejects_cross_book_assets() {
    let directory = tempfile::tempdir().unwrap();
    let database = Database::open(directory.path().join("data.sqlite3"));
    let project = sample_project(&database, "原书任务", "资料");
    let other = sample_project(&database, "其他书", "其他");
    let writing = sample_writing(&database, &project, "完整指导");
    let source = Path::new(&project.workspace_path).join("原书.docx");
    let bytes = vec![b'a'; 600_000];
    fs::write(&source, &bytes).unwrap();
    let asset_id = database
        .link_asset(&project.id, source.to_str().unwrap(), None)
        .unwrap();
    let other_source = Path::new(&other.workspace_path).join("另一本.pdf");
    fs::write(&other_source, b"other").unwrap();
    let foreign = database
        .link_asset(&other.id, other_source.to_str().unwrap(), None)
        .unwrap();
    assert!(database
        .start_audio_job(StartAudioJobInput {
            project_id: project.id.clone(),
            writing_version_id: writing.id.clone(),
            source: AudioSourceInput::Asset { asset_id: foreign },
            format: "deep_dive".into(),
        })
        .is_err());

    let input = serde_json::from_value(serde_json::json!({
        "projectId": project.id,
        "writingVersionId": writing.id,
        "source": { "kind": "asset", "assetId": asset_id },
        "format": "brief"
    }))
    .unwrap();
    let job = database.start_audio_job(input).unwrap();
    database
        .observe_audio_job(&job.id, job.run_id, &AudioObservation::LoginReady)
        .unwrap();
    database
        .observe_audio_job(
            &job.id,
            job.run_id,
            &AudioObservation::NotebookCreated {
                notebook_url: "https://notebook.google.com/notebook/source".into(),
            },
        )
        .unwrap();
    let snapshot = database.audio_source_snapshot(&job.id, job.run_id).unwrap();
    assert_eq!(fs::read(&snapshot).unwrap(), bytes);
    fs::write(&source, vec![b'b'; 600_000]).unwrap();
    let error = database
        .audio_source_snapshot(&job.id, job.run_id)
        .unwrap_err();
    assert!(error.contains("SHA-256"), "{error}");
    assert_eq!(
        fs::read(&snapshot).unwrap(),
        bytes,
        "原上传快照不得随原书改动"
    );
    database.control_audio_job(&job.id, "pause").unwrap();
    assert!(database.audio_source_snapshot(&job.id, job.run_id).is_err());
}

#[test]
fn archives_verified_audio_once_and_keeps_project_asset_in_sync() {
    let directory = tempfile::tempdir().unwrap();
    let database = Database::open(directory.path().join("data.sqlite3"));
    let project = sample_project(&database, "归档书籍", "资料");
    let writing = sample_writing(&database, &project, "指导");
    let job = start_text_job(&database, &project, &writing);
    advance_to_download(&database, &job);
    let download = database.audio_download_path(&job.id, job.run_id).unwrap();
    fs::create_dir_all(download.parent().unwrap()).unwrap();
    fs::write(&download, wav_bytes()).unwrap();
    let completed = database
        .complete_audio_download(&job.id, job.run_id, download.to_str().unwrap())
        .unwrap();
    assert_eq!(completed.status, "succeeded");
    assert_eq!(completed.stage, "completed");
    assert!(completed.asset_id.is_some());
    assert_eq!(completed.output_size, Some(wav_bytes().len() as i64));
    let audio = database
        .list_assets(&project.id)
        .unwrap()
        .into_iter()
        .find(|asset| asset.asset_type == "audio")
        .unwrap();
    assert!(audio.file_path.ends_with(".wav"));
    assert!(Path::new(&audio.file_path).is_file());
    let repeated = database
        .complete_audio_download(&job.id, job.run_id, download.to_str().unwrap())
        .unwrap();
    assert_eq!(repeated.asset_id, completed.asset_id);
    assert_eq!(
        database
            .list_assets(&project.id)
            .unwrap()
            .iter()
            .filter(|asset| asset.asset_type == "audio")
            .count(),
        1
    );
}

#[test]
fn rejects_html_and_outside_task_paths_without_archiving() {
    let directory = tempfile::tempdir().unwrap();
    let database = Database::open(directory.path().join("data.sqlite3"));
    let project = sample_project(&database, "异常下载", "资料");
    let writing = sample_writing(&database, &project, "指导");
    let job = start_text_job(&database, &project, &writing);
    advance_to_download(&database, &job);
    let outside = directory.path().join("audio.wav");
    fs::write(&outside, wav_bytes()).unwrap();
    assert!(database
        .complete_audio_download(&job.id, job.run_id, outside.to_str().unwrap())
        .is_err());
    let html = database.audio_download_path(&job.id, job.run_id).unwrap();
    fs::create_dir_all(html.parent().unwrap()).unwrap();
    fs::write(&html, b"<html>not audio</html>").unwrap();
    assert!(database
        .complete_audio_download(&job.id, job.run_id, html.to_str().unwrap())
        .is_err());
    assert!(database
        .list_assets(&project.id)
        .unwrap()
        .iter()
        .all(|asset| asset.asset_type != "audio"));
}

#[test]
fn restart_and_backup_recovery_pause_remote_task_for_manual_verification() {
    let directory = tempfile::tempdir().unwrap();
    let database = Database::open(directory.path().join("data.sqlite3"));
    let project = sample_project(&database, "恢复书籍", "资料");
    let writing = sample_writing(&database, &project, "指导");
    let job = start_text_job(&database, &project, &writing);
    database
        .observe_audio_job(&job.id, job.run_id, &AudioObservation::LoginReady)
        .unwrap();
    let backup = database
        .create_backup(directory.path().to_str().unwrap())
        .unwrap();
    database.control_audio_job(&job.id, "cancel").unwrap();
    database.restore_backup(&backup).unwrap();
    let recovered = database.audio_job_data(&project.id).unwrap().job.unwrap();
    assert_eq!(recovered.status, "waiting_user");
    assert_eq!(recovered.stage, "verify_remote");
    assert!(recovered.run_id > 1);
    database.control_audio_job(&recovered.id, "cancel").unwrap();
    database.recover_audio_jobs().unwrap();
    assert_eq!(
        database
            .audio_job_data(&project.id)
            .unwrap()
            .job
            .unwrap()
            .status,
        "cancelled"
    );
}

#[test]
fn verify_remote_allows_only_read_only_generation_observations() {
    let directory = tempfile::tempdir().unwrap();
    let database = Database::open(directory.path().join("data.sqlite3"));
    let project = sample_project(&database, "远端核对", "资料");
    let writing = sample_writing(&database, &project, "指导");
    let job = start_text_job(&database, &project, &writing);
    database
        .observe_audio_job(&job.id, job.run_id, &AudioObservation::LoginReady)
        .unwrap();
    database.recover_audio_jobs().unwrap();
    let recovered = database.audio_job_data(&project.id).unwrap().job.unwrap();
    assert_eq!(
        (recovered.status.as_str(), recovered.stage.as_str()),
        ("waiting_user", "verify_remote")
    );
    let resumed = database.control_audio_job(&recovered.id, "resume").unwrap();
    assert_eq!(
        (resumed.status.as_str(), resumed.stage.as_str()),
        ("preparing", "verify_remote")
    );
    for observation in [
        AudioObservation::SourceSubmitted,
        AudioObservation::AudioConfigured,
        AudioObservation::GenerationSubmitted,
    ] {
        assert!(database
            .observe_audio_job(&resumed.id, resumed.run_id, &observation)
            .is_err());
    }
    let generating = database
        .observe_audio_job(&resumed.id, resumed.run_id, &AudioObservation::Generating)
        .unwrap();
    assert_eq!(
        (generating.status.as_str(), generating.stage.as_str()),
        ("generating", "generating")
    );
    let downloading = database
        .observe_audio_job(
            &resumed.id,
            resumed.run_id,
            &AudioObservation::GenerationComplete {
                download_url: None,
                suggested_filename: None,
            },
        )
        .unwrap();
    assert_eq!(
        (downloading.status.as_str(), downloading.stage.as_str()),
        ("downloading", "downloading")
    );

    database.control_audio_job(&resumed.id, "cancel").unwrap();
    let second = start_text_job(&database, &project, &writing);
    database
        .observe_audio_job(&second.id, second.run_id, &AudioObservation::LoginReady)
        .unwrap();
    database
        .observe_audio_job(
            &second.id,
            second.run_id,
            &AudioObservation::NotebookCreated {
                notebook_url: "https://notebook.google.com/notebook/recovered".into(),
            },
        )
        .unwrap();
    database.recover_audio_jobs().unwrap();
    let waiting = database.audio_job_data(&project.id).unwrap().job.unwrap();
    assert_eq!(
        (waiting.id.as_str(), waiting.stage.as_str()),
        (second.id.as_str(), "verify_remote")
    );
    let verify_only = database.control_audio_job(&waiting.id, "resume").unwrap();
    let completed = database
        .observe_audio_job(
            &verify_only.id,
            verify_only.run_id,
            &AudioObservation::GenerationComplete {
                download_url: None,
                suggested_filename: None,
            },
        )
        .unwrap();
    assert_eq!(
        (completed.status.as_str(), completed.stage.as_str()),
        ("downloading", "downloading")
    );
}
#[test]
fn asset_transaction_failure_keeps_audio_for_retry_without_duplicate_registration() {
    for restart in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let database = Database::open(directory.path().join("data.sqlite3"));
        let project = sample_project(&database, "事务重试", "资料");
        let writing = sample_writing(&database, &project, "指导");
        let job = start_text_job(&database, &project, &writing);
        advance_to_download(&database, &job);
        let download = database.audio_download_path(&job.id, job.run_id).unwrap();
        fs::create_dir_all(download.parent().unwrap()).unwrap();
        fs::write(&download, wav_bytes()).unwrap();
        database
        .with_connection(|connection| {
            connection.execute_batch("CREATE TRIGGER fail_audio_asset BEFORE INSERT ON assets WHEN NEW.asset_type='audio' BEGIN SELECT RAISE(ABORT, 'forced audio failure'); END;").map_err(|error| error.to_string())
        })
        .unwrap();
        assert!(database
            .complete_audio_download(&job.id, job.run_id, download.to_str().unwrap())
            .is_err());
        let file = Path::new(&project.workspace_path)
            .join("audio")
            .join(format!("事务重试-音频-{}.wav", job.id));
        assert!(
            file.is_file(),
            "the verified project audio must survive the failed transaction"
        );
        let waiting = database.audio_job_data(&project.id).unwrap().job.unwrap();
        assert_eq!(waiting.status, "waiting_user");
        assert_eq!(waiting.stage, "importing");
        database
            .with_connection(|connection| {
                connection
                    .execute("DROP TRIGGER fail_audio_asset", [])
                    .map_err(|error| error.to_string())
            })
            .unwrap();
        if restart {
            database.recover_audio_jobs().unwrap();
        }
        let resumed = database.control_audio_job(&waiting.id, "resume").unwrap();
        let completed = database
            .retry_audio_import(&resumed.id, resumed.run_id)
            .unwrap()
            .unwrap();
        assert_eq!(completed.status, "succeeded");
        assert_eq!(
            database
                .list_assets(&project.id)
                .unwrap()
                .iter()
                .filter(|asset| asset.asset_type == "audio")
                .count(),
            1
        );
    }
}

#[test]
fn initial_and_legacy_extension_jobs_resume_without_remote_side_effects() {
    for initial_stage in ["opening_site", "awaiting_extension"] {
        let directory = tempfile::tempdir().unwrap();
        let database = Database::open(directory.path().join("data.sqlite3"));
        let project = sample_project(&database, "尚未开始", "资料");
        let writing = sample_writing(&database, &project, "指导");
        let original = start_text_job(&database, &project, &writing);
        database
            .with_connection(|connection| {
                connection
                    .execute(
                        "UPDATE audio_jobs SET stage=?2 WHERE id=?1",
                        params![original.id, initial_stage],
                    )
                    .map_err(|e| e.to_string())?;
                Ok(())
            })
            .unwrap();
        database.recover_audio_jobs().unwrap();
        let resumed = database.control_audio_job(&original.id, "resume").unwrap();
        assert_eq!(
            (resumed.status.as_str(), resumed.stage.as_str()),
            ("created", "opening_site")
        );
        assert!(resumed.run_id > original.run_id);
        let ready = database
            .observe_audio_job(&resumed.id, resumed.run_id, &AudioObservation::LoginReady)
            .unwrap();
        assert_eq!(ready.stage, "creating_notebook");
    }
}

#[test]
fn notebook_identity_requires_an_exact_official_notebook_url() {
    assert!(validate_notebook_url(
        "https://notebook.google.com/notebook/task-1"
    ));
    for url in [
        "https://notebook.google.com/",
        "https://notebook.google.com/notebook/task/other",
        "https://notebook.google.com:444/notebook/task",
        "https://notebook.google.com.evil.test/notebook/task",
        "https://user@notebook.google.com/notebook/task",
        "https://notebook.google.com/notebook/",
    ] {
        assert!(!validate_notebook_url(url), "{url}");
    }
}
