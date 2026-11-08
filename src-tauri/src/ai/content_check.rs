use super::{
    publishing::TranscriptDraft,
    revision::{self, PublishingField},
    WritingContent,
};
use serde::Deserialize;
use serde_json::{Map, Value};

pub const VERSION: &str = "content-check-v1";
pub const PROMPT: &str = include_str!("../../../prompts/content-check-v1.md");

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Check {
    verified: bool,
    reason: String,
    changes: Map<String, Value>,
}

pub fn apply(
    raw: &str,
    base: &WritingContent,
    book: &str,
    seconds: i64,
) -> Result<WritingContent, String> {
    let check: Check =
        serde_json::from_str(raw).map_err(|_| "内容校核必须返回结论和必要修改项。")?;
    if check.reason.trim().is_empty() || check.reason.chars().count() > 500 {
        return Err("内容校核缺少有效依据。".into());
    }
    if !check.verified {
        return Err(format!(
            "无法完成内容校核：{}。原稿和音频已保留。",
            check.reason.trim()
        ));
    }
    let fields = check
        .changes
        .keys()
        .map(|key| {
            let field: PublishingField = serde_json::from_value(Value::String(key.clone()))
                .map_err(|_| "内容校核包含不允许修改的字段。")?;
            if field == PublishingField::AudioPrompt {
                return Err("内容校核不能改写实际音频指导。".to_string());
            }
            Ok(field)
        })
        .collect::<Result<Vec<_>, String>>()?;
    let content = if fields.is_empty() {
        base.clone()
    } else {
        revision::revise(
            &serde_json::to_string(&check.changes).map_err(|e| e.to_string())?,
            base,
            &fields,
            book,
            seconds,
        )?
    };
    let mut value = serde_json::to_value(&content).map_err(|e| e.to_string())?;
    let object = value.as_object_mut().ok_or("发布材料结构无效。")?;
    object.remove("format");
    let audio = object
        .remove("audioPrompt")
        .and_then(|v| v.as_str().map(str::to_owned))
        .ok_or("原音频指导缺失。")?;
    // Reuse the existing final publishing validation, including all three cover prompts.
    serde_json::from_value::<TranscriptDraft>(value)
        .map_err(|_| "发布材料结构不完整。")?
        .into_content(audio)?;
    Ok(content)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn base() -> WritingContent {
        serde_json::from_value(serde_json::json!({"format":"publishing","title":"原标题","description":"原介绍","tags":"#读书 #主题 #理解","audioPrompt":"实际使用的指导，不可变","coverTitle":"封面标题","coverSubtitle":"书籍副标题","coverPromptLandscape":"4:3 横向场景","coverPromptPortrait":"3:4 纵向场景","coverPromptWidescreen":"16:9 宽阔场景","coverDesign":{"titleSize":136,"subtitleSize":50,"textColor":"#ffffff","subtitleColor":"#ffffff","accentColor":"#abcdef","backgroundColor":"#123456","overlayOpacity":0.7}})).unwrap()
    }
    #[test]
    fn keeps_consistent_content_and_only_applies_explicit_changes() {
        let original = base();
        assert_eq!(
            apply(
                r#"{"verified":true,"reason":"一致","changes":{}}"#,
                &original,
                "书",
                600
            )
            .unwrap(),
            original
        );
        let changed = apply(
            r#"{"verified":true,"reason":"字幕谈的是阅读","changes":{"title":"阅读"}}"#,
            &original,
            "书",
            600,
        )
        .unwrap();
        let mut expected = serde_json::to_value(original).unwrap();
        expected["title"] = Value::String("阅读".into());
        assert_eq!(serde_json::to_value(changed).unwrap(), expected);
    }
    #[test]
    fn rejects_rewritten_audio_invalid_content_and_extra_fields() {
        for raw in [
            r#"{"verified":true,"reason":"替换","changes":{"audioPrompt":"换指导"}}"#,
            r#"{"verified":true,"reason":"","changes":{}}"#,
            r#"{"verified":true,"reason":"调整","changes":{"title":""}}"#,
            r#"{"verified":true,"reason":"调整","changes":{"coverPromptWidescreen":"4:3错误"}}"#,
            r#"{"verified":true,"reason":"一致","changes":{},"passed":true}"#,
            r#"{"reason":"无法核对","changes":{}}"#,
            r#"{"verified":false,"reason":"字幕缺少有效讲述","changes":{}}"#,
            r#"{"verified":false,"reason":"不能确定","changes":{"title":"猜测标题"}}"#,
        ] {
            assert!(apply(raw, &base(), "书", 600).is_err());
        }
    }
}
