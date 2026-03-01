use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct AppConfig {
  pub language: String,
  pub theme: String,
  pub sync_mode: String,
  pub unknown_skill_install_permission: bool,
  pub backup_folder: Option<String>,
  pub last_backup_time: Option<String>,
  pub openrouter_api_key: Option<String>,
  pub translate_target_language: String,
}

impl Default for AppConfig {
  fn default() -> Self {
    Self {
      language: "en".to_string(),
      theme: "system".to_string(),
      sync_mode: "symlink".to_string(),
      unknown_skill_install_permission: false,
      backup_folder: None,
      last_backup_time: None,
      openrouter_api_key: None,
      translate_target_language: "zh-CN".to_string(),
    }
  }
}


pub fn config_path() -> Result<PathBuf, String> {
  let config_dir = dirs_next::config_dir().ok_or("无法获取配置目录")?;
  Ok(config_dir.join("youskill").join("youskill.config"))
}

pub fn load_config() -> Result<AppConfig, String> {
  let path = config_path()?;
  if !path.exists() {
    return Ok(AppConfig::default());
  }
  let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
  serde_json::from_str(&content).map_err(|e| e.to_string())
}

pub fn save_config(config: &AppConfig) -> Result<(), String> {
  let path = config_path()?;
  if let Some(parent) = path.parent() {
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
  }
  let content = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
  fs::write(&path, content).map_err(|e| e.to_string())
}
