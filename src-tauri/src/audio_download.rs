use super::{AudioAutomation, DownloadIntent};
use crate::database::Database;
use std::{
    cell::RefCell,
    sync::{
        atomic::{AtomicI64, Ordering},
        Arc, Mutex,
    },
};
use tauri::{AppHandle, Manager, WebviewWindow};
use webview2_com::{
    CoTaskMemPWSTR, DownloadStartingEventHandler,
    Microsoft::Web::WebView2::Win32::{
        ICoreWebView2DownloadOperation, ICoreWebView2_4, COREWEBVIEW2_DOWNLOAD_STATE_COMPLETED,
        COREWEBVIEW2_DOWNLOAD_STATE_IN_PROGRESS,
    },
    NavigationStartingEventHandler, StateChangedEventHandler,
};
use windows_core::Interface;

// COM objects stay on the WebView UI thread, including cancellation and event removal.
thread_local! {
    static ACTIVE: RefCell<Option<(String, i64, ICoreWebView2DownloadOperation)>> = const { RefCell::new(None) };
}

pub fn cancel(window: &WebviewWindow) {
    let _ = window.with_webview(|_| {
        ACTIVE.with(|active| {
            let operation = active.borrow_mut().take();
            if let Some((_, _, operation)) = operation {
                let _ = unsafe { operation.Cancel() };
            }
        })
    });
}

pub async fn attach(window: &WebviewWindow, app: AppHandle) -> Result<(), String> {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let sender = Arc::new(Mutex::new(Some(sender)));
    window
        .with_webview(move |webview| {
            let result = (|| -> windows_core::Result<()> {
                let view: ICoreWebView2_4 =
                    unsafe { webview.controller().CoreWebView2()? }.cast()?;
                let navigation_app = app.clone();
                let navigation =
                    NavigationStartingEventHandler::create(Box::new(move |_, args| {
                        let Some(args) = args else { return Ok(()) };
                        let mut uri = windows_core::PWSTR::null();
                        let mut id = 0;
                        let mut redirected = windows_core::BOOL::default();
                        unsafe {
                            args.Uri(&mut uri)?;
                            args.NavigationId(&mut id)?;
                            args.IsRedirected(&mut redirected)?;
                        }
                        let uri = webview2_com::take_pwstr(uri);
                        let state = navigation_app.state::<AudioAutomation>();
                        if let Ok(mut pending) = state.download.lock() {
                            if let Some(intent) = pending.as_mut() {
                                let allowed = !intent.started
                                    && intent.epoch == state.epoch.load(Ordering::SeqCst)
                                    && intent.armed_at.elapsed()
                                        < std::time::Duration::from_secs(15)
                                    && intent.allow_navigation(&uri, id, redirected.as_bool());
                                unsafe {
                                    args.SetCancel(!allowed)?;
                                }
                            }
                        }
                        Ok(())
                    }));
                let mut navigation_token = 0;
                unsafe {
                    view.add_NavigationStarting(&navigation, &mut navigation_token)?;
                }
                let handler = DownloadStartingEventHandler::create(Box::new(move |_, args| {
                    let Some(args) = args else {
                        return Ok(());
                    };
                    let operation = unsafe { args.DownloadOperation()? };
                    let mut uri = windows_core::PWSTR::null();
                    unsafe {
                        operation.Uri(&mut uri)?;
                    }
                    let uri = webview2_com::take_pwstr(uri);
                    let state = app.state::<AudioAutomation>();
                    let intent = state.download.lock().ok().and_then(|mut pending| {
                        let intent = pending.as_mut()?;
                        if intent.started
                            || intent.epoch != state.epoch.load(Ordering::SeqCst)
                            || !intent.matches_url(&uri)
                            || intent.path.exists()
                            || intent.armed_at.elapsed() > std::time::Duration::from_secs(15)
                        {
                            return None;
                        }
                        intent.started = true;
                        Some(intent.clone())
                    });
                    let Some(intent) = intent else {
                        unsafe {
                            args.SetCancel(true)?;
                        }
                        return Ok(());
                    };
                    let result = start(&app, &args, &operation, &intent);
                    if let Err(message) = result {
                        unsafe {
                            args.SetCancel(true)?;
                        }
                        if let Ok(mut pending) = state.download.lock() {
                            *pending = None;
                        }
                        let _ = app.state::<Database>().audio_report_error(
                            &intent.job_id,
                            intent.run_id,
                            "download_failed",
                            &message,
                        );
                        state.notify();
                    }
                    Ok(())
                }));
                let mut token = 0;
                unsafe {
                    view.add_DownloadStarting(&handler, &mut token)?;
                }
                Ok(())
            })()
            .map_err(|e| format!("无法初始化音频下载监听：{e}"));
            if let Ok(mut sender) = sender.lock() {
                if let Some(sender) = sender.take() {
                    let _ = sender.send(result);
                }
            }
        })
        .map_err(|e| e.to_string())?;
    tokio::time::timeout(std::time::Duration::from_secs(10), receiver)
        .await
        .map_err(|_| "初始化音频下载监听超时。")?
        .map_err(|_| "网站窗口已关闭。")?
}

fn start(
    app: &AppHandle,
    args: &webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2DownloadStartingEventArgs,
    operation: &ICoreWebView2DownloadOperation,
    intent: &DownloadIntent,
) -> Result<(), String> {
    app.state::<Database>()
        .audio_download_started(&intent.job_id, intent.run_id)?;
    let target = crate::paths::shell_path(&intent.path);
    let path = CoTaskMemPWSTR::from(target.as_str());
    let token = Arc::new(AtomicI64::new(0));
    let event_token = token.clone();
    let handle = app.clone();
    let intent = intent.clone();
    let expected = intent.clone();
    let callback = StateChangedEventHandler::create(Box::new(move |operation, _| {
        let Some(operation) = operation else {
            return Ok(());
        };
        let mut state = COREWEBVIEW2_DOWNLOAD_STATE_IN_PROGRESS;
        unsafe {
            operation.State(&mut state)?;
        }
        if state == COREWEBVIEW2_DOWNLOAD_STATE_IN_PROGRESS {
            return Ok(());
        }
        let _ = unsafe { operation.remove_StateChanged(event_token.load(Ordering::SeqCst)) };
        ACTIVE.with(|active| {
            let matches = active
                .borrow()
                .as_ref()
                .is_some_and(|(id, run, _)| *id == intent.job_id && *run == intent.run_id);
            if matches {
                active.borrow_mut().take();
            }
        });
        let app = handle.clone();
        let completed = intent.clone();
        tauri::async_runtime::spawn_blocking(move || {
            let automation = app.state::<AudioAutomation>();
            let matching = automation.download.lock().ok().is_some_and(|pending| {
                pending.as_ref().is_some_and(|item| {
                    item.job_id == completed.job_id && item.run_id == completed.run_id
                })
            });
            if !matching {
                return;
            }
            let database = app.state::<Database>();
            if state == COREWEBVIEW2_DOWNLOAD_STATE_COMPLETED {
                let result = database.complete_audio_download(
                    &completed.job_id,
                    completed.run_id,
                    &completed.path.to_string_lossy(),
                );
                if let Err(message) = result {
                    let _ = database.audio_report_error(
                        &completed.job_id,
                        completed.run_id,
                        "download_import_failed",
                        &message,
                    );
                }
            } else {
                let _ = database.audio_report_error(
                    &completed.job_id,
                    completed.run_id,
                    "download_interrupted",
                    "音频下载已中断，继续跟进将只重试当前音频下载。",
                );
            }
            if let Ok(mut pending) = automation.download.lock() {
                if pending.as_ref().is_some_and(|item| {
                    item.job_id == completed.job_id && item.run_id == completed.run_id
                }) {
                    pending.take();
                }
            }
            automation.notify();
        });
        Ok(())
    }));
    let mut value = 0;
    unsafe {
        args.SetResultFilePath(*path.as_ref().as_pcwstr())
            .map_err(|e| e.to_string())?;
        args.SetHandled(true).map_err(|e| e.to_string())?;
        operation
            .add_StateChanged(&callback, &mut value)
            .map_err(|e| e.to_string())?;
        token.store(value, Ordering::SeqCst);
        args.SetCancel(false).map_err(|e| e.to_string())?;
    }
    ACTIVE.with(|active| {
        *active.borrow_mut() = Some((expected.job_id, expected.run_id, operation.clone()))
    });
    Ok(())
}
