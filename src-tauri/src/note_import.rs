use serde::Serialize;
use std::{
    fs::{self, File},
    io::Read,
    path::Path,
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportedNote {
    pub text: String,
    pub source_name: String,
    pub source_path: String,
}

pub fn read_note(path: &Path) -> Result<ImportedNote, String> {
    let path = fs::canonicalize(path).map_err(|e| format!("读书资料不可用：{e}"))?;
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if !matches!(ext.as_str(), "txt" | "md") {
        return Err("目前仅支持 TXT 和 Markdown（.md）资料。".into());
    }
    let mut bytes = Vec::new();
    File::open(&path)
        .map_err(|e| e.to_string())?
        .take(1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 1024 * 1024 {
        return Err("读书资料不能超过 1 MB。".into());
    }
    let text = String::from_utf8(bytes).map_err(|_| "请将读书资料保存为 UTF-8 编码后导入。")?;
    let text = text
        .trim_start_matches('\u{feff}')
        .replace("\r\n", "\n")
        .replace('\r', "\n");
    if text.trim().is_empty() || text.chars().count() > 100_000 || text.contains('\0') {
        return Err("资料不能为空、包含无效字符或超过 10 万字。".into());
    }
    Ok(ImportedNote {
        text,
        source_name: path
            .file_name()
            .ok_or("文件名无效。")?
            .to_string_lossy()
            .into_owned(),
        source_path: path.to_string_lossy().into_owned(),
    })
}

#[tauri::command]
pub fn import_reading_notes(path: String) -> Result<ImportedNote, String> {
    read_note(Path::new(&path))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn imports_utf8_markdown_with_source_and_rejects_invalid_files() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("中文笔记.md");
        fs::write(&path, "\u{feff}# 笔记\r\n保留原文\r下一行").unwrap();
        let imported = read_note(&path).unwrap();
        assert_eq!(imported.text, "# 笔记\n保留原文\n下一行");
        assert_eq!(imported.source_name, "中文笔记.md");
        assert!(Path::new(&imported.source_path).is_absolute());
        fs::write(&path, [0xff, 0xfe]).unwrap();
        assert!(read_note(&path).unwrap_err().contains("UTF-8"));
        fs::write(&path, " ").unwrap();
        assert!(read_note(&path).is_err());
        fs::write(&path, "字".repeat(100_001)).unwrap();
        assert!(read_note(&path).is_err());
        fs::write(&path, vec![b'a'; 1024 * 1024 + 1]).unwrap();
        assert!(read_note(&path).is_err());
        assert!(read_note(&dir.path().join("missing.txt")).is_err());
        let pdf = dir.path().join("资料.pdf");
        fs::write(&pdf, b"pdf").unwrap();
        assert!(read_note(&pdf).unwrap_err().contains("仅支持"));
    }
}
