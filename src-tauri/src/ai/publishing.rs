use super::WritingContent;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublishingDraft {
    title: String,
    description: String,
    introduction: String,
    sections: Vec<Section>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Section {
    heading: String,
    guidance: String,
}
impl PublishingDraft {
    pub fn into_content(self, book: &str) -> Result<WritingContent, String> {
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
        let audio_prompt = include_str!("../../../prompts/audio-template-v1.md")
            .replace(
                "{{book}}",
                book.trim().trim_start_matches('《').trim_end_matches('》'),
            )
            .replace("{{introduction}}", self.introduction.trim())
            .replace("{{sections}}", &sections);
        let content = WritingContent::Publishing {
            title: self.title.trim().into(),
            description: self.description.trim().into(),
            audio_prompt,
        };
        content.validate("publishing", None)?;
        Ok(content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn draft() -> serde_json::Value {
        serde_json::json!({"title":"一本书的故事", "description":"今天用十分钟聊这本书。", "introduction":"今天我们要聊的这本书，是《明朝那些事儿》。", "sections":(0..5).map(|i| serde_json::json!({"heading":format!("主题{i}"),"guidance":format!("按原书来源讲主题{i}，不写完整口播稿。") })).collect::<Vec<_>>()})
    }
    #[test]
    fn template_keeps_fixed_host_duration_and_book_specific_five_parts() {
        let value: PublishingDraft = serde_json::from_value(draft()).unwrap();
        let content = value.into_content("明朝那些事儿").unwrap();
        let WritingContent::Publishing { audio_prompt, .. } = content else {
            panic!()
        };
        assert!(audio_prompt.contains("8—10分钟"));
        assert!(audio_prompt.contains("欢迎大家收听今天的节目，我是甲子。"));
        assert!(audio_prompt.contains("《明朝那些事儿》这本书"));
        assert!(audio_prompt.contains("## 第五部分：主题4"));
        assert!(audio_prompt.contains("下一期，\n我们继续一起读书。"));
        assert!(!audio_prompt.contains("{{"));
    }
    #[test]
    fn rejects_incomplete_output_and_excessive_publish_text_before_saving() {
        let mut value = draft();
        value["sections"].as_array_mut().unwrap().pop();
        assert!(serde_json::from_value::<PublishingDraft>(value)
            .unwrap()
            .into_content("书")
            .is_err());
        for (field, count) in [("title", 31), ("description", 1001), ("introduction", 1501)] {
            let mut value = draft();
            value[field] = serde_json::json!("字".repeat(count));
            assert!(serde_json::from_value::<PublishingDraft>(value)
                .unwrap()
                .into_content("书")
                .is_err());
        }
        let mut value = draft();
        value["sections"][0]["guidance"] = serde_json::json!("");
        assert!(serde_json::from_value::<PublishingDraft>(value)
            .unwrap()
            .into_content("书")
            .is_err());
    }
}
