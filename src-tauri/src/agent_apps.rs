use crate::models::AgentInfo;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::RwLock;

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
pub struct InternalAgentApp {
  pub id: &'static str,
  pub display_name: &'static str,
  pub project_path: Option<&'static str>,
  pub global_path: Option<&'static str>,
}

// Combined agent app with ownership
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentApp {
  pub id: String,
  pub display_name: String,
  pub project_path: Option<String>,
  pub global_path: Option<String>,
  pub is_internal: bool,
}

// Global cache for local agent apps
static LOCAL_AGENT_APPS: RwLock<Option<Vec<AgentInfo>>> = RwLock::new(None);

// Get internal agent apps (built-in)
pub fn internal_agent_apps() -> Vec<InternalAgentApp> {
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

// Get path to user_agent_apps.json
pub fn user_agent_apps_path() -> Result<PathBuf, String> {
  let config_dir = dirs_next::config_dir().ok_or("无法获取配置目录")?;
  Ok(config_dir.join("youskill").join("user_agent_apps.json"))
}

// Load user agent apps from JSON file
pub fn load_user_agent_apps() -> Result<Vec<UserAgentApp>, String> {
  let path = user_agent_apps_path()?;
  if !path.exists() {
    return Ok(Vec::new());
  }
  let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
  serde_json::from_str(&content).map_err(|e| e.to_string())
}

// Save user agent apps to JSON file
pub fn save_user_agent_apps(apps: &[UserAgentApp]) -> Result<(), String> {
  let path = user_agent_apps_path()?;
  if let Some(parent) = path.parent() {
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
  }
  let content = serde_json::to_string_pretty(apps).map_err(|e| e.to_string())?;
  fs::write(&path, content).map_err(|e| e.to_string())
}

// Add a user agent app
pub fn add_user_agent_app(app: UserAgentApp) -> Result<(), String> {
  let mut apps = load_user_agent_apps().unwrap_or_default();
  apps.push(app);
  save_user_agent_apps(&apps)
}

// Remove a user agent app by id
pub fn remove_user_agent_app(id: &str) -> Result<(), String> {
  let mut apps = load_user_agent_apps().unwrap_or_default();
  apps.retain(|app| app.id != id);
  save_user_agent_apps(&apps)
}

// Update a user agent app by id
pub fn update_user_agent_app(id: &str, app: UserAgentApp) -> Result<(), String> {
  let mut apps = load_user_agent_apps().unwrap_or_default();
  let index = apps
    .iter()
    .position(|a| a.id == id)
    .ok_or(format!("Agent app with id '{}' not found", id))?;

  // Ensure the id doesn't change
  let updated_app = UserAgentApp {
    id: id.to_string(),
    ..app
  };

  apps[index] = updated_app;
  save_user_agent_apps(&apps)
}

// Get all agent apps (internal + user, user apps override internal if same id or global_path)
pub fn all_agent_apps() -> Vec<AgentApp> {
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

  // Track global paths from internal apps to detect duplicates
  let internal_global_paths: std::collections::HashSet<String> = internal
    .iter()
    .filter_map(|app| app.global_path.map(|p| p.to_string()))
    .collect();

  // Add user apps, overriding internal apps with same id or global_path
  for user_app in user_apps {
    // Remove any internal app with the same id or global_path
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

// Legacy function for backward compatibility
// Returns all agent apps as AgentInfo (for skill installation)
pub fn agent_infos() -> Vec<AgentInfo> {
  local_agent_apps()
}

// Check if a global_path already exists in all_agent_apps
pub fn check_global_path_exists(global_path: &str) -> bool {
  let apps = all_agent_apps();
  apps
    .iter()
    .any(|app| app.global_path.as_ref() == Some(&global_path.to_string()))
}

// Get agent app by id
pub fn get_agent_app(id: &str) -> Option<AgentApp> {
  let apps = all_agent_apps();
  apps.into_iter().find(|app| app.id == id)
}

// Generate id from display_name (lowercase, replace spaces with hyphens)
pub fn generate_id_from_display_name(display_name: &str) -> String {
  display_name
    .to_lowercase()
    .replace(' ', "-")
    .chars()
    .filter(|c| c.is_alphanumeric() || *c == '-')
    .collect()
}
