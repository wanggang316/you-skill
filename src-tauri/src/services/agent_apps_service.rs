use crate::models::AgentInfo;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::RwLock;
use uuid::Uuid;

// User-defined agent app stored in user_agent_apps.json
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserAgentApp {
  pub id: String,
  pub display_name: String,
  pub global_path: String,
  pub project_path: Option<String>,
}

// Internal agent apps (built-in)
#[derive(Debug, Clone)]
struct InternalAgentApp {
  pub id: &'static str,
  pub display_name: &'static str,
  pub project_path: Option<&'static str>,
  pub global_path: Option<&'static str>,
}

// Combined agent app with ownership
#[derive(Debug, Clone, Serialize, Deserialize)]
struct AgentApp {
  pub id: String,
  pub display_name: String,
  pub project_path: Option<String>,
  pub global_path: Option<String>,
  pub is_internal: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AgentAppDetail {
  pub id: String,
  pub display_name: String,
  pub project_path: Option<String>,
  pub global_path: Option<String>,
  pub is_internal: bool,
  pub is_installed: bool,
}

// Global cache for local agent apps
static LOCAL_AGENT_APPS: RwLock<Option<Vec<AgentInfo>>> = RwLock::new(None);

impl From<AgentApp> for AgentAppDetail {
  fn from(app: AgentApp) -> Self {
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

// Expand tilde in path
pub fn expand_tilde(path: &str) -> PathBuf {
  if path.starts_with("~/") {
    if let Some(home) = dirs_next::home_dir() {
      return home.join(&path[2..]);
    }
  }
  PathBuf::from(path)
}

// Get local agent apps (actually installed on the system)
pub fn local_agent_apps() -> Vec<AgentInfo> {
  // Check cache first
  if let Ok(cache) = LOCAL_AGENT_APPS.read() {
    if let Some(ref cached) = *cache {
      return cached.clone();
    }
  }

  let all_apps = all_agent_apps();
  let mut local_apps = Vec::new();

  for app in all_apps {
    if let Some(global_path) = app.global_path {
      let expanded_path = expand_tilde(&global_path);
      if expanded_path.exists() {
        local_apps.push(AgentInfo {
          id: app.id.clone(),
          display_name: app.display_name.clone(),
          project_path: app.project_path.clone(),
          global_path: Some(global_path),
        });
      }
    }
  }

  // Update cache
  if let Ok(mut cache) = LOCAL_AGENT_APPS.write() {
    *cache = Some(local_apps.clone());
  }

  local_apps
}

// Refresh the local agent apps cache (call when user apps change)
pub fn refresh_local_agent_apps() {
  if let Ok(mut cache) = LOCAL_AGENT_APPS.write() {
    *cache = None;
  }
  // Trigger recomputation by calling local_agent_apps
  local_agent_apps();
}

pub fn create_user_agent_app(
  display_name: String,
  global_path: String,
  project_path: Option<String>,
) -> Result<AgentAppDetail, String> {
  let display_name = display_name.trim().to_string();
  let global_path = global_path.trim().to_string();
  let project_path = project_path.map(|p| p.trim().to_string());

  validate_user_agent_app(&display_name, &global_path, project_path.as_deref())?;

  let id = generate_id_from_display_name(&display_name);
  let user_app = UserAgentApp {
    id: id.clone(),
    display_name,
    global_path,
    project_path,
  };

  let mut apps = load_user_agent_apps().unwrap_or_default();
  apps.push(user_app);
  save_user_agent_apps(&apps)?;
  refresh_local_agent_apps();

  let app = get_agent_app(&id).ok_or("Failed to retrieve created app")?;
  Ok(app.into())
}

pub fn delete_user_agent_app_by_id(id: &str) -> Result<(), String> {
  let app = get_agent_app(id).ok_or(format!("Agent app '{}' not found", id))?;
  if app.is_internal {
    return Err("Cannot remove internal agent apps".to_string());
  }

  remove_user_agent_app(id)?;
  refresh_local_agent_apps();
  Ok(())
}

pub fn update_user_agent_app_detail(
  id: String,
  display_name: String,
  global_path: String,
  project_path: Option<String>,
) -> Result<AgentAppDetail, String> {
  let display_name = display_name.trim().to_string();
  let global_path = global_path.trim().to_string();
  let project_path = project_path.map(|p| p.trim().to_string());

  let app = get_agent_app(&id).ok_or(format!("Agent app '{}' not found", id))?;
  if app.is_internal {
    return Err("Cannot update internal agent apps".to_string());
  }

  validate_user_agent_app(&display_name, &global_path, project_path.as_deref())?;

  let user_app = UserAgentApp {
    id: id.clone(),
    display_name,
    global_path,
    project_path,
  };

  update_user_agent_app(&id, user_app)?;
  refresh_local_agent_apps();

  let updated_app = get_agent_app(&id).ok_or("Failed to retrieve updated app")?;
  Ok(updated_app.into())
}

pub fn list_internal_agent_app_details() -> Vec<AgentAppDetail> {
  let local_ids = local_agent_app_ids();
  all_agent_apps()
    .into_iter()
    .filter(|app| app.is_internal)
    .map(|app| {
      let is_installed = local_ids.contains(&app.id);
      let detail: AgentAppDetail = app.into();
      AgentAppDetail {
        is_installed,
        ..detail
      }
    })
    .collect()
}

pub fn list_user_agent_app_details() -> Vec<AgentAppDetail> {
  let local_ids = local_agent_app_ids();
  all_agent_apps()
    .into_iter()
    .filter(|app| !app.is_internal)
    .map(|app| {
      let is_installed = local_ids.contains(&app.id);
      let detail: AgentAppDetail = app.into();
      AgentAppDetail {
        is_installed,
        ..detail
      }
    })
    .collect()
}

fn internal_agent_apps() -> Vec<InternalAgentApp> {
  vec![
    InternalAgentApp {
      id: "claude-code",
      display_name: "Claude Code",
      project_path: Some(".claude/skills"),
      global_path: Some("~/.claude/skills"),
    },
    InternalAgentApp {
      id: "codex",
      display_name: "Codex",
      project_path: Some(".codex/skills"),
      global_path: Some("~/.codex/skills"),
    },
    InternalAgentApp {
      id: "cursor",
      display_name: "Cursor",
      project_path: Some(".cursor/skills"),
      global_path: Some("~/.cursor/skills"),
    },
    InternalAgentApp {
      id: "cline",
      display_name: "Cline",
      project_path: Some(".cline/skills"),
      global_path: Some("~/.cline/skills"),
    },
    InternalAgentApp {
      id: "opencode",
      display_name: "OpenCode",
      project_path: Some(".opencode/skills"),
      global_path: Some("~/.config/opencode/skills"),
    },
    InternalAgentApp {
      id: "openhands",
      display_name: "OpenHands",
      project_path: Some(".openhands/skills"),
      global_path: Some("~/.openhands/skills"),
    },
    InternalAgentApp {
      id: "github-copilot",
      display_name: "GitHub Copilot",
      project_path: Some(".github/skills"),
      global_path: Some("~/.copilot/skills"),
    },
    InternalAgentApp {
      id: "continue",
      display_name: "Continue",
      project_path: Some(".continue/skills"),
      global_path: Some("~/.continue/skills"),
    },
    InternalAgentApp {
      id: "gemini-cli",
      display_name: "Gemini CLI",
      project_path: Some(".gemini/skills"),
      global_path: Some("~/.gemini/skills"),
    },
    InternalAgentApp {
      id: "goose",
      display_name: "Goose",
      project_path: Some(".goose/skills"),
      global_path: Some("~/.config/goose/skills"),
    },
    InternalAgentApp {
      id: "windsurf",
      display_name: "Windsurf",
      project_path: Some(".windsurf/skills"),
      global_path: Some("~/.codeium/windsurf/skills"),
    },
    InternalAgentApp {
      id: "roo",
      display_name: "Roo Code",
      project_path: Some(".roo/skills"),
      global_path: Some("~/.roo/skills"),
    },
    InternalAgentApp {
      id: "kiro-cli",
      display_name: "Kiro CLI",
      project_path: Some(".kiro/skills"),
      global_path: Some("~/.kiro/skills"),
    },
    InternalAgentApp {
      id: "qwen-code",
      display_name: "Qwen Code",
      project_path: Some(".qwen/skills"),
      global_path: Some("~/.qwen/skills"),
    },
    InternalAgentApp {
      id: "amp",
      display_name: "AMP",
      project_path: Some(".agents/skills"),
      global_path: Some("~/.config/agents/skills"),
    },
    InternalAgentApp {
      id: "antigravity",
      display_name: "Antigravity",
      project_path: Some(".agent/skills"),
      global_path: Some("~/.gemini/antigravity/skills"),
    },
    InternalAgentApp {
      id: "command-code",
      display_name: "Command Code",
      project_path: Some(".commandcode/skills"),
      global_path: Some("~/.commandcode/skills"),
    },
    InternalAgentApp {
      id: "crush",
      display_name: "Crush",
      project_path: Some(".crush/skills"),
      global_path: Some("~/.config/crush/skills"),
    },
    InternalAgentApp {
      id: "trae",
      display_name: "Trae",
      project_path: Some(".trae/skills"),
      global_path: Some("~/.trae/skills"),
    },
    InternalAgentApp {
      id: "trae-cn",
      display_name: "Trae CN",
      project_path: Some(".trae-cn/skills"),
      global_path: Some("~/.trae-cn/skills"),
    },
    InternalAgentApp {
      id: "vscode",
      display_name: "VSCode",
      project_path: Some(".github/skills"),
      global_path: Some("~/.vscode/skills"),
    },
  ]
}

fn user_agent_apps_path() -> Result<PathBuf, String> {
  let config_dir = dirs_next::config_dir().ok_or("无法获取配置目录")?;
  Ok(config_dir.join("youskill").join("user_agent_apps.json"))
}

fn load_user_agent_apps() -> Result<Vec<UserAgentApp>, String> {
  let path = user_agent_apps_path()?;
  if !path.exists() {
    return Ok(Vec::new());
  }
  let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
  serde_json::from_str(&content).map_err(|e| e.to_string())
}

fn save_user_agent_apps(apps: &[UserAgentApp]) -> Result<(), String> {
  let path = user_agent_apps_path()?;
  if let Some(parent) = path.parent() {
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
  }
  let content = serde_json::to_string_pretty(apps).map_err(|e| e.to_string())?;
  fs::write(&path, content).map_err(|e| e.to_string())
}

fn remove_user_agent_app(id: &str) -> Result<(), String> {
  let mut apps = load_user_agent_apps().unwrap_or_default();
  apps.retain(|app| app.id != id);
  save_user_agent_apps(&apps)
}

fn update_user_agent_app(id: &str, app: UserAgentApp) -> Result<(), String> {
  let mut apps = load_user_agent_apps().unwrap_or_default();
  let index = apps
    .iter()
    .position(|a| a.id == id)
    .ok_or(format!("Agent app with id '{}' not found", id))?;

  let updated_app = UserAgentApp {
    id: id.to_string(),
    ..app
  };

  apps[index] = updated_app;
  save_user_agent_apps(&apps)
}

fn all_agent_apps() -> Vec<AgentApp> {
  let internal = internal_agent_apps();
  let user_apps = load_user_agent_apps().unwrap_or_default();

  let mut result: Vec<AgentApp> = internal
    .iter()
    .map(|app| AgentApp {
      id: app.id.to_string(),
      display_name: app.display_name.to_string(),
      project_path: app.project_path.map(|p| p.to_string()),
      global_path: app.global_path.map(|p| p.to_string()),
      is_internal: true,
    })
    .collect();

  let internal_global_paths: std::collections::HashSet<String> = internal
    .iter()
    .filter_map(|app| app.global_path.map(|p| p.to_string()))
    .collect();

  for user_app in user_apps {
    result.retain(|app| {
      app.id != user_app.id
        && (!internal_global_paths.contains(&user_app.global_path)
          || app.global_path.as_ref() != Some(&user_app.global_path))
    });

    result.push(AgentApp {
      id: user_app.id.clone(),
      display_name: user_app.display_name.clone(),
      project_path: user_app.project_path.clone(),
      global_path: Some(user_app.global_path.clone()),
      is_internal: false,
    });
  }

  result
}

fn check_global_path_exists(global_path: &str) -> bool {
  let expanded_path = expand_tilde(global_path);
  expanded_path.exists()
}

fn get_agent_app(id: &str) -> Option<AgentApp> {
  let apps = all_agent_apps();
  apps.into_iter().find(|app| app.id == id)
}

fn generate_id_from_display_name(_display_name: &str) -> String {
  Uuid::new_v4().to_string()
}

fn local_agent_app_ids() -> std::collections::HashSet<String> {
  local_agent_apps().into_iter().map(|app| app.id).collect()
}

fn validate_user_agent_app(
  display_name: &str,
  global_path: &str,
  project_path: Option<&str>,
) -> Result<(), String> {
  let project_path = project_path.unwrap_or_default();

  if display_name.is_empty() {
    return Err("Display name is required".to_string());
  }

  if global_path.is_empty() {
    return Err("Global path is required".to_string());
  }

  if project_path.is_empty() {
    return Err("Project path is required".to_string());
  }

  let local_apps = local_agent_apps();

  if local_apps
    .iter()
    .any(|app| app.display_name.eq_ignore_ascii_case(display_name))
  {
    return Err(format!("Display name '{}' already exists", display_name));
  }

  if local_apps
    .iter()
    .any(|app| app.global_path.as_deref() == Some(global_path))
  {
    return Err(format!("Global path '{}' already exists", global_path));
  }

  if local_apps
    .iter()
    .any(|app| app.project_path.as_deref() == Some(project_path))
  {
    return Err(format!("Project path '{}' already exists", project_path));
  }

  if !check_global_path_exists(global_path) {
    return Err(format!("Global path folder does not exist: {}", global_path));
  }

  Ok(())
}
