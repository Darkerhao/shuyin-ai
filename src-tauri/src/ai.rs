use serde::{Deserialize, Serialize};
use std::{collections::HashMap, future::Future, pin::Pin, sync::Mutex, time::Duration};
use tokio::sync::watch;

pub mod images;
pub mod publishing;
pub mod revision;
pub const PROMPT_VERSION: &str = "publishing-v2";
pub const SYSTEM_PROMPT: &str = include_str!("../../prompts/publishing-v2.md");
pub const TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "format", rename_all = "lowercase", deny_unknown_fields)]
pub enum WritingContent {
    Markdown {
        text: String,
    },
    List {
        items: Vec<String>,
    },
    Publishing {
        title: String,
        description: String,
        #[serde(rename = "audioPrompt")]
        audio_prompt: String,
        #[serde(rename = "coverTitle", default)]
        cover_title: String,
        #[serde(rename = "coverSubtitle", default)]
        cover_subtitle: String,
        #[serde(rename = "coverPromptLandscape", default)]
        cover_prompt_landscape: String,
        #[serde(rename = "coverPromptPortrait", default)]
        cover_prompt_portrait: String,
    },
}
impl WritingContent {
    pub fn validate(&self, kind: &str, count: Option<usize>) -> Result<(), String> {
        let valid = match (kind, self) {
            (
                "publishing",
                Self::Publishing {
                    title,
                    description,
                    audio_prompt,
                    cover_title,
                    cover_subtitle,
                    cover_prompt_landscape,
                    cover_prompt_portrait,
                },
            ) => {
                !title.trim().is_empty()
                    && title.chars().count() <= 30
                    && !description.trim().is_empty()
                    && description.chars().count() <= 1000
                    && !audio_prompt.trim().is_empty()
                    && audio_prompt.chars().count() <= 20_000
                    && [
                        (cover_title, 30),
                        (cover_subtitle, 60),
                        (cover_prompt_landscape, 2000),
                        (cover_prompt_portrait, 2000),
                    ]
                    .iter()
                    .all(|(value, limit)| value.chars().count() <= *limit)
            }
            ("outline" | "script", Self::Markdown { text }) => {
                !text.trim().is_empty() && text.chars().count() <= 100_000
            }
            ("titles" | "descriptions" | "hashtags", Self::List { items }) => {
                !items.is_empty()
                    && items.len() <= 20
                    && count.is_none_or(|n| n == items.len())
                    && items
                        .iter()
                        .all(|s| !s.trim().is_empty() && s.chars().count() <= 10_000)
            }
            _ => false,
        };
        if valid {
            Ok(())
        } else {
            Err("内容结构、数量或长度不符合要求，原有版本未修改。".into())
        }
    }
}
pub fn validate_kind(kind: &str) -> Result<(), String> {
    if matches!(
        kind,
        "publishing" | "outline" | "script" | "titles" | "descriptions" | "hashtags"
    ) {
        Ok(())
    } else {
        Err("不支持的创作类型。".into())
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiError {
    pub code: String,
    pub message: String,
}
impl AiError {
    pub fn new(code: &str, message: &str) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
    pub fn database(message: String) -> Self {
        Self::new("storage", &message)
    }
}

pub fn normalize_base_url(value: &str) -> Result<String, String> {
    let url = reqwest::Url::parse(value.trim()).map_err(|_| "服务地址不是有效的 URL。")?;
    let local = matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"));
    if (url.scheme() != "https" && !(url.scheme() == "http" && local))
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err("请使用 HTTPS 服务地址（本机服务可用 HTTP），地址中不能包含密钥、账号、查询参数或片段。".into());
    }
    Ok(url.as_str().trim_end_matches('/').to_string())
}

pub struct TextRequest {
    pub base_url: String,
    pub model: String,
    pub key: Option<String>,
    pub system: String,
    pub user: String,
}
pub trait TextProvider {
    fn generate(
        &self,
        request: TextRequest,
    ) -> Pin<Box<dyn Future<Output = Result<String, AiError>> + Send + '_>>;
}
pub struct OpenAiCompatible {
    client: reqwest::Client,
}
impl OpenAiCompatible {
    pub fn new(timeout: Duration, base_url: &str) -> Result<Self, AiError> {
        let mut builder = reqwest::Client::builder()
            .timeout(timeout)
            .connect_timeout(Duration::from_secs(15))
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never());
        // Local model services must stay local even when HTTP_PROXY is configured.
        let local = reqwest::Url::parse(base_url)
            .ok()
            .is_some_and(|url| matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]")));
        if local {
            builder = builder.no_proxy();
        }
        let client = builder
            .build()
            .map_err(|_| AiError::new("network", "无法初始化网络客户端。"))?;
        Ok(Self { client })
    }
}
fn network_error(error: reqwest::Error) -> AiError {
    if error.is_timeout() {
        AiError::new("timeout", "模型请求超时，请检查服务后手动重试。")
    } else {
        AiError::new("network", "无法连接模型服务，请检查网络、证书和服务地址。")
    }
}
impl TextProvider for OpenAiCompatible {
    fn generate(
        &self,
        request: TextRequest,
    ) -> Pin<Box<dyn Future<Output = Result<String, AiError>> + Send + '_>> {
        Box::pin(async move {
            let data = self.post_json(&request.base_url, "chat/completions", request.key,
                serde_json::json!({"model":request.model,"messages":[
                    {"role":"system","content":request.system}, {"role":"user","content":request.user}
                ],"stream":false}), 2_000_000).await?;
            let choice = &data["choices"][0];
            if choice["finish_reason"].as_str() != Some("stop") {
                return Err(AiError::new(
                    "incomplete",
                    "模型输出未完整结束或被拒绝，未保存。请调整资料后重试。",
                ));
            }
            choice["message"]["content"]
                .as_str()
                .filter(|s| !s.trim().is_empty())
                .map(str::to_string)
                .ok_or_else(|| AiError::new("invalid_response", "模型未返回可用文本。"))
        })
    }
}

impl OpenAiCompatible {
    async fn post_json(
        &self,
        base_url: &str,
        endpoint: &str,
        key: Option<String>,
        body: serde_json::Value,
        limit: usize,
    ) -> Result<serde_json::Value, AiError> {
        let mut call = self
            .client
            .post(format!("{base_url}/{endpoint}"))
            .json(&body);
        if let Some(key) = key.filter(|s| !s.is_empty()) {
            call = call.bearer_auth(key);
        }
        let mut response = call.send().await.map_err(network_error)?;
        let status = response.status().as_u16();
        if !response.status().is_success() {
            // Never surface provider response bodies/headers: some gateways echo secrets.
            return Err(match status {
                401 | 403 => AiError::new(
                    "authentication",
                    "模型服务拒绝认证，请检查 API Key 和模型权限。",
                ),
                429 => AiError::new("rate_limit", "模型服务限流或额度不足，请稍后重试。"),
                300..=399 => AiError::new(
                    "redirect",
                    "服务返回重定向，请直接配置最终可信的 API 地址。",
                ),
                _ => AiError::new(
                    "http",
                    &format!("模型服务返回 HTTP {status}，请检查地址与模型配置。"),
                ),
            });
        }
        let mut body = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(network_error)? {
            if body.len() + chunk.len() > limit {
                return Err(AiError::new("invalid_response", "模型响应过长，未保存。"));
            }
            body.extend_from_slice(&chunk);
        }
        let data: serde_json::Value = serde_json::from_slice(&body)
            .map_err(|_| AiError::new("invalid_response", "模型服务未返回有效 JSON。"))?;
        Ok(data)
    }
}

/// Transient cancellation signals only; SQLite remains the authority for task status.
#[derive(Default)]
pub struct ActiveTasks(pub Mutex<HashMap<String, watch::Sender<bool>>>);

#[cfg(test)]
mod tests;
