use crate::config::{
  load_config, save_config, DEFAULT_TRANSLATE_DISPLAY_MODE, DEFAULT_TRANSLATE_TEXT_STYLE,
  TRANSLATE_DISPLAY_MODES, TRANSLATE_TEXT_STYLES,
};
use crate::services::ai_service::{self, OpenRouterModelOption};
use crate::services::backup_service::{self, BackupResult};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SettingsPayload {
  pub language: String,
  pub theme: String,
  pub sync_mode: String,
  #[serde(default)]
  pub openrouter_api_key: Option<String>,
  #[serde(default = "default_translate_target_language")]
  pub translate_target_language: String,
  #[serde(default = "default_translate_model")]
  pub translate_model: String,
  #[serde(default = "default_translate_display_mode")]
  pub translate_display_mode: String,
  #[serde(default = "default_translate_text_style")]
  pub translate_text_style: String,
  pub backup_folder: Option<String>,
  pub last_backup_time: Option<String>,
}

fn default_translate_target_language() -> String {
  String::new()
}

fn default_translate_model() -> String {
  String::new()
}

fn default_translate_display_mode() -> String {
  DEFAULT_TRANSLATE_DISPLAY_MODE.to_string()
}

fn default_translate_text_style() -> String {
  DEFAULT_TRANSLATE_TEXT_STYLE.to_string()
}

/// Returns `value` when it is one of `allowed`, otherwise `fallback`.
fn normalize_choice(value: &str, allowed: &[&str], fallback: &str) -> String {
  let value = value.trim();
  if allowed.contains(&value) {
    value.to_string()
  } else {
    fallback.to_string()
  }
}

#[tauri::command]
pub fn get_settings() -> Result<SettingsPayload, String> {
  let config = load_config()?;
  Ok(SettingsPayload {
    language: config.language,
    theme: config.theme,
    sync_mode: config.sync_mode,
    openrouter_api_key: config.openrouter_api_key,
    translate_target_language: config.translate_target_language,
    translate_model: config.translate_model,
    translate_display_mode: normalize_choice(
      &config.translate_display_mode,
      &TRANSLATE_DISPLAY_MODES,
      DEFAULT_TRANSLATE_DISPLAY_MODE,
    ),
    translate_text_style: normalize_choice(
      &config.translate_text_style,
      &TRANSLATE_TEXT_STYLES,
      DEFAULT_TRANSLATE_TEXT_STYLE,
    ),
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
  config.translate_model = if settings.translate_model.trim().is_empty() {
    default_translate_model()
  } else {
    settings.translate_model.trim().to_string()
  };
  config.translate_display_mode = normalize_choice(
    &settings.translate_display_mode,
    &TRANSLATE_DISPLAY_MODES,
    DEFAULT_TRANSLATE_DISPLAY_MODE,
  );
  config.translate_text_style = normalize_choice(
    &settings.translate_text_style,
    &TRANSLATE_TEXT_STYLES,
    DEFAULT_TRANSLATE_TEXT_STYLE,
  );
  save_config(&config)?;
  Ok(SettingsPayload {
    language: config.language,
    theme: config.theme,
    sync_mode: config.sync_mode,
    openrouter_api_key: config.openrouter_api_key,
    translate_target_language: config.translate_target_language,
    translate_model: config.translate_model,
    translate_display_mode: normalize_choice(
      &config.translate_display_mode,
      &TRANSLATE_DISPLAY_MODES,
      DEFAULT_TRANSLATE_DISPLAY_MODE,
    ),
    translate_text_style: normalize_choice(
      &config.translate_text_style,
      &TRANSLATE_TEXT_STYLES,
      DEFAULT_TRANSLATE_TEXT_STYLE,
    ),
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

#[tauri::command]
pub async fn list_openrouter_models(
  search: Option<String>,
) -> Result<Vec<OpenRouterModelOption>, String> {
  let config = load_config()?;
  ai_service::list_openrouter_models(config.openrouter_api_key.as_deref(), search).await
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn normalize_choice_keeps_allowed_values() {
    assert_eq!(
      normalize_choice(" translation ", &TRANSLATE_DISPLAY_MODES, "bilingual"),
      "translation"
    );
    assert_eq!(
      normalize_choice("dashed_underline", &TRANSLATE_TEXT_STYLES, "none"),
      "dashed_underline"
    );
  }

  #[test]
  fn normalize_choice_falls_back_for_unknown_values() {
    assert_eq!(
      normalize_choice("", &TRANSLATE_DISPLAY_MODES, DEFAULT_TRANSLATE_DISPLAY_MODE),
      "bilingual"
    );
    assert_eq!(
      normalize_choice("wavy", &TRANSLATE_TEXT_STYLES, DEFAULT_TRANSLATE_TEXT_STYLE),
      "none"
    );
  }

  #[test]
  fn settings_payload_defaults_translate_view_options() {
    let payload: SettingsPayload = serde_json::from_str(
      r#"{"language":"en","theme":"system","sync_mode":"copy","backup_folder":null,"last_backup_time":null}"#,
    )
    .unwrap();
    assert_eq!(payload.translate_display_mode, "bilingual");
    assert_eq!(payload.translate_text_style, "none");
  }

  #[test]
  fn app_config_defaults_translate_view_options_for_old_files() {
    let config: crate::config::AppConfig = serde_json::from_str(r#"{"language":"zh"}"#).unwrap();
    assert_eq!(config.translate_display_mode, "bilingual");
    assert_eq!(config.translate_text_style, "none");
  }
}
