//! Shared Windows host-side CDP. No debugging port or remote-page IPC is required.
use serde_json::Value;
use tauri::WebviewWindow;

#[cfg(windows)]
pub async fn call(window: &WebviewWindow, method: &str, params: Value) -> Result<Value, String> {
    use std::sync::{Arc, Mutex};
    use webview2_com::{CallDevToolsProtocolMethodCompletedHandler, CoTaskMemPWSTR};

    let (sender, receiver) = tokio::sync::oneshot::channel();
    let sender = Arc::new(Mutex::new(Some(sender)));
    let method = method.to_owned();
    let params = params.to_string();
    window
        .with_webview(move |webview| {
            let callback_sender = sender.clone();
            let callback = CallDevToolsProtocolMethodCompletedHandler::create(Box::new(
                move |status, json| {
                    if let Ok(mut sender) = callback_sender.lock() {
                        if let Some(sender) = sender.take() {
                            let _ = sender.send(status.map(|_| json).map_err(|e| e.to_string()));
                        }
                    }
                    Ok(())
                },
            ));
            let method = CoTaskMemPWSTR::from(method.as_str());
            let params = CoTaskMemPWSTR::from(params.as_str());
            // The controller and callback are used only on the WebView UI thread.
            let result = unsafe {
                webview.controller().CoreWebView2().and_then(|view| {
                    view.CallDevToolsProtocolMethod(
                        *method.as_ref().as_pcwstr(),
                        *params.as_ref().as_pcwstr(),
                        &callback,
                    )
                })
            };
            if let Err(error) = result {
                if let Ok(mut sender) = sender.lock() {
                    if let Some(sender) = sender.take() {
                        let _ = sender.send(Err(error.to_string()));
                    }
                }
            }
        })
        .map_err(|e| format!("无法访问网站窗口：{e}"))?;
    let json = tokio::time::timeout(std::time::Duration::from_secs(30), receiver)
        .await
        .map_err(|_| "网页操作超时，请在网站窗口检查结果后再继续。")?
        .map_err(|_| "网站窗口已关闭。")?
        .map_err(|e| format!("浏览器操作失败：{e}"))?;
    serde_json::from_str(&json).map_err(|_| "浏览器返回了无效结果。".into())
}

#[cfg(not(windows))]
pub async fn call(_window: &WebviewWindow, _method: &str, _params: Value) -> Result<Value, String> {
    Err("原生网站操作目前仅支持 Windows。".into())
}
