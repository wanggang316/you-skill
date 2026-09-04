//! Hub -> target deployment: install (copy or symlink), uninstall, push, adopt.

use crate::models::{
  ActionResult, AgentRootMatch, HubSkillView, HubState, InstallMode, InstallRecord, InstallRequest,
  InstallScope, InstallTargetSpec, SkillRecord, TargetState, UninstallRequest,
};
use crate::services::agent_apps_service::LEGACY_USER_ROOTS;
use crate::services::drift_service::{build_view, probe_hub, target_state};
use crate::services::env::Env;
use crate::services::lock_service::{ops_guard, store};
use crate::utils::folder::{copy_dir, replace_dir_atomic, CopyOpts};
use crate::utils::hash::{hash_dir, hash_dir_cached, invalidate_hash_cache};
use crate::utils::path::{
  expand_home_with, is_symlink, is_within, legacy_agents_root, normalize_dir_path, path_to_string,
  remove_path_any, same_path, symlink_points_to,
};
use crate::utils::time::now_rfc3339;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ResolvedTarget {
  pub scope: InstallScope,
  pub project_path: Option<String>,
  pub path: PathBuf,
  pub agent_id: String,
}

pub fn resolve_target(
  env: &Env,
  name: &str,
  spec: &InstallTargetSpec,
) -> Result<ResolvedTarget, String> {
  let app = env
    .agent(&spec.agent_id)
    .ok_or_else(|| format!("Unknown agent app: {}", spec.agent_id))?;
  let project_root = match spec.scope {
    InstallScope::User => None,
    InstallScope::Project => Some(normalize_dir_path(
      spec.project_path.as_deref().unwrap_or_default(),
      &env.home,
    )?),
  };
  let root = env.agent_root(app, spec.scope, project_root.as_deref())?;
  let path = root.join(name);
  // Check the agent root, not the target itself: an existing symlink target would resolve
  // into the hub and be rejected wrongly.
  if is_within(&root, &env.youskill_root) {
    return Err(format!(
      "{} points inside the YouSkill hub directory",
      app.display_name
    ));
  }
  Ok(ResolvedTarget {
    scope: spec.scope,
    project_path: project_root.map(|p| path_to_string(&p)),
    path,
    agent_id: spec.agent_id.clone(),
  })
}

/// Which agent root (if any) a skill directory lives in. Checks user-level roots first,
/// then registered projects, then infers an unregistered project from the path suffix.
pub fn classify_agent_root(env: &Env, skill_dir: &Path) -> Option<AgentRootMatch> {
  let parent = skill_dir.parent()?;

  let mut user_ids: Vec<String> = env
    .agent_apps
    .iter()
    .filter(|app| {
      env
        .agent_root(app, InstallScope::User, None)
        .map(|root| same_path(&root, parent))
        .unwrap_or(false)
    })
    .map(|app| app.id.clone())
    .collect();
  if user_ids.is_empty() {
    for (agent_id, legacy_path) in LEGACY_USER_ROOTS {
      if env.agent(agent_id).is_some()
        && same_path(&expand_home_with(legacy_path, &env.home), parent)
      {
        user_ids.push(agent_id.to_string());
      }
    }
  }
  if !user_ids.is_empty() {
    return Some(AgentRootMatch {
      scope: InstallScope::User,
      project_path: None,
      agent_ids: user_ids,
      registered_project: false,
    });
  }

  for project in &env.projects {
    let project_root = Path::new(&project.path);
    let ids: Vec<String> = env
      .agent_apps
      .iter()
      .filter(|app| {
        env
          .agent_root(app, InstallScope::Project, Some(project_root))
          .map(|root| same_path(&root, parent))
          .unwrap_or(false)
      })
      .map(|app| app.id.clone())
      .collect();
    if !ids.is_empty() {
      return Some(AgentRootMatch {
        scope: InstallScope::Project,
        project_path: Some(project.path.clone()),
        agent_ids: ids,
        registered_project: true,
      });
    }
  }

  let parent_text = parent.to_string_lossy().replace('\\', "/");
  let mut inferred: Option<(String, Vec<String>)> = None;
  for app in &env.agent_apps {
    let Some(project_path) = app.project_path.as_deref() else {
      continue;
    };
    let suffix = format!("/{}", project_path.trim_matches('/'));
    if let Some(root) = parent_text.strip_suffix(&suffix) {
      if root.is_empty() {
        continue;
      }
      match &mut inferred {
        Some((existing_root, ids)) if existing_root == root => ids.push(app.id.clone()),
        Some(_) => {},
        None => inferred = Some((root.to_string(), vec![app.id.clone()])),
      }
    }
  }
  inferred.map(|(root, ids)| AgentRootMatch {
    scope: InstallScope::Project,
    project_path: Some(root),
    agent_ids: ids,
    registered_project: false,
  })
}

fn now() -> String {
  now_rfc3339()
}

fn view_of(env: &Env, name: &str) -> Result<HubSkillView, String> {
  let record = store(env)
    .get(name)?
    .ok_or_else(|| format!("Skill '{}' is not in the hub", name))?;
  Ok(build_view(env, name, &record))
}

/// Write the hub skill to `target` with the requested mode. Returns the effective mode
/// (Windows falls back to a copy when symlinks are not permitted).
pub fn write_target(
  hub_dir: &Path,
  target: &Path,
  mode: InstallMode,
) -> Result<InstallMode, String> {
  if target.exists() || is_symlink(target) {
    remove_path_any(target)?;
  }
  if let Some(parent) = target.parent() {
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
  }
  let effective = match mode {
    InstallMode::Symlink => {
      #[cfg(unix)]
      {
        std::os::unix::fs::symlink(hub_dir, target).map_err(|e| e.to_string())?;
        InstallMode::Symlink
      }
      #[cfg(windows)]
      {
        match std::os::windows::fs::symlink_dir(hub_dir, target) {
          Ok(()) => InstallMode::Symlink,
          Err(_) => {
            copy_dir(hub_dir, target, &CopyOpts::skill())?;
            InstallMode::Copy
          },
        }
      }
    },
    InstallMode::Copy => {
      copy_dir(hub_dir, target, &CopyOpts::skill())?;
      InstallMode::Copy
    },
  };
  invalidate_hash_cache(target);
  Ok(effective)
}

struct TargetGroup {
  scope: InstallScope,
  project_path: Option<String>,
  path: PathBuf,
  agent_ids: Vec<String>,
}

fn group_targets(
  env: &Env,
  name: &str,
  specs: &[InstallTargetSpec],
) -> Result<Vec<TargetGroup>, String> {
  let mut groups: Vec<TargetGroup> = Vec::new();
  for spec in specs {
    let resolved = resolve_target(env, name, spec)?;
    if let Some(group) = groups
      .iter_mut()
      .find(|group| same_path(&group.path, &resolved.path))
    {
      if !group.agent_ids.contains(&resolved.agent_id) {
        group.agent_ids.push(resolved.agent_id);
      }
    } else {
      groups.push(TargetGroup {
        scope: resolved.scope,
        project_path: resolved.project_path,
        path: resolved.path,
        agent_ids: vec![resolved.agent_id],
      });
    }
  }
  Ok(groups)
}

enum Plan {
  /// Files are already right (or only the record needs updating).
  Keep(InstallMode),
  Write(InstallMode),
}

pub fn install_skill(env: &Env, request: InstallRequest) -> Result<ActionResult, String> {
  let _ops = ops_guard();
  let name = request.name.trim().to_string();
  let store = store(env);
  let record = store
    .get(&name)?
    .ok_or_else(|| format!("Skill '{}' is not in the hub", name))?;
  let hub_dir = env.hub_dir(&name);
  if !hub_dir.is_dir() {
    return Err(format!(
      "Hub copy is missing: {}",
      hub_dir.to_string_lossy()
    ));
  }
  if request.targets.is_empty() {
    return Err("At least one install target is required".to_string());
  }
  let requested_mode = request.mode.unwrap_or_default();
  let hub_hash = hash_dir_cached(&hub_dir)?;
  let groups = group_targets(env, &name, &request.targets)?;

  let mut blockers = Vec::new();
  let mut plans: Vec<(usize, Plan)> = Vec::new();
  for (index, group) in groups.iter().enumerate() {
    let existing = record
      .installs
      .iter()
      .find(|install| same_path(Path::new(&install.path), &group.path));
    let plan = if let Some(existing) = existing {
      let (state, _) = target_state(env, &name, existing, Some(&hub_hash));
      let mode = request.mode.unwrap_or(existing.mode);
      if mode != existing.mode && !request.force {
        blockers.push(format!(
          "{} is already installed as {:?}; use force to switch to {:?}",
          existing.path, existing.mode, mode
        ));
        continue;
      }
      if mode != existing.mode {
        let has_local_changes = matches!(state, TargetState::Modified | TargetState::Conflict);
        if has_local_changes && !request.force {
          blockers.push(format!("{} has local changes", existing.path));
          continue;
        }
        Plan::Write(mode)
      } else {
        match state {
          TargetState::Missing | TargetState::BrokenLink => Plan::Write(mode),
          _ => Plan::Keep(mode),
        }
      }
    } else {
      match classify_new_target(
        env,
        &name,
        &group.path,
        &hub_hash,
        requested_mode,
        request.force,
      ) {
        Ok(plan) => plan,
        Err(blocker) => {
          blockers.push(blocker);
          continue;
        },
      }
    };
    plans.push((index, plan));
  }

  if !blockers.is_empty() {
    return Ok(ActionResult::blocked(
      blockers,
      Some(build_view(env, &name, &record)),
    ));
  }

  let mut errors = Vec::new();
  let mut effective_modes: BTreeMap<usize, InstallMode> = BTreeMap::new();
  for (index, plan) in plans {
    let group = &groups[index];
    match plan {
      Plan::Keep(mode) => {
        effective_modes.insert(index, mode);
      },
      Plan::Write(mode) => match write_target(&hub_dir, &group.path, mode) {
        Ok(effective) => {
          effective_modes.insert(index, effective);
        },
        Err(err) => errors.push(format!("{}: {}", group.path.to_string_lossy(), err)),
      },
    }
  }

  let timestamp = now();
  store.update(|lock| {
    let record = lock
      .skills
      .get_mut(&name)
      .ok_or_else(|| format!("Skill '{}' is not in the hub", name))?;
    for (index, mode) in &effective_modes {
      let group = &groups[*index];
      upsert_install(record, group, *mode, &hub_hash, &timestamp);
    }
    Ok(())
  })?;

  if !errors.is_empty() {
    return Err(errors.join("\n"));
  }
  Ok(ActionResult::applied(view_of(env, &name)?))
}

fn upsert_install(
  record: &mut SkillRecord,
  group: &TargetGroup,
  mode: InstallMode,
  hub_hash: &str,
  timestamp: &str,
) {
  let path_text = path_to_string(&group.path);
  if let Some(existing) = record
    .installs
    .iter_mut()
    .find(|install| same_path(Path::new(&install.path), &group.path))
  {
    existing.mode = mode;
    existing.hash = hub_hash.to_string();
    for id in &group.agent_ids {
      if !existing.agent_ids.contains(id) {
        existing.agent_ids.push(id.clone());
      }
    }
    return;
  }
  record.installs.push(InstallRecord {
    scope: group.scope,
    project_path: group.project_path.clone(),
    path: path_text,
    mode,
    hash: hub_hash.to_string(),
    installed_at: timestamp.to_string(),
    agent_ids: group.agent_ids.clone(),
  });
}

/// Decide what to do with a target path that has no install record yet. `Err` carries a
/// blocker message (only returned when `force` is false).
fn classify_new_target(
  env: &Env,
  name: &str,
  path: &Path,
  hub_hash: &str,
  mode: InstallMode,
  force: bool,
) -> Result<Plan, String> {
  let hub_dir = env.hub_dir(name);
  if !path.exists() && !is_symlink(path) {
    return Ok(Plan::Write(mode));
  }
  if is_symlink(path) {
    if symlink_points_to(path, &hub_dir) {
      return Ok(if mode == InstallMode::Symlink {
        Plan::Keep(InstallMode::Symlink)
      } else {
        Plan::Write(mode)
      });
    }
    let legacy = legacy_agents_root(&env.home).join(name);
    if symlink_points_to(path, &legacy) || !path.exists() || force {
      return Ok(Plan::Write(mode));
    }
    return Err(format!(
      "{} is a symlink to another location",
      path.to_string_lossy()
    ));
  }
  if !path.is_dir() {
    return Err(format!(
      "{} exists and is not a directory",
      path.to_string_lossy()
    ));
  }
  let current = hash_dir(path).unwrap_or_default();
  if current == hub_hash {
    return Ok(if mode == InstallMode::Copy {
      Plan::Keep(InstallMode::Copy)
    } else {
      Plan::Write(mode)
    });
  }
  if force {
    return Ok(Plan::Write(mode));
  }
  Err(format!(
    "{} already exists with different content; adopt it into the hub or use force to overwrite",
    path.to_string_lossy()
  ))
}

pub fn uninstall_skill(env: &Env, request: UninstallRequest) -> Result<ActionResult, String> {
  let _ops = ops_guard();
  let name = request.name.trim().to_string();
  let store = store(env);
  let record = store
    .get(&name)?
    .ok_or_else(|| format!("Skill '{}' is not in the hub", name))?;
  let hub_hash = hash_dir_cached(&env.hub_dir(&name)).ok();

  // Map each spec to the install record that owns that agent id (by scope + project).
  let mut removals: Vec<(String, Vec<String>)> = Vec::new(); // (install.path, agent ids)
  for spec in &request.targets {
    // A deleted project directory must still allow its records to be cleaned up.
    let project_root = spec
      .project_path
      .as_deref()
      .map(|p| normalize_dir_path(p, &env.home).unwrap_or_else(|_| PathBuf::from(p.trim())));
    for install in &record.installs {
      if install.scope != spec.scope {
        continue;
      }
      if spec.scope == InstallScope::Project {
        let same_project = match (&install.project_path, &project_root) {
          (Some(a), Some(b)) => same_path(Path::new(a), b),
          _ => false,
        };
        if !same_project {
          continue;
        }
      }
      if !install.agent_ids.contains(&spec.agent_id) {
        continue;
      }
      match removals.iter_mut().find(|(path, _)| path == &install.path) {
        Some((_, ids)) => {
          if !ids.contains(&spec.agent_id) {
            ids.push(spec.agent_id.clone());
          }
        },
        None => removals.push((install.path.clone(), vec![spec.agent_id.clone()])),
      }
    }
  }

  let mut blockers = Vec::new();
  let mut files_to_remove: Vec<PathBuf> = Vec::new();
  for (path, ids) in &removals {
    let Some(install) = record.installs.iter().find(|i| &i.path == path) else {
      continue;
    };
    let remaining: Vec<&String> = install
      .agent_ids
      .iter()
      .filter(|id| !ids.contains(id))
      .collect();
    if !remaining.is_empty() {
      continue;
    }
    let target = Path::new(path);
    if !target.exists() && !is_symlink(target) {
      continue;
    }
    if is_symlink(target) {
      files_to_remove.push(target.to_path_buf());
      continue;
    }
    let (state, _) = target_state(env, &name, install, hub_hash.as_deref());
    match state {
      TargetState::Modified | TargetState::Conflict if !request.force => {
        blockers.push(format!("{} has local changes", path));
      },
      _ => files_to_remove.push(target.to_path_buf()),
    }
  }

  if !blockers.is_empty() {
    return Ok(ActionResult::blocked(
      blockers,
      Some(build_view(env, &name, &record)),
    ));
  }

  let mut errors = Vec::new();
  for path in &files_to_remove {
    if let Err(err) = remove_path_any(path) {
      errors.push(format!("{}: {}", path.to_string_lossy(), err));
    } else {
      invalidate_hash_cache(path);
    }
  }

  store.update(|lock| {
    let record = lock
      .skills
      .get_mut(&name)
      .ok_or_else(|| format!("Skill '{}' is not in the hub", name))?;
    for (path, ids) in &removals {
      if let Some(install) = record.installs.iter_mut().find(|i| &i.path == path) {
        install.agent_ids.retain(|id| !ids.contains(id));
      }
    }
    record
      .installs
      .retain(|install| !install.agent_ids.is_empty());
    Ok(())
  })?;

  if !errors.is_empty() {
    return Err(errors.join("\n"));
  }
  Ok(ActionResult::applied(view_of(env, &name)?))
}

/// Push the hub copy to copy-mode targets. Symlink targets are skipped. Without `force`,
/// targets with local edits (Modified / Conflict) are left alone and reported as blockers.
pub fn push_targets(
  env: &Env,
  name: &str,
  only: Option<&[String]>,
  force: bool,
) -> Result<ActionResult, String> {
  let store = store(env);
  let record = store
    .get(name)?
    .ok_or_else(|| format!("Skill '{}' is not in the hub", name))?;
  let hub_dir = env.hub_dir(name);
  if !hub_dir.is_dir() {
    return Err(format!(
      "Hub copy is missing: {}",
      hub_dir.to_string_lossy()
    ));
  }
  let hub_hash = hash_dir_cached(&hub_dir)?;

  let mut blockers = Vec::new();
  let mut errors = Vec::new();
  let mut pushed: Vec<String> = Vec::new();
  for install in &record.installs {
    if let Some(only) = only {
      if !only
        .iter()
        .any(|p| same_path(Path::new(p), Path::new(&install.path)))
      {
        continue;
      }
    }
    let target = Path::new(&install.path);
    if is_symlink(target) {
      if symlink_points_to(target, &hub_dir) {
        pushed.push(install.path.clone());
      }
      continue;
    }
    let (state, _) = target_state(env, name, install, Some(&hub_hash));
    match state {
      TargetState::InSync => {
        pushed.push(install.path.clone());
      },
      TargetState::Outdated | TargetState::Missing => {
        match replace_dir_atomic(&hub_dir, target, &CopyOpts::skill(), &[]) {
          Ok(()) => {
            invalidate_hash_cache(target);
            pushed.push(install.path.clone());
          },
          Err(err) => errors.push(format!("{}: {}", install.path, err)),
        }
      },
      TargetState::Modified | TargetState::Conflict => {
        if force {
          match replace_dir_atomic(&hub_dir, target, &CopyOpts::skill(), &[]) {
            Ok(()) => {
              invalidate_hash_cache(target);
              pushed.push(install.path.clone());
            },
            Err(err) => errors.push(format!("{}: {}", install.path, err)),
          }
        } else {
          blockers.push(format!("{} has local changes", install.path));
        }
      },
      TargetState::BrokenLink => {
        errors.push(format!("{} is a broken symlink", install.path));
      },
    }
  }

  if !pushed.is_empty() {
    store.update(|lock| {
      let record = lock
        .skills
        .get_mut(name)
        .ok_or_else(|| format!("Skill '{}' is not in the hub", name))?;
      for install in &mut record.installs {
        if pushed.contains(&install.path) {
          install.hash = hub_hash.clone();
        }
      }
      Ok(())
    })?;
  }

  if !errors.is_empty() {
    return Err(errors.join("\n"));
  }
  let view = view_of(env, name)?;
  if blockers.is_empty() {
    Ok(ActionResult::applied(view))
  } else {
    Ok(ActionResult::blocked(blockers, Some(view)))
  }
}

/// Take the content of one copy target as the new hub version.
pub fn adopt_target(
  env: &Env,
  name: &str,
  path: &str,
  force: bool,
) -> Result<ActionResult, String> {
  let store = store(env);
  let record = store
    .get(name)?
    .ok_or_else(|| format!("Skill '{}' is not in the hub", name))?;
  let install = record
    .installs
    .iter()
    .find(|install| same_path(Path::new(&install.path), Path::new(path)))
    .ok_or_else(|| format!("{} is not an install target of '{}'", path, name))?;
  let target = Path::new(&install.path);
  if is_symlink(target) {
    return Err("Symlink targets already mirror the hub".to_string());
  }
  if !target.is_dir() {
    return Err(format!("{} does not exist", install.path));
  }

  let probe = probe_hub(env, name, &record);
  let mut blockers = Vec::new();
  if matches!(probe.state, HubState::Modified | HubState::NameMismatch) && !force {
    blockers.push(format!("Hub copy of '{}' has local changes", name));
  }
  if !blockers.is_empty() {
    return Ok(ActionResult::blocked(
      blockers,
      Some(build_view(env, name, &record)),
    ));
  }

  let hub_dir = env.hub_dir(name);
  if hub_dir.exists() {
    super::hub_service::trash_dir(env, &hub_dir)?;
  }
  copy_dir(target, &hub_dir, &CopyOpts::skill())?;
  invalidate_hash_cache(&hub_dir);
  let new_hash = hash_dir(&hub_dir)?;
  let timestamp = now();
  let adopted_path = install.path.clone();
  store.update(|lock| {
    let record = lock
      .skills
      .get_mut(name)
      .ok_or_else(|| format!("Skill '{}' is not in the hub", name))?;
    record.hash = new_hash.clone();
    record.updated_at = timestamp.clone();
    if let Some(install) = record
      .installs
      .iter_mut()
      .find(|install| install.path == adopted_path)
    {
      install.hash = new_hash.clone();
    }
    Ok(())
  })?;
  Ok(ActionResult::applied(view_of(env, name)?))
}

/// Record an already-present directory (found by scan or migration) as an install target
/// without touching its files. Symlinks into the hub are recorded as `symlink`, everything
/// else as `copy` with its current content hash as the merge base.
pub fn register_existing_install(
  env: &Env,
  name: &str,
  path: &Path,
  scope: InstallScope,
  project_path: Option<String>,
  agent_ids: &[String],
) -> Result<(), String> {
  let hub_dir = env.hub_dir(name);
  // The hub hash is the merge base: a differing copy shows up as `Modified` (local edits
  // relative to the hub) rather than `Outdated`.
  let hash = hash_dir_cached(&hub_dir)?;
  let mode = if symlink_points_to(path, &hub_dir) {
    InstallMode::Symlink
  } else {
    InstallMode::Copy
  };
  let timestamp = now();
  let path_text = path_to_string(path);
  store(env).update(|lock| {
    let record = lock
      .skills
      .get_mut(name)
      .ok_or_else(|| format!("Skill '{}' is not in the hub", name))?;
    if let Some(install) = record
      .installs
      .iter_mut()
      .find(|install| same_path(Path::new(&install.path), path))
    {
      install.mode = mode;
      install.hash = hash.clone();
      for id in agent_ids {
        if !install.agent_ids.contains(id) {
          install.agent_ids.push(id.clone());
        }
      }
    } else {
      record.installs.push(InstallRecord {
        scope,
        project_path: project_path.clone(),
        path: path_text.clone(),
        mode,
        hash: hash.clone(),
        installed_at: timestamp.clone(),
        agent_ids: agent_ids.to_vec(),
      });
    }
    Ok(())
  })
}

/// Replace a symlink target with a real copy of the hub content (used before the hub copy
/// is removed so agents keep working).
pub fn materialize_symlink(
  env: &Env,
  name: &str,
  install: &InstallRecord,
) -> Result<InstallMode, String> {
  let target = Path::new(&install.path);
  if !is_symlink(target) {
    return Ok(install.mode);
  }
  let hub_dir = env.hub_dir(name);
  if !hub_dir.is_dir() {
    return Err(format!(
      "Hub copy is missing: {}",
      hub_dir.to_string_lossy()
    ));
  }
  write_target(&hub_dir, target, InstallMode::Copy)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::models::{AgentApp, SkillSource, UserProject};
  use crate::services::lock_service::LockStore;
  use std::fs;

  fn write(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
  }

  fn app(id: &str, global: &str, project: &str) -> AgentApp {
    AgentApp {
      id: id.to_string(),
      display_name: id.to_string(),
      project_path: Some(project.to_string()),
      global_path: Some(global.to_string()),
      detect_path: None,
      profile_path: Some("AGENTS.md".to_string()),
      global_profile_path: None,
      is_user_custom: false,
    }
  }

  fn env_with_hub(tmp: &Path, projects: Vec<UserProject>) -> Env {
    let env = Env::for_test(
      tmp,
      vec![
        app("claude-code", "~/.claude/skills", ".claude/skills"),
        app("codex", "~/.codex/skills", ".agents/skills"),
        app("cursor", "~/.cursor/skills", ".agents/skills"),
      ],
      projects,
    );
    let hub = env.hub_dir("foo");
    write(&hub, "SKILL.md", "---\nname: foo\n---\nv1");
    let hash = hash_dir(&hub).unwrap();
    LockStore::new(env.lock_path.clone())
      .update(|lock| {
        lock.skills.insert(
          "foo".to_string(),
          SkillRecord {
            source: SkillSource::None,
            hash,
            imported_at: "t".to_string(),
            updated_at: "t".to_string(),
            installs: vec![],
          },
        );
        Ok(())
      })
      .unwrap();
    env
  }

  fn spec(scope: InstallScope, project: Option<&Path>, agent: &str) -> InstallTargetSpec {
    InstallTargetSpec {
      scope,
      project_path: project.map(|p| p.to_string_lossy().to_string()),
      agent_id: agent.to_string(),
    }
  }

  #[test]
  fn copy_install_then_edit_is_modified_and_uninstall_needs_force() {
    let tmp = tempfile::tempdir().unwrap();
    let env = env_with_hub(tmp.path(), vec![]);
    let result = install_skill(
      &env,
      InstallRequest {
        name: "foo".to_string(),
        targets: vec![spec(InstallScope::User, None, "claude-code")],
        mode: Some(InstallMode::Copy),
        force: false,
      },
    )
    .unwrap();
    assert!(result.applied);
    let target = env.home.join(".claude/skills/foo");
    assert!(target.join("SKILL.md").is_file());
    let view = result.skill.unwrap();
    assert_eq!(view.installs.len(), 1);
    assert_eq!(view.installs[0].state, TargetState::InSync);
    assert_eq!(view.installs[0].record.mode, InstallMode::Copy);

    write(&target, "SKILL.md", "edited");
    let view = view_of(&env, "foo").unwrap();
    assert_eq!(view.installs[0].state, TargetState::Modified);
    assert!(view.has_drift);

    let blocked = uninstall_skill(
      &env,
      UninstallRequest {
        name: "foo".to_string(),
        targets: vec![spec(InstallScope::User, None, "claude-code")],
        force: false,
      },
    )
    .unwrap();
    assert!(!blocked.applied);
    assert!(target.exists());

    let forced = uninstall_skill(
      &env,
      UninstallRequest {
        name: "foo".to_string(),
        targets: vec![spec(InstallScope::User, None, "claude-code")],
        force: true,
      },
    )
    .unwrap();
    assert!(forced.applied);
    assert!(!target.exists());
    assert!(view_of(&env, "foo").unwrap().installs.is_empty());
  }

  #[cfg(unix)]
  #[test]
  fn shared_project_path_reuses_one_record_and_symlink_stays_in_sync() {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("proj");
    fs::create_dir_all(&project).unwrap();
    let env = env_with_hub(
      tmp.path(),
      vec![UserProject {
        name: "proj".to_string(),
        path: project.to_string_lossy().to_string(),
        workspace_path: None,
      }],
    );
    let result = install_skill(
      &env,
      InstallRequest {
        name: "foo".to_string(),
        targets: vec![
          spec(InstallScope::Project, Some(&project), "codex"),
          spec(InstallScope::Project, Some(&project), "cursor"),
        ],
        mode: Some(InstallMode::Symlink),
        force: false,
      },
    )
    .unwrap();
    assert!(result.applied);
    let view = result.skill.unwrap();
    assert_eq!(view.installs.len(), 1);
    assert_eq!(view.installs[0].record.agent_ids, vec!["codex", "cursor"]);
    assert_eq!(view.installs[0].record.mode, InstallMode::Symlink);
    assert!(is_symlink(&project.join(".agents/skills/foo")));

    // Hub edits are visible through the link without a push.
    write(&env.hub_dir("foo"), "SKILL.md", "v2");
    let view = view_of(&env, "foo").unwrap();
    assert_eq!(view.installs[0].state, TargetState::InSync);
    assert_eq!(view.hub_state, crate::models::HubState::Modified);

    // Requesting a different mode for the same path is blocked without force.
    let blocked = install_skill(
      &env,
      InstallRequest {
        name: "foo".to_string(),
        targets: vec![spec(InstallScope::Project, Some(&project), "codex")],
        mode: Some(InstallMode::Copy),
        force: false,
      },
    )
    .unwrap();
    assert!(!blocked.applied);

    // Removing one agent keeps the files; removing the last one unlinks.
    uninstall_skill(
      &env,
      UninstallRequest {
        name: "foo".to_string(),
        targets: vec![spec(InstallScope::Project, Some(&project), "codex")],
        force: false,
      },
    )
    .unwrap();
    assert!(is_symlink(&project.join(".agents/skills/foo")));
    uninstall_skill(
      &env,
      UninstallRequest {
        name: "foo".to_string(),
        targets: vec![spec(InstallScope::Project, Some(&project), "cursor")],
        force: false,
      },
    )
    .unwrap();
    assert!(!is_symlink(&project.join(".agents/skills/foo")));
  }

  #[test]
  fn install_onto_foreign_dir_is_blocked_then_forced() {
    let tmp = tempfile::tempdir().unwrap();
    let env = env_with_hub(tmp.path(), vec![]);
    let target = env.home.join(".claude/skills/foo");
    write(&target, "SKILL.md", "someone else's copy");
    let request = InstallRequest {
      name: "foo".to_string(),
      targets: vec![spec(InstallScope::User, None, "claude-code")],
      mode: Some(InstallMode::Copy),
      force: false,
    };
    let blocked = install_skill(&env, request.clone()).unwrap();
    assert!(!blocked.applied);
    assert_eq!(
      fs::read_to_string(target.join("SKILL.md")).unwrap(),
      "someone else's copy"
    );
    let forced = install_skill(
      &env,
      InstallRequest {
        force: true,
        ..request
      },
    )
    .unwrap();
    assert!(forced.applied);
    assert!(fs::read_to_string(target.join("SKILL.md"))
      .unwrap()
      .ends_with("v1"));
  }

  #[test]
  fn push_and_adopt_round_trip() {
    let tmp = tempfile::tempdir().unwrap();
    let env = env_with_hub(tmp.path(), vec![]);
    install_skill(
      &env,
      InstallRequest {
        name: "foo".to_string(),
        targets: vec![spec(InstallScope::User, None, "claude-code")],
        mode: Some(InstallMode::Copy),
        force: false,
      },
    )
    .unwrap();
    let target = env.home.join(".claude/skills/foo");

    // Hub moves on -> target Outdated -> push.
    write(&env.hub_dir("foo"), "SKILL.md", "v2");
    let view = view_of(&env, "foo").unwrap();
    assert_eq!(view.installs[0].state, TargetState::Outdated);
    let pushed = push_targets(&env, "foo", None, false).unwrap();
    assert!(pushed.applied);
    assert_eq!(fs::read_to_string(target.join("SKILL.md")).unwrap(), "v2");
    assert_eq!(
      view_of(&env, "foo").unwrap().installs[0].state,
      TargetState::InSync
    );

    // Target edited while hub unchanged -> Modified -> adopt is blocked because hub itself
    // is still Modified relative to the record; accept the hub first, then adopt.
    write(&target, "SKILL.md", "v3");
    let blocked = adopt_target(&env, "foo", &target.to_string_lossy(), false).unwrap();
    assert!(!blocked.applied);
    super::super::hub_service::accept_hub(&env, "foo").unwrap();
    let adopted = adopt_target(&env, "foo", &target.to_string_lossy(), false).unwrap();
    assert!(adopted.applied);
    assert_eq!(
      fs::read_to_string(env.hub_dir("foo").join("SKILL.md")).unwrap(),
      "v3"
    );
    let view = view_of(&env, "foo").unwrap();
    assert_eq!(view.hub_state, crate::models::HubState::Ok);
    assert_eq!(view.installs[0].state, TargetState::InSync);
    assert!(env.trash_root.is_dir());
  }

  #[test]
  fn target_inside_hub_is_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let mut env = env_with_hub(tmp.path(), vec![]);
    env.agent_apps.push(app("evil", "~/.youskill/skills", ".x"));
    let err = install_skill(
      &env,
      InstallRequest {
        name: "foo".to_string(),
        targets: vec![spec(InstallScope::User, None, "evil")],
        mode: None,
        force: false,
      },
    );
    assert!(err.is_err());
  }

  #[test]
  fn classify_agent_root_matches_user_project_and_inferred() {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("proj");
    fs::create_dir_all(project.join(".agents/skills/x")).unwrap();
    let env = env_with_hub(
      tmp.path(),
      vec![UserProject {
        name: "proj".to_string(),
        path: project.to_string_lossy().to_string(),
        workspace_path: None,
      }],
    );
    let user_dir = env.home.join(".claude/skills/x");
    fs::create_dir_all(&user_dir).unwrap();
    let m = classify_agent_root(&env, &user_dir).unwrap();
    assert_eq!(m.scope, InstallScope::User);
    assert_eq!(m.agent_ids, vec!["claude-code"]);

    let m = classify_agent_root(&env, &project.join(".agents/skills/x")).unwrap();
    assert_eq!(m.scope, InstallScope::Project);
    assert!(m.registered_project);
    assert_eq!(m.agent_ids, vec!["codex", "cursor"]);

    let other = tmp.path().join("other/.claude/skills/x");
    fs::create_dir_all(&other).unwrap();
    let m = classify_agent_root(&env, &other).unwrap();
    assert!(!m.registered_project);
    assert_eq!(m.agent_ids, vec!["claude-code"]);
    assert!(m.project_path.unwrap().ends_with("other"));

    assert!(classify_agent_root(&env, &tmp.path().join("random/x")).is_none());

    // App-specific directory used before the shared ~/.agents/skills.
    let legacy = env.home.join(".cursor/skills/x");
    fs::create_dir_all(&legacy).unwrap();
    let m = classify_agent_root(&env, &legacy).unwrap();
    assert_eq!(m.scope, InstallScope::User);
    assert_eq!(m.agent_ids, vec!["cursor"]);
  }
}
