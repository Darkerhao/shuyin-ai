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
fn pencil_manifest_closes_character_objects_missing_their_final_brace() {
    let head = r#"{"mask":[],"strokes":[{"p":[[20,20],[80,80]],"w":1.2,"a":0.6}],"shades":[]}"#;
    let character = |id: &str, closed: bool| {
        format!(
            r#"{{"id":"{id}","name":"{id}","description":"{id}","heads":{{"front":{head},"side":{head}}}{}"#,
            if closed { "}" } else { "" }
        )
    };
    let scene = r#"{"first":0,"last":0,"title":"开场","description":"画面","characters":["a"]}"#;
    let valid = format!(
        r#"{{"characters":[{}],"scenes":[{scene}]}}"#,
        character("a", true)
    );
    let malformed = format!(
        r#"{{"characters":[{},{}],"scenes":[{scene}]}}"#,
        character("a", false),
        character("b", false)
    );
    assert_eq!(close_character_objects(&valid), valid);
    assert_eq!(
        parse_manifest(&valid, &plan(), 0, 1)
            .unwrap()
            .characters
            .len(),
        1
    );
    assert_eq!(
        parse_manifest(&malformed, &plan(), 0, 1)
            .unwrap()
            .characters
            .len(),
        2
    );
}

#[test]
fn pencil_geometry_rejects_foreign_character_and_degenerate_regions() {
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
fn pencil_artwork_reports_rejected_review_fields_and_json_positions() {
    let raw = artwork().to_string();
    let reviewed = format!(r#"{},"z_end_turn_review":""#, &raw[..raw.len() - 1]);
    let error = parse_artwork(&reviewed, 0, 3, &[character()]).unwrap_err();
    assert!(error.contains("z_end_turn_review"), "{error}");
    assert!(error.contains("line 1 column"), "{error}");
    let error = parse_artwork(r#"{"design":"unterminated"#, 0, 3, &[character()]).unwrap_err();
    assert!(error.contains("EOF"), "{error}");
    assert!(error.contains("line 1 column"), "{error}");
}

#[test]
fn pencil_artwork_closes_valid_regions_and_preserves_strokes_and_closed_curves() {
    let mut value = artwork();
    let closed = json!([[20, 20], [80, 20], [80, 80, 20, 80, 20, 20]]);
    value["layers"][0]["mask"] = json!([[[20, 20], [80, 20], [80, 80]], closed]);
    value["layers"][0]["shades"] =
        json!([{"p":[[20,20],[80,20,80,80]],"angle":65,"gap":5,"w":0.7,"a":0.2}]);
    let unnormalized: Artwork = serde_json::from_value(value.clone()).unwrap();
    assert!(unnormalized.validate(0, 3).unwrap_err().contains("闭合"));
    let parsed = parse_artwork(&value.to_string(), 0, 3, &[character()]).unwrap();
    let mut expected = value;
    expected["layers"][0]["mask"][0]
        .as_array_mut()
        .unwrap()
        .push(json!([20, 20]));
    expected["layers"][0]["shades"][0]["p"]
        .as_array_mut()
        .unwrap()
        .push(json!([20, 20]));
    let expected =
        serde_json::to_value(serde_json::from_value::<Artwork>(expected).unwrap()).unwrap();
    assert_eq!(serde_json::to_value(&parsed).unwrap(), expected);
    let saved = serde_json::to_string(&parsed).unwrap();
    assert_eq!(
        serde_json::to_value(parse_artwork(&saved, 0, 3, &[character()]).unwrap()).unwrap(),
        expected
    );
}

#[test]
fn pencil_manifest_closes_head_regions_before_freezing() {
    let mut cast = serde_json::to_value(character()).unwrap();
    for view in ["front", "side"] {
        cast["heads"][view]["mask"] = json!([[[20, 20], [80, 20], [80, 80]]]);
        cast["heads"][view]["shades"] =
            json!([{"p":[[20,20],[80,20],[80,80]],"angle":65,"gap":5,"w":0.7,"a":0.2}]);
    }
    let manifest = json!({"characters":[cast],"scenes":[{"first":0,"last":3,"title":"阅读","description":"查阅账簿","characters":["reader"]}]});
    let parsed = parse_manifest(&manifest.to_string(), &plan(), 0, 4).unwrap();
    for head in [
        &parsed.characters[0].heads.front,
        &parsed.characters[0].heads.side,
    ] {
        assert_eq!(head.mask[0].last().unwrap(), &head.mask[0][0]);
        assert_eq!(head.shades[0].p.last().unwrap(), &head.shades[0].p[0]);
    }
    parsed.characters[0].validate().unwrap();
}

#[test]
fn pencil_region_closure_rejects_invalid_geometry_and_counts_added_commands() {
    for p in [
        json!([]),
        json!([[0, 0], [100, 100]]),
        json!([[0, 0], [50, 50], [100, 100]]),
        json!([[0, 0], [0, 0], [0, 0]]),
        json!([[0, 0], [1700, 0], [100, 100]]),
        json!([[0, 0], [100, 0], [100, 100, 0]]),
    ] {
        let mut value = artwork();
        value["layers"][0]["mask"] = json!([p]);
        assert!(parse_artwork(&value.to_string(), 0, 3, &[character()]).is_err());
    }
    let mut value = artwork();
    value["layers"][0]["mask"] = json!([[[20, 20], [80, 20], [80, 80]]]);
    value["layers"][0]["strokes"][0]["p"] =
        json!((0..6497).map(|i| [20 + i % 2, 20]).collect::<Vec<_>>());
    assert!(parse_artwork(&value.to_string(), 0, 3, &[character()])
        .unwrap_err()
        .contains("预算"));
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
struct Responses {
    replies: Mutex<Vec<String>>,
    requests: Mutex<Vec<TextRequest>>,
}
impl Responses {
    fn new(replies: Vec<String>) -> Self {
        Self {
            replies: Mutex::new(replies),
            requests: Mutex::new(vec![]),
        }
    }
}
impl TextProvider for Responses {
    fn generate(
        &self,
        request: TextRequest,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<String, crate::ai::AiError>> + Send + '_>,
    > {
        Box::pin(async move {
            self.requests.lock().unwrap().push(request);
            let mut r = self.replies.lock().unwrap();
            if r.is_empty() {
                Err(crate::ai::AiError::new("test", "断线"))
            } else {
                Ok(r.remove(0))
            }
        })
    }
}
#[tokio::test]
async fn pencil_resume_preserves_saved_scene_and_only_closes_pending_regions() {
    let mut p = plan();
    p.scenes.push(Scene {
        first: 0,
        last: 1,
        kind: "focus".into(),
        title: "已完成".into(),
        environment: "plain".into(),
        framing: "reference".into(),
        labels: vec![],
        beats: vec![],
        image: None,
        painting: None,
        artwork: Some(parse_artwork(&artwork().to_string(), 0, 1, &[character()]).unwrap()),
    });
    p.pencil = Some(PencilPlan {
        characters: vec![character()],
        pending: vec![PlannedScene {
            first: 2,
            last: 3,
            title: "继续阅读".into(),
            description: "增补账簿".into(),
            characters: vec!["reader".into()],
        }],
    });
    let original = serde_json::to_value(&p).unwrap();
    let request = TextRequest {
        base_url: "http://localhost".into(),
        model: "test".into(),
        reasoning_effort: None,
        key: None,
        system: String::new(),
        user: String::new(),
        response_format: None,
    };
    let mut value = artwork();
    value["layers"][0]["cue"] = json!(2);
    let raw = value.to_string();
    let reviewed = format!(r#"{},"z_end_turn_review":""#, &raw[..raw.len() - 1]);
    let provider = Responses::new(vec![reviewed]);
    assert!(generate(&mut p, &provider, &request, |_| panic!(
        "invalid JSON must not be saved"
    ))
    .await
    .unwrap_err()
    .contains("z_end_turn_review"));
    assert_eq!(serde_json::to_value(&p).unwrap(), original);
    value["layers"][0]["mask"] = json!([[[20, 20], [80, 20]]]);
    let provider = Responses::new(vec![value.to_string()]);
    assert!(generate(&mut p, &provider, &request, |_| panic!(
        "invalid artwork must not be saved"
    ))
    .await
    .is_err());
    assert_eq!(serde_json::to_value(&p).unwrap(), original);

    value["layers"][0]["mask"] = json!([[[20, 20], [80, 20], [80, 80]]]);
    let provider = Responses::new(vec![value.to_string()]);
    let mut snapshots = vec![];
    generate(&mut p, &provider, &request, |p| {
        snapshots.push(serde_json::to_string(p).unwrap());
        Ok(())
    })
    .await
    .unwrap();
    assert_eq!(snapshots.len(), 1);
    let reopened: VideoPlan = serde_json::from_str(&snapshots[0]).unwrap();
    let saved = serde_json::to_value(&reopened).unwrap();
    assert_eq!(saved["cues"], original["cues"]);
    assert_eq!(saved["scenes"][0], original["scenes"][0]);
    assert_eq!(
        saved["pencil"]["characters"],
        original["pencil"]["characters"]
    );
    assert_eq!(reopened.scenes.len(), 2);
    assert!(reopened.pencil.as_ref().unwrap().pending.is_empty());
    validate_plan(&reopened).unwrap();
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
        response_format: None,
    };
    let provider = Responses::new(vec![manifest]);
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
    let provider = Responses::new(vec![artwork().to_string()]);
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

#[tokio::test]
async fn pencil_initial_and_resumed_drawings_request_the_same_strict_schema() {
    let mut p = plan();
    let manifest = json!({"characters":[character()],"scenes":[{"first":0,"last":3,"title":"阅读","description":"查阅账簿","characters":["reader"]}]}).to_string();
    let request = TextRequest {
        base_url: "http://localhost".into(),
        model: "test".into(),
        reasoning_effort: None,
        response_format: None,
        key: None,
        system: String::new(),
        user: String::new(),
    };
    let provider = Responses::new(vec![manifest]);
    assert!(generate(&mut p, &provider, &request, |_| Ok(()))
        .await
        .is_err());
    let initial = provider.requests.lock().unwrap()[1]
        .response_format
        .clone()
        .unwrap();
    assert_eq!(initial["type"], "json_schema");
    assert_eq!(initial["json_schema"]["strict"], true);
    assert_eq!(
        initial["json_schema"]["schema"]["additionalProperties"],
        false
    );
    let provider = Responses::new(vec![artwork().to_string()]);
    generate(&mut p, &provider, &request, |_| Ok(()))
        .await
        .unwrap();
    assert_eq!(
        provider.requests.lock().unwrap()[0]
            .response_format
            .as_ref(),
        Some(&initial)
    );
    validate_plan(&p).unwrap();
}
