use crate::services::library_sync_service::SyncPolicy;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

pub const TRANSLATE_DISPLAY_MODES: [&str; 2] = ["bilingual", "translation"];
pub const DEFAULT_TRANSLATE_DISPLAY_MODE: &str = TRANSLATE_DISPLAY_MODES[0];
pub const TRANSLATE_TEXT_STYLES: [&str; 2] = ["none", "dashed_underline"];
pub const DEFAULT_TRANSLATE_TEXT_STYLE: &str = TRANSLATE_TEXT_STYLES[0];
/// Actions of the top bar that accept a keyboard shortcut.
pub const SHORTCUT_ACTIONS: [&str; 7] = [
  "library",
  "instructions",
  "projects",
  "market",
  "import",
  "search",
  "settings",
];

/// Time between two auto sync cycles.
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
pub enum AutoSyncInterval {
  #[serde(rename = "15m")]
  Minutes15,
  #[default]
  #[serde(rename = "1h")]
  Hour1,
  #[serde(rename = "6h")]
  Hours6,
  #[serde(rename = "24h")]
  Hours24,
}

impl AutoSyncInterval {
  pub fn duration(self) -> Duration {
    let minutes = match self {
      Self::Minutes15 => 15,
      Self::Hour1 => 60,
      Self::Hours6 => 6 * 60,
      Self::Hours24 => 24 * 60,
    };
    Duration::from_secs(minutes * 60)
  }
}

/// Library auto sync of this computer (`docs/decisions/0005-library-auto-sync.md`). Not
/// synced. Off by default, also for a config file from before auto sync.
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(default)]
pub struct AutoSyncSettings {
  pub enabled: bool,
  pub backup: bool,
  pub restore: bool,
  pub interval: AutoSyncInterval,
  pub policy: SyncPolicy,
}

impl Default for AutoSyncSettings {
  fn default() -> Self {
    Self {
      enabled: false,
      backup: true,
      restore: true,
      interval: AutoSyncInterval::default(),
      policy: SyncPolicy::default(),
    }
  }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct AppConfig {
  pub language: String,
  pub theme: String,
  /// Default install mode: "copy" or "symlink".
  pub sync_mode: String,
  pub backup_folder: Option<String>,
  pub last_backup_time: Option<String>,
  pub openrouter_api_key: Option<String>,
  pub translate_target_language: String,
  pub translate_model: String,
  /// Translated markdown display mode: "bilingual" or "translation".
  pub translate_display_mode: String,
  /// Translated text style: "none" or "dashed_underline".
  pub translate_text_style: String,
  /// GitHub CLI account used for GitHub requests. `None` follows the active `gh` account.
  pub github_account: Option<String>,
  /// Shortcuts the user changed, by action. Actions not listed use their default key.
  pub shortcuts: BTreeMap<String, String>,
  pub auto_sync: AutoSyncSettings,
}

impl Default for AppConfig {
  fn default() -> Self {
    Self {
      language: "en".to_string(),
      theme: "system".to_string(),
      sync_mode: "copy".to_string(),
      backup_folder: None,
      last_backup_time: None,
      openrouter_api_key: None,
      translate_target_language: String::new(),
      translate_model: String::new(),
      translate_display_mode: DEFAULT_TRANSLATE_DISPLAY_MODE.to_string(),
      translate_text_style: DEFAULT_TRANSLATE_TEXT_STYLE.to_string(),
      github_account: None,
      shortcuts: BTreeMap::new(),
      auto_sync: AutoSyncSettings::default(),
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

#[cfg(test)]
mod tests {
  use super::*;
  use crate::services::library_sync_service::{
    ConflictPolicy, RemovalPolicy, RepositoryChangePolicy,
  };

  #[test]
  fn auto_sync_is_off_for_a_config_from_before_auto_sync() {
    let config: AppConfig = serde_json::from_str(r#"{"language":"zh"}"#).unwrap();
    assert_eq!(config.auto_sync, AutoSyncSettings::default());
    assert!(!config.auto_sync.enabled);
    assert!(config.auto_sync.backup && config.auto_sync.restore);
    assert_eq!(config.auto_sync.interval, AutoSyncInterval::Hour1);
    assert_eq!(
      config.auto_sync.policy.repository_change,
      RepositoryChangePolicy::Apply
    );
    assert_eq!(config.auto_sync.policy.removal, RemovalPolicy::Ask);
    assert_eq!(config.auto_sync.policy.conflict, ConflictPolicy::Ask);
  }

  #[test]
  fn auto_sync_settings_round_trip() {
    let settings: AutoSyncSettings = serde_json::from_str(
      r#"{"enabled":true,"backup":false,"interval":"15m","policy":{"conflict":"keep_both"}}"#,
    )
    .unwrap();
    assert!(settings.enabled && !settings.backup && settings.restore);
    assert_eq!(settings.interval.duration(), Duration::from_secs(15 * 60));
    assert_eq!(settings.policy.conflict, ConflictPolicy::KeepBoth);
    assert_eq!(settings.policy.removal, RemovalPolicy::Ask);
    let value = serde_json::to_value(settings).unwrap();
    assert_eq!(value["interval"], "15m");
    assert_eq!(value["policy"]["repositoryChange"], "apply");
    assert_eq!(
      AutoSyncInterval::Hours24.duration(),
      Duration::from_secs(86_400)
    );
  }
}
