use serde::Serialize;
use tauri::{AppHandle, Emitter};

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Completion {
    Duration,
    Writing,
    Revision,
    CoverBackground,
    CoverLandscape,
    CoverPortrait,
    CoverWidescreen,
    #[cfg(windows)]
    Audio,
    Subtitles,
    ContentCheck,
    Storyboard,
    SceneRevision,
    Illustration,
    Preview,
    Video,
    PublishPackage,
    PublishReady,
    PublishRecorded,
}

#[derive(Clone, Serialize)]
struct Completed<'a> {
    id: &'a str,
    kind: Completion,
}

pub fn notify(app: &AppHandle, id: &str, kind: Completion) {
    let _ = app.emit_to("main", "workflow-completed", Completed { id, kind });
}

pub fn cover(app: &AppHandle, version: &crate::database::covers::CoverVersion) {
    let kind = match (version.config.width, version.config.height) {
        (1440, 1080) => Completion::CoverLandscape,
        (1080, 1440) => Completion::CoverPortrait,
        _ => Completion::CoverWidescreen,
    };
    notify(app, &version.id, kind);
}
