use crate::models::{RemoteSkill, RemoteSkillsResponse};
use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;

const API_KEY: &str = "4586a151c7950fa482a099dfa916ff9d616d02d9f10e710c31819607aabb9fff";

#[derive(Debug, Deserialize)]
struct SkillsResponse {
  skills: Vec<ApiSkill>,
  total: i64,
  page: i32,
  page_size: i32,
}

#[derive(Debug, Deserialize)]
struct ApiSkill {
  id: i64,
  name: String,
  source: String,
  url: String,
  #[allow(dead_code)]
  description: Option<String>,
  #[serde(default)]
  star_count: i64,
  #[serde(default)]
  heat_score: i64,
  #[serde(default)]
  install_count: i64,
  path: Option<String>,
}

#[tauri::command]
pub async fn fetch_remote_skills(
  skip: Option<u32>,
  limit: Option<u32>,
  search: Option<String>,
  sort_by: Option<String>,
  sort_order: Option<String>,
) -> Result<RemoteSkillsResponse, String> {
  let client = Client::builder()
    .timeout(Duration::from_secs(10))
    .build()
    .map_err(|e| format!("Failed to create HTTP client: {}", e))?;
  let base_url = "https://you-skills-console.vercel.app/api/skills";

  let mut url = base_url.to_string();
  let mut params: Vec<(String, String)> = Vec::new();

  // Pagination params
  if let Some(skip) = skip {
    params.push(("skip".to_string(), skip.to_string()));
  }
  if let Some(limit) = limit {
    params.push(("limit".to_string(), limit.to_string()));
  }

  // Search param
  if let Some(query) = search {
    if !query.trim().is_empty() {
      params.push(("search".to_string(), query.trim().to_string()));
    }
  }

  // Sorting params - default to heat_score desc
  let sort_by = sort_by.unwrap_or_else(|| "heat_score".to_string());
  let sort_order = sort_order.unwrap_or_else(|| "desc".to_string());
  params.push(("sort_by".to_string(), sort_by));
  params.push(("sort_order".to_string(), sort_order));

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

  let response: SkillsResponse = match client
    .get(&url)
    .header("X-API-Key", API_KEY)
    .send()
    .await {
    Ok(resp) => {
      if resp.status().is_success() {
        resp.json::<SkillsResponse>().await.map_err(|e| {
          format!("JSON 解析失败: {}", e)
        })?
      } else {
        return Err(format!("API 返回错误: {}", resp.status()));
      }
    }
    Err(e) => {
      return Err(format!("网络连接失败: {}", e));
    }
  };

  let total = response.total;
  let page_size = response.page_size as i64;
  let skills: Vec<RemoteSkill> = response
    .skills
    .into_iter()
    .map(|skill| RemoteSkill {
      id: skill.id.to_string(),
      skill_id: skill.id.to_string(),
      name: skill.name,
      star_count: skill.star_count as u64,
      heat_score: skill.heat_score as u64,
      install_count: skill.install_count as u64,
      source: skill.source.clone(),
      url: Some(skill.url.clone()).filter(|s| !s.is_empty()),
      path: skill.path.clone().filter(|s| !s.is_empty()),
    })
    .collect();

  // Calculate if there are more results
  let current_count = skills.len() as i64;
  let has_more = current_count > 0 && (response.page as i64 * page_size + current_count) < total;

  Ok(RemoteSkillsResponse {
    skills,
    total,
    has_more,
  })
}

#[tauri::command]
pub async fn record_skill_install(skill_id: String) -> Result<(), String> {
  let client = Client::builder()
    .timeout(Duration::from_secs(10))
    .build()
    .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

  let url = format!("https://you-skills-console.vercel.app/api/skills/{}/install", skill_id);

  let response = client
    .post(&url)
    .header("X-API-Key", API_KEY)
    .send()
    .await;

  match response {
    Ok(resp) => {
      if resp.status().is_success() {
        Ok(())
      } else {
        Err(format!("API returned error: {}", resp.status()))
      }
    }
    Err(e) => Err(format!("Failed to record install: {}", e)),
  }
}
