use super::{AiError, OpenAiCompatible};
use base64::{engine::general_purpose::STANDARD, Engine};
use std::{future::Future, io::Cursor, pin::Pin, time::Duration};

pub const PROMPT_VERSION: &str = "cover-background-v1";
pub const SYSTEM_PROMPT: &str = include_str!("../../../prompts/cover-background-v1.md");
pub const TIMEOUT: Duration = Duration::from_secs(180);
pub const MAX_IMAGE_BYTES: usize = 15 * 1024 * 1024;

#[cfg(test)]
pub(crate) fn test_png() -> Vec<u8> {
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, 4, 4);
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(&[60; 48]).unwrap();
        writer.finish().unwrap();
    }
    bytes
}

pub struct ImageRequest {
    pub base_url: String,
    pub model: String,
    pub key: Option<String>,
    pub prompt: String,
}
pub trait ImageProvider {
    fn generate_image(
        &self,
        request: ImageRequest,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, AiError>> + Send + '_>>;
}

// Decode all pixels before a remote response can become a project asset.
pub fn validate_image(bytes: &[u8]) -> Result<(), AiError> {
    let invalid = || {
        AiError::new(
            "invalid_image",
            "图片服务未返回有效的静态 PNG（最大 15 MB、4000 万像素），当前封面保留。",
        )
    };
    if bytes.is_empty() || bytes.len() > MAX_IMAGE_BYTES {
        return Err(invalid());
    }
    let decoder = png::Decoder::new_with_limits(
        Cursor::new(bytes),
        png::Limits {
            bytes: 160 * 1024 * 1024,
        },
    );
    let mut reader = decoder.read_info().map_err(|_| invalid())?;
    let info = reader.info();
    if u64::from(info.width) * u64::from(info.height) > 40_000_000
        || info.animation_control.is_some()
    {
        return Err(invalid());
    }
    let size = reader
        .output_buffer_size()
        .filter(|n| *n <= 160 * 1024 * 1024)
        .ok_or_else(invalid)?;
    reader
        .next_frame(&mut vec![0; size])
        .map_err(|_| invalid())?;
    reader.finish().map_err(|_| invalid())?;
    Ok(())
}

impl ImageProvider for OpenAiCompatible {
    fn generate_image(
        &self,
        request: ImageRequest,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, AiError>> + Send + '_>> {
        Box::pin(async move {
            let data = self.post_json(&request.base_url, "images/generations", request.key,
                serde_json::json!({"model":request.model,"prompt":request.prompt,"n":1,"size":"1024x1024","output_format":"png"}),
                MAX_IMAGE_BYTES * 4 / 3 + 1024 * 1024).await?;
            let encoded = data["data"]
                .as_array()
                .filter(|a| a.len() == 1)
                .and_then(|a| a[0]["b64_json"].as_str())
                .ok_or_else(|| {
                    AiError::new(
                        "invalid_response",
                        "图片服务需返回一张 base64 PNG；请配置兼容 GPT Image 的图片模型。",
                    )
                })?;
            let bytes = STANDARD.decode(encoded).map_err(|_| {
                AiError::new("invalid_response", "图片数据编码无效，当前封面保留。")
            })?;
            validate_image(&bytes)?;
            Ok(bytes)
        })
    }
}
