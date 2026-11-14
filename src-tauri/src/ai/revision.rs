use super::{
    publishing::{parse_tags, AudioDraft, CoverDesign},
    WritingContent,
};
use serde::{Deserialize, Serialize};

pub const PROMPT_VERSION: &str = "publishing-revision-v8";
pub const SYSTEM_PROMPT: &str = include_str!("../../../prompts/publishing-revision-v8.md");

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PublishingField {
    Title,
    Description,
    Tags,
    AudioPrompt,
    CoverTitle,
    CoverSubtitle,
    CoverPromptLandscape,
    CoverPromptPortrait,
    CoverPromptWidescreen,
    CoverDesign,
}
impl PublishingField {
    pub(crate) fn key(self) -> &'static str {
        match self {
            Self::Title => "title",
            Self::Description => "description",
            Self::Tags => "tags",
            Self::AudioPrompt => "audioPrompt",
            Self::CoverTitle => "coverTitle",
            Self::CoverSubtitle => "coverSubtitle",
            Self::CoverPromptLandscape => "coverPromptLandscape",
            Self::CoverPromptPortrait => "coverPromptPortrait",
            Self::CoverPromptWidescreen => "coverPromptWidescreen",
            Self::CoverDesign => "coverDesign",
        }
    }
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RevisionInput {
    pub based_on: String,
    pub fields: Vec<PublishingField>,
    pub instruction: String,
}
impl RevisionInput {
    pub fn validate(&self) -> Result<(), String> {
        if self.fields.is_empty()
            || self
                .fields
                .iter()
                .enumerate()
                .any(|(i, field)| self.fields[..i].contains(field))
            || self.instruction.chars().count() > 2000
        {
            return Err("请选择不重复的调整项，修改要求不能超过 2000 字。".into());
        }
        Ok(())
    }
}

// Model output is a patch: fields outside the explicit selection cannot be changed.
pub fn revise(
    raw: &str,
    base: &WritingContent,
    fields: &[PublishingField],
    book: &str,
    chapter: Option<&str>,
) -> Result<WritingContent, String> {
    let patch: serde_json::Map<String, serde_json::Value> =
        serde_json::from_str(raw).map_err(|_| "模型输出不符合调整内容结构，原有内容保留。")?;
    if patch.len() != fields.len() || fields.iter().any(|field| !patch.contains_key(field.key())) {
        return Err("模型必须且只能返回选中的调整项，原有内容保留。".into());
    }
    let mut content = serde_json::to_value(base).map_err(|e| e.to_string())?;
    for field in fields {
        let value = &patch[field.key()];
        if *field == PublishingField::CoverDesign {
            let design: CoverDesign = serde_json::from_value(value.clone())
                .map_err(|_| "封面排版推荐结构无效，原有内容保留。")?;
            design.validate()?;
            content[field.key()] = serde_json::to_value(design).map_err(|e| e.to_string())?;
            continue;
        }
        let text = if *field == PublishingField::AudioPrompt {
            serde_json::from_value::<AudioDraft>(value.clone())
                .map_err(|_| "音频调整需要书籍引入及五个完整部分。")?
                .into_prompt(book, chapter)?
        } else if *field == PublishingField::Tags {
            parse_tags(value.as_str().ok_or("标签调整结果必须是文字。")?)?
        } else {
            let text = value.as_str().ok_or("调整结果必须是文字。")?.trim();
            if text.is_empty() {
                return Err("调整结果不能为空，原有内容保留。".into());
            }
            if (*field == PublishingField::CoverPromptLandscape && !text.contains("4:3"))
                || (*field == PublishingField::CoverPromptPortrait && !text.contains("3:4"))
                || (*field == PublishingField::CoverPromptWidescreen && !text.contains("16:9"))
            {
                return Err("封面提示词必须标明对应比例，原有内容保留。".into());
            }
            text.to_string()
        };
        content[field.key()] = serde_json::Value::String(text);
    }
    let prompts = [
        PublishingField::CoverPromptLandscape,
        PublishingField::CoverPromptPortrait,
        PublishingField::CoverPromptWidescreen,
    ];
    for field in fields.iter().filter(|field| prompts.contains(field)) {
        if prompts
            .iter()
            .any(|other| other != field && content[field.key()] == content[other.key()])
        {
            return Err("各尺寸提示词需要分别构图，原有内容保留。".into());
        }
    }
    let content: WritingContent = serde_json::from_value(content).map_err(|e| e.to_string())?;
    content.validate("publishing", None)?;
    Ok(content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn generates_tags_for_legacy_content_and_preserves_unselected_fields() {
        let base = original();
        let updated = revise(
            &json!({"tags":" #读书  #测试书\n#好书分享 "}).to_string(),
            &base,
            &[PublishingField::Tags],
            "测试书",
            None,
        )
        .unwrap();
        let mut expected = serde_json::to_value(&base).unwrap();
        assert_eq!(expected["tags"], "");
        expected["tags"] = json!("#读书 #测试书 #好书分享");
        assert_eq!(serde_json::to_value(updated).unwrap(), expected);
        for invalid in [
            json!({"tags":"#读书", "description":"擅自修改内容"}),
            json!({"tags":"读书"}),
            json!({"tags":""}),
            json!({"tags":["#读书"]}),
            json!({"tags":format!("#{}", "字".repeat(500))}),
        ] {
            assert!(revise(
                &invalid.to_string(),
                &base,
                &[PublishingField::Tags],
                "测试书",
                None
            )
            .is_err());
        }
    }

    #[test]
    fn widescreen_prompt_can_be_added_to_old_content_without_changing_other_fields() {
        let base = original();
        let text = "横版 16:9，宽屏城墙两侧延伸，中央留白，无字背景。";
        let updated = revise(
            &json!({"coverPromptWidescreen":text}).to_string(),
            &base,
            &[PublishingField::CoverPromptWidescreen],
            "测试书",
            None,
        )
        .unwrap();
        let mut expected = serde_json::to_value(&base).unwrap();
        expected["coverPromptWidescreen"] = json!(text);
        assert_eq!(serde_json::to_value(updated).unwrap(), expected);
        for invalid in [
            "横版 4:3 错误比例".to_string(),
            format!("16:9 {}", "字".repeat(2000)),
        ] {
            assert!(revise(
                &json!({"coverPromptWidescreen":invalid}).to_string(),
                &base,
                &[PublishingField::CoverPromptWidescreen],
                "测试书",
                None
            )
            .is_err());
        }
    }

    fn original() -> WritingContent {
        serde_json::from_value(json!({"format":"publishing","title":"原标题","description":"原内容\n保留换行","audioPrompt":"原音频提示词","coverTitle":"原封面标题","coverSubtitle":"原封面副标题","coverPromptLandscape":"横版 4:3 原画面","coverPromptPortrait":"竖版 3:4 原画面"})).unwrap()
    }

    #[test]
    fn design_revision_adds_recommendation_to_legacy_content_and_preserves_all_text() {
        let base = original();
        let design = serde_json::json!({"titleSize":148,"subtitleSize":54,"textColor":"#ffe45c","subtitleColor":"#ffffff","accentColor":"#ff7547","backgroundColor":"#141414","overlayOpacity":0.75});
        let raw = serde_json::json!({"coverDesign":design});
        let revised = revise(
            &raw.to_string(),
            &base,
            &[PublishingField::CoverDesign],
            "书",
            None,
        )
        .unwrap();
        let mut expected = serde_json::to_value(&base).unwrap();
        expected["coverDesign"] = design;
        assert_eq!(serde_json::to_value(&revised).unwrap(), expected);
        let text_only = revise(
            r#"{"title":"新标题"}"#,
            &revised,
            &[PublishingField::Title],
            "书",
            None,
        )
        .unwrap();
        expected["title"] = serde_json::json!("新标题");
        assert_eq!(serde_json::to_value(text_only).unwrap(), expected);
        for invalid in [
            serde_json::json!("文字"),
            serde_json::json!({"titleSize":148}),
            serde_json::json!(null),
        ] {
            assert!(revise(
                &serde_json::json!({"coverDesign":invalid}).to_string(),
                &base,
                &[PublishingField::CoverDesign],
                "书",
                None
            )
            .is_err());
        }
    }
    #[test]
    fn revises_only_requested_fields_and_rejects_unrequested_changes() {
        let base = original();
        let revised = revise(
            r#"{"title":"新标题"}"#,
            &base,
            &[PublishingField::Title],
            "测试书",
            None,
        )
        .unwrap();
        let mut expected = serde_json::to_value(&base).unwrap();
        expected["title"] = json!("新标题");
        assert_eq!(serde_json::to_value(revised).unwrap(), expected);
        for invalid in [
            r#"{"title":"新标题","description":"偷偷改内容"}"#,
            "{}",
            r#"{"title":" "}"#,
        ] {
            assert!(revise(invalid, &base, &[PublishingField::Title], "测试书", None).is_err());
        }
        assert!(revise(
            &json!({"title":"字".repeat(31)}).to_string(),
            &base,
            &[PublishingField::Title],
            "测试书",
            None
        )
        .is_err());
        assert!(revise(
            r#"{"coverPromptPortrait":"横版 4:3 错误比例"}"#,
            &base,
            &[PublishingField::CoverPromptPortrait],
            "测试书",
            None
        )
        .is_err());
    }
    #[test]
    fn audio_revision_keeps_fixed_template_and_other_content() {
        let raw = json!({"audioPrompt":{"introduction":"今天聊这本书。","sections":(1..=5).map(|i|json!({"heading":format!("主题{i}"),"guidance":"根据原书来源讲述，不编造。"})).collect::<Vec<_>>()}});
        let content = revise(
            &raw.to_string(),
            &original(),
            &[PublishingField::AudioPrompt],
            "测试书",
            None,
        )
        .unwrap();
        let value = serde_json::to_value(&content).unwrap();
        let audio = value["audioPrompt"].as_str().unwrap();
        assert!(audio.contains("欢迎大家收听今天的节目，我是甲子。"));
        assert!(!audio.contains("30分钟"));
        assert!(!audio.contains("10分钟"));
        assert!(!audio.contains("30分钟读懂一本书"));
        assert!(!audio.contains("接下来30分钟"));
        assert!(audio.contains("接下来，我们一起读懂这本书。"));
        assert!(audio.ends_with(
            "我是甲子。如果你喜欢这样的读书分享，记得关注我。下一期，我们继续一起读书。\n"
        ));
        assert!(audio.contains("## 第五部分：主题5"));
        assert!(audio.contains("《测试书》这本书"));
        let mut expected = serde_json::to_value(original()).unwrap();
        expected["audioPrompt"] = value["audioPrompt"].clone();
        assert_eq!(value, expected);
        let mut invalid = raw;
        invalid["audioPrompt"]["sections"]
            .as_array_mut()
            .unwrap()
            .pop();
        assert!(revise(
            &invalid.to_string(),
            &original(),
            &[PublishingField::AudioPrompt],
            "测试书",
            None
        )
        .is_err());
    }
}
