use super::WritingContent;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CoverDesign {
    pub title_size: u32,
    pub subtitle_size: u32,
    pub text_color: String,
    pub subtitle_color: String,
    pub accent_color: String,
    pub background_color: String,
    pub overlay_opacity: f64,
}
impl CoverDesign {
    pub fn validate(&self) -> Result<(), String> {
        if !(40..=160).contains(&self.title_size)
            || !(24..=80).contains(&self.subtitle_size)
            || !(0.0..=0.85).contains(&self.overlay_opacity)
            || [
                &self.text_color,
                &self.subtitle_color,
                &self.accent_color,
                &self.background_color,
            ]
            .iter()
            .any(|color| {
                color.len() != 7
                    || !color.starts_with('#')
                    || !color.as_bytes()[1..].iter().all(u8::is_ascii_hexdigit)
            })
        {
            return Err("封面推荐字号、颜色或蒙层无效，原有内容保留。".into());
        }
        Ok(())
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PublishingDraft {
    title: String,
    description: String,
    introduction: String,
    sections: Vec<Section>,
    cover_title: String,
    cover_subtitle: String,
    cover_prompt_landscape: String,
    cover_prompt_portrait: String,
    cover_design: CoverDesign,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Section {
    heading: String,
    guidance: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AudioDraft {
    introduction: String,
    sections: Vec<Section>,
}
impl AudioDraft {
    pub fn into_prompt(self, book: &str, duration_seconds: i64) -> Result<String, String> {
        super::audio::validate_duration(duration_seconds)?;
        if self.sections.len() != 5
            || self.introduction.trim().is_empty()
            || self.introduction.chars().count() > 1500
            || self.sections.iter().any(|s| {
                s.heading.trim().is_empty()
                    || s.heading.chars().count() > 100
                    || s.guidance.trim().is_empty()
                    || s.guidance.chars().count() > 2000
            })
        {
            return Err("音频提示词需要书籍引入及五个完整部分，原有内容保留。".into());
        }
        let opening = (duration_seconds / 3).min(60);
        let closing = (duration_seconds / 3).min(45);
        let body = duration_seconds - opening - closing;
        let sections = self
            .sections
            .iter()
            .zip(["一", "二", "三", "四", "五"])
            .enumerate()
            .map(|(index, (s, number))| {
                let seconds = body / 5 + i64::from((index as i64) < body % 5);
                format!(
                    "## 第{number}部分：{}（约{}）\n\n{}",
                    s.heading.trim(),
                    super::audio::duration_label(seconds),
                    s.guidance.trim()
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n---\n\n");
        Ok(include_str!("../../../prompts/audio-template-v2.md")
            .replace(
                "{{duration}}",
                &super::audio::duration_label(duration_seconds),
            )
            .replace("{{word_count}}", &(duration_seconds * 220 / 60).to_string())
            .replace(
                "{{opening_duration}}",
                &super::audio::duration_label(opening),
            )
            .replace(
                "{{closing_duration}}",
                &super::audio::duration_label(closing),
            )
            .replace(
                "{{book}}",
                book.trim().trim_start_matches('《').trim_end_matches('》'),
            )
            .replace("{{introduction}}", self.introduction.trim())
            .replace("{{sections}}", &sections))
    }
}
impl PublishingDraft {
    pub fn into_content(self, book: &str, duration_seconds: i64) -> Result<WritingContent, String> {
        self.cover_design.validate()?;
        if [
            &self.cover_title,
            &self.cover_subtitle,
            &self.cover_prompt_landscape,
            &self.cover_prompt_portrait,
        ]
        .iter()
        .any(|value| value.trim().is_empty())
            || !self.cover_prompt_landscape.contains("4:3")
            || !self.cover_prompt_portrait.contains("3:4")
            || self.cover_prompt_landscape.trim() == self.cover_prompt_portrait.trim()
        {
            return Err(
                "封面需要主标题、副标题及分别标明 4:3、3:4 的两份不同提示词，原有内容保留。".into(),
            );
        }
        let audio_prompt = AudioDraft {
            introduction: self.introduction,
            sections: self.sections,
        }
        .into_prompt(book, duration_seconds)?;
        let content = WritingContent::Publishing {
            title: self.title.trim().into(),
            description: self.description.trim().into(),
            audio_prompt,
            cover_title: self.cover_title.trim().into(),
            cover_subtitle: self.cover_subtitle.trim().into(),
            cover_prompt_landscape: self.cover_prompt_landscape.trim().into(),
            cover_prompt_portrait: self.cover_prompt_portrait.trim().into(),
            cover_design: Some(Box::new(self.cover_design)),
        };
        content.validate("publishing", None)?;
        Ok(content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selected_duration_controls_all_template_timings_and_word_budget() {
        for minutes in [1, 5, 15, 30, 60, 1440] {
            let seconds = minutes * 60;
            let content = serde_json::from_value::<PublishingDraft>(draft())
                .unwrap()
                .into_content("测试书", seconds)
                .unwrap();
            let WritingContent::Publishing { audio_prompt, .. } = content else {
                panic!()
            };
            assert!(audio_prompt.contains(&format!("## 时长\n{minutes}分钟。")));
            assert!(audio_prompt.contains(&format!("接下来{minutes}分钟，我们一起读懂这本书。")));
            assert!(audio_prompt.contains(&format!("“{minutes}分钟读懂一本书”")));
            assert!(audio_prompt.contains(&format!("口播文字约{}字", minutes * 220)));
            assert!(!audio_prompt.contains("十分钟"));
            assert!(!audio_prompt.contains("8—10"));
            assert!(!audio_prompt.contains("{{"));
            let opening = (seconds / 3).min(60);
            let closing = (seconds / 3).min(45);
            let body = seconds - opening - closing;
            let section_total: i64 = (0..5).map(|i| body / 5 + i64::from(i < body % 5)).sum();
            assert_eq!(opening + closing + section_total, seconds);
            for i in 0..5 {
                assert!(audio_prompt.contains(&format!(
                    "主题{i}（约{}）",
                    super::super::audio::duration_label(body / 5 + i64::from(i < body % 5))
                )));
            }
        }
    }
    fn draft() -> serde_json::Value {
        serde_json::json!({"title":"一本书的故事", "description":"今天用十分钟聊这本书。", "introduction":"今天我们要聊的这本书，是《明朝那些事儿》。", "sections":(0..5).map(|i| serde_json::json!({"heading":format!("主题{i}"),"guidance":format!("按原书来源讲主题{i}，不写完整口播稿。") })).collect::<Vec<_>>(), "coverTitle":"权力背后的人间故事", "coverSubtitle":"《明朝那些事儿》里的起落", "coverPromptLandscape":"横版 4:3，城阙横向展开，中央留白，无文字。", "coverPromptPortrait":"竖版 3:4，宫门向远处延伸，中央留白，无文字。", "coverDesign":{"titleSize":148,"subtitleSize":54,"textColor":"#ffe45c","subtitleColor":"#ffffff","accentColor":"#ff7547","backgroundColor":"#141414","overlayOpacity":0.75}})
    }
    #[test]
    fn validates_design_before_accepting_ai_output() {
        let mut missing = draft();
        missing.as_object_mut().unwrap().remove("coverDesign");
        assert!(serde_json::from_value::<PublishingDraft>(missing).is_err());
        for (field, invalid) in [
            ("titleSize", serde_json::json!(161)),
            ("titleSize", serde_json::json!(140.5)),
            ("subtitleSize", serde_json::json!(23)),
            ("textColor", serde_json::json!("red")),
            ("subtitleColor", serde_json::json!("#fff")),
            ("accentColor", serde_json::json!("#gggggg")),
            ("backgroundColor", serde_json::json!("#12345678")),
            ("overlayOpacity", serde_json::json!(0.9)),
        ] {
            let mut value = draft();
            value["coverDesign"][field] = invalid;
            if let Ok(value) = serde_json::from_value::<PublishingDraft>(value) {
                assert!(value.into_content("书", 600).is_err());
            }
        }
        let content = serde_json::from_value::<PublishingDraft>(draft())
            .unwrap()
            .into_content("书", 600)
            .unwrap();
        assert_eq!(
            serde_json::to_value(content).unwrap()["coverDesign"],
            draft()["coverDesign"]
        );
    }
    #[test]
    fn template_keeps_fixed_host_duration_and_book_specific_five_parts() {
        let value: PublishingDraft = serde_json::from_value(draft()).unwrap();
        let content = value.into_content("明朝那些事儿", 600).unwrap();
        let WritingContent::Publishing {
            audio_prompt,
            cover_title,
            cover_subtitle,
            cover_prompt_landscape,
            cover_prompt_portrait,
            ..
        } = content
        else {
            panic!()
        };
        assert!(audio_prompt.contains("10分钟"));
        assert!(audio_prompt.contains("欢迎大家收听今天的节目，我是甲子。"));
        assert!(audio_prompt.contains("《明朝那些事儿》这本书"));
        assert!(audio_prompt.contains("## 第五部分：主题4"));
        assert!(audio_prompt.contains("下一期，\n我们继续一起读书。"));
        assert!(!audio_prompt.contains("{{"));
        assert_eq!(cover_title, "权力背后的人间故事");
        assert_eq!(cover_subtitle, "《明朝那些事儿》里的起落");
        assert!(cover_prompt_landscape.contains("4:3"));
        assert!(cover_prompt_portrait.contains("3:4"));
    }
    #[test]
    fn rejects_incomplete_output_and_excessive_publish_text_before_saving() {
        let mut value = draft();
        value["sections"].as_array_mut().unwrap().pop();
        assert!(serde_json::from_value::<PublishingDraft>(value)
            .unwrap()
            .into_content("书", 600)
            .is_err());
        for (field, count) in [("title", 31), ("description", 1001), ("introduction", 1501)] {
            let mut value = draft();
            value[field] = serde_json::json!("字".repeat(count));
            assert!(serde_json::from_value::<PublishingDraft>(value)
                .unwrap()
                .into_content("书", 600)
                .is_err());
        }
        let mut value = draft();
        value["sections"][0]["guidance"] = serde_json::json!("");
        assert!(serde_json::from_value::<PublishingDraft>(value)
            .unwrap()
            .into_content("书", 600)
            .is_err());
    }
    #[test]
    fn rejects_missing_empty_oversized_and_wrong_ratio_cover_fields() {
        for (field, limit) in [
            ("coverTitle", 30),
            ("coverSubtitle", 60),
            ("coverPromptLandscape", 2000),
            ("coverPromptPortrait", 2000),
        ] {
            let mut missing = draft();
            missing.as_object_mut().unwrap().remove(field);
            assert!(serde_json::from_value::<PublishingDraft>(missing).is_err());
            for text in [" ".into(), format!("4:3 3:4 {}", "字".repeat(limit))] {
                let mut invalid = draft();
                invalid[field] = serde_json::json!(text);
                assert!(serde_json::from_value::<PublishingDraft>(invalid)
                    .unwrap()
                    .into_content("书", 600)
                    .is_err());
            }
        }
        let mut swapped = draft();
        swapped["coverPromptPortrait"] = swapped["coverPromptLandscape"].clone();
        assert!(serde_json::from_value::<PublishingDraft>(swapped)
            .unwrap()
            .into_content("书", 600)
            .is_err());
        let mut same = draft();
        same["coverPromptLandscape"] = serde_json::json!("4:3 和 3:4 相同画面");
        same["coverPromptPortrait"] = same["coverPromptLandscape"].clone();
        assert!(serde_json::from_value::<PublishingDraft>(same)
            .unwrap()
            .into_content("书", 600)
            .is_err());
    }
}
