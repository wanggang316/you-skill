use crate::models::LocalSkill;
use crate::services::scan_service;

#[tauri::command]
pub async fn scan_local_skills() -> Result<Vec<LocalSkill>, String> {
  scan_service::scan_local_skills()
}
