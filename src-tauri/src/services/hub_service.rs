//! The central skill repository (`~/.youskill/skills`) and its lock file.

use crate::models::{
  ActionResult, HubSkillView, HubState, ImportItem, ImportOutcome, InstallMode, InstallRecord,
  InstallScope, SkillRecord, SkillSource, SyncAction,
};
use crate::services::drift_service::{build_view, probe_hub};
use crate::services::env::Env;
use crate::services::install_service::{
  adopt_target, classify_agent_root, materialize_symlink, push_targets,
};
use crate::services::lock_service::{ops_guard, store};
use crate::utils::folder::{
  copy_dir, is_staged_temp_path, move_dir, replace_dir_atomic, CopyOpts, SKILL_MD,
};
use crate::utils::hash::{hash_dir, hash_dir_cached, invalidate_hash_cache};
use crate::utils::path::{
  is_symlink, is_within, path_to_string, remove_path_any, validate_skill_name,
};
use crate::utils::time::{now_file_stamp, now_rfc3339};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

const TRASH_MAX_AGE: Duration = Duration::from_secs(7 * 24 * 60 * 60);

pub fn list_hub_skills(env: &Env) -> Result<Vec<HubSkillView>, String> {
  adopt_untracked_hub_dirs(env)?;
  let lock = store(env).read()?;
  Ok(
    lock
      .skills
      .iter()
      .map(|(name, record)| build_view(env, name, record))
      .collect(),
  )
}

pub fn get_hub_skill(env: &Env, name: &str) -> Result<Option<HubSkillView>, String> {
  Ok(
    store(env)
      .get(name)?
      .map(|record| build_view(env, name, &record)),
  )
}

/// Directories dropped into the hub by hand become tracked records with no source.
fn adopt_untracked_hub_dirs(env: &Env) -> Result<(), String> {
  if !env.hub_root.is_dir() {
    return Ok(());
  }
  let store = store(env);
  let known = store.read()?.skills;
  let mut untracked: Vec<(String, String)> = Vec::new();
  for entry in fs::read_dir(&env.hub_root).map_err(|e| e.to_string())? {
    let entry = entry.map_err(|e| e.to_string())?;
    let dir = entry.path();
    if !dir.is_dir() || !dir.join(SKILL_MD).is_file() {
      continue;
    }
    let name = entry.file_name().to_string_lossy().to_string();
    if known.contains_key(&name) || validate_skill_name(&name).is_err() {
      continue;
    }
    if let Ok(hash) = hash_dir_cached(&dir) {
      untracked.push((name, hash));
    }
  }
  if untracked.is_empty() {
    return Ok(());
  }
  let _ops = ops_guard();
  let timestamp = now_rfc3339();
  store.update(|lock| {
    for (name, hash) in untracked {
      lock.skills.entry(name).or_insert(SkillRecord {
        source: SkillSource::None,
        hash,
        imported_at: timestamp.clone(),
        updated_at: timestamp.clone(),
        installs: Vec::new(),
      });
    }
    Ok(())
  })
}

/// Move a directory into `~/.youskill/.trash/<name>-<stamp>` so a replacement can be undone.
pub fn trash_dir(env: &Env, dir: &Path) -> Result<PathBuf, String> {
  let name = dir
    .file_name()
    .map(|n| n.to_string_lossy().to_string())
    .unwrap_or_else(|| "skill".to_string());
  fs::create_dir_all(&env.trash_root).map_err(|e| e.to_string())?;
  let mut dest = env
    .trash_root
    .join(format!("{}-{}", name, now_file_stamp()));
  let mut counter = 1;
  while dest.exists() {
    dest = env
      .trash_root
      .join(format!("{}-{}-{}", name, now_file_stamp(), counter));
    counter += 1;
  }
  if is_symlink(dir) {
    remove_path_any(dir)?;
    return Ok(dest);
  }
  move_dir(dir, &dest)?;
  invalidate_hash_cache(dir);
  Ok(dest)
}

pub fn sweep_trash(env: &Env) {
  let Ok(entries) = fs::read_dir(&env.trash_root) else {
    return;
  };
  let now = SystemTime::now();
  for entry in entries.flatten() {
    let Ok(meta) = entry.metadata() else {
      continue;
    };
    let Ok(modified) = meta.modified() else {
      continue;
    };
    if now.duration_since(modified).unwrap_or_default() > TRASH_MAX_AGE {
      let _ = remove_path_any(&entry.path());
    }
  }
}

/// Copy staged skill directories into the hub and record them. With `overwrite`, an
/// existing skill of the same name is replaced (its installs are kept and unmodified copy
/// targets are pushed afterwards).
pub fn import_skills(
  env: &Env,
  items: Vec<ImportItem>,
  overwrite: bool,
) -> Result<Vec<ImportOutcome>, String> {
  let _ops = ops_guard();
  let mut outcomes = Vec::new();
  let mut errors = Vec::new();
  let mut replaced_names = Vec::new();

  for item in items {
    match import_one(env, &item, overwrite) {
      Ok(outcome) => {
        if outcome.replaced {
          replaced_names.push(outcome.name.clone());
        }
        outcomes.push(outcome);
      },
      Err(err) => errors.push(format!("{}: {}", item.name, err)),
    }
  }

  for name in replaced_names {
    if let Err(err) = push_targets(env, &name, None, false) {
      tracing::warn!("push after replacing '{}' failed: {}", name, err);
    }
  }

  if !errors.is_empty() {
    if outcomes.is_empty() {
      return Err(errors.join("\n"));
    }
    tracing::warn!("import finished with errors: {}", errors.join("; "));
  }
  Ok(outcomes)
}

fn import_one(env: &Env, item: &ImportItem, overwrite: bool) -> Result<ImportOutcome, String> {
  let name = item.name.trim().to_string();
  validate_skill_name(&name)?;
  let src = Path::new(&item.tmp_path);
  if !src.is_dir() || !src.join(SKILL_MD).is_file() {
    return Err(format!(
      "Staged skill directory is invalid: {}",
      src.to_string_lossy()
    ));
  }

  let store = store(env);
  let lock = store.read()?;
  let existing_key = lock
    .skills
    .keys()
    .find(|key| key.eq_ignore_ascii_case(&name))
    .cloned();
  if let Some(key) = &existing_key {
    if key != &name {
      return Err(format!(
        "A skill named '{}' already exists (names are case-insensitive)",
        key
      ));
    }
    if !overwrite {
      return Err("Skill already exists in the hub".to_string());
    }
  }

  // A folder that is itself an agent target is not a source: register it as an install.
  let mut source = item.source.clone();
  let mut auto_install: Option<(InstallScope, Option<String>, String, Vec<String>)> = None;
  if let SkillSource::Folder { path } = &source {
    let folder = Path::new(path);
    if is_within(folder, &env.youskill_root) {
      source = SkillSource::None;
    } else if let Some(matched) = classify_agent_root(env, folder) {
      auto_install = Some((
        matched.scope,
        matched.project_path,
        path_to_string(folder),
        matched.agent_ids,
      ));
      source = SkillSource::None;
    }
  }

  let hub_dir = env.hub_dir(&name);
  if hub_dir.exists() || is_symlink(&hub_dir) {
    trash_dir(env, &hub_dir)?;
  }
  fs::create_dir_all(&env.hub_root).map_err(|e| e.to_string())?;
  copy_dir(src, &hub_dir, &CopyOpts::skill())?;
  invalidate_hash_cache(&hub_dir);
  let hash = hash_dir(&hub_dir)?;
  if is_staged_temp_path(src) {
    let _ = remove_path_any(src);
  }

  let timestamp = now_rfc3339();
  let replaced = existing_key.is_some();
  store.update(|lock| {
    let record = lock.skills.entry(name.clone()).or_insert(SkillRecord {
      source: SkillSource::None,
      hash: hash.clone(),
      imported_at: timestamp.clone(),
      updated_at: timestamp.clone(),
      installs: Vec::new(),
    });
    record.source = source.clone();
    record.hash = hash.clone();
    record.updated_at = timestamp.clone();
    if let Some((scope, project_path, path, agent_ids)) = &auto_install {
      if let Some(install) = record.installs.iter_mut().find(|i| &i.path == path) {
        install.hash = hash.clone();
        for id in agent_ids {
          if !install.agent_ids.contains(id) {
            install.agent_ids.push(id.clone());
          }
        }
      } else {
        record.installs.push(InstallRecord {
          scope: *scope,
          project_path: project_path.clone(),
          path: path.clone(),
          mode: InstallMode::Copy,
          hash: hash.clone(),
          installed_at: timestamp.clone(),
          agent_ids: agent_ids.clone(),
        });
      }
    }
    Ok(())
  })?;

  Ok(ImportOutcome {
    name,
    hub_path: path_to_string(&hub_dir),
    hash,
    replaced,
  })
}

/// Replace the hub copy with fresh content from the source (already downloaded / located
/// at `src_dir`). Refuses without `force` when the hub has local edits.
pub fn pull_from_dir(
  env: &Env,
  name: &str,
  src_dir: &Path,
  updated_source: Option<SkillSource>,
  force: bool,
) -> Result<ActionResult, String> {
  let _ops = ops_guard();
  let store = store(env);
  let record = store
    .get(name)?
    .ok_or_else(|| format!("Skill '{}' is not in the hub", name))?;
  if !src_dir.is_dir() || !src_dir.join(SKILL_MD).is_file() {
    return Err(format!(
      "Source directory is invalid: {}",
      src_dir.to_string_lossy()
    ));
  }
  let probe = probe_hub(env, name, &record);
  if matches!(probe.state, HubState::Modified | HubState::NameMismatch) && !force {
    return Ok(ActionResult::blocked(
      vec![format!("Hub copy of '{}' has local changes", name)],
      Some(build_view(env, name, &record)),
    ));
  }

  let hub_dir = env.hub_dir(name);
  if hub_dir.exists() || is_symlink(&hub_dir) {
    trash_dir(env, &hub_dir)?;
  }
  copy_dir(src_dir, &hub_dir, &CopyOpts::skill())?;
  invalidate_hash_cache(&hub_dir);
  let hash = hash_dir(&hub_dir)?;
  if is_staged_temp_path(src_dir) {
    let _ = remove_path_any(src_dir);
  }
  let timestamp = now_rfc3339();
  store.update(|lock| {
    let record = lock
      .skills
      .get_mut(name)
      .ok_or_else(|| format!("Skill '{}' is not in the hub", name))?;
    record.hash = hash.clone();
    record.updated_at = timestamp.clone();
    if let Some(source) = &updated_source {
      record.source = source.clone();
    }
    Ok(())
  })?;
  drop(store);
  push_targets(env, name, None, false)
}

/// Mirror the hub copy back into a `folder` source.
pub fn push_source(env: &Env, name: &str, force: bool) -> Result<ActionResult, String> {
  let _ops = ops_guard();
  let store = store(env);
  let record = store
    .get(name)?
    .ok_or_else(|| format!("Skill '{}' is not in the hub", name))?;
  let SkillSource::Folder { path } = &record.source else {
    return Err("Only folder sources can be pushed to".to_string());
  };
  let source_dir = Path::new(path);
  if !source_dir.is_dir() {
    return Err(format!("Source folder is missing: {}", path));
  }
  let hub_dir = env.hub_dir(name);
  if !hub_dir.is_dir() {
    return Err(format!(
      "Hub copy is missing: {}",
      hub_dir.to_string_lossy()
    ));
  }
  let source_hash = hash_dir(source_dir)?;
  if source_hash != record.hash && !force {
    return Ok(ActionResult::blocked(
      vec![format!("Source folder {} has changes not in the hub", path)],
      Some(build_view(env, name, &record)),
    ));
  }
  replace_dir_atomic(&hub_dir, source_dir, &CopyOpts::skill(), &[".git"])?;
  invalidate_hash_cache(source_dir);
  Ok(ActionResult::applied(build_view(env, name, &record)))
}

/// Accept whatever is in the hub directory as the current version.
pub fn accept_hub(env: &Env, name: &str) -> Result<ActionResult, String> {
  let _ops = ops_guard();
  let store = store(env);
  let hub_dir = env.hub_dir(name);
  if !hub_dir.is_dir() {
    return Err(format!(
      "Hub copy is missing: {}",
      hub_dir.to_string_lossy()
    ));
  }
  invalidate_hash_cache(&hub_dir);
  let hash = hash_dir(&hub_dir)?;
  let timestamp = now_rfc3339();
  let record = store.update(|lock| {
    let record = lock
      .skills
      .get_mut(name)
      .ok_or_else(|| format!("Skill '{}' is not in the hub", name))?;
    record.hash = hash.clone();
    record.updated_at = timestamp.clone();
    Ok(record.clone())
  })?;
  Ok(ActionResult::applied(build_view(env, name, &record)))
}

/// Remove a skill from the hub. With `remove_installs = false`, symlink targets are turned
/// into real copies first so agents keep working.
pub fn remove_hub_skill(env: &Env, name: &str, remove_installs: bool) -> Result<(), String> {
  let _ops = ops_guard();
  let store = store(env);
  let record = store
    .get(name)?
    .ok_or_else(|| format!("Skill '{}' is not in the hub", name))?;

  let mut errors = Vec::new();
  for install in &record.installs {
    let target = Path::new(&install.path);
    if remove_installs {
      if target.exists() || is_symlink(target) {
        if let Err(err) = remove_path_any(target) {
          errors.push(format!("{}: {}", install.path, err));
        } else {
          invalidate_hash_cache(target);
        }
      }
    } else if let Err(err) = materialize_symlink(env, name, install) {
      errors.push(format!("{}: {}", install.path, err));
    }
  }

  let hub_dir = env.hub_dir(name);
  if hub_dir.exists() || is_symlink(&hub_dir) {
    trash_dir(env, &hub_dir)?;
  }
  store.update(|lock| {
    lock.skills.remove(name);
    Ok(())
  })?;
  if !errors.is_empty() {
    return Err(errors.join("\n"));
  }
  Ok(())
}

/// Offline sync actions. `PullSource` for GitHub sources needs a download and is handled by
/// the command layer, which then calls `pull_from_dir`.
pub fn sync_skill(env: &Env, name: &str, action: SyncAction) -> Result<ActionResult, String> {
  match action {
    SyncAction::PullSource { force } => {
      let record = store(env)
        .get(name)?
        .ok_or_else(|| format!("Skill '{}' is not in the hub", name))?;
      match &record.source {
        SkillSource::Folder { path } => pull_from_dir(env, name, Path::new(path), None, force),
        SkillSource::Github { .. } => {
          Err("GitHub sources must be pulled through the async command".to_string())
        },
        SkillSource::Zip { .. } | SkillSource::None => {
          Err("This skill has no source to pull from".to_string())
        },
      }
    },
    SyncAction::PushTargets { targets, force } => {
      let _ops = ops_guard();
      push_targets(env, name, targets.as_deref(), force)
    },
    SyncAction::AdoptTarget { path, force } => {
      let _ops = ops_guard();
      adopt_target(env, name, &path, force)
    },
    SyncAction::AcceptHub => accept_hub(env, name),
    SyncAction::PushSource { force } => push_source(env, name, force),
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::models::{AgentApp, InstallRequest, InstallTargetSpec, TargetState};
  use crate::services::install_service::install_skill;
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

  fn staged(tmp: &Path, name: &str, body: &str) -> ImportItem {
    let dir = tmp.join("staged").join(name);
    write(&dir, "SKILL.md", &format!("---\nname: {name}\n---\n{body}"));
    ImportItem {
      name: name.to_string(),
      tmp_path: dir.to_string_lossy().to_string(),
      source: SkillSource::None,
    }
  }

  #[test]
  fn import_new_conflict_and_overwrite() {
    let tmp = tempfile::tempdir().unwrap();
    let env = env(tmp.path());
    let outcomes = import_skills(&env, vec![staged(tmp.path(), "foo", "v1")], false).unwrap();
    assert_eq!(outcomes.len(), 1);
    assert!(!outcomes[0].replaced);
    assert!(env.hub_dir("foo").join("SKILL.md").is_file());
    let first_hash = outcomes[0].hash.clone();

    // Same name without overwrite -> error; different case -> error.
    assert!(import_skills(&env, vec![staged(tmp.path(), "foo", "v2")], false).is_err());
    assert!(import_skills(&env, vec![staged(tmp.path(), "FOO", "v2")], true).is_err());

    // Install, then overwrite: installs are kept and the unmodified copy is pushed.
    install_skill(
      &env,
      InstallRequest {
        name: "foo".to_string(),
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
    let outcomes = import_skills(&env, vec![staged(tmp.path(), "foo", "v2")], true).unwrap();
    assert!(outcomes[0].replaced);
    assert_ne!(outcomes[0].hash, first_hash);
    let view = get_hub_skill(&env, "foo").unwrap().unwrap();
    assert_eq!(view.installs.len(), 1);
    assert_eq!(view.installs[0].state, TargetState::InSync);
    assert!(
      fs::read_to_string(env.home.join(".claude/skills/foo/SKILL.md"))
        .unwrap()
        .ends_with("v2")
    );
    assert!(env.trash_root.read_dir().unwrap().next().is_some());
  }

  #[test]
  fn import_rejects_invalid_names_and_missing_skill_md() {
    let tmp = tempfile::tempdir().unwrap();
    let env = env(tmp.path());
    let mut item = staged(tmp.path(), "bad", "x");
    item.name = "../bad".to_string();
    assert!(import_skills(&env, vec![item], false).is_err());
    let empty = tmp.path().join("empty");
    fs::create_dir_all(&empty).unwrap();
    let item = ImportItem {
      name: "empty".to_string(),
      tmp_path: empty.to_string_lossy().to_string(),
      source: SkillSource::None,
    };
    assert!(import_skills(&env, vec![item], false).is_err());
  }

  #[test]
  fn folder_inside_agent_root_becomes_install_not_source() {
    let tmp = tempfile::tempdir().unwrap();
    let env = env(tmp.path());
    let dir = env.home.join(".claude/skills/foo");
    write(&dir, "SKILL.md", "---\nname: foo\n---\n");
    let item = ImportItem {
      name: "foo".to_string(),
      tmp_path: dir.to_string_lossy().to_string(),
      source: SkillSource::Folder {
        path: dir.to_string_lossy().to_string(),
      },
    };
    import_skills(&env, vec![item], false).unwrap();
    let view = get_hub_skill(&env, "foo").unwrap().unwrap();
    assert_eq!(view.source, SkillSource::None);
    assert_eq!(view.installs.len(), 1);
    assert_eq!(view.installs[0].record.agent_ids, vec!["claude-code"]);
    assert_eq!(view.installs[0].state, TargetState::InSync);
    assert!(dir.is_dir(), "source dir outside temp must not be removed");
  }

  #[test]
  fn folder_source_pull_and_push() {
    let tmp = tempfile::tempdir().unwrap();
    let env = env(tmp.path());
    let src = tmp.path().join("mysrc");
    write(&src, "SKILL.md", "---\nname: foo\n---\nv1");
    let item = ImportItem {
      name: "foo".to_string(),
      tmp_path: src.to_string_lossy().to_string(),
      source: SkillSource::Folder {
        path: src.to_string_lossy().to_string(),
      },
    };
    import_skills(&env, vec![item], false).unwrap();

    write(&src, "SKILL.md", "---\nname: foo\n---\nv2");
    let view = get_hub_skill(&env, "foo").unwrap().unwrap();
    assert_eq!(
      view.source_state,
      crate::models::SourceState::SourceModified
    );
    let pulled = sync_skill(&env, "foo", SyncAction::PullSource { force: false }).unwrap();
    assert!(pulled.applied);
    assert!(fs::read_to_string(env.hub_dir("foo").join("SKILL.md"))
      .unwrap()
      .ends_with("v2"));

    // Edit hub -> pull is blocked; accept -> push back to source.
    write(&env.hub_dir("foo"), "SKILL.md", "---\nname: foo\n---\nv3");
    write(&src, "SKILL.md", "---\nname: foo\n---\nv2b");
    let blocked = sync_skill(&env, "foo", SyncAction::PullSource { force: false }).unwrap();
    assert!(!blocked.applied);
    accept_hub(&env, "foo").unwrap();
    let blocked = sync_skill(&env, "foo", SyncAction::PushSource { force: false }).unwrap();
    assert!(!blocked.applied, "source changed too");
    let pushed = sync_skill(&env, "foo", SyncAction::PushSource { force: true }).unwrap();
    assert!(pushed.applied);
    assert!(fs::read_to_string(src.join("SKILL.md"))
      .unwrap()
      .ends_with("v3"));
  }

  #[cfg(unix)]
  #[test]
  fn remove_hub_skill_materializes_or_removes_installs() {
    let tmp = tempfile::tempdir().unwrap();
    let env = env(tmp.path());
    import_skills(&env, vec![staged(tmp.path(), "foo", "v1")], false).unwrap();
    install_skill(
      &env,
      InstallRequest {
        name: "foo".to_string(),
        targets: vec![InstallTargetSpec {
          scope: InstallScope::User,
          project_path: None,
          agent_id: "claude-code".to_string(),
        }],
        mode: Some(InstallMode::Symlink),
        force: false,
      },
    )
    .unwrap();
    let target = env.home.join(".claude/skills/foo");
    assert!(is_symlink(&target));

    remove_hub_skill(&env, "foo", false).unwrap();
    assert!(!is_symlink(&target));
    assert!(target.join("SKILL.md").is_file());
    assert!(!env.hub_dir("foo").exists());
    assert!(get_hub_skill(&env, "foo").unwrap().is_none());

    import_skills(&env, vec![staged(tmp.path(), "bar", "v1")], false).unwrap();
    install_skill(
      &env,
      InstallRequest {
        name: "bar".to_string(),
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
    remove_hub_skill(&env, "bar", true).unwrap();
    assert!(!env.home.join(".claude/skills/bar").exists());
  }

  #[test]
  fn untracked_hub_dirs_are_adopted_on_list() {
    let tmp = tempfile::tempdir().unwrap();
    let env = env(tmp.path());
    write(
      &env.hub_dir("manual"),
      "SKILL.md",
      "---\nname: manual\n---\n",
    );
    let list = list_hub_skills(&env).unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].name, "manual");
    assert_eq!(list[0].hub_state, HubState::Ok);
    assert!(!list[0].has_drift);
  }
}
