use crate::models::{DetectedSkill, InstallGithubRequest, InstallNativeRequest, InstallResult};
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
pub async fn detect_github_auto(
  github_path: String,
  skill_name: String,
) -> Result<DetectedSkill, String> {
  skill_service::detect_github_auto(github_path, skill_name)
}

#[tauri::command]
pub fn install_from_native(request: InstallNativeRequest) -> Result<InstallResult, String> {
  skill_service::install_from_native(request)
}

#[tauri::command]
pub fn install_from_github(request: InstallGithubRequest) -> Result<InstallResult, String> {
  skill_service::install_from_github(request)
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
