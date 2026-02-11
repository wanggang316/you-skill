use crate::models::{CanonicalCheckResult, UnifyRequest, UnifyResult};
use crate::services::manage_service;

#[tauri::command]
pub fn delete_skill(path: String) -> Result<(), String> {
  manage_service::delete_skill(path)
}

#[tauri::command]
pub fn delete_skill_complete(
  canonical_path: String,
  scope: String,
  agents: Vec<String>,
) -> Result<(), String> {
  manage_service::delete_skill_complete(canonical_path, scope, agents)
}

#[tauri::command]
pub fn move_skill(from: String, to: String) -> Result<(), String> {
  manage_service::move_skill(from, to)
}

#[tauri::command]
pub fn copy_skill(from: String, to: String) -> Result<(), String> {
  manage_service::copy_skill(from, to)
}

#[tauri::command]
pub fn check_canonical_skill(name: String, scope: String) -> Result<CanonicalCheckResult, String> {
  manage_service::check_canonical_skill(name, scope)
}

#[tauri::command]
pub fn unify_skill(request: UnifyRequest) -> Result<UnifyResult, String> {
  manage_service::unify_skill(request)
}

#[tauri::command]
pub fn set_agent_link(
  name: String,
  agent: String,
  scope: String,
  linked: bool,
) -> Result<(), String> {
  manage_service::set_agent_link(name, agent, scope, linked)
}
