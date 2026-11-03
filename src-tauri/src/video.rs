use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::OnceLock};

pub mod environment;
pub mod subtitles;

pub const PROMPT: &str = include_str!("../../prompts/video-storyboard-v5.md");
pub const BATCH_SIZE: usize = 24;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct VisualLimits {
    max_scene_cues: usize,
    max_beats: usize,
    label_chars: usize,
    title_chars: usize,
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

pub fn storyboard_prompt() -> String {
    format!(
        "{PROMPT}\n可用绘图目录（仅可选用这些键）：\n{}",
        include_str!("../video/visual-catalog.json")
    )
}

pub fn parse_storyboard(raw: &str, first: usize, end: usize) -> Result<Vec<Scene>, String> {
    let scenes: Vec<Scene> =
        serde_json::from_str(raw).map_err(|_| "模型分镜不是合法 JSON，已保留字幕和此前分镜。")?;
    validate_scenes(&scenes, first, end)?;
    if scenes.iter().any(|scene| scene.image.is_some()) {
        return Err("分镜模型只能选择手绘目录；关键图片须由用户选用并确认来源。".into());
    }
    if scenes.iter().any(|scene| scene.beats[0].cue != scene.first) {
        return Err(
            "模型分镜的首个物体须从本镜第一条字幕开始。请点击“继续准备字幕与分镜”重试本批，字幕和此前分镜已保留。"
                .into(),
        );
    }
    Ok(scenes)
}

pub fn storyboard_input(plan: &VideoPlan, first: usize, end: usize) -> String {
    let cues = |from: usize, to: usize| {
        plan.cues[from..to].iter().enumerate().map(|(offset, c)| serde_json::json!({"index": from + offset, "start": c.start, "end": c.end, "text": c.text})).collect::<Vec<_>>()
    };
    serde_json::json!({
        "book": plan.title,
        "duration": plan.duration,
        "opening": cues(0, plan.cues.len().min(6)),
        "ending": cues(plan.cues.len().saturating_sub(6), plan.cues.len()),
        "previousScenes": &plan.scenes[plan.scenes.len().saturating_sub(3)..],
        "followingContext": cues(end, (end + 3).min(plan.cues.len())),
        "cues": cues(first, end),
    })
    .to_string()
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
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VideoPlan {
    pub title: String,
    pub audio_sha256: String,
    pub duration: f64,
    pub cues: Vec<Cue>,
    pub scenes: Vec<Scene>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subtitle_review: Option<subtitles::SubtitleReview>,
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

#[derive(Debug, Clone, Copy)]
pub enum VideoWork {
    Prepare,
    Render,
    Preview(PreviewRequest),
}

impl VideoWork {
    pub fn stage(self) -> &'static str {
        match self {
            Self::Prepare => "preparing",
            Self::Render => "rendering",
            Self::Preview(_) => "previewing",
        }
    }

    pub fn window(self, plan: &VideoPlan) -> Result<Option<PreviewWindow>, String> {
        if matches!(self, Self::Prepare) {
            return Ok(None);
        }
        validate_cues(&plan.cues, plan.duration)?;
        validate_scenes(&plan.scenes, 0, plan.cues.len())?;
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
            return Ok(Some(PreviewWindow {
                start,
                end: (start + f64::from(request.seconds)).min(plan.duration),
            }));
        }
        Ok(None)
    }
}

pub fn validate_cues(cues: &[Cue], duration: f64) -> Result<(), String> {
    if !duration.is_finite() || !(0.0..=7200.0).contains(&duration) || duration == 0.0 {
        return Err("首版支持时长不超过两小时的有效音频。".into());
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
    let mut next = first;
    for scene in scenes {
        if scene.beats.is_empty() && scene.image.is_none() {
            return Err("旧版文字图解分镜：请保留字幕并重新生成分镜，升级为同款手绘动画。".into());
        }
        if scene.first != next
            || scene.last < scene.first
            || scene.last >= end
            || scene.last - scene.first >= catalog.limits.max_scene_cues
            || !catalog.kinds.contains_key(&scene.kind)
            || !catalog.environments.contains_key(&scene.environment)
            || !catalog.framings.contains_key(&scene.framing)
            || !scene.labels.is_empty()
            || scene.beats.len() > catalog.limits.max_beats
            || !short_text(&scene.title, catalog.limits.title_chars)
        {
            return Err(
                "分镜无效：须连续覆盖字幕，每镜最多16条、1—3个具体物体，使用目录内的构图、场景、景别和短标题。"
                    .into(),
            );
        }
        if let Some(image) = &scene.image {
            image.validate()?;
            if !scene.beats.is_empty() {
                return Err("图片与手绘物体请选择一种画面主体。".into());
            }
        }
        let mut previous = scene.first;
        for beat in &scene.beats {
            if beat.cue < previous
                || beat.cue > scene.last
                || !catalog.objects.contains_key(&beat.object)
                || !catalog.actions.contains_key(&beat.action)
                || !short_text(&beat.label, catalog.limits.label_chars)
            {
                return Err(
                    "物体分镜无效：请选择目录中的物体和动作，按本镜字幕顺序触发，每个标签最多8字。"
                        .into(),
                );
            }
            previous = beat.cue;
        }
        next = scene.last + 1;
    }
    if next != end || scenes.is_empty() {
        return Err("分镜未完整覆盖本段字幕。".into());
    }
    Ok(())
}

fn short_text(text: &str, max: usize) -> bool {
    !text.trim().is_empty()
        && text.chars().count() <= max
        && !text
            .chars()
            .any(|c| c.is_control() || matches!(c, '{' | '}' | '`' | '*'))
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
    Ok((numbers[0] * 3600 + numbers[1] * 60 + numbers[2]) as f64 + numbers[3] as f64 / 1000.0)
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

fn srt_time(seconds: f64) -> String {
    let millis = (seconds * 1000.0).round() as u64;
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
    fn storyboard_batches_prefer_sentence_endings_without_losing_the_hard_limit() {
        let mut cues = (0..50)
            .map(|i| Cue {
                start: i as f64,
                end: i as f64 + 0.9,
                text: "继续讲述".into(),
            })
            .collect::<Vec<_>>();
        cues[18].text = "第一个完整观点。".into();
        cues[20].text = "接下来如何选择？".into();
        assert_eq!(storyboard_batch_end(&cues, 0), 21);
        assert_eq!(storyboard_batch_end(&cues, 21), 45);
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
    fn storyboard_must_cover_exact_batch_and_use_allowed_components() {
        let mut scenes = vec![Scene {
            image: None,
            first: 24,
            last: 27,
            kind: "route".into(),
            title: "变化".into(),
            environment: "plain".into(),
            framing: "reference".into(),
            labels: vec![],
            beats: vec![
                Beat {
                    cue: 24,
                    object: "book".into(),
                    action: "draw".into(),
                    label: "之前".into(),
                },
                Beat {
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
        assert!(storyboard_prompt().contains("certificate"));
        assert!(storyboard_prompt().contains("compass"));
    }
}
