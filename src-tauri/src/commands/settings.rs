use crate::config::{load_config, save_config};
use crate::services::backup_service::{self, BackupResult};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SettingsPayload {
  pub language: String,
  pub theme: String,
  pub sync_mode: String,
  #[serde(default)]
  pub unknown_skill_install_permission: bool,
  #[serde(default)]
  pub openrouter_api_key: Option<String>,
  #[serde(default = "default_translate_target_language")]
  pub translate_target_language: String,
  pub backup_folder: Option<String>,
  pub last_backup_time: Option<String>,
}

fn default_translate_target_language() -> String {
  "zh-CN".to_string()
}

#[tauri::command]
pub fn get_settings() -> Result<SettingsPayload, String> {
  let config = load_config()?;
  Ok(SettingsPayload {
    language: config.language,
    theme: config.theme,
    sync_mode: config.sync_mode,
    unknown_skill_install_permission: config.unknown_skill_install_permission,
    openrouter_api_key: config.openrouter_api_key,
    translate_target_language: config.translate_target_language,
    backup_folder: config.backup_folder,
    last_backup_time: config.last_backup_time,
  })
}

#[tauri::command]
pub fn update_settings(settings: SettingsPayload) -> Result<SettingsPayload, String> {
  let mut config = load_config()?;
  config.language = settings.language.clone();
  config.theme = settings.theme.clone();
  config.sync_mode = settings.sync_mode.clone();
  config.unknown_skill_install_permission = settings.unknown_skill_install_permission;
  config.openrouter_api_key = settings
    .openrouter_api_key
    .as_ref()
    .map(|value| value.trim().to_string())
    .filter(|value| !value.is_empty());
  config.translate_target_language = if settings.translate_target_language.trim().is_empty() {
    default_translate_target_language()
  } else {
    settings.translate_target_language.trim().to_string()
  };
  save_config(&config)?;
  Ok(SettingsPayload {
    language: config.language,
    theme: config.theme,
    sync_mode: config.sync_mode,
    unknown_skill_install_permission: config.unknown_skill_install_permission,
    openrouter_api_key: config.openrouter_api_key,
    translate_target_language: config.translate_target_language,
    backup_folder: config.backup_folder,
    last_backup_time: config.last_backup_time,
  })
}

#[tauri::command]
pub fn set_backup_folder(path: String) -> Result<Option<String>, String> {
  let mut config = load_config()?;
  config.backup_folder = Some(path);
  save_config(&config)?;
  Ok(config.backup_folder)
}

#[tauri::command]
pub fn open_backup_folder(path: String) -> Result<(), String> {
  backup_service::open_backup_folder(path)
}

#[tauri::command]
pub async fn backup_skills(backup_folder: String) -> Result<BackupResult, String> {
  backup_service::backup_skills(backup_folder).await
}
