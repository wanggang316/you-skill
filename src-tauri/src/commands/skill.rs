use crate::models::{
  DetectedSkill, InstallRequest, InstallResult,
};
use crate::services::skill_service;

#[tauri::command]
pub fn detect_zip(zip_path: String) -> Result<DetectedSkill, String> {
  skill_service::detect_zip(zip_path)
}

#[tauri::command]
pub fn detect_folder(folder_path: String) -> Result<DetectedSkill, String> {
  skill_service::detect_folder(folder_path)
}

#[tauri::command]
pub async fn detect_github_manual(github_path: String) -> Result<Vec<DetectedSkill>, String> {
  skill_service::detect_github_manual(github_path)
}

#[tauri::command]
pub async fn detect_github_auto(github_path: String) -> Result<DetectedSkill, String> {
  skill_service::detect_github_auto(github_path)
}

#[tauri::command]
pub fn install(request: InstallRequest) -> Result<InstallResult, String> {
  skill_service::install(request)
}

#[tauri::command]
pub fn open_in_file_manager(file_path: String) -> Result<(), String> {
  skill_service::open_in_file_manager(file_path)
}

#[tauri::command]
pub async fn read_skill_readme(skill_path: String) -> Result<String, String> {
  skill_service::read_skill_readme(skill_path).await
}

#[tauri::command]
pub fn check_skill_update(skill_name: String, remote_sha: String) -> Result<bool, String> {
  skill_service::check_skill_update(skill_name, remote_sha)
}
