use super::*;
use crate::database::{
    ai::WritingVersion,
    projects::{Project, ProjectInput},
    Database,
};
use rusqlite::Connection;
use std::{fs, path::Path};

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
        .observe_audio_job(id, run, &BridgeObservation::LoginReady)
        .unwrap();
    database
        .observe_audio_job(
            id,
            run,
            &BridgeObservation::NotebookCreated {
                notebook_url: "https://notebook.google.com/notebook/task".into(),
            },
        )
        .unwrap();
    database
        .observe_audio_job(id, run, &BridgeObservation::SourceSubmitted)
        .unwrap();
    database
        .observe_audio_job(id, run, &BridgeObservation::SourceReady)
        .unwrap();
    database
        .observe_audio_job(id, run, &BridgeObservation::AudioConfigured)
        .unwrap();
    database
        .observe_audio_job(id, run, &BridgeObservation::GenerationSubmitted)
        .unwrap();
    database
        .observe_audio_job(id, run, &BridgeObservation::Generating)
        .unwrap();
    database
        .observe_audio_job(
            id,
            run,
            &BridgeObservation::GenerationComplete {
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
    let bridge = database.audio_bridge_job().unwrap().unwrap();
    assert_eq!(bridge.prompt, "第一版完整指导");
    assert_eq!(bridge.target_duration_seconds, 1_500);
    assert!(matches!(bridge.source, BridgeSourceData::Text { ref text } if text == "第一版资料"));

    let project_file = project.workspace_path.clone();
    let input: ProjectInput = serde_json::from_value(serde_json::json!({
        "title": "书一", "author": "", "publisher": "", "isbn": "", "description": "",
        "sourceNotes": "后来改写", "theme": "", "targetAudience": "", "contentStyle": "",
        "targetDurationSeconds": 900, "status": "draft"
    }))
    .unwrap();
    database.save_project(Some(&project.id), input).unwrap();
    assert_eq!(
        database.audio_bridge_job().unwrap().unwrap().prompt,
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
fn source_chunk_verifies_size_and_hash_and_rejects_cross_book_assets() {
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

    let job = database
        .start_audio_job(StartAudioJobInput {
            project_id: project.id.clone(),
            writing_version_id: writing.id.clone(),
            source: AudioSourceInput::Asset { asset_id },
            format: "brief".into(),
        })
        .unwrap();
    let first = database.audio_source_chunk(&job.id, job.run_id, 0).unwrap();
    assert_eq!(first.bytes.len(), 256 * 1024);
    assert_eq!(first.name, "原书.docx");
    assert_eq!(first.total_size, 600_000);
    let second = database
        .audio_source_chunk(&job.id, job.run_id, first.next_offset)
        .unwrap();
    assert_eq!(second.bytes.len(), 256 * 1024);
    let last = database
        .audio_source_chunk(&job.id, job.run_id, second.next_offset)
        .unwrap();
    let transferred = [first.bytes, second.bytes, last.bytes].concat();
    assert_eq!(transferred, bytes);
    assert_eq!(last.next_offset, last.total_size);
    assert_eq!(last.sha256, hash_file(&source).unwrap().1);
    fs::write(&source, vec![b'b'; 600_000]).unwrap();
    let error = database
        .audio_source_chunk(&job.id, job.run_id, second.next_offset)
        .expect_err("同大小修改必须在传输完成前拒绝");
    assert!(error.contains("SHA-256"), "{error}");
    assert!(database.audio_source_chunk(&job.id, job.run_id, 0).is_err());
}

#[test]
fn archives_verified_audio_once_and_keeps_project_asset_in_sync() {
    let directory = tempfile::tempdir().unwrap();
    let database = Database::open(directory.path().join("data.sqlite3"));
    let project = sample_project(&database, "归档书籍", "资料");
    let writing = sample_writing(&database, &project, "指导");
    let job = start_text_job(&database, &project, &writing);
    advance_to_download(&database, &job);
    let download = directory
        .path()
        .join("ShuyinAI")
        .join(&job.id)
        .join("audio.download");
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
    let html = directory
        .path()
        .join("ShuyinAI")
        .join(&job.id)
        .join("audio.m4a");
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
        .observe_audio_job(&job.id, job.run_id, &BridgeObservation::LoginReady)
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
        .observe_audio_job(&job.id, job.run_id, &BridgeObservation::LoginReady)
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
        BridgeObservation::SourceSubmitted,
        BridgeObservation::AudioConfigured,
        BridgeObservation::GenerationSubmitted,
    ] {
        assert!(database
            .observe_audio_job(&resumed.id, resumed.run_id, &observation)
            .is_err());
    }
    let generating = database
        .observe_audio_job(&resumed.id, resumed.run_id, &BridgeObservation::Generating)
        .unwrap();
    assert_eq!(
        (generating.status.as_str(), generating.stage.as_str()),
        ("generating", "generating")
    );
    let downloading = database
        .observe_audio_job(
            &resumed.id,
            resumed.run_id,
            &BridgeObservation::GenerationComplete {
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
            &BridgeObservation::GenerationComplete {
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
    let directory = tempfile::tempdir().unwrap();
    let database = Database::open(directory.path().join("data.sqlite3"));
    let project = sample_project(&database, "事务重试", "资料");
    let writing = sample_writing(&database, &project, "指导");
    let job = start_text_job(&database, &project, &writing);
    advance_to_download(&database, &job);
    let download = directory
        .path()
        .join("ShuyinAI")
        .join(&job.id)
        .join("audio.wav");
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
    let resumed = database.control_audio_job(&waiting.id, "resume").unwrap();
    let completed = database
        .complete_audio_download(&resumed.id, resumed.run_id, download.to_str().unwrap())
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
