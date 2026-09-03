use crate::models::{DetectedSkill, SkillDirectoryEntry};
use crate::services::{skill_service, translate_service};

#[tauri::command]
pub async fn detect_zip(zip_path: String) -> Result<Vec<DetectedSkill>, String> {
  tauri::async_runtime::spawn_blocking(move || skill_service::detect_zip(zip_path))
    .await
    .map_err(|e| format!("detect_zip join error: {}", e))?
}

#[tauri::command]
pub async fn detect_folder(folder_path: String) -> Result<Vec<DetectedSkill>, String> {
  tauri::async_runtime::spawn_blocking(move || skill_service::detect_folder(folder_path))
    .await
    .map_err(|e| format!("detect_folder join error: {}", e))?
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
pub async fn read_skill_relative_file_bytes(
  skill_path: String,
  relative_path: String,
) -> Result<Vec<u8>, String> {
  skill_service::read_skill_relative_file_bytes(skill_path, relative_path).await
}

#[tauri::command]
pub async fn translate_skill_markdown(markdown: String) -> Result<String, String> {
  translate_service::translate_skill_markdown(markdown).await
}
