use std::path::Path;

// Windows shell consumers do not accept canonicalize's verbatim path prefix.
pub(crate) fn shell_path(path: &Path) -> String {
    let path = path.to_string_lossy();
    if let Some(unc) = path.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{unc}")
    } else {
        path.strip_prefix(r"\\?\").unwrap_or(&path).to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_paths_preserve_unicode_spaces_and_network_roots() {
        assert_eq!(
            shell_path(Path::new(r"\\?\D:\书籍 素材\视频.mp4")),
            r"D:\书籍 素材\视频.mp4"
        );
        assert_eq!(
            shell_path(Path::new(r"\\?\UNC\server\share\视频.mp4")),
            r"\\server\share\视频.mp4"
        );
        assert_eq!(
            shell_path(Path::new(r"D:\书引 AI\bridge.exe")),
            r"D:\书引 AI\bridge.exe"
        );
    }
}
