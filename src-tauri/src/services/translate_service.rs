use crate::config::load_config;
use reqwest::Client;
use serde_json::{json, Value};
use std::time::Duration;

const OPENROUTER_CHAT_COMPLETIONS_URL: &str = "https://openrouter.ai/api/v1/chat/completions";
const DEFAULT_MODEL: &str = "openai/gpt-4o-mini";

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

pub async fn translate_skill_markdown(markdown: String) -> Result<String, String> {
  if markdown.trim().is_empty() {
    return Ok(markdown);
  }

  let config = load_config()?;
  let api_key = config
    .openrouter_api_key
    .as_deref()
    .map(str::trim)
    .filter(|value| !value.is_empty())
    .ok_or_else(|| "OpenRouter API Key 未配置，请先在设置中填写".to_string())?;

  let target_language = if config.translate_target_language.trim().is_empty() {
    "zh-CN".to_string()
  } else {
    config.translate_target_language.trim().to_string()
  };

  let system_prompt = format!(
    concat!(
      "You are a professional technical translator for SKILL.md files.\n",
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
      "model": DEFAULT_MODEL,
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

  Ok(content)
}
