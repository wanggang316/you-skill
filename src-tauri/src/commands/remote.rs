use crate::models::{RemoteSkill, RemoteSkillsResponse};
use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;

#[derive(Debug, Deserialize)]
struct SkillsResponse {
  skills: Vec<ApiSkill>,
}

#[derive(Debug, Deserialize)]
struct ApiSkill {
  id: i64,
  name: String,
  source: String,
  url: String,
  description: Option<String>,
  #[serde(default)]
  star_count: i64,
  path: Option<String>,
}

#[tauri::command]
pub async fn fetch_remote_skills(
  page: Option<u32>,
  page_size: Option<u32>,
  query: Option<String>,
) -> Result<RemoteSkillsResponse, String> {
  let client = Client::builder()
    .timeout(Duration::from_secs(10))
    .build()
    .map_err(|e| format!("Failed to create HTTP client: {}", e))?;
  let base_url = "http://127.0.0.1:8000/api/skills";

  let mut url = base_url.to_string();
  let mut params: Vec<(String, String)> = Vec::new();

  if let Some(page) = page {
    params.push(("page".to_string(), page.to_string()));
  }
  if let Some(limit) = page_size {
    params.push(("limit".to_string(), limit.to_string()));
  }
  if let Some(search) = query.clone() {
    if !search.trim().is_empty() {
      params.push(("search".to_string(), search.trim().to_string()));
    }
  }

  if !params.is_empty() {
    url.push('?');
    url.push_str(
      &params
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<String>>()
        .join("&"),
    );
  }

  let response: SkillsResponse = match client.get(&url).send().await {
    Ok(resp) => {
      if resp.status().is_success() {
        resp.json::<SkillsResponse>().await.map_err(|e| {
          format!("JSON 解析失败: {}", e)
        })?
      } else {
        return Err(format!(
          "API 返回错误状态码: {}，请检查 127.0.0.1:8000 服务",
          resp.status()
        ));
      }
    }
    Err(e) => {
      return Err(format!(
        "无法连接到 127.0.0.1:8000: {}，请确保服务已启动",
        e
      ));
    }
  };

  let mut skills: Vec<RemoteSkill> = response
    .skills
    .into_iter()
    .map(|skill| RemoteSkill {
      id: skill.id.to_string(),
      skill_id: skill.id.to_string(),
      name: skill.name,
      installs: skill.star_count as u64,
      source: skill.source.clone(),
      url: Some(skill.url.clone()).filter(|s| !s.is_empty()),
      path: skill.path.clone().filter(|s| !s.is_empty()),
    })
    .collect();

  if let Some(search) = query {
    let needle = search.to_lowercase();
    if !needle.trim().is_empty() {
      skills.retain(|skill| {
        skill.name.to_lowercase().contains(&needle)
          || skill.source.to_lowercase().contains(&needle)
          || skill.skill_id.to_lowercase().contains(&needle)
      });
    }
  }

  Ok(RemoteSkillsResponse {
    skills,
    has_more: false,
  })
}
