use crate::models::LocalSkill;
use crate::services::scan_service;

#[tauri::command]
pub fn get_scan_roots() -> Result<Vec<String>, String> {
  scan_service::get_scan_roots()
}

#[tauri::command]
pub fn add_scan_root(path: String) -> Result<Vec<String>, String> {
  scan_service::add_scan_root(path)
}

#[tauri::command]
pub fn remove_scan_root(path: String) -> Result<Vec<String>, String> {
  scan_service::remove_scan_root(path)
}

#[tauri::command]
pub async fn scan_local_skills() -> Result<Vec<LocalSkill>, String> {
  scan_service::scan_local_skills()
}
