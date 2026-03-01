use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::Duration;

const OPENROUTER_CHAT_COMPLETIONS_URL: &str = "https://openrouter.ai/api/v1/chat/completions";
const OPENROUTER_MODELS_URL: &str = "https://openrouter.ai/api/v1/models";

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

pub async fn translate_markdown_with_openrouter(
  api_key: &str,
  model: &str,
  target_language: &str,
  markdown: &str,
) -> Result<String, String> {
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

  payload
    .get("choices")
    .and_then(Value::as_array)
    .and_then(|choices| choices.first())
    .and_then(|choice| choice.get("message"))
    .and_then(|message| message.get("content"))
    .and_then(get_content_text)
    .map(|text| text.trim().to_string())
    .filter(|text| !text.is_empty())
    .ok_or_else(|| "OpenRouter 未返回可用翻译结果".to_string())
}

pub async fn list_openrouter_models(
  api_key: Option<&str>,
  search: Option<String>,
) -> Result<Vec<OpenRouterModelOption>, String> {
  let client = Client::builder()
    .timeout(Duration::from_secs(30))
    .build()
    .map_err(|error| format!("Failed to create HTTP client: {}", error))?;

  let mut request = client
    .get(OPENROUTER_MODELS_URL)
    .header("HTTP-Referer", "https://github.com/gumpwang/you-skill")
    .header("X-Title", "YouSkill");

  if let Some(api_key) = api_key.map(str::trim).filter(|value| !value.is_empty()) {
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
