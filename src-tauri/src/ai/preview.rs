use super::{
    publishing::{AudioDraft, Section},
    revision::PublishingField,
};
use std::collections::BTreeMap;

/// Read only JSON string values. An unfinished string contributes its decoded prefix;
/// the original response still goes through the unchanged final business validation.
pub(crate) fn texts(
    raw: &str,
    selected: Option<&[PublishingField]>,
    book: &str,
    chapter: Option<&str>,
) -> BTreeMap<String, String> {
    let mut values = BTreeMap::new();
    let mut reader = Reader { raw, offset: 0 };
    if !raw.trim_start().starts_with('{') {
        return values;
    }
    reader.value("", &mut values, 0);
    let prefix = if values.keys().any(|key| key.starts_with("audioPrompt.")) {
        "audioPrompt."
    } else {
        ""
    };
    let text = |field: &str| {
        values
            .get(&format!("{prefix}{field}"))
            .cloned()
            .unwrap_or_default()
    };
    let draft = AudioDraft {
        hook: text("hook"),
        introduction: text("introduction"),
        sections: (0..5)
            .filter_map(|index| {
                let heading = text(&format!("sections.{index}.heading"));
                let guidance = text(&format!("sections.{index}.guidance"));
                (!heading.is_empty() || !guidance.is_empty())
                    .then_some(Section { heading, guidance })
            })
            .collect(),
        closing: text("closing"),
    };
    let audio = (!draft.hook.is_empty()
        || !draft.introduction.is_empty()
        || !draft.sections.is_empty()
        || !draft.closing.is_empty())
    .then(|| draft.render(book, chapter));
    let mut shown = BTreeMap::new();
    for field in [
        PublishingField::Title,
        PublishingField::Description,
        PublishingField::Tags,
        PublishingField::AudioPrompt,
        PublishingField::CoverTitle,
        PublishingField::CoverSubtitle,
        PublishingField::CoverPromptLandscape,
        PublishingField::CoverPromptPortrait,
        PublishingField::CoverPromptWidescreen,
    ] {
        if selected.is_some_and(|fields| !fields.contains(&field)) {
            continue;
        }
        let text = if field == PublishingField::AudioPrompt {
            audio.as_ref()
        } else {
            values.get(field.key())
        };
        if let Some(text) = text.filter(|text| !text.is_empty()) {
            shown.insert(field.key().into(), text.clone());
        }
    }
    shown
}

struct Reader<'a> {
    raw: &'a str,
    offset: usize,
}
impl Reader<'_> {
    fn peek(&mut self) -> Option<u8> {
        while self
            .raw
            .as_bytes()
            .get(self.offset)
            .is_some_and(u8::is_ascii_whitespace)
        {
            self.offset += 1;
        }
        self.raw.as_bytes().get(self.offset).copied()
    }
    fn value(&mut self, path: &str, values: &mut BTreeMap<String, String>, depth: usize) -> bool {
        // Writing output has at most four levels; do not recurse into arbitrary model data.
        if depth > 8 {
            return false;
        }
        match self.peek() {
            Some(b'"') => {
                let Some((text, complete)) = self.string() else {
                    return false;
                };
                values.insert(path.into(), text);
                complete
            }
            Some(b'{') => {
                self.offset += 1;
                if self.peek() == Some(b'}') {
                    self.offset += 1;
                    return true;
                }
                loop {
                    let Some((key, true)) = self.string() else {
                        return false;
                    };
                    if self.peek() != Some(b':') {
                        return false;
                    }
                    self.offset += 1;
                    let next = if path.is_empty() {
                        key
                    } else {
                        format!("{path}.{key}")
                    };
                    if !self.value(&next, values, depth + 1) {
                        return false;
                    }
                    match self.peek() {
                        Some(b'}') => {
                            self.offset += 1;
                            return true;
                        }
                        Some(b',') => {
                            self.offset += 1;
                        }
                        _ => return false,
                    }
                }
            }
            Some(b'[') => {
                self.offset += 1;
                if self.peek() == Some(b']') {
                    self.offset += 1;
                    return true;
                }
                for index in 0.. {
                    if !self.value(&format!("{path}.{index}"), values, depth + 1) {
                        return false;
                    }
                    match self.peek() {
                        Some(b']') => {
                            self.offset += 1;
                            return true;
                        }
                        Some(b',') => {
                            self.offset += 1;
                        }
                        _ => return false,
                    }
                }
                false
            }
            Some(_) => {
                let start = self.offset;
                while self.raw.as_bytes().get(self.offset).is_some_and(|byte| {
                    !matches!(byte, b',' | b'}' | b']') && !byte.is_ascii_whitespace()
                }) {
                    self.offset += 1;
                }
                self.offset > start
            }
            None => false,
        }
    }
    fn unicode(&mut self) -> Option<u16> {
        let digits = self.raw.get(self.offset..self.offset + 4)?;
        let value = u16::from_str_radix(digits, 16).ok()?;
        self.offset += 4;
        Some(value)
    }
    fn string(&mut self) -> Option<(String, bool)> {
        if self.peek() != Some(b'"') {
            return None;
        }
        self.offset += 1;
        let mut text = String::new();
        while let Some(character) = self.raw.get(self.offset..)?.chars().next() {
            self.offset += character.len_utf8();
            match character {
                '"' => return Some((text, true)),
                '\\' => {
                    let Some(escape) = self.raw.as_bytes().get(self.offset).copied() else {
                        break;
                    };
                    self.offset += 1;
                    let decoded = match escape {
                        b'"' => '"',
                        b'\\' => '\\',
                        b'/' => '/',
                        b'b' => '\u{0008}',
                        b'f' => '\u{000c}',
                        b'n' => '\n',
                        b'r' => '\r',
                        b't' => '\t',
                        b'u' => {
                            let Some(high) = self.unicode() else { break };
                            let scalar = if (0xd800..=0xdbff).contains(&high) {
                                if self.raw.get(self.offset..self.offset + 2) != Some("\\u") {
                                    break;
                                }
                                self.offset += 2;
                                let Some(low) = self.unicode() else { break };
                                if !(0xdc00..=0xdfff).contains(&low) {
                                    return None;
                                }
                                0x10000 + ((u32::from(high) - 0xd800) << 10) + u32::from(low)
                                    - 0xdc00
                            } else {
                                u32::from(high)
                            };
                            char::from_u32(scalar)?
                        }
                        _ => return None,
                    };
                    text.push(decoded);
                }
                value if value.is_control() => return None,
                value => text.push(value),
            }
        }
        Some((text, false))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::revision::PublishingField;

    #[test]
    fn audio_previews_match_saved_introduction_for_book_and_chapter() {
        let draft = serde_json::json!({
            "title":"本期标题", "description":"本期介绍", "tags":"#读书 #测试书 #选择",
            "hook":"一个选择会带来什么变化？", "introduction":"从关键选择出发，看看人物面对的困境。",
            "sections":(1..=5).map(|index|serde_json::json!({"heading":format!("看点{index}"),"guidance":"根据来源讲清因果。"})).collect::<Vec<_>>(),
            "closing":"回到开头，选择需要放回具体处境中理解。",
            "coverTitle":"选择的力量", "coverSubtitle":"本期看点",
            "coverPromptLandscape":"横版4:3无字背景", "coverPromptPortrait":"竖版3:4无字背景", "coverPromptWidescreen":"宽屏16:9无字背景",
            "coverDesign":{"titleSize":148,"subtitleSize":54,"textColor":"#ffe45c","subtitleColor":"#ffffff","accentColor":"#ff7547","backgroundColor":"#141414","overlayOpacity":0.75}
        });
        for chapter in [None, Some("序言"), Some("上篇·第二节")] {
            let content =
                serde_json::from_value::<crate::ai::publishing::PublishingDraft>(draft.clone())
                    .unwrap()
                    .into_content("测试书", chapter)
                    .unwrap();
            let crate::ai::WritingContent::Publishing { audio_prompt, .. } = &content else {
                panic!()
            };
            let shown = texts(&draft.to_string(), None, "测试书", chapter);
            assert_eq!(
                shown["audioPrompt"], *audio_prompt,
                "complete preview for {chapter:?}"
            );
            let patch = serde_json::json!({"audioPrompt":{
                "hook":draft["hook"],"introduction":draft["introduction"],"sections":draft["sections"],"closing":draft["closing"]
            }});
            let revised = crate::ai::revision::revise(
                &patch.to_string(),
                &content,
                &[PublishingField::AudioPrompt],
                "测试书",
                chapter,
            )
            .unwrap();
            let crate::ai::WritingContent::Publishing { audio_prompt, .. } = revised else {
                panic!()
            };
            assert_eq!(
                texts(
                    &patch.to_string(),
                    Some(&[PublishingField::AudioPrompt]),
                    "测试书",
                    chapter
                )["audioPrompt"],
                audio_prompt
            );
        }
    }
    #[test]
    fn previews_partial_tags_without_leaking_unselected_text() {
        let raw = r##"{"title":"当前书籍","tags":"#读书 #当前主"##;
        assert_eq!(texts(raw, None, "测试书", None)["tags"], "#读书 #当前主");
        let shown = texts(raw, Some(&[PublishingField::Tags]), "测试书", None);
        assert_eq!(shown.len(), 1);
        assert_eq!(shown["tags"], "#读书 #当前主");
        assert!(!texts(raw, Some(&[PublishingField::Title]), "测试书", None).contains_key("tags"));
    }

    #[test]
    fn partial_candidates_decode_text_and_audio_without_json_or_unselected_fields() {
        let partial = r#"{"title":"读懂\"一本书\"","description":"第一行\n第二行\uD83D\uDE00","hook":"本期引子","introduction":"书籍引入","sections":[{"heading":"主题一","guidance":"已经收到的指导"},{"heading":"主题二","guidance":"正在生"#;
        let shown = texts(partial, None, "测试书", Some("序言"));
        assert_eq!(shown["title"], "读懂\"一本书\"");
        assert_eq!(shown["description"], "第一行\n第二行😀");
        let audio = &shown["audioPrompt"];
        let positions = [
            "## 开场引子",
            "本期引子",
            "欢迎大家收听今天的节目，我是甲子。",
            "这一期，我们一起读《测试书》中的“序言”。",
            "书籍引入",
            "## 第一部分：主题一",
            "已经收到的指导",
            "## 第二部分：主题二",
            "正在生",
        ]
        .map(|text| audio.find(text).unwrap());
        assert!(positions.windows(2).all(|pair| pair[0] < pair[1]));
        assert!(!audio.contains("{{"));
        let shown = texts(
            partial,
            Some(&[PublishingField::Description]),
            "测试书",
            None,
        );
        assert_eq!(shown.len(), 1);
        assert_eq!(shown["description"], "第一行\n第二行😀");
    }
    #[test]
    fn unicode_escape_prefixes_never_display_half_surrogates_or_raw_escapes() {
        for raw in [
            r#"{"title":"前\"#,
            r#"{"title":"前\u"#,
            r#"{"title":"前\uD8"#,
            r#"{"title":"前\uD83D"#,
            r#"{"title":"前\uD83D\uDE"#,
        ] {
            assert_eq!(texts(raw, None, "测试书", None)["title"], "前");
        }
        assert_eq!(
            texts(r#"{"title":"前\uD83D\uDE00后"#, None, "测试书", None)["title"],
            "前😀后"
        );
    }

    #[test]
    fn revision_previews_include_only_selected_text_and_nested_audio_guidance() {
        let raw = r##"{"reasoning_content":"PRIVATE_REASONING","title":"未选标题","audioPrompt":{"hook":"新的引子","introduction":"新的引入","sections":[{"heading":"第一主题","guidance":"继续讲"}],"closing":"回应引子的收束"},"coverDesign":{"textColor":"#ffffff"}}"##;
        let shown = texts(
            raw,
            Some(&[PublishingField::AudioPrompt, PublishingField::CoverDesign]),
            "测试书",
            None,
        );
        assert_eq!(shown.len(), 1);
        let audio = &shown["audioPrompt"];
        assert!(audio.contains("这一期，我们一起读《测试书》。\n新的引入"));
        assert!(audio.contains("## 第一部分：第一主题\n\n继续讲"));
        assert!(audio.contains("## 结尾收束（逐字朗读）\n\n回应引子的收束"));
        assert!(!audio.contains("PRIVATE_REASONING") && !audio.contains("未选标题"));
        assert!(texts("非JSON的思考内容", None, "测试书", None).is_empty());
    }
}
