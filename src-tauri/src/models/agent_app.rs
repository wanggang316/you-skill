use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AgentApp {
  pub id: String,
  pub display_name: String,
  pub project_path: Option<String>,
  pub global_path: Option<String>,
  pub is_user_custom: bool,
}
