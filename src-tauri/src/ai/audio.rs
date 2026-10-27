use serde::{Deserialize, Serialize};

pub const PROMPT_VERSION: &str = "audio-duration-v1";
pub const SYSTEM_PROMPT: &str = include_str!("../../../prompts/audio-duration-v1.md");

pub fn validate_duration(seconds: i64) -> Result<(), String> {
    if !(60..=86400).contains(&seconds) || seconds % 60 != 0 {
        return Err("音频目标时长须为 1—1440 分钟的整数。".into());
    }
    Ok(())
}

pub fn duration_label(seconds: i64) -> String {
    match (seconds / 60, seconds % 60) {
        (0, remainder) => format!("{remainder}秒"),
        (minutes, 0) => format!("{minutes}分钟"),
        (minutes, remainder) => format!("{minutes}分{remainder}秒"),
    }
}

pub fn context(seconds: i64) -> serde_json::Value {
    serde_json::json!({
        "targetDurationSeconds": seconds,
        "duration": duration_label(seconds),
        "seriesName": format!("{}读懂一本书", duration_label(seconds)),
        "spokenCharacters": seconds * 220 / 60
    })
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DurationRecommendation {
    pub minutes: i64,
    pub reason: String,
}
impl DurationRecommendation {
    pub fn parse(raw: &str) -> Result<Self, String> {
        let recommendation: Self =
            serde_json::from_str(raw).map_err(|_| "时长推荐结构无效，原时长与内容保留。")?;
        if !(1..=1440).contains(&recommendation.minutes)
            || recommendation.reason.trim().is_empty()
            || recommendation.reason.chars().count() > 500
        {
            return Err("时长推荐或说明无效，原时长与内容保留。".into());
        }
        Ok(recommendation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_invalid_recommendations_and_durations() {
        for raw in [
            "{}",
            r#"{"minutes":0,"reason":"短篇"}"#,
            r#"{"minutes":1441,"reason":"长篇"}"#,
            r#"{"minutes":10.5,"reason":"短篇"}"#,
            r#"{"minutes":10,"reason":" "}"#,
            r#"{"minutes":10,"reason":"短篇","extra":true}"#,
        ] {
            assert!(DurationRecommendation::parse(raw).is_err());
        }
        assert_eq!(
            DurationRecommendation::parse(
                r#"{"minutes":25,"reason":"按长篇小说的人物关系与主题安排。"}"#
            )
            .unwrap()
            .minutes,
            25
        );
        for seconds in [0, 59, 61, 86460] {
            assert!(validate_duration(seconds).is_err());
        }
        for seconds in [60, 600, 86400] {
            assert!(validate_duration(seconds).is_ok());
        }
    }
}
