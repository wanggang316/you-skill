use std::path::Path;

use crate::config::{load_config, save_config};
use crate::services::backup_service::{
  backup_skills as backup_skills_service, BackupResult as ServiceBackupResult,
};

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct BackupResult {
  pub success: bool,
  pub message: String,
  pub backup_path: Option<String>,
  pub backup_time: Option<String>,
}

impl From<ServiceBackupResult> for BackupResult {
  fn from(result: ServiceBackupResult) -> Self {
    BackupResult {
      success: result.success,
      message: result.message,
      backup_path: result.backup_path,
      backup_time: result.backup_time,
    }
  }
}

#[tauri::command]
pub fn get_backup_folder() -> Result<Option<String>, String> {
  let config = load_config()?;
  Ok(config.backup_folder)
}

#[tauri::command]
pub fn get_last_backup_time() -> Result<Option<String>, String> {
  let config = load_config()?;
  Ok(config.last_backup_time)
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
  let backup_path = Path::new(&path);
  if !backup_path.exists() {
    std::fs::create_dir_all(backup_path).map_err(|e| format!("创建备份目录失败: {}", e))?;
  }

  #[cfg(target_os = "macos")]
  {
    std::process::Command::new("open")
      .arg(&path)
      .spawn()
      .map_err(|e| format!("打开目录失败: {}", e))?;
  }

  #[cfg(target_os = "windows")]
  {
    std::process::Command::new("explorer")
      .arg(&path)
      .spawn()
      .map_err(|e| format!("打开目录失败: {}", e))?;
  }

  #[cfg(target_os = "linux")]
  {
    std::process::Command::new("xdg-open")
      .arg(&path)
      .spawn()
      .map_err(|e| format!("打开目录失败: {}", e))?;
  }

  Ok(())
}

#[tauri::command]
pub async fn backup_skills(backup_folder: String) -> Result<BackupResult, String> {
  // Use spawn_blocking to run CPU-intensive operation on separate thread
  let result = tokio::task::spawn_blocking(move || backup_skills_service(backup_folder))
    .await
    .map_err(|e| format!("备份任务执行失败: {}", e))??;

  // If backup succeeded, save backup time to config
  if result.success {
    if let Ok(mut config) = load_config() {
      config.last_backup_time = result.backup_time.clone();
      let _ = save_config(&config);
    }
  }

  Ok(result.into())
}
