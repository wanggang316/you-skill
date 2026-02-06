use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct AppConfig {
  pub scan_roots: Vec<String>,
  pub settings: AppSettings,
  pub backup_folder: Option<String>,
  pub last_backup_time: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct AppSettings {
  pub language: String,
  pub theme: String,
  pub sync_mode: String,
}

impl Default for AppConfig {
  fn default() -> Self {
    Self {
      scan_roots: Vec::new(),
      settings: AppSettings::default(),
      backup_folder: None,
      last_backup_time: None,
    }
  }
}

impl Default for AppSettings {
  fn default() -> Self {
    Self {
      language: "en".to_string(),
      theme: "system".to_string(),
      sync_mode: "symlink".to_string(),
    }
  }
}

pub fn config_path() -> Result<PathBuf, String> {
  let home = dirs_next::home_dir().ok_or("无法获取用户目录")?;
  Ok(home.join(".skill-kit").join("config.json"))
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

pub fn normalize_path(path: &str) -> String {
  let normalized = Path::new(path).to_path_buf();
  normalized.to_string_lossy().to_string()
}
