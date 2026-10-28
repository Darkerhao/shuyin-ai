#![cfg_attr(windows, windows_subsystem = "windows")]

mod protocol;

use protocol::{read_frame, write_frame};
use std::{
    env,
    fs::{self, File, OpenOptions},
    io,
    path::{Path, PathBuf},
    process::ExitCode,
    thread,
    time::Duration,
};

const APP_DIRECTORY: &str = "com.shuyin.desktop";
const BRIDGE_DIRECTORY: &str = "audio-bridge";
const PIPE_OPEN_ATTEMPTS: usize = 20;

fn pipe_marker_path() -> Option<PathBuf> {
    let local = env::var_os("LOCALAPPDATA").map(PathBuf::from)?;
    let file = if cfg!(debug_assertions) {
        "pipe-dev.txt"
    } else {
        "pipe.txt"
    };
    Some(local.join(APP_DIRECTORY).join(BRIDGE_DIRECTORY).join(file))
}

fn quoted_json_value(line: &str, key: &str) -> Option<String> {
    let marker = format!("\"{key}\"");
    let start = line.find(&marker)? + marker.len();
    let rest = line[start..].trim_start();
    let rest = rest.strip_prefix(':')?.trim_start();
    let rest = rest.strip_prefix('"')?;
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

fn parse_pipe_name(contents: &str) -> Option<String> {
    let text = contents.trim().trim_start_matches('\u{feff}').trim();
    if text.is_empty() {
        return None;
    }
    let value = if text.starts_with('{') {
        quoted_json_value(text, "pipeName")
            .or_else(|| quoted_json_value(text, "pipe_name"))
            .or_else(|| quoted_json_value(text, "name"))?
    } else {
        text.lines()
            .map(str::trim)
            .find(|line| !line.is_empty())?
            .split_once('=')
            .map_or_else(
                || text.lines().next().unwrap_or_default().trim(),
                |(_, value)| value.trim(),
            )
            .to_owned()
    };
    let value = value.trim();
    if value.is_empty() {
        None
    } else if value.starts_with(r"\\.\pipe\") {
        Some(value.to_owned())
    } else {
        Some(format!(
            r"\\.\pipe\{}",
            value.trim_start_matches(r"\\.\pipe\")
        ))
    }
}

fn read_pipe_name() -> io::Result<String> {
    let path = pipe_marker_path()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "LOCALAPPDATA is unavailable"))?;
    let contents = fs::read_to_string(&path).map_err(|error| {
        io::Error::new(
            error.kind(),
            format!(
                "cannot read audio bridge marker {}: {error}",
                path.display()
            ),
        )
    })?;
    parse_pipe_name(&contents).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "audio bridge marker does not contain a pipe name: {}",
                path.display()
            ),
        )
    })
}

fn open_pipe() -> io::Result<File> {
    let mut last_error = None;
    for attempt in 0..PIPE_OPEN_ATTEMPTS {
        match read_pipe_name().and_then(|name| {
            OpenOptions::new()
                .read(true)
                .write(true)
                .open(Path::new(&name))
        }) {
            Ok(file) => return Ok(file),
            Err(error) => last_error = Some(error),
        }
        if attempt + 1 < PIPE_OPEN_ATTEMPTS {
            thread::sleep(Duration::from_millis(100));
        }
    }
    Err(last_error
        .unwrap_or_else(|| io::Error::new(io::ErrorKind::NotFound, "audio bridge is unavailable")))
}

fn forward_message(mut pipe: File, payload: &[u8]) -> io::Result<Vec<u8>> {
    write_frame(&mut pipe, payload)?;
    read_frame(&mut pipe)?.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "audio bridge closed without a response",
        )
    })
}

fn run() -> io::Result<()> {
    let mut stdin = io::stdin().lock();
    let mut stdout = io::stdout().lock();
    while let Some(payload) = read_frame(&mut stdin)? {
        let response = forward_message(open_pipe()?, &payload)?;
        write_frame(&mut stdout, &response)?;
    }
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            // stdout is reserved for Native Messaging frames.
            eprintln!("shuyin audio bridge: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_plain_and_json_pipe_markers() {
        assert_eq!(
            parse_pipe_name("shuyin-audio-123\n").as_deref(),
            Some(r"\\.\pipe\shuyin-audio-123")
        );
        assert_eq!(
            parse_pipe_name(r#"{"pipeName":"shuyin-audio-456"}"#).as_deref(),
            Some(r"\\.\pipe\shuyin-audio-456")
        );
        assert_eq!(
            parse_pipe_name(r"name=\\.\pipe\shuyin-audio-789").as_deref(),
            Some(r"\\.\pipe\shuyin-audio-789")
        );
        assert!(parse_pipe_name("  ").is_none());
    }
}
