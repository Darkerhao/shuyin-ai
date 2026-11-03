use super::*;
use crate::database::drafts::EditorDraft;

impl Database {
    pub fn read_video_draft(&self, id: &str) -> Result<Option<EditorDraft>, String> {
        self.with_connection(|c| {
            let raw: Option<String> = c
                .query_row(
                    "SELECT editor_draft_json FROM video_jobs WHERE id=?1",
                    [id],
                    |r| r.get(0),
                )
                .map_err(|_| "视频任务不存在。")?;
            raw.map(|raw| serde_json::from_str(&raw).map_err(|e| e.to_string()))
                .transpose()
        })
    }

    pub fn save_video_draft(&self, id: &str, draft: Option<EditorDraft>) -> Result<(), String> {
        self.with_connection(|c| {
            let job = get(c, id)?;
            if matches!(job.status.as_str(), "running" | "succeeded") {
                return Err("执行中或已完成的视频不能暂存编辑。".into());
            }
            let raw = draft
                .map(|mut draft| -> Result<String, String> {
                    if draft.based_on.as_deref() != Some(job.run_id.to_string().as_str()) {
                        return Err("视频任务已更新，请重新读取后编辑。".into());
                    }
                    if !draft.content.is_object() {
                        return Err("视频草稿格式无效。".into());
                    }
                    draft.updated_at = c
                        .query_row(&format!("SELECT {NOW}"), [], |r| r.get(0))
                        .map_err(|e| e.to_string())?;
                    let raw = serde_json::to_string(&draft).map_err(|e| e.to_string())?;
                    if raw.len() > 4 * 1024 * 1024 {
                        return Err("视频草稿超过4MB。".into());
                    }
                    Ok(raw)
                })
                .transpose()?;
            c.execute(
                "UPDATE video_jobs SET editor_draft_json=?1 WHERE id=?2",
                params![raw, id],
            )
            .map_err(|e| e.to_string())?;
            Ok(())
        })
    }

    // Freeze the first transcript before proofreading. Never capture already corrected legacy cues.
    pub fn save_original_video_cues(
        &self,
        id: &str,
        run_id: i64,
        cues: &[Cue],
    ) -> Result<(), String> {
        self.with_connection(|c| {
            let job = get(c, id)?;
            if job.run_id != run_id || job.status != "running" { return Err("视频任务已过期。".into()); }
            validate_cues(cues, job.plan.duration)?;
            c.execute("UPDATE video_jobs SET original_cues_json=COALESCE(original_cues_json,?1) WHERE id=?2", params![serde_json::to_string(cues).map_err(|e| e.to_string())?, id]).map_err(|e| e.to_string())?;
            Ok(())
        })
    }

    pub fn original_video_cues(&self, id: &str) -> Result<Vec<Cue>, String> {
        self.with_connection(|c| {
            let job = get(c, id)?;
            let raw: Option<String> = c
                .query_row(
                    "SELECT original_cues_json FROM video_jobs WHERE id=?1",
                    [id],
                    |r| r.get(0),
                )
                .map_err(|e| e.to_string())?;
            if let Some(raw) = raw {
                return serde_json::from_str(&raw).map_err(|e| e.to_string());
            }
            // Older jobs registered transcript.srt without an explicit original-source relation.
            // Only inspect registered assets in this exact job's run directories, earliest run first.
            let root = Path::new(&projects::get(c, &job.project_id)?.workspace_path)
                .join("video")
                .join(id);
            let root = root.canonicalize().unwrap_or(root);
            let mut q = c
                .prepare(
                    "SELECT file_path FROM assets WHERE project_id=?1 AND asset_type='subtitle'",
                )
                .map_err(|e| e.to_string())?;
            let paths = q
                .query_map([&job.project_id], |r| r.get::<_, String>(0))
                .map_err(|e| e.to_string())?
                .collect::<rusqlite::Result<Vec<_>>>()
                .map_err(|e| e.to_string())?;
            let mut originals: Vec<_> = paths
                .into_iter()
                .filter_map(|path| {
                    let path = PathBuf::from(path);
                    let parent = path.parent()?;
                    if path.file_name()? != "transcript.srt" || parent.parent()? != root {
                        return None;
                    }
                    let run = parent
                        .file_name()?
                        .to_str()?
                        .strip_prefix("run-")?
                        .split('-')
                        .next()?
                        .parse::<i64>()
                        .ok()?;
                    Some((run, path))
                })
                .collect();
            originals.sort();
            let path = if let Some((_, path)) = originals.first() {
                path.clone()
            } else if let Some(source) = job.source_subtitle_id {
                asset_path(c, &job.project_id, &source, "subtitle")?
            } else {
                return Err("此任务没有可读取的原始SRT，仍可编辑和试听当前字幕。".into());
            };
            let text = crate::video_process::read_small(&path)
                .map_err(|_| "原始SRT文件无法读取，请在素材管理中重新定位。")?;
            crate::video::parse_srt(&text, job.plan.duration)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn draft(run_id: i64, text: &str) -> EditorDraft {
        EditorDraft {
            based_on: Some(run_id.to_string()),
            content: json!({"subtitles":text,"scenes":"[]","position":{"section":"srt","index":0,"onlyChanges":true,"cursor":4}}),
            updated_at: String::new(),
        }
    }

    #[test]
    fn video_drafts_survive_reopen_and_backup_without_changing_plans_or_other_jobs() {
        let (dir, db, input) = super::super::tests::fixture();
        let second = db
            .create_video_job(StartVideoInput {
                project_id: input.project_id.clone(),
                audio_asset_id: input.audio_asset_id.clone(),
                provider_id: input.provider_id.clone(),
                subtitle_asset_id: None,
            })
            .unwrap();
        let first = db.create_video_job(input).unwrap();
        let saved = serde_json::to_value(&first.plan).unwrap();
        db.save_video_draft(&first.id, Some(draft(first.run_id, "未完成的时间码")))
            .unwrap();
        db.save_video_draft(&second.id, Some(draft(second.run_id, "另一个任务")))
            .unwrap();
        let backup = db.create_backup(dir.path().to_str().unwrap()).unwrap();
        drop(db);
        let db = Database::open(dir.path().join("data.sqlite3"));
        assert_eq!(
            db.read_video_draft(&first.id).unwrap().unwrap().content["subtitles"],
            "未完成的时间码"
        );
        assert_eq!(
            serde_json::to_value(db.video_job(&first.id).unwrap().plan).unwrap(),
            saved
        );
        assert!(db.video_job(&first.id).unwrap().draft_updated_at.is_some());
        db.save_video_draft(&first.id, None).unwrap();
        assert_eq!(
            db.read_video_draft(&second.id).unwrap().unwrap().content["subtitles"],
            "另一个任务"
        );
        db.restore_backup(&backup).unwrap();
        assert_eq!(
            db.read_video_draft(&first.id).unwrap().unwrap().content["position"]["cursor"],
            4
        );
    }

    #[test]
    fn failed_formal_save_keeps_draft_success_clears_it_and_running_rejects_late_writes() {
        let (_dir, db, input) = super::super::tests::fixture();
        let job = db.create_video_job(input).unwrap();
        let active = db
            .claim_video_job(&job.id, VideoWork::Prepare, None)
            .unwrap();
        let cues = vec![Cue {
            start: 0.0,
            end: 2.0,
            text: "原文".into(),
        }];
        let mut plan = active.plan;
        plan.duration = 3.0;
        plan.cues = cues.clone();
        db.save_video_progress(&job.id, active.run_id, &plan, "storyboard")
            .unwrap();
        db.end_video_job(&job.id, active.run_id, "awaiting_confirmation", "")
            .unwrap();
        db.save_video_draft(&job.id, Some(draft(active.run_id, "未完成")))
            .unwrap();
        assert!(db.edit_video_plan(&job.id, vec![], vec![]).is_err());
        assert!(db.read_video_draft(&job.id).unwrap().is_some());
        db.edit_video_plan(&job.id, cues, vec![]).unwrap();
        assert!(db.read_video_draft(&job.id).unwrap().is_none());
        db.save_video_draft(&job.id, Some(draft(active.run_id, "另一次编辑")))
            .unwrap();
        let next = db
            .claim_video_job(&job.id, VideoWork::Prepare, None)
            .unwrap();
        assert!(db
            .save_video_draft(&job.id, Some(draft(active.run_id, "迟到写入")))
            .is_err());
        assert!(db.read_video_draft(&job.id).unwrap().is_none());
        db.end_video_job(&job.id, next.run_id, "failed", "stopped")
            .unwrap();
        assert!(db
            .save_video_draft(&job.id, Some(draft(active.run_id, "旧代次")))
            .is_err());
    }

    #[test]
    fn original_cues_are_frozen_before_correction_and_survive_reopening() {
        let (dir, db, input) = super::super::tests::fixture();
        let job = db.create_video_job(input).unwrap();
        let mut active = db
            .claim_video_job(&job.id, VideoWork::Prepare, None)
            .unwrap();
        active.plan.duration = 3.0;
        db.save_video_progress(&job.id, active.run_id, &active.plan, "transcribing")
            .unwrap();
        let original = vec![Cue {
            start: 0.0,
            end: 2.0,
            text: "孔字".into(),
        }];
        db.save_original_video_cues(&job.id, active.run_id, &original)
            .unwrap();
        active.plan.cues = vec![Cue {
            text: "孔子".into(),
            ..original[0].clone()
        }];
        db.save_original_video_cues(&job.id, active.run_id, &active.plan.cues)
            .unwrap();
        db.save_video_progress(&job.id, active.run_id, &active.plan, "proofreading")
            .unwrap();
        drop(db);
        let db = Database::open(dir.path().join("data.sqlite3"));
        assert_eq!(db.original_video_cues(&job.id).unwrap(), original);
        assert_eq!(db.video_job(&job.id).unwrap().plan.cues[0].text, "孔子");
    }

    #[test]
    fn legacy_original_uses_earliest_registered_transcript_of_exact_job_and_reports_missing_files()
    {
        let (_dir, db, input) = super::super::tests::fixture();
        let project = db.get_project(&input.project_id).unwrap();
        let job = db.create_video_job(input).unwrap();
        let mut active = db
            .claim_video_job(&job.id, VideoWork::Prepare, None)
            .unwrap();
        active.plan.duration = 3.0;
        db.save_video_progress(&job.id, active.run_id, &active.plan, "transcribing")
            .unwrap();
        let root = Path::new(&project.workspace_path).join("video");
        let mut original_path = PathBuf::new();
        for (owner, run, text) in [
            ("other-job", 0, "别人的原文"),
            (job.id.as_str(), 10, "已校对"),
            (job.id.as_str(), 2, "真正原文"),
        ] {
            let path = root
                .join(owner)
                .join(format!("run-{run}-123"))
                .join("transcript.srt");
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(
                &path,
                crate::video::to_srt(&[Cue {
                    start: 0.0,
                    end: 2.0,
                    text: text.into(),
                }]),
            )
            .unwrap();
            db.link_asset(&project.id, path.to_str().unwrap(), None)
                .unwrap();
            if run == 2 {
                original_path = path;
            }
        }
        assert_eq!(db.original_video_cues(&job.id).unwrap()[0].text, "真正原文");
        std::fs::remove_file(original_path).unwrap();
        assert!(db
            .original_video_cues(&job.id)
            .unwrap_err()
            .contains("无法读取"));
    }

    #[test]
    fn v15_migration_preserves_existing_jobs_and_runs_once() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("v15.sqlite3");
        let c = Connection::open(&path).unwrap();
        for sql in crate::database::MIGRATIONS.iter().take(15) {
            c.execute_batch(sql).unwrap();
        }
        c.execute_batch("INSERT INTO book_projects(id,title,target_duration_seconds,status,workspace_path,created_at,updated_at) VALUES('book','旧书',600,'draft','unused','original','original'); INSERT INTO video_jobs(id,project_id,status,stage,plan_json,created_at,updated_at) VALUES('job','book','draft','preparing','{}','original','original'); PRAGMA user_version=15;").unwrap();
        drop(c);
        for _ in 0..2 {
            let db = Database::open(path.clone());
            db.with_connection(|c| {
                let row: (String, Option<String>, Option<String>) = c.query_row("SELECT updated_at,editor_draft_json,original_cues_json FROM video_jobs WHERE id='job'", [], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?))).unwrap();
                assert_eq!(row, ("original".into(), None, None));
                assert_eq!(c.pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0)).unwrap(), 16);
                Ok(())
            }).unwrap();
        }
    }
}
