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
    tags: String,
    introduction: String,
    sections: Vec<Section>,
    cover_title: String,
    cover_subtitle: String,
    cover_prompt_landscape: String,
    cover_prompt_portrait: String,
    cover_prompt_widescreen: String,
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
    pub fn into_prompt(
        self,
        book: &str,
        duration_seconds: i64,
        chapter: Option<&str>,
    ) -> Result<String, String> {
        super::audio::validate_duration(duration_seconds)?;
        if (chapter.is_none() && self.sections.len() != 5)
            || !(1..=5).contains(&self.sections.len())
            || self.introduction.trim().is_empty()
            || self.introduction.chars().count() > 1500
            || self.sections.iter().any(|s| {
                s.heading.trim().is_empty()
                    || s.heading.chars().count() > 100
                    || s.guidance.trim().is_empty()
                    || s.guidance.chars().count() > 2000
            })
        {
            return Err(
                "音频提示词需要完整引入；整书为五部分，章节为1—5部分，原有内容保留。".into(),
            );
        }
        let opening = (duration_seconds / 3).min(60);
        let closing = (duration_seconds / 3).min(45);
        let body = duration_seconds - opening - closing;
        let section_count = self.sections.len() as i64;
        let sections = self
            .sections
            .iter()
            .zip(["一", "二", "三", "四", "五"])
            .enumerate()
            .map(|(index, (s, number))| {
                let seconds =
                    body / section_count + i64::from((index as i64) < body % section_count);
                format!(
                    "## 第{number}部分：{}（约{}）\n\n{}",
                    s.heading.trim(),
                    super::audio::duration_label(seconds),
                    s.guidance.trim()
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n---\n\n");
        let fixed_opening = chapter.map_or_else(
            || "欢迎大家收听今天的节目，我是甲子。接下来，我们一起读懂这本书。".to_string(),
            |chapter| {
                format!(
                    "欢迎大家收听今天的节目，我是甲子。今天，我们来读《{}》中的“{}”这一章。",
                    book.trim().trim_start_matches('《').trim_end_matches('》'),
                    chapter
                )
            },
        );
        let scope = chapter.map_or_else(String::new, |chapter| format!("本期精讲《{book}》的“{chapter}”。仅以本次上传的章节正文为来源，细致解释重要内容、关键概念、人物动机或论证过程，不只概括要点，不推测其他章节、后续情节或全书结论。"));
        Ok(include_str!("../../../prompts/audio-template-v4.md")
            .replace("{{opening}}", &fixed_opening)
            .replace("{{scope_instruction}}", &scope)
            .replace("{{section_count}}", &section_count.to_string())
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
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TranscriptDraft {
    title: String,
    description: String,
    tags: String,
    cover_title: String,
    cover_subtitle: String,
    cover_prompt_landscape: String,
    cover_prompt_portrait: String,
    cover_prompt_widescreen: String,
    cover_design: CoverDesign,
}

impl TranscriptDraft {
    pub fn into_content(self, audio_prompt: String) -> Result<WritingContent, String> {
        let tags = parse_tags(&self.tags)?;
        self.cover_design.validate()?;
        validate_cover_prompts(
            &self.cover_title,
            &self.cover_subtitle,
            &self.cover_prompt_landscape,
            &self.cover_prompt_portrait,
            &self.cover_prompt_widescreen,
        )?;
        if self.title.chars().count() + self.description.chars().count() + tags.chars().count()
            > 1000
            || [
                &self.title,
                &self.description,
                &self.cover_title,
                &self.cover_subtitle,
            ]
            .iter()
            .any(|text| {
                ["{{", "}}", "[书名]", "[作者]", "待填写"]
                    .iter()
                    .any(|value| text.contains(value))
            })
        {
            return Err("发布文案超过平台长度或含未完成占位内容，已保存成果保留。".into());
        }
        let content = WritingContent::Publishing {
            title: self.title.trim().into(),
            description: self.description.trim().into(),
            tags,
            audio_prompt,
            cover_title: self.cover_title.trim().into(),
            cover_subtitle: self.cover_subtitle.trim().into(),
            cover_prompt_landscape: self.cover_prompt_landscape.trim().into(),
            cover_prompt_portrait: self.cover_prompt_portrait.trim().into(),
            cover_prompt_widescreen: self.cover_prompt_widescreen.trim().into(),
            cover_design: Some(Box::new(self.cover_design)),
        };
        content.validate("publishing", None)?;
        Ok(content)
    }
}

fn validate_cover_prompts(
    title: &str,
    subtitle: &str,
    landscape: &str,
    portrait: &str,
    widescreen: &str,
) -> Result<(), String> {
    if [title, subtitle, landscape, portrait, widescreen]
        .iter()
        .any(|value| value.trim().is_empty())
        || !landscape.contains("4:3")
        || !portrait.contains("3:4")
        || !widescreen.contains("16:9")
        || landscape.trim() == portrait.trim()
        || widescreen.trim() == landscape.trim()
        || widescreen.trim() == portrait.trim()
    {
        return Err(
            "封面需要主标题、副标题及分别标明 4:3、3:4、16:9 的三份不同提示词，原有内容保留。"
                .into(),
        );
    }
    Ok(())
}
impl PublishingDraft {
    pub fn into_content(
        self,
        book: &str,
        duration_seconds: i64,
        chapter: Option<&str>,
    ) -> Result<WritingContent, String> {
        let tags = parse_tags(&self.tags)?;
        self.cover_design.validate()?;
        validate_cover_prompts(
            &self.cover_title,
            &self.cover_subtitle,
            &self.cover_prompt_landscape,
            &self.cover_prompt_portrait,
            &self.cover_prompt_widescreen,
        )?;
        let audio_prompt = AudioDraft {
            introduction: self.introduction,
            sections: self.sections,
        }
        .into_prompt(book, duration_seconds, chapter)?;
        let content = WritingContent::Publishing {
            title: self.title.trim().into(),
            description: self.description.trim().into(),
            tags,
            audio_prompt,
            cover_title: self.cover_title.trim().into(),
            cover_subtitle: self.cover_subtitle.trim().into(),
            cover_prompt_landscape: self.cover_prompt_landscape.trim().into(),
            cover_prompt_portrait: self.cover_prompt_portrait.trim().into(),
            cover_prompt_widescreen: self.cover_prompt_widescreen.trim().into(),
            cover_design: Some(Box::new(self.cover_design)),
        };
        content.validate("publishing", None)?;
        Ok(content)
    }
}

pub(super) fn parse_tags(raw: &str) -> Result<String, String> {
    let tags = raw.split_whitespace().collect::<Vec<_>>();
    if tags.is_empty()
        || tags.iter().any(|tag| {
            tag.strip_prefix('#').is_none_or(|name| {
                name.is_empty()
                    || !name
                        .chars()
                        .all(|character| character.is_alphanumeric() || character == '_')
            })
        })
    {
        return Err("标签须使用空格分隔的 #标签 格式，原有内容保留。".into());
    }
    Ok(tags.join(" "))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn transcript_output_keeps_actual_audio_guidance_and_rejects_unasked_fields() {
        let mut value = draft();
        value.as_object_mut().unwrap().remove("introduction");
        value.as_object_mut().unwrap().remove("sections");
        let content = serde_json::from_value::<TranscriptDraft>(value.clone())
            .unwrap()
            .into_content("实际音频使用的旧版指导".into())
            .unwrap();
        let WritingContent::Publishing { audio_prompt, .. } = content else {
            panic!("expected publishing content");
        };
        assert_eq!(audio_prompt, "实际音频使用的旧版指导");
        value["audioPrompt"] = serde_json::json!("模型不得替换指导");
        assert!(serde_json::from_value::<TranscriptDraft>(value).is_err());
    }
    #[test]
    fn generates_valid_hashtags_without_putting_them_in_the_description() {
        let mut value = draft();
        value["tags"] = serde_json::json!("  #读书  #明朝那些事儿\n#历史_人物 #Book2026  ");
        let content = serde_json::from_value::<PublishingDraft>(value.clone())
            .unwrap()
            .into_content("明朝那些事儿", 600, None)
            .unwrap();
        let content = serde_json::to_value(content).unwrap();
        assert_eq!(content["tags"], "#读书 #明朝那些事儿 #历史_人物 #Book2026");
        assert_eq!(content["description"], value["description"]);
    }
    #[test]
    fn rejects_missing_empty_malformed_and_oversized_generated_hashtags() {
        let mut missing = draft();
        missing.as_object_mut().unwrap().remove("tags");
        assert!(serde_json::from_value::<PublishingDraft>(missing).is_err());
        for tags in [
            " ".into(),
            "#".into(),
            "读书 #好书分享".into(),
            "#读书#历史".into(),
            "#读书， #历史".into(),
            "#读书 @作者".into(),
            format!("#{}", "字".repeat(500)),
        ] {
            let mut invalid = draft();
            invalid["tags"] = serde_json::json!(tags);
            if let Ok(value) = serde_json::from_value::<PublishingDraft>(invalid) {
                assert!(value.into_content("书", 600, None).is_err());
            }
        }
    }
    #[test]
    fn selected_duration_controls_planning_without_spoken_time_promises() {
        for minutes in [1, 5, 15, 18, 30, 60, 1440] {
            let seconds = minutes * 60;
            let content = serde_json::from_value::<PublishingDraft>(draft())
                .unwrap()
                .into_content("测试书", seconds, None)
                .unwrap();
            let WritingContent::Publishing { audio_prompt, .. } = content else {
                panic!()
            };
            assert!(audio_prompt.contains(&format!("目标时长：{minutes}分钟")));
            let opening = audio_prompt
                .split_once("## 固定开场（逐字朗读）\n\n")
                .unwrap()
                .1
                .split_once("\n\n## 书籍引入")
                .unwrap()
                .0;
            assert_eq!(
                opening,
                "欢迎大家收听今天的节目，我是甲子。接下来，我们一起读懂这本书。"
            );
            let closing = audio_prompt
                .split_once("## 固定结尾（逐字朗读）\n\n")
                .unwrap()
                .1
                .trim();
            assert_eq!(closing, "如果你对今天的内容感兴趣，建议读一读《测试书》这本书。\n我是甲子。如果你喜欢这样的读书分享，记得关注我。下一期，我们继续一起读书。");
            assert!(!audio_prompt.contains(&format!("接下来{minutes}分钟")));
            assert!(!audio_prompt.contains(&format!("{minutes}分钟读懂一本书")));
            assert!(audio_prompt.contains("第一句必须是固定开场的第一句"));
            assert!(audio_prompt.contains("固定结尾之后立即结束"));
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
        serde_json::json!({"title":"一本书的故事", "description":"今天用十分钟聊这本书。", "tags":"#读书 #明朝那些事儿 #历史", "introduction":"今天我们要聊的这本书，是《明朝那些事儿》。", "sections":(0..5).map(|i| serde_json::json!({"heading":format!("主题{i}"),"guidance":format!("按原书来源讲主题{i}，不写完整口播稿。") })).collect::<Vec<_>>(), "coverTitle":"权力背后的人间故事", "coverSubtitle":"《明朝那些事儿》里的起落", "coverPromptLandscape":"横版 4:3，城阙横向展开，中央留白，无文字。", "coverPromptPortrait":"竖版 3:4，宫门向远处延伸，中央留白，无文字。", "coverPromptWidescreen":"横版 16:9，城墙横向绵延，远景开阔，中央留白，无文字。", "coverDesign":{"titleSize":148,"subtitleSize":54,"textColor":"#ffe45c","subtitleColor":"#ffffff","accentColor":"#ff7547","backgroundColor":"#141414","overlayOpacity":0.75}})
    }
    #[test]
    fn chapter_audio_accepts_one_to_five_detailed_sections_without_book_wide_opening() {
        for count in 0..=6 {
            let mut value = draft();
            value["sections"] = serde_json::json!((0..count).map(|i| serde_json::json!({"heading":format!("本章论点{i}"),"guidance":"细致解释本章的论证、关键概念及重要细节。"})).collect::<Vec<_>>());
            let parsed: PublishingDraft = serde_json::from_value(value.clone()).unwrap();
            let result = parsed.into_content("测试书", 300, Some("第一章"));
            if !(1..=5).contains(&count) {
                assert!(result.is_err());
                continue;
            }
            let WritingContent::Publishing { audio_prompt, .. } = result.unwrap() else {
                panic!()
            };
            assert!(audio_prompt.contains("《测试书》中的“第一章”这一章"));
            assert!(audio_prompt.contains("本期精讲"));
            assert!(!audio_prompt.contains("接下来，我们一起读懂这本书"));
            assert!(!audio_prompt.contains("{{"));
            assert_eq!(audio_prompt.matches("## 第").count(), count);
            if count != 5 {
                assert!(serde_json::from_value::<PublishingDraft>(value)
                    .unwrap()
                    .into_content("测试书", 300, None)
                    .is_err());
            }
        }
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
                assert!(value.into_content("书", 600, None).is_err());
            }
        }
        let content = serde_json::from_value::<PublishingDraft>(draft())
            .unwrap()
            .into_content("书", 600, None)
            .unwrap();
        assert_eq!(
            serde_json::to_value(content).unwrap()["coverDesign"],
            draft()["coverDesign"]
        );
    }
    #[test]
    fn template_keeps_fixed_host_duration_and_book_specific_five_parts() {
        let value: PublishingDraft = serde_json::from_value(draft()).unwrap();
        let content = value.into_content("明朝那些事儿", 600, None).unwrap();
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
        assert!(audio_prompt.contains("下一期，我们继续一起读书。"));
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
            .into_content("书", 600, None)
            .is_err());
        for (field, count) in [("title", 31), ("description", 1001), ("introduction", 1501)] {
            let mut value = draft();
            value[field] = serde_json::json!("字".repeat(count));
            assert!(serde_json::from_value::<PublishingDraft>(value)
                .unwrap()
                .into_content("书", 600, None)
                .is_err());
        }
        let mut value = draft();
        value["sections"][0]["guidance"] = serde_json::json!("");
        assert!(serde_json::from_value::<PublishingDraft>(value)
            .unwrap()
            .into_content("书", 600, None)
            .is_err());
    }
    #[test]
    fn rejects_missing_empty_oversized_and_wrong_ratio_cover_fields() {
        for (field, limit) in [
            ("coverTitle", 30),
            ("coverSubtitle", 60),
            ("coverPromptLandscape", 2000),
            ("coverPromptPortrait", 2000),
            ("coverPromptWidescreen", 2000),
        ] {
            let mut missing = draft();
            missing.as_object_mut().unwrap().remove(field);
            assert!(serde_json::from_value::<PublishingDraft>(missing).is_err());
            for text in [" ".into(), format!("4:3 3:4 16:9 {}", "字".repeat(limit))] {
                let mut invalid = draft();
                invalid[field] = serde_json::json!(text);
                assert!(serde_json::from_value::<PublishingDraft>(invalid)
                    .unwrap()
                    .into_content("书", 600, None)
                    .is_err());
            }
        }
        let mut swapped = draft();
        swapped["coverPromptPortrait"] = swapped["coverPromptLandscape"].clone();
        assert!(serde_json::from_value::<PublishingDraft>(swapped)
            .unwrap()
            .into_content("书", 600, None)
            .is_err());
        let mut same = draft();
        same["coverPromptLandscape"] = serde_json::json!("4:3 和 3:4 相同画面");
        same["coverPromptPortrait"] = same["coverPromptLandscape"].clone();
        assert!(serde_json::from_value::<PublishingDraft>(same)
            .unwrap()
            .into_content("书", 600, None)
            .is_err());
    }
}
