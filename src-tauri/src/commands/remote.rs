use crate::models::{RemoteSkill, RemoteSkillsResponse};
use crate::services::remote_service;

#[tauri::command]
pub async fn fetch_remote_skills(
  skip: Option<u32>,
  limit: Option<u32>,
  search: Option<String>,
  sort_by: Option<String>,
  sort_order: Option<String>,
) -> Result<RemoteSkillsResponse, String> {
  remote_service::fetch_remote_skills(skip, limit, search, sort_by, sort_order).await
}

#[tauri::command]
pub async fn fetch_skills_by_names(names: Vec<String>) -> Result<Vec<RemoteSkill>, String> {
  remote_service::fetch_skills_by_names(names).await
}

#[tauri::command]
pub async fn record_skill_install(skill_id: String) -> Result<(), String> {
  remote_service::record_skill_install(skill_id).await
}
