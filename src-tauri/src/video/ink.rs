use super::{validate_scenes, Scene, VideoPlan};
use serde::{Deserialize, Serialize};

pub const PROMPT: &str = include_str!("../../../prompts/video-ink-storyboard-v1.md");
pub const IMAGE_VERSION: &str = "video-ink-painting-v1";
pub const IMAGE_PROMPT: &str = include_str!("../../../prompts/video-ink-painting-v1.md");

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Painting {
    pub description: String,
    pub motion: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
}

impl Painting {
    pub fn validate(&self) -> Result<(), String> {
        if !(80..=1200).contains(&self.description.trim().chars().count())
            || self
                .description
                .chars()
                .any(|c| c.is_control() && c != '\n')
            || !["static", "push_in", "pull_out", "pan_left", "pan_right"]
                .contains(&self.motion.as_str())
        {
            return Err("水墨画面须有80—1200字的具体描述，并选用静止、推近、拉远或平移。".into());
        }
        Ok(())
    }
}

pub fn parse(raw: &str, first: usize, end: usize) -> Result<Vec<Scene>, String> {
    let scenes: Vec<Scene> =
        serde_json::from_str(raw).map_err(|_| "水墨分镜不是合法JSON，原字幕和已完成画面保留。")?;
    validate_scenes(&scenes, first, end)?;
    if scenes.iter().any(|scene| {
        scene.image.is_some()
            || scene
                .painting
                .as_ref()
                .is_none_or(|painting| painting.task_id.is_some())
    }) {
        return Err("水墨分镜须提供画面描述，不允许模型指定图片或任务引用。".into());
    }
    Ok(scenes)
}

pub fn prompt() -> String {
    format!("{}\n只返回合法JSON数组，不要输出代码围栏。", PROMPT)
}

pub fn input(plan: &VideoPlan, first: usize, end: usize) -> String {
    let cues = |from: usize, to: usize| {
        plan.cues[from..to].iter().enumerate().map(|(offset,cue)|serde_json::json!({"index":from+offset,"start":cue.start,"end":cue.end,"text":cue.text})).collect::<Vec<_>>()
    };
    let scenes = plan.scenes[plan.scenes.len().saturating_sub(3)..].iter().map(|scene|serde_json::json!({"first":scene.first,"last":scene.last,"kind":scene.kind,"title":scene.title,"direction":scene.painting.as_ref().map(|p|p.description.as_str())})).collect::<Vec<_>>();
    serde_json::json!({"book":plan.title,"duration":plan.duration,"opening":cues(0,plan.cues.len().min(6)),"ending":cues(plan.cues.len().saturating_sub(6),plan.cues.len()),"previousScenes":scenes,"followingContext":cues(end,(end+3).min(plan.cues.len())),"cues":cues(first,end)}).to_string()
}

pub fn image_description(plan: &VideoPlan, index: usize) -> String {
    let scene = &plan.scenes[index];
    fn direction(scene: &Scene) -> Option<&str> {
        scene.painting.as_ref().map(|p| p.description.as_str())
    }
    serde_json::json!({
        "scene":index+1,"title":scene.title,"direction":direction(scene),
        "narration":plan.cues[scene.first..=scene.last].iter().map(|cue|cue.text.as_str()).collect::<Vec<_>>(),
        "previousDirection":index.checked_sub(1).and_then(|i|direction(&plan.scenes[i])),
        "nextDirection":plan.scenes.get(index+1).and_then(direction),
        "continuity":"同一部影片使用统一的精细中国水墨画语言、主体特征与纸墨质感；只绘本镜台词成立的内容。"
    }).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn ink_storyboard_keeps_detailed_audio_based_art_direction_and_rejects_asset_injection() {
        let scene = json!({"first":0,"last":1,"kind":"focus","title":"江畔读书","beats":[],
            "painting":{"description":"近景以细密皴擦描绘案上翻开的书卷和磨损纸边；中景是一名低头阅读者的侧影，淡墨勾出袖口和手势；远景以湿墨层层晕染江岸与远山，主体清楚，前后空间自然衔接，并交代留白和墨色的变化节奏。","motion":"push_in"}});
        let scenes = parse(&json!([scene.clone()]).to_string(), 0, 2).unwrap();
        let saved = serde_json::to_value(&scenes[0]).unwrap();
        assert_eq!(saved["painting"]["motion"], "push_in");
        assert!(saved["painting"]["description"]
            .as_str()
            .unwrap()
            .contains("磨损纸边"));
        let mut injected = scene.clone();
        injected["image"] = json!({"assetId":"external"});
        assert!(parse(&json!([injected]).to_string(), 0, 2).is_err());
        for value in ["简笔画", "", "https://example.com/image.png"] {
            let mut invalid = scene.clone();
            invalid["painting"]["description"] = json!(value);
            assert!(parse(&json!([invalid]).to_string(), 0, 2).is_err());
        }
        let mut invalid = scene;
        invalid["painting"]["motion"] = json!("arbitrary-script");
        assert!(parse(&json!([invalid]).to_string(), 0, 2).is_err());
        let mut invalid = scenes[0].clone();
        invalid.painting.as_mut().unwrap().description = "墨".repeat(1201);
        assert!(invalid.painting.unwrap().validate().is_err());
    }

    #[test]
    fn ink_input_sends_audio_cues_and_previous_direction_without_task_or_asset_ids() {
        let plan: VideoPlan = serde_json::from_value(json!({"title":"书","audioSha256":"source","duration":4,"cues":[{"start":0,"end":2,"text":"江边"},{"start":2,"end":4,"text":"读书"}],"scenes":[{"first":0,"last":1,"kind":"focus","title":"画面","painting":{"description":"近景以细笔描绘江边书案与纸张纤维，中景呈现人物安静阅读的姿态，远景以淡墨铺陈水岸空间，主体关系、衣袖纹理和墨色留白均与本镜字幕对应。人物动作与景物关系自然相连。","motion":"static","taskId":"secret"}}]})).unwrap();
        let user = input(&plan, 0, 2);
        assert!(user.contains("江边"));
        assert!(user.contains("江边书案"));
        assert!(!user.contains("secret"));
    }
}
