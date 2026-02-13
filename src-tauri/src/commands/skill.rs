use crate::models::{
  DetectedSkill, InstallFolderRequest, InstallGithubRequest, InstallResult, InstallZipRequest,
};
use crate::services::{install_service, skill_service};

#[tauri::command]
pub async fn detect_github_skills(url: String) -> Result<Vec<DetectedSkill>, String> {
  install_service::detect_github_skills(url)
}

#[tauri::command]
pub fn install_package(request: InstallZipRequest) -> Result<InstallResult, String> {
  skill_service::install_package(request.zip_path, request.skill_path, request.agents)
}

#[tauri::command]
pub async fn install_github_manual(request: InstallGithubRequest) -> Result<InstallResult, String> {
  skill_service::install_github_manual(request.url, request.skill_path, request.agents)
}

#[tauri::command]
pub async fn install_github_auto(request: InstallGithubRequest) -> Result<InstallResult, String> {
  skill_service::install_github_auto(
    request.url,
    request.skill_path,
    request.agents,
    request.skill_folder_hash,
  )
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
pub fn install_folder(request: InstallFolderRequest) -> Result<InstallResult, String> {
  skill_service::install_folder(request.folder_path, request.skill_path, request.agents)
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
