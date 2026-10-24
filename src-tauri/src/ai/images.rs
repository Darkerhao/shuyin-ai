use super::{AiError, OpenAiCompatible};
use base64::{engine::general_purpose::STANDARD, Engine};
use image::{ImageFormat, ImageReader, Limits};
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
#[cfg(test)]
fn test_image(format: ImageFormat) -> Vec<u8> {
    let image = image::RgbImage::from_pixel(4, 4, image::Rgb([60, 90, 120]));
    let mut bytes = Vec::new();
    image
        .write_to(&mut Cursor::new(&mut bytes), format)
        .unwrap();
    bytes
}

#[cfg(test)]
pub(crate) fn test_jpeg() -> Vec<u8> {
    test_image(ImageFormat::Jpeg)
}

#[cfg(test)]
pub(crate) fn test_webp() -> Vec<u8> {
    test_image(ImageFormat::WebP)
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
pub fn validate_image(bytes: &[u8]) -> Result<&'static str, AiError> {
    let invalid = || {
        AiError::new(
            "invalid_image",
            "图片服务未返回有效的静态 PNG、JPEG 或 WebP（最大 15 MB、4000 万像素），当前封面保留。",
        )
    };
    if bytes.is_empty() || bytes.len() > MAX_IMAGE_BYTES {
        return Err(invalid());
    }
    let format = image::guess_format(bytes).map_err(|_| invalid())?;
    let extension = match format {
        ImageFormat::Png if !bytes.windows(4).any(|w| w == b"acTL") => "png",
        ImageFormat::Jpeg => "jpg",
        ImageFormat::WebP if !bytes.windows(4).any(|w| w == b"ANIM" || w == b"ANMF") => "webp",
        _ => return Err(invalid()),
    };
    let mut reader = ImageReader::with_format(Cursor::new(bytes), format);
    let mut limits = Limits::default();
    limits.max_alloc = Some(160 * 1024 * 1024);
    reader.limits(limits);
    let decoded = reader.decode().map_err(|_| invalid())?;
    if u64::from(decoded.width()) * u64::from(decoded.height()) > 40_000_000 {
        return Err(invalid());
    }
    Ok(extension)
}

impl ImageProvider for OpenAiCompatible {
    fn generate_image(
        &self,
        request: ImageRequest,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, AiError>> + Send + '_>> {
        Box::pin(async move {
            let data = self.post_json(&request.base_url, "images/generations", request.key,
                serde_json::json!({"model":request.model,"prompt":request.prompt,"n":1,"size":"1024x1024","response_format":"b64_json"}),
                MAX_IMAGE_BYTES * 4 / 3 + 1024 * 1024).await?;
            let encoded = data["data"]
                .as_array()
                .filter(|a| a.len() == 1)
                .and_then(|a| a[0]["b64_json"].as_str())
                .ok_or_else(|| {
                    AiError::new(
                        "invalid_response",
                        "图片服务需返回一张 base64 PNG、JPEG 或 WebP；请检查图片模型配置。",
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
