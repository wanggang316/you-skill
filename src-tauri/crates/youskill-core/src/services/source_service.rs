//! Network checks against skill sources (GitHub) and the GitHub pull path.

use crate::models::{
  ActionResult, DetectedSkill, SkillSource, SourcePull, SourcePullResult, SourcePullStatus,
  SourceUpdate,
};
use crate::services::env::Env;
use crate::services::hub_service::{pull_from_dir, pull_hub_from_dir, resolve_skill, HubGuard};
use crate::services::lock_service::store;
use crate::services::migration_service::migrate_skill_ids;
use crate::services::remote_service::fetch_skills_by_names;
use crate::services::skill_service::detect_github_auto;
use crate::utils::folder::{copy_dir, create_temp_dir, is_staged_temp_path, CopyOpts, SKILL_MD};
use crate::utils::github::GithubHelper;
use crate::utils::path::remove_path_any;
use crate::utils::time::now_rfc3339;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

struct Candidate {
  id: String,
  name: String,
  repo: String,
  /// `None` when the source has no recorded branch.
  branch: Option<String>,
  skill_path: String,
  remote_sha: Option<String>,
}

/// Look up the latest tree SHA for every GitHub-sourced skill (or the skills the given ids
/// or names select), store it in the lock, and report which skills have an update available.
pub async fn check_source_updates(
  env: Env,
  keys: Option<Vec<String>>,
) -> Result<Vec<SourceUpdate>, String> {
  migrate_skill_ids(&env)?;
  let lock = store(&env).read()?;
  let candidates: Vec<Candidate> = lock
    .skills
    .iter()
    .filter(|(id, record)| {
      keys
        .as_ref()
        .map(|list| list.iter().any(|key| key == *id || *key == record.name))
        .unwrap_or(true)
    })
    .filter_map(|(id, record)| match &record.source {
      SkillSource::Github {
        repo,
        skill_path,
        branch,
        remote_sha,
        ..
      } => Some(Candidate {
        id: id.clone(),
        name: record.name.clone(),
        repo: repo.trim().to_string(),
        branch: branch.clone().filter(|b| !b.trim().is_empty()),
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

  // 1. The marketplace index knows the tree sha for skills it indexes (cheap, no rate
  // limit), but it can lag behind GitHub. Trust it only when it confirms the installed
  // sha; a different sha must be confirmed against GitHub.
  let names_to_query: Vec<String> = candidates.iter().map(|c| c.name.clone()).collect();
  if let Ok(remote_skills) = fetch_skills_by_names(names_to_query).await {
    for remote in remote_skills {
      let Some(sha) = remote.skill_path_sha.as_ref().filter(|s| !s.is_empty()) else {
        continue;
      };
      let matches = candidates
        .iter()
        .find(|c| c.name == remote.name && c.repo.eq_ignore_ascii_case(remote.source.trim()));
      if let Some(candidate) = matches.filter(|c| marketplace_confirms(c, sha)) {
        latest.insert(candidate.id.clone(), sha.clone());
      }
    }
  }

  // 2. Everything else goes to the GitHub trees API, one request per (repo, branch).
  let mut groups: BTreeMap<(String, Option<String>), Vec<&Candidate>> = BTreeMap::new();
  for candidate in &candidates {
    if latest.contains_key(&candidate.id) {
      continue;
    }
    if candidate.skill_path.trim().is_empty() {
      errors.insert(
        candidate.id.clone(),
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
        errors.insert(member.id.clone(), format!("Invalid repository: {}", repo));
      }
      continue;
    };
    let paths: Vec<String> = members.iter().map(|m| m.skill_path.clone()).collect();
    let (branch, shas) = on_first_branch(branch.as_deref(), |branch| {
      let (owner, repo_name, paths) = (&owner, &repo_name, &paths);
      async move { GithubHelper::get_tree_shas(owner, repo_name, &branch, paths).await }
    })
    .await;
    match shas {
      Ok(shas) => {
        for member in members {
          match shas.get(&member.skill_path) {
            Some(sha) => {
              latest.insert(member.id.clone(), sha.clone());
            },
            None => {
              errors.insert(
                member.id.clone(),
                format!("Skill folder not found on branch {}", branch),
              );
            },
          }
        }
      },
      Err(err) => {
        for member in members {
          errors.insert(member.id.clone(), err.clone());
        }
      },
    }
  }

  let checked_at = now_rfc3339();
  let latest_snapshot = latest.clone();
  store(&env).update(|lock| {
    for (id, sha) in &latest_snapshot {
      if let Some(record) = lock.skills.get_mut(id) {
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
        let latest_sha = latest.get(&candidate.id).cloned();
        let update_available = match (&candidate.remote_sha, &latest_sha) {
          (Some(current), Some(latest)) => current != latest,
          _ => false,
        };
        SourceUpdate {
          error: errors.remove(&candidate.id),
          id: candidate.id,
          name: candidate.name,
          remote_sha: candidate.remote_sha,
          latest_remote_sha: latest_sha,
          update_available,
        }
      })
      .collect(),
  )
}

/// Run `fetch` on the recorded branch. Without one, try the default branches in the order
/// a download tries them, so the check and the pull look at the same branch. Returns the
/// branch of the result.
async fn on_first_branch<T, F, Fut>(
  branch: Option<&str>,
  mut fetch: F,
) -> (String, Result<T, String>)
where
  F: FnMut(String) -> Fut,
  Fut: std::future::Future<Output = Result<T, String>>,
{
  let branches = match branch {
    Some(branch) => vec![branch.to_string()],
    None => GithubHelper::default_branches(),
  };
  let mut last = (String::new(), Err("No branch to check".to_string()));
  for branch in branches {
    let result = fetch(branch.clone()).await;
    if result.is_ok() {
      return (branch, result);
    }
    last = (branch, result);
  }
  last
}

/// A marketplace sha is only a shortcut when it equals the installed sha.
fn marketplace_confirms(candidate: &Candidate, marketplace_sha: &str) -> bool {
  candidate.remote_sha.as_deref() == Some(marketplace_sha)
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
  key: String,
  force: bool,
) -> Result<ActionResult, String> {
  let (id, record) = resolve_skill(&env, &key)?;
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

  let detected = stage_github_source(&repo, &url, branch.as_deref(), &record.name).await?;
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
  tokio::task::spawn_blocking(move || {
    pull_from_dir(&env, &id, Path::new(&tmp_path), Some(updated_source), force)
  })
  .await
  .map_err(|e| format!("pull join error: {}", e))?
}

/// One GitHub-sourced skill of a batch pull.
struct PullMember {
  id: String,
  name: String,
  repo: String,
  url: String,
  skill_path: String,
  marketplace_id: Option<String>,
  latest_remote_sha: Option<String>,
  /// Hub hash when the update was decided; see `SourcePull`.
  hub_hash: Option<String>,
  force: bool,
}

/// Skills of one `(repo, branch)`; the key has the repository in lowercase.
type PullGroups = BTreeMap<(String, Option<String>), Vec<PullMember>>;

/// Pull the GitHub sources of `pulls` into the hub. Each `(repo, branch)` is downloaded one
/// time. The remote sha comes from the `latest_remote_sha` that the last source check
/// stored. A skill is skipped when its hub copy is no longer in the state the update was
/// decided on (`hub_hash`), or has local changes without `force`. Install targets are not
/// pushed. Returns one result for each pull, in order.
pub async fn pull_github_sources(
  env: Env,
  pulls: Vec<SourcePull>,
) -> Result<Vec<SourcePullResult>, String> {
  migrate_skill_ids(&env)?;
  let (groups, mut results) = plan_batch_pull(&env, &pulls)?;
  for ((_, branch), members) in groups {
    let tree = download_branch_tree(&members[0].repo, branch.as_deref()).await;
    let env = env.clone();
    let pulled = tokio::task::spawn_blocking(move || pull_group_from_tree(&env, &members, tree))
      .await
      .map_err(|e| format!("pull join error: {}", e))?;
    results.extend(pulled);
  }
  results.sort_by_key(|result| pulls.iter().position(|pull| pull.id == result.id));
  Ok(results)
}

/// Group the GitHub skills of `pulls` by `(repo, branch)`. Skills that cannot be pulled get
/// a failed result.
fn plan_batch_pull(
  env: &Env,
  pulls: &[SourcePull],
) -> Result<(PullGroups, Vec<SourcePullResult>), String> {
  let lock = store(env).read()?;
  let mut groups = PullGroups::new();
  let mut failed = Vec::new();
  for pull in pulls {
    let key = &pull.id;
    let Some((id, record)) = lock
      .skills
      .get_key_value(key)
      .or_else(|| lock.skills.iter().find(|(_, record)| record.name == *key))
    else {
      failed.push(pull_failed(
        key,
        key,
        format!("Skill '{}' is not in the hub", key),
      ));
      continue;
    };
    let SkillSource::Github {
      repo,
      url,
      skill_path,
      branch,
      marketplace_id,
      latest_remote_sha,
      ..
    } = &record.source
    else {
      failed.push(pull_failed(
        id,
        &record.name,
        "Skill is not sourced from GitHub".to_string(),
      ));
      continue;
    };
    let branch = branch.clone().filter(|b| !b.trim().is_empty());
    groups
      .entry((repo.trim().to_ascii_lowercase(), branch))
      .or_default()
      .push(PullMember {
        id: id.clone(),
        name: record.name.clone(),
        repo: repo.trim().to_string(),
        url: url.clone(),
        skill_path: skill_path.clone(),
        marketplace_id: marketplace_id.clone(),
        latest_remote_sha: latest_remote_sha.clone(),
        hub_hash: pull.hub_hash.clone(),
        force: pull.force,
      });
  }
  Ok((groups, failed))
}

/// Download and extract one `(repo, branch)` into a temp directory. Returns the directory
/// and the branch that was downloaded. Without a recorded branch the default branch is used,
/// the same as a single pull; a recorded branch must exist.
async fn download_branch_tree(
  repo: &str,
  branch: Option<&str>,
) -> Result<(PathBuf, String), String> {
  let (owner, repo_name) = GithubHelper::parse_github_url(repo)?;
  let dest = create_temp_dir(&format!("pull-github-{}-{}", owner, repo_name))?;
  let downloaded = GithubHelper::clone_repo_to(&owner, &repo_name, branch, &dest).await;
  match downloaded {
    Ok(got) if branch.is_none_or(|wanted| wanted == got) => Ok((dest, got)),
    Ok(_) => {
      let _ = remove_path_any(&dest);
      Err(format!(
        "Branch {} not found in {}",
        branch.unwrap_or_default(),
        repo
      ))
    },
    Err(err) => {
      let _ = remove_path_any(&dest);
      Err(err)
    },
  }
}

/// Pull each member from the extracted tree of its `(repo, branch)`, then remove the tree.
/// A failed download fails every member.
fn pull_group_from_tree(
  env: &Env,
  members: &[PullMember],
  tree: Result<(PathBuf, String), String>,
) -> Vec<SourcePullResult> {
  let (root, branch) = match tree {
    Ok(tree) => tree,
    Err(err) => {
      return members
        .iter()
        .map(|member| pull_failed(&member.id, &member.name, err.clone()))
        .collect();
    },
  };
  let results = members
    .iter()
    .map(|member| match pull_member(env, &root, &branch, member) {
      Ok(result) => result,
      Err(err) => pull_failed(&member.id, &member.name, err),
    })
    .collect();
  if is_staged_temp_path(&root) {
    let _ = remove_path_any(&root);
  }
  results
}

fn pull_member(
  env: &Env,
  root: &Path,
  branch: &str,
  member: &PullMember,
) -> Result<SourcePullResult, String> {
  let folder = GithubHelper::skill_folder_of(&member.skill_path)?;
  let src = root.join(&folder);
  if !src.join(SKILL_MD).is_file() {
    return Err(format!("Skill folder not found on branch {}", branch));
  }
  // A staged copy, because the pull removes its source and skill folders can nest.
  let staged_root = create_temp_dir("staged")?;
  let staged = staged_root.join(&member.name);
  let pulled = copy_dir(&src, &staged, &CopyOpts::skill()).and_then(|()| {
    let source = SkillSource::Github {
      repo: member.repo.clone(),
      url: member.url.clone(),
      skill_path: member.skill_path.clone(),
      branch: Some(branch.to_string()),
      remote_sha: member.latest_remote_sha.clone(),
      marketplace_id: member.marketplace_id.clone(),
      latest_remote_sha: member.latest_remote_sha.clone(),
      checked_at: Some(now_rfc3339()),
    };
    let guard = HubGuard {
      force: member.force,
      expected_hash: member.hub_hash.as_deref(),
    };
    pull_hub_from_dir(env, &member.id, &staged, Some(source), guard)
  });
  let _ = remove_path_any(&staged_root);
  let (result, reason) = pulled?;
  Ok(SourcePullResult {
    id: member.id.clone(),
    name: member.name.clone(),
    status: if result.applied {
      SourcePullStatus::Updated
    } else {
      SourcePullStatus::Skipped
    },
    message: (!result.blockers.is_empty()).then(|| result.blockers.join("\n")),
    reason,
  })
}

fn pull_failed(id: &str, name: &str, message: String) -> SourcePullResult {
  SourcePullResult {
    id: id.to_string(),
    name: name.to_string(),
    status: SourcePullStatus::Failed,
    message: Some(message),
    reason: None,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn candidate(remote_sha: Option<&str>) -> Candidate {
    Candidate {
      id: "00000000-0000-4000-8000-000000000001".to_string(),
      name: "pdf".to_string(),
      repo: "anthropics/skills".to_string(),
      branch: Some("main".to_string()),
      skill_path: "skills/pdf/SKILL.md".to_string(),
      remote_sha: remote_sha.map(str::to_string),
    }
  }

  #[test]
  fn marketplace_sha_equal_to_installed_sha_is_trusted() {
    assert!(marketplace_confirms(&candidate(Some("a")), "a"));
  }

  #[test]
  fn marketplace_sha_different_from_installed_sha_needs_github() {
    assert!(!marketplace_confirms(&candidate(Some("a")), "b"));
  }

  #[test]
  fn marketplace_sha_without_installed_sha_needs_github() {
    assert!(!marketplace_confirms(&candidate(None), "a"));
  }

  fn tried_branches(branch: Option<&str>, ok_on: &str) -> (Vec<String>, String, bool) {
    let mut tried = Vec::new();
    let (used, result) = tokio::runtime::Builder::new_current_thread()
      .build()
      .unwrap()
      .block_on(on_first_branch(branch, |b| {
        tried.push(b.clone());
        let ok = b == ok_on;
        async move {
          if ok {
            Ok(())
          } else {
            Err(format!("HTTP error: 404 on {b}"))
          }
        }
      }));
    (tried, used, result.is_ok())
  }

  #[test]
  fn a_check_without_branch_falls_back_to_master_like_the_download() {
    let (tried, used, ok) = tried_branches(None, "master");
    assert_eq!(tried, vec!["main", "master"]);
    assert_eq!(used, "master");
    assert!(ok);
  }

  #[test]
  fn a_check_with_a_recorded_branch_uses_only_that_branch() {
    let (tried, used, ok) = tried_branches(Some("feature/x"), "main");
    assert_eq!(tried, vec!["feature/x"]);
    assert_eq!(used, "feature/x");
    assert!(!ok);
  }

  mod batch_pull {
    use super::*;
    use crate::models::{
      ImportItem, InstallScope, InstallTargetSpec, SourceSkipReason, TargetState,
    };
    use crate::services::hub_service::{find_skill, import_skills};
    use crate::services::install_service::install_skill;
    use crate::utils::hash::{hash_dir, invalidate_hash_cache};
    use std::fs;

    fn write(root: &Path, rel: &str, content: &str) {
      let path = root.join(rel);
      fs::create_dir_all(path.parent().unwrap()).unwrap();
      fs::write(path, content).unwrap();
    }

    fn skill_md(name: &str, body: &str) -> String {
      format!("---\nname: {name}\n---\n{body}")
    }

    fn github(repo: &str, folder: &str, branch: Option<&str>, latest: &str) -> SkillSource {
      SkillSource::Github {
        repo: repo.to_string(),
        url: format!("https://github.com/{repo}.git"),
        skill_path: format!("{folder}/SKILL.md"),
        branch: branch.map(str::to_string),
        remote_sha: Some("old".to_string()),
        marketplace_id: None,
        latest_remote_sha: Some(latest.to_string()),
        checked_at: None,
      }
    }

    /// Import a skill with body "v1" from `source`; returns its id.
    fn add(env: &Env, tmp: &Path, name: &str, source: SkillSource) -> String {
      let dir = tmp.join("staged").join(name);
      write(&dir, "SKILL.md", &skill_md(name, "v1"));
      import_skills(
        env,
        vec![ImportItem {
          name: name.to_string(),
          tmp_path: dir.to_string_lossy().to_string(),
          source,
          replace_id: None,
        }],
        false,
      )
      .unwrap()
      .remove(0)
      .id
    }

    /// Pulls without a recorded hub state and without force.
    fn plain(ids: &[String]) -> Vec<SourcePull> {
      ids
        .iter()
        .map(|id| SourcePull {
          id: id.clone(),
          hub_hash: None,
          force: false,
        })
        .collect()
    }

    fn hub_body(env: &Env, id: &str, name: &str) -> String {
      fs::read_to_string(env.hub_dir(id, name).join("SKILL.md")).unwrap()
    }

    /// An extracted repository tree with the skill folders at `skills/<name>`.
    fn tree(tmp: &Path, names: &[&str]) -> PathBuf {
      let root = tmp.join("tree");
      for name in names {
        write(
          &root,
          &format!("skills/{name}/SKILL.md"),
          &skill_md(name, "v2"),
        );
      }
      root
    }

    fn statuses(results: &[SourcePullResult]) -> Vec<(String, SourcePullStatus)> {
      results.iter().map(|r| (r.name.clone(), r.status)).collect()
    }

    #[test]
    fn groups_skills_by_repository_and_branch() {
      let tmp = tempfile::tempdir().unwrap();
      let env = Env::for_test(tmp.path(), vec![], vec![]);
      let a = add(
        &env,
        tmp.path(),
        "a",
        github("O/R", "skills/a", Some("main"), "s"),
      );
      let b = add(
        &env,
        tmp.path(),
        "b",
        github("o/r", "skills/b", Some("main"), "s"),
      );
      let c = add(
        &env,
        tmp.path(),
        "c",
        github("o/r", "skills/c", Some("next"), "s"),
      );
      let folder = tmp.path().join("folder-skill");
      write(&folder, "SKILL.md", &skill_md("d", "v1"));
      let d = add(
        &env,
        tmp.path(),
        "d",
        SkillSource::Folder {
          path: folder.to_string_lossy().to_string(),
        },
      );

      let ids = vec![a, b, c, d.clone(), "missing".to_string()];
      let (groups, failed) = plan_batch_pull(&env, &plain(&ids)).unwrap();
      let keys: Vec<_> = groups.keys().cloned().collect();
      assert_eq!(
        keys,
        vec![
          ("o/r".to_string(), Some("main".to_string())),
          ("o/r".to_string(), Some("next".to_string())),
        ]
      );
      assert_eq!(groups[&keys[0]].len(), 2);
      let failed_ids: Vec<_> = failed.iter().map(|r| r.id.as_str()).collect();
      assert_eq!(failed_ids, vec![d.as_str(), "missing"]);
      assert!(failed.iter().all(|r| r.status == SourcePullStatus::Failed));
    }

    #[test]
    fn pulls_every_skill_of_a_group_from_one_tree_and_stores_the_remote_sha() {
      let tmp = tempfile::tempdir().unwrap();
      let env = Env::for_test(tmp.path(), vec![], vec![]);
      let a = add(
        &env,
        tmp.path(),
        "a",
        github("o/r", "skills/a", None, "sha-a"),
      );
      let b = add(
        &env,
        tmp.path(),
        "b",
        github("o/r", "skills/b", None, "sha-b"),
      );
      let root = tree(tmp.path(), &["a", "b"]);

      let (groups, _) = plan_batch_pull(&env, &plain(&[a.clone(), b.clone()])).unwrap();
      assert_eq!(groups.len(), 1);
      let members = groups.into_values().next().unwrap();
      let results = pull_group_from_tree(&env, &members, Ok((root.clone(), "main".into())));

      assert_eq!(
        statuses(&results),
        vec![
          ("a".to_string(), SourcePullStatus::Updated),
          ("b".to_string(), SourcePullStatus::Updated),
        ]
      );
      assert!(hub_body(&env, &a, "a").ends_with("v2"));
      assert!(hub_body(&env, &b, "b").ends_with("v2"));
      // A tree outside the staged temp directories is left alone.
      assert!(root.join("skills/a/SKILL.md").is_file());
      let (_, record) = find_skill(&env, &a).unwrap().unwrap();
      match record.source {
        SkillSource::Github {
          remote_sha,
          latest_remote_sha,
          branch,
          ..
        } => {
          assert_eq!(remote_sha.as_deref(), Some("sha-a"));
          assert_eq!(latest_remote_sha.as_deref(), Some("sha-a"));
          assert_eq!(branch.as_deref(), Some("main"));
        },
        other => panic!("unexpected source {other:?}"),
      }
    }

    #[test]
    fn local_changes_are_skipped_without_force_and_overwritten_with_force() {
      let tmp = tempfile::tempdir().unwrap();
      let env = Env::for_test(tmp.path(), vec![], vec![]);
      let a = add(&env, tmp.path(), "a", github("o/r", "skills/a", None, "s"));
      let hub = env.hub_dir(&a, "a");
      write(&hub, "SKILL.md", &skill_md("a", "local"));
      invalidate_hash_cache(&hub);
      let root = tree(tmp.path(), &["a"]);
      let tree_of = || Ok((root.clone(), "main".to_string()));
      let seen = hash_dir(&hub).unwrap();
      let pull = |force| SourcePull {
        id: a.clone(),
        hub_hash: Some(seen.clone()),
        force,
      };

      let (groups, _) = plan_batch_pull(&env, &[pull(false)]).unwrap();
      let members = groups.into_values().next().unwrap();
      let skipped = pull_group_from_tree(&env, &members, tree_of());
      assert_eq!(skipped[0].status, SourcePullStatus::Skipped);
      assert_eq!(skipped[0].reason, Some(SourceSkipReason::LocalChanges));
      assert!(hub_body(&env, &a, "a").ends_with("local"));

      let (groups, _) = plan_batch_pull(&env, &[pull(true)]).unwrap();
      let members = groups.into_values().next().unwrap();
      let forced = pull_group_from_tree(&env, &members, tree_of());
      assert_eq!(forced[0].status, SourcePullStatus::Updated);
      assert_eq!(forced[0].reason, None);
      assert!(hub_body(&env, &a, "a").ends_with("v2"));
    }

    /// Decide the update of a skill (with local edits and a confirmed overwrite, or clean),
    /// run `change` on its hub copy, then pull it with the hash seen at the decision.
    fn pull_after_change(
      local_edit: bool,
      change: impl FnOnce(&Path),
    ) -> (tempfile::TempDir, Env, String, SourcePullResult) {
      let tmp = tempfile::tempdir().unwrap();
      let env = Env::for_test(tmp.path(), vec![], vec![]);
      let a = add(&env, tmp.path(), "a", github("o/r", "skills/a", None, "s"));
      let hub = env.hub_dir(&a, "a");
      if local_edit {
        write(&hub, "SKILL.md", &skill_md("a", "local"));
      }
      write(&hub, "notes.md", "mine");
      if !local_edit {
        // An extra file is a local change too; record it as the imported state.
        crate::services::hub_service::accept_hub(&env, &a).unwrap();
      }
      let pulls = vec![SourcePull {
        id: a.clone(),
        hub_hash: Some(hash_dir(&hub).unwrap()),
        force: local_edit,
      }];
      change(&hub);
      let root = tree(tmp.path(), &["a"]);

      let (groups, _) = plan_batch_pull(&env, &pulls).unwrap();
      let members = groups.into_values().next().unwrap();
      let mut results = pull_group_from_tree(&env, &members, Ok((root, "main".into())));
      (tmp, env, a, results.remove(0))
    }

    fn assert_skipped_after_check(result: &SourcePullResult) {
      assert_eq!(result.status, SourcePullStatus::Skipped);
      assert_eq!(result.reason, Some(SourceSkipReason::ChangedAfterCheck));
    }

    fn remove_skill_md(hub: &Path) {
      fs::remove_file(hub.join("SKILL.md")).unwrap();
    }

    #[test]
    fn a_forced_skill_edited_after_the_confirmation_is_skipped() {
      let (_tmp, env, a, result) = pull_after_change(true, |hub| {
        write(hub, "SKILL.md", &skill_md("a", "local again"));
      });
      assert_skipped_after_check(&result);
      assert!(hub_body(&env, &a, "a").ends_with("local again"));
    }

    #[test]
    fn a_forced_skill_whose_skill_md_was_removed_after_the_confirmation_is_skipped() {
      let (_tmp, env, a, result) = pull_after_change(true, remove_skill_md);
      assert_skipped_after_check(&result);
      let hub = env.hub_dir(&a, "a");
      assert!(!hub.join("SKILL.md").exists());
      assert_eq!(fs::read_to_string(hub.join("notes.md")).unwrap(), "mine");
    }

    #[test]
    fn a_clean_skill_whose_skill_md_was_removed_after_the_check_is_skipped() {
      let (_tmp, env, a, result) = pull_after_change(false, remove_skill_md);
      assert_skipped_after_check(&result);
      let hub = env.hub_dir(&a, "a");
      assert!(!hub.join("SKILL.md").exists());
      assert_eq!(fs::read_to_string(hub.join("notes.md")).unwrap(), "mine");
    }

    #[test]
    fn a_clean_skill_edited_after_the_check_is_skipped() {
      let (_tmp, env, a, result) = pull_after_change(false, |hub| {
        write(hub, "SKILL.md", &skill_md("a", "late edit"));
      });
      assert_skipped_after_check(&result);
      assert!(hub_body(&env, &a, "a").ends_with("late edit"));
    }

    #[test]
    fn a_skill_unchanged_since_the_decision_is_updated() {
      for local_edit in [true, false] {
        let (_tmp, env, a, result) = pull_after_change(local_edit, |_| {});
        assert_eq!(
          result.status,
          SourcePullStatus::Updated,
          "local_edit={local_edit}"
        );
        assert!(hub_body(&env, &a, "a").ends_with("v2"));
      }
    }

    #[test]
    fn a_missing_skill_folder_fails_only_that_skill() {
      let tmp = tempfile::tempdir().unwrap();
      let env = Env::for_test(tmp.path(), vec![], vec![]);
      let a = add(&env, tmp.path(), "a", github("o/r", "skills/a", None, "s"));
      let b = add(&env, tmp.path(), "b", github("o/r", "skills/b", None, "s"));
      let root = tree(tmp.path(), &["b"]);

      let (groups, _) = plan_batch_pull(&env, &plain(&[a, b])).unwrap();
      let members = groups.into_values().next().unwrap();
      let results = pull_group_from_tree(&env, &members, Ok((root, "dev".into())));
      assert_eq!(results[0].status, SourcePullStatus::Failed);
      assert!(results[0]
        .message
        .as_deref()
        .unwrap()
        .contains("branch dev"));
      assert_eq!(results[1].status, SourcePullStatus::Updated);
    }

    #[test]
    fn a_failed_download_fails_every_skill_of_the_group() {
      let tmp = tempfile::tempdir().unwrap();
      let env = Env::for_test(tmp.path(), vec![], vec![]);
      let a = add(&env, tmp.path(), "a", github("o/r", "skills/a", None, "s"));
      let b = add(&env, tmp.path(), "b", github("o/r", "skills/b", None, "s"));

      let (groups, _) = plan_batch_pull(&env, &plain(&[a.clone(), b])).unwrap();
      let members = groups.into_values().next().unwrap();
      let results = pull_group_from_tree(&env, &members, Err("HTTP error: 404".into()));
      assert!(results
        .iter()
        .all(|r| r.status == SourcePullStatus::Failed
          && r.message.as_deref() == Some("HTTP error: 404")));
      assert!(hub_body(&env, &a, "a").ends_with("v1"));
    }

    #[test]
    fn a_pull_does_not_push_to_install_targets() {
      let tmp = tempfile::tempdir().unwrap();
      let env = Env::for_test(
        tmp.path(),
        vec![crate::models::AgentApp {
          id: "claude-code".to_string(),
          display_name: "Claude Code".to_string(),
          project_path: Some(".claude/skills".to_string()),
          global_path: Some("~/.claude/skills".to_string()),
          detect_path: None,
          profile_path: None,
          global_profile_path: None,
          is_user_custom: false,
        }],
        vec![],
      );
      let a = add(&env, tmp.path(), "a", github("o/r", "skills/a", None, "s"));
      install_skill(
        &env,
        crate::models::InstallRequest {
          name: a.clone(),
          targets: vec![InstallTargetSpec {
            scope: InstallScope::User,
            project_path: None,
            agent_id: "claude-code".to_string(),
          }],
          mode: Some(crate::models::InstallMode::Copy),
          force: false,
        },
      )
      .unwrap();
      let root = tree(tmp.path(), &["a"]);

      let (groups, _) = plan_batch_pull(&env, &plain(std::slice::from_ref(&a))).unwrap();
      let members = groups.into_values().next().unwrap();
      let results = pull_group_from_tree(&env, &members, Ok((root, "main".into())));
      assert_eq!(results[0].status, SourcePullStatus::Updated);

      let target = env.home.join(".claude/skills/a/SKILL.md");
      assert!(fs::read_to_string(target).unwrap().ends_with("v1"));
      let view = crate::services::hub_service::list_hub_skills(&env)
        .unwrap()
        .into_iter()
        .find(|view| view.id == a)
        .unwrap();
      assert_eq!(view.installs[0].state, TargetState::Outdated);
    }
  }
}
