use super::{network_error, AiError};

fn incomplete() -> AiError {
    AiError::new(
        "incomplete",
        "模型流式输出未完整结束或被拒绝，未保存。请手动重试。",
    )
}
fn invalid() -> AiError {
    AiError::new("invalid_response", "模型流式响应格式无效，未保存。")
}

// HTTP chunks can split UTF-8 and SSE lines anywhere. Decode only complete lines,
// dispatch only complete events, and never treat reasoning deltas as answer text.
pub(super) async fn read(
    mut response: reqwest::Response,
    log: &mut super::diagnostics::Call<'_>,
) -> Result<String, AiError> {
    let mut line = Vec::new();
    let mut event = String::new();
    let mut content = String::new();
    let mut stopped = false;
    let mut received = 0;
    while let Some(chunk) = response.chunk().await.map_err(network_error)? {
        received += chunk.len();
        // SSE adds envelopes to every token, including reasoning and heartbeats.
        if received > 16_000_000 {
            return Err(AiError::new("invalid_response", "模型响应过长，未保存。"));
        }
        for byte in chunk {
            if byte != b'\n' {
                line.push(byte);
                continue;
            }
            let text = std::str::from_utf8(&line)
                .map_err(|_| invalid())?
                .trim_end_matches('\r');
            if text.is_empty() {
                if event.trim() == "[DONE]" {
                    return if stopped && !content.trim().is_empty() {
                        Ok(content)
                    } else {
                        Err(incomplete())
                    };
                }
                if !event.is_empty() {
                    let data: serde_json::Value =
                        serde_json::from_str(&event).map_err(|_| invalid())?;
                    log.usage(&data["usage"]);
                    if data.get("error").is_some() {
                        // Provider errors may echo credentials or private prompts.
                        return Err(AiError::new(
                            "http",
                            "模型服务在流式生成中返回错误，未保存。请检查服务后手动重试。",
                        ));
                    }
                    let choices = data["choices"].as_array().ok_or_else(invalid)?;
                    if let Some(choice) = choices.iter().find(|c| c["index"] == 0) {
                        if stopped {
                            return Err(incomplete());
                        }
                        if let Some(value) = choice["delta"].get("content").filter(|v| !v.is_null())
                        {
                            content.push_str(value.as_str().ok_or_else(invalid)?);
                            if content.len() > 2_000_000 {
                                return Err(invalid());
                            }
                        }
                        if let Some(reason) = choice.get("finish_reason").filter(|v| !v.is_null()) {
                            if reason != "stop" {
                                return Err(incomplete());
                            }
                            stopped = true;
                        }
                    }
                    event.clear();
                }
            } else if let Some(data) = text.strip_prefix("data:") {
                event.push_str(data.strip_prefix(' ').unwrap_or(data));
                event.push('\n');
            }
            line.clear();
        }
    }
    // EOF without [DONE] is a disconnected stream, not a complete answer.
    Err(incomplete())
}
