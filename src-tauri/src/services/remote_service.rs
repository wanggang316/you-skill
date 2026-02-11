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
  /// GitHub tree SHA for skill folder (for update detection)
  #[serde(default)]
  skill_path_sha: Option<String>,
  /// Git branch name (e.g., "main", "master")
  #[serde(default)]
  branch: Option<String>,
}

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

  // Sorting params - default to heat_score desc
  let sort_by = sort_by.unwrap_or_else(|| "heat_score".to_string());
  let sort_order = sort_order.unwrap_or_else(|| "desc".to_string());

  let response: SkillsResponse = match client
    .get(base_url)
    .query(
      &[
        ("skip", skip.map(|s| s.to_string())),
        ("limit", limit.map(|l| l.to_string())),
        (
          "search",
          search
            .filter(|s| !s.trim().is_empty())
            .map(|s| s.trim().to_string()),
        ),
        ("sort_by", Some(sort_by)),
        ("sort_order", Some(sort_order)),
      ]
      .into_iter()
      .filter_map(|(k, v)| v.map(|val| (k, val)))
      .collect::<Vec<_>>(),
    )
    .header("X-API-Key", API_KEY)
    .send()
    .await
  {
    Ok(resp) => {
      if resp.status().is_success() {
        resp
          .json::<SkillsResponse>()
          .await
          .map_err(|e| format!("JSON 解析失败: {}", e))?
      } else {
        return Err(format!("API 返回错误: {}", resp.status()));
      }
    },
    Err(e) => {
      return Err(format!("网络连接失败: {}", e));
    },
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
      skill_path_sha: skill.skill_path_sha.filter(|s| !s.is_empty()),
      branch: skill.branch.filter(|s| !s.is_empty()),
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

/// Fetch skills by a list of names (for update detection)
pub async fn fetch_skills_by_names(names: Vec<String>) -> Result<Vec<RemoteSkill>, String> {
  tracing::info!("Fetching skills by names: {:?}", names);
  if names.is_empty() {
    return Ok(Vec::new());
  }

  let client = Client::builder()
    .timeout(Duration::from_secs(10))
    .build()
    .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

  let url = "https://you-skills-console.vercel.app/api/skills/by-names";
  let names_param = names.join(",");

  let response: Vec<ApiSkill> = match client
    .get(url)
    .query(&[("names", names_param)])
    .header("X-API-Key", API_KEY)
    .send()
    .await
  {
    Ok(resp) => {
      if resp.status().is_success() {
        resp
          .json::<Vec<ApiSkill>>()
          .await
          .map_err(|e| format!("JSON parsing failed: {}", e))?
      } else {
        return Err(format!("API returned error: {}", resp.status()));
      }
    },
    Err(e) => {
      return Err(format!("Network connection failed: {}", e));
    },
  };

  let skills: Vec<RemoteSkill> = response
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
      skill_path_sha: skill.skill_path_sha.filter(|s| !s.is_empty()),
      branch: skill.branch.filter(|s| !s.is_empty()),
    })
    .collect();

  Ok(skills)
}

pub async fn record_skill_install(skill_id: String) -> Result<(), String> {
  let client = Client::builder()
    .timeout(Duration::from_secs(10))
    .build()
    .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

  let url = format!(
    "https://you-skills-console.vercel.app/api/skills/{}/install",
    skill_id
  );

  let response = client.post(&url).header("X-API-Key", API_KEY).send().await;

  match response {
    Ok(resp) => {
      if resp.status().is_success() {
        Ok(())
      } else {
        Err(format!("API returned error: {}", resp.status()))
      }
    },
    Err(e) => Err(format!("Failed to record install: {}", e)),
  }
}
