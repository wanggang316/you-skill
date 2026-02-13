use crate::models::AgentApp;
use crate::utils::path::expand_home;
use std::fs;
use std::path::PathBuf;
use std::sync::RwLock;
use uuid::Uuid;

// Global cache for local agent apps
static LOCAL_AGENT_APPS: RwLock<Option<Vec<AgentApp>>> = RwLock::new(None);

// Get local agent apps (actually installed on the system)
pub fn local_agent_apps() -> Vec<AgentApp> {
  // Check cache first
  if let Ok(cache) = LOCAL_AGENT_APPS.read() {
    if let Some(ref cached) = *cache {
      return cached.clone();
    }
  }

  let all_apps = all_agent_apps();
  let mut local_apps = Vec::new();

  for app in all_apps {
    if let Some(global_path) = &app.global_path {
      let expanded_path = expand_home(global_path);
      if expanded_path.exists() {
        local_apps.push(app);
      }
    }
  }

  // Update cache
  if let Ok(mut cache) = LOCAL_AGENT_APPS.write() {
    *cache = Some(local_apps.clone());
  }

  tracing::info!("local_apps: {:?}", local_apps);
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
) -> Result<AgentApp, String> {
  let display_name = display_name.trim().to_string();
  let global_path = global_path.trim().to_string();
  let project_path = project_path.map(|p| p.trim().to_string());

  validate_user_agent_app(&display_name, &global_path, project_path.as_deref(), None)?;

  let id = generate_id_from_display_name(&display_name);
  let user_app = AgentApp {
    id: id.clone(),
    display_name,
    global_path: Some(global_path),
    project_path,
    is_user_custom: true,
  };

  let mut apps = load_user_agent_apps().unwrap_or_default();
  apps.push(user_app);
  save_user_agent_apps(&apps)?;
  refresh_local_agent_apps();

  let app = get_agent_app(&id).ok_or("Failed to retrieve created app")?;
  Ok(app)
}

pub fn delete_user_agent_app_by_id(id: &str) -> Result<(), String> {
  let app = get_agent_app(id).ok_or(format!("Agent app '{}' not found", id))?;
  if !app.is_user_custom {
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
) -> Result<AgentApp, String> {
  let display_name = display_name.trim().to_string();
  let global_path = global_path.trim().to_string();
  let project_path = project_path.map(|p| p.trim().to_string());

  let app = get_agent_app(&id).ok_or(format!("Agent app '{}' not found", id))?;
  if !app.is_user_custom {
    return Err("Cannot update internal agent apps".to_string());
  }

  validate_user_agent_app(
    &display_name,
    &global_path,
    project_path.as_deref(),
    Some(id.as_str()),
  )?;

  let user_app = AgentApp {
    id: id.clone(),
    display_name,
    global_path: Some(global_path),
    project_path,
    is_user_custom: true,
  };

  update_user_agent_app(&id, user_app)?;
  refresh_local_agent_apps();

  let updated_app = get_agent_app(&id).ok_or("Failed to retrieve updated app")?;
  Ok(updated_app)
}

fn internal_agent_apps() -> Vec<AgentApp> {
  vec![
    AgentApp {
      id: "claude-code".to_string(),
      display_name: "Claude Code".to_string(),
      project_path: Some(".claude/skills".to_string()),
      global_path: Some("~/.claude/skills".to_string()),
      is_user_custom: false,
    },
    AgentApp {
      id: "codex".to_string(),
      display_name: "Codex".to_string(),
      project_path: Some(".codex/skills".to_string()),
      global_path: Some("~/.codex/skills".to_string()),
      is_user_custom: false,
    },
    AgentApp {
      id: "cursor".to_string(),
      display_name: "Cursor".to_string(),
      project_path: Some(".cursor/skills".to_string()),
      global_path: Some("~/.cursor/skills".to_string()),
      is_user_custom: false,
    },
    AgentApp {
      id: "cline".to_string(),
      display_name: "Cline".to_string(),
      project_path: Some(".cline/skills".to_string()),
      global_path: Some("~/.cline/skills".to_string()),
      is_user_custom: false,
    },
    AgentApp {
      id: "opencode".to_string(),
      display_name: "OpenCode".to_string(),
      project_path: Some(".opencode/skills".to_string()),
      global_path: Some("~/.config/opencode/skills".to_string()),
      is_user_custom: false,
    },
    AgentApp {
      id: "openhands".to_string(),
      display_name: "OpenHands".to_string(),
      project_path: Some(".openhands/skills".to_string()),
      global_path: Some("~/.openhands/skills".to_string()),
      is_user_custom: false,
    },
    AgentApp {
      id: "github-copilot".to_string(),
      display_name: "GitHub Copilot".to_string(),
      project_path: Some(".github/skills".to_string()),
      global_path: Some("~/.copilot/skills".to_string()),
      is_user_custom: false,
    },
    AgentApp {
      id: "continue".to_string(),
      display_name: "Continue".to_string(),
      project_path: Some(".continue/skills".to_string()),
      global_path: Some("~/.continue/skills".to_string()),
      is_user_custom: false,
    },
    AgentApp {
      id: "gemini-cli".to_string(),
      display_name: "Gemini CLI".to_string(),
      project_path: Some(".gemini/skills".to_string()),
      global_path: Some("~/.gemini/skills".to_string()),
      is_user_custom: false,
    },
    AgentApp {
      id: "goose".to_string(),
      display_name: "Goose".to_string(),
      project_path: Some(".goose/skills".to_string()),
      global_path: Some("~/.config/goose/skills".to_string()),
      is_user_custom: false,
    },
    AgentApp {
      id: "windsurf".to_string(),
      display_name: "Windsurf".to_string(),
      project_path: Some(".windsurf/skills".to_string()),
      global_path: Some("~/.codeium/windsurf/skills".to_string()),
      is_user_custom: false,
    },
    AgentApp {
      id: "roo".to_string(),
      display_name: "Roo Code".to_string(),
      project_path: Some(".roo/skills".to_string()),
      global_path: Some("~/.roo/skills".to_string()),
      is_user_custom: false,
    },
    AgentApp {
      id: "kiro-cli".to_string(),
      display_name: "Kiro CLI".to_string(),
      project_path: Some(".kiro/skills".to_string()),
      global_path: Some("~/.kiro/skills".to_string()),
      is_user_custom: false,
    },
    AgentApp {
      id: "qwen-code".to_string(),
      display_name: "Qwen Code".to_string(),
      project_path: Some(".qwen/skills".to_string()),
      global_path: Some("~/.qwen/skills".to_string()),
      is_user_custom: false,
    },
    AgentApp {
      id: "amp".to_string(),
      display_name: "AMP".to_string(),
      project_path: Some(".agents/skills".to_string()),
      global_path: Some("~/.config/agents/skills".to_string()),
      is_user_custom: false,
    },
    AgentApp {
      id: "antigravity".to_string(),
      display_name: "Antigravity".to_string(),
      project_path: Some(".agent/skills".to_string()),
      global_path: Some("~/.gemini/antigravity/skills".to_string()),
      is_user_custom: false,
    },
    AgentApp {
      id: "command-code".to_string(),
      display_name: "Command Code".to_string(),
      project_path: Some(".commandcode/skills".to_string()),
      global_path: Some("~/.commandcode/skills".to_string()),
      is_user_custom: false,
    },
    AgentApp {
      id: "crush".to_string(),
      display_name: "Crush".to_string(),
      project_path: Some(".crush/skills".to_string()),
      global_path: Some("~/.config/crush/skills".to_string()),
      is_user_custom: false,
    },
    AgentApp {
      id: "trae".to_string(),
      display_name: "Trae".to_string(),
      project_path: Some(".trae/skills".to_string()),
      global_path: Some("~/.trae/skills".to_string()),
      is_user_custom: false,
    },
    AgentApp {
      id: "trae-cn".to_string(),
      display_name: "Trae CN".to_string(),
      project_path: Some(".trae-cn/skills".to_string()),
      global_path: Some("~/.trae-cn/skills".to_string()),
      is_user_custom: false,
    },
    AgentApp {
      id: "vscode".to_string(),
      display_name: "VSCode".to_string(),
      project_path: Some(".github/skills".to_string()),
      global_path: Some("~/.vscode/skills".to_string()),
      is_user_custom: false,
    },
  ]
}

fn user_agent_apps_path() -> Result<PathBuf, String> {
  let config_dir = dirs_next::config_dir().ok_or("无法获取配置目录")?;
  Ok(config_dir.join("youskill").join("user_agent_apps.json"))
}

fn load_user_agent_apps() -> Result<Vec<AgentApp>, String> {
  let path = user_agent_apps_path()?;
  if !path.exists() {
    return Ok(Vec::new());
  }
  let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
  serde_json::from_str(&content).map_err(|e| e.to_string())
}

fn save_user_agent_apps(apps: &[AgentApp]) -> Result<(), String> {
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

fn update_user_agent_app(id: &str, app: AgentApp) -> Result<(), String> {
  let mut apps = load_user_agent_apps().unwrap_or_default();
  let index = apps
    .iter()
    .position(|a| a.id == id)
    .ok_or(format!("Agent app with id '{}' not found", id))?;

  let updated_app = AgentApp {
    id: id.to_string(),
    ..app
  };

  apps[index] = updated_app;
  save_user_agent_apps(&apps)
}

fn all_agent_apps() -> Vec<AgentApp> {
  let internal = internal_agent_apps();
  let user_apps = load_user_agent_apps().unwrap_or_default();

  let internal_global_paths: std::collections::HashSet<String> = internal
    .iter()
    .filter_map(|app| app.global_path.clone())
    .collect();
  let mut result: Vec<AgentApp> = internal;

  for mut user_app in user_apps {
    let user_global_path = user_app.global_path.clone();
    result.retain(|app| {
      app.id != user_app.id
        && match &user_global_path {
          Some(path) => {
            !internal_global_paths.contains(path) || app.global_path.as_ref() != Some(path)
          },
          None => true,
        }
    });
    user_app.is_user_custom = true;
    result.push(user_app);
  }

  result
}

fn check_global_path_exists(global_path: &str) -> bool {
  let expanded_path = expand_home(global_path);
  expanded_path.exists()
}

fn get_agent_app(id: &str) -> Option<AgentApp> {
  let apps = all_agent_apps();
  apps.into_iter().find(|app| app.id == id)
}

fn generate_id_from_display_name(_display_name: &str) -> String {
  Uuid::new_v4().to_string()
}

fn validate_user_agent_app(
  display_name: &str,
  global_path: &str,
  project_path: Option<&str>,
  current_id: Option<&str>,
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
    .filter(|app| Some(app.id.as_str()) != current_id)
    .any(|app| app.display_name.eq_ignore_ascii_case(display_name))
  {
    return Err(format!("Display name '{}' already exists", display_name));
  }

  if local_apps
    .iter()
    .filter(|app| Some(app.id.as_str()) != current_id)
    .any(|app| app.global_path.as_deref() == Some(global_path))
  {
    return Err(format!("Global path '{}' already exists", global_path));
  }

  if local_apps
    .iter()
    .filter(|app| Some(app.id.as_str()) != current_id)
    .any(|app| app.project_path.as_deref() == Some(project_path))
  {
    return Err(format!("Project path '{}' already exists", project_path));
  }

  if !check_global_path_exists(global_path) {
    return Err(format!(
      "Global path folder does not exist: {}",
      global_path
    ));
  }

  Ok(())
}
