use crate::models::AgentApp;
use crate::services::agent_apps_service::{
  create_user_agent_app, delete_user_agent_app_by_id, list_internal_agent_app_details,
  list_user_agent_app_details, local_agent_apps, refresh_local_agent_apps,
  update_user_agent_app_detail,
};

/// List local agent apps (installed on system)
#[tauri::command]
pub fn list_local_agent_apps() -> Result<Vec<AgentApp>, String> {
  Ok(local_agent_apps())
}

/// Refresh agent apps - clears cache and re-scans filesystem
#[tauri::command]
pub fn refresh_agent_apps() -> Result<Vec<AgentApp>, String> {
  refresh_local_agent_apps();
  Ok(local_agent_apps())
}

/// Add a user agent app
#[tauri::command]
pub fn add_user_agent_app(
  display_name: String,
  global_path: String,
  project_path: Option<String>,
) -> Result<AgentApp, String> {
  create_user_agent_app(display_name, global_path, project_path)
}

/// Remove a user agent app
#[tauri::command]
pub fn remove_user_agent_app(id: String) -> Result<(), String> {
  delete_user_agent_app_by_id(&id)
}

/// Update a user agent app
#[tauri::command]
pub fn update_user_agent_app(
  id: String,
  display_name: String,
  global_path: String,
  project_path: Option<String>,
) -> Result<AgentApp, String> {
  update_user_agent_app_detail(id, display_name, global_path, project_path)
}

/// Get internal agent apps
#[tauri::command]
pub fn list_internal_agent_apps() -> Result<Vec<AgentApp>, String> {
  Ok(list_internal_agent_app_details())
}

/// Get user agent apps
#[tauri::command]
pub fn list_user_agent_apps() -> Result<Vec<AgentApp>, String> {
  Ok(list_user_agent_app_details())
}
