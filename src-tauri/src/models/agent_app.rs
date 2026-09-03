use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AgentApp {
  pub id: String,
  pub display_name: String,
  pub project_path: Option<String>,
  pub global_path: Option<String>,
  /// Directory whose presence means the app is installed (e.g. `~/.cursor`). Needed when
  /// `global_path` is the shared `~/.agents/skills`, which says nothing about the app.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub detect_path: Option<String>,
  #[serde(default)]
  pub is_user_custom: bool,
}
