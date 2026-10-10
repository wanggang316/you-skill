use crate::config::{config_path, load_config, AppConfig};
use crate::services::ai_service;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

fn build_translation_system_prompt(target_language: &str) -> String {
  format!(
    concat!(
      "You are a professional technical translator for Markdown documentation.\n",
      "Translate the user-provided markdown into {target_language}.\n",
      "Rules:\n",
      "1. Keep markdown structure exactly (headings, lists, tables, blockquotes).\n",
      "2. Keep code fences, inline code, file paths, commands, URLs, env var names unchanged.\n",
      "3. Keep YAML frontmatter keys unchanged; translate only human-readable values.\n",
      "4. Do not add explanations, notes, or surrounding markdown/code fences.\n",
      "5. Return only the translated markdown content."
    ),
    target_language = target_language
  )
}

fn build_translation_cache_key(model: &str, target_language: &str, markdown: &str) -> String {
  let mut hasher = Sha256::new();
  hasher.update(markdown.as_bytes());
  let markdown_hash = format!("{:x}", hasher.finalize());
  let raw = format!(
    "v1|model={}|lang={}|hash={}",
    model.trim(),
    target_language.trim(),
    markdown_hash
  );
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

fn translation_cache_file_path(cache_dir: &Path, cache_key: &str) -> PathBuf {
  cache_dir.join(format!("{}.md", cache_key))
}

fn load_translation_cache_entry(cache_dir: &Path, cache_key: &str) -> Option<String> {
  let path = translation_cache_file_path(cache_dir, cache_key);
  if !path.exists() {
    return None;
  }

  let content = match fs::read_to_string(&path) {
    Ok(content) => content,
    Err(error) => {
      tracing::warn!(
        "Failed to read translation cache file {}: {}",
        path.display(),
        error
      );
      return None;
    },
  };

  if content.trim().is_empty() {
    return None;
  }
  Some(content)
}

fn save_translation_cache_entry(
  cache_dir: &Path,
  cache_key: &str,
  translated_markdown: &str,
) -> Result<(), String> {
  fs::create_dir_all(cache_dir).map_err(|error| format!("创建翻译缓存目录失败: {}", error))?;
  let path = translation_cache_file_path(cache_dir, cache_key);
  fs::write(path, translated_markdown).map_err(|error| format!("写入翻译缓存失败: {}", error))
}

/// Returns the cached translation for `markdown` under the current model and target
/// language. Never calls the translation API.
pub fn cached_skill_translation(markdown: &str) -> Result<Option<String>, String> {
  let config = load_config()?;
  Ok(lookup_cached_translation(
    &config,
    &translation_cache_dir_path()?,
    markdown,
  ))
}

fn lookup_cached_translation(
  config: &AppConfig,
  cache_dir: &Path,
  markdown: &str,
) -> Option<String> {
  let target_language = config.translate_target_language.trim();
  let model = config.translate_model.trim();
  if markdown.trim().is_empty() || target_language.is_empty() || model.is_empty() {
    return None;
  }
  let cache_key = build_translation_cache_key(model, target_language, markdown);
  load_translation_cache_entry(cache_dir, &cache_key)
}

pub async fn translate_skill_markdown(markdown: String) -> Result<String, String> {
  if markdown.trim().is_empty() {
    return Ok(markdown);
  }

  let config = load_config()?;
  let target_language = config.translate_target_language.trim().to_string();
  if target_language.is_empty() {
    return Err("Target language 未配置，请先在设置中填写".to_string());
  }
  let model = config.translate_model.trim().to_string();
  if model.is_empty() {
    return Err("Model 未配置，请先在设置中填写".to_string());
  }
  let cache_dir = translation_cache_dir_path()?;
  if let Some(cached_markdown) = lookup_cached_translation(&config, &cache_dir, &markdown) {
    return Ok(cached_markdown);
  }

  let api_key = config
    .openrouter_api_key
    .as_deref()
    .map(str::trim)
    .filter(|value| !value.is_empty())
    .ok_or_else(|| "OpenRouter API Key 未配置，请先在设置中填写".to_string())?;
  let messages = vec![
    ai_service::OpenRouterMessage {
      role: "system".to_string(),
      content: build_translation_system_prompt(&target_language),
    },
    ai_service::OpenRouterMessage {
      role: "user".to_string(),
      content: markdown.clone(),
    },
  ];
  let content = ai_service::chat_completion_with_openrouter(api_key, &model, messages, 0.1).await?;

  let cache_key = build_translation_cache_key(&model, &target_language, &markdown);
  if let Err(error) = save_translation_cache_entry(&cache_dir, &cache_key, &content) {
    tracing::warn!("Failed to save translation cache: {}", error);
  }

  Ok(content)
}

#[cfg(test)]
mod tests {
  use super::*;

  fn translate_config() -> AppConfig {
    AppConfig {
      translate_target_language: "Chinese".to_string(),
      translate_model: "test/model".to_string(),
      ..AppConfig::default()
    }
  }

  #[test]
  fn lookup_returns_saved_translation() {
    let dir = tempfile::tempdir().unwrap();
    let config = translate_config();
    let key = build_translation_cache_key("test/model", "Chinese", "# Title");
    save_translation_cache_entry(dir.path(), &key, "# 标题").unwrap();

    assert_eq!(
      lookup_cached_translation(&config, dir.path(), "# Title").as_deref(),
      Some("# 标题")
    );
  }

  #[test]
  fn lookup_misses_when_markdown_or_settings_change() {
    let dir = tempfile::tempdir().unwrap();
    let config = translate_config();
    let key = build_translation_cache_key("test/model", "Chinese", "# Title");
    save_translation_cache_entry(dir.path(), &key, "# 标题").unwrap();

    assert_eq!(
      lookup_cached_translation(&config, dir.path(), "# Other"),
      None
    );
    let other_model = AppConfig {
      translate_model: "other/model".to_string(),
      ..translate_config()
    };
    assert_eq!(
      lookup_cached_translation(&other_model, dir.path(), "# Title"),
      None
    );
    let unconfigured = AppConfig::default();
    assert_eq!(
      lookup_cached_translation(&unconfigured, dir.path(), "# Title"),
      None
    );
  }

  #[test]
  fn lookup_ignores_blank_cache_entry() {
    let dir = tempfile::tempdir().unwrap();
    let config = translate_config();
    let key = build_translation_cache_key("test/model", "Chinese", "# Title");
    save_translation_cache_entry(dir.path(), &key, "  \n").unwrap();

    assert_eq!(
      lookup_cached_translation(&config, dir.path(), "# Title"),
      None
    );
  }
}
