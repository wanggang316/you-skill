use crate::services::agent_apps_service::local_agent_apps;
use crate::models::AgentInfo;

#[tauri::command]
pub fn list_agents() -> Result<Vec<AgentInfo>, String> {
  Ok(local_agent_apps())
}
