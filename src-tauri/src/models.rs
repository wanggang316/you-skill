use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LocalSkill {
  pub name: String,
  pub description: Option<String>,
  pub scope: String,
  pub canonical_path: String,
  pub agents: Vec<String>,
  pub managed_status: String,
  pub name_conflict: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AgentInfo {
  pub id: String,
  pub display_name: String,
  pub project_path: Option<String>,
  pub global_path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RemoteSkill {
  pub id: String,
  pub skill_id: String,
  pub name: String,
  pub installs: u64,
  pub source: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RemoteSkillsResponse {
  pub skills: Vec<RemoteSkill>,
  pub has_more: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InstallRequest {
  pub source: String,
  pub skill_id: String,
  pub agent: String,
  pub global: bool,
  pub project_dir: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InstallResult {
  pub success: bool,
  pub stdout: String,
  pub stderr: String,
  pub message: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CanonicalCheckResult {
  pub exists: bool,
  pub canonical_path: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UnifyRequest {
  pub name: String,
  pub agent: String,
  pub scope: String,
  pub current_path: String,
  pub prefer: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UnifyResult {
  pub success: bool,
  pub message: String,
}
