use crate::config::{config_path, load_config};
use crate::services::ai_service;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;
const DEFAULT_MODEL: &str = "openai/gpt-4o-mini";

fn build_translation_cache_key(model: &str, target_language: &str, markdown: &str) -> String {
  let mut hasher = Sha256::new();
  hasher.update(markdown.as_bytes());
  let markdown_hash = format!("{:x}", hasher.finalize());
  let raw = format!("v1|model={}|lang={}|hash={}", model.trim(), target_language.trim(), markdown_hash);
  let mut key_hasher = Sha256::new();
  key_hasher.update(raw.as_bytes());
  format!("{:x}", key_hasher.finalize())
}

fn translation_cache_dir_path() -> Result<PathBuf, String> {
  let base = config_path()?;
  let parent = base
    .parent()
    .ok_or_else(|| "无法获取翻译缓存目录".to_string())?;
  Ok(parent.join("translate_cache"))
}

fn translation_cache_file_path(cache_key: &str) -> Result<PathBuf, String> {
  Ok(translation_cache_dir_path()?.join(format!("{}.md", cache_key)))
}

fn load_translation_cache_entry(cache_key: &str) -> Option<String> {
  let path = match translation_cache_file_path(cache_key) {
    Ok(path) => path,
    Err(error) => {
      tracing::warn!("Failed to resolve translation cache file path: {}", error);
      return None;
    },
  };

  if !path.exists() {
    return None;
  }

  let content = match fs::read_to_string(&path) {
    Ok(content) => content,
    Err(error) => {
      tracing::warn!("Failed to read translation cache file {}: {}", path.display(), error);
      return None;
    },
  };

  if content.trim().is_empty() {
    return None;
  }
  Some(content)
}

fn save_translation_cache_entry(cache_key: &str, translated_markdown: &str) -> Result<(), String> {
  let dir = translation_cache_dir_path()?;
  fs::create_dir_all(&dir).map_err(|error| format!("创建翻译缓存目录失败: {}", error))?;
  let path = translation_cache_file_path(cache_key)?;
  fs::write(path, translated_markdown).map_err(|error| format!("写入翻译缓存失败: {}", error))
}

pub async fn translate_skill_markdown(markdown: String) -> Result<String, String> {
  if markdown.trim().is_empty() {
    return Ok(markdown);
  }

  let config = load_config()?;
  let target_language = if config.translate_target_language.trim().is_empty() {
    "zh-CN".to_string()
  } else {
    config.translate_target_language.trim().to_string()
  };
  let model = if config.translate_model.trim().is_empty() {
    DEFAULT_MODEL.to_string()
  } else {
    config.translate_model.trim().to_string()
  };
  let cache_key = build_translation_cache_key(&model, &target_language, &markdown);

  if let Some(cached_markdown) = load_translation_cache_entry(&cache_key) {
    if !cached_markdown.trim().is_empty() {
      return Ok(cached_markdown);
    }
  }

  let api_key = config
    .openrouter_api_key
    .as_deref()
    .map(str::trim)
    .filter(|value| !value.is_empty())
    .ok_or_else(|| "OpenRouter API Key 未配置，请先在设置中填写".to_string())?;
  let content =
    ai_service::translate_markdown_with_openrouter(api_key, &model, &target_language, &markdown).await?;

  if let Err(error) = save_translation_cache_entry(&cache_key, &content) {
    tracing::warn!("Failed to save translation cache: {}", error);
  }

  Ok(content)
}
