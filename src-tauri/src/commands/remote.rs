use crate::models::{RemoteSkill, RemoteSkillsResponse};
use reqwest::Client;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct ApiSkill {
  id: String,
  #[serde(rename = "skillId")]
  skill_id: String,
  name: String,
  installs: u64,
  source: String,
}

#[derive(Debug, Deserialize)]
struct ApiResponse {
  skills: Vec<ApiSkill>,
  #[serde(rename = "hasMore")]
  has_more: bool,
}

#[tauri::command]
pub async fn fetch_remote_skills(
  page: Option<u32>,
  page_size: Option<u32>,
  query: Option<String>,
) -> Result<RemoteSkillsResponse, String> {
  let client = Client::new();
  let base_url = "https://skills.sh/api/skills";

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

  let mut api_response = match client.get(&url).send().await {
    Ok(resp) => resp.json::<ApiResponse>().await.ok(),
    Err(_) => None,
  };

  if api_response.is_none() {
    api_response = match client.get(base_url).send().await {
      Ok(resp) => resp.json::<ApiResponse>().await.ok(),
      Err(_) => None,
    };
  }

  let api_response = api_response.ok_or("无法获取 skills.sh 列表")?;

  let mut skills: Vec<RemoteSkill> = api_response
    .skills
    .into_iter()
    .map(|skill| RemoteSkill {
      id: skill.id,
      skill_id: skill.skill_id,
      name: skill.name,
      installs: skill.installs,
      source: skill.source,
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
    has_more: api_response.has_more,
  })
}
