use serde::{Deserialize, Serialize};

/// Numeric polylines only. The host constructs drawing commands; model code is never executed.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Drawing {
    pub strokes: Vec<Vec<[f64; 2]>>,
}

impl Drawing {
    pub fn validate(&self) -> Result<usize, String> {
        let limits = &super::visual_catalog().limits;
        if self.strokes.is_empty() || self.strokes.len() > limits.max_drawing_strokes {
            return Err(format!(
                "drawing.strokes 须包含有效笔画，整镜最多 {} 笔。",
                limits.max_drawing_strokes
            ));
        }
        let mut points = 0;
        for (index, stroke) in self.strokes.iter().enumerate() {
            points += stroke.len();
            if points > limits.max_drawing_points {
                return Err(format!(
                    "drawing 笔画点数超过 {}，请简化线条。",
                    limits.max_drawing_points
                ));
            }
            if stroke.len() < 2 || !stroke.windows(2).any(|pair| pair[0] != pair[1]) {
                return Err(format!("drawing 第 {} 笔须有至少两个不同的点。", index + 1));
            }
            if stroke.iter().any(|[x, y]| {
                !x.is_finite()
                    || !y.is_finite()
                    || !(0.0..=limits.drawing_width).contains(x)
                    || !(0.0..=limits.drawing_height).contains(y)
            }) {
                return Err(format!(
                    "drawing 第 {} 笔坐标无效：x 须为 0—{}，y 须为 0—{} 的有限数值。",
                    index + 1,
                    limits.drawing_width,
                    limits.drawing_height
                ));
            }
        }
        Ok(points)
    }
}
