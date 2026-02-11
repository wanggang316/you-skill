use crate::models::{
  DetectedSkill, InstallGithubRequest, InstallRequest, InstallResult, InstallZipRequest,
};
use crate::services::install_service;

#[tauri::command]
pub fn install_skill(request: InstallRequest) -> Result<InstallResult, String> {
  install_service::install_skill(
    request.source,
    request.skill_id,
    request.agent,
    request.global,
    request.project_dir,
  )
}

#[tauri::command]
pub async fn detect_github_skills(url: String) -> Result<Vec<DetectedSkill>, String> {
  install_service::detect_github_skills(url)
}

#[tauri::command]
pub fn install_zip_skill(request: InstallZipRequest) -> Result<InstallResult, String> {
  install_service::install_zip_skill(request.zip_path, request.skill_path, request.agents)
}

#[tauri::command]
pub async fn install_github_skill(request: InstallGithubRequest) -> Result<InstallResult, String> {
  install_service::install_github_skill(request.url, request.skill_path, request.agents)
}

#[tauri::command]
pub fn detect_zip_skills(zip_path: String) -> Result<Vec<DetectedSkill>, String> {
  install_service::detect_zip_skills(zip_path)
}

#[tauri::command]
pub fn detect_folder_skills(folder_path: String) -> Result<Vec<DetectedSkill>, String> {
  install_service::detect_folder_skills(folder_path)
}

#[tauri::command]
pub fn install_folder_skill(
  request: crate::models::InstallFolderRequest,
) -> Result<InstallResult, String> {
  install_service::install_folder_skill(request.folder_path, request.skill_path, request.agents)
}
