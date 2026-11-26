use super::Scene;
use serde::Deserialize;
use std::{collections::BTreeMap, sync::OnceLock};

#[derive(Deserialize)]
pub struct SceneKit {
    pub webgl: bool,
    pub objects: BTreeMap<String, String>,
}

fn kits() -> &'static BTreeMap<String, SceneKit> {
    static KITS: OnceLock<BTreeMap<String, SceneKit>> = OnceLock::new();
    KITS.get_or_init(|| {
        serde_json::from_str(include_str!("../../video/scene-kits.json"))
            .expect("bundled scene kits")
    })
}

pub fn contains(object: &str) -> bool {
    kits().values().any(|kit| kit.objects.contains_key(object))
}

pub fn validate(theme: &str, scenes: &[Scene]) -> Result<(), String> {
    for scene in scenes {
        let mut globes = 0;
        let mut product_cue = None;
        for beat in &scene.beats {
            if !contains(&beat.object) {
                continue;
            }
            let kit = kits()
                .get(theme)
                .filter(|kit| kit.objects.contains_key(&beat.object))
                .ok_or("分镜包含其他主题的专属主体，请选择当前主题目录中的物体。")?;
            if kit.webgl && !matches!(beat.action.as_str(), "draw" | "check" | "cross") {
                return Err("三维主体支持出现、画勾和画叉；请使用目录内的动作。".into());
            }
            if theme == "globe" {
                globes += 1;
            }
            if theme == "exploded" {
                if product_cue == Some(beat.cue) {
                    return Err("每个产品须使用不同的字幕锚点，按讲述顺序依次展示。".into());
                }
                product_cue = Some(beat.cue);
            }
        }
        if globes > 1 {
            return Err("每镜只支持一个地球仪；请保留一个主体。".into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_subjects_keep_theme_and_action_boundaries() {
        for (theme, kit) in kits() {
            for object in kit.objects.keys() {
                let mut scene: Scene = serde_json::from_value(serde_json::json!({"first":0,"last":0,"kind":"focus","title":"示意","beats":[{"cue":0,"object":object,"action":"draw","label":"主体"}]})).unwrap();
                super::super::validate_scenes(std::slice::from_ref(&scene), 0, 1).unwrap();
                validate(theme, std::slice::from_ref(&scene)).unwrap();
                assert!(validate("classic", std::slice::from_ref(&scene)).is_err());
                scene.beats[0].action = "slide".into();
                assert_eq!(
                    validate(theme, std::slice::from_ref(&scene)).is_ok(),
                    !kit.webgl
                );
            }
        }
    }
}
