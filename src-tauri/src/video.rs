use serde::{Deserialize, Serialize};

pub const PROMPT: &str = include_str!("../../prompts/video-storyboard-v1.md");
pub const BATCH_SIZE: usize = 24;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VideoSettings {
    pub engine_root: String,
    pub node_path: String,
    pub python_path: String,
    pub model_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cue {
    pub start: f64,
    pub end: f64,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scene {
    pub first: usize,
    pub last: usize,
    pub kind: String,
    pub title: String,
    pub labels: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VideoPlan {
    pub title: String,
    pub audio_sha256: String,
    pub duration: f64,
    pub cues: Vec<Cue>,
    pub scenes: Vec<Scene>,
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
    let mut next = first;
    for scene in scenes {
        let count = scene.labels.len();
        let valid_kind = match scene.kind.as_str() {
            "book" => (1..=2).contains(&count),
            "quote" => (1..=3).contains(&count),
            "compare" => count == 2,
            "flow" | "timeline" => (2..=4).contains(&count),
            _ => false,
        };
        if scene.first != next
            || scene.last < scene.first
            || scene.last >= end
            || scene.last - scene.first >= 8
            || !valid_kind
            || !short_text(&scene.title, 20)
            || scene.labels.iter().any(|label| !short_text(label, 12))
        {
            return Err("分镜无效：须连续覆盖字幕，每镜最多8条，使用支持的图形和短标签。".into());
        }
        next = scene.last + 1;
    }
    if next != end || scenes.is_empty() {
        return Err("分镜未完整覆盖本段字幕。".into());
    }
    Ok(())
}

fn short_text(text: &str, max: usize) -> bool {
    !text.trim().is_empty() && text.chars().count() <= max && !text.chars().any(char::is_control)
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
    fn storyboard_must_cover_exact_batch_and_use_allowed_components() {
        let mut scenes = vec![Scene {
            first: 24,
            last: 27,
            kind: "flow".into(),
            title: "变化".into(),
            labels: vec!["之前".into(), "之后".into()],
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
}
