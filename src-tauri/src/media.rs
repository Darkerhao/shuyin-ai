use serde::Serialize;
use std::{
    fs::File,
    io::{Cursor, Read, Seek, SeekFrom},
    path::Path,
};

const MAX_MOOV_BYTES: u64 = 32 * 1024 * 1024;

#[derive(Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoMetadata {
    pub duration_seconds: f64,
    pub width: u32,
    pub height: u32,
    pub has_audio: bool,
}

fn read_exact(file: &mut File, length: usize) -> Result<Vec<u8>, String> {
    let mut bytes = vec![0; length];
    file.read_exact(&mut bytes)
        .map_err(|_| "最终视频结构不完整或无法读取。".to_string())?;
    Ok(bytes)
}

fn read_metadata_boxes(path: &Path) -> Result<Vec<u8>, String> {
    let mut file = File::open(path).map_err(|_| "最终视频文件无法读取。")?;
    let length = file.metadata().map_err(|e| e.to_string())?.len();
    let mut offset = 0;
    let mut ftyp = None;
    let mut moov = None;
    while offset < length {
        file.seek(SeekFrom::Start(offset))
            .map_err(|e| e.to_string())?;
        let header = read_exact(&mut file, 8)?;
        let size32 = u32::from_be_bytes(header[0..4].try_into().unwrap()) as u64;
        let name = &header[4..8];
        let (size, header_length) = if size32 == 1 {
            let large = read_exact(&mut file, 8)?;
            (u64::from_be_bytes(large.try_into().unwrap()), 16)
        } else if size32 == 0 {
            (length - offset, 8)
        } else {
            (size32, 8)
        };
        if size < header_length || offset.saturating_add(size) > length {
            return Err("最终视频盒结构无效。".into());
        }
        if matches!(name, b"ftyp" | b"moov") {
            if size > MAX_MOOV_BYTES {
                return Err("视频元数据过大，无法安全读取。".into());
            }
            file.seek(SeekFrom::Start(offset))
                .map_err(|e| e.to_string())?;
            let bytes = read_exact(&mut file, size as usize)?;
            if name == b"ftyp" {
                ftyp = Some(bytes);
            } else {
                moov = Some(bytes);
            }
        }
        offset += size;
    }
    let mut metadata = ftyp.ok_or("最终视频缺少 MP4 文件头。")?;
    metadata.extend(moov.ok_or("最终视频缺少可读取的轨道信息。")?);
    Ok(metadata)
}

pub fn inspect(path: &Path) -> Result<VideoMetadata, String> {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if !matches!(extension.as_str(), "mp4" | "mov" | "m4v") {
        return Err("最终成片请使用 MP4、MOV 或 M4V 格式。".into());
    }
    let bytes = read_metadata_boxes(path)?;
    let context = mp4parse::read_mp4(&mut Cursor::new(bytes))
        .map_err(|_| "最终视频无法解析，请重新从剪映导出。")?;
    let video = context
        .tracks
        .iter()
        .find(|track| track.track_type == mp4parse::TrackType::Video)
        .ok_or("最终视频中没有视频轨道。")?;
    let header = video.tkhd.as_ref().ok_or("最终视频缺少画面尺寸信息。")?;
    let duration_seconds = match (video.duration, video.timescale) {
        (Some(duration), Some(timescale)) if timescale.0 > 0 => {
            duration.0 as f64 / timescale.0 as f64
        }
        _ => {
            let timescale = context.timescale.ok_or("最终视频缺少时长信息。")?;
            if timescale.0 == 0 {
                return Err("最终视频时长信息无效。".into());
            }
            header.duration as f64 / timescale.0 as f64
        }
    };
    if !duration_seconds.is_finite() || duration_seconds <= 0.0 {
        return Err("最终视频时长无效。".into());
    }
    Ok(VideoMetadata {
        duration_seconds,
        width: header.width >> 16,
        height: header.height >> 16,
        has_audio: context
            .tracks
            .iter()
            .any(|track| track.track_type == mp4parse::TrackType::Audio),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn reads_portrait_video_duration_dimensions_and_audio() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("final.mp4");
        fs::write(&path, include_bytes!("../tests/fixtures/portrait.mp4")).unwrap();
        let metadata = inspect(&path).unwrap();
        assert_eq!((metadata.width, metadata.height), (1080, 1920));
        assert!(metadata.has_audio);
        assert!((metadata.duration_seconds - 1.0).abs() < 0.2);
    }

    #[test]
    fn rejects_unsupported_or_truncated_final_video() {
        let directory = tempfile::tempdir().unwrap();
        let unsupported = directory.path().join("final.mkv");
        fs::write(&unsupported, b"not video").unwrap();
        assert!(inspect(&unsupported).unwrap_err().contains("MP4"));
        let truncated = directory.path().join("final.mp4");
        fs::write(&truncated, b"not video").unwrap();
        assert!(inspect(&truncated).is_err());
    }
}
