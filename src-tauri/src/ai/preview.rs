use super::revision::PublishingField;
use std::collections::BTreeMap;

/// Read only JSON string values. An unfinished string contributes its decoded prefix;
/// the original response still goes through the unchanged final business validation.
pub(crate) fn texts(raw: &str, selected: Option<&[PublishingField]>) -> BTreeMap<String, String> {
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
    let mut audio = Vec::new();
    if let Some(text) = values.get(&format!("{prefix}introduction")) {
        audio.push(text.clone());
    }
    for index in 0..5 {
        let section = ["heading", "guidance"]
            .iter()
            .filter_map(|key| values.get(&format!("{prefix}sections.{index}.{key}")))
            .cloned()
            .collect::<Vec<_>>()
            .join("\n");
        if !section.is_empty() {
            audio.push(section);
        }
    }
    let audio = audio.join("\n\n");
    let mut shown = BTreeMap::new();
    for field in [
        PublishingField::Title,
        PublishingField::Description,
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
            Some(&audio)
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
    fn partial_candidates_decode_text_and_audio_without_json_or_unselected_fields() {
        let partial = r#"{"title":"读懂\"一本书\"","description":"第一行\n第二行\uD83D\uDE00","introduction":"书籍引入","sections":[{"heading":"主题一","guidance":"已经收到的指导"},{"heading":"主题二","guidance":"正在生"#;
        let shown = texts(partial, None);
        assert_eq!(shown["title"], "读懂\"一本书\"");
        assert_eq!(shown["description"], "第一行\n第二行😀");
        assert_eq!(
            shown["audioPrompt"],
            "书籍引入\n\n主题一\n已经收到的指导\n\n主题二\n正在生"
        );
        let shown = texts(partial, Some(&[PublishingField::Description]));
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
            assert_eq!(texts(raw, None)["title"], "前");
        }
        assert_eq!(
            texts(r#"{"title":"前\uD83D\uDE00后"#, None)["title"],
            "前😀后"
        );
    }

    #[test]
    fn revision_previews_include_only_selected_text_and_nested_audio_guidance() {
        let raw = r##"{"reasoning_content":"PRIVATE_REASONING","title":"未选标题","audioPrompt":{"introduction":"新的引入","sections":[{"heading":"第一主题","guidance":"继续讲"}]},"coverDesign":{"textColor":"#ffffff"}}"##;
        let shown = texts(
            raw,
            Some(&[PublishingField::AudioPrompt, PublishingField::CoverDesign]),
        );
        assert_eq!(shown.len(), 1);
        assert_eq!(shown["audioPrompt"], "新的引入\n\n第一主题\n继续讲");
        assert!(texts("非JSON的思考内容", None).is_empty());
    }
}
