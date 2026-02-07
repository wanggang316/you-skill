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

#[tauri::command]
pub fn get_api_key() -> Result<Option<String>, String> {
  let config = load_config()?;
  Ok(config.api_key)
}

#[tauri::command]
pub fn set_api_key(api_key: Option<String>) -> Result<(), String> {
  let mut config = load_config()?;
  config.api_key = api_key;
  save_config(&config)?;
  Ok(())
}
