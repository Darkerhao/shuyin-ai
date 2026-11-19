//! Persisted pencil artwork. Models supply numbers; characters own frozen head geometry.
use super::{Cue, Scene, VideoPlan};
use crate::ai::{TextProvider, TextRequest};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, time::Duration};

pub const TIMEOUT: Duration = Duration::from_secs(1200);
pub const PLAN_PROMPT: &str = include_str!("../../../prompts/video-pencil-plan-v2.md");
pub const DRAW_PROMPT: &str = include_str!("../../../prompts/video-pencil-draw-v3.md");

pub fn drawing_response_format() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../../../prompts/video-pencil-draw-v3.schema.json"
    ))
    .expect("bundled pencil artwork schema must be valid JSON")
}
pub type Path = Vec<Vec<f64>>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Stroke {
    pub p: Path,
    pub w: f64,
    pub a: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Shade {
    pub p: Path,
    pub angle: f64,
    pub gap: f64,
    pub w: f64,
    pub a: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Text {
    pub x: f64,
    pub y: f64,
    pub text: String,
    pub size: f64,
    pub a: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Head {
    pub mask: Vec<Path>,
    pub strokes: Vec<Stroke>,
    pub shades: Vec<Shade>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Heads {
    pub front: Head,
    pub side: Head,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Character {
    pub id: String,
    pub name: String,
    pub description: String,
    pub heads: Heads,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Portrait {
    pub character_id: String,
    pub view: String,
    pub x: f64,
    pub y: f64,
    pub scale: f64,
    pub flip: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Layer {
    pub cue: usize,
    pub name: String,
    pub mask: Vec<Path>,
    pub strokes: Vec<Stroke>,
    pub shades: Vec<Shade>,
    pub texts: Vec<Text>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub portrait: Option<Portrait>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Artwork {
    pub design: String,
    pub layers: Vec<Layer>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlannedScene {
    pub first: usize,
    pub last: usize,
    pub title: String,
    pub description: String,
    pub characters: Vec<String>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PencilPlan {
    pub characters: Vec<Character>,
    pub pending: Vec<PlannedScene>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    characters: Vec<Character>,
    scenes: Vec<PlannedScene>,
}

fn number(v: f64, min: f64, max: f64) -> bool {
    v.is_finite() && (min..=max).contains(&v)
}
fn text(s: &str, max: usize) -> bool {
    !s.trim().is_empty() && s.chars().count() <= max && !s.chars().any(char::is_control)
}
fn path(p: &Path, width: f64, height: f64, closed: bool) -> Result<usize, String> {
    if p.len() < 2
        || p[0].len() != 2
        || p.iter().any(|c| {
            ![2, 4, 6].contains(&c.len())
                || c.iter()
                    .enumerate()
                    .any(|(i, v)| !number(*v, 0.0, if i % 2 == 0 { width } else { height }))
        })
    {
        return Err("精绘路径必须是画布内有限数值，首项为起点，其余为直线或贝塞尔控制点。".into());
    }
    if p.iter()
        .skip(1)
        .all(|c| c.chunks(2).all(|xy| xy == p[0].as_slice()))
    {
        return Err("精绘路径不能为零长度。".into());
    }
    if closed && p.last().unwrap()[p.last().unwrap().len() - 2..] != p[0] {
        return Err("精绘遮挡与阴影区域必须闭合。".into());
    }
    Ok(p.len())
}
fn geometry(
    mask: &[Path],
    strokes: &[Stroke],
    shades: &[Shade],
    w: f64,
    h: f64,
) -> Result<usize, String> {
    if mask.len() > 24 || strokes.len() > 600 || shades.len() > 60 {
        return Err("精绘图层资源超限。".into());
    }
    let mut commands = 0;
    for p in mask {
        commands += path(p, w, h, true)?;
    }
    for s in strokes {
        if !number(s.w, 0.4, 4.5) || !number(s.a, 0.05, 0.9) {
            return Err("精绘线宽或深浅无效。".into());
        }
        commands += path(&s.p, w, h, false)?;
    }
    for s in shades {
        if !number(s.w, 0.4, 4.5)
            || !number(s.a, 0.05, 0.9)
            || !number(s.angle, 0.0, 180.0)
            || !number(s.gap, 2.5, 12.0)
        {
            return Err("精绘排线参数无效。".into());
        }
        commands += path(&s.p, w, h, true)?;
    }
    Ok(commands)
}
/// Regions include the edge back to their start. Canonicalize model input once;
/// stored artwork and the renderer still require explicitly closed paths.
fn close_regions(
    mask: &mut [Path],
    shades: &mut [Shade],
    width: f64,
    height: f64,
) -> Result<(), String> {
    for p in mask.iter_mut().chain(shades.iter_mut().map(|s| &mut s.p)) {
        path(p, width, height, false)?;
        if p.last().unwrap()[p.last().unwrap().len() - 2..] == p[0] {
            continue;
        }
        let mut points = p.iter().flat_map(|c| c.chunks(2));
        let a = points.next().unwrap();
        let b = points.find(|p| *p != a).unwrap();
        if points.all(|c| (b[0] - a[0]) * (c[1] - a[1]) == (b[1] - a[1]) * (c[0] - a[0])) {
            return Err("精绘遮挡与阴影区域不能退化为直线。".into());
        }
        p.push(p[0].clone());
    }
    Ok(())
}
fn sampled(commands: &Path) -> Vec<[f64; 2]> {
    let mut a = [commands[0][0], commands[0][1]];
    let mut result = vec![a];
    for c in commands.iter().skip(1) {
        let d = [c[c.len() - 2], c[c.len() - 1]];
        if c.len() == 2 {
            result.push(d);
            a = d;
            continue;
        }
        let b = [c[0], c[1]];
        let n = (((d[0] - a[0]).hypot(d[1] - a[1]) + (b[0] - a[0]).hypot(b[1] - a[1])) / 5.0)
            .ceil()
            .clamp(8.0, 60.0) as usize;
        for i in 1..=n {
            let t = i as f64 / n as f64;
            let v = 1.0 - t;
            result.push(if c.len() == 4 {
                [
                    v * v * a[0] + 2.0 * v * t * b[0] + t * t * d[0],
                    v * v * a[1] + 2.0 * v * t * b[1] + t * t * d[1],
                ]
            } else {
                [
                    v * v * v * a[0]
                        + 3.0 * v * v * t * b[0]
                        + 3.0 * v * t * t * c[2]
                        + t * t * t * d[0],
                    v * v * v * a[1]
                        + 3.0 * v * v * t * b[1]
                        + 3.0 * v * t * t * c[3]
                        + t * t * t * d[1],
                ]
            });
        }
        a = d;
    }
    result
}
fn expanded(strokes: &[Stroke], shades: &[Shade], scale: f64, flip: bool) -> usize {
    let mut count = strokes.len();
    for shade in shades {
        let angle = shade.angle.to_radians();
        let (uy, ux) = angle.sin_cos();
        let vx = -uy;
        let vy = ux;
        let p: Vec<_> = sampled(&shade.p)
            .into_iter()
            .map(|[x, y]| {
                let x = if flip { 200.0 - x } else { x };
                let x = x * scale;
                let y = y * scale;
                [x * ux + y * uy, x * vx + y * vy]
            })
            .collect();
        let low = p.iter().map(|p| p[1]).fold(f64::INFINITY, f64::min);
        let high = p.iter().map(|p| p[1]).fold(f64::NEG_INFINITY, f64::max);
        let mut v = low + shade.gap * 0.4;
        while v < high {
            let mut hits = vec![];
            for pair in p.windows(2) {
                let a = pair[0];
                let b = pair[1];
                if (a[1] <= v && b[1] > v) || (b[1] <= v && a[1] > v) {
                    hits.push(a[0] + (v - a[1]) * (b[0] - a[0]) / (b[1] - a[1]));
                }
            }
            hits.sort_by(f64::total_cmp);
            count += hits
                .as_chunks::<2>()
                .0
                .iter()
                .filter(|p| p[1] - p[0] >= 0.8)
                .count();
            if count > 6000 {
                return count;
            }
            v += shade.gap;
        }
    }
    count
}
impl Character {
    pub fn validate(&self) -> Result<(), String> {
        if !text(&self.id, 64)
            || !self
                .id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_')
            || !text(&self.name, 80)
            || !text(&self.description, 1600)
        {
            return Err("人物标识、名称或设定无效。".into());
        }
        for head in [&self.heads.front, &self.heads.side] {
            if head.strokes.is_empty()
                || head.strokes.len() > 100
                || geometry(&head.mask, &head.strokes, &head.shades, 200.0, 240.0)? > 1600
            {
                return Err("人物头像须为200×240内的完整数值几何。".into());
            }
        }
        Ok(())
    }
}
impl Artwork {
    pub fn validate(&self, first: usize, last: usize) -> Result<(), String> {
        if !text(&self.design, 6000) || self.layers.is_empty() || self.layers.len() > 24 {
            return Err("精绘场景设计或图层数无效。".into());
        }
        let (mut previous, mut commands, mut strokes, mut shades, mut texts) = (first, 0, 0, 0, 0);
        for (i, l) in self.layers.iter().enumerate() {
            if !text(&l.name, 80) || l.cue < previous || l.cue > last || (i == 0 && l.cue != first)
            {
                return Err("精绘图层须按本镜字幕顺序出现，首层从本镜第一句开始。".into());
            }
            previous = l.cue;
            commands += geometry(&l.mask, &l.strokes, &l.shades, 1600.0, 900.0)?;
            strokes += l.strokes.len();
            shades += l.shades.len();
            texts += l.texts.len();
            for t in &l.texts {
                if !text(&t.text, 40)
                    || !number(t.x, 0.0, 1600.0)
                    || !number(t.y, 0.0, 900.0)
                    || !number(t.size, 16.0, 30.0)
                    || !number(t.a, 0.15, 0.9)
                {
                    return Err("精绘标签内容或位置无效。".into());
                }
            }
            if let Some(p) = &l.portrait {
                if !text(&p.character_id, 64)
                    || !matches!(p.view.as_str(), "front" | "side")
                    || !number(p.scale, 0.15, 3.0)
                    || !number(p.x, 0.0, 1600.0 - 200.0 * p.scale)
                    || !number(p.y, 0.0, 900.0 - 240.0 * p.scale)
                {
                    return Err("人物头像引用、视角或位置无效。".into());
                }
            }
        }
        if commands > 6500 || strokes > 600 || shades > 60 || texts > 30 {
            return Err("整镜精绘路径、阴影或标签超过预算。".into());
        }
        Ok(())
    }
    pub fn validate_characters(&self, cast: &[Character]) -> Result<(), String> {
        let mut count = 0;
        let mut seen = BTreeSet::new();
        for l in &self.layers {
            count += expanded(&l.strokes, &l.shades, 1.0, false);
        }
        for p in self.layers.iter().filter_map(|l| l.portrait.as_ref()) {
            if let Some(c) = cast.iter().find(|c| c.id == p.character_id) {
                if !seen.insert(&p.character_id) {
                    return Err("同一镜头不能重复引用同一人物头像。".into());
                }
                let head = if p.view == "front" {
                    &c.heads.front
                } else {
                    &c.heads.side
                };
                count += expanded(&head.strokes, &head.shades, p.scale, p.flip);
            } else {
                return Err(format!(
                    "人物头像 {} 不在本任务冻结设定中。",
                    p.character_id
                ));
            }
        }
        if count > 6000 {
            return Err("精绘排线展开后超过6000笔预算，请缩小阴影区域或增大排线间距。".into());
        }
        Ok(())
    }
    pub fn character_ids(&self) -> impl Iterator<Item = &str> {
        self.layers
            .iter()
            .filter_map(|l| l.portrait.as_ref().map(|p| p.character_id.as_str()))
    }

    pub fn validate_cast(&self, expected: &[String]) -> Result<(), String> {
        let actual: BTreeSet<_> = self.character_ids().collect();
        let expected: BTreeSet<_> = expected.iter().map(String::as_str).collect();
        if actual != expected {
            return Err(
                "画稿人物与本镜规划不一致，须逐一引用冻结头像，不能遗漏或新增人物。".into(),
            );
        }
        Ok(())
    }
}
pub fn batch_end(cues: &[Cue], first: usize, duration: f64) -> usize {
    let mut end = (first + 48).min(cues.len());
    if end < cues.len() {
        end = ((first + 36)..end)
            .rev()
            .find(|&i| {
                matches!(
                    cues[i].text.trim().chars().last(),
                    Some('。' | '！' | '？' | '!' | '?' | '；' | ';')
                )
            })
            .map_or(end, |i| i + 1);
        if cues.len() - end <= 12 && duration - cues[end].start < 12.0 {
            end = cues.len();
        } else {
            while end > first && duration - cues[end].start < 12.0 {
                end -= 1;
            }
            if end == first {
                end = cues.len();
            }
        }
    }
    end
}
pub fn validate_plan(plan: &VideoPlan) -> Result<(), String> {
    if let Some(p) = &plan.pencil {
        if p.characters.len() > 64 {
            return Err("人物设定数量超过预算。".into());
        }
        let mut ids = BTreeSet::new();
        for c in &p.characters {
            c.validate()?;
            if !ids.insert(&c.id) {
                return Err("人物ID重复。".into());
            }
        }
    }
    let cast = plan
        .pencil
        .as_ref()
        .map_or(&[][..], |p| p.characters.as_slice());
    for s in &plan.scenes {
        if let Some(a) = &s.artwork {
            if plan.theme != "ink" {
                return Err("精绘画稿仅适用于铅笔素描主题。".into());
            }
            a.validate(s.first, s.last)?;
            a.validate_characters(cast)?;
        }
    }
    Ok(())
}
pub fn merge_short(scenes: &mut Vec<PlannedScene>, cues: &[Cue], duration: f64) {
    let mut i = 0;
    while scenes.len() > 1 && i < scenes.len() {
        let end = scenes.get(i + 1).map_or(duration, |s| cues[s.first].start);
        if end - cues[scenes[i].first].start >= 12.0 {
            i += 1;
            continue;
        }
        let at = if i == 0 { 0 } else { i - 1 };
        let right = scenes.remove(at + 1);
        let left = &mut scenes[at];
        left.description = format!(
            "{}\n随后在同一画面增补（字幕{}—{}）：{}。已有主体保留，不重新画已出现人物。",
            left.description, right.first, right.last, right.description
        );
        left.last = right.last;
        for id in right.characters {
            if !left.characters.contains(&id) {
                left.characters.push(id);
            }
        }
        i = at;
    }
}
fn parse_manifest(
    raw: &str,
    plan: &VideoPlan,
    first: usize,
    end: usize,
) -> Result<PencilPlan, String> {
    let mut manifest: Manifest = serde_json::from_str(&close_character_objects(raw))
        .map_err(|e| format!("精绘规划JSON结构无效（{e}），已保留原任务。"))?;
    let mut result = plan.pencil.clone().unwrap_or_default();
    for mut c in manifest.characters {
        for head in [&mut c.heads.front, &mut c.heads.side] {
            close_regions(&mut head.mask, &mut head.shades, 200.0, 240.0)?;
        }
        c.validate()?;
        if result.characters.iter().any(|old| old.id == c.id) {
            return Err("规划不能重定义已经冻结的人物。".into());
        }
        result.characters.push(c);
    }
    if result.characters.len() > 64 || manifest.scenes.len() > 16 {
        return Err("精绘规划的人物或镜头数超过预算。".into());
    }
    let mut next = first;
    for s in &manifest.scenes {
        if s.first != next
            || s.last < s.first
            || s.last >= end
            || !text(&s.title, 20)
            || !text(&s.description, 1800)
            || s.characters
                .iter()
                .any(|id| !result.characters.iter().any(|c| &c.id == id))
        {
            return Err("精绘规划的字幕覆盖、人物引用或画面设计无效。".into());
        }
        next = s.last + 1;
    }
    if next != end || manifest.scenes.is_empty() {
        return Err("精绘规划未完整覆盖本批字幕。".into());
    }
    let batch_end = plan.cues.get(end).map_or(plan.duration, |c| c.start);
    merge_short(&mut manifest.scenes, &plan.cues, batch_end);
    result.pending = manifest.scenes;
    Ok(result)
}

/// Models consistently omit the brace closing a character object after `heads`.
/// Restore it only at the unambiguous boundary between sibling characters or
/// before the end of the characters array; valid JSON is unchanged.
fn close_character_objects(raw: &str) -> String {
    let mut output = String::with_capacity(raw.len() + 8);
    let mut stack = Vec::new();
    let mut string = false;
    let mut escaped = false;
    for (index, ch) in raw.char_indices() {
        if string {
            output.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                string = false;
            }
            continue;
        }
        if ch == '"' {
            string = true;
        } else if ch == '{' || ch == '[' {
            stack.push(ch);
        } else if ch == '}' || ch == ']' {
            if ch == ']' && matches!(stack.as_slice(), [.., '[', '{']) {
                output.push('}');
                stack.pop();
            }
            stack.pop();
        } else if ch == ','
            && matches!(stack.as_slice(), [.., '[', '{'])
            && raw[index..].starts_with(",{\"id\":\"")
        {
            output.push('}');
            stack.pop();
        }
        output.push(ch);
    }
    output
}
pub fn drawing_input(plan: &VideoPlan, cues: &[Cue], scene: &PlannedScene) -> String {
    let cast = plan
        .pencil
        .as_ref()
        .map_or(&[][..], |p| p.characters.as_slice());
    let start = cues[scene.first].start;
    let end = cues.get(scene.last + 1).map_or(plan.duration, |c| c.start);
    serde_json::json!({"book":plan.title,"scene":scene,"duration":end-start,
        "cues":(scene.first..=scene.last).map(|i|serde_json::json!({"index":i,"start":cues[i].start,"end":cues[i].end,"text":cues[i].text})).collect::<Vec<_>>(),
        "characters":cast.iter().filter(|c|scene.characters.contains(&c.id)).map(|c|serde_json::json!({"id":c.id,"name":c.name,"description":c.description,"headCanvas":[200,240],"views":["front","side"],"neckAnchor":[100,235]})).collect::<Vec<_>>()
    }).to_string()
}
pub fn parse_artwork(
    raw: &str,
    first: usize,
    last: usize,
    cast: &[Character],
) -> Result<Artwork, String> {
    let mut artwork: Artwork = serde_json::from_str(raw)
        .map_err(|e| format!("精绘画稿JSON结构无效（{e}），已保留原任务。"))?;
    for layer in &mut artwork.layers {
        close_regions(&mut layer.mask, &mut layer.shades, 1600.0, 900.0)?;
    }
    artwork.validate(first, last)?;
    artwork.validate_characters(cast)?;
    Ok(artwork)
}
pub async fn generate(
    plan: &mut VideoPlan,
    provider: &impl TextProvider,
    request: &TextRequest,
    mut save: impl FnMut(&VideoPlan) -> Result<(), String>,
) -> Result<(), String> {
    loop {
        let first = plan.scenes.last().map_or(0, |s| s.last + 1);
        if first >= plan.cues.len() {
            return Ok(());
        }
        if plan.pencil.as_ref().is_none_or(|p| p.pending.is_empty()) {
            let end = batch_end(&plan.cues, first, plan.duration);
            let mut req = request.clone();
            req.system = PLAN_PROMPT.into();
            req.user=serde_json::json!({"book":plan.title,"first":first,"end":end,"duration":plan.duration,
                "cues":(first..end).map(|i|serde_json::json!({"index":i,"start":plan.cues[i].start,"end":plan.cues[i].end,"text":plan.cues[i].text})).collect::<Vec<_>>(),
                "existingCharacters":plan.pencil.as_ref().map(|p|p.characters.iter().map(|c|serde_json::json!({"id":c.id,"name":c.name,"description":c.description})).collect::<Vec<_>>()).unwrap_or_default()
            }).to_string();
            let raw = provider
                .generate(req)
                .await
                .map_err(|e| super::model_failure(e.message))?;
            plan.pencil =
                Some(parse_manifest(&raw, plan, first, end).map_err(super::model_failure)?);
            save(plan)?;
        }
        let scene = plan.pencil.as_ref().unwrap().pending[0].clone();
        if scene.first != first {
            return Err("精绘待办与已保存字幕范围不一致，请重新生成分镜。".into());
        }
        let mut req = request.clone();
        req.system = DRAW_PROMPT.into();
        req.user = drawing_input(plan, &plan.cues, &scene);
        req.response_format = Some(drawing_response_format());
        let raw = provider
            .generate(req)
            .await
            .map_err(|e| super::model_failure(e.message))?;
        let artwork = parse_artwork(
            &raw,
            scene.first,
            scene.last,
            &plan.pencil.as_ref().unwrap().characters,
        )
        .map_err(super::model_failure)?;
        artwork
            .validate_cast(&scene.characters)
            .map_err(super::model_failure)?;
        plan.scenes.push(Scene {
            first: scene.first,
            last: scene.last,
            kind: "focus".into(),
            title: scene.title,
            environment: "plain".into(),
            framing: "reference".into(),
            labels: vec![],
            beats: vec![],
            image: None,
            painting: None,
            artwork: Some(artwork),
        });
        plan.pencil.as_mut().unwrap().pending.remove(0);
        save(plan)?;
    }
}

#[cfg(test)]
mod tests;
