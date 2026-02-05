use crate::models::AgentInfo;
use crate::paths::agent_infos;

#[tauri::command]
pub fn list_agents() -> Result<Vec<AgentInfo>, String> {
  Ok(agent_infos())
}
