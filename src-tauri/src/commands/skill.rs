use crate::models::{
  DetectedSkill, InstallGithubRequest, InstallNativeRequest, InstallResult, InstallUnknownRequest,
  InstallScope, LocalSkill, ManageSkillAgentAppsRequest, SkillDirectoryEntry, SkillUpdateCheckItem,
  SourceCheckResult,
};
use crate::services::skill_service;

#[tauri::command]
pub fn list_skills(scope: InstallScope, project_path: Option<String>) -> Result<Vec<LocalSkill>, String> {
  skill_service::list_skills(scope, project_path)
}

#[tauri::command]
pub fn delete_skill(name: String, scope: InstallScope, project_path: Option<String>) -> Result<(), String> {
  skill_service::delete_skill(name, scope, project_path)
}

#[tauri::command]
pub fn detect_zip(zip_path: String) -> Result<Vec<DetectedSkill>, String> {
  skill_service::detect_zip(zip_path)
}

#[tauri::command]
pub fn detect_folder(folder_path: String) -> Result<Vec<DetectedSkill>, String> {
  skill_service::detect_folder(folder_path)
}

#[tauri::command]
pub async fn detect_github_manual(github_path: String) -> Result<Vec<DetectedSkill>, String> {
  skill_service::detect_github_manual(github_path).await
}

#[tauri::command]
pub async fn detect_github_auto(
  github_path: String,
  skill_name: String,
) -> Result<DetectedSkill, String> {
  skill_service::detect_github_auto(github_path, skill_name).await
}

#[tauri::command]
pub async fn install_from_native(request: InstallNativeRequest) -> Result<InstallResult, String> {
  tauri::async_runtime::spawn_blocking(move || skill_service::install_from_native(request))
    .await
    .map_err(|e| format!("install_from_native join error: {}", e))?
}

#[tauri::command]
pub async fn install_from_github(request: InstallGithubRequest) -> Result<InstallResult, String> {
  skill_service::install_from_github(request).await
}

#[tauri::command]
pub fn check_skill_version(
  name: String,
  root_folder: Option<String>,
  skill_paths: Vec<String>,
) -> Result<SourceCheckResult, String> {
  skill_service::check_skill_version(name, root_folder, skill_paths)
}

#[tauri::command]
pub async fn install_from_unknown(request: InstallUnknownRequest) -> Result<InstallResult, String> {
  tauri::async_runtime::spawn_blocking(move || skill_service::install_from_unknown(request))
    .await
    .map_err(|e| format!("install_from_unknown join error: {}", e))?
}

#[tauri::command]
pub async fn manage_skill_agent_apps(
  request: ManageSkillAgentAppsRequest,
) -> Result<InstallResult, String> {
  tauri::async_runtime::spawn_blocking(move || skill_service::manage_skill_agent_apps(request))
    .await
    .map_err(|e| format!("manage_skill_agent_apps join error: {}", e))?
}

#[tauri::command]
pub fn open_in_file_manager(file_path: String) -> Result<(), String> {
  skill_service::open_in_file_manager(file_path)
}

#[tauri::command]
pub async fn read_skill_file(skill_path: String) -> Result<String, String> {
  skill_service::read_skill_file(skill_path).await
}

#[tauri::command]
pub fn list_skill_directory(skill_path: String) -> Result<Vec<SkillDirectoryEntry>, String> {
  skill_service::list_skill_directory(skill_path)
}

#[tauri::command]
pub async fn read_skill_relative_file(
  skill_path: String,
  relative_path: String,
) -> Result<String, String> {
  skill_service::read_skill_relative_file(skill_path, relative_path).await
}

#[tauri::command]
pub fn check_skills_updates(checks: Vec<SkillUpdateCheckItem>) -> Result<Vec<String>, String> {
  skill_service::check_skills_updates(checks)
}
