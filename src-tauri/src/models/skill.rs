use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LocalSkill {
  pub name: String,
  pub source: Option<String>,
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
  #[serde(skip_serializing_if = "Option::is_none")]
  pub description: Option<String>,
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

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum InstallScope {
  #[default]
  Global,
  Project,
}

#[derive(Clone, Debug)]
pub enum InstallTarget {
  Global,
  Project(PathBuf),
}

impl InstallTarget {
  pub fn validate_install_scope(
    scope: &InstallScope,
    project_path: &Option<String>,
  ) -> Result<(), String> {
    let _ = Self::from_scope(scope, project_path)?;
    Ok(())
  }

  pub fn from_scope(scope: &InstallScope, project_path: &Option<String>) -> Result<Self, String> {
    match scope {
      InstallScope::Global => Ok(InstallTarget::Global),
      InstallScope::Project => Ok(InstallTarget::Project(Self::normalize_project_root(project_path)?)),
    }
  }

  fn normalize_project_root(project_path: &Option<String>) -> Result<PathBuf, String> {
    let project_path = project_path
      .as_ref()
      .map(|item| item.trim().to_string())
      .filter(|item| !item.is_empty())
      .ok_or("project_path is required for project scope".to_string())?;

    let path = PathBuf::from(project_path);
    if !path.exists() || !path.is_dir() {
      return Err(format!("Project path does not exist: {}", path.to_string_lossy()));
    }
    Ok(path)
  }

  pub fn root_path(&self) -> Result<PathBuf, String> {
    match self {
      InstallTarget::Project(root) => Ok(root.clone()),
      InstallTarget::Global => dirs_next::home_dir().ok_or("Could not find home directory".to_string()),
    }
  }

  pub fn skill_folder_path(&self) -> Result<PathBuf, String> {
    Ok(self.root_path()?.join(".agents").join("skills"))
  }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InstallNativeRequest {
  pub name: String,
  pub tmp_path: String,
  pub skill_path: String,
  pub agent_apps: Vec<String>,
  pub method: InstallMethod,
  #[serde(default)]
  pub scope: InstallScope,
  #[serde(default)]
  pub project_path: Option<String>,
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
  #[serde(default)]
  pub scope: InstallScope,
  #[serde(default)]
  pub project_path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InstallUnknownRequest {
  pub name: String,
  pub source_path: String,
  pub agent_apps: Vec<String>,
  pub method: InstallMethod,
  #[serde(default)]
  pub scope: InstallScope,
  #[serde(default)]
  pub project_path: Option<String>,
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
  #[serde(default)]
  pub scope: InstallScope,
  #[serde(default)]
  pub project_path: Option<String>,
}

#[derive(Clone, Debug)]
pub struct SelectedAgentPath {
  pub display_name: String,
  pub install_root: PathBuf,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SkillUpdateCheckItem {
  pub name: String,
  pub source: String,
  pub remote_sha: String,
}
