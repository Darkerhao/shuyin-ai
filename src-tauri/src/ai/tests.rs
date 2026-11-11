use super::images::{test_jpeg, test_png, test_webp, ImageProvider, ImageRequest};
use super::*;
use base64::{engine::general_purpose::STANDARD, Engine};
use std::sync::Arc;
use tokio::sync::oneshot;

fn image_request(base_url: String) -> ImageRequest {
    ImageRequest {
        base_url,
        model: "configured-image-model".into(),
        key: Some("contract-only-secret".into()),
        prompt: "无字山林".into(),
        width: 1920,
        height: 1080,
    }
}

#[tokio::test(flavor = "current_thread")]
async fn call_diagnostics_record_returned_usage_timing_errors_and_cancellation_without_secrets() {
    let (dir, db, input) = crate::database::video::tests::fixture();
    let job = db.create_video_job(input).unwrap();
    let body=serde_json::json!({"choices":[{"finish_reason":"stop","message":{"content":"PRIVATE_ANSWER"}}],"usage":{"prompt_tokens":12,"completion_tokens":7,"total_tokens":19}}).to_string();
    let (url, thread) = server(200, &body, Duration::from_millis(25));
    let request = TextRequest {
        base_url: url.clone(),
        model: "diagnostic-model".into(),
        key: Some("PRIVATE_KEY".into()),
        reasoning_effort: None,
        system: "PRIVATE_PROMPT".into(),
        user: "PRIVATE_USER".into(),
    };
    let result = OpenAiCompatible::new(TIMEOUT, &url)
        .unwrap()
        .observed(&db, Some(&job.id), "storyboard")
        .generate(request.clone())
        .await
        .unwrap();
    assert_eq!(result, "PRIVATE_ANSWER");
    thread.join().unwrap();
    let calls = serde_json::to_value(db.list_model_calls(1, 20).unwrap().rows).unwrap();
    assert_eq!(calls[0]["inputTokens"], 12);
    assert_eq!(calls[0]["outputTokens"], 7);
    assert_eq!(calls[0]["totalTokens"], 19);
    assert_eq!(calls[0]["status"], "succeeded");
    assert!(
        calls[0]["firstResponseMs"].as_i64().unwrap() <= calls[0]["elapsedMs"].as_i64().unwrap()
    );
    for value in [
        "PRIVATE_KEY",
        "PRIVATE_PROMPT",
        "PRIVATE_USER",
        "PRIVATE_ANSWER",
    ] {
        assert!(!calls.to_string().contains(value));
    }
    let (url, thread) = server(524, "PRIVATE_ERROR", Duration::ZERO);
    let mut failed = request.clone();
    failed.base_url = url.clone();
    OpenAiCompatible::new(TIMEOUT, &url)
        .unwrap()
        .observed(&db, Some(&job.id), "proofreading")
        .generate(failed)
        .await
        .unwrap_err();
    thread.join().unwrap();
    let (request_received, received) = oneshot::channel();
    let (release_response, response_released) = oneshot::channel();
    let (url, thread) = reply_server(move |_, _| {
        request_received.send(()).unwrap();
        response_released.blocking_recv().unwrap();
    });
    let mut cancelled = request;
    cancelled.base_url = url.clone();
    let provider =
        OpenAiCompatible::new(TIMEOUT, &url)
            .unwrap()
            .observed(&db, Some(&job.id), "writing");
    {
        let mut generation = provider.generate(cancelled);
        // Cancel only after the complete request arrives, independently of SQLite/CI speed.
        tokio::select! {
            result = &mut generation => panic!("request completed before cancellation: {result:?}"),
            result = tokio::time::timeout(Duration::from_secs(5), received) => {
                result.expect("server did not receive the request").unwrap();
            }
        }
    }
    release_response.send(()).unwrap();
    assert!(thread
        .join()
        .unwrap()
        .starts_with("POST /v1/chat/completions HTTP/1.1"));
    let calls = serde_json::to_value(db.list_model_calls(1, 20).unwrap().rows).unwrap();
    let cancelled = calls
        .as_array()
        .unwrap()
        .iter()
        .find(|value| value["stage"] == "writing")
        .unwrap();
    assert_eq!(cancelled["status"], "cancelled");
    assert_eq!(cancelled["errorCode"], "cancelled");
    assert!(cancelled["firstResponseMs"].is_null());
    assert!(calls
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v["errorCode"] == "timeout"));
    drop(provider);
    drop(db);
    let db = crate::database::Database::open(dir.path().join("data.sqlite3"));
    assert_eq!(db.list_model_calls(1, 20).unwrap().rows.len(), 3);
}

#[tokio::test(flavor = "current_thread")]
async fn image_contract_decodes_image_and_uses_the_configured_model() {
    let jpeg = test_jpeg();
    let body = serde_json::json!({"data":[{"b64_json":STANDARD.encode(&jpeg)}]}).to_string();
    let (url, h) = server(200, &body, Duration::ZERO);
    let bytes = OpenAiCompatible::new(TIMEOUT, &url)
        .unwrap()
        .generate_image(image_request(url))
        .await
        .unwrap();
    assert_eq!(bytes, jpeg);
    let received = h.join().unwrap();
    assert!(received.starts_with("POST /v1/images/generations HTTP/1.1"));
    assert!(received
        .to_lowercase()
        .contains("authorization: bearer contract-only-secret"));
    let body: serde_json::Value =
        serde_json::from_str(received.split("\r\n\r\n").nth(1).unwrap()).unwrap();
    assert_eq!(
        body,
        serde_json::json!({"model":"configured-image-model","prompt":"无字山林","n":1,"size":"1536x1024","response_format":"b64_json"})
    );
}

#[tokio::test(flavor = "current_thread")]
async fn image_contract_uses_portrait_size_for_a_portrait_cover() {
    let body = serde_json::json!({"data":[{"b64_json":STANDARD.encode(test_png())}]}).to_string();
    let (url, handle) = server(200, &body, Duration::ZERO);
    let mut request = image_request(url.clone());
    request.width = 1080;
    request.height = 1440;
    OpenAiCompatible::new(TIMEOUT, &url)
        .unwrap()
        .generate_image(request)
        .await
        .unwrap();
    let received = handle.join().unwrap();
    let body: serde_json::Value =
        serde_json::from_str(received.split("\r\n\r\n").nth(1).unwrap()).unwrap();
    assert_eq!(body["size"], "1024x1536");
}

#[tokio::test(flavor = "current_thread")]
async fn image_errors_timeout_and_unusable_content_never_return_an_asset() {
    for (status, code) in [
        (401, "authentication"),
        (429, "rate_limit"),
        (302, "redirect"),
        (503, "http"),
        (504, "timeout"),
        (524, "timeout"),
    ] {
        let (url, h) = server(status, "contract-only-secret", Duration::ZERO);
        let error = OpenAiCompatible::new(TIMEOUT, &url)
            .unwrap()
            .generate_image(image_request(url))
            .await
            .unwrap_err();
        assert_eq!(error.code, code);
        assert!(!error.message.contains("contract-only-secret"));
        h.join().unwrap();
    }
    for body in [
        "not json".to_string(),
        "{\"data\":[]}".into(),
        serde_json::json!({"data":[{"url":"https://example.com/image.png"}]}).to_string(),
        serde_json::json!({"data":[{"b64_json":"%%%"}]}).to_string(),
        serde_json::json!({"data":[{"b64_json":STANDARD.encode(b"not a png")}]}).to_string(),
    ] {
        let (url, h) = server(200, &body, Duration::ZERO);
        assert!(OpenAiCompatible::new(TIMEOUT, &url)
            .unwrap()
            .generate_image(image_request(url))
            .await
            .is_err());
        h.join().unwrap();
    }
    let (url, h) = server(200, "{}", Duration::from_millis(120));
    let error = OpenAiCompatible::new(Duration::from_millis(30), &url)
        .unwrap()
        .generate_image(image_request(url))
        .await
        .unwrap_err();
    assert_eq!(error.code, "timeout");
    h.join().unwrap();
}

#[test]
fn image_validation_rejects_truncated_corrupt_and_oversized_data() {
    let png = test_png();
    let jpeg = test_jpeg();
    let webp = test_webp();
    assert_eq!(images::validate_image(&png).unwrap(), ("png", 4, 4));
    assert_eq!(images::validate_image(&jpeg).unwrap(), ("jpg", 4, 4));
    assert_eq!(images::validate_image(&webp).unwrap(), ("webp", 4, 4));
    assert!(images::validate_image(&png[..png.len() - 12]).is_err());
    assert!(images::validate_image(&jpeg[..jpeg.len() / 2]).is_err());
    let mut corrupt = png.clone();
    corrupt[45] ^= 0xff;
    assert!(images::validate_image(&corrupt).is_err());
    assert!(images::validate_image(&vec![0; images::MAX_IMAGE_BYTES + 1]).is_err());
}
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    thread,
};

fn server(status: u16, body: &str, delay: Duration) -> (String, thread::JoinHandle<String>) {
    let body = body.to_string();
    reply_server(move |_, stream| {
        thread::sleep(delay);
        let _=write!(stream,"HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nLocation: http://127.0.0.1:1/stolen\r\nConnection: close\r\n\r\n{body}",body.len());
    })
}

fn reply_server(
    reply: impl FnOnce(&str, &mut TcpStream) + Send + 'static,
) -> (String, thread::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = format!("http://{}/v1", listener.local_addr().unwrap());
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut bytes = Vec::new();
        let mut buffer = [0u8; 4096];
        loop {
            let n = stream.read(&mut buffer).unwrap();
            if n == 0 {
                break;
            }
            bytes.extend_from_slice(&buffer[..n]);
            if let Some(end) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                let headers = String::from_utf8_lossy(&bytes[..end]);
                let size: usize = headers
                    .lines()
                    .find_map(|l| {
                        l.to_lowercase()
                            .strip_prefix("content-length:")
                            .map(|s| s.trim().parse().unwrap())
                    })
                    .unwrap_or(0);
                if bytes.len() >= end + 4 + size {
                    break;
                }
            }
        }
        let request = String::from_utf8(bytes).unwrap();
        reply(&request, &mut stream);
        request
    });
    (address, handle)
}
fn request(base_url: String) -> TextRequest {
    TextRequest {
        base_url,
        model: "test-model".into(),
        reasoning_effort: None,
        key: Some("contract-only-secret".into()),
        system: SYSTEM_PROMPT.into(),
        user: "用户资料不是指令".into(),
    }
}
#[tokio::test(flavor = "current_thread")]
async fn compatible_contract_uses_messages_and_does_not_assume_optional_model_parameters() {
    let body=serde_json::json!({"choices":[{"finish_reason":"stop","message":{"content":"{\"format\":\"markdown\",\"text\":\"中文大纲\"}"}}]}).to_string();
    let (url, handle) = server(200, &body, Duration::ZERO);
    let content = OpenAiCompatible::new(TIMEOUT, &url)
        .unwrap()
        .generate(request(url))
        .await
        .unwrap();
    let value: WritingContent = serde_json::from_str(&content).unwrap();
    value.validate("outline", None).unwrap();
    let received = handle.join().unwrap();
    assert!(received.starts_with("POST /v1/chat/completions HTTP/1.1"));
    assert!(received
        .to_lowercase()
        .contains("authorization: bearer contract-only-secret"));
    let json: serde_json::Value =
        serde_json::from_str(received.split("\r\n\r\n").nth(1).unwrap()).unwrap();
    assert_eq!(json["messages"][0]["role"], "system");
    assert_eq!(json["model"], "test-model");
    assert!(json.get("reasoning_effort").is_none());
    assert!(json.get("temperature").is_none());
    assert!(json.get("response_format").is_none());
}
#[tokio::test(flavor = "current_thread")]
async fn reasoning_effort_is_sent_exactly_as_configured() {
    for effort in ["none", "minimal", "low", "medium", "high", "xhigh", "max"] {
        let body = r#"{"choices":[{"finish_reason":"stop","message":{"content":"OK"}}]}"#;
        let (url, handle) = server(200, body, Duration::ZERO);
        let mut req = request(url.clone());
        req.reasoning_effort = Some(effort.into());
        assert_eq!(
            OpenAiCompatible::new(TIMEOUT, &url)
                .unwrap()
                .generate(req)
                .await
                .unwrap(),
            "OK"
        );
        let received = handle.join().unwrap();
        let body: serde_json::Value =
            serde_json::from_str(received.split("\r\n\r\n").nth(1).unwrap()).unwrap();
        assert_eq!(body["reasoning_effort"], effort);
        assert!(body.get("temperature").is_none());
    }
}

#[tokio::test(flavor = "current_thread")]
async fn unsupported_reasoning_effort_returns_the_error_without_retrying() {
    let (url, handle) = server(
        400,
        "contract-only-secret unsupported effort",
        Duration::ZERO,
    );
    let mut req = request(url.clone());
    req.reasoning_effort = Some("max".into());
    let error = OpenAiCompatible::new(TIMEOUT, &url)
        .unwrap()
        .generate(req)
        .await
        .unwrap_err();
    assert_eq!(error.code, "http");
    assert!(error.message.contains("400"));
    assert!(!error.message.contains("contract-only-secret"));
    let received = handle.join().unwrap();
    assert_eq!(received.matches("POST /v1/chat/completions").count(), 1);
}

#[tokio::test(flavor = "current_thread")]
async fn provider_errors_are_sanitized_and_redirects_are_rejected() {
    for (status, code) in [
        (401, "authentication"),
        (429, "rate_limit"),
        (503, "http"),
        (302, "redirect"),
    ] {
        let (url, h) = server(
            status,
            "contract-only-secret echoed by gateway",
            Duration::ZERO,
        );
        let error = OpenAiCompatible::new(TIMEOUT, &url)
            .unwrap()
            .generate(request(url))
            .await
            .unwrap_err();
        assert_eq!(error.code, code);
        assert!(!serde_json::to_string(&error)
            .unwrap()
            .contains("contract-only-secret"));
        h.join().unwrap();
    }
}
#[tokio::test(flavor = "current_thread")]
async fn gateway_timeouts_are_reported_as_timeouts_without_retrying_or_exposing_secrets() {
    for status in [524, 504] {
        let (url, handle) = server(
            status,
            "contract-only-secret echoed by gateway",
            Duration::ZERO,
        );
        let error = OpenAiCompatible::new(TIMEOUT, &url)
            .unwrap()
            .generate(request(url))
            .await
            .unwrap_err();
        assert_eq!(error.code, "timeout", "HTTP {status}");
        assert!(error.message.contains(&status.to_string()));
        assert!(error.message.contains("网关"));
        assert!(!error.message.contains("检查地址与模型配置"));
        assert!(!error.message.contains("contract-only-secret"));
        let received = handle.join().unwrap();
        assert_eq!(received.matches("POST /v1/chat/completions").count(), 1);
    }
}

#[tokio::test(flavor = "current_thread")]
async fn malformed_incomplete_empty_and_timeout_responses_fail() {
    for body in [
        "not json",
        r#"{"choices":[{"finish_reason":"length","message":{"content":"partial"}}]}"#,
        r#"{"choices":[{"finish_reason":"stop","message":{"content":""}}]}"#,
    ] {
        let (url, h) = server(200, body, Duration::ZERO);
        assert!(OpenAiCompatible::new(TIMEOUT, &url)
            .unwrap()
            .generate(request(url))
            .await
            .is_err());
        h.join().unwrap();
    }
    let (url, h) = server(200, "{}", Duration::from_millis(120));
    let error = OpenAiCompatible::new(Duration::from_millis(30), &url)
        .unwrap()
        .generate(request(url))
        .await
        .unwrap_err();
    assert_eq!(error.code, "timeout");
    h.join().unwrap();
}
#[test]
fn content_and_endpoint_validation() {
    for url in [
        "https://api.example.com/v1?key=secret",
        "https://user:secret@api.example.com/v1",
        "http://api.example.com/v1",
        "file:///key",
        "https://example.com/#secret",
    ] {
        assert!(normalize_base_url(url).is_err());
    }
    assert_eq!(
        normalize_base_url(" http://127.0.0.1:8080/v1/ ").unwrap(),
        "http://127.0.0.1:8080/v1"
    );
    assert!(WritingContent::List {
        items: vec!["标题".into()]
    }
    .validate("titles", Some(5))
    .is_err());
    assert!(WritingContent::Markdown { text: " ".into() }
        .validate("outline", None)
        .is_err());
    assert!(WritingContent::List {
        items: vec!["标题".into()]
    }
    .validate("script", None)
    .is_err());
    assert!(serde_json::from_str::<WritingContent>(
        r#"{"format":"markdown","text":"ok","unexpected":true}"#
    )
    .is_err());
}
#[test]
#[ignore = "Explicit native credential-store verification; creates and deletes one isolated test entry"]
fn native_system_credential_roundtrip() {
    use crate::credentials::{Credentials, SystemCredentials};
    let unique = format!("com.shuyin.m2-test.{}", std::process::id());
    let vault = SystemCredentials(unique);
    vault.set("roundtrip", "isolated-test-value").unwrap();
    let secret = vault.get("roundtrip").unwrap();
    vault.delete("roundtrip").unwrap();
    assert_eq!(secret.as_deref(), Some("isolated-test-value"));
    assert!(vault.get("roundtrip").unwrap().is_none());
}

fn sse_server(body: String) -> (String, thread::JoinHandle<String>) {
    reply_server(move |_, stream| {
        write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream; charset=utf-8\r\nTransfer-Encoding: chunked\r\n\r\n").unwrap();
        // Split JSON, CRLF and Chinese UTF-8 across HTTP chunks.
        for chunk in body.as_bytes().chunks(7) {
            if write!(stream, "{:x}\r\n", chunk.len()).is_err()
                || stream.write_all(chunk).is_err()
                || stream.write_all(b"\r\n").is_err()
            {
                return;
            }
        }
        let _ = stream.write_all(b"0\r\n\r\n");
    })
}

#[tokio::test(flavor = "current_thread")]
async fn streaming_avoids_a_gateway_that_times_out_buffered_completions() {
    let (url, handle) = reply_server(|request, stream| {
        let body: serde_json::Value =
            serde_json::from_str(request.split("\r\n\r\n").nth(1).unwrap()).unwrap();
        let (status, mime, response) = if body["stream"] == true {
            (200, "text/event-stream", "data: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\"分镜\"},\"finish_reason\":null}]}\n\ndata: {\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}]}\n\ndata: [DONE]\n\n")
        } else {
            (
                524,
                "text/plain",
                "gateway timed out waiting for the full generation",
            )
        };
        write!(stream, "HTTP/1.1 {status} Test\r\nContent-Type: {mime}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{response}", response.len()).unwrap();
    });
    let result = OpenAiCompatible::new(TIMEOUT, &url)
        .unwrap()
        .generate(request(url))
        .await;
    handle.join().unwrap();
    assert_eq!(result.unwrap(), "分镜");
}

#[tokio::test(flavor = "current_thread")]
async fn streaming_collects_only_content_and_preserves_reasoning_setting() {
    let (_dir, db, _) = crate::database::video::tests::fixture();
    let (url, handle) = sse_server(concat!(
        ": heartbeat\r\n\r\n",
        "data: {\"choices\":[{\"index\":0,\"delta\":{\"reasoning_content\":\"private reasoning\"}}]}\r\n\r\n",
        "data: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\"中文\"}}]}\r\n\r\n",
        "data: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\"分镜\"},\"finish_reason\":\"stop\"}]}\n\n",
        "data: {\"choices\":[],\"usage\":{\"total_tokens\":10}}\n\n",
        "data: [DONE]\n\n"
    ).into());
    let mut req = request(url.clone());
    req.reasoning_effort = Some("high".into());
    let result = OpenAiCompatible::new(TIMEOUT, &url)
        .unwrap()
        .observed(&db, None, "connection_test")
        .generate(req)
        .await;
    let received = handle.join().unwrap();
    assert_eq!(result.unwrap(), "中文分镜");
    let body: serde_json::Value =
        serde_json::from_str(received.split("\r\n\r\n").nth(1).unwrap()).unwrap();
    assert_eq!(body["stream"], true);
    assert_eq!(body["reasoning_effort"], "high");
    assert_eq!(body["stream_options"]["include_usage"], true);
    let calls = serde_json::to_value(db.list_model_calls(1, 20).unwrap().rows).unwrap();
    assert_eq!(calls[0]["totalTokens"], 10);
    assert!(calls[0]["inputTokens"].is_null());
    assert!(calls[0]["outputTokens"].is_null());
}

#[tokio::test(flavor = "current_thread")]
async fn streaming_rejects_partial_refused_malformed_and_error_responses() {
    for body in [
        "data: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\"partial\"}}]}\n\n",
        "data: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\"partial\"},\"finish_reason\":\"length\"}]}\n\ndata: [DONE]\n\n",
        "data: {\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}]}\n\ndata: [DONE]\n\n",
        "data: [DONE]\n\n",
        "data: not-json contract-only-secret\n\n",
        "data: {\"error\":{\"message\":\"contract-only-secret\"}}\n\n",
        "data: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\"partial\"},\"finish_reason\":\"stop\"}]}\n\n",
    ] {
        let (url, handle) = sse_server(body.into());
        let error = OpenAiCompatible::new(TIMEOUT, &url).unwrap().generate(request(url)).await.unwrap_err();
        assert!(!error.message.contains("contract-only-secret"));
        handle.join().unwrap();
    }
}

#[tokio::test(flavor = "current_thread")]
async fn streaming_done_finishes_without_waiting_for_connection_close() {
    let (url, handle) = reply_server(|_, stream| {
        write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\ndata: {{\"choices\":[{{\"index\":0,\"delta\":{{\"content\":\"OK\"}},\"finish_reason\":\"stop\"}}]}}\n\ndata: [DONE]\n\n").unwrap();
        stream.flush().unwrap();
        thread::sleep(Duration::from_millis(500));
    });
    let result = OpenAiCompatible::new(Duration::from_millis(250), &url)
        .unwrap()
        .generate(request(url))
        .await;
    handle.join().unwrap();
    assert_eq!(result.unwrap(), "OK");
}

#[tokio::test(flavor = "current_thread")]
async fn streaming_heartbeats_do_not_extend_the_total_timeout() {
    let (url, handle) = reply_server(|_, stream| {
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n"
        )
        .unwrap();
        for _ in 0..30 {
            if stream.write_all(b": heartbeat\n\n").is_err() {
                break;
            }
            let _ = stream.flush();
            thread::sleep(Duration::from_millis(20));
        }
    });
    let error = OpenAiCompatible::new(Duration::from_millis(150), &url)
        .unwrap()
        .generate(request(url))
        .await
        .unwrap_err();
    handle.join().unwrap();
    assert_eq!(error.code, "timeout");
}

#[tokio::test(flavor = "current_thread")]
async fn streaming_rejects_oversized_answers() {
    let (url, handle) = reply_server(|_, stream| {
        let event = serde_json::json!({"choices":[{"index":0,"delta":{"content":"a".repeat(2_000_001)},"finish_reason":"stop"}]}).to_string();
        let _ = write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\ndata: {event}\n\ndata: [DONE]\n\n");
    });
    let error = OpenAiCompatible::new(TIMEOUT, &url)
        .unwrap()
        .generate(request(url))
        .await
        .unwrap_err();
    handle.join().unwrap();
    assert_eq!(error.code, "invalid_response");
}

#[tokio::test(flavor = "current_thread")]
async fn live_candidates_arrive_before_completion_and_never_include_reasoning_or_json() {
    let (release, released) = oneshot::channel();
    let (url, server) = reply_server(move |_, stream| {
        let reasoning = serde_json::json!({"choices":[{"index":0,"delta":{"reasoning_content":"PRIVATE_REASONING"}}]});
        let first = serde_json::json!({"choices":[{"index":0,"delta":{"content":r#"{"title":"正在生成"#}}]});
        write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\ndata: {reasoning}\n\ndata: {first}\n\n").unwrap();
        stream.flush().unwrap();
        if released.blocking_recv().is_err() {
            return;
        }
        let last = serde_json::json!({"choices":[{"index":0,"delta":{"content":r#"的标题","description":"完成正文"}"#},"finish_reason":"stop"}]});
        let _ = write!(stream, "data: {last}\n\ndata: [DONE]\n\n");
    });
    let (received, preview_received) = oneshot::channel();
    let first = Arc::new(Mutex::new(Some(received)));
    let provider = OpenAiCompatible::new(TIMEOUT, &url)
        .unwrap()
        .with_content_observer(move |raw| {
            let shown = super::preview::texts(raw, None);
            if !shown.is_empty() {
                if let Some(sender) = first.lock().unwrap().take() {
                    let _ = sender.send(shown);
                }
            }
        });
    let mut generation = provider.generate(request(url));
    let early = tokio::select! {
        result = &mut generation => panic!("generation finished before live preview: {result:?}"),
        result = tokio::time::timeout(Duration::from_secs(5), preview_received) => result.unwrap().unwrap(),
    };
    release.send(()).unwrap();
    assert_eq!(early.len(), 1);
    assert_eq!(early["title"], "正在生成");
    let raw = generation.await.unwrap();
    assert_eq!(super::preview::texts(&raw, None)["title"], "正在生成的标题");
    assert!(!raw.contains("PRIVATE_REASONING"));
    assert!(server.join().unwrap().contains("chat/completions"));
}
