use super::{
    ai::ProviderConfig,
    projects::{get, new_id, NOW},
    Asset, Database,
};
use crate::ai::images::{validate_image, PROMPT_VERSION, SYSTEM_PROMPT};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::{fs::OpenOptions, io::Write, path::Path, time::UNIX_EPOCH};

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageGenerationInput {
    pub project_id: String,
    pub provider_id: String,
    pub description: String,
}
pub struct PreparedImage {
    pub provider: ProviderConfig,
    pub prompt: String,
}
impl Database {
    pub fn start_cover_generation(
        &self,
        mut input: ImageGenerationInput,
    ) -> Result<String, String> {
        input.description = input.description.trim().into();
        if input.description.is_empty() || input.description.chars().count() > 2000 {
            return Err("请填写画面描述，最多 2000 字。".into());
        }
        self.with_connection(|c| {
            get(c, &input.project_id)?;
            let provider = super::ai::provider(c, &input.provider_id)?;
            if !provider.input.is_enabled || provider.input.model_kind != "image" {
                return Err("请选择已启用的图片模型。".into());
            }
            let id = new_id(c)?;
            c.execute(&format!("INSERT INTO ai_tasks(id,project_id,kind,provider_id,model_name,prompt_version,status,input_json,created_at) VALUES(?1,?2,'cover',?3,?4,?5,'pending',?6,{NOW})"),
                params![id,input.project_id,input.provider_id,provider.input.model_name,PROMPT_VERSION,serde_json::to_string(&input).map_err(|e|e.to_string())?]).map_err(|e|e.to_string())?;
            Ok(id)
        })
    }
    pub fn prepare_cover_generation(&self, id: &str) -> Result<PreparedImage, String> {
        self.with_connection(|c| {
            let json: String = c.query_row("SELECT input_json FROM ai_tasks WHERE id=?1 AND kind='cover' AND status='pending'",[id],|r|r.get(0)).map_err(|_|"图片任务已取消、已运行或不存在。")?;
            let input: ImageGenerationInput = serde_json::from_str(&json).map_err(|_|"图片任务输入无效。")?;
            let project = get(c, &input.project_id)?;
            let provider = super::ai::provider(c, &input.provider_id)?;
            if !provider.input.is_enabled || provider.input.model_kind != "image" { return Err("请选择已启用的图片模型。".into()); }
            let data = serde_json::json!({"bookTitle":project.input.title,"author":project.input.author,"visualDescription":input.description});
            c.execute("UPDATE ai_tasks SET status='running',model_name=?2 WHERE id=?1", params![id,provider.input.model_name]).map_err(|e|e.to_string())?;
            Ok(PreparedImage { provider, prompt: format!("{SYSTEM_PROMPT}\n{data}") })
        })
    }
    pub fn finish_cover_generation(&self, task_id: &str, bytes: &[u8]) -> Result<Asset, String> {
        let extension = validate_image(bytes).map_err(|e| e.message)?;
        self.with_connection(|c| {
            let tx = c.transaction().map_err(|e|e.to_string())?;
            let project_id: String = tx.query_row("SELECT project_id FROM ai_tasks WHERE id=?1 AND kind='cover' AND status='running'", [task_id], |r|r.get(0)).map_err(|_|"生成已取消或结束，图片未写入。")?;
            let project = get(&tx, &project_id)?;
            let id = new_id(&tx)?;
            let name = format!("ai-background-{id}.{extension}");
            let directory = Path::new(&project.workspace_path).join("covers");
            std::fs::create_dir_all(&directory).map_err(|e|format!("封面目录不可写：{e}"))?;
            let path = directory.join(&name);
            let mut file = OpenOptions::new().write(true).create_new(true).open(&path).map_err(|e|format!("背景保存失败：{e}"))?;
            let written = file.write_all(bytes).and_then(|()|file.sync_all());
            drop(file);
            let result = (|| {
                written.map_err(|e|format!("背景保存失败：{e}"))?;
                let modified = std::fs::metadata(&path).and_then(|m|m.modified()).map_err(|e|e.to_string())?.duration_since(UNIX_EPOCH).map_err(|e|e.to_string())?.as_secs() as i64;
                let created_at: String = tx.query_row(&format!("SELECT {NOW}"),[],|r|r.get(0)).map_err(|e|e.to_string())?;
                let file_path = path.to_string_lossy().into_owned();
                tx.execute("INSERT INTO assets(id,project_id,asset_type,file_path,original_name,file_size,modified_at,created_at) VALUES(?1,?2,'image',?3,?4,?5,?6,?7)",params![id,project_id,file_path,name,bytes.len() as i64,modified,created_at]).map_err(|e|e.to_string())?;
                tx.execute("UPDATE ai_tasks SET status='succeeded',asset_id=?2,completed_at=?3 WHERE id=?1",params![task_id,id,created_at]).map_err(|e|e.to_string())?;
                tx.execute("UPDATE book_projects SET updated_at=?2 WHERE id=?1",params![project_id,created_at]).map_err(|e|e.to_string())?;
                tx.commit().map_err(|e|e.to_string())?;
                Ok(Asset { id, project_id, asset_type: "image".into(), file_path, original_name: name, file_size: bytes.len() as i64, modified_at: modified, exists_status: "available".into(), created_at })
            })();
            if result.is_err() { let _ = std::fs::remove_file(path); }
            result
        })
    }
}
