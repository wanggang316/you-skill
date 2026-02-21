use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LocalSkill {
  pub name: String,
  pub global_folder: Option<String>,
  pub installed_agent_apps: Vec<InstalledAgentApp>,
  pub source_type: SourceType,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SourceType {
  Github,
  Native,
  Unknown,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InstalledAgentApp {
  pub id: String,
  pub skill_folder: String,
  pub method: InstallMethod,
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
pub struct InstallResult {
  pub success: bool,
  pub stdout: String,
  pub stderr: String,
  pub message: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DetectedSkill {
  pub name: String,
  pub tmp_path: String,
  pub skill_path: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum InstallMethod {
  Symlink,
  Copy,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InstallNativeRequest {
  pub name: String,
  pub tmp_path: String,
  pub skill_path: String,
  pub agent_apps: Vec<String>,
  pub method: InstallMethod,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InstallGithubRequest {
  pub name: String,
  pub tmp_path: String,
  pub skill_path: String,
  pub source_url: String,
  pub skill_folder_hash: Option<String>,
  pub agent_apps: Vec<String>,
  pub method: InstallMethod,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InstallUnknownRequest {
  pub name: String,
  pub source_path: String,
  pub agent_apps: Vec<String>,
  pub method: InstallMethod,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SourceCheckResult {
  pub source_path: Option<String>,
  pub version_groups: Vec<SourceVersionGroup>,
  pub requires_selection: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SourceVersionGroup {
  pub version: String,
  pub source_path: String,
  pub paths: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ManageSkillAgentAppsRequest {
  pub name: String,
  pub source_type: SourceType,
  pub agent_apps: Vec<String>,
  pub method: InstallMethod,
  pub source_path: String,
}
