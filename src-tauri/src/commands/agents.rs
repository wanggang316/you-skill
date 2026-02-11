use crate::models::AgentInfo;
use crate::services::agent_apps_service::local_agent_apps;

#[tauri::command]
pub fn list_agents() -> Result<Vec<AgentInfo>, String> {
  Ok(local_agent_apps())
}
