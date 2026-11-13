use super::Database;
use rusqlite::{params, OptionalExtension};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InterfacePreferences {
    pub sidebar_prompt: bool,
}

#[derive(Serialize)]
pub struct ProjectStage {
    pub path: &'static str,
    pub status: String,
    pub tone: &'static str,
}

fn stage(path: &'static str, status: impl Into<String>, tone: &'static str) -> ProjectStage {
    ProjectStage {
        path,
        status: status.into(),
        tone,
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResumeTask {
    pub project_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chapter_id: Option<String>,
    pub title: String,
    pub kind: String,
    pub task_id: String,
    pub status: String,
    pub stage: String,
    pub updated_at: String,
    pub running: bool,
}

#[derive(Serialize)]
pub struct WorkflowSnapshot {
    pub resume: Vec<ResumeTask>,
    pub running: Vec<ResumeTask>,
}

impl Database {
    pub fn interface_preferences(&self) -> Result<InterfacePreferences, String> {
        self.with_connection(|c| {
            let saved: Option<String> = c
                .query_row(
                    "SELECT value FROM app_settings WHERE key='sidebar_prompt_visible'",
                    [],
                    |r| r.get(0),
                )
                .optional()
                .map_err(|e| e.to_string())?;
            let sidebar_prompt = saved
                .map(|value| serde_json::from_str::<bool>(&value).map_err(|e| e.to_string()))
                .transpose()?
                .unwrap_or(true);
            Ok(InterfacePreferences { sidebar_prompt })
        })
    }

    pub fn set_sidebar_prompt(&self, visible: bool) -> Result<(), String> {
        self.with_connection(|c| {
            c.execute("INSERT INTO app_settings(key,value) VALUES('sidebar_prompt_visible',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value", [visible.to_string()]).map_err(|e| e.to_string())?;
            Ok(())
        })
    }

    pub fn project_progress(
        &self,
        project_id: &str,
        chapter_id: Option<&str>,
    ) -> Result<Vec<ProjectStage>, String> {
        let owner = chapter_id.map_or_else(
            || project_id.to_owned(),
            |id| format!("{project_id}:chapter:{id}"),
        );
        let assets = self.list_assets(project_id, chapter_id)?;
        let writing = self.read_editor_draft(&owner, "writing")?;
        let versions = self.list_versions(project_id, "publishing", chapter_id)?;
        let covers = self.list_cover_versions(project_id, chapter_id)?;
        let cover_draft = self.read_editor_draft(&owner, "cover")?;
        let tasks = self.list_publish_tasks(project_id, chapter_id)?;
        let (generating, cover_generating, audio_running, video) = self.with_connection(|c| {
            let (generating, cover_generating, audio_running) = c.query_row("SELECT EXISTS(SELECT 1 FROM ai_tasks WHERE project_id=?1 AND chapter_id IS ?2 AND status IN ('pending','running') AND kind='publishing'), EXISTS(SELECT 1 FROM ai_tasks WHERE project_id=?1 AND chapter_id IS ?2 AND status IN ('pending','running') AND kind='cover'), EXISTS(SELECT 1 FROM audio_jobs WHERE project_id=?1 AND chapter_id IS ?2 AND status IN ('created','preparing','generating','downloading','importing'))", params![project_id,chapter_id], |r| Ok((r.get::<_, bool>(0)?, r.get::<_, bool>(1)?, r.get::<_, bool>(2)?))).map_err(|e| e.to_string())?;
            let video = c.query_row("SELECT status,stage FROM video_jobs WHERE project_id=?1 AND chapter_id IS ?2 ORDER BY (status='running') DESC, MAX(updated_at,COALESCE(json_extract(editor_draft_json,'$.updatedAt'),updated_at)) DESC,id DESC LIMIT 1", params![project_id,chapter_id], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))).optional().map_err(|e| e.to_string())?;
            Ok((generating, cover_generating, audio_running, video))
        })?;
        let mut result = vec![if chapter_id.is_none()
            && assets
                .iter()
                .any(|a| a.asset_type == "document" && a.exists_status != "available")
        {
            stage("", "来源待检查", "attention")
        } else {
            stage("", "资料已保存", "ready")
        }];
        result.push(if generating {
            stage("writing", "生成中", "working")
        } else if audio_running {
            stage("writing", "音频制作中", "working")
        } else if writing.is_some() {
            stage("writing", "有草稿", "attention")
        } else if versions.is_empty() {
            stage("writing", "未开始", "neutral")
        } else {
            stage("writing", "已保存版本", "ready")
        });
        let mut exported = 0;
        let mut changed = false;
        for (width, height) in [(1440, 1080), (1080, 1440)] {
            let saved = covers
                .iter()
                .find(|v| v.config.width == width && v.config.height == height);
            if let Some(saved) = saved {
                if saved.exported_path.is_some()
                    && self.read_cover_export(project_id, &saved.id).is_ok()
                {
                    exported += 1;
                }
            }
            if let Some(values) = cover_draft
                .as_ref()
                .and_then(|d| d.content.get("covers"))
                .and_then(|v| v.as_array())
            {
                if let Some(config) = values.iter().find(|v| {
                    v.get("width").and_then(|v| v.as_u64()) == Some(width as u64)
                        && v.get("height").and_then(|v| v.as_u64()) == Some(height as u64)
                }) {
                    changed |= match (
                        serde_json::from_value::<super::covers::CoverConfig>(config.clone()),
                        saved,
                    ) {
                        (Ok(mut current), Some(saved)) => {
                            let mut previous = saved.config.clone();
                            current.subtitle_color = Some(
                                current
                                    .subtitle_color
                                    .clone()
                                    .unwrap_or_else(|| current.text_color.clone()),
                            );
                            previous.subtitle_color = Some(
                                previous
                                    .subtitle_color
                                    .clone()
                                    .unwrap_or_else(|| previous.text_color.clone()),
                            );
                            previous != current
                        }
                        _ => true,
                    };
                }
            }
        }
        result.push(if cover_generating {
            stage("covers", "生成背景中", "working")
        } else if changed {
            stage("covers", "需保存或重新导出", "attention")
        } else if exported > 0 {
            stage(
                "covers",
                format!("已导出 {exported}/2"),
                if exported == 2 { "ready" } else { "attention" },
            )
        } else if covers.is_empty() {
            stage("covers", "未开始", "neutral")
        } else {
            stage("covers", "已保存配置", "attention")
        });
        let video_available = assets
            .iter()
            .filter(|a| a.asset_type == "video" && a.exists_status == "available")
            .any(|a| {
                crate::media::inspect(std::path::Path::new(&a.file_path)).is_ok_and(|m| {
                    m.duration_seconds > 0.0 && m.width > 0 && m.height > 0 && m.has_audio
                })
            });
        result.push(match video.as_ref().map(|v| v.0.as_str()) {
            Some("running") => stage(
                "video",
                if video.as_ref().is_some_and(|v| v.1 == "rendering") {
                    "渲染中"
                } else {
                    "准备中"
                },
                "working",
            ),
            Some("awaiting_confirmation") => stage("video", "待确认分镜", "attention"),
            _ if video_available => stage("video", "成片可用", "ready"),
            Some("failed" | "cancelled") => stage("video", "需要继续制作", "attention"),
            _ => stage("video", "未开始", "neutral"),
        });
        result.push(match tasks.first().map(|task| task.status.as_str()) {
            Some("published") => stage("publishing", "已发布", "ready"),
            Some("ready") => stage("publishing", "已就绪", "ready"),
            Some(_) => stage("publishing", "待补齐或核对", "attention"),
            None => stage("publishing", "未开始", "neutral"),
        });
        Ok(result)
    }

    pub fn workflow_snapshot(&self) -> Result<WorkflowSnapshot, String> {
        self.with_connection(|c| {
            // Recent activity determines what the creator was doing; active work always wins.
            let mut q = c.prepare("SELECT p.id,p.title || CASE WHEN ch.id IS NOT NULL THEN ' · ' || ch.title WHEN t.chapter_id='new' THEN ' · 新章节' ELSE '' END,t.kind,t.id,t.status,t.stage,t.updated,t.running,t.chapter_id FROM book_projects p JOIN (
                SELECT project_id,'video' kind,id,status,stage,MAX(updated_at,COALESCE(json_extract(editor_draft_json,'$.updatedAt'),updated_at)) updated,status='running' running,chapter_id FROM video_jobs
                UNION ALL SELECT project_id,'audio',id,status,stage,updated_at,status IN ('created','preparing','generating','downloading','importing'),chapter_id FROM audio_jobs
                UNION ALL SELECT project_id,'publishing',id,status,'',updated_at,0,chapter_id FROM publish_tasks
                UNION ALL SELECT project_id,CASE WHEN kind='video_scene_revision' OR kind='video_illustration' THEN 'video' WHEN kind='cover' THEN 'covers' ELSE 'writing' END,id,status,kind,created_at,1,chapter_id FROM ai_tasks WHERE status IN ('pending','running')
                UNION ALL SELECT project_id,CASE kind WHEN 'cover' THEN 'covers' WHEN 'project' THEN 'project' WHEN 'publishing' THEN 'publishing' ELSE 'writing' END,CASE WHEN kind='publishing' THEN CASE WHEN instr(owner_id,':chapter:')>0 THEN 'new' ELSE substr(owner_id,instr(owner_id,':')+1) END ELSE '' END,'draft','',updated_at,0,
                    CASE WHEN instr(owner_id,':chapter:')>0 THEN replace(substr(owner_id,instr(owner_id,':chapter:')+9),':new','') ELSE (SELECT chapter_id FROM publish_tasks WHERE id=substr(owner_id,instr(owner_id,':')+1)) END FROM editor_drafts
                UNION ALL SELECT project_id,'writing',id,'saved','',created_at,0,chapter_id FROM writing_versions
                UNION ALL SELECT project_id,'covers',id,'saved','',created_at,0,chapter_id FROM cover_versions
            ) t ON t.project_id=p.id LEFT JOIN book_chapters ch ON ch.id=t.chapter_id AND ch.project_id=p.id WHERE p.status!='archived' ORDER BY t.running DESC,t.updated DESC,t.id DESC").map_err(|e|e.to_string())?;
            let rows = q.query_map([], |r| Ok(ResumeTask { project_id:r.get(0)?,chapter_id:r.get(8)?,title:r.get(1)?,kind:r.get(2)?,task_id:r.get(3)?,status:r.get(4)?,stage:r.get(5)?,updated_at:r.get(6)?,running:r.get(7)? })).map_err(|e|e.to_string())?;
            let mut resume = BTreeMap::new();
            let mut running = Vec::new();
            for row in rows {
                let row = row.map_err(|e|e.to_string())?;
                resume.entry((row.project_id.clone(), row.chapter_id.clone())).or_insert_with(|| row.clone());
                if row.running { running.push(row); }
            }
            let mut resume: Vec<_> = resume.into_values().collect();
            resume.sort_by(|a,b| b.running.cmp(&a.running).then_with(|| b.updated_at.cmp(&a.updated_at)).then_with(|| b.task_id.cmp(&a.task_id)));
            Ok(WorkflowSnapshot { resume,running })
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::video::VideoWork;
    #[test]
    fn project_progress_reports_running_work_before_newer_drafts() {
        let (_dir, db, input) = crate::database::video::tests::fixture();
        let project = input.project_id.clone();
        let job = db.create_video_job(input).unwrap();
        db.claim_video_job(&job.id, VideoWork::Prepare, None)
            .unwrap();
        db.with_connection(|c| {
            c.execute("INSERT INTO video_jobs(id,project_id,status,stage,plan_json,created_at,updated_at) VALUES('newer-draft',?1,'draft','preparing','{}','9999','9999')", [&project]).unwrap();
            c.execute("INSERT INTO audio_jobs(id,project_id,input_json,status,stage,run_id,created_at,updated_at) VALUES('audio-active',?1,'{}','generating','generating',1,'now','now')", [&project]).unwrap();
            c.execute("INSERT INTO ai_tasks(id,project_id,kind,model_name,prompt_version,status,input_json,created_at) VALUES('cover-active',?1,'cover','local-test','cover-background-v2','running','{}','now')", [&project]).unwrap();
            Ok(())
        }).unwrap();
        let progress = db.project_progress(&project, None).unwrap();
        assert_eq!(progress[1].status, "音频制作中");
        assert_eq!(progress[2].status, "生成背景中");
        assert_eq!(progress[3].status, "准备中");
        assert_eq!(progress[3].tone, "working");
    }
    #[test]
    fn interface_preferences_persist_without_a_schema_change() {
        let (dir, db, _) = crate::database::video::tests::fixture();
        let version = db
            .with_connection(|c| {
                c.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
                    .map_err(|e| e.to_string())
            })
            .unwrap();
        assert!(db.interface_preferences().unwrap().sidebar_prompt);
        db.set_sidebar_prompt(false).unwrap();
        drop(db);
        let db = crate::database::Database::open(dir.path().join("data.sqlite3"));
        assert!(!db.interface_preferences().unwrap().sidebar_prompt);
        assert_eq!(
            db.with_connection(|c| c
                .query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
                .map_err(|e| e.to_string()))
                .unwrap(),
            version
        );
        db.set_sidebar_prompt(true).unwrap();
        assert!(db.interface_preferences().unwrap().sidebar_prompt);
    }

    #[test]
    fn project_progress_uses_real_outputs_instead_of_the_manual_book_status() {
        let (_dir, db, _) = crate::database::video::tests::fixture();
        let project = crate::database::m4_tests::project(&db);
        let mut input = project.input.clone();
        input.status = "published".into();
        db.save_project(Some(&project.id), input).unwrap();
        let before = db.project_progress(&project.id, None).unwrap();
        assert_eq!(
            before.iter().map(|v| v.path).collect::<Vec<_>>(),
            ["", "writing", "covers", "video", "publishing"]
        );
        assert_eq!(before[1].status, "未开始");
        assert_eq!(before[4].status, "未开始");
        let input = crate::database::m4_tests::input(&db, &project);
        let task = db.save_publish_task(None, input).unwrap();
        db.prepare_publish_task(&task.id).unwrap();
        let after = db.project_progress(&project.id, None).unwrap();
        assert_eq!(after[2].status, "已导出 2/2");
        assert_eq!(after[3].status, "成片可用");
        assert_eq!(after[4].status, "已就绪");
        assert!(db.project_progress("missing-book", None).is_err());
    }

    #[test]
    fn project_progress_distinguishes_drafts_missing_outputs_and_effectively_equal_legacy_colors() {
        let (_dir, db, _) = crate::database::video::tests::fixture();
        let project = crate::database::m4_tests::project(&db);
        let input = crate::database::m4_tests::input(&db, &project);
        let versions = db.list_cover_versions(&project.id, None).unwrap();
        let mut configs: Vec<_> = versions.iter().map(|v| v.config.clone()).collect();
        let mut wide = configs[0].clone();
        wide.width = 1920;
        wide.height = 1080;
        configs.push(wide);
        for config in &mut configs {
            config.subtitle_color = Some(config.text_color.clone());
        }
        db.save_editor_draft(
            &project.id,
            "cover",
            Some(crate::database::drafts::EditorDraft {
                based_on: None,
                content: serde_json::json!({"schemaVersion":2,"activeSize":0,"covers":configs}),
                updated_at: String::new(),
            }),
        )
        .unwrap();
        assert_eq!(
            db.project_progress(&project.id, None).unwrap()[2].status,
            "已导出 2/2"
        );
        configs[0].title = "新的未保存标题".into();
        db.save_editor_draft(
            &project.id,
            "cover",
            Some(crate::database::drafts::EditorDraft {
                based_on: None,
                content: serde_json::json!({"schemaVersion":2,"activeSize":0,"covers":configs}),
                updated_at: String::new(),
            }),
        )
        .unwrap();
        db.save_editor_draft(
            &project.id,
            "writing",
            Some(crate::database::drafts::EditorDraft {
                based_on: None,
                content: serde_json::json!({"title":"未完成稿"}),
                updated_at: String::new(),
            }),
        )
        .unwrap();
        let progress = db.project_progress(&project.id, None).unwrap();
        assert_eq!(progress[1].status, "有草稿");
        assert_eq!(progress[2].status, "需保存或重新导出");
        db.save_editor_draft(&project.id, "cover", None).unwrap();
        std::fs::remove_file(versions[0].exported_path.as_ref().unwrap()).unwrap();
        assert_eq!(
            db.project_progress(&project.id, None).unwrap()[2].status,
            "已导出 1/2"
        );
        let video = db
            .list_assets(&project.id, None)
            .unwrap()
            .into_iter()
            .find(|a| Some(&a.id) == input.video_asset_id.as_ref())
            .unwrap();
        std::fs::write(&video.file_path, b"not a video").unwrap();
        assert_ne!(
            db.project_progress(&project.id, None).unwrap()[3].tone,
            "ready"
        );
    }

    #[test]
    fn exporting_video_preserves_originals_and_refuses_overwrites_or_wrong_formats() {
        let (dir, db, _) = crate::database::video::tests::fixture();
        let project = crate::database::m4_tests::project(&db);
        let input = crate::database::m4_tests::input(&db, &project);
        let id = input.video_asset_id.unwrap();
        let asset = db
            .list_assets(&project.id, None)
            .unwrap()
            .into_iter()
            .find(|a| a.id == id)
            .unwrap();
        let original = std::fs::read(&asset.file_path).unwrap();
        let metadata = db.video_asset_metadata(&project.id, &id).unwrap();
        assert_eq!((metadata.width, metadata.height), (1080, 1920));
        let output = dir.path().join("另存成片.MP4");
        db.export_video_asset(&project.id, &id, output.to_str().unwrap())
            .unwrap();
        assert_eq!(std::fs::read(&output).unwrap(), original);
        assert!(db
            .export_video_asset(&project.id, &id, output.to_str().unwrap())
            .unwrap_err()
            .contains("已存在"));
        assert_eq!(std::fs::read(&asset.file_path).unwrap(), original);
        assert_eq!(std::fs::read(&output).unwrap(), original);
        let wrong = dir.path().join("不转换格式.mov");
        assert!(db
            .export_video_asset(&project.id, &id, wrong.to_str().unwrap())
            .is_err());
        assert!(!wrong.exists());
        assert!(db
            .video_asset_metadata(&project.id, "missing-asset")
            .is_err());
        assert!(db
            .export_video_asset(&project.id, &id, "relative.mp4")
            .is_err());
    }

    #[test]
    fn workflow_uses_real_video_task_and_prefers_running_over_newer_drafts() {
        let (_dir, db, input) = crate::database::video::tests::fixture();
        let project = input.project_id.clone();
        let job = db.create_video_job(input).unwrap();
        db.with_connection(|c|{c.execute("INSERT INTO editor_drafts(owner_id,project_id,kind,content_json,updated_at) VALUES(?1,?1,'writing','{}','9999')",[&project]).unwrap();Ok(())}).unwrap();
        assert_eq!(db.workflow_snapshot().unwrap().resume[0].kind, "writing");
        db.claim_video_job(&job.id, VideoWork::Prepare, None)
            .unwrap();
        let snapshot = db.workflow_snapshot().unwrap();
        assert_eq!(snapshot.running.len(), 1);
        assert_eq!(snapshot.resume[0].task_id, job.id);
        assert_eq!(snapshot.resume[0].stage, "preparing");
        db.end_video_job(&job.id, 1, "awaiting_confirmation", "")
            .unwrap();
        assert!(db.workflow_snapshot().unwrap().running.is_empty());
    }
}
