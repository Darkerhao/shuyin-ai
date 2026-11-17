use super::*;
use serde_json::json;
use std::sync::Mutex;

fn head() -> serde_json::Value {
    json!({"mask":[],"strokes":[{"p":[[20,20],[80,80]],"w":1.2,"a":0.6}],"shades":[]})
}
fn character() -> Character {
    serde_json::from_value(json!({"id":"reader","name":"读者","description":"方巾与窄领","heads":{"front":head(),"side":head()}})).unwrap()
}
fn artwork() -> serde_json::Value {
    json!({"design":"完整的读者侧身场景","layers":[{"cue":0,"name":"读者","mask":[],"strokes":[{"p":[[50,300],[100,400]],"w":1.2,"a":0.6}],"shades":[],"texts":[],"portrait":{"characterId":"reader","view":"front","x":80,"y":150,"scale":0.6,"flip":false}}]})
}
fn plan() -> VideoPlan {
    VideoPlan {
        duration: 20.,
        cues: (0..4)
            .map(|i| Cue {
                start: f64::from(i) * 5.,
                end: f64::from(i + 1) * 5.,
                text: format!("台词{i}"),
            })
            .collect(),
        theme: "ink".into(),
        ..Default::default()
    }
}

#[test]
fn pencil_short_scenes_merge_preserving_cue_coverage_and_all_content() {
    let mut p = plan();
    p.cues[3].start = 17.;
    let mut scenes = vec![
        PlannedScene {
            first: 0,
            last: 2,
            title: "查账".into(),
            description: "阅读账簿".into(),
            characters: vec!["reader".into()],
        },
        PlannedScene {
            first: 3,
            last: 3,
            title: "上告".into(),
            description: "在原画面增补公文".into(),
            characters: vec!["reader".into()],
        },
    ];
    merge_short(&mut scenes, &p.cues, p.duration);
    assert_eq!(scenes.len(), 1);
    assert_eq!((scenes[0].first, scenes[0].last), (0, 3));
    assert!(scenes[0].description.contains("公文"));
    assert_eq!(scenes[0].characters, vec!["reader"]);
}
#[test]
fn pencil_geometry_rejects_foreign_character_and_unclosed_regions() {
    let a = parse_artwork(&artwork().to_string(), 0, 3, &[character()]).unwrap();
    assert_eq!(
        a.layers[0].portrait.as_ref().unwrap().character_id,
        "reader"
    );
    assert!(parse_artwork(&artwork().to_string(), 0, 3, &[]).is_err());
    let mut value = artwork();
    value["layers"][0]["mask"] = json!([[[0, 0], [100, 100]]]);
    assert!(parse_artwork(&value.to_string(), 0, 3, &[character()]).is_err());
    let mut value = artwork();
    value["characters"] = json!([]);
    assert!(parse_artwork(&value.to_string(), 0, 3, &[character()]).is_err());
}
#[test]
fn pencil_required_cast_and_expanded_budget_are_checked_before_adoption() {
    let a = parse_artwork(&artwork().to_string(), 0, 3, &[character()]).unwrap();
    assert!(a.validate_cast(&["reader".into()]).is_ok());
    assert!(a.validate_cast(&[]).is_err());
    let mut no_head = artwork();
    no_head["layers"][0]
        .as_object_mut()
        .unwrap()
        .remove("portrait");
    let a = parse_artwork(&no_head.to_string(), 0, 3, &[character()]).unwrap();
    assert!(a.validate_cast(&["reader".into()]).is_err());
    let mut repeated = artwork();
    let layer = repeated["layers"][0].clone();
    repeated["layers"].as_array_mut().unwrap().push(layer);
    assert!(parse_artwork(&repeated.to_string(), 0, 3, &[character()]).is_err());
    let mut huge = artwork();
    huge["layers"][0]["shades"]=json!((0..60).map(|_|json!({"p":[[0,0],[1600,0],[1600,900],[0,900],[0,0]],"angle":65,"gap":2.5,"w":0.7,"a":0.2})).collect::<Vec<_>>());
    assert!(parse_artwork(&huge.to_string(), 0, 3, &[character()])
        .unwrap_err()
        .contains("6000"));
}
#[test]
fn pencil_batch_does_not_leave_a_two_second_49th_cue_as_an_independent_scene() {
    let cues = (0..49)
        .map(|i| Cue {
            start: f64::from(i) * 2.0,
            end: f64::from(i + 1) * 2.0,
            text: "台词。".into(),
        })
        .collect::<Vec<_>>();
    assert_eq!(batch_end(&cues, 0, 98.0), 49);
    assert_eq!(batch_end(&cues[..40], 0, 80.0), 40);
    let mut fast_tail = cues[..48].to_vec();
    fast_tail.extend((0..13).map(|i| Cue {
        start: 96.0 + f64::from(i) * 0.5,
        end: 96.0 + f64::from(i + 1) * 0.5,
        text: "短句。".into(),
    }));
    let end = batch_end(&fast_tail, 0, 102.5);
    assert!(end < 48);
    assert!(102.5 - fast_tail[end].start >= 12.0);
}
struct Responses(Mutex<Vec<String>>);
impl TextProvider for Responses {
    fn generate(
        &self,
        _: TextRequest,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<String, crate::ai::AiError>> + Send + '_>,
    > {
        Box::pin(async move {
            let mut r = self.0.lock().unwrap();
            if r.is_empty() {
                Err(crate::ai::AiError::new("test", "断线"))
            } else {
                Ok(r.remove(0))
            }
        })
    }
}
#[tokio::test]
async fn pencil_resume_uses_persisted_queue_and_frozen_heads_without_replanning() {
    let mut p = plan();
    let cast = character();
    let manifest=json!({"characters":[cast],"scenes":[{"first":0,"last":3,"title":"阅读","description":"保持人物，查阅账簿","characters":["reader"]}]}).to_string();
    let request = TextRequest {
        base_url: "http://localhost".into(),
        model: "test".into(),
        reasoning_effort: None,
        key: None,
        system: String::new(),
        user: String::new(),
    };
    let provider = Responses(Mutex::new(vec![manifest]));
    let mut snapshots = vec![];
    assert!(generate(&mut p, &provider, &request, |p| {
        snapshots.push(serde_json::to_string(p).unwrap());
        Ok(())
    })
    .await
    .is_err());
    assert_eq!(snapshots.len(), 1);
    let mut reopened: VideoPlan = serde_json::from_str(&snapshots[0]).unwrap();
    let frozen = serde_json::to_value(&reopened.pencil.as_ref().unwrap().characters).unwrap();
    let provider = Responses(Mutex::new(vec![artwork().to_string()]));
    generate(&mut reopened, &provider, &request, |_| Ok(()))
        .await
        .unwrap();
    assert_eq!(reopened.scenes.len(), 1);
    assert!(reopened.pencil.as_ref().unwrap().pending.is_empty());
    assert_eq!(
        serde_json::to_value(&reopened.pencil.as_ref().unwrap().characters).unwrap(),
        frozen
    );
    validate_plan(&reopened).unwrap();
}
