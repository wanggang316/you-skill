//! Network checks against skill sources (GitHub) and the GitHub pull path.

use crate::models::{ActionResult, DetectedSkill, SkillSource, SourceUpdate};
use crate::services::env::Env;
use crate::services::hub_service::pull_from_dir;
use crate::services::lock_service::store;
use crate::services::remote_service::fetch_skills_by_names;
use crate::services::skill_service::detect_github_auto;
use crate::utils::github::GithubHelper;
use crate::utils::time::now_rfc3339;
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

struct Candidate {
  name: String,
  repo: String,
  branch: String,
  skill_path: String,
  remote_sha: Option<String>,
}

/// Look up the latest tree SHA for every GitHub-sourced skill (or the given names), store
/// it in the lock, and report which skills have an update available.
pub async fn check_source_updates(
  env: Env,
  names: Option<Vec<String>>,
) -> Result<Vec<SourceUpdate>, String> {
  let lock = store(&env).read()?;
  let candidates: Vec<Candidate> = lock
    .skills
    .iter()
    .filter(|(name, _)| {
      names
        .as_ref()
        .map(|list| list.iter().any(|n| n == *name))
        .unwrap_or(true)
    })
    .filter_map(|(name, record)| match &record.source {
      SkillSource::Github {
        repo,
        skill_path,
        branch,
        remote_sha,
        ..
      } => Some(Candidate {
        name: name.clone(),
        repo: repo.trim().to_string(),
        branch: branch.clone().unwrap_or_else(|| "main".to_string()),
        skill_path: skill_path.clone(),
        remote_sha: remote_sha.clone(),
      }),
      _ => None,
    })
    .collect();

  if candidates.is_empty() {
    return Ok(Vec::new());
  }

  let mut latest: HashMap<String, String> = HashMap::new();
  let mut errors: HashMap<String, String> = HashMap::new();

  // 1. Marketplace knows the tree sha for skills it indexes (cheap, no rate limit).
  let names_to_query: Vec<String> = candidates.iter().map(|c| c.name.clone()).collect();
  if let Ok(remote_skills) = fetch_skills_by_names(names_to_query).await {
    for remote in remote_skills {
      let Some(sha) = remote.skill_path_sha.as_ref().filter(|s| !s.is_empty()) else {
        continue;
      };
      let matches = candidates
        .iter()
        .find(|c| c.name == remote.name && c.repo.eq_ignore_ascii_case(remote.source.trim()));
      if let Some(candidate) = matches {
        latest.insert(candidate.name.clone(), sha.clone());
      }
    }
  }

  // 2. Everything else goes to the GitHub trees API, one request per (repo, branch).
  let mut groups: BTreeMap<(String, String), Vec<&Candidate>> = BTreeMap::new();
  for candidate in &candidates {
    if latest.contains_key(&candidate.name) {
      continue;
    }
    if candidate.skill_path.trim().is_empty() {
      errors.insert(
        candidate.name.clone(),
        "Skill path inside the repository is unknown".to_string(),
      );
      continue;
    }
    groups
      .entry((candidate.repo.clone(), candidate.branch.clone()))
      .or_default()
      .push(candidate);
  }
  for ((repo, branch), members) in groups {
    let Ok((owner, repo_name)) = GithubHelper::parse_github_url(&repo) else {
      for member in members {
        errors.insert(member.name.clone(), format!("Invalid repository: {}", repo));
      }
      continue;
    };
    let paths: Vec<String> = members.iter().map(|m| m.skill_path.clone()).collect();
    match GithubHelper::get_tree_shas(&owner, &repo_name, &branch, &paths).await {
      Ok(shas) => {
        for member in members {
          match shas.get(&member.skill_path) {
            Some(sha) => {
              latest.insert(member.name.clone(), sha.clone());
            },
            None => {
              errors.insert(
                member.name.clone(),
                format!("Skill folder not found on branch {}", branch),
              );
            },
          }
        }
      },
      Err(err) => {
        for member in members {
          errors.insert(member.name.clone(), err.clone());
        }
      },
    }
  }

  let checked_at = now_rfc3339();
  let latest_snapshot = latest.clone();
  store(&env).update(|lock| {
    for (name, sha) in &latest_snapshot {
      if let Some(record) = lock.skills.get_mut(name) {
        if let SkillSource::Github {
          latest_remote_sha,
          checked_at: checked,
          ..
        } = &mut record.source
        {
          *latest_remote_sha = Some(sha.clone());
          *checked = Some(checked_at.clone());
        }
      }
    }
    Ok(())
  })?;

  Ok(
    candidates
      .into_iter()
      .map(|candidate| {
        let latest_sha = latest.get(&candidate.name).cloned();
        let update_available = match (&candidate.remote_sha, &latest_sha) {
          (Some(current), Some(latest)) => current != latest,
          _ => false,
        };
        SourceUpdate {
          name: candidate.name.clone(),
          remote_sha: candidate.remote_sha,
          latest_remote_sha: latest_sha,
          update_available,
          error: errors.remove(&candidate.name),
        }
      })
      .collect(),
  )
}

/// Download the current version of a GitHub-sourced skill into a staged temp directory.
pub async fn stage_github_source(
  repo: &str,
  url: &str,
  branch: Option<&str>,
  name: &str,
) -> Result<DetectedSkill, String> {
  let reference = match branch {
    Some(b) if !b.is_empty() => format!("https://github.com/{}/tree/{}", repo, b),
    _ => url.to_string(),
  };
  detect_github_auto(reference, name.to_string()).await
}

/// Download the current version of a GitHub-sourced skill and replace the hub copy.
pub async fn pull_github_source(
  env: Env,
  name: String,
  force: bool,
) -> Result<ActionResult, String> {
  let record = store(&env)
    .get(&name)?
    .ok_or_else(|| format!("Skill '{}' is not in the hub", name))?;
  let SkillSource::Github {
    repo,
    url,
    skill_path,
    branch,
    marketplace_id,
    latest_remote_sha,
    ..
  } = record.source.clone()
  else {
    return Err("Skill is not sourced from GitHub".to_string());
  };

  let detected = stage_github_source(&repo, &url, branch.as_deref(), &name).await?;
  let downloaded_branch = detected.branch.clone().or(branch);
  let remote_sha = match GithubHelper::get_skill_folder_hash(
    &url,
    &detected.skill_path,
    downloaded_branch.as_deref(),
  )
  .await
  {
    Ok(sha) => Some(sha),
    Err(_) => latest_remote_sha.clone(),
  };

  let updated_source = SkillSource::Github {
    repo,
    url,
    skill_path: if skill_path.trim().is_empty() {
      detected.skill_path.clone()
    } else {
      skill_path
    },
    branch: downloaded_branch,
    remote_sha: remote_sha.clone(),
    marketplace_id,
    latest_remote_sha: remote_sha,
    checked_at: Some(now_rfc3339()),
  };

  let tmp_path = detected.tmp_path.clone();
  tauri::async_runtime::spawn_blocking(move || {
    pull_from_dir(
      &env,
      &name,
      Path::new(&tmp_path),
      Some(updated_source),
      force,
    )
  })
  .await
  .map_err(|e| format!("pull join error: {}", e))?
}
