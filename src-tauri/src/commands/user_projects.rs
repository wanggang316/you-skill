use crate::models::UserProject;
use crate::services::user_projects_service;

#[tauri::command]
pub fn list_user_projects() -> Result<Vec<UserProject>, String> {
  user_projects_service::list_user_projects()
}

#[tauri::command]
pub fn add_user_project(name: String, path: String) -> Result<UserProject, String> {
  user_projects_service::add_user_project(name, path)
}

#[tauri::command]
pub fn update_user_project(
  original_name: String,
  name: String,
  path: String,
) -> Result<UserProject, String> {
  user_projects_service::update_user_project(original_name, name, path)
}

#[tauri::command]
pub fn remove_user_project(name: String) -> Result<(), String> {
  user_projects_service::remove_user_project(name)
}
