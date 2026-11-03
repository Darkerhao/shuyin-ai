use super::VideoSettings;
use crate::{database::Database, video_process as process};
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    sync::{atomic::AtomicBool, Arc},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentItem {
    id: String,
    status: String,
    detail: String,
    path: String,
    version: String,
}

impl EnvironmentItem {
    fn notice(id: &str, status: &str, detail: &str) -> Self {
        Self {
            id: id.into(),
            status: status.into(),
            detail: detail.into(),
            path: String::new(),
            version: String::new(),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentReport {
    settings: VideoSettings,
    items: Vec<EnvironmentItem>,
}

fn first_file(names: &[&str]) -> Option<PathBuf> {
    std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
        .flat_map(|dir| names.iter().map(move |name| dir.join(name)))
        .find(|path| {
            path.is_absolute()
                && path.is_file()
                && !path
                    .components()
                    .any(|part| part.as_os_str() == "WindowsApps")
        })
}

fn discover(settings: &mut VideoSettings, workspace: &Path) {
    let cwd = std::env::current_dir().unwrap_or_default();
    if settings.node_path.is_empty() {
        if let Some(path) = first_file(&["node.exe"]) {
            settings.node_path = process::external_path(&path);
        }
    }
    if settings.engine_root.is_empty() {
        for base in [workspace, cwd.as_path()]
            .into_iter()
            .flat_map(|p| p.ancestors().take(3))
        {
            if let Some(path) = [
                base.join("whiteboard-teacher-video/engine"),
                base.join("engine"),
                base.join("node_modules/@frametell/engine"),
            ]
            .into_iter()
            .find(|p| p.join("package.json").is_file())
            {
                settings.engine_root = process::external_path(&path);
                break;
            }
        }
    }
    if settings.python_path.is_empty() {
        let candidate = [
            workspace,
            cwd.as_path(),
            Path::new(&settings.engine_root)
                .parent()
                .unwrap_or(workspace),
        ]
        .into_iter()
        .flat_map(|base| {
            [
                base.join(".tools/video-runtime/Scripts/python.exe"),
                base.join("runtime/Scripts/python.exe"),
            ]
        })
        .find(|path| path.is_file())
        .or_else(|| first_file(&["python.exe", "python3.exe"]));
        if let Some(path) = candidate {
            settings.python_path = process::external_path(&path);
        }
    }
    if settings.model_path.is_empty() {
        if let Some(root) = Path::new(&settings.python_path).ancestors().nth(3) {
            if let Some(path) = [root.join("model-small"), root.join("video-model-small")]
                .into_iter()
                .find(|path| path.join("model.bin").is_file())
            {
                settings.model_path = process::external_path(&path);
            }
        }
    }
}

fn check_script(
    directory: &Path,
    name: &str,
    executable: &str,
    source: &[u8],
    input: serde_json::Value,
) -> Result<Vec<EnvironmentItem>, String> {
    let script = directory.join(name);
    let request = directory.join(format!("{name}-input.json"));
    let output = directory.join(format!("{name}-output.json"));
    process::write_new(&script, source)?;
    process::write_new(
        &request,
        &serde_json::to_vec(&input).map_err(|e| e.to_string())?,
    )?;
    process::run(
        executable,
        &script,
        &request,
        &output,
        Arc::new(AtomicBool::new(false)),
        Duration::from_secs(45),
    )?;
    serde_json::from_str(&process::read_small(&output)?).map_err(|e| e.to_string())
}

pub fn inspect(
    database: &Database,
    mut settings: VideoSettings,
    needs_transcription: bool,
) -> Result<EnvironmentReport, String> {
    if !cfg!(target_os = "windows") {
        return Err("视频环境检查目前支持 Windows。".into());
    }
    let workspace = PathBuf::from(database.workspace_settings()?.root);
    discover(&mut settings, &workspace);
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let directory = workspace.join(format!(".video-check-{stamp}"));
    std::fs::create_dir_all(&workspace).map_err(|e| e.to_string())?;
    std::fs::create_dir(&directory).map_err(|e| e.to_string())?;
    let mut items = match check_script(
        &directory,
        "runtime.mjs",
        &settings.node_path,
        include_bytes!("../../video/runtime.mjs"),
        serde_json::json!({"engineRoot":settings.engine_root}),
    ) {
        Ok(items) => items,
        Err(error) => {
            let mut items = vec![EnvironmentItem::notice("node", "error", &error)];
            for id in ["engine", "ffmpeg", "browser", "fonts"] {
                items.push(EnvironmentItem::notice(
                    id,
                    "error",
                    "Node 检查未完成，请先修复 Node 路径后重新检查。",
                ));
            }
            items
        }
    };
    if needs_transcription {
        match check_script(
            &directory,
            "transcribe.py",
            &settings.python_path,
            include_bytes!("../../video/transcribe.py"),
            serde_json::json!({"action":"environment","modelPath":settings.model_path}),
        ) {
            Ok(python) => items.extend(python),
            Err(error) => {
                items.push(EnvironmentItem::notice("python", "error", &error));
                items.push(EnvironmentItem::notice(
                    "model",
                    "error",
                    "请先修复 Python 路径后检查模型。",
                ));
            }
        }
    } else {
        for id in ["python", "model"] {
            items.push(EnvironmentItem::notice(
                id,
                "skipped",
                "使用已有字幕，不需要本地语音识别环境。",
            ));
        }
    }
    // Only this invocation's newly created scratch directory is removed, never a configured path.
    std::fs::remove_dir_all(&directory)
        .map_err(|e| format!("检查已结束，临时检查目录无法清理：{e}"))?;
    Ok(EnvironmentReport { settings, items })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn discovery_preserves_explicit_paths_and_finds_an_engine_beside_workspace() {
        let dir = tempfile::tempdir().unwrap();
        let engine = dir.path().join("whiteboard-teacher-video/engine");
        std::fs::create_dir_all(&engine).unwrap();
        std::fs::write(engine.join("package.json"), "{}").unwrap();
        let mut settings = VideoSettings {
            engine_root: "explicit-engine".into(),
            node_path: "explicit-node".into(),
            python_path: "explicit-python".into(),
            model_path: "explicit-model".into(),
        };
        discover(&mut settings, dir.path());
        assert_eq!(settings.engine_root, "explicit-engine");
        assert_eq!(settings.node_path, "explicit-node");
        assert_eq!(settings.python_path, "explicit-python");
        assert_eq!(settings.model_path, "explicit-model");
        settings.engine_root.clear();
        discover(&mut settings, dir.path());
        assert_eq!(settings.engine_root, process::external_path(&engine));
    }
}
