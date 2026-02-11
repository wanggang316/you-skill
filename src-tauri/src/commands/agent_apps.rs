use crate::agent_apps::{
  add_user_agent_app, all_agent_apps, check_global_path_exists, generate_id_from_display_name,
  get_agent_app, local_agent_apps, refresh_local_agent_apps, remove_user_agent_app, UserAgentApp,
};
use crate::models::AgentInfo;

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct AgentAppDetail {
  pub id: String,
  pub display_name: String,
  pub project_path: Option<String>,
  pub global_path: Option<String>,
  pub is_internal: bool,
  pub is_installed: bool,
}

impl From<crate::agent_apps::AgentApp> for AgentAppDetail {
  fn from(app: crate::agent_apps::AgentApp) -> Self {
    AgentAppDetail {
      id: app.id,
      display_name: app.display_name,
      project_path: app.project_path,
      global_path: app.global_path,
      is_internal: app.is_internal,
      is_installed: false,
    }
  }
}

/// List all agent apps (internal + user)
#[tauri::command]
pub fn list_all_agent_apps() -> Result<Vec<AgentAppDetail>, String> {
  let all_apps = all_agent_apps();
  let local_apps_ids: std::collections::HashSet<String> =
    local_agent_apps().into_iter().map(|app| app.id).collect();

  Ok(
    all_apps
      .into_iter()
      .map(|app| {
        let is_installed = local_apps_ids.contains(&app.id);
        let detail: AgentAppDetail = app.into();
        AgentAppDetail {
          is_installed,
          ..detail
        }
      })
      .collect(),
  )
}

/// List local agent apps (installed on system)
#[tauri::command]
pub fn list_local_agent_apps() -> Result<Vec<AgentInfo>, String> {
  Ok(local_agent_apps())
}

/// Refresh agent apps - clears cache and re-scans filesystem
#[tauri::command]
pub fn refresh_agent_apps() -> Result<Vec<AgentInfo>, String> {
  refresh_local_agent_apps();
  Ok(local_agent_apps())
}

/// Add a user agent app
#[tauri::command]
pub fn add_agent_app(
  display_name: String,
  global_path: String,
  project_path: Option<String>,
) -> Result<AgentAppDetail, String> {
  // Check if global_path already exists
  if check_global_path_exists(&global_path) {
    return Err(format!(
      "Global path '{}' already exists in agent apps",
      global_path
    ));
  }

  // Generate id from display_name
  let id = generate_id_from_display_name(&display_name);

  // Check if id already exists (very unlikely but possible)
  if get_agent_app(&id).is_some() {
    return Err(format!("Agent app with id '{}' already exists", id));
  }

  let user_app = UserAgentApp {
    id: id.clone(),
    display_name,
    global_path,
    project_path,
  };

  add_user_agent_app(user_app)?;

  // Refresh local agent apps cache
  refresh_local_agent_apps();

  // Return the created app
  let app = get_agent_app(&id).ok_or("Failed to retrieve created app")?;
  let mut detail: AgentAppDetail = app.into();
  detail.is_installed = false;
  Ok(detail)
}

/// Remove a user agent app
#[tauri::command]
pub fn remove_agent_app(id: String) -> Result<(), String> {
  // Only allow removing user apps (not internal)
  let app = get_agent_app(&id).ok_or(format!("Agent app '{}' not found", id))?;

  if app.is_internal {
    return Err("Cannot remove internal agent apps".to_string());
  }

  remove_user_agent_app(&id)?;

  // Refresh local agent apps cache
  refresh_local_agent_apps();

  Ok(())
}

/// Validate a new agent app (check for duplicates)
#[tauri::command]
pub fn validate_agent_app(
  display_name: String,
  global_path: String,
) -> Result<ValidateResult, String> {
  let mut errors = Vec::new();
  let mut warnings = Vec::new();

  // Check if global_path already exists
  if check_global_path_exists(&global_path) {
    errors.push(format!("Global path '{}' already exists", global_path));
  }

  // Check if display_name would generate a duplicate id
  let id = generate_id_from_display_name(&display_name);
  if get_agent_app(&id).is_some() {
    errors.push(format!(
      "An agent app with similar name already exists (id: {})",
      id
    ));
  }

  // Check if path format looks valid
  if !global_path.starts_with('~') && !global_path.starts_with('/') {
    warnings
      .push("Global path should start with ~ (home directory) or / (absolute path)".to_string());
  }

  Ok(ValidateResult { errors, warnings })
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ValidateResult {
  pub errors: Vec<String>,
  pub warnings: Vec<String>,
}

/// Get internal agent apps
#[tauri::command]
pub fn list_internal_agent_apps() -> Result<Vec<AgentAppDetail>, String> {
  let all_apps = all_agent_apps();
  let local_apps_ids: std::collections::HashSet<String> =
    local_agent_apps().into_iter().map(|app| app.id).collect();

  Ok(
    all_apps
      .into_iter()
      .filter(|app| app.is_internal)
      .map(|app| {
        let is_installed = local_apps_ids.contains(&app.id);
        let detail: AgentAppDetail = app.into();
        AgentAppDetail {
          is_installed,
          ..detail
        }
      })
      .collect(),
  )
}

/// Get user agent apps
#[tauri::command]
pub fn list_user_agent_apps() -> Result<Vec<AgentAppDetail>, String> {
  let all_apps = all_agent_apps();
  let local_apps_ids: std::collections::HashSet<String> =
    local_agent_apps().into_iter().map(|app| app.id).collect();

  Ok(
    all_apps
      .into_iter()
      .filter(|app| !app.is_internal)
      .map(|app| {
        let is_installed = local_apps_ids.contains(&app.id);
        let detail: AgentAppDetail = app.into();
        AgentAppDetail {
          is_installed,
          ..detail
        }
      })
      .collect(),
  )
}
