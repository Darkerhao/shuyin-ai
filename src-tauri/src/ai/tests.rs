use super::images::{test_jpeg, test_png, test_webp, ImageProvider, ImageRequest};
use super::*;
use base64::{engine::general_purpose::STANDARD, Engine};

fn image_request(base_url: String) -> ImageRequest {
    ImageRequest {
        base_url,
        model: "configured-image-model".into(),
        key: Some("contract-only-secret".into()),
        prompt: "无字山林".into(),
    }
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
        serde_json::json!({"model":"configured-image-model","prompt":"无字山林","n":1,"size":"1024x1024","response_format":"b64_json"})
    );
}

#[tokio::test(flavor = "current_thread")]
async fn image_errors_timeout_and_unusable_content_never_return_an_asset() {
    for (status, code) in [
        (401, "authentication"),
        (429, "rate_limit"),
        (302, "redirect"),
        (503, "http"),
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
    assert_eq!(images::validate_image(&png).unwrap(), "png");
    assert_eq!(images::validate_image(&jpeg).unwrap(), "jpg");
    assert_eq!(images::validate_image(&webp).unwrap(), "webp");
    assert!(images::validate_image(&png[..png.len() - 12]).is_err());
    assert!(images::validate_image(&jpeg[..jpeg.len() / 2]).is_err());
    let mut corrupt = png.clone();
    corrupt[45] ^= 0xff;
    assert!(images::validate_image(&corrupt).is_err());
    assert!(images::validate_image(&vec![0; images::MAX_IMAGE_BYTES + 1]).is_err());
}
use std::{
    io::{Read, Write},
    net::TcpListener,
    thread,
};

fn server(status: u16, body: &str, delay: Duration) -> (String, thread::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = format!("http://{}/v1", listener.local_addr().unwrap());
    let body = body.to_string();
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
        thread::sleep(delay);
        let _=write!(stream,"HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nLocation: http://127.0.0.1:1/stolen\r\nConnection: close\r\n\r\n{body}",body.len());
        String::from_utf8(bytes).unwrap()
    });
    (address, handle)
}
fn request(base_url: String) -> TextRequest {
    TextRequest {
        base_url,
        model: "test-model".into(),
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
    assert!(json.get("temperature").is_none());
    assert!(json.get("response_format").is_none());
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
