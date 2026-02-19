use crate::config::{load_config, save_config};
use crate::services::backup_service::{self, BackupResult};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SettingsPayload {
  pub language: String,
  pub theme: String,
  pub sync_mode: String,
  #[serde(default)]
  pub known_skill_install_permission: bool,
  pub backup_folder: Option<String>,
  pub last_backup_time: Option<String>,
}

#[tauri::command]
pub fn get_settings() -> Result<SettingsPayload, String> {
  let config = load_config()?;
  Ok(SettingsPayload {
    language: config.language,
    theme: config.theme,
    sync_mode: config.sync_mode,
    known_skill_install_permission: config.known_skill_install_permission,
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
  config.known_skill_install_permission = settings.known_skill_install_permission;
  save_config(&config)?;
  Ok(SettingsPayload {
    language: config.language,
    theme: config.theme,
    sync_mode: config.sync_mode,
    known_skill_install_permission: config.known_skill_install_permission,
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
