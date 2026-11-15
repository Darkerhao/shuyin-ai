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
    hook: String,
    introduction: String,
    sections: Vec<Section>,
    closing: String,
    cover_title: String,
    cover_subtitle: String,
    cover_prompt_landscape: String,
    cover_prompt_portrait: String,
    cover_prompt_widescreen: String,
    cover_design: CoverDesign,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Section {
    pub heading: String,
    pub guidance: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AudioDraft {
    pub hook: String,
    pub introduction: String,
    pub sections: Vec<Section>,
    pub closing: String,
}
impl AudioDraft {
    pub fn into_prompt(self, book: &str, chapter: Option<&str>) -> Result<String, String> {
        if (chapter.is_none() && self.sections.len() != 5)
            || !(1..=5).contains(&self.sections.len())
            || [
                (&self.hook, 200),
                (&self.introduction, 1500),
                (&self.closing, 600),
            ]
            .iter()
            .any(|(text, limit)| text.trim().is_empty() || text.chars().count() > *limit)
            || self.sections.iter().any(|s| {
                s.heading.trim().is_empty()
                    || s.heading.chars().count() > 100
                    || s.guidance.trim().is_empty()
                    || s.guidance.chars().count() > 2000
            })
        {
            return Err(
                "音频提示词需要完整引子、本期看点和结尾收束；整书为五部分，章节为1—5部分，原有内容保留。".into(),
            );
        }
        Ok(self.render(book, chapter))
    }

    // Partial streaming previews use this renderer; only into_prompt accepts a final draft.
    pub(super) fn render(&self, book: &str, chapter: Option<&str>) -> String {
        let sections = self
            .sections
            .iter()
            .zip(["一", "二", "三", "四", "五"])
            .map(|(s, number)| {
                format!(
                    "## 第{number}部分：{}\n\n{}",
                    s.heading.trim(),
                    s.guidance.trim()
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n---\n\n");
        let scope = chapter.map_or_else(String::new, |chapter| format!("本期精讲《{book}》的“{chapter}”。仅以本次上传的章节正文为来源，细致解释重要内容、关键概念、人物动机或论证过程，不只概括要点，不推测其他章节、后续情节或全书结论。"));
        let introduction = chapter.map_or_else(
            || format!("这一期，我们一起读《{book}》。"),
            |chapter| format!("这一期，我们一起读《{book}》中的“{chapter}”。"),
        );
        include_str!("../../../prompts/audio-template-v7.md")
            .replace("{{scope_instruction}}", &scope)
            .replace("{{hook}}", self.hook.trim())
            .replace(
                "{{introduction}}",
                &format!("{introduction}\n{}", self.introduction.trim()),
            )
            .replace("{{sections}}", &sections)
            .replace("{{closing}}", self.closing.trim())
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
    pub fn into_content(self, book: &str, chapter: Option<&str>) -> Result<WritingContent, String> {
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
            hook: self.hook,
            introduction: self.introduction,
            sections: self.sections,
            closing: self.closing,
        }
        .into_prompt(book, chapter)?;
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
        for field in ["hook", "introduction", "sections", "closing"] {
            value.as_object_mut().unwrap().remove(field);
        }
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
            .into_content("明朝那些事儿", None)
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
                assert!(value.into_content("书", None).is_err());
            }
        }
    }
    #[test]
    fn audio_guidance_has_no_duration_budget_and_keeps_fixed_spoken_segments() {
        let content = serde_json::from_value::<PublishingDraft>(draft())
            .unwrap()
            .into_content("测试书", None)
            .unwrap();
        let WritingContent::Publishing { audio_prompt, .. } = content else {
            panic!()
        };
        let opening = audio_prompt
            .split_once("## 固定开场（逐字朗读）\n\n")
            .unwrap()
            .1
            .split_once("\n\n## 开场介绍")
            .unwrap()
            .0;
        assert_eq!(opening, "欢迎大家收听今天的节目，我是甲子。");
        let closing = audio_prompt
            .split_once("## 固定结尾（逐字朗读）\n\n")
            .unwrap()
            .1
            .trim();
        assert_eq!(
            closing,
            "我是甲子，陪你读懂书里的故事，也读懂故事里的人。喜欢这期分享，记得关注，我们下期见。"
        );
        for removed in ["目标时长：", "口播文字约", "（约", "时长预算不足", "{{"]
        {
            assert!(!audio_prompt.contains(removed), "{removed}");
        }
        assert!(audio_prompt.contains("不设目标时长、口播字数或分段时间预算"));
        assert_eq!(audio_prompt.matches("## 第").count(), 5);
    }
    #[test]
    fn audio_introduction_announces_the_book_and_any_saved_chapter_title() {
        for (chapter, expected) in [
            (None, "这一期，我们一起读《测试书》。"),
            (
                Some("第五回：发矫诏诸镇应曹公破关兵三英战吕布"),
                "这一期，我们一起读《测试书》中的“第五回：发矫诏诸镇应曹公破关兵三英战吕布”。",
            ),
            (
                Some("第三章：习惯的形成"),
                "这一期，我们一起读《测试书》中的“第三章：习惯的形成”。",
            ),
            (Some("序言"), "这一期，我们一起读《测试书》中的“序言”。"),
            (Some("论求知"), "这一期，我们一起读《测试书》中的“论求知”。"),
            (
                Some("上篇·第二节"),
                "这一期，我们一起读《测试书》中的“上篇·第二节”。",
            ),
            (
                Some("Chapter IV — A New Beginning"),
                "这一期，我们一起读《测试书》中的“Chapter IV — A New Beginning”。",
            ),
        ] {
            let mut value = draft();
            value["introduction"] =
                serde_json::json!("从关键选择出发，看看人物如何面对眼前的困境。");
            let content = serde_json::from_value::<PublishingDraft>(value)
                .unwrap()
                .into_content("测试书", chapter)
                .unwrap();
            let WritingContent::Publishing { audio_prompt, .. } = content else {
                panic!()
            };
            let introduction = audio_prompt
                .split_once("## 开场介绍（逐字朗读）\n\n")
                .expect("本期介绍必须被列为定稿口播")
                .1
                .split_once("\n\n---")
                .unwrap()
                .0;
            assert_eq!(
                introduction,
                format!("{expected}\n从关键选择出发，看看人物如何面对眼前的困境。")
            );
            assert_eq!(introduction.matches(expected).count(), 1);
            let welcome = audio_prompt
                .find("欢迎大家收听今天的节目，我是甲子。")
                .unwrap();
            let announcement = audio_prompt.find(expected).unwrap();
            assert!(
                welcome < announcement && announcement < audio_prompt.find("## 第一部分").unwrap()
            );
            let reference = crate::video::subtitles::SubtitleReference::from_prompt(
                "测试书".into(),
                String::new(),
                &audio_prompt,
            );
            assert!(reference.opening.contains(introduction));
        }
    }
    #[test]
    fn audio_bookends_surround_the_program_and_are_available_to_subtitle_review() {
        let value = draft();
        let hook = value["hook"].as_str().unwrap();
        let closing = value["closing"].as_str().unwrap();
        let content = serde_json::from_value::<PublishingDraft>(value.clone())
            .unwrap()
            .into_content("明朝那些事儿", None)
            .unwrap();
        let WritingContent::Publishing { audio_prompt, .. } = content else {
            panic!()
        };
        let ordered = [
            hook,
            "欢迎大家收听今天的节目，我是甲子。",
            value["introduction"].as_str().unwrap(),
            "## 第一部分：主题0",
            "## 第五部分：主题4",
            closing,
            "我是甲子，陪你读懂书里的故事，也读懂故事里的人。",
        ];
        let positions = ordered.map(|text| audio_prompt.find(text).unwrap());
        assert!(positions.windows(2).all(|pair| pair[0] < pair[1]));
        let reference = crate::video::subtitles::SubtitleReference::from_prompt(
            "明朝那些事儿".into(),
            String::new(),
            &audio_prompt,
        );
        assert_eq!(
            reference.opening,
            format!("{hook}\n欢迎大家收听今天的节目，我是甲子。\n这一期，我们一起读《明朝那些事儿》。\n{}", value["introduction"].as_str().unwrap())
        );
        assert_eq!(reference.ending, format!("{closing}\n我是甲子，陪你读懂书里的故事，也读懂故事里的人。喜欢这期分享，记得关注，我们下期见。"));
        assert!(!reference.opening.contains("按原书来源"));
        assert!(!reference.ending.contains("按原书来源"));
    }
    #[test]
    fn rejects_missing_empty_and_oversized_audio_bookends() {
        for (field, limit) in [("hook", 200), ("closing", 600)] {
            let mut missing = draft();
            missing.as_object_mut().unwrap().remove(field);
            assert!(serde_json::from_value::<PublishingDraft>(missing).is_err());
            for text in [" ".into(), "字".repeat(limit + 1)] {
                let mut invalid = draft();
                invalid[field] = serde_json::json!(text);
                assert!(serde_json::from_value::<PublishingDraft>(invalid)
                    .unwrap()
                    .into_content("书", None)
                    .is_err());
            }
        }
    }
    fn draft() -> serde_json::Value {
        serde_json::json!({"title":"一本书的故事", "description":"今天用十分钟聊这本书。", "tags":"#读书 #明朝那些事儿 #历史", "hook":"一个人的选择，怎样改变一个时代的走向？", "introduction":"从关键选择出发，看看人物如何面对眼前的困境。", "sections":(0..5).map(|i| serde_json::json!({"heading":format!("主题{i}"),"guidance":format!("按原书来源讲主题{i}，不写完整口播稿。") })).collect::<Vec<_>>(), "closing":"回到开头的问题，个人选择与时代处境相互影响。读懂人物怎样作出决定，才能看清故事中的因果。", "coverTitle":"权力背后的人间故事", "coverSubtitle":"《明朝那些事儿》里的起落", "coverPromptLandscape":"横版 4:3，城阙横向展开，中央留白，无文字。", "coverPromptPortrait":"竖版 3:4，宫门向远处延伸，中央留白，无文字。", "coverPromptWidescreen":"横版 16:9，城墙横向绵延，远景开阔，中央留白，无文字。", "coverDesign":{"titleSize":148,"subtitleSize":54,"textColor":"#ffe45c","subtitleColor":"#ffffff","accentColor":"#ff7547","backgroundColor":"#141414","overlayOpacity":0.75}})
    }
    #[test]
    fn chapter_audio_accepts_one_to_five_detailed_sections_without_book_wide_opening() {
        for count in 0..=6 {
            let mut value = draft();
            value["sections"] = serde_json::json!((0..count).map(|i| serde_json::json!({"heading":format!("本章论点{i}"),"guidance":"细致解释本章的论证、关键概念及重要细节。"})).collect::<Vec<_>>());
            let parsed: PublishingDraft = serde_json::from_value(value.clone()).unwrap();
            let result = parsed.into_content("测试书", Some("第一章"));
            if !(1..=5).contains(&count) {
                assert!(result.is_err());
                continue;
            }
            let WritingContent::Publishing { audio_prompt, .. } = result.unwrap() else {
                panic!()
            };
            assert!(audio_prompt.contains("本期精讲《测试书》的“第一章”"));
            assert!(audio_prompt.contains("本期精讲"));
            assert!(!audio_prompt.contains("接下来，我们一起读懂这本书"));
            assert!(!audio_prompt.contains("{{"));
            assert_eq!(audio_prompt.matches("## 第").count(), count);
            if count != 5 {
                assert!(serde_json::from_value::<PublishingDraft>(value)
                    .unwrap()
                    .into_content("测试书", None)
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
                assert!(value.into_content("书", None).is_err());
            }
        }
        let content = serde_json::from_value::<PublishingDraft>(draft())
            .unwrap()
            .into_content("书", None)
            .unwrap();
        assert_eq!(
            serde_json::to_value(content).unwrap()["coverDesign"],
            draft()["coverDesign"]
        );
    }
    #[test]
    fn template_keeps_fixed_host_and_book_specific_five_parts() {
        let value: PublishingDraft = serde_json::from_value(draft()).unwrap();
        let content = value.into_content("明朝那些事儿", None).unwrap();
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
        assert!(!audio_prompt.contains("10分钟"));
        assert!(audio_prompt.contains("欢迎大家收听今天的节目，我是甲子。"));
        assert!(audio_prompt.contains("《明朝那些事儿》"));
        assert!(audio_prompt.contains("## 第五部分：主题4"));
        assert!(audio_prompt.contains("喜欢这期分享，记得关注，我们下期见。"));
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
            .into_content("书", None)
            .is_err());
        for (field, count) in [("title", 31), ("description", 1001), ("introduction", 1501)] {
            let mut value = draft();
            value[field] = serde_json::json!("字".repeat(count));
            assert!(serde_json::from_value::<PublishingDraft>(value)
                .unwrap()
                .into_content("书", None)
                .is_err());
        }
        let mut value = draft();
        value["sections"][0]["guidance"] = serde_json::json!("");
        assert!(serde_json::from_value::<PublishingDraft>(value)
            .unwrap()
            .into_content("书", None)
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
                    .into_content("书", None)
                    .is_err());
            }
        }
        let mut swapped = draft();
        swapped["coverPromptPortrait"] = swapped["coverPromptLandscape"].clone();
        assert!(serde_json::from_value::<PublishingDraft>(swapped)
            .unwrap()
            .into_content("书", None)
            .is_err());
        let mut same = draft();
        same["coverPromptLandscape"] = serde_json::json!("4:3 和 3:4 相同画面");
        same["coverPromptPortrait"] = same["coverPromptLandscape"].clone();
        assert!(serde_json::from_value::<PublishingDraft>(same)
            .unwrap()
            .into_content("书", None)
            .is_err());
    }
}
