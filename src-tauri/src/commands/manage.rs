use crate::services::manage_service;

#[tauri::command]
pub fn set_agent_link(
  name: String,
  agent: String,
  scope: String,
  linked: bool,
) -> Result<(), String> {
  manage_service::set_agent_link(name, agent, scope, linked)
}
