use super::{validate_cues, VideoPlan};
use serde::{Deserialize, Serialize};

pub const PROMPT: &str = include_str!("../../../prompts/subtitle-proofread-v1.md");
pub const VERSION: &str = "subtitle-proofread-v1";
pub const BATCH_SIZE: usize = 60;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SubtitleReview {
    pub prompt_version: String,
    pub completed: usize,
    pub reference: SubtitleReference,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SubtitleReference {
    pub book: String,
    pub author: String,
    pub opening: String,
    pub ending: String,
}

impl SubtitleReference {
    pub fn from_prompt(book: String, author: String, prompt: &str) -> Self {
        let mut reference = Self {
            book,
            author,
            ..Default::default()
        };
        let mut section = "";
        for line in prompt.lines() {
            let line = line.trim();
            if let Some(heading) = line.strip_prefix("## ") {
                section = if heading.starts_with("固定开场") || heading.starts_with("开场") {
                    "opening"
                } else if heading.starts_with("固定结尾") || heading.starts_with("结尾") {
                    "ending"
                } else {
                    ""
                };
            } else if line.starts_with('#') || line == "---" {
                section = "";
            } else if !line.is_empty() {
                let target = match section {
                    "opening" => &mut reference.opening,
                    "ending" => &mut reference.ending,
                    _ => continue,
                };
                if !target.is_empty() {
                    target.push('\n');
                }
                target.push_str(line);
            }
        }
        reference
    }
}

impl SubtitleReview {
    pub fn new(reference: SubtitleReference) -> Self {
        Self {
            prompt_version: VERSION.into(),
            completed: 0,
            reference,
        }
    }
}

pub fn pending(plan: &VideoPlan) -> bool {
    plan.subtitle_review
        .as_ref()
        .is_some_and(|review| review.completed < plan.cues.len())
}

pub fn input(plan: &VideoPlan, first: usize, end: usize) -> String {
    let cues = |from: usize, to: usize| {
        plan.cues[from..to].iter().enumerate().map(|(offset, cue)| {
        serde_json::json!({"index": from + offset, "start": cue.start, "end": cue.end, "text": cue.text})
    }).collect::<Vec<_>>()
    };
    serde_json::json!({
        "reference": plan.subtitle_review.as_ref().map(|r| &r.reference),
        "totalCues": plan.cues.len(),
        "duration": plan.duration,
        "previousContext": cues(first.saturating_sub(6), first),
        "followingContext": cues(end, (end + 6).min(plan.cues.len())),
        "cues": cues(first, end),
    })
    .to_string()
}

// Validate the entire response before changing any saved text. Timecodes are never model output.
pub fn apply(plan: &mut VideoPlan, first: usize, end: usize, raw: &str) -> Result<(), String> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct ReviewedCue {
        index: usize,
        text: String,
    }
    let invalid = "字幕校对结果无效：必须逐条保留索引、数量和1—32字单行文本。已保存字幕未修改。";
    let review = plan.subtitle_review.as_ref().ok_or(invalid)?;
    if first != review.completed
        || end <= first
        || end > plan.cues.len()
        || end - first > BATCH_SIZE
    {
        return Err(invalid.into());
    }
    let reviewed: Vec<ReviewedCue> = serde_json::from_str(raw).map_err(|_| invalid)?;
    if reviewed.len() != end - first
        || reviewed
            .iter()
            .enumerate()
            .any(|(i, cue)| cue.index != first + i)
    {
        return Err(invalid.into());
    }
    let mut cues = plan.cues.clone();
    for (cue, checked) in cues[first..end].iter_mut().zip(reviewed) {
        cue.text = checked.text;
    }
    validate_cues(&cues, plan.duration).map_err(|_| invalid)?;
    plan.cues = cues;
    plan.subtitle_review.as_mut().ok_or(invalid)?.completed = end;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan() -> VideoPlan {
        VideoPlan {
            duration: 100.0,
            cues: (0..65).map(|i| super::super::Cue { start: i as f64, end: i as f64 + 0.9, text: "我是夹子".into() }).collect(),
            subtitle_review: Some(SubtitleReview::new(SubtitleReference::from_prompt("原书".into(), "作者".into(), "## 固定开场（逐字朗读）\n我是甲子。\n## 正文\n不作为逐字稿\n## 固定结尾（逐字朗读）\n下一期继续读书。"))),
            ..Default::default()
        }
    }

    #[test]
    fn extracts_saved_bookends_without_treating_guidance_as_a_transcript() {
        let reference = plan().subtitle_review.unwrap().reference;
        assert_eq!(reference.opening, "我是甲子。");
        assert_eq!(reference.ending, "下一期继续读书。");
        for template in [
            include_str!("../../../prompts/audio-template-v1.md"),
            include_str!("../../../prompts/audio-template-v2.md"),
            include_str!("../../../prompts/audio-template-v3.md"),
        ] {
            let reference = SubtitleReference::from_prompt("书".into(), String::new(), template);
            assert!(reference.opening.contains("甲子"));
            assert!(reference.ending.contains("甲子"));
            assert!(!reference.opening.contains("##"));
        }
        let reference = SubtitleReference::from_prompt(
            "书".into(),
            String::new(),
            "自由格式指导，不猜开场和结尾。",
        );
        assert!(reference.opening.is_empty() && reference.ending.is_empty());
    }

    #[test]
    fn batches_cover_every_cue_with_neighbour_context_and_bookends() {
        let mut plan = plan();
        let original = plan.cues.clone();
        for (first, end) in [(0, 60), (60, 65)] {
            let input: serde_json::Value = serde_json::from_str(&input(&plan, first, end)).unwrap();
            assert_eq!(input["cues"].as_array().unwrap().len(), end - first);
            assert_eq!(input["cues"][0]["index"], first);
            assert_eq!(input["reference"]["opening"], "我是甲子。");
            if first == 60 {
                assert_eq!(input["previousContext"][0]["index"], 54);
                assert_eq!(input["previousContext"][0]["text"], "我是甲子");
                assert!(input["followingContext"].as_array().unwrap().is_empty());
            } else {
                assert_eq!(input["followingContext"][0]["index"], 60);
            }
            let raw = serde_json::to_string(
                &(first..end)
                    .map(|index| serde_json::json!({"index":index,"text":"我是甲子"}))
                    .collect::<Vec<_>>(),
            )
            .unwrap();
            apply(&mut plan, first, end, &raw).unwrap();
        }
        assert!(!pending(&plan));
        for (before, after) in original.iter().zip(&plan.cues) {
            assert_eq!((before.start, before.end), (after.start, after.end));
            assert_eq!(after.text, "我是甲子");
        }
        assert!(plan.scenes.is_empty());
        assert!(
            super::super::to_srt(&plan.cues).contains("00:00:00,000 --> 00:00:00,900\n我是甲子")
        );
    }

    #[test]
    fn malformed_partial_reordered_or_timestamp_edits_never_change_saved_text() {
        for raw in [
            "not json",
            "[]",
            r#"[{"index":0,"text":"我是甲子"}]"#,
            r#"[{"index":1,"text":"我是甲子"},{"index":0,"text":"原文"}]"#,
            r#"[{"index":0,"text":"我是甲子"},{"index":0,"text":"原文"}]"#,
            r#"[{"index":0,"text":"我是甲子"},{"index":2,"text":"原文"}]"#,
            r#"[{"index":0,"text":"我是甲子"},{"index":1,"text":""}]"#,
            r#"[{"index":0,"text":"我是甲子"},{"index":1,"text":"两行\n文字"}]"#,
            r#"[{"index":0,"text":"我是甲子","start":1},{"index":1,"text":"原文"}]"#,
            r#"[{"index":0,"text":"我是甲子"},{"index":1,"text":"123456789012345678901234567890123"}]"#,
        ] {
            let mut plan = plan();
            let before = serde_json::to_string(&plan).unwrap();
            assert!(apply(&mut plan, 0, 2, raw).is_err(), "{raw}");
            assert_eq!(serde_json::to_string(&plan).unwrap(), before);
        }
    }
}
