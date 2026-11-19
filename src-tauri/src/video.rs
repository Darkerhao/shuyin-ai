use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::OnceLock};

pub mod drawing;
pub mod environment;
pub mod ink;
pub mod pencil;
pub mod revision;
pub mod subtitles;

pub const PROMPT: &str = include_str!("../../prompts/video-storyboard-v6.md");
pub const VIDEO_THEMES: [&str; 2] = ["classic", "ink"];
pub const BATCH_SIZE: usize = 12;
pub const MODEL_FAILURE_PREFIX: &str = "模型生成失败：";

pub fn model_failure(message: impl std::fmt::Display) -> String {
    format!("{MODEL_FAILURE_PREFIX}{message}")
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct VisualLimits {
    max_scene_cues: usize,
    label_chars: usize,
    title_chars: usize,
    object_chars: usize,
    drawing_width: f64,
    drawing_height: f64,
    max_drawing_points: usize,
    max_drawing_strokes: usize,
}

#[derive(Deserialize)]
struct VisualCatalog {
    limits: VisualLimits,
    kinds: BTreeMap<String, String>,
    environments: BTreeMap<String, String>,
    framings: BTreeMap<String, String>,
    actions: BTreeMap<String, String>,
    objects: BTreeMap<String, String>,
}

fn visual_catalog() -> &'static VisualCatalog {
    static CATALOG: OnceLock<VisualCatalog> = OnceLock::new();
    CATALOG.get_or_init(|| {
        serde_json::from_str(include_str!("../video/visual-catalog.json"))
            .expect("bundled visual catalog")
    })
}

pub fn storyboard_prompt(theme: &str) -> String {
    format!(
        "{PROMPT}\n{}\n可用构图、动作与绘制边界：\n{}",
        if theme == "ink" {
            include_str!("../../prompts/video-pencil-style-v2.md")
        } else {
            "本次画风：书卷手绘。保留物体、动作、关系对照和短标签的讲解构图。"
        },
        {
            let mut catalog: serde_json::Value =
                serde_json::from_str(include_str!("../video/visual-catalog.json"))
                    .expect("bundled catalog");
            catalog.as_object_mut().unwrap().remove("objects");
            catalog.to_string()
        }
    )
}

pub fn parse_storyboard(raw: &str, first: usize, end: usize) -> Result<Vec<Scene>, String> {
    let scenes: Vec<Scene> =
        serde_json::from_str(raw).map_err(|_| "模型分镜不是合法 JSON，已保留字幕和此前分镜。")?;
    validate_scenes(&scenes, first, end).map_err(|error| format!("本批分镜校验未通过：{error}"))?;
    for (index, scene) in scenes.iter().enumerate() {
        let error =
            |detail: String| format!("本批分镜校验未通过：{}", scene_error(index, scene, detail));
        if scene.image.is_some() || scene.painting.is_some() || scene.artwork.is_some() {
            return Err(error(
                "image/painting/artwork 非空；此入口须返回物体笔画，精绘场景使用独立的逐镜协议。"
                    .into(),
            ));
        }
        if scene.beats[0].cue != scene.first {
            return Err(error(format!(
                "第 1 个物体的 cue={}，应为 {}；首个物体须从本镜第一条字幕开始。",
                scene.beats[0].cue, scene.first
            )));
        }
    }
    Ok(scenes)
}

pub fn storyboard_input(plan: &VideoPlan, first: usize, end: usize) -> String {
    let cues = |from: usize, to: usize| {
        plan.cues[from..to].iter().enumerate().map(|(offset, c)| serde_json::json!({"index": from + offset, "start": c.start, "end": c.end, "text": c.text})).collect::<Vec<_>>()
    };
    serde_json::json!({
        "book": plan.title, "theme": plan.theme,
        "duration": plan.duration,
        "opening": cues(0, plan.cues.len().min(6)),
        "ending": cues(plan.cues.len().saturating_sub(6), plan.cues.len()),
        "previousScenes": &plan.scenes[plan.scenes.len().saturating_sub(3)..],
        "followingContext": cues(end, (end + 3).min(plan.cues.len())),
        "cues": cues(first, end),
    })
    .to_string()
}

pub fn require_drawings(scenes: &[Scene]) -> Result<(), String> {
    for (index, scene) in scenes.iter().enumerate() {
        if let Some(beat) = scene.beats.iter().position(|beat| beat.drawing.is_none()) {
            return Err(scene_error(
                index,
                scene,
                format!(
                    "第 {} 个物体缺少 drawing.strokes；新分镜须给出实际笔画，不能只提供物体名称。",
                    beat + 1
                ),
            ));
        }
    }
    Ok(())
}

pub fn storyboard_batch_end(cues: &[Cue], first: usize) -> usize {
    let end = (first + BATCH_SIZE).min(cues.len());
    if end == cues.len() {
        return end;
    }
    ((first + BATCH_SIZE - 6)..end)
        .rev()
        .find(|&i| {
            matches!(
                cues[i].text.trim().chars().last(),
                Some('。' | '！' | '？' | '!' | '?' | '；' | ';')
            )
        })
        .map_or(end, |i| i + 1)
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VideoSettings {
    pub engine_root: String,
    pub node_path: String,
    pub python_path: String,
    pub model_path: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cue {
    pub start: f64,
    pub end: f64,
    pub text: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct VideoProgress {
    pub started_at: u64,
    pub updated_at: u64,
    pub stage: String,
    pub phase: String,
    pub completed: f64,
    pub total: f64,
    pub unit: Option<String>,
    pub speed: Option<f64>,
    pub cue_count: usize,
    pub preview: Vec<Cue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scene {
    pub first: usize,
    pub last: usize,
    pub kind: String,
    pub title: String,
    #[serde(default = "plain_environment")]
    pub environment: String,
    #[serde(default = "reference_framing")]
    pub framing: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub labels: Vec<String>,
    #[serde(default)]
    pub beats: Vec<Beat>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<SceneImage>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub painting: Option<ink::Painting>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artwork: Option<pencil::Artwork>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SceneImage {
    pub asset_id: String,
    pub source: String,
    pub rights: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub creator: String,
    pub fit: String,
    pub x: f64,
    pub y: f64,
    #[serde(default)]
    pub sha256: String,
}
impl SceneImage {
    pub fn validate(&self) -> Result<(), String> {
        let text = |value: &str, limit| {
            !value.trim().is_empty()
                && value.chars().count() <= limit
                && !value.chars().any(|ch| ch.is_control() && ch != '\n')
        };
        if !text(&self.asset_id, 128)
            || !text(&self.source, 1000)
            || !text(&self.rights, 1000)
            || (!self.creator.is_empty() && !text(&self.creator, 200))
            || !matches!(self.fit.as_str(), "contain" | "cover")
            || !self.x.is_finite()
            || !self.y.is_finite()
            || !(0.0..=1.0).contains(&self.x)
            || !(0.0..=1.0).contains(&self.y)
        {
            return Err(
                "图片镜头须选择本书素材、填写来源和授权说明，并使用有效的显示方式和裁切位置。"
                    .into(),
            );
        }
        Ok(())
    }
}

fn plain_environment() -> String {
    "plain".into()
}

fn reference_framing() -> String {
    "reference".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Beat {
    pub cue: usize,
    pub object: String,
    pub action: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drawing: Option<drawing::Drawing>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VideoPlan {
    pub title: String,
    pub audio_sha256: String,
    pub duration: f64,
    pub cues: Vec<Cue>,
    pub scenes: Vec<Scene>,
    #[serde(default = "classic_theme", skip_serializing_if = "is_classic_theme")]
    pub theme: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_provider_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subtitle_review: Option<subtitles::SubtitleReview>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pencil: Option<pencil::PencilPlan>,
}

impl Default for VideoPlan {
    fn default() -> Self {
        Self {
            title: String::new(),
            audio_sha256: String::new(),
            duration: 0.0,
            cues: Vec::new(),
            scenes: Vec::new(),
            theme: classic_theme(),
            image_provider_id: None,
            subtitle_review: None,
            pencil: None,
        }
    }
}

fn classic_theme() -> String {
    "classic".into()
}

fn is_classic_theme(theme: &str) -> bool {
    theme == "classic"
}

pub fn validate_theme(theme: &str) -> Result<(), String> {
    if VIDEO_THEMES.contains(&theme) {
        Ok(())
    } else {
        Err("视频主题无效，请选择书卷手绘或铅笔素描。".into())
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreviewRequest {
    pub first_scene: usize,
    pub seconds: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PreviewWindow {
    pub start: f64,
    pub end: f64,
}

impl PreviewWindow {
    pub fn is_valid(self, duration: f64) -> bool {
        self.start.is_finite()
            && self.end.is_finite()
            && self.start >= 0.0
            && self.end <= duration
            && self.end > self.start
            && milliseconds(self.end - self.start) <= 60_000
    }
}

#[derive(Debug, Clone, Copy)]
pub enum VideoWork {
    Prepare,
    Transcript,
    Render,
    Preview(PreviewRequest),
}

impl VideoWork {
    pub fn stage(self) -> &'static str {
        match self {
            Self::Prepare => "preparing",
            Self::Transcript => "transcribing",
            Self::Render => "rendering",
            Self::Preview(_) => "previewing",
        }
    }

    pub fn window(self, plan: &VideoPlan) -> Result<Option<PreviewWindow>, String> {
        if matches!(self, Self::Prepare | Self::Transcript) {
            return Ok(None);
        }
        validate_theme(&plan.theme)?;
        validate_cues(&plan.cues, plan.duration)?;
        validate_scenes(&plan.scenes, 0, plan.cues.len())?;
        pencil::validate_plan(plan)?;
        if plan.image_provider_id.is_some()
            && plan
                .scenes
                .iter()
                .any(|scene| scene.painting.is_none() || scene.image.is_none())
        {
            return Err("水墨画作尚未全部完成，请先继续准备画作。".into());
        }
        if let Self::Preview(request) = self {
            if !matches!(request.seconds, 30 | 60) {
                return Err("试片请选择30秒或60秒。".into());
            }
            let scene = plan
                .scenes
                .get(request.first_scene)
                .ok_or("试片起始分镜不存在。")?;
            let start = if request.first_scene == 0 {
                0.0
            } else {
                plan.cues[scene.first].start
            };
            let window = PreviewWindow {
                start,
                end: (start + f64::from(request.seconds)).min(plan.duration),
            };
            if !window.is_valid(plan.duration) {
                return Err("该分镜没有可生成试片的音频范围。".into());
            }
            return Ok(Some(window));
        }
        Ok(None)
    }
}

pub fn validate_cues(cues: &[Cue], duration: f64) -> Result<(), String> {
    if !duration.is_finite() || duration <= 0.0 {
        return Err("音频时长无效，须为大于零的有限数值。".into());
    }
    if cues.is_empty() || cues.len() > 10000 {
        return Err("字幕为空或超过一万条。".into());
    }
    let mut end = 0.0;
    for (index, cue) in cues.iter().enumerate() {
        if !cue.start.is_finite()
            || !cue.end.is_finite()
            || cue.start < end
            || cue.end <= cue.start
            || cue.end > duration + 0.05
            || cue.text.trim().is_empty()
            || cue.text.chars().count() > 32
            || cue.text.chars().any(char::is_control)
        {
            return Err(format!(
                "第 {} 条字幕无效：须有不重叠的音频时间码，且每条为1—32字。",
                index + 1
            ));
        }
        end = cue.end;
    }
    Ok(())
}

pub fn validate_scenes(scenes: &[Scene], first: usize, end: usize) -> Result<(), String> {
    let catalog = visual_catalog();
    if scenes.is_empty() {
        return Err("分镜数组为空；须返回至少一镜，连续覆盖本次字幕范围。".into());
    }
    let mut next = first;
    for (index, scene) in scenes.iter().enumerate() {
        let error = |detail| scene_error(index, scene, detail);
        let mut drawing_points = 0;
        let mut drawing_strokes = 0;
        if scene.beats.is_empty()
            && scene.image.is_none()
            && scene.painting.is_none()
            && scene.artwork.is_none()
        {
            return Err(error(
                "beats 物体数为 0，应至少有一个主体；请保留字幕并重新生成分镜。".into(),
            ));
        }
        if let Some(painting) = &scene.painting {
            painting.validate().map_err(error)?;
            if !scene.beats.is_empty() {
                return Err(error("水墨画作与手绘物体不能同时作为主体。".into()));
            }
        }
        if scene.first != next {
            return Err(error(format!(
                "first={}，应为 {next}；分镜须连续覆盖字幕，不能遗漏或重复。",
                scene.first
            )));
        }
        if scene.last < scene.first {
            return Err(error(format!(
                "last={} 小于 first={}；末项索引不得小于起始索引。",
                scene.last, scene.first
            )));
        }
        if scene.last >= end {
            return Err(error(format!(
                "last={} 越界，应小于 {end}；本次起始字幕索引为 {first}。",
                scene.last
            )));
        }
        let cue_count = scene.last - scene.first + 1;
        if cue_count > catalog.limits.max_scene_cues && scene.artwork.is_none() {
            return Err(error(format!(
                "first/last 覆盖 {cue_count} 条字幕，每镜最多 {} 条。",
                catalog.limits.max_scene_cues
            )));
        }
        for (field, value, allowed) in [
            ("kind", &scene.kind, &catalog.kinds),
            ("environment", &scene.environment, &catalog.environments),
            ("framing", &scene.framing, &catalog.framings),
        ] {
            validate_catalog_choice(field, value, allowed).map_err(error)?;
        }
        if !scene.labels.is_empty() {
            return Err(error(format!(
                "旧版 labels 字段有 {} 项，应为空；物体标签须放在 beats 的 label 中。",
                scene.labels.len()
            )));
        }
        validate_short_text("title", &scene.title, catalog.limits.title_chars).map_err(error)?;
        if let Some(artwork) = &scene.artwork {
            if scene.image.is_some() || scene.painting.is_some() || !scene.beats.is_empty() {
                return Err(error("精绘场景不能与旧物体或图片混用。".into()));
            }
            artwork.validate(scene.first, scene.last).map_err(error)?;
        }
        if let Some(image) = &scene.image {
            image.validate().map_err(error)?;
            if !scene.beats.is_empty() {
                return Err(error("图片与手绘物体请选择一种画面主体。".into()));
            }
        }
        let mut previous = scene.first;
        for (beat_index, beat) in scene.beats.iter().enumerate() {
            let beat_error = |detail| error(format!("第 {} 个物体：{detail}", beat_index + 1));
            if beat.cue < scene.first || beat.cue > scene.last {
                return Err(beat_error(format!(
                    "cue={} 越界，应在 {}—{} 内。",
                    beat.cue, scene.first, scene.last
                )));
            }
            if beat.cue < previous {
                return Err(beat_error(format!(
                    "cue={} 小于前一物体的 {previous}；物体须按字幕索引顺序触发。",
                    beat.cue
                )));
            }
            if let Some(drawing) = &beat.drawing {
                validate_short_text("object", &beat.object, catalog.limits.object_chars)
                    .map_err(beat_error)?;
                drawing_points += drawing.validate().map_err(beat_error)?;
                drawing_strokes += drawing.strokes.len();
                if drawing_points > catalog.limits.max_drawing_points
                    || drawing_strokes > catalog.limits.max_drawing_strokes
                {
                    return Err(beat_error(format!(
                        "整镜笔画资源超过上限（{} 点 / {} 笔）；请简化重复线条，保留所需主体。",
                        catalog.limits.max_drawing_points, catalog.limits.max_drawing_strokes
                    )));
                }
            } else {
                validate_catalog_choice("object", &beat.object, &catalog.objects)
                    .map_err(beat_error)?;
            }
            validate_catalog_choice("action", &beat.action, &catalog.actions)
                .map_err(beat_error)?;
            if !beat.label.is_empty() || beat.drawing.is_none() {
                validate_short_text("label", &beat.label, catalog.limits.label_chars)
                    .map_err(beat_error)?;
            }
            previous = beat.cue;
        }
        next = scene.last + 1;
    }
    if next != end {
        return Err(format!(
            "第 {} 镜之后缺少字幕索引 {next}—{}（从 0 开始）；分镜须完整覆盖本次字幕范围。",
            scenes.len(),
            end - 1
        ));
    }
    Ok(())
}

fn scene_error(index: usize, scene: &Scene, detail: impl std::fmt::Display) -> String {
    format!(
        "第 {} 镜（字幕索引 {}—{}，从 0 开始）：{detail}",
        index + 1,
        scene.first,
        scene.last
    )
}

fn diagnostic_value(value: &str) -> String {
    let mut chars = value.chars();
    let mut preview: String = chars.by_ref().take(80).collect();
    if chars.next().is_some() {
        preview.push('…');
    }
    format!("{preview:?}")
}

fn validate_catalog_choice(
    field: &str,
    value: &str,
    allowed: &BTreeMap<String, String>,
) -> Result<(), String> {
    if !allowed.contains_key(value) {
        return Err(format!(
            "{field}={} 不在绘图目录中；允许值：{}。",
            diagnostic_value(value),
            allowed
                .keys()
                .map(String::as_str)
                .collect::<Vec<_>>()
                .join("、")
        ));
    }
    Ok(())
}

fn validate_short_text(field: &str, text: &str, max: usize) -> Result<(), String> {
    let count = text.chars().count();
    let reason = if text.trim().is_empty() {
        format!("为空；应为 1—{max} 字")
    } else if count > max {
        format!("共 {count} 字；最多 {max} 字")
    } else if text.chars().any(char::is_control) {
        "含控制字符；不得换行或包含控制字符".into()
    } else if text.chars().any(|c| matches!(c, '{' | '}' | '`' | '*')) {
        "含格式标记；不得包含 {、}、`、*".into()
    } else {
        return Ok(());
    };
    Err(format!("{field}={} {reason}。", diagnostic_value(text)))
}

fn timestamp(text: &str) -> Result<f64, String> {
    let parts: Vec<_> = text.split([':', ',']).collect();
    if parts.len() != 4
        || parts[0].len() != 2
        || parts[1].len() != 2
        || parts[2].len() != 2
        || parts[3].len() != 3
        || parts.iter().any(|p| !p.bytes().all(|b| b.is_ascii_digit()))
    {
        return Err("字幕时间格式须为 HH:MM:SS,mmm。".into());
    }
    let numbers: Vec<u32> = parts.iter().map(|p| p.parse().unwrap()).collect();
    if numbers[1] >= 60 || numbers[2] >= 60 {
        return Err("字幕时间无效。".into());
    }
    let millis = (numbers[0] * 3600 + numbers[1] * 60 + numbers[2]) * 1000 + numbers[3];
    Ok(f64::from(millis) / 1000.0)
}

pub fn parse_srt(text: &str, duration: f64) -> Result<Vec<Cue>, String> {
    if text.len() > 2_000_000 {
        return Err("字幕文件不能超过2MB。".into());
    }
    let normalized = text.trim_start_matches('\u{feff}').replace("\r\n", "\n");
    let mut cues = Vec::new();
    for block in normalized
        .trim()
        .split("\n\n")
        .filter(|b| !b.trim().is_empty())
    {
        let lines: Vec<_> = block.lines().collect();
        if lines.len() < 3 || lines[0].trim().parse::<usize>().ok() != Some(cues.len() + 1) {
            return Err("SRT字幕必须从1连续编号，并包含时间码与正文。".into());
        }
        let (start, end) = lines[1].split_once(" --> ").ok_or("SRT时间码无效。")?;
        cues.push(Cue {
            start: timestamp(start.trim())?,
            end: timestamp(end.trim())?,
            text: lines[2..].join(" ").trim().into(),
        });
    }
    validate_cues(&cues, duration)?;
    Ok(cues)
}

pub fn milliseconds(seconds: f64) -> u64 {
    (seconds * 1000.0).round() as u64
}

fn srt_time(seconds: f64) -> String {
    let millis = milliseconds(seconds);
    format!(
        "{:02}:{:02}:{:02},{:03}",
        millis / 3_600_000,
        millis / 60_000 % 60,
        millis / 1000 % 60,
        millis % 1000
    )
}

pub fn to_srt(cues: &[Cue]) -> String {
    cues.iter()
        .enumerate()
        .map(|(i, c)| {
            format!(
                "{}\n{} --> {}\n{}\n\n",
                i + 1,
                srt_time(c.start),
                srt_time(c.end),
                c.text
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn free_drawings_accept_arbitrary_subjects_and_many_objects() {
        let raw = serde_json::json!([{"first":0,"last":0,"kind":"focus","title":"营前对峙","beats":
            (0..12).map(|i| serde_json::json!({"cue":0,"object":format!("战士与长枪{i}"),"action":"draw","label":"",
                "drawing":{"strokes":[[[i*70,30],[i*70+30,200]],[[i*70+10,60],[i*70+20,40],[i*70+30,60]]]}})).collect::<Vec<_>>()
        }]);
        let scenes = parse_storyboard(&raw.to_string(), 0, 1).unwrap();
        require_drawings(&scenes).unwrap();
        assert_eq!(scenes[0].beats.len(), 12);
        let saved = serde_json::to_string(&scenes).unwrap();
        assert!(parse_storyboard(&saved, 0, 1).is_ok());
        for bad in [
            serde_json::json!([]),
            serde_json::json!([[[0, 0]]]),
            serde_json::json!([[[0, 0], [1001, 0]]]),
            serde_json::json!([[[0, 0], [1, 301]]]),
            serde_json::json!([[[0, 0], [0, 0]]]),
        ] {
            let mut value = raw.clone();
            value[0]["beats"][0]["drawing"]["strokes"] = bad;
            assert!(parse_storyboard(&value.to_string(), 0, 1).is_err());
        }
        let mut invalid = raw.clone();
        invalid[0]["beats"][0]["drawing"]["strokes"] =
            serde_json::json!(vec![vec![[0, 0], [1, 1]]; 2001]);
        assert!(parse_storyboard(&invalid.to_string(), 0, 1).is_err());
        let mut legacy = scenes;
        legacy[0].beats[0].drawing = None;
        assert!(require_drawings(&legacy)
            .unwrap_err()
            .contains("缺少 drawing.strokes"));
    }

    #[test]
    fn pencil_and_classic_share_the_protocol_but_keep_distinct_drawing_rules() {
        let pencil = storyboard_prompt("ink");
        let classic = storyboard_prompt("classic");
        assert!(pencil.contains("铅笔素描"));
        assert!(pencil.contains("不生成图片"));
        assert!(pencil.contains("排线"));
        assert!(classic.contains("书卷手绘"));
        assert!(!classic.contains("铅笔素描分镜规则"));
        assert!(revision::prompt("ink").contains("铅笔素描分镜规则"));
        for prompt in [&pencil, &classic] {
            assert!(prompt.contains("台词"));
            assert!(prompt.contains("maxSceneCues"));
        }
    }

    #[test]
    fn storyboard_batches_prefer_sentence_endings_without_losing_the_hard_limit() {
        let mut cues = (0..50)
            .map(|i| Cue {
                start: i as f64,
                end: i as f64 + 0.9,
                text: "继续讲述".into(),
            })
            .collect::<Vec<_>>();
        cues[6].text = "第一个完整观点。".into();
        cues[8].text = "接下来如何选择？".into();
        assert_eq!(storyboard_batch_end(&cues, 0), 9);
        assert_eq!(storyboard_batch_end(&cues, 9), 21);
        assert_eq!(storyboard_batch_end(&cues, 45), 50);
    }

    #[test]
    fn preview_window_uses_saved_scene_boundaries_and_never_changes_the_source_plan() {
        let plan: VideoPlan = serde_json::from_value(serde_json::json!({
            "title":"原声", "audioSha256":"unchanged", "duration":90,
            "cues":[{"start":0.5,"end":2,"text":"开场"},{"start":35.125,"end":50,"text":"正文"},{"start":75,"end":85,"text":"结尾"}],
            "scenes":[
                {"first":0,"last":0,"kind":"opening","title":"开场","beats":[{"cue":0,"object":"book","action":"draw","label":"书籍"}]},
                {"first":1,"last":1,"kind":"focus","title":"正文","beats":[{"cue":1,"object":"pen","action":"draw","label":"书写"}]},
                {"first":2,"last":2,"kind":"closing","title":"结尾","beats":[{"cue":2,"object":"book","action":"draw","label":"回扣"}]}
            ]
        })).unwrap();
        let original = serde_json::to_string(&plan).unwrap();
        let work = VideoWork::Preview(PreviewRequest {
            first_scene: 1,
            seconds: 30,
        });
        let window = work.window(&plan).unwrap().unwrap();
        assert_eq!((window.start, window.end), (35.125, 65.125));
        let tail = VideoWork::Preview(PreviewRequest {
            first_scene: 2,
            seconds: 30,
        })
        .window(&plan)
        .unwrap()
        .unwrap();
        assert_eq!((tail.start, tail.end), (75.0, 90.0));
        for request in [
            PreviewRequest {
                first_scene: 3,
                seconds: 30,
            },
            PreviewRequest {
                first_scene: 0,
                seconds: 90,
            },
        ] {
            assert!(VideoWork::Preview(request).window(&plan).is_err());
        }
        assert_eq!(serde_json::to_string(&plan).unwrap(), original);
        assert!(VideoWork::Preview(PreviewRequest {
            first_scene: 0,
            seconds: 30
        })
        .window(&VideoPlan::default())
        .is_err());
    }

    #[test]
    fn saved_plans_default_to_the_classic_theme_and_reject_unknown_styles() {
        let mut plan: VideoPlan = serde_json::from_value(serde_json::json!({
            "title":"原声", "audioSha256":"hash", "duration":1,
            "cues":[{"start":0,"end":1,"text":"一句"}],
            "scenes":[{"first":0,"last":0,"kind":"opening","title":"开场","beats":[{"cue":0,"object":"book","action":"draw","label":"书"}]}]
        })).unwrap();
        assert_eq!(plan.theme, "classic");
        assert!(serde_json::to_value(&plan).unwrap().get("theme").is_none());
        assert!(validate_theme("classic").is_ok());
        assert!(validate_theme("ink").is_ok());
        plan.theme = "watercolor".into();
        assert!(serde_json::to_value(&plan).unwrap().get("theme").is_some());
        assert!(VideoWork::Render.window(&plan).is_err());
    }

    #[test]
    fn subtitles_preserve_measured_silence_and_round_trip() {
        let srt = "\u{feff}1\r\n00:00:00,500 --> 00:00:02,000\r\n第一句\r\n\r\n2\r\n00:00:05,125 --> 00:00:08,000\r\n第二句\r\n";
        let cues = parse_srt(srt, 10.0).unwrap();
        assert_eq!(cues[1].start, 5.125);
        assert_eq!(to_srt(&cues), "1\n00:00:00,500 --> 00:00:02,000\n第一句\n\n2\n00:00:05,125 --> 00:00:08,000\n第二句\n\n");
        assert!(parse_srt(&to_srt(&cues), 7.0).is_err());
    }
    #[test]
    fn rejects_overlapping_empty_long_and_nonfinite_captions() {
        for cue in [
            Cue {
                start: 1.0,
                end: 0.5,
                text: "错".into(),
            },
            Cue {
                start: f64::NAN,
                end: 1.0,
                text: "错".into(),
            },
            Cue {
                start: 0.0,
                end: 1.0,
                text: "字".repeat(33),
            },
        ] {
            assert!(validate_cues(&[cue], 10.0).is_err());
        }
        assert!(validate_cues(&[], 10.0).is_err());
        assert!(parse_srt(
            "1\n00:00:00,000 --> 00:00:02,000\n前\n\n2\n00:00:01,000 --> 00:00:03,000\n后",
            10.0
        )
        .is_err());
    }
    #[test]
    fn subtitles_use_the_full_audio_timeline_without_a_duration_cap() {
        let subtitles =
            "1\n00:00:00,000 --> 00:00:02,000\n开头\n\n2\n02:59:58,000 --> 03:00:00,000\n结尾";
        let cues = parse_srt(subtitles, 10800.0).unwrap();
        assert_eq!(cues[1].end, 10800.0);
        for duration in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert!(validate_cues(&cues, duration).is_err());
        }
        assert!(validate_cues(&cues, 10799.0).is_err());
    }
    #[test]
    fn generated_storyboards_start_each_subject_at_the_first_cue_but_keep_later_conclusions() {
        for first in [0, 24] {
            let raw = serde_json::json!([
                {"first":first,"last":first+2,"kind":"focus","title":"原书","beats":[
                    {"cue":first,"object":"book","action":"draw","label":"原书"},
                    {"cue":first+2,"object":"certificate","action":"check","label":"结论"}]},
                {"first":first+3,"last":first+5,"kind":"route","title":"写作","beats":[
                    {"cue":first+3,"object":"pen","action":"draw","label":"写作"}]}
            ]);
            let scenes = parse_storyboard(&raw.to_string(), first, first + 6).unwrap();
            assert_eq!(scenes[0].beats[0].cue, first);
            assert_eq!(scenes[0].beats[1].cue, first + 2);
            assert_eq!(scenes[0].beats[1].action, "check");
            assert_eq!(scenes[1].beats[0].cue, first + 3);
            for scene_index in [0, 1] {
                let mut delayed = raw.clone();
                delayed[scene_index]["beats"][0]["cue"] = (scenes[scene_index].first + 1).into();
                let delayed = delayed.to_string();
                let manual: Vec<Scene> = serde_json::from_str(&delayed).unwrap();
                assert!(validate_scenes(&manual, first, first + 6).is_ok());
                assert!(parse_storyboard(&delayed, first, first + 6)
                    .unwrap_err()
                    .contains("第一条字幕"));
            }
        }
    }

    #[test]
    fn storyboard_validation_reports_the_scene_field_value_and_constraint() {
        use serde_json::json;
        let raw = json!([
            {"first":24,"last":26,"kind":"focus","title":"原书","beats":[{"cue":24,"object":"book","action":"draw","label":"书"}]},
            {"first":27,"last":29,"kind":"route","title":"变化","beats":[{"cue":27,"object":"pen","action":"draw","label":"写作"}]}
        ]);
        for (field, value, end, expected) in [
            ("first", json!(28), 30, vec!["first=28", "应为 27", "连续"]),
            ("first", json!(26), 30, vec!["first=26", "应为 27", "连续"]),
            ("last", json!(26), 30, vec!["last=26", "first=27"]),
            (
                "last",
                json!(30),
                30,
                vec!["last=30", "小于 30", "索引为 24"],
            ),
            ("last", json!(43), 44, vec!["17 条字幕", "最多 16 条"]),
            (
                "kind",
                json!("unknown-kind"),
                30,
                vec!["kind", "unknown-kind", "允许值", "focus"],
            ),
            (
                "environment",
                json!("forest"),
                30,
                vec!["environment", "forest", "允许值", "courtyard"],
            ),
            (
                "framing",
                json!("zoom"),
                30,
                vec!["framing", "zoom", "允许值", "medium"],
            ),
            ("title", json!(""), 30, vec!["title", "为空", "1—20"]),
            (
                "title",
                json!("长".repeat(21)),
                30,
                vec!["title", "21 字", "最多 20"],
            ),
            ("title", json!("**标题**"), 30, vec!["title", "格式标记"]),
            ("title", json!("两行\n标题"), 30, vec!["title", "控制字符"]),
            (
                "labels",
                json!(["旧字段"]),
                30,
                vec!["labels", "1 项", "为空"],
            ),
            (
                "beats",
                json!([]),
                30,
                vec!["beats", "0", "至少", "重新生成分镜"],
            ),
        ] {
            let mut invalid = raw.clone();
            invalid[1][field] = value;
            let error = parse_storyboard(&invalid.to_string(), 24, end).unwrap_err();
            assert!(error.contains("第 2 镜"), "{field}: {error}");
            assert!(
                error.contains("本批") && error.contains("从 0 开始"),
                "{error}"
            );
            for part in expected {
                assert!(error.contains(part), "missing {part}: {error}");
            }
        }
        assert!(parse_storyboard(&raw.to_string(), 24, 30).is_ok());
        let error = parse_storyboard(&raw.to_string(), 24, 31).unwrap_err();
        assert!(error.contains("缺少字幕索引 30—30"), "{error}");
        assert!(parse_storyboard("[]", 24, 30)
            .unwrap_err()
            .contains("分镜数组为空"));
    }

    #[test]
    fn storyboard_validation_locates_invalid_beats_and_bounds_value_previews() {
        use serde_json::json;
        let raw = json!([{"first":24,"last":27,"kind":"focus","title":"原书","beats":[
            {"cue":24,"object":"book","action":"draw","label":"原书"},
            {"cue":26,"object":"pen","action":"draw","label":"写作"}
        ]}]);
        for (field, value, expected) in [
            ("cue", json!(23), vec!["cue=23", "24—27"]),
            ("cue", json!(28), vec!["cue=28", "24—27"]),
            (
                "object",
                json!("dragon"),
                vec!["object", "dragon", "允许值", "book"],
            ),
            (
                "action",
                json!("spin"),
                vec!["action", "spin", "允许值", "draw"],
            ),
            (
                "label",
                json!("过长的物体标签九字"),
                vec!["label", "9 字", "最多 8"],
            ),
        ] {
            let mut invalid = raw.clone();
            invalid[0]["beats"][1][field] = value;
            let error = parse_storyboard(&invalid.to_string(), 24, 28).unwrap_err();
            assert!(
                error.contains("第 1 镜") && error.contains("第 2 个物体"),
                "{error}"
            );
            for part in expected {
                assert!(error.contains(part), "missing {part}: {error}");
            }
        }
        let mut invalid = raw.clone();
        invalid[0]["beats"][0]["cue"] = json!(25);
        invalid[0]["beats"][1]["cue"] = json!(24);
        let error = parse_storyboard(&invalid.to_string(), 24, 28).unwrap_err();
        assert!(
            error.contains("cue=24") && error.contains("前一物体的 25"),
            "{error}"
        );
        invalid[0]["beats"][1]["cue"] = json!(26);
        let error = parse_storyboard(&invalid.to_string(), 24, 28).unwrap_err();
        assert!(
            error.contains("第 1 镜") && error.contains("cue=25") && error.contains("应为 24"),
            "{error}"
        );
        invalid = raw;
        invalid[0]["title"] = json!(format!("{}PRIVATE_TAIL", "长".repeat(2000)));
        let error = parse_storyboard(&invalid.to_string(), 24, 28).unwrap_err();
        assert!(error.contains("2012 字"), "{error}");
        assert!(!error.contains("PRIVATE_TAIL") && error.chars().count() < 300);
    }

    #[test]
    fn storyboard_must_cover_exact_batch_and_use_allowed_components() {
        let mut scenes = vec![Scene {
            image: None,
            painting: None,
            artwork: None,
            first: 24,
            last: 27,
            kind: "route".into(),
            title: "变化".into(),
            environment: "plain".into(),
            framing: "reference".into(),
            labels: vec![],
            beats: vec![
                Beat {
                    drawing: None,
                    cue: 24,
                    object: "book".into(),
                    action: "draw".into(),
                    label: "之前".into(),
                },
                Beat {
                    drawing: None,
                    cue: 27,
                    object: "certificate".into(),
                    action: "check".into(),
                    label: "之后".into(),
                },
            ],
        }];
        assert!(validate_scenes(&scenes, 24, 28).is_ok());
        assert!(validate_scenes(&scenes, 0, 28).is_err());
        scenes[0].kind = "eval".into();
        assert!(validate_scenes(&scenes, 24, 28).is_err());
        assert!(serde_json::from_str::<Scene>(
            r#"{"first":0,"last":0,"kind":"book","title":"x","labels":["x"],"code":"evil"}"#
        )
        .is_err());
    }

    #[test]
    fn object_storyboards_reject_unknown_drawings_actions_labels_and_anchors() {
        let raw = r#"{"first":0,"last":3,"kind":"collapse","title":"变化","beats":[{"cue":0,"object":"crown","action":"tilt","label":"皇冠"},{"cue":3,"object":"certificate","action":"check","label":"公民"}]}"#;
        let scene: Scene = serde_json::from_str(raw).unwrap();
        assert!(validate_scenes(std::slice::from_ref(&scene), 0, 4).is_ok());
        for (field, value) in [
            ("object", "../../evil.js"),
            ("action", "eval"),
            ("label", "超过八个字的物体标签"),
            ("label", "{r:错误}"),
        ] {
            let mut value_json: serde_json::Value = serde_json::from_str(raw).unwrap();
            value_json["beats"][0][field] = serde_json::Value::String(value.into());
            let bad: Scene = serde_json::from_value(value_json).unwrap();
            assert!(validate_scenes(&[bad], 0, 4).is_err());
        }
        let mut bad = scene.clone();
        bad.beats[1].cue = 4;
        assert!(validate_scenes(&[bad], 0, 4).is_err());
        let mut bad = scene;
        bad.beats[0].cue = 2;
        bad.beats[1].cue = 1;
        assert!(validate_scenes(&[bad], 0, 4).is_err());
        let old: Scene = serde_json::from_str(
            r#"{"first":0,"last":3,"kind":"book","title":"旧图解","labels":["书籍"]}"#,
        )
        .unwrap();
        assert!(validate_scenes(&[old], 0, 4)
            .unwrap_err()
            .contains("重新生成分镜"));
    }

    #[test]
    fn scene_layers_default_only_when_absent_and_validate_against_the_shared_catalog() {
        let raw = serde_json::json!({"first":0,"last":0,"kind":"focus","title":"方向","beats":[{"cue":0,"object":"book","action":"draw","label":"阅读"}]});
        let old: Scene = serde_json::from_value(raw.clone()).unwrap();
        let decoded = serde_json::to_value(&old).unwrap();
        assert_eq!(decoded["environment"], "plain");
        assert_eq!(decoded["framing"], "reference");
        for environment in [
            "plain",
            "study",
            "courtyard",
            "street",
            "riverbank",
            "window",
            "path",
        ] {
            for framing in ["reference", "wide", "medium", "detail"] {
                let mut value = raw.clone();
                value["environment"] = environment.into();
                value["framing"] = framing.into();
                value["beats"][0]["object"] = "compass".into();
                let scene: Scene = serde_json::from_value(value).unwrap();
                assert!(validate_scenes(&[scene], 0, 1).is_ok());
            }
        }
        for (field, invalid) in [
            ("environment", "forest"),
            ("environment", "../scene.svg"),
            ("framing", "zoom"),
        ] {
            let mut value = raw.clone();
            value[field] = invalid.into();
            let scene: Scene = serde_json::from_value(value).unwrap();
            assert!(validate_scenes(&[scene], 0, 1).is_err());
        }
        for field in ["environment", "framing", "camera"] {
            let mut value = raw.clone();
            value[field] = serde_json::Value::Null;
            assert!(serde_json::from_value::<Scene>(value).is_err());
        }
    }

    #[test]
    fn storyboard_requests_share_the_catalog_and_keep_context_outside_batch_coverage() {
        let plan = VideoPlan {
            title: "原书".into(),
            duration: 30.0,
            cues: (0..30)
                .map(|i| Cue {
                    start: i as f64,
                    end: i as f64 + 1.0,
                    text: format!("字幕{i}"),
                })
                .collect(),
            ..Default::default()
        };
        let input: serde_json::Value =
            serde_json::from_str(&storyboard_input(&plan, 24, 30)).unwrap();
        assert_eq!(input["cues"].as_array().unwrap().len(), 6);
        assert_eq!(input["cues"][0]["index"], 24);
        assert_eq!(input["opening"][0]["index"], 0);
        assert_eq!(input["ending"][5]["index"], 29);
        assert_eq!(visual_catalog().objects.len(), 93);
        assert_eq!(visual_catalog().environments.len(), 7);
        assert_eq!(visual_catalog().framings.len(), 4);
        assert!(storyboard_prompt("classic").contains("drawing.strokes"));
        assert!(!storyboard_prompt("classic").contains("\"objects\""));
    }
}
