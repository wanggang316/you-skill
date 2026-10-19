//! Scan a folder for skills and reconcile them with the hub.

use crate::models::{
  ImportItem, ImportOutcome, LockFile, ScanDecision, ScanItem, ScanResolution, ScanStatus,
  SkillRecord, SkillSource,
};
use crate::services::env::Env;
use crate::services::hub_service::{find_skill, import_skills, pull_from_dir};
use crate::services::install_service::{classify_agent_root, register_existing_install};
use crate::services::lock_service::{ops_guard, store};
use crate::services::migration_service::{
  migrate_skill_ids, read_legacy_github_lock, read_legacy_project_lock,
};
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
  migrate_skill_ids(env)?;
  let lock = store(env).read()?;
  let mut items = Vec::new();

  for dir in dirs {
    let Some(name) = read_skill_name(&dir) else {
      continue;
    };
    let path_text = path_to_string(&dir);
    if is_within(&dir, &env.hub_root) {
      items.push(ScanItem {
        id: None,
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
        id: None,
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

    let in_agent_root = classify_agent_root(env, &dir);
    let source_hint = legacy_source_hint(env, &dir, &name);
    let (hash, error) = match hash_dir_cached(&dir) {
      Ok(hash) => (Some(hash), None),
      Err(err) => (None, Some(err)),
    };
    let record = match_record(env, &lock, &dir, &name, hash.as_deref());
    let id = record.map(|(id, _)| id.clone());
    let hub_dir = record.map(|(id, record)| env.hub_dir(id, &record.name));
    let hub_hash = hub_dir
      .as_deref()
      .and_then(|hub_dir| hash_dir_cached(hub_dir).ok());

    if is_symlink(&dir)
      && hub_dir
        .as_deref()
        .is_some_and(|hub_dir| symlink_points_to(&dir, hub_dir))
    {
      items.push(ScanItem {
        id,
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

    let status = match (record, &hash, &hub_hash) {
      (None, _, _) => ScanStatus::New,
      (Some(_), Some(hash), Some(hub_hash)) if hash == hub_hash => ScanStatus::Identical,
      (Some(_), _, _) => ScanStatus::Different,
    };
    items.push(ScanItem {
      id,
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

/// The hub skill a scanned folder belongs to: the skill a symlink points to, the skill that
/// has the folder as an install, the skill with the name and the same content, else the
/// oldest skill with the name.
fn match_record<'a>(
  env: &Env,
  lock: &'a LockFile,
  dir: &Path,
  name: &str,
  hash: Option<&str>,
) -> Option<(&'a String, &'a SkillRecord)> {
  let records = || lock.skills.iter();
  if is_symlink(dir) {
    if let Some(found) =
      records().find(|(id, record)| symlink_points_to(dir, &env.hub_dir(id, &record.name)))
    {
      return Some(found);
    }
  }
  if let Some(found) = records().find(|(_, record)| {
    record
      .installs
      .iter()
      .any(|install| same_path(Path::new(&install.path), dir))
  }) {
    return Some(found);
  }
  if let Some(hash) = hash {
    if let Some(found) = records().find(|(id, record)| {
      record.name.eq_ignore_ascii_case(name)
        && hash_dir_cached(&env.hub_dir(id, &record.name))
          .ok()
          .as_deref()
          == Some(hash)
    }) {
      return Some(found);
    }
  }
  lock.find_by_name(name)
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
  let mut decisions = decisions;
  // A register-only decision reads the hub copy as its merge base, so let whichever
  // decision writes that copy run first.
  decisions.sort_by_key(|decision| !decision.resolution.writes_hub());

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
  let existing = match decision.id.as_deref() {
    Some(id) => find_skill(env, id)?,
    None => store(env)
      .read()?
      .find_by_name(&name)
      .map(|(id, record)| (id.clone(), record.clone())),
  };
  let exists = existing.is_some();
  let mut target_id = existing.as_ref().map(|(id, _)| id.clone());

  let resolution = match decision.resolution {
    ScanResolution::Skip => return Ok(None),
    ScanResolution::Import if exists => ScanResolution::AdoptIntoHub,
    other => other,
  };

  let outcome = match resolution {
    ScanResolution::Skip => unreachable!(),
    ScanResolution::RegisterOnly => {
      if !exists {
        return Err("Skill is not in the hub".to_string());
      }
      if in_agent_root.is_none() {
        return Err("Folder is not an agent skills directory".to_string());
      }
      None
    },
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
          replace_id: None,
        }],
        false,
      )?;
      let outcome = outcomes.pop();
      target_id = outcome.as_ref().map(|outcome| outcome.id.clone());
      outcome
    },
    ScanResolution::AdoptIntoHub => {
      let id = target_id
        .clone()
        .ok_or_else(|| "Skill is not in the hub".to_string())?;
      let result = pull_from_dir(env, &id, &dir, decision.source.clone(), true)?;
      let skill = result
        .skill
        .ok_or_else(|| "Skill is not in the hub".to_string())?;
      Some(ImportOutcome {
        id: skill.id,
        name: skill.name,
        hub_path: skill.hub_path,
        hash: skill.hash,
        replaced: true,
      })
    },
    ScanResolution::PushFromHub => {
      let Some((id, record)) = existing else {
        return Err("Skill is not in the hub".to_string());
      };
      let hub_dir = env.hub_dir(&id, &record.name);
      if !hub_dir.is_dir() {
        return Err("Hub copy is missing".to_string());
      }
      let _ops = ops_guard(env)?;
      if same_path(&dir, &hub_dir) {
        return Ok(None);
      }
      replace_dir_atomic(&hub_dir, &dir, &CopyOpts::skill(), &[".git"])?;
      invalidate_hash_cache(&dir);
      let hash = hash_dir_cached(&hub_dir)?;
      Some(ImportOutcome {
        id,
        name: record.name,
        hub_path: path_to_string(&hub_dir),
        hash,
        replaced: false,
      })
    },
  };

  let register = decision.register_install || resolution == ScanResolution::RegisterOnly;
  if let Some(matched) = in_agent_root {
    if let Some(id) = target_id.filter(|_| register) {
      let _ops = ops_guard(env)?;
      register_existing_install(
        env,
        &id,
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
        detect_path: None,
        profile_path: Some("AGENTS.md".to_string()),
        global_profile_path: None,
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
    write(
      &env.hub_root.join("beta"),
      "SKILL.md",
      "---\nname: beta\n---\nB",
    );
    write(
      &env.hub_root.join("gamma"),
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
        id: None,
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
        id: None,
        name: "foo".to_string(),
        path: items[0].path.clone(),
        resolution: ScanResolution::AdoptIntoHub,
        register_install: true,
        source: None,
      }],
    )
    .unwrap();
    assert!(fs::read_to_string(env.hub_dir_of("foo").join("SKILL.md"))
      .unwrap()
      .ends_with("F2"));
    let view = get_hub_skill(&env, "foo").unwrap().unwrap();
    assert_eq!(view.installs[0].state, TargetState::InSync);

    // Push from hub overwrites the agent copy.
    write(
      &env.hub_dir_of("foo"),
      "SKILL.md",
      "---\nname: foo\n---\nF3",
    );
    crate::services::hub_service::accept_hub(&env, "foo").unwrap();
    import_scanned(
      &env,
      vec![ScanDecision {
        id: None,
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
  fn register_only_adds_a_second_copy_of_a_skill_imported_in_the_same_batch() {
    let tmp = tempfile::tempdir().unwrap();
    let env = env(tmp.path());
    let user_dir = env.home.join(".claude/skills/dup");
    let project_dir = tmp.path().join("proj/.claude/skills/dup");
    write(&user_dir, "SKILL.md", "---\nname: dup\n---\nUSER");
    write(&project_dir, "SKILL.md", "---\nname: dup\n---\nPROJECT");

    let decision = |path: &Path, resolution: ScanResolution| ScanDecision {
      id: None,
      name: "dup".to_string(),
      path: path_to_string(path),
      resolution,
      register_install: false,
      source: None,
    };
    // Register-only comes first on purpose: the batch has to run it after the import, or it
    // has no hub copy to use as a merge base.
    let outcomes = import_scanned(
      &env,
      vec![
        decision(&project_dir, ScanResolution::RegisterOnly),
        decision(&user_dir, ScanResolution::Import),
      ],
    )
    .unwrap();

    assert_eq!(outcomes.len(), 1);
    assert!(fs::read_to_string(env.hub_dir_of("dup").join("SKILL.md"))
      .unwrap()
      .ends_with("USER"));
    let view = get_hub_skill(&env, "dup").unwrap().unwrap();
    assert_eq!(view.installs.len(), 2);
    let install = |path: &Path| {
      view
        .installs
        .iter()
        .find(|install| same_path(Path::new(&install.record.path), path))
        .unwrap()
    };
    assert_eq!(install(&user_dir).state, TargetState::InSync);
    assert_eq!(install(&project_dir).state, TargetState::Modified);
  }

  #[test]
  fn register_only_without_a_hub_copy_fails_without_stopping_the_batch() {
    let tmp = tempfile::tempdir().unwrap();
    let env = env(tmp.path());
    let orphan = env.home.join(".claude/skills/orphan");
    let other = env.home.join(".claude/skills/other");
    write(&orphan, "SKILL.md", "---\nname: orphan\n---\n");
    write(&other, "SKILL.md", "---\nname: other\n---\n");

    let outcomes = import_scanned(
      &env,
      vec![
        ScanDecision {
          id: None,
          name: "orphan".to_string(),
          path: path_to_string(&orphan),
          resolution: ScanResolution::RegisterOnly,
          register_install: false,
          source: None,
        },
        ScanDecision {
          id: None,
          name: "other".to_string(),
          path: path_to_string(&other),
          resolution: ScanResolution::Import,
          register_install: true,
          source: None,
        },
      ],
    )
    .unwrap();

    assert_eq!(outcomes.len(), 1);
    assert_eq!(outcomes[0].name, "other");
    assert!(get_hub_skill(&env, "orphan").unwrap().is_none());
  }

  #[test]
  fn scan_selects_the_same_name_skill_by_install_path_then_content() {
    use crate::models::{InstallMode, InstallRequest, InstallTargetSpec};
    use crate::services::install_service::install_skill;
    let tmp = tempfile::tempdir().unwrap();
    let env = env(tmp.path());
    let staged = |label: &str, repo: &str| {
      let dir = tmp.path().join("staged").join(label).join("tdd");
      write(&dir, "SKILL.md", &format!("---\nname: tdd\n---\n{label}"));
      ImportItem {
        name: "tdd".to_string(),
        tmp_path: path_to_string(&dir),
        source: SkillSource::Folder {
          path: format!("/src/{repo}/tdd"),
        },
        replace_id: None,
      }
    };
    let a = import_skills(&env, vec![staged("A", "a")], false).unwrap()[0]
      .id
      .clone();
    let b = import_skills(&env, vec![staged("B", "b")], false).unwrap()[0]
      .id
      .clone();
    install_skill(
      &env,
      InstallRequest {
        name: b.clone(),
        targets: vec![InstallTargetSpec {
          scope: InstallScope::User,
          project_path: None,
          agent_id: "claude-code".to_string(),
        }],
        mode: Some(InstallMode::Copy),
        force: false,
      },
    )
    .unwrap();

    // An install of `b`, edited: still `b`, by its install path.
    let claude = env.home.join(".claude/skills");
    write(&claude, "tdd/SKILL.md", "---\nname: tdd\n---\nedited");
    let items = scan_folder(&env, &claude.to_string_lossy(), DEFAULT_SCAN_DEPTH).unwrap();
    assert_eq!(items[0].id.as_deref(), Some(b.as_str()));
    assert_eq!(items[0].status, ScanStatus::Different);

    // A loose copy of `b`: `b`, by its content. Another copy: the oldest skill.
    let loose = tmp.path().join("loose");
    write(&loose, "copy/SKILL.md", "---\nname: tdd\n---\nB");
    write(&loose, "other/SKILL.md", "---\nname: tdd\n---\nC");
    let items = scan_folder(&env, &loose.to_string_lossy(), DEFAULT_SCAN_DEPTH).unwrap();
    let by_dir = |dir: &str| {
      items
        .iter()
        .find(|item| item.path.ends_with(dir))
        .unwrap()
        .clone()
    };
    assert_eq!(by_dir("copy").id.as_deref(), Some(b.as_str()));
    assert_eq!(by_dir("copy").status, ScanStatus::Identical);
    assert_eq!(by_dir("other").id.as_deref(), Some(a.as_str()));

    // A decision with the id adopts into that skill.
    import_scanned(
      &env,
      vec![ScanDecision {
        id: Some(b.clone()),
        name: "tdd".to_string(),
        path: by_dir("other").path,
        resolution: ScanResolution::AdoptIntoHub,
        register_install: false,
        source: None,
      }],
    )
    .unwrap();
    assert!(fs::read_to_string(env.hub_dir_of(&b).join("SKILL.md"))
      .unwrap()
      .ends_with("C"));
    assert!(fs::read_to_string(env.hub_dir_of(&a).join("SKILL.md"))
      .unwrap()
      .ends_with("A"));
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
