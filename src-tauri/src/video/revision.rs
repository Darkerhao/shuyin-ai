use super::{parse_srt, pencil, storyboard_prompt, validate_scenes, Cue, Scene, VideoPlan};
use serde::Deserialize;

pub const PROMPT_VERSION: &str = "video-scene-revision-v2";
pub fn prompt(theme: &str) -> String {
    format!(
        "{}\n{}",
        storyboard_prompt(theme),
        include_str!("../../../prompts/video-scene-revision-v2.md")
    )
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SceneRevisionInput {
    pub job_id: String,
    pub run_id: i64,
    pub provider_id: String,
    pub scene_index: usize,
    pub subtitles: String,
    pub scene: Scene,
    pub previous_scene: Option<Scene>,
    pub next_scene: Option<Scene>,
    pub instruction: String,
}

impl SceneRevisionInput {
    pub fn validate(&self, plan: &VideoPlan) -> Result<Vec<Cue>, String> {
        if self.instruction.trim().is_empty() || self.instruction.chars().count() > 2000 {
            return Err("请填写这一镜的修改要求，最多2000字。".into());
        }
        let cues = parse_srt(&self.subtitles, plan.duration)?;
        let scope = "镜头范围已变化，请先保存并重新读取分镜。";
        if plan
            .scenes
            .last()
            .and_then(|scene| scene.last.checked_add(1))
            != Some(cues.len())
        {
            return Err(scope.into());
        }
        for (index, scene) in [
            (Some(self.scene_index), Some(&self.scene)),
            (
                self.scene_index.checked_sub(1),
                self.previous_scene.as_ref(),
            ),
            (self.scene_index.checked_add(1), self.next_scene.as_ref()),
        ] {
            let Some(scene) = scene else { continue };
            let saved = index.and_then(|i| plan.scenes.get(i)).ok_or(scope)?;
            if (scene.first, scene.last) != (saved.first, saved.last) || scene.last >= cues.len() {
                return Err(scope.into());
            }
            validate_scenes(std::slice::from_ref(scene), scene.first, scene.last + 1)?;
        }
        if let Some(artwork) = &self.scene.artwork {
            let cast = &plan
                .pencil
                .as_ref()
                .ok_or("任务缺少冻结人物设定。")?
                .characters;
            artwork.validate_characters(cast)?;
        }
        Ok(cues)
    }

    pub fn user(&self, plan: &VideoPlan, cues: &[Cue]) -> String {
        let scene = &self.scene;
        if let Some(artwork) = &scene.artwork {
            let planned = pencil::PlannedScene {
                first: scene.first,
                last: scene.last,
                title: scene.title.clone(),
                description: format!("{}\n本次修改：{}", artwork.design, self.instruction),
                characters: artwork.character_ids().map(str::to_owned).collect(),
            };
            return pencil::drawing_input(plan, cues, &planned);
        }
        // Image assets, local paths and rights notes are not sent to the text model.
        let scene_context = |scene: &Scene| {
            serde_json::json!({
                "first":scene.first,"last":scene.last,"kind":scene.kind,"title":scene.title,
                "environment":scene.environment,"framing":scene.framing,"beats":scene.beats,"hasImage":scene.image.is_some()
            })
        };
        let cue_context = |first: usize, end: usize| {
            cues[first..end].iter().enumerate().map(|(offset, cue)| serde_json::json!({"index":first+offset,"start":cue.start,"end":cue.end,"text":cue.text})).collect::<Vec<_>>()
        };
        serde_json::json!({
            "book":plan.title,"instruction":self.instruction,"targetScene":scene_context(scene),"hasImage":scene.image.is_some(),
            "cues":cue_context(scene.first,scene.last+1),
            "previousCues":cue_context(scene.first.saturating_sub(3),scene.first),
            "followingCues":cue_context(scene.last+1,(scene.last+4).min(cues.len())),
            "previousScene":self.previous_scene.as_ref().map(scene_context),
            "nextScene":self.next_scene.as_ref().map(scene_context),
        }).to_string()
    }
}

pub fn parse_candidate(original: &Scene, raw: &str) -> Result<Scene, String> {
    let values: Vec<serde_json::Value> =
        serde_json::from_str(raw).map_err(|_| "模型须返回只含这一镜的JSON数组。")?;
    if values.len() != 1
        || ["image", "labels", "artwork", "painting"]
            .iter()
            .any(|key| values[0].get(*key).is_some())
    {
        return Err("模型只能返回这一镜的手绘字段，不能更换图片或返回其他镜头。".into());
    }
    let mut candidate: Scene = serde_json::from_value(values.into_iter().next().unwrap())
        .map_err(|_| "候选分镜结构无效，原分镜保留。")?;
    if (candidate.first, candidate.last) != (original.first, original.last) {
        return Err("候选改变了本镜字幕范围，原分镜保留。".into());
    }
    candidate.image = original.image.clone();
    if candidate.image.is_none() {
        super::require_drawings(std::slice::from_ref(&candidate))?;
    }
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

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use serde_json::json;

    pub(crate) fn input() -> SceneRevisionInput {
        serde_json::from_value(json!({
            "jobId":"job","runId":1,"providerId":"model","sceneIndex":1,"instruction":"把转折放在第二句",
            "subtitles":"1\n00:00:00,000 --> 00:00:02,000\n开场\n\n2\n00:00:02,000 --> 00:00:04,000\n观点\n\n3\n00:00:04,000 --> 00:00:06,000\n结论\n\n4\n00:00:06,000 --> 00:00:08,000\n结尾\n",
            "previousScene":{"first":0,"last":0,"kind":"opening","title":"开场","beats":[{"cue":0,"object":"book","action":"draw","label":"书籍"}]},
            "scene":{"first":1,"last":2,"kind":"focus","title":"原观点","beats":[{"cue":1,"object":"book","action":"draw","label":"阅读"}]},
            "nextScene":{"first":3,"last":3,"kind":"closing","title":"结尾","beats":[{"cue":3,"object":"book","action":"draw","label":"结尾"}]}
        })).unwrap()
    }
    pub(crate) fn plan(input: &SceneRevisionInput) -> VideoPlan {
        VideoPlan {
            title: "书名".into(),
            duration: 8.0,
            cues: parse_srt(&input.subtitles, 8.0).unwrap(),
            scenes: vec![
                input.previous_scene.clone().unwrap(),
                input.scene.clone(),
                input.next_scene.clone().unwrap(),
            ],
            ..Default::default()
        }
    }
    pub(crate) fn candidate() -> serde_json::Value {
        json!([{"first":1,"last":2,"kind":"focus","title":"新的观点","environment":"study","framing":"detail","beats":[{"cue":1,"object":"长枪","action":"draw","label":"思考","drawing":{"strokes":[[[20,20],[60,100]]]}},{"cue":2,"object":"营寨","action":"check","label":"结论","drawing":{"strokes":[[[100,20],[160,100]]]}}]}])
    }

    #[test]
    fn scene_revision_preserves_range_and_builds_context_from_current_edits() {
        let mut input = input();
        let plan = plan(&input);
        input.previous_scene.as_mut().unwrap().title = "已修改的邻镜".into();
        let cues = input.validate(&plan).unwrap();
        let user: serde_json::Value = serde_json::from_str(&input.user(&plan, &cues)).unwrap();
        assert_eq!(user["cues"].as_array().unwrap().len(), 2);
        assert_eq!(user["cues"][0]["index"], 1);
        assert_eq!(user["previousScene"]["title"], "已修改的邻镜");
        assert_eq!(user["nextScene"]["first"], 3);
        let scene = parse_candidate(&input.scene, &candidate().to_string()).unwrap();
        assert_eq!((scene.first, scene.last), (1, 2));
        assert_eq!(scene.beats[1].cue, 2);
        assert_eq!(input.scene.title, "原观点");
    }

    #[test]
    fn scene_revision_rejects_extra_scenes_changed_anchors_and_uncontrolled_fields() {
        let original = &input().scene;
        let mut wrong = candidate();
        wrong[0]["first"] = json!(0);
        assert!(parse_candidate(original, &wrong.to_string()).is_err());
        let mut wrong = candidate();
        wrong[0]["beats"][0]["cue"] = json!(2);
        assert!(parse_candidate(original, &wrong.to_string()).is_err());
        let mut wrong = candidate();
        wrong[0]["beats"][0]["drawing"]["strokes"] = json!([[[0, 0], [1001, 0]]]);
        assert!(parse_candidate(original, &wrong.to_string()).is_err());
        for field in ["image", "cues", "labels", "camera", "artwork", "painting"] {
            let mut wrong = candidate();
            wrong[0][field] = json!(null);
            assert!(
                parse_candidate(original, &wrong.to_string()).is_err(),
                "{field}"
            );
        }
        let wrong = json!([candidate()[0], candidate()[0]]);
        assert!(parse_candidate(original, &wrong.to_string()).is_err());
        assert!(parse_candidate(original, "[]").is_err());
        assert!(parse_candidate(original, "broken").is_err());
    }

    #[test]
    fn image_revision_preserves_the_selected_asset_without_sending_its_private_fields() {
        let mut input = input();
        input.scene.image = Some(serde_json::from_value(json!({"assetId":"private-asset","source":"D:/private/source.jpg","rights":"私人来源备注","fit":"cover","x":0.25,"y":0.75,"sha256":"abc"})).unwrap());
        input.scene.beats.clear();
        let plan = plan(&input);
        let cues = input.validate(&plan).unwrap();
        let user = input.user(&plan, &cues);
        for secret in ["private-asset", "D:/private", "私人来源备注", "sha256"] {
            assert!(!user.contains(secret));
        }
        let mut raw = candidate();
        raw[0]["beats"] = json!([]);
        let scene = parse_candidate(&input.scene, &raw.to_string()).unwrap();
        assert_eq!(
            serde_json::to_value(scene.image).unwrap(),
            serde_json::to_value(&input.scene.image).unwrap()
        );
        assert!(parse_candidate(&input.scene, &candidate().to_string()).is_err());
    }
}
