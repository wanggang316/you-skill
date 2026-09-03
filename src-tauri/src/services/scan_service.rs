//! Scan a folder for skills and reconcile them with the hub.

use crate::models::{
  ImportItem, ImportOutcome, ScanDecision, ScanItem, ScanResolution, ScanStatus, SkillSource,
};
use crate::services::env::Env;
use crate::services::hub_service::{import_skills, pull_from_dir};
use crate::services::install_service::{classify_agent_root, register_existing_install};
use crate::services::lock_service::{ops_guard, store};
use crate::services::migration_service::{read_legacy_github_lock, read_legacy_project_lock};
use crate::utils::folder::{find_skill_dirs, read_skill_name, replace_dir_atomic, CopyOpts};
use crate::utils::hash::{hash_dir_cached, invalidate_hash_cache};
use crate::utils::path::{
  is_symlink, is_within, normalize_dir_path, path_to_string, same_path, symlink_points_to,
  validate_skill_name,
};
use std::path::Path;

pub const DEFAULT_SCAN_DEPTH: usize = 6;

pub fn scan_folder(env: &Env, path: &str, max_depth: usize) -> Result<Vec<ScanItem>, String> {
  let root = normalize_dir_path(path, &env.home)?;
  let dirs = find_skill_dirs(&root, max_depth.max(1))?;
  let lock = store(env).read()?;
  let mut items = Vec::new();

  for dir in dirs {
    let Some(name) = read_skill_name(&dir) else {
      continue;
    };
    let path_text = path_to_string(&dir);
    if is_within(&dir, &env.hub_root) {
      items.push(ScanItem {
        name,
        path: path_text,
        hash: None,
        status: ScanStatus::Hub,
        hub_hash: None,
        in_agent_root: None,
        source_hint: None,
        error: None,
      });
      continue;
    }
    if let Err(err) = validate_skill_name(&name) {
      items.push(ScanItem {
        name,
        path: path_text,
        hash: None,
        status: ScanStatus::InvalidName,
        hub_hash: None,
        in_agent_root: None,
        source_hint: None,
        error: Some(err),
      });
      continue;
    }

    let hub_dir = env.hub_dir(&name);
    let in_agent_root = classify_agent_root(env, &dir);
    let source_hint = legacy_source_hint(env, &dir, &name);
    let record = lock.skills.get(&name);
    let hub_hash = record.and_then(|_| hash_dir_cached(&hub_dir).ok());

    if is_symlink(&dir) && symlink_points_to(&dir, &hub_dir) {
      items.push(ScanItem {
        name,
        path: path_text,
        hash: hub_hash.clone(),
        status: ScanStatus::Linked,
        hub_hash,
        in_agent_root,
        source_hint,
        error: None,
      });
      continue;
    }

    let (hash, error) = match hash_dir_cached(&dir) {
      Ok(hash) => (Some(hash), None),
      Err(err) => (None, Some(err)),
    };
    let status = match (record, &hash, &hub_hash) {
      (None, _, _) => ScanStatus::New,
      (Some(_), Some(hash), Some(hub_hash)) if hash == hub_hash => ScanStatus::Identical,
      (Some(_), _, _) => ScanStatus::Different,
    };
    items.push(ScanItem {
      name,
      path: path_text,
      hash,
      status,
      hub_hash,
      in_agent_root,
      source_hint,
      error,
    });
  }

  Ok(items)
}

/// GitHub provenance recorded by the previous lock formats next to a scanned skill:
/// `<parent>/.skill-lock.json` (user-level) or `<project>/skills-lock.json`.
pub fn legacy_source_hint(env: &Env, skill_dir: &Path, name: &str) -> Option<SkillSource> {
  let parent = skill_dir.parent()?;
  let dir_name = skill_dir.file_name()?.to_string_lossy().to_string();

  if let Some(lock) = read_legacy_github_lock(&parent.join(".skill-lock.json")) {
    let entry = lock.skills.get(name).or_else(|| lock.skills.get(&dir_name));
    if let Some(entry) = entry {
      return Some(entry.to_source());
    }
  }

  let project_root = classify_agent_root(env, skill_dir)
    .and_then(|m| m.project_path)
    .or_else(|| parent.parent().map(path_to_string))?;
  let lock = read_legacy_project_lock(&Path::new(&project_root).join("skills-lock.json"))?;
  let entry = lock
    .skills
    .get(name)
    .or_else(|| lock.skills.get(&dir_name))?;
  entry.to_source()
}

pub fn import_scanned(
  env: &Env,
  decisions: Vec<ScanDecision>,
) -> Result<Vec<ImportOutcome>, String> {
  let mut outcomes = Vec::new();
  let mut errors = Vec::new();

  for decision in decisions {
    match import_one_decision(env, &decision) {
      Ok(Some(outcome)) => outcomes.push(outcome),
      Ok(None) => {},
      Err(err) => errors.push(format!("{}: {}", decision.name, err)),
    }
  }

  if !errors.is_empty() && outcomes.is_empty() {
    return Err(errors.join("\n"));
  }
  if !errors.is_empty() {
    tracing::warn!("scan import finished with errors: {}", errors.join("; "));
  }
  Ok(outcomes)
}

fn import_one_decision(
  env: &Env,
  decision: &ScanDecision,
) -> Result<Option<ImportOutcome>, String> {
  let name = decision.name.trim().to_string();
  let dir = normalize_dir_path(&decision.path, &env.home)?;
  let in_agent_root = classify_agent_root(env, &dir);
  let exists = store(env).get(&name)?.is_some();

  let resolution = match decision.resolution {
    ScanResolution::Skip => return Ok(None),
    ScanResolution::Import if exists => ScanResolution::AdoptIntoHub,
    other => other,
  };

  let outcome = match resolution {
    ScanResolution::Skip => unreachable!(),
    ScanResolution::Import => {
      let source = decision.source.clone().unwrap_or(SkillSource::Folder {
        path: path_to_string(&dir),
      });
      let mut outcomes = import_skills(
        env,
        vec![ImportItem {
          name: name.clone(),
          tmp_path: path_to_string(&dir),
          source,
        }],
        false,
      )?;
      outcomes.pop()
    },
    ScanResolution::AdoptIntoHub => {
      let result = pull_from_dir(env, &name, &dir, decision.source.clone(), true)?;
      let hash = result
        .skill
        .as_ref()
        .map(|s| s.hash.clone())
        .unwrap_or_default();
      Some(ImportOutcome {
        name: name.clone(),
        hub_path: path_to_string(&env.hub_dir(&name)),
        hash,
        replaced: true,
      })
    },
    ScanResolution::PushFromHub => {
      if !exists {
        return Err("Skill is not in the hub".to_string());
      }
      let hub_dir = env.hub_dir(&name);
      if !hub_dir.is_dir() {
        return Err("Hub copy is missing".to_string());
      }
      let _ops = ops_guard();
      if same_path(&dir, &hub_dir) {
        return Ok(None);
      }
      replace_dir_atomic(&hub_dir, &dir, &CopyOpts::skill(), &[".git"])?;
      invalidate_hash_cache(&dir);
      let hash = hash_dir_cached(&hub_dir)?;
      Some(ImportOutcome {
        name: name.clone(),
        hub_path: path_to_string(&hub_dir),
        hash,
        replaced: false,
      })
    },
  };

  if let Some(matched) = in_agent_root {
    if decision.register_install && store(env).get(&name)?.is_some() {
      let _ops = ops_guard();
      register_existing_install(
        env,
        &name,
        &dir,
        matched.scope,
        matched.project_path.clone(),
        &matched.agent_ids,
      )?;
    }
  }

  Ok(outcome)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::models::{AgentApp, InstallScope, TargetState};
  use crate::services::hub_service::get_hub_skill;
  use std::fs;

  fn write(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
  }

  fn env(tmp: &Path) -> Env {
    Env::for_test(
      tmp,
      vec![AgentApp {
        id: "claude-code".to_string(),
        display_name: "Claude Code".to_string(),
        project_path: Some(".claude/skills".to_string()),
        global_path: Some("~/.claude/skills".to_string()),
        is_user_custom: false,
      }],
      vec![],
    )
  }

  #[test]
  fn scan_reports_new_identical_different_and_nested_pruned() {
    let tmp = tempfile::tempdir().unwrap();
    let env = env(tmp.path());
    let scan_root = tmp.path().join("scan");
    write(&scan_root, "alpha/SKILL.md", "---\nname: alpha\n---\nA");
    write(
      &scan_root,
      "alpha/nested/SKILL.md",
      "---\nname: nested\n---\n",
    );
    write(&scan_root, "beta/SKILL.md", "---\nname: beta\n---\nB");
    write(&scan_root, "gamma/SKILL.md", "---\nname: gamma\n---\nG");

    // beta identical in hub, gamma differs.
    write(&env.hub_dir("beta"), "SKILL.md", "---\nname: beta\n---\nB");
    write(
      &env.hub_dir("gamma"),
      "SKILL.md",
      "---\nname: gamma\n---\nG2",
    );
    crate::services::hub_service::list_hub_skills(&env).unwrap();

    let items = scan_folder(&env, &scan_root.to_string_lossy(), DEFAULT_SCAN_DEPTH).unwrap();
    let by_name = |n: &str| items.iter().find(|i| i.name == n).unwrap().clone();
    assert_eq!(items.len(), 3);
    assert_eq!(by_name("alpha").status, ScanStatus::New);
    assert_eq!(by_name("beta").status, ScanStatus::Identical);
    assert_eq!(by_name("gamma").status, ScanStatus::Different);
    assert!(by_name("alpha").in_agent_root.is_none());
  }

  #[test]
  fn scan_inside_agent_root_and_import_registers_install() {
    let tmp = tempfile::tempdir().unwrap();
    let env = env(tmp.path());
    let claude = env.home.join(".claude/skills");
    write(&claude, "foo/SKILL.md", "---\nname: foo\n---\nF");

    let items = scan_folder(&env, &claude.to_string_lossy(), DEFAULT_SCAN_DEPTH).unwrap();
    assert_eq!(items.len(), 1);
    let matched = items[0].in_agent_root.clone().unwrap();
    assert_eq!(matched.scope, InstallScope::User);
    assert_eq!(matched.agent_ids, vec!["claude-code"]);

    let outcomes = import_scanned(
      &env,
      vec![ScanDecision {
        name: "foo".to_string(),
        path: items[0].path.clone(),
        resolution: ScanResolution::Import,
        register_install: true,
        source: None,
      }],
    )
    .unwrap();
    assert_eq!(outcomes.len(), 1);
    let view = get_hub_skill(&env, "foo").unwrap().unwrap();
    assert_eq!(view.source, SkillSource::None);
    assert_eq!(view.installs.len(), 1);
    assert_eq!(view.installs[0].state, TargetState::InSync);

    // Edit the agent copy, then adopt it into the hub via scan.
    write(&claude, "foo/SKILL.md", "---\nname: foo\n---\nF2");
    let items = scan_folder(&env, &claude.to_string_lossy(), DEFAULT_SCAN_DEPTH).unwrap();
    assert_eq!(items[0].status, ScanStatus::Different);
    import_scanned(
      &env,
      vec![ScanDecision {
        name: "foo".to_string(),
        path: items[0].path.clone(),
        resolution: ScanResolution::AdoptIntoHub,
        register_install: true,
        source: None,
      }],
    )
    .unwrap();
    assert!(fs::read_to_string(env.hub_dir("foo").join("SKILL.md"))
      .unwrap()
      .ends_with("F2"));
    let view = get_hub_skill(&env, "foo").unwrap().unwrap();
    assert_eq!(view.installs[0].state, TargetState::InSync);

    // Push from hub overwrites the agent copy.
    write(&env.hub_dir("foo"), "SKILL.md", "---\nname: foo\n---\nF3");
    crate::services::hub_service::accept_hub(&env, "foo").unwrap();
    import_scanned(
      &env,
      vec![ScanDecision {
        name: "foo".to_string(),
        path: items[0].path.clone(),
        resolution: ScanResolution::PushFromHub,
        register_install: true,
        source: None,
      }],
    )
    .unwrap();
    assert!(fs::read_to_string(claude.join("foo/SKILL.md"))
      .unwrap()
      .ends_with("F3"));
  }

  #[test]
  fn legacy_lock_provides_source_hint() {
    let tmp = tempfile::tempdir().unwrap();
    let env = env(tmp.path());
    let root = tmp.path().join("legacy");
    write(&root, "foo/SKILL.md", "---\nname: foo\n---\n");
    write(
      &root,
      ".skill-lock.json",
      r#"{"version":3,"skills":{"foo":{"source":"o/r","sourceType":"github","sourceUrl":"https://github.com/o/r.git","skillPath":"skills/foo/SKILL.md","skillFolderHash":"abc","installedAt":"t","updatedAt":"t"}}}"#,
    );
    let items = scan_folder(&env, &root.to_string_lossy(), DEFAULT_SCAN_DEPTH).unwrap();
    match items[0].source_hint.clone().unwrap() {
      SkillSource::Github {
        repo, remote_sha, ..
      } => {
        assert_eq!(repo, "o/r");
        assert_eq!(remote_sha.as_deref(), Some("abc"));
      },
      other => panic!("unexpected {other:?}"),
    }
  }
}
