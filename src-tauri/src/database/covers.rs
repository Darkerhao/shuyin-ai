use super::{
    projects::{get, new_id, NOW},
    Database,
};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::{
    fs::OpenOptions,
    io::{Read, Write},
    path::Path,
};

const MAX_IMAGE_BYTES: usize = 15 * 1024 * 1024;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CoverConfig {
    pub schema_version: u32,
    pub template_id: String,
    pub width: u32,
    pub height: u32,
    pub title: String,
    pub subtitle: String,
    pub brand: String,
    pub font_family: String,
    pub title_size: u32,
    pub subtitle_size: u32,
    pub align: String,
    pub position: String,
    pub background_color: String,
    pub text_color: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subtitle_color: Option<String>,
    pub accent_color: String,
    pub background_asset_id: Option<String>,
    pub overlay_opacity: f64,
    #[serde(default = "center")]
    pub background_x: f64,
    #[serde(default = "center")]
    pub background_y: f64,
    #[serde(default = "one")]
    pub background_zoom: f64,
}
fn center() -> f64 {
    0.5
}
fn one() -> f64 {
    1.0
}

impl CoverConfig {
    fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1
            || !matches!(
                self.template_id.as_str(),
                "classic" | "night" | "spotlight" | "ember" | "ocean"
            )
            || !matches!(
                (self.width, self.height),
                (1440, 1080) | (1080, 1440) | (1920, 1080)
            )
            || !matches!(self.font_family.as_str(), "sans" | "serif")
            || !matches!(self.align.as_str(), "left" | "center")
            || !matches!(self.position.as_str(), "top" | "center" | "bottom")
            || !(40..=160).contains(&self.title_size)
            || !(24..=80).contains(&self.subtitle_size)
            || !(0.0..=0.85).contains(&self.overlay_opacity)
            || !(0.0..=1.0).contains(&self.background_x)
            || !(0.0..=1.0).contains(&self.background_y)
            || !(1.0..=3.0).contains(&self.background_zoom)
        {
            return Err("封面尺寸、模板或排版设置无效。".into());
        }
        if self.title.trim().is_empty()
            || self.title.chars().count() > 200
            || self.subtitle.chars().count() > 300
            || self.brand.chars().count() > 60
        {
            return Err("主标题必填且最多 200 字，副标题最多 300 字，品牌标识最多 60 字。".into());
        }
        for color in [
            &self.background_color,
            &self.text_color,
            &self.accent_color,
            self.subtitle_color.as_ref().unwrap_or(&self.text_color),
        ] {
            if color.len() != 7
                || !color.starts_with('#')
                || !color.as_bytes()[1..].iter().all(u8::is_ascii_hexdigit)
            {
                return Err("封面颜色必须是六位十六进制颜色。".into());
            }
        }
        Ok(())
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CoverVersion {
    pub id: String,
    pub project_id: String,
    pub version_no: u32,
    pub config: CoverConfig,
    pub exported_path: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct CoverPreset {
    pub id: String,
    pub name: String,
    pub style: serde_json::Value,
}

fn background_path(c: &Connection, project_id: &str, asset_id: &str) -> Result<String, String> {
    c.query_row(
        "SELECT file_path FROM assets WHERE id=?1 AND project_id=?2 AND asset_type='image'",
        params![asset_id, project_id],
        |r| r.get(0),
    )
    .map_err(|_| "背景图关联已失效，请重新选择图片。".into())
}

fn validate_png(bytes: &[u8], config: &CoverConfig) -> Result<(), String> {
    if bytes.len() < 45
        || bytes.len() > MAX_IMAGE_BYTES
        || !bytes.starts_with(b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR")
        || !bytes.ends_with(b"\0\0\0\0IEND\xaeB`\x82")
        || bytes[16..20] != config.width.to_be_bytes()
        || bytes[20..24] != config.height.to_be_bytes()
    {
        return Err("PNG 数据无效、超过 15 MB 或与封面尺寸不一致。".into());
    }
    Ok(())
}

impl Database {
    pub fn list_cover_presets(&self) -> Result<Vec<CoverPreset>, String> {
        self.with_connection(|c| {
            let mut q = c
                .prepare("SELECT id,name,style_json FROM cover_presets ORDER BY created_at DESC,id")
                .map_err(|e| e.to_string())?;
            let rows = q
                .query_map([], |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                    ))
                })
                .map_err(|e| e.to_string())?;
            rows.map(|row| {
                let (id, name, json) = row.map_err(|e| e.to_string())?;
                Ok(CoverPreset {
                    id,
                    name,
                    style: serde_json::from_str(&json).map_err(|e| e.to_string())?,
                })
            })
            .collect()
        })
    }
    pub fn save_cover_preset(&self, name: &str, config: CoverConfig) -> Result<(), String> {
        config.validate()?;
        if name.trim().is_empty() || name.chars().count() > 60 {
            return Err("样式名称必填且最多 60 字。".into());
        }
        let mut style = serde_json::to_value(config).map_err(|e| e.to_string())?;
        style.as_object_mut().unwrap().retain(|key, _| {
            matches!(
                key.as_str(),
                "templateId"
                    | "brand"
                    | "fontFamily"
                    | "titleSize"
                    | "subtitleSize"
                    | "align"
                    | "position"
                    | "backgroundColor"
                    | "textColor"
                    | "subtitleColor"
                    | "accentColor"
                    | "overlayOpacity"
            )
        });
        self.with_connection(|c| {
            c.execute(&format!("INSERT INTO cover_presets(id,name,style_json,created_at) VALUES(?1,?2,?3,{NOW})"),params![new_id(c)?,name.trim(),style.to_string()]).map_err(|e| match e { rusqlite::Error::SqliteFailure(ref code,_) if code.code==rusqlite::ErrorCode::ConstraintViolation => "已有同名样式，请使用新名称。".into(), _=>e.to_string() })?;
            Ok(())
        })
    }
    pub fn delete_cover_preset(&self, id: &str) -> Result<(), String> {
        self.with_connection(|c| {
            c.execute("DELETE FROM cover_presets WHERE id=?1", [id])
                .map_err(|e| e.to_string())?;
            Ok(())
        })
    }
    pub fn read_cover_export(&self, project_id: &str, id: &str) -> Result<Vec<u8>, String> {
        let (path,config): (String,String) = self.with_connection(|c| c.query_row("SELECT exported_path,config_json FROM cover_versions WHERE id=?1 AND project_id=?2 AND exported_path IS NOT NULL",params![id,project_id],|r|Ok((r.get(0)?,r.get(1)?))).map_err(|_|"已导出的封面不存在。".to_owned()))?;
        let config: CoverConfig = serde_json::from_str(&config).map_err(|e| e.to_string())?;
        let mut bytes = Vec::new();
        std::fs::File::open(path)
            .map_err(|e| e.to_string())?
            .take((MAX_IMAGE_BYTES + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        validate_png(&bytes, &config)?;
        Ok(bytes)
    }
    pub fn list_cover_versions(&self, project_id: &str) -> Result<Vec<CoverVersion>, String> {
        self.with_connection(|c| {
            get(c, project_id)?;
            let mut query = c.prepare("SELECT id,version_no,config_json,exported_path,created_at FROM cover_versions WHERE project_id=?1 ORDER BY version_no DESC").map_err(|e| e.to_string())?;
            let rows = query.query_map([project_id], |r| Ok((r.get::<_, String>(0)?, r.get::<_, u32>(1)?, r.get::<_, String>(2)?, r.get::<_, Option<String>>(3)?, r.get::<_, String>(4)?))).map_err(|e| e.to_string())?;
            rows.map(|row| {
                let (id, version_no, json, exported_path, created_at) = row.map_err(|e| e.to_string())?;
                let config = serde_json::from_str(&json).map_err(|_| "封面配置无法读取。")?;
                Ok(CoverVersion { id, project_id: project_id.into(), version_no, config, exported_path, created_at })
            }).collect()
        })
    }

    pub fn read_cover_background(
        &self,
        project_id: &str,
        asset_id: &str,
    ) -> Result<Vec<u8>, String> {
        let path = self.with_connection(|c| background_path(c, project_id, asset_id))?;
        let extension = Path::new(&path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        if !matches!(extension.as_str(), "png" | "jpg" | "jpeg" | "webp") {
            return Err("背景仅支持 PNG、JPEG 或 WebP。".into());
        }
        let file =
            std::fs::File::open(path).map_err(|_| "背景图不存在或无法读取，请重新选择图片。")?;
        let mut bytes = Vec::new();
        file.take((MAX_IMAGE_BYTES + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|_| "背景图读取失败。")?;
        if bytes.len() > MAX_IMAGE_BYTES {
            return Err("背景图不能超过 15 MB。".into());
        }
        let valid = bytes.starts_with(b"\x89PNG\r\n\x1a\n")
            || bytes.starts_with(b"\xff\xd8\xff")
            || (bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP"));
        if !valid {
            return Err("背景图片格式无效。".into());
        }
        Ok(bytes)
    }

    pub fn save_cover_version(
        &self,
        project_id: &str,
        config: CoverConfig,
        png: Option<&[u8]>,
    ) -> Result<CoverVersion, String> {
        config.validate()?;
        if let Some(bytes) = png {
            validate_png(bytes, &config)?;
        }
        self.with_connection(|c| {
            let tx = c.transaction().map_err(|e| e.to_string())?;
            let project = get(&tx, project_id)?;
            // Saving a draft is allowed even when its background file is missing.
            if let Some(id) = &config.background_asset_id { background_path(&tx, project_id, id)?; }
            let id = new_id(&tx)?;
            let version_no: u32 = tx.query_row("SELECT coalesce(max(version_no),0)+1 FROM cover_versions WHERE project_id=?1", [project_id], |r| r.get(0)).map_err(|e| e.to_string())?;
            let mut created_file = None;
            if let Some(bytes) = png {
                let directory = Path::new(&project.workspace_path).join("covers");
                std::fs::create_dir_all(&directory).map_err(|e| format!("封面目录不可写：{e}"))?;
                let path = directory.join(format!("cover-{}x{}-v{version_no}-{id}.png", config.width, config.height));
                let mut file = OpenOptions::new().write(true).create_new(true).open(&path).map_err(|e| format!("封面导出失败：{e}"))?;
                let written = file.write_all(bytes).and_then(|()| file.sync_all());
                drop(file);
                if let Err(error) = written {
                    let _ = std::fs::remove_file(&path);
                    return Err(format!("封面导出失败：{error}"));
                }
                created_file = Some(path);
            }
            let exported_path = created_file.as_ref().map(|p| p.to_string_lossy().into_owned());
            let result = (|| {
                let created_at: String = tx.query_row(&format!("SELECT {NOW}"), [], |r| r.get(0)).map_err(|e| e.to_string())?;
                let json = serde_json::to_string(&config).map_err(|e| e.to_string())?;
                tx.execute("INSERT INTO cover_versions(id,project_id,version_no,config_json,exported_path,created_at) VALUES(?1,?2,?3,?4,?5,?6)", params![id,project_id,version_no,json,exported_path,created_at]).map_err(|e| e.to_string())?;
                tx.execute("UPDATE book_projects SET updated_at=?1 WHERE id=?2", params![created_at,project_id]).map_err(|e| e.to_string())?;
                tx.commit().map_err(|e| e.to_string())?;
                Ok(CoverVersion { id, project_id: project_id.into(), version_no, config, exported_path, created_at })
            })();
            if result.is_err() {
                if let Some(path) = created_file { let _ = std::fs::remove_file(path); }
            }
            result
        })
    }
}
