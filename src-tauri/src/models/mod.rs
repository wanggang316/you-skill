pub mod agent_app;

pub use agent_app::AgentApp;

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
  pub created_at: Option<i64>,
  pub conflict_with_managed: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RemoteSkill {
  pub id: String,
  pub skill_id: String,
  pub name: String,
  pub star_count: u64,
  pub heat_score: u64,
  pub install_count: u64,
  pub source: String,
  pub url: Option<String>,
  pub path: Option<String>,
  /// GitHub tree SHA for the skill folder (for update detection)
  #[serde(skip_serializing_if = "Option::is_none")]
  pub skill_path_sha: Option<String>,
  /// Git branch name (e.g., "main", "master")
  #[serde(skip_serializing_if = "Option::is_none")]
  pub branch: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RemoteSkillsResponse {
  pub skills: Vec<RemoteSkill>,
  pub total: i64,
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DetectedSkill {
  pub name: String,
  pub path: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InstallZipRequest {
  pub zip_path: String,
  pub skill_path: String,
  pub agents: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InstallGithubRequest {
  pub url: String,
  pub skill_path: String,
  pub agents: Vec<String>,
  pub skill_folder_hash: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InstallFolderRequest {
  pub folder_path: String,
  pub skill_path: String,
  pub agents: Vec<String>,
}
