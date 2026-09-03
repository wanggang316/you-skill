use crate::models::{ProjectCandidate, ProjectRegistration, UserProject, UserWorkspace};
use crate::services::env::Env;
use crate::services::user_projects_service;
use crate::services::workspace_service::{self, DEFAULT_WORKSPACE_DEPTH};

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

#[tauri::command]
pub fn list_workspaces() -> Result<Vec<UserWorkspace>, String> {
  workspace_service::list_workspaces()
}

#[tauri::command]
pub fn add_workspace(name: String, path: String) -> Result<UserWorkspace, String> {
  let env = Env::current()?;
  workspace_service::add_workspace(&env, name, path)
}

#[tauri::command]
pub fn remove_workspace(path: String, remove_projects: bool) -> Result<(), String> {
  workspace_service::remove_workspace(&path, remove_projects)
}

/// Look for project folders inside a workspace: a folder with an agent skills directory or
/// an agent instruction file (`AGENTS.md`, `CLAUDE.md`).
#[tauri::command]
pub async fn scan_workspace(
  path: String,
  max_depth: Option<usize>,
) -> Result<Vec<ProjectCandidate>, String> {
  let env = Env::current()?;
  tauri::async_runtime::spawn_blocking(move || {
    workspace_service::scan_workspace(&env, &path, max_depth.unwrap_or(DEFAULT_WORKSPACE_DEPTH))
  })
  .await
  .map_err(|e| format!("scan_workspace join error: {}", e))?
}

#[tauri::command]
pub fn register_projects(projects: Vec<ProjectRegistration>) -> Result<Vec<UserProject>, String> {
  workspace_service::register_projects(projects)
}
