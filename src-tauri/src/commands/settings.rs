use crate::config::{load_config, save_config, AppSettings};

#[tauri::command]
pub fn get_settings() -> Result<AppSettings, String> {
  let config = load_config()?;
  Ok(config.settings)
}

#[tauri::command]
pub fn update_settings(settings: AppSettings) -> Result<AppSettings, String> {
  let mut config = load_config()?;
  config.settings = settings.clone();
  save_config(&config)?;
  Ok(settings)
}
