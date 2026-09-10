use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserProject {
  pub name: String,
  pub path: String,
  /// Workspace this project was discovered in; `None` when it was added by hand.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub workspace_path: Option<String>,
}

/// A registered project as listed to the UI, with whether its folder still exists.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserProjectView {
  #[serde(flatten)]
  pub project: UserProject,
  /// The folder was deleted or renamed since the project was added.
  pub missing: bool,
}

/// A folder that holds projects. Scanning it registers the projects it contains.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserWorkspace {
  pub name: String,
  pub path: String,
}

/// A folder inside a workspace that looks like a project: it has an agent's project skills
/// directory, or an agent instruction file (`AGENTS.md`, `CLAUDE.md`).
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProjectCandidate {
  pub name: String,
  pub path: String,
  /// Agents whose project skills directory exists here.
  #[serde(default)]
  pub agent_ids: Vec<String>,
  /// Instruction files found here, relative to the project.
  #[serde(default)]
  pub profiles: Vec<String>,
  /// Number of skills in the agent skills directories.
  pub skill_count: usize,
  /// Already in the project list.
  pub registered: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProjectRegistration {
  pub name: String,
  pub path: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub workspace_path: Option<String>,
}
