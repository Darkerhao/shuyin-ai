use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

#[cfg(target_os = "windows")]
pub(crate) use crate::paths::shell_path as external_path;
#[cfg(not(target_os = "windows"))]
pub(crate) fn external_path(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

pub fn write_new(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|e| e.to_string())
}

pub fn read_small(path: &Path) -> Result<String, String> {
    let mut bytes = Vec::new();
    File::open(path)
        .map_err(|e| e.to_string())?
        .take(2_000_001)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 2_000_000 {
        return Err("任务输出超过2MB，无法安全读取。".into());
    }
    String::from_utf8(bytes).map_err(|_| "任务输出不是有效的UTF-8文本。".into())
}

#[cfg(target_os = "windows")]
fn process_job(child: &std::process::Child) -> Result<std::os::windows::io::OwnedHandle, String> {
    use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
    use windows_sys::Win32::System::JobObjects::*;
    // The worker waits for a gate file, so no descendant can escape before assignment.
    unsafe {
        let raw = CreateJobObjectW(std::ptr::null(), std::ptr::null());
        if raw.is_null() {
            return Err(std::io::Error::last_os_error().to_string());
        }
        let job = OwnedHandle::from_raw_handle(raw);
        let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        if SetInformationJobObject(
            raw,
            JobObjectExtendedLimitInformation,
            &limits as *const _ as _,
            std::mem::size_of_val(&limits) as u32,
        ) == 0
            || AssignProcessToJobObject(raw, child.as_raw_handle()) == 0
        {
            return Err(std::io::Error::last_os_error().to_string());
        }
        Ok(job)
    }
}

pub fn run(
    executable: &str,
    script: &Path,
    input: &Path,
    output: &Path,
    cancelled: Arc<AtomicBool>,
    timeout: Duration,
) -> Result<(), String> {
    if !cfg!(target_os = "windows") {
        return Err("首版原声视频生成暂仅支持 Windows。".into());
    }
    if cancelled.load(Ordering::SeqCst) {
        return Err("视频任务已停止。".into());
    }
    if !Path::new(executable).is_absolute() || !Path::new(executable).is_file() {
        return Err("请先配置可用的 Node/Python 可执行文件绝对路径。".into());
    }
    let directory = input.parent().ok_or("任务目录无效。")?;
    let gate = output.with_extension("ready");
    let log_path = output.with_extension("log");
    let log = File::create(&log_path).map_err(|e| e.to_string())?;
    let mut command = Command::new(external_path(Path::new(executable)));
    command
        .args([script, input, output, &gate].map(external_path))
        .current_dir(external_path(directory))
        .env("PYTHONUTF8", "1")
        .env("PYTHONDONTWRITEBYTECODE", "1")
        .env("HF_HUB_OFFLINE", "1")
        .env("TEMP", external_path(directory))
        .env("TMP", external_path(directory))
        .stdin(Stdio::null())
        .stdout(log.try_clone().map_err(|e| e.to_string())?)
        .stderr(log);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let mut child = command
        .spawn()
        .map_err(|e| format!("无法启动视频运行环境：{e}"))?;
    #[cfg(target_os = "windows")]
    let _job = match process_job(&child) {
        Ok(job) => job,
        Err(e) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!("无法隔离视频子进程：{e}"));
        }
    };
    if let Err(e) = write_new(&gate, b"ready") {
        let _ = child.kill();
        let _ = child.wait();
        return Err(e);
    }
    let start = Instant::now();
    loop {
        if cancelled.load(Ordering::SeqCst) || start.elapsed() > timeout {
            let _ = child.kill();
            let _ = child.wait();
            return Err(if cancelled.load(Ordering::SeqCst) {
                "视频任务已停止。"
            } else {
                "该阶段等待超时，已停止子进程；可以手动重试。"
            }
            .into());
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                if status.success() && output.is_file() {
                    return Ok(());
                }
                let mut file = File::open(&log_path).map_err(|e| e.to_string())?;
                let length = file.metadata().map_err(|e| e.to_string())?.len();
                file.seek(SeekFrom::Start(length.saturating_sub(4000)))
                    .map_err(|e| e.to_string())?;
                let mut bytes = Vec::new();
                file.read_to_end(&mut bytes).map_err(|e| e.to_string())?;
                return Err(format!(
                    "视频处理失败（{}）：{}",
                    status,
                    String::from_utf8_lossy(&bytes)
                ));
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(100)),
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(e.to_string());
            }
        }
    }
}

pub fn fingerprint(path: &Path) -> Result<String, String> {
    use sha2::{Digest, Sha256};
    let mut file = File::open(path).map_err(|e| e.to_string())?;
    let before = file.metadata().map_err(|e| e.to_string())?;
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
    }
    let after = fs::metadata(path).map_err(|e| e.to_string())?;
    if before.len() != after.len() || before.modified().ok() != after.modified().ok() {
        return Err("音频读取期间发生变化，请重试。".into());
    }
    Ok(hash
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires SHUYIN_TEST_NODE pointing to an installed Node executable"]
    fn cancelling_video_worker_terminates_its_descendants() {
        let node = std::env::var("SHUYIN_TEST_NODE").expect("Set SHUYIN_TEST_NODE to node.exe");
        let dir = tempfile::tempdir().unwrap();
        let marker = dir.path().join("child-ready");
        let escaped = dir.path().join("child-survived");
        let script = dir.path().join("worker.mjs");
        write_new(&script, br#"
import fs from 'node:fs'; import {spawn} from 'node:child_process';
const data=JSON.parse(fs.readFileSync(process.argv[2],'utf8'));
while(!fs.existsSync(process.argv[4])) await new Promise(r=>setTimeout(r,20));
spawn(process.execPath,['-e',"const fs=require('fs');fs.writeFileSync(process.argv[1],'ready');setTimeout(()=>fs.writeFileSync(process.argv[2],'escaped'),1000)",data.marker,data.escaped],{stdio:'ignore',windowsHide:true});
setInterval(()=>{},100);
"#).unwrap();
        let input = dir.path().join("input.json");
        write_new(
            &input,
            &serde_json::to_vec(&serde_json::json!({"marker":marker,"escaped":escaped})).unwrap(),
        )
        .unwrap();
        let cancel = Arc::new(AtomicBool::new(false));
        let signal = cancel.clone();
        let watched = marker.clone();
        let thread = std::thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(10);
            while !watched.exists() && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(20));
            }
            signal.store(true, Ordering::SeqCst);
        });
        let canonical = fs::canonicalize(dir.path()).unwrap();
        let result = run(
            &node,
            &canonical.join("worker.mjs"),
            &canonical.join("input.json"),
            &canonical.join("output.json"),
            cancel,
            Duration::from_secs(15),
        );
        thread.join().unwrap();
        assert!(
            marker.is_file(),
            "worker must start its child before cancellation"
        );
        assert!(result.unwrap_err().contains("停止"));
        std::thread::sleep(Duration::from_millis(1200));
        assert!(
            !escaped.exists(),
            "the child must not outlive the cancelled task"
        );
    }
}
