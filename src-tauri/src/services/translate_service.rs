use crate::config::{config_path, load_config};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

const OPENROUTER_CHAT_COMPLETIONS_URL: &str = "https://openrouter.ai/api/v1/chat/completions";
const OPENROUTER_MODELS_URL: &str = "https://openrouter.ai/api/v1/models";
const DEFAULT_MODEL: &str = "openai/gpt-4o-mini";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenRouterModelOption {
  pub id: String,
  pub name: String,
}

#[derive(Debug, Deserialize)]
struct OpenRouterModelsResponse {
  data: Vec<OpenRouterModelItem>,
}

#[derive(Debug, Deserialize)]
struct OpenRouterModelItem {
  id: String,
  name: Option<String>,
}

fn get_content_text(content: &Value) -> Option<String> {
  if let Some(text) = content.as_str() {
    return Some(text.to_string());
  }

  let parts = content.as_array()?;
  let merged = parts
    .iter()
    .filter_map(|item| item.get("text").and_then(Value::as_str))
    .collect::<Vec<_>>()
    .join("");

  if merged.trim().is_empty() {
    None
  } else {
    Some(merged)
  }
}

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

  let system_prompt = format!(
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
  );

  let client = Client::builder()
    .timeout(Duration::from_secs(90))
    .build()
    .map_err(|error| format!("Failed to create HTTP client: {}", error))?;

  let response = client
    .post(OPENROUTER_CHAT_COMPLETIONS_URL)
    .header("Authorization", format!("Bearer {}", api_key))
    .header("Content-Type", "application/json")
    .header("HTTP-Referer", "https://github.com/gumpwang/you-skill")
    .header("X-Title", "YouSkill")
    .json(&json!({
      "model": model,
      "temperature": 0.1,
      "messages": [
        {
          "role": "system",
          "content": system_prompt
        },
        {
          "role": "user",
          "content": markdown
        }
      ]
    }))
    .send()
    .await
    .map_err(|error| format!("调用 OpenRouter 失败: {}", error))?;

  if !response.status().is_success() {
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    return Err(format!("OpenRouter 请求失败 ({}): {}", status, body));
  }

  let payload: Value = response
    .json()
    .await
    .map_err(|error| format!("解析 OpenRouter 响应失败: {}", error))?;

  let content = payload
    .get("choices")
    .and_then(Value::as_array)
    .and_then(|choices| choices.first())
    .and_then(|choice| choice.get("message"))
    .and_then(|message| message.get("content"))
    .and_then(get_content_text)
    .map(|text| text.trim().to_string())
    .filter(|text| !text.is_empty())
    .ok_or_else(|| "OpenRouter 未返回可用翻译结果".to_string())?;

  if let Err(error) = save_translation_cache_entry(&cache_key, &content) {
    tracing::warn!("Failed to save translation cache: {}", error);
  }

  Ok(content)
}

pub async fn list_openrouter_models(search: Option<String>) -> Result<Vec<OpenRouterModelOption>, String> {
  let config = load_config()?;
  let client = Client::builder()
    .timeout(Duration::from_secs(30))
    .build()
    .map_err(|error| format!("Failed to create HTTP client: {}", error))?;

  let mut request = client
    .get(OPENROUTER_MODELS_URL)
    .header("HTTP-Referer", "https://github.com/gumpwang/you-skill")
    .header("X-Title", "YouSkill");

  if let Some(api_key) = config
    .openrouter_api_key
    .as_deref()
    .map(str::trim)
    .filter(|value| !value.is_empty())
  {
    request = request.header("Authorization", format!("Bearer {}", api_key));
  }

  let response = request
    .send()
    .await
    .map_err(|error| format!("获取 OpenRouter 模型列表失败: {}", error))?;

  if !response.status().is_success() {
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    return Err(format!("OpenRouter 模型列表请求失败 ({}): {}", status, body));
  }

  let payload: OpenRouterModelsResponse = response
    .json()
    .await
    .map_err(|error| format!("解析 OpenRouter 模型列表失败: {}", error))?;

  let needle = search.unwrap_or_default().trim().to_lowercase();

  let mut models = payload
    .data
    .into_iter()
    .filter_map(|item| {
      let name = item.name.unwrap_or_else(|| item.id.clone());
      let matched = needle.is_empty()
        || item.id.to_lowercase().contains(&needle)
        || name.to_lowercase().contains(&needle);
      if !matched {
        return None;
      }
      Some(OpenRouterModelOption { id: item.id, name })
    })
    .collect::<Vec<_>>();

  models.sort_by(|a, b| a.id.cmp(&b.id));
  Ok(models)
}
