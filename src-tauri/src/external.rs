use std::process::Command;

pub(crate) fn open_url(url: &str) -> Result<(), String> {
    if !url.starts_with("https://") {
        return Err("只允许打开 HTTPS 地址。".into());
    }
    #[cfg(target_os = "windows")]
    let mut command = {
        use std::os::windows::process::CommandExt;
        let mut command = Command::new("explorer.exe");
        command.creation_flags(0x08000000);
        command
    };
    #[cfg(target_os = "macos")]
    let mut command = Command::new("open");
    #[cfg(target_os = "linux")]
    let mut command = Command::new("xdg-open");
    command
        .arg(url)
        .spawn()
        .map_err(|e| format!("无法打开外部工具：{e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_https_urls_before_spawning_a_process() {
        assert!(open_url("http://example.com").is_err());
        assert!(open_url("file:///C:/private.txt").is_err());
    }
}
