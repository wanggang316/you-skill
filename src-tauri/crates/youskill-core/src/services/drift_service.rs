//! Offline drift computation: compares the hub copy against its recorded hash, every
//! install target against the hub (three-way, using the hash recorded at install time as
//! the merge base), and the source when it is a local folder.

use crate::models::{
  HubSkillView, HubState, InstallRecord, InstallScope, InstallView, SkillRecord, SkillSource,
  SourceState, TargetState,
};
use crate::services::env::Env;
use crate::services::install_service::agents_reading;
use crate::utils::file::FileHelper;
use crate::utils::folder::SKILL_MD;
use crate::utils::hash::hash_dir_cached;
use crate::utils::path::{is_symlink, symlink_points_to, symlink_target_string};
use std::path::Path;

pub struct HubProbe {
  pub state: HubState,
  pub hash: Option<String>,
  pub description: Option<String>,
}

pub fn probe_hub(env: &Env, id: &str, record: &SkillRecord) -> HubProbe {
  let dir = env.hub_dir(id, &record.name);
  if !dir.is_dir() {
    return HubProbe {
      state: HubState::Missing,
      hash: None,
      description: None,
    };
  }
  let skill_md = dir.join(SKILL_MD);
  if !skill_md.is_file() {
    return HubProbe {
      state: HubState::Invalid,
      hash: None,
      description: None,
    };
  }
  let Ok(hash) = hash_dir_cached(&dir) else {
    return HubProbe {
      state: HubState::Invalid,
      hash: None,
      description: None,
    };
  };
  let frontmatter = FileHelper::read_skill_frontmatter(&skill_md).unwrap_or_default();
  let state = match frontmatter.name.as_deref() {
    Some(fm_name) if fm_name != record.name => HubState::NameMismatch,
    _ if hash != record.hash => HubState::Modified,
    _ => HubState::Ok,
  };
  HubProbe {
    state,
    hash: Some(hash),
    description: frontmatter.description,
  }
}

/// Three-way comparison of one install target of the hub copy at `hub_dir`.
pub fn target_state(
  hub_dir: &Path,
  install: &InstallRecord,
  hub_hash: Option<&str>,
) -> (TargetState, Option<String>) {
  let path = Path::new(&install.path);

  if is_symlink(path) {
    if symlink_points_to(path, hub_dir) {
      return if hub_dir.is_dir() {
        (TargetState::InSync, hub_hash.map(str::to_string))
      } else {
        (TargetState::BrokenLink, None)
      };
    }
    if !path.is_dir() {
      return (TargetState::BrokenLink, None);
    }
  } else if !path.is_dir() {
    return (TargetState::Missing, None);
  }

  let Ok(current) = hash_dir_cached(path) else {
    return (TargetState::Missing, None);
  };
  let state = compare_three_way(&current, hub_hash, &install.hash);
  (state, Some(current))
}

pub fn compare_three_way(target: &str, hub: Option<&str>, base: &str) -> TargetState {
  match hub {
    Some(hub) if hub == target => TargetState::InSync,
    Some(hub) => {
      if target == base {
        TargetState::Outdated
      } else if hub == base {
        TargetState::Modified
      } else {
        TargetState::Conflict
      }
    },
    None => {
      if target == base {
        TargetState::InSync
      } else {
        TargetState::Modified
      }
    },
  }
}

pub fn source_state(record: &SkillRecord) -> SourceState {
  match &record.source {
    SkillSource::Github {
      remote_sha,
      latest_remote_sha,
      ..
    } => match (remote_sha, latest_remote_sha) {
      (Some(current), Some(latest)) => {
        if current == latest {
          SourceState::InSync
        } else {
          SourceState::UpdateAvailable
        }
      },
      _ => SourceState::Unchecked,
    },
    SkillSource::Folder { path } => {
      let dir = Path::new(path);
      if !dir.is_dir() {
        return SourceState::SourceMissing;
      }
      match hash_dir_cached(dir) {
        Ok(hash) if hash == record.hash => SourceState::InSync,
        Ok(_) => SourceState::SourceModified,
        Err(_) => SourceState::SourceMissing,
      }
    },
    SkillSource::Zip { .. } | SkillSource::None => SourceState::NotCheckable,
  }
}

pub fn build_install_view(
  env: &Env,
  hub_dir: &Path,
  install: &InstallRecord,
  hub_hash: Option<&str>,
) -> InstallView {
  let (state, current_hash) = target_state(hub_dir, install, hub_hash);
  let project_missing = install.scope == InstallScope::Project
    && !install
      .project_path
      .as_deref()
      .map(|p| Path::new(p).is_dir())
      .unwrap_or(false);
  let missing_agent_ids = install
    .agent_ids
    .iter()
    .filter(|id| env.agent(id).is_none())
    .cloned()
    .collect();

  // Show every agent that reads the directory, not only the ones named at install time.
  let mut record = install.clone();
  let mut agent_ids = agents_reading(env, install);
  for id in &install.agent_ids {
    if !agent_ids.contains(id) {
      agent_ids.push(id.clone());
    }
  }
  if !agent_ids.is_empty() {
    record.agent_ids = agent_ids;
  }

  InstallView {
    record,
    state,
    current_hash,
    project_missing,
    missing_agent_ids,
    link_target: symlink_target_string(Path::new(&install.path)),
  }
}

pub fn build_view(env: &Env, id: &str, record: &SkillRecord) -> HubSkillView {
  let hub_dir = env.hub_dir(id, &record.name);
  let probe = probe_hub(env, id, record);
  let source_state = source_state(record);
  let installs: Vec<InstallView> = record
    .installs
    .iter()
    .map(|install| build_install_view(env, &hub_dir, install, probe.hash.as_deref()))
    .collect();

  let has_drift = probe.state != HubState::Ok
    || matches!(
      source_state,
      SourceState::UpdateAvailable | SourceState::SourceModified
    )
    || installs
      .iter()
      .any(|install| install.state != TargetState::InSync);

  HubSkillView {
    id: id.to_string(),
    name: record.name.clone(),
    hub_path: hub_dir.to_string_lossy().to_string(),
    description: probe.description,
    source: record.source.clone(),
    hash: record.hash.clone(),
    hub_hash: probe.hash,
    hub_state: probe.state,
    source_state,
    imported_at: record.imported_at.clone(),
    updated_at: record.updated_at.clone(),
    installs,
    has_drift,
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::models::InstallMode;
  use crate::utils::hash::hash_dir;
  use std::fs;

  fn write(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
  }

  const ID: &str = "00000000-0000-4000-8000-000000000001";

  fn record(hash: &str, install: Option<InstallRecord>) -> SkillRecord {
    SkillRecord {
      name: "foo".to_string(),
      source: SkillSource::None,
      hash: hash.to_string(),
      imported_at: "t".to_string(),
      updated_at: "t".to_string(),
      installs: install.into_iter().collect(),
    }
  }

  fn install(path: &Path, base: &str, mode: InstallMode) -> InstallRecord {
    InstallRecord {
      scope: InstallScope::User,
      project_path: None,
      path: path.to_string_lossy().to_string(),
      mode,
      hash: base.to_string(),
      installed_at: "t".to_string(),
      agent_ids: vec![],
    }
  }

  #[test]
  fn three_way_states() {
    assert_eq!(compare_three_way("a", Some("a"), "a"), TargetState::InSync);
    assert_eq!(
      compare_three_way("a", Some("b"), "a"),
      TargetState::Outdated
    );
    assert_eq!(
      compare_three_way("b", Some("a"), "a"),
      TargetState::Modified
    );
    assert_eq!(
      compare_three_way("b", Some("c"), "a"),
      TargetState::Conflict
    );
    assert_eq!(compare_three_way("a", None, "a"), TargetState::InSync);
    assert_eq!(compare_three_way("b", None, "a"), TargetState::Modified);
  }

  #[test]
  fn hub_states() {
    let tmp = tempfile::tempdir().unwrap();
    let env = Env::for_test(tmp.path(), vec![], vec![]);
    let hub = env.hub_dir(ID, "foo");

    assert_eq!(
      probe_hub(&env, ID, &record("x", None)).state,
      HubState::Missing
    );

    fs::create_dir_all(&hub).unwrap();
    assert_eq!(
      probe_hub(&env, ID, &record("x", None)).state,
      HubState::Invalid
    );

    write(&hub, "SKILL.md", "---\nname: foo\ndescription: d\n---\n");
    let hash = hash_dir(&hub).unwrap();
    let probe = probe_hub(&env, ID, &record(&hash, None));
    assert_eq!(probe.state, HubState::Ok);
    assert_eq!(probe.description.as_deref(), Some("d"));
    assert_eq!(
      probe_hub(&env, ID, &record("stale", None)).state,
      HubState::Modified
    );

    write(&hub, "SKILL.md", "---\nname: other\n---\n");
    assert_eq!(
      probe_hub(&env, ID, &record("stale", None)).state,
      HubState::NameMismatch
    );
  }

  #[test]
  fn target_states_for_copies_and_links() {
    let tmp = tempfile::tempdir().unwrap();
    let env = Env::for_test(tmp.path(), vec![], vec![]);
    let hub = env.hub_dir(ID, "foo");
    write(&hub, "SKILL.md", "v1");
    let base = hash_dir(&hub).unwrap();

    let target = tmp.path().join("target");
    write(&target, "SKILL.md", "v1");
    let inst = install(&target, &base, InstallMode::Copy);
    assert_eq!(
      target_state(&hub, &inst, Some(&base)).0,
      TargetState::InSync
    );

    write(&hub, "SKILL.md", "v2");
    let hub_v2 = hash_dir(&hub).unwrap();
    assert_eq!(
      target_state(&hub, &inst, Some(&hub_v2)).0,
      TargetState::Outdated
    );

    write(&target, "SKILL.md", "edited");
    assert_eq!(
      target_state(&hub, &inst, Some(&hub_v2)).0,
      TargetState::Conflict
    );
    assert_eq!(
      target_state(&hub, &inst, Some(&base)).0,
      TargetState::Modified
    );

    let missing = install(&tmp.path().join("nope"), &base, InstallMode::Copy);
    assert_eq!(
      target_state(&hub, &missing, Some(&base)).0,
      TargetState::Missing
    );

    #[cfg(unix)]
    {
      let link = tmp.path().join("link");
      std::os::unix::fs::symlink(&hub, &link).unwrap();
      let linked = install(&link, &base, InstallMode::Symlink);
      assert_eq!(
        target_state(&hub, &linked, Some(&hub_v2)).0,
        TargetState::InSync
      );

      let dangling = tmp.path().join("dangling");
      std::os::unix::fs::symlink(tmp.path().join("gone"), &dangling).unwrap();
      let broken = install(&dangling, &base, InstallMode::Symlink);
      assert_eq!(
        target_state(&hub, &broken, Some(&hub_v2)).0,
        TargetState::BrokenLink
      );
    }
  }

  #[test]
  fn folder_source_states() {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("src");
    write(&src, "SKILL.md", "v1");
    let hash = hash_dir(&src).unwrap();
    let mut rec = record(&hash, None);
    rec.source = SkillSource::Folder {
      path: src.to_string_lossy().to_string(),
    };
    assert_eq!(source_state(&rec), SourceState::InSync);
    write(&src, "SKILL.md", "v2");
    assert_eq!(source_state(&rec), SourceState::SourceModified);
    fs::remove_dir_all(&src).unwrap();
    assert_eq!(source_state(&rec), SourceState::SourceMissing);
  }

  #[test]
  fn github_source_states() {
    let mut rec = record("h", None);
    rec.source = SkillSource::Github {
      repo: "o/r".to_string(),
      url: "u".to_string(),
      skill_path: "SKILL.md".to_string(),
      branch: None,
      remote_sha: Some("a".to_string()),
      marketplace_id: None,
      latest_remote_sha: None,
      checked_at: None,
    };
    assert_eq!(source_state(&rec), SourceState::Unchecked);
    if let SkillSource::Github {
      latest_remote_sha, ..
    } = &mut rec.source
    {
      *latest_remote_sha = Some("a".to_string());
    }
    assert_eq!(source_state(&rec), SourceState::InSync);
    if let SkillSource::Github {
      latest_remote_sha, ..
    } = &mut rec.source
    {
      *latest_remote_sha = Some("b".to_string());
    }
    assert_eq!(source_state(&rec), SourceState::UpdateAvailable);
  }
}
