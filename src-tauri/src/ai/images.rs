use super::{AiError, OpenAiCompatible};
use base64::{engine::general_purpose::STANDARD, Engine};
use image::{ImageFormat, ImageReader, Limits};
use std::{future::Future, io::Cursor, pin::Pin, time::Duration};

pub const PROMPT_VERSION: &str = "cover-background-v2";
pub const SYSTEM_PROMPT: &str = include_str!("../../../prompts/cover-background-v2.md");
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
pub(crate) fn test_image(format: ImageFormat, width: u32, height: u32) -> Vec<u8> {
    let image = image::RgbImage::from_pixel(width, height, image::Rgb([60, 90, 120]));
    let mut bytes = Vec::new();
    image
        .write_to(&mut Cursor::new(&mut bytes), format)
        .unwrap();
    bytes
}

#[cfg(test)]
pub(crate) fn test_jpeg() -> Vec<u8> {
    test_image(ImageFormat::Jpeg, 4, 4)
}

#[cfg(test)]
pub(crate) fn test_webp() -> Vec<u8> {
    test_image(ImageFormat::WebP, 4, 4)
}

pub struct ImageRequest {
    pub base_url: String,
    pub model: String,
    pub key: Option<String>,
    pub prompt: String,
    pub width: u32,
    pub height: u32,
}
pub trait ImageProvider {
    fn generate_image(
        &self,
        request: ImageRequest,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, AiError>> + Send + '_>>;
}

// Decode all pixels before a remote response can become a project asset.
pub fn validate_image(bytes: &[u8]) -> Result<(&'static str, u32, u32), AiError> {
    let invalid = || {
        AiError::new(
            "invalid_image",
            "图片服务未返回有效的静态 PNG、JPEG 或 WebP（最大 15 MB、4000 万像素），已保存内容保留。",
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
    Ok((extension, decoded.width(), decoded.height()))
}

impl ImageProvider for OpenAiCompatible<'_> {
    fn generate_image(
        &self,
        request: ImageRequest,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, AiError>> + Send + '_>> {
        Box::pin(async move {
            let mut log =
                super::diagnostics::Call::start(self.diagnostics.as_ref(), &request.model)?;
            let result=async {
            let size = format!("{}x{}", request.width, request.height);
            let data = self.post_json(&request.base_url, "images/generations", request.key,
                serde_json::json!({"model":request.model,"prompt":request.prompt,"n":1,"size":size,"response_format":"b64_json"}),
                MAX_IMAGE_BYTES * 4 / 3 + 1024 * 1024, &mut log).await?;
            log.usage(&data["usage"]);
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
                AiError::new("invalid_response", "图片数据编码无效，已保存内容保留。")
            })?;
            let (_, width, height) = validate_image(&bytes)?;
            if (width, height) != (request.width, request.height) {
                return Err(AiError::new("image_size_mismatch", &format!(
                    "图片服务返回 {width}×{height}，与请求的 {}×{} 不一致。请使用支持该尺寸的图片模型，原背景和编辑内容已保留。",
                    request.width, request.height
                )));
            }
            Ok(bytes)
            }.await;
            log.finish(&result)?;
            result
        })
    }
}
