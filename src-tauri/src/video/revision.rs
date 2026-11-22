use super::{parse_srt, storyboard_prompt, validate_scenes, Cue, Scene};
use serde::{Deserialize, Serialize};

pub const PROMPT_VERSION: &str = "video-scene-revision-v1";
pub fn prompt() -> String {
    format!(
        "{}\n{}",
        storyboard_prompt(),
        include_str!("../../../prompts/video-scene-revision-v1.md")
    )
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SceneRevisionInput {
    pub job_id: String,
    pub run_id: i64,
    pub provider_id: String,
    pub scene_index: usize,
    pub subtitles: String,
    pub scenes: Vec<Scene>,
    pub instruction: String,
}

impl SceneRevisionInput {
    pub fn validate(&self, duration: f64, saved_scenes: &[Scene]) -> Result<Vec<Cue>, String> {
        if self.instruction.trim().is_empty() || self.instruction.chars().count() > 2000 {
            return Err("请填写这一镜的修改要求，最多2000字。".into());
        }
        if self.scene_index >= self.scenes.len()
            || self.scenes.len() != saved_scenes.len()
            || self
                .scenes
                .iter()
                .zip(saved_scenes)
                .any(|(a, b)| (a.first, a.last) != (b.first, b.last))
        {
            return Err("镜头范围已变化，请先保存并重新读取分镜。".into());
        }
        let cues = parse_srt(&self.subtitles, duration)?;
        validate_scenes(&self.scenes, 0, cues.len())?;
        Ok(cues)
    }

    pub fn user(&self, book: &str, cues: &[Cue]) -> String {
        // Image assets, local paths and rights notes are not sent to the text model.
        let scene_context = |scene: &Scene| {
            serde_json::json!({
                "first":scene.first,"last":scene.last,"kind":scene.kind,"title":scene.title,
                "environment":scene.environment,"framing":scene.framing,"beats":scene.beats,"hasImage":scene.image.is_some()
            })
        };
        let scene = &self.scenes[self.scene_index];
        let cue_context = |first: usize, end: usize| {
            cues[first..end].iter().enumerate().map(|(offset, cue)| serde_json::json!({"index":first+offset,"start":cue.start,"end":cue.end,"text":cue.text})).collect::<Vec<_>>()
        };
        serde_json::json!({
            "book":book,"instruction":self.instruction,"targetScene":scene_context(scene),"hasImage":scene.image.is_some(),
            "cues":cue_context(scene.first,scene.last+1),
            "previousCues":cue_context(scene.first.saturating_sub(3),scene.first),
            "followingCues":cue_context(scene.last+1,(scene.last+4).min(cues.len())),
            "previousScene":self.scene_index.checked_sub(1).map(|i| scene_context(&self.scenes[i])),
            "nextScene":self.scenes.get(self.scene_index+1).map(scene_context),
        }).to_string()
    }

    pub fn candidate(&self, raw: &str) -> Result<Scene, String> {
        let values: Vec<serde_json::Value> =
            serde_json::from_str(raw).map_err(|_| "模型须返回只含这一镜的JSON数组。")?;
        if values.len() != 1
            || values[0].get("image").is_some()
            || values[0].get("labels").is_some()
        {
            return Err("模型只能返回这一镜的手绘字段，不能更换图片或返回其他镜头。".into());
        }
        let mut candidate: Scene = serde_json::from_value(values.into_iter().next().unwrap())
            .map_err(|_| "候选分镜结构无效，原分镜保留。")?;
        let original = &self.scenes[self.scene_index];
        if (candidate.first, candidate.last) != (original.first, original.last) {
            return Err("候选改变了本镜字幕范围，原分镜保留。".into());
        }
        candidate.image = original.image.clone();
        validate_scenes(
            std::slice::from_ref(&candidate),
            original.first,
            original.last + 1,
        )?;
        if candidate.image.is_none() && candidate.beats[0].cue != original.first {
            return Err("候选的首个物体须从本镜第一条字幕开始，原分镜保留。".into());
        }
        Ok(candidate)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use serde_json::json;

    pub(crate) fn input() -> SceneRevisionInput {
        serde_json::from_value(json!({
            "jobId":"job","runId":1,"providerId":"model","sceneIndex":1,"instruction":"把转折放在第二句",
            "subtitles":"1\n00:00:00,000 --> 00:00:02,000\n开场\n\n2\n00:00:02,000 --> 00:00:04,000\n观点\n\n3\n00:00:04,000 --> 00:00:06,000\n结论\n\n4\n00:00:06,000 --> 00:00:08,000\n结尾\n",
            "scenes":[
                {"first":0,"last":0,"kind":"opening","title":"开场","beats":[{"cue":0,"object":"book","action":"draw","label":"书籍"}]},
                {"first":1,"last":2,"kind":"focus","title":"原观点","beats":[{"cue":1,"object":"book","action":"draw","label":"阅读"}]},
                {"first":3,"last":3,"kind":"closing","title":"结尾","beats":[{"cue":3,"object":"book","action":"draw","label":"结尾"}]}
            ]
        })).unwrap()
    }
    pub(crate) fn candidate() -> serde_json::Value {
        json!([{"first":1,"last":2,"kind":"focus","title":"新的观点","environment":"study","framing":"detail","beats":[{"cue":1,"object":"pen","action":"draw","label":"思考"},{"cue":2,"object":"book","action":"check","label":"结论"}]}])
    }

    #[test]
    fn scene_revision_preserves_range_and_builds_context_from_current_edits() {
        let mut input = input();
        input.scenes[0].title = "已修改的邻镜".into();
        let cues = input.validate(8.0, &input.scenes).unwrap();
        let user: serde_json::Value = serde_json::from_str(&input.user("书名", &cues)).unwrap();
        assert_eq!(user["cues"].as_array().unwrap().len(), 2);
        assert_eq!(user["cues"][0]["index"], 1);
        assert_eq!(user["previousScene"]["title"], "已修改的邻镜");
        assert_eq!(user["nextScene"]["first"], 3);
        let scene = input.candidate(&candidate().to_string()).unwrap();
        assert_eq!((scene.first, scene.last), (1, 2));
        assert_eq!(scene.beats[1].cue, 2);
        assert_eq!(input.scenes[1].title, "原观点");
    }

    #[test]
    fn scene_revision_rejects_extra_scenes_changed_anchors_and_uncontrolled_fields() {
        let input = input();
        let mut wrong = candidate();
        wrong[0]["first"] = json!(0);
        assert!(input.candidate(&wrong.to_string()).is_err());
        let mut wrong = candidate();
        wrong[0]["beats"][0]["cue"] = json!(2);
        assert!(input.candidate(&wrong.to_string()).is_err());
        let mut wrong = candidate();
        wrong[0]["beats"][0]["object"] = json!("arbitrary-svg");
        assert!(input.candidate(&wrong.to_string()).is_err());
        for field in ["image", "cues", "labels", "camera"] {
            let mut wrong = candidate();
            wrong[0][field] = json!(null);
            assert!(input.candidate(&wrong.to_string()).is_err(), "{field}");
        }
        let wrong = json!([candidate()[0], candidate()[0]]);
        assert!(input.candidate(&wrong.to_string()).is_err());
        assert!(input.candidate("[]").is_err());
        assert!(input.candidate("broken").is_err());
    }

    #[test]
    fn image_revision_preserves_the_selected_asset_without_sending_its_private_fields() {
        let mut input = input();
        input.scenes[1].image = Some(serde_json::from_value(json!({"assetId":"private-asset","source":"D:/private/source.jpg","rights":"私人来源备注","fit":"cover","x":0.25,"y":0.75,"sha256":"abc"})).unwrap());
        input.scenes[1].beats.clear();
        let cues = input.validate(8.0, &input.scenes).unwrap();
        let user = input.user("书名", &cues);
        for secret in ["private-asset", "D:/private", "私人来源备注", "sha256"] {
            assert!(!user.contains(secret));
        }
        let mut raw = candidate();
        raw[0]["beats"] = json!([]);
        let scene = input.candidate(&raw.to_string()).unwrap();
        assert_eq!(
            serde_json::to_value(scene.image).unwrap(),
            serde_json::to_value(&input.scenes[1].image).unwrap()
        );
        assert!(input.candidate(&candidate().to_string()).is_err());
    }
}
