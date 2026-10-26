use super::{
    assets::open_path,
    covers::CoverConfig,
    projects::{create_named_directory, get, new_id, NOW},
    Database,
};
use crate::media;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

const PORTAL: &str = "https://creator.douyin.com/";
const MAX_IMAGE_BYTES: u64 = 15 * 1024 * 1024;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PublishInput {
    #[serde(default)]
    pub writing_version_id: Option<String>,
    pub project_id: String,
    pub title: String,
    pub description: String,
    pub tags: String,
    pub video_asset_id: Option<String>,
    #[serde(default)]
    pub audio_asset_id: Option<String>,
    pub landscape_cover_id: Option<String>,
    pub portrait_cover_id: Option<String>,
    pub title_confirmed: bool,
    pub covers_confirmed: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckItem {
    pub key: &'static str,
    pub label: &'static str,
    pub passed: bool,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PublishRecordInput {
    pub confirmed: bool,
    pub published_at: String,
    pub post_url: String,
    pub notes: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishRecord {
    pub published_at: String,
    pub post_url: String,
    pub notes: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishTask {
    pub id: String,
    #[serde(flatten)]
    pub input: PublishInput,
    pub status: String,
    pub task_directory: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub checklist: Vec<CheckItem>,
    pub record: Option<PublishRecord>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishQueueItem {
    pub project_title: String,
    #[serde(flatten)]
    pub task: PublishTask,
}

/// A validated, saved task. Callers cannot supply arbitrary upload paths or text.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PublishSnapshot {
    pub task_id: String,
    pub project_title: String,
    pub task_directory: String,
    pub input: PublishInput,
    pub video: String,
    pub landscape: String,
    pub portrait: String,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct FileReference {
    path: String,
    size: u64,
    modified: u64,
}

#[derive(Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct Manifest {
    schema_version: u32,
    task_id: String,
    platform: String,
    input: PublishInput,
    video: FileReference,
    #[serde(default)]
    audio: Option<FileReference>,
    landscape: FileReference,
    portrait: FileReference,
}

fn reference(path: &str) -> Result<FileReference, String> {
    let metadata = fs::metadata(path).map_err(|_| "素材文件缺失或无法读取，请重新定位。")?;
    if !metadata.is_file() || metadata.len() == 0 {
        return Err("素材必须是非空文件。".into());
    }
    let modified = metadata
        .modified()
        .map_err(|e| e.to_string())?
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_millis() as u64;
    Ok(FileReference {
        path: path.into(),
        size: metadata.len(),
        modified,
    })
}

fn video_path(c: &Connection, input: &PublishInput) -> Result<String, String> {
    c.query_row(
        "SELECT file_path FROM assets WHERE id=?1 AND project_id=?2 AND asset_type='video'",
        params![input.video_asset_id, input.project_id],
        |r| r.get(0),
    )
    .optional()
    .map_err(|e| e.to_string())?
    .ok_or_else(|| "请选择本书剪映导出的最终成片。".into())
}

fn audio_path(c: &Connection, input: &PublishInput) -> Result<String, String> {
    c.query_row(
        "SELECT file_path FROM assets WHERE id=?1 AND project_id=?2 AND asset_type='audio'",
        params![input.audio_asset_id, input.project_id],
        |r| r.get(0),
    )
    .optional()
    .map_err(|e| e.to_string())?
    .ok_or_else(|| "请选择本书的音频成品。".into())
}

fn final_video_ready(c: &Connection, input: &PublishInput) -> bool {
    video_path(c, input)
        .and_then(|path| media::inspect(Path::new(&path)))
        .is_ok_and(|metadata| {
            metadata.duration_seconds > 0.0 && metadata.is_portrait_9_16() && metadata.has_audio
        })
}

fn cover_path(
    c: &Connection,
    project_id: &str,
    id: &Option<String>,
    width: u32,
    height: u32,
) -> Result<String, String> {
    let (json, path): (String, Option<String>) = c
        .query_row(
            "SELECT config_json,exported_path FROM cover_versions WHERE id=?1 AND project_id=?2",
            params![id, project_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|_| "请选择本书已导出的封面。")?;
    let config: CoverConfig = serde_json::from_str(&json).map_err(|_| "封面配置无法读取。")?;
    if (config.width, config.height) != (width, height) {
        return Err("封面比例不匹配，请分别选择 4:3 和 3:4 封面。".into());
    }
    path.ok_or_else(|| "封面尚未导出，请先在封面工坊导出两张图片。".into())
}

fn load(c: &Connection, id: &str) -> Result<PublishTask, String> {
    let mut task = c.query_row("SELECT id,project_id,title,description,tags,video_asset_id,audio_asset_id,landscape_cover_id,portrait_cover_id,title_confirmed,covers_confirmed,status,task_directory,created_at,updated_at,writing_version_id FROM publish_tasks WHERE id=?1", [id], |r| Ok(PublishTask {
        id: r.get(0)?, input: PublishInput { project_id:r.get(1)?, title:r.get(2)?, description:r.get(3)?, tags:r.get(4)?, video_asset_id:r.get(5)?, audio_asset_id:r.get(6)?, landscape_cover_id:r.get(7)?, portrait_cover_id:r.get(8)?, title_confirmed:r.get(9)?, covers_confirmed:r.get(10)?, writing_version_id:r.get(15)? }, status:r.get(11)?, task_directory:r.get(12)?, created_at:r.get(13)?, updated_at:r.get(14)?, checklist:Vec::new(), record:None
    })).optional().map_err(|e| e.to_string())?.ok_or("发布任务不存在。")?;
    task.record = c
        .query_row(
            "SELECT published_at,post_url,notes FROM publish_records WHERE publish_task_id=?1",
            [id],
            |r| {
                Ok(PublishRecord {
                    published_at: r.get(0)?,
                    post_url: r.get(1)?,
                    notes: r.get(2)?,
                })
            },
        )
        .optional()
        .map_err(|e| e.to_string())?;
    Ok(task)
}

fn manifest(c: &Connection, task: &PublishTask) -> Result<Manifest, String> {
    let i = &task.input;
    Ok(Manifest {
        schema_version: 2,
        task_id: task.id.clone(),
        platform: "douyin".into(),
        input: i.clone(),
        video: reference(&video_path(c, i)?)?,
        audio: i
            .audio_asset_id
            .as_ref()
            .map(|_| audio_path(c, i).and_then(|path| reference(&path)))
            .transpose()?,
        landscape: reference(&cover_path(
            c,
            &i.project_id,
            &i.landscape_cover_id,
            1440,
            1080,
        )?)?,
        portrait: reference(&cover_path(
            c,
            &i.project_id,
            &i.portrait_cover_id,
            1080,
            1440,
        )?)?,
    })
}

fn read_limited(path: &Path, limit: u64) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    File::open(path)
        .map_err(|e| e.to_string())?
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > limit {
        return Err("文件超过允许大小。".into());
    }
    Ok(bytes)
}

fn cover_bytes(path: &Path, width: u32, height: u32) -> Result<Vec<u8>, String> {
    let bytes = read_limited(path, MAX_IMAGE_BYTES)?;
    let mut reader =
        image::ImageReader::with_format(std::io::Cursor::new(&bytes), image::ImageFormat::Png);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(width);
    limits.max_image_height = Some(height);
    limits.max_alloc = Some(32 * 1024 * 1024);
    reader.limits(limits);
    let decoded = reader
        .decode()
        .map_err(|_| "封面 PNG 损坏或尺寸不正确，请重新导出。")?;
    if (decoded.width(), decoded.height()) != (width, height) {
        return Err("封面 PNG 尺寸不正确。".into());
    }
    Ok(bytes)
}

fn text_files(m: &Manifest) -> Vec<(&'static str, String)> {
    vec![
        ("title.txt",m.input.title.clone()), ("description.txt",m.input.description.clone()), ("tags.txt",m.input.tags.clone()),
        ("video-path.txt",m.video.path.clone()),
        ("README.txt",format!("书引 AI · 抖音发布准备\n\n1. 在 {PORTAL} 登录自己的账号。\n2. 从 video-path.txt 的原路径选择视频上传；任务包未复制视频。\n3. 分别上传 cover-landscape.png（4:3）和 cover-portrait.png（3:4）。\n4. 复制 title.txt、description.txt 与可选 tags.txt，检查预览后手动发布。\n5. 发布成功后回到书引 AI，确认并记录作品链接。\n\n音频仅作制作记录，不上传抖音；请保留音频和视频源文件。此任务包不代表已发布。\n")),
    ]
}

fn package_valid(c: &Connection, task: &PublishTask) -> bool {
    let Some(directory) = task.task_directory.as_deref() else {
        return false;
    };
    let Ok(current) = manifest(c, task) else {
        return false;
    };
    let directory = Path::new(directory);
    let Ok(bytes) = read_limited(&directory.join("manifest.json"), 64 * 1024) else {
        return false;
    };
    let Ok(saved) = serde_json::from_slice::<Manifest>(&bytes) else {
        return false;
    };
    if saved != current {
        return false;
    }
    text_files(&current).iter().all(|(name, text)| {
        read_limited(&directory.join(name), 64 * 1024).is_ok_and(|bytes| bytes == text.as_bytes())
    }) && [
        ("cover-landscape.png", &current.landscape),
        ("cover-portrait.png", &current.portrait),
    ]
    .iter()
    .all(|(name, source)| {
        // Covers are bounded small files; matching size alone cannot detect a damaged copy.
        let Ok(original) = read_limited(Path::new(&source.path), MAX_IMAGE_BYTES) else {
            return false;
        };
        read_limited(&directory.join(name), MAX_IMAGE_BYTES).is_ok_and(|copy| copy == original)
    })
}

// Called only for a newly copied restore directory, never for original task packages.
pub(super) fn rebase_package(c: &Connection, id: &str) -> Result<(), String> {
    let task = load(c, id)?;
    let directory = Path::new(task.task_directory.as_deref().ok_or("任务包位置缺失。")?);
    let mut m: Manifest =
        serde_json::from_slice(&read_limited(&directory.join("manifest.json"), 64 * 1024)?)
            .map_err(|e| e.to_string())?;
    m.input = task.input.clone();
    m.task_id = task.id.clone();
    let video = video_path(c, &m.input)?;
    m.video = reference(&video).unwrap_or(FileReference {
        path: video,
        size: m.video.size,
        modified: m.video.modified,
    });
    m.audio = if m.input.audio_asset_id.is_some() {
        Some(reference(&audio_path(c, &m.input)?)?)
    } else {
        None
    };
    m.landscape = reference(&cover_path(
        c,
        &m.input.project_id,
        &m.input.landscape_cover_id,
        1440,
        1080,
    )?)?;
    m.portrait = reference(&cover_path(
        c,
        &m.input.project_id,
        &m.input.portrait_cover_id,
        1080,
        1440,
    )?)?;
    for (name, text) in text_files(&m) {
        fs::write(directory.join(name), text).map_err(|e| e.to_string())?;
    }
    fs::write(
        directory.join("manifest.json"),
        serde_json::to_vec_pretty(&m).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    checked(c, id).map(|_| ())
}

fn checked(c: &Connection, id: &str) -> Result<PublishTask, String> {
    let mut task = load(c, id)?;
    let i = &task.input;
    task.checklist = [
        (
            "title",
            "已填写并确认发布标题",
            !i.title.trim().is_empty() && i.title_confirmed,
        ),
        (
            "description",
            "已填写发布内容",
            !i.description.trim().is_empty(),
        ),
        (
            "audio",
            "音频成品已导入且文件可用",
            audio_path(c, i).and_then(|p| reference(&p)).is_ok(),
        ),
        (
            "video",
            "最终成片为竖版、含音轨且文件可用",
            final_video_ready(c, i),
        ),
        (
            "landscape",
            "横版 4:3 封面已导出且可用",
            cover_path(c, &i.project_id, &i.landscape_cover_id, 1440, 1080)
                .and_then(|p| reference(&p))
                .is_ok(),
        ),
        (
            "portrait",
            "竖版 3:4 封面已导出且可用",
            cover_path(c, &i.project_id, &i.portrait_cover_id, 1080, 1440)
                .and_then(|p| reference(&p))
                .is_ok(),
        ),
        ("covers", "已确认两张封面", i.covers_confirmed),
        (
            "package",
            "任务包完整且与当前素材一致",
            package_valid(c, &task),
        ),
    ]
    .into_iter()
    .map(|(key, label, passed)| CheckItem { key, label, passed })
    .collect();
    // Recheck external files on every read. A once-ready task must not remain ready after relocation or loss.
    if task.status == "ready" && task.checklist.iter().any(|item| !item.passed) {
        task.updated_at = c.query_row(
            &format!("UPDATE publish_tasks SET status='draft',updated_at={NOW} WHERE id=?1 RETURNING updated_at"),
            [id], |r| r.get(0),
        ).map_err(|e| e.to_string())?;
        task.status = "draft".into();
    }
    Ok(task)
}

impl Database {
    pub fn list_publish_queue(&self) -> Result<Vec<PublishQueueItem>, String> {
        self.with_connection(|c| {
            let mut statement = c.prepare("SELECT t.id,p.title FROM publish_tasks t JOIN book_projects p ON p.id=t.project_id ORDER BY t.created_at,t.id").map_err(|e|e.to_string())?;
            let rows = statement.query_map([], |r| Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?))).map_err(|e|e.to_string())?.collect::<rusqlite::Result<Vec<_>>>().map_err(|e|e.to_string())?;
            rows.into_iter().map(|(id, project_title)| Ok(PublishQueueItem { project_title, task: checked(c,&id)? })).collect()
        })
    }

    pub fn publish_snapshot(&self, id: &str) -> Result<PublishSnapshot, String> {
        self.with_connection(|c| {
            let task = checked(c, id)?;
            if task.status != "ready" {
                return Err("请先生成有效任务包；已发布任务不能再次上传。".into());
            }
            let directory = Path::new(task.task_directory.as_deref().ok_or("任务包位置缺失。")?);
            Ok(PublishSnapshot {
                task_id: task.id.clone(),
                project_title: get(c, &task.input.project_id)?.input.title,
                task_directory: directory.to_string_lossy().into_owned(),
                video: video_path(c, &task.input)?,
                landscape: directory
                    .join("cover-landscape.png")
                    .to_string_lossy()
                    .into_owned(),
                portrait: directory
                    .join("cover-portrait.png")
                    .to_string_lossy()
                    .into_owned(),
                input: task.input,
            })
        })
    }

    pub fn list_publish_tasks(&self, project_id: &str) -> Result<Vec<PublishTask>, String> {
        self.with_connection(|c| {
            get(c,project_id)?;
            let ids = c.prepare("SELECT id FROM publish_tasks WHERE project_id=?1 ORDER BY created_at DESC,id DESC").map_err(|e|e.to_string())?.query_map([project_id],|r|r.get::<_,String>(0)).map_err(|e|e.to_string())?.collect::<rusqlite::Result<Vec<_>>>().map_err(|e|e.to_string())?;
            ids.iter().map(|id|checked(c,id)).collect()
        })
    }

    pub fn save_publish_task(
        &self,
        id: Option<&str>,
        mut input: PublishInput,
    ) -> Result<PublishTask, String> {
        input.title = input.title.trim().into();
        input.description = input.description.trim().into();
        input.tags = input.tags.trim().into();
        if input.title.chars().count() > 30
            || input.description.chars().count() > 1000
            || input.tags.chars().count() > 500
        {
            return Err("标题最多 30 字，发布内容最多 1000 字，标签最多 500 字。".into());
        }
        self.with_connection(|c| {
            let tx = c.transaction().map_err(|e|e.to_string())?;
            get(&tx,&input.project_id)?;
            if let Some(version)=&input.writing_version_id {
                if !tx.query_row("SELECT EXISTS(SELECT 1 FROM writing_versions WHERE id=?1 AND project_id=?2 AND kind='publishing')",params![version,input.project_id],|r|r.get::<_,bool>(0)).map_err(|e|e.to_string())? { return Err("文案版本不属于当前书籍。".into()); }
            }
            if input.video_asset_id.is_some() { video_path(&tx,&input)?; }
            if input.audio_asset_id.is_some() { audio_path(&tx,&input)?; }
            for (id,w,h) in [(&input.landscape_cover_id,1440,1080),(&input.portrait_cover_id,1080,1440)] { if id.is_some() { cover_path(&tx,&input.project_id,id,w,h)?; } }
            let id = if let Some(id)=id {
                let old = load(&tx,id)?;
                if old.input.project_id != input.project_id || old.status=="published" { return Err("已发布任务不能修改，或任务不属于这本书；请新建任务。".into()); }
                id.to_owned()
            } else { new_id(&tx)? };
            tx.execute(&format!("INSERT INTO publish_tasks(id,project_id,title,description,tags,video_asset_id,audio_asset_id,landscape_cover_id,portrait_cover_id,title_confirmed,covers_confirmed,status,created_at,updated_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,'draft',{NOW},{NOW}) ON CONFLICT(id) DO UPDATE SET title=excluded.title,description=excluded.description,tags=excluded.tags,video_asset_id=excluded.video_asset_id,audio_asset_id=excluded.audio_asset_id,landscape_cover_id=excluded.landscape_cover_id,portrait_cover_id=excluded.portrait_cover_id,title_confirmed=excluded.title_confirmed,covers_confirmed=excluded.covers_confirmed,status='draft',task_directory=NULL,updated_at=excluded.updated_at"),params![id,input.project_id,input.title,input.description,input.tags,input.video_asset_id,input.audio_asset_id,input.landscape_cover_id,input.portrait_cover_id,input.title_confirmed,input.covers_confirmed]).map_err(|e|e.to_string())?;
            tx.execute(&format!("UPDATE book_projects SET updated_at={NOW} WHERE id=?1"),[&input.project_id]).map_err(|e|e.to_string())?;
            tx.execute("UPDATE publish_tasks SET writing_version_id=?1 WHERE id=?2",params![input.writing_version_id,id]).map_err(|e|e.to_string())?;
            tx.commit().map_err(|e|e.to_string())?;
            checked(c,&id)
        })
    }

    pub fn prepare_publish_task(&self, id: &str) -> Result<PublishTask, String> {
        self.with_connection(|c| {
            let task = checked(c,id)?;
            if task.status=="published" { return Err("已发布任务不再生成任务包，请新建任务。".into()); }
            let missing:Vec<_> = task.checklist.iter().filter(|i|i.key!="package" && !i.passed).map(|i|i.label).collect();
            if !missing.is_empty() { return Err(format!("发布准备未完成：{}。",missing.join("；"))); }
            let snapshot = manifest(c,&task)?;
            let landscape = cover_bytes(Path::new(&snapshot.landscape.path),1440,1080)?;
            let portrait = cover_bytes(Path::new(&snapshot.portrait.path),1080,1440)?;
            let project = get(c,&task.input.project_id)?;
            let workspace = PathBuf::from(project.workspace_path);
            if !workspace.is_dir() { return Err("项目工作目录不可用，请在设置中重新定位。".into()); }
            let parent = workspace.join("publish");
            fs::create_dir_all(&parent).map_err(|e|format!("无法创建发布目录：{e}"))?;
            let directory = create_named_directory(&fs::canonicalize(parent).map_err(|e|e.to_string())?, &format!("{}-发布任务包", project.input.title))?;
            let result = (|| {
                let write = |name:&str,bytes:&[u8]| -> Result<(),String> {
                    let mut file = OpenOptions::new().write(true).create_new(true).open(directory.join(name)).map_err(|e|e.to_string())?;
                    file.write_all(bytes).and_then(|_|file.sync_all()).map_err(|e|e.to_string())
                };
                for (name,text) in text_files(&snapshot) { write(name,text.as_bytes())?; }
                write("cover-landscape.png",&landscape)?; write("cover-portrait.png",&portrait)?;
                write("manifest.json",&serde_json::to_vec_pretty(&snapshot).map_err(|e|e.to_string())?)?;
                if snapshot != manifest(c,&task)? { return Err("生成期间素材发生变化，请刷新并重试。".into()); }
                let tx = c.transaction().map_err(|e|e.to_string())?;
                tx.execute(&format!("UPDATE publish_tasks SET status='ready',task_directory=?1,updated_at={NOW} WHERE id=?2"),params![directory.to_string_lossy(),id]).map_err(|e|e.to_string())?;
                tx.commit().map_err(|e|e.to_string())
            })();
            if let Err(error) = result {
                // Only this call's freshly created directory is removed; old packages and sources stay intact.
                let cleanup = fs::remove_dir_all(&directory);
                return Err(if cleanup.is_err() { format!("任务包生成失败：{error}。未完成目录保留在 {}",directory.display()) } else { format!("任务包生成失败：{error}；源素材和已保存草稿保留。") });
            }
            checked(c,id)
        })
    }

    pub fn record_publish_success(
        &self,
        id: &str,
        input: PublishRecordInput,
    ) -> Result<PublishTask, String> {
        if !input.confirmed {
            return Err("请明确确认作品已在平台发布成功。".into());
        }
        if input.post_url.chars().count() > 2048 || input.notes.chars().count() > 2000 {
            return Err("作品链接或备注过长。".into());
        }
        if !input.post_url.trim().is_empty() {
            let url = reqwest::Url::parse(input.post_url.trim())
                .map_err(|_| "作品链接必须是完整的 HTTPS 地址。")?;
            if url.scheme() != "https"
                || url.host_str().is_none()
                || !url.username().is_empty()
                || url.password().is_some()
            {
                return Err("作品链接必须是完整的 HTTPS 地址。".into());
            }
        }
        self.with_connection(|c| {
            let task = checked(c,id)?;
            if task.status!="ready" { return Err("请先生成完整任务包；已发布任务不能重复确认。".into()); }
            let published_at:Option<String> = c.query_row("SELECT strftime('%Y-%m-%dT%H:%M:%fZ',?1)",[&input.published_at],|r|r.get(0)).map_err(|e|e.to_string())?;
            let published_at = published_at.ok_or("发布时间无效。")?;
            let tx = c.transaction().map_err(|e|e.to_string())?;
            tx.execute(&format!("INSERT INTO publish_records(publish_task_id,published_at,post_url,notes,created_at) VALUES(?1,?2,?3,?4,{NOW})"),params![id,published_at,input.post_url.trim(),input.notes.trim()]).map_err(|e|e.to_string())?;
            tx.execute(&format!("UPDATE publish_tasks SET status='published',updated_at={NOW} WHERE id=?1"),[id]).map_err(|e|e.to_string())?;
            tx.execute(&format!("UPDATE book_projects SET status=CASE WHEN status='archived' THEN status ELSE 'published' END,updated_at={NOW} WHERE id=?1"),[&task.input.project_id]).map_err(|e|e.to_string())?;
            tx.commit().map_err(|e|e.to_string())?;
            checked(c,id)
        })
    }

    pub fn open_publish_item(&self, id: &str, item: &str) -> Result<(), String> {
        if item == "portal" {
            self.with_connection(|c| load(c, id).map(|_| ()))?;
            return open_portal();
        }
        if item == "video" {
            let asset_id = self.with_connection(|c| {
                let task = load(c, id)?;
                video_path(c, &task.input)?;
                task.input
                    .video_asset_id
                    .ok_or_else(|| "请先关联最终成片。".into())
            })?;
            return self.open_asset(&asset_id, false);
        }
        let path = self.with_connection(|c| {
            let task = checked(c, id)?;
            let directory = PathBuf::from(task.task_directory.ok_or("尚未生成任务包。")?);
            if !directory.is_dir() {
                return Err("任务包目录已丢失或无法读取。".into());
            }
            match item {
                "directory" => Ok(directory),
                "landscape" => Ok(directory.join("cover-landscape.png")),
                "portrait" => Ok(directory.join("cover-portrait.png")),
                _ => Err("不支持的发布文件操作。".into()),
            }
        })?;
        open_path(&path)
    }
}

fn open_portal() -> Result<(), String> {
    #[cfg(target_os = "windows")]
    let mut command = {
        use std::os::windows::process::CommandExt;
        let mut command = std::process::Command::new("explorer.exe");
        command.creation_flags(0x08000000);
        command
    };
    #[cfg(target_os = "macos")]
    let mut command = std::process::Command::new("open");
    #[cfg(target_os = "linux")]
    let mut command = std::process::Command::new("xdg-open");
    command
        .arg(PORTAL)
        .spawn()
        .map_err(|e| format!("无法打开发布入口：{e}"))?;
    Ok(())
}
