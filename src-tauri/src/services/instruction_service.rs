//! The instruction library (`~/.youskill/instructions/<name>.md`): agent instruction files
//! kept once and deployed to the `AGENTS.md` / `CLAUDE.md` locations each agent reads.
//! Mirrors the skill hub (import, install, three-way drift, push / adopt, scan) for single
//! files; install records share the skill types.

use crate::models::{
  AgentRootMatch, HubState, InstallMode, InstallRecord, InstallRequest, InstallScope,
  InstallTargetSpec, InstallView, InstructionActionResult, InstructionImportItem,
  InstructionImportOutcome, InstructionLockFile, InstructionRecord, InstructionScanDecision,
  InstructionScanItem, InstructionView, ScanResolution, ScanStatus, SkillDiff, SyncAction,
  TargetState, UninstallRequest, LOCK_VERSION,
};
use crate::services::diff_service::diff_files;
use crate::services::drift_service::compare_three_way;
use crate::services::env::Env;
use crate::services::install_service::ResolvedTarget;
use crate::services::lock_service::ops_guard;
use crate::utils::hash::hash_file;
use crate::utils::path::{
  is_symlink, is_within, normalize_dir_path, path_to_string, remove_path_any, same_path,
  symlink_points_to, validate_instruction_name,
};
use crate::utils::time::{now_file_stamp, now_rfc3339};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

const DESCRIPTION_MAX_CHARS: usize = 160;

// ---------------------------------------------------------------------------
// Lock file
// ---------------------------------------------------------------------------

static WRITE_LOCK: Mutex<()> = Mutex::new(());

pub fn read_lock(env: &Env) -> Result<InstructionLockFile, String> {
  let path = &env.instruction_lock_path;
  if !path.exists() {
    return Ok(InstructionLockFile::default());
  }
  let content = fs::read_to_string(path)
    .map_err(|e| format!("Failed to read {}: {}", path.to_string_lossy(), e))?;
  if content.trim().is_empty() {
    return Ok(InstructionLockFile::default());
  }
  serde_json::from_str(&content)
    .map_err(|e| format!("Failed to parse {}: {}", path.to_string_lossy(), e))
}

fn get_record(env: &Env, name: &str) -> Result<InstructionRecord, String> {
  read_lock(env)?
    .instructions
    .get(name)
    .cloned()
    .ok_or_else(|| format!("Instruction '{}' is not in the library", name))
}

/// Read-modify-write under a process-wide mutex; written through a temp file + rename.
fn update_lock<T>(
  env: &Env,
  f: impl FnOnce(&mut InstructionLockFile) -> Result<T, String>,
) -> Result<T, String> {
  let _guard = WRITE_LOCK
    .lock()
    .unwrap_or_else(|poisoned| poisoned.into_inner());
  let mut lock = read_lock(env)?;
  let result = f(&mut lock)?;
  lock.version = LOCK_VERSION;
  let path = &env.instruction_lock_path;
  if let Some(parent) = path.parent() {
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
  }
  let content = serde_json::to_string_pretty(&lock).map_err(|e| e.to_string())?;
  let tmp = path.with_extension("json.tmp");
  fs::write(&tmp, content).map_err(|e| e.to_string())?;
  fs::rename(&tmp, path).map_err(|e| {
    let _ = fs::remove_file(&tmp);
    e.to_string()
  })?;
  Ok(result)
}

fn record_mut<'a>(
  lock: &'a mut InstructionLockFile,
  name: &str,
) -> Result<&'a mut InstructionRecord, String> {
  lock
    .instructions
    .get_mut(name)
    .ok_or_else(|| format!("Instruction '{}' is not in the library", name))
}

// ---------------------------------------------------------------------------
// Locations
// ---------------------------------------------------------------------------

pub fn resolve_target(env: &Env, spec: &InstallTargetSpec) -> Result<ResolvedTarget, String> {
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
  let path = env.agent_profile(app, spec.scope, project_root.as_deref())?;
  // Check the directory, not the file: a symlink into the library must stay installable.
  if path
    .parent()
    .map(|parent| is_within(parent, &env.youskill_root))
    .unwrap_or(false)
  {
    return Err(format!(
      "{} points inside the YouSkill directory",
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

/// Every installed agent app that reads the file of this install.
pub fn agents_reading(env: &Env, install: &InstallRecord) -> Vec<String> {
  let file = Path::new(&install.path);
  let project_root = install.project_path.as_deref().map(PathBuf::from);
  env
    .agent_apps
    .iter()
    .filter(|app| {
      env
        .agent_profile(app, install.scope, project_root.as_deref())
        .map(|path| same_path(&path, file))
        .unwrap_or(false)
    })
    .map(|app| app.id.clone())
    .collect()
}

/// Which agent instruction location (if any) a file is: user level first, then registered
/// projects, then a project inferred from the path suffix.
pub fn classify_profile(env: &Env, file: &Path) -> Option<AgentRootMatch> {
  let user_ids: Vec<String> = env
    .agent_apps
    .iter()
    .filter(|app| {
      env
        .agent_profile(app, InstallScope::User, None)
        .map(|path| same_path(&path, file))
        .unwrap_or(false)
    })
    .map(|app| app.id.clone())
    .collect();
  if !user_ids.is_empty() {
    return Some(AgentRootMatch {
      scope: InstallScope::User,
      project_path: None,
      agent_ids: user_ids,
      registered_project: false,
    });
  }

  for project in &env.projects {
    let root = Path::new(&project.path);
    let ids: Vec<String> = env
      .agent_apps
      .iter()
      .filter(|app| {
        env
          .agent_profile(app, InstallScope::Project, Some(root))
          .map(|path| same_path(&path, file))
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

  let file_text = file.to_string_lossy().replace('\\', "/");
  let mut inferred: Option<(String, Vec<String>)> = None;
  for app in &env.agent_apps {
    let Some(profile) = app.profile_path.as_deref() else {
      continue;
    };
    let suffix = format!("/{}", profile.trim_matches('/'));
    if let Some(root) = file_text.strip_suffix(&suffix) {
      if root.is_empty() {
        continue;
      }
      match &mut inferred {
        Some((existing, ids)) if existing == root => ids.push(app.id.clone()),
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

// ---------------------------------------------------------------------------
// Drift
// ---------------------------------------------------------------------------

struct HubProbe {
  state: HubState,
  hash: Option<String>,
  description: Option<String>,
}

fn probe_hub(env: &Env, name: &str, record: &InstructionRecord) -> HubProbe {
  let file = env.instruction_file(name);
  if !file.is_file() {
    return HubProbe {
      state: HubState::Missing,
      hash: None,
      description: None,
    };
  }
  let Ok(hash) = hash_file(&file) else {
    return HubProbe {
      state: HubState::Invalid,
      hash: None,
      description: None,
    };
  };
  let state = if hash != record.hash {
    HubState::Modified
  } else {
    HubState::Ok
  };
  HubProbe {
    state,
    hash: Some(hash),
    description: first_line(&file),
  }
}

/// The first line of text after any frontmatter, without heading markers.
fn first_line(file: &Path) -> Option<String> {
  let content = fs::read_to_string(file).ok()?;
  let mut lines = content.lines().map(str::trim).peekable();
  while lines.peek().map(|line| line.is_empty()).unwrap_or(false) {
    lines.next();
  }
  if lines.peek() == Some(&"---") {
    lines.next();
    for line in lines.by_ref() {
      if line == "---" {
        break;
      }
    }
  }
  let line = lines
    .find(|line| !line.is_empty())?
    .trim_start_matches('#')
    .trim();
  if line.is_empty() {
    return None;
  }
  Some(line.chars().take(DESCRIPTION_MAX_CHARS).collect())
}

/// Three-way comparison of one install target, like a skill target but for a file.
pub fn target_state(
  env: &Env,
  name: &str,
  install: &InstallRecord,
  hub_hash: Option<&str>,
) -> (TargetState, Option<String>) {
  let path = Path::new(&install.path);
  let hub_file = env.instruction_file(name);
  if is_symlink(path) {
    if symlink_points_to(path, &hub_file) {
      return if hub_file.is_file() {
        (TargetState::InSync, hub_hash.map(str::to_string))
      } else {
        (TargetState::BrokenLink, None)
      };
    }
    if !path.is_file() {
      return (TargetState::BrokenLink, None);
    }
  } else if !path.is_file() {
    return (TargetState::Missing, None);
  }
  let Ok(current) = hash_file(path) else {
    return (TargetState::Missing, None);
  };
  let state = compare_three_way(&current, hub_hash, &install.hash);
  (state, Some(current))
}

fn build_install_view(
  env: &Env,
  name: &str,
  install: &InstallRecord,
  hub_hash: Option<&str>,
) -> InstallView {
  let (state, current_hash) = target_state(env, name, install, hub_hash);
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
  }
}

pub fn build_view(env: &Env, name: &str, record: &InstructionRecord) -> InstructionView {
  let probe = probe_hub(env, name, record);
  let installs: Vec<InstallView> = record
    .installs
    .iter()
    .map(|install| build_install_view(env, name, install, probe.hash.as_deref()))
    .collect();
  let has_drift = probe.state != HubState::Ok
    || installs
      .iter()
      .any(|install| install.state != TargetState::InSync);
  InstructionView {
    name: name.to_string(),
    hub_path: path_to_string(&env.instruction_file(name)),
    description: probe.description,
    hash: record.hash.clone(),
    hub_hash: probe.hash,
    hub_state: probe.state,
    imported_at: record.imported_at.clone(),
    updated_at: record.updated_at.clone(),
    installs,
    has_drift,
  }
}

fn view_of(env: &Env, name: &str) -> Result<InstructionView, String> {
  Ok(build_view(env, name, &get_record(env, name)?))
}

// ---------------------------------------------------------------------------
// Library
// ---------------------------------------------------------------------------

pub fn list_instructions(env: &Env) -> Result<Vec<InstructionView>, String> {
  adopt_untracked_files(env)?;
  let lock = read_lock(env)?;
  Ok(
    lock
      .instructions
      .iter()
      .map(|(name, record)| build_view(env, name, record))
      .collect(),
  )
}

pub fn get_instruction(env: &Env, name: &str) -> Result<Option<InstructionView>, String> {
  Ok(
    read_lock(env)?
      .instructions
      .get(name)
      .map(|record| build_view(env, name, record)),
  )
}

pub fn read_instruction(env: &Env, name: &str) -> Result<String, String> {
  let file = env.instruction_file(name);
  fs::read_to_string(&file).map_err(|e| format!("Failed to read {}: {}", file.to_string_lossy(), e))
}

/// Markdown files dropped into the library folder by hand become tracked records.
fn adopt_untracked_files(env: &Env) -> Result<(), String> {
  if !env.instructions_root.is_dir() {
    return Ok(());
  }
  let known = read_lock(env)?.instructions;
  let mut untracked: Vec<(String, String)> = Vec::new();
  for entry in fs::read_dir(&env.instructions_root).map_err(|e| e.to_string())? {
    let entry = entry.map_err(|e| e.to_string())?;
    let file = entry.path();
    if !file.is_file() {
      continue;
    }
    let Some(name) = library_name(&file) else {
      continue;
    };
    if known.contains_key(&name) || validate_instruction_name(&name).is_err() {
      continue;
    }
    if let Ok(hash) = hash_file(&file) {
      untracked.push((name, hash));
    }
  }
  if untracked.is_empty() {
    return Ok(());
  }
  let _ops = ops_guard();
  let timestamp = now_rfc3339();
  update_lock(env, |lock| {
    for (name, hash) in untracked {
      lock.instructions.entry(name).or_insert(InstructionRecord {
        hash,
        imported_at: timestamp.clone(),
        updated_at: timestamp.clone(),
        installs: Vec::new(),
      });
    }
    Ok(())
  })
}

/// `<name>` of a `<name>.md` library file.
fn library_name(file: &Path) -> Option<String> {
  let file_name = file.file_name()?.to_string_lossy();
  let name = file_name.strip_suffix(".md")?;
  if name.is_empty() {
    return None;
  }
  Some(name.to_string())
}

/// Move a library file into `~/.youskill/.trash/<file>-<stamp>` so a replacement can be
/// undone.
fn trash_file(env: &Env, file: &Path) -> Result<(), String> {
  if is_symlink(file) {
    return remove_path_any(file);
  }
  let name = file
    .file_name()
    .map(|n| n.to_string_lossy().to_string())
    .unwrap_or_else(|| "instruction.md".to_string());
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
  if fs::rename(file, &dest).is_err() {
    fs::copy(file, &dest).map_err(|e| e.to_string())?;
    fs::remove_file(file).map_err(|e| e.to_string())?;
  }
  Ok(())
}

fn existing_key(lock: &InstructionLockFile, name: &str) -> Option<String> {
  lock
    .instructions
    .keys()
    .find(|key| key.eq_ignore_ascii_case(name))
    .cloned()
}

/// Copy Markdown files into the library and record them. With `overwrite`, an existing
/// entry of the same name is replaced (installs are kept and unmodified copies pushed).
pub fn import_instructions(
  env: &Env,
  items: Vec<InstructionImportItem>,
  overwrite: bool,
) -> Result<Vec<InstructionImportOutcome>, String> {
  let _ops = ops_guard();
  let mut outcomes = Vec::new();
  let mut errors = Vec::new();
  let mut replaced_names = Vec::new();
  for item in items {
    match import_one(
      env,
      &item.name,
      Path::new(item.path.trim()),
      overwrite,
      true,
    ) {
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
  finish_import(outcomes, errors)
}

fn finish_import(
  outcomes: Vec<InstructionImportOutcome>,
  errors: Vec<String>,
) -> Result<Vec<InstructionImportOutcome>, String> {
  if !errors.is_empty() {
    if outcomes.is_empty() {
      return Err(errors.join("\n"));
    }
    tracing::warn!("import finished with errors: {}", errors.join("; "));
  }
  Ok(outcomes)
}

/// Import one file. A file that is itself an agent location is registered as an install
/// (when `register` is set) so the library entry and the file stay linked.
fn import_one(
  env: &Env,
  name: &str,
  src: &Path,
  overwrite: bool,
  register: bool,
) -> Result<InstructionImportOutcome, String> {
  let name = name.trim().to_string();
  validate_instruction_name(&name)?;
  if !src.is_file() {
    return Err(format!("Not a file: {}", src.to_string_lossy()));
  }
  let lock = read_lock(env)?;
  let existing = existing_key(&lock, &name);
  if let Some(key) = &existing {
    if key != &name {
      return Err(format!(
        "An instruction named '{}' already exists (names are case-insensitive)",
        key
      ));
    }
    if !overwrite {
      return Err("Instruction already exists in the library".to_string());
    }
  }
  let auto_install = if register && !is_within(src, &env.youskill_root) {
    classify_profile(env, src)
  } else {
    None
  };

  let hub_file = env.instruction_file(&name);
  if hub_file.exists() || is_symlink(&hub_file) {
    trash_file(env, &hub_file)?;
  }
  fs::create_dir_all(&env.instructions_root).map_err(|e| e.to_string())?;
  fs::copy(src, &hub_file).map_err(|e| e.to_string())?;
  let hash = hash_file(&hub_file)?;
  let timestamp = now_rfc3339();
  let replaced = existing.is_some();
  let src_text = path_to_string(src);
  update_lock(env, |lock| {
    let record = lock
      .instructions
      .entry(name.clone())
      .or_insert(InstructionRecord {
        hash: hash.clone(),
        imported_at: timestamp.clone(),
        updated_at: timestamp.clone(),
        installs: Vec::new(),
      });
    record.hash = hash.clone();
    record.updated_at = timestamp.clone();
    if let Some(matched) = &auto_install {
      upsert_install(
        record,
        matched.scope,
        matched.project_path.clone(),
        &src_text,
        InstallMode::Copy,
        &matched.agent_ids,
        &hash,
        &timestamp,
      );
    }
    Ok(())
  })?;
  Ok(InstructionImportOutcome {
    name,
    hub_path: path_to_string(&hub_file),
    hash,
    replaced,
  })
}

/// Start a new library entry from text.
pub fn create_instruction(
  env: &Env,
  name: &str,
  content: &str,
) -> Result<InstructionImportOutcome, String> {
  let _ops = ops_guard();
  let name = name.trim().to_string();
  validate_instruction_name(&name)?;
  let lock = read_lock(env)?;
  if let Some(key) = existing_key(&lock, &name) {
    return Err(format!("An instruction named '{}' already exists", key));
  }
  let hub_file = env.instruction_file(&name);
  if hub_file.exists() || is_symlink(&hub_file) {
    return Err(format!(
      "A file named {} already exists in the library folder",
      hub_file.to_string_lossy()
    ));
  }
  fs::create_dir_all(&env.instructions_root).map_err(|e| e.to_string())?;
  let body = if content.trim().is_empty() {
    format!("# {}\n", name)
  } else if content.ends_with('\n') {
    content.to_string()
  } else {
    format!("{}\n", content)
  };
  fs::write(&hub_file, body).map_err(|e| e.to_string())?;
  let hash = hash_file(&hub_file)?;
  let timestamp = now_rfc3339();
  update_lock(env, |lock| {
    lock.instructions.insert(
      name.clone(),
      InstructionRecord {
        hash: hash.clone(),
        imported_at: timestamp.clone(),
        updated_at: timestamp.clone(),
        installs: Vec::new(),
      },
    );
    Ok(())
  })?;
  Ok(InstructionImportOutcome {
    name,
    hub_path: path_to_string(&hub_file),
    hash,
    replaced: false,
  })
}

/// Accept whatever is in the library file as the current version.
pub fn accept_hub(env: &Env, name: &str) -> Result<InstructionActionResult, String> {
  let _ops = ops_guard();
  let hub_file = env.instruction_file(name);
  if !hub_file.is_file() {
    return Err(format!(
      "Library copy is missing: {}",
      hub_file.to_string_lossy()
    ));
  }
  let hash = hash_file(&hub_file)?;
  let timestamp = now_rfc3339();
  let record = update_lock(env, |lock| {
    let record = record_mut(lock, name)?;
    record.hash = hash.clone();
    record.updated_at = timestamp.clone();
    Ok(record.clone())
  })?;
  Ok(InstructionActionResult::applied(build_view(
    env, name, &record,
  )))
}

/// Remove an instruction from the library. With `remove_installs = false`, symlink targets
/// become real files first so agents keep reading them.
pub fn remove_instruction(env: &Env, name: &str, remove_installs: bool) -> Result<(), String> {
  let _ops = ops_guard();
  let record = get_record(env, name)?;
  let hub_file = env.instruction_file(name);
  let mut errors = Vec::new();
  for install in &record.installs {
    let target = Path::new(&install.path);
    if remove_installs {
      if target.exists() || is_symlink(target) {
        if let Err(err) = remove_path_any(target) {
          errors.push(format!("{}: {}", install.path, err));
        }
      }
    } else if let Err(err) = materialize_symlink(&hub_file, target) {
      errors.push(format!("{}: {}", install.path, err));
    }
  }
  if hub_file.exists() || is_symlink(&hub_file) {
    trash_file(env, &hub_file)?;
  }
  update_lock(env, |lock| {
    lock.instructions.remove(name);
    Ok(())
  })?;
  if !errors.is_empty() {
    return Err(errors.join("\n"));
  }
  Ok(())
}

/// Replace a symlink into the library with a copy of the library file.
fn materialize_symlink(hub_file: &Path, target: &Path) -> Result<(), String> {
  if !is_symlink(target) || !symlink_points_to(target, hub_file) {
    return Ok(());
  }
  if !hub_file.is_file() {
    return Err("library copy is missing".to_string());
  }
  remove_path_any(target)?;
  fs::copy(hub_file, target)
    .map(|_| ())
    .map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Install / uninstall
// ---------------------------------------------------------------------------

struct TargetGroup {
  scope: InstallScope,
  project_path: Option<String>,
  path: PathBuf,
  agent_ids: Vec<String>,
}

fn group_targets(env: &Env, specs: &[InstallTargetSpec]) -> Result<Vec<TargetGroup>, String> {
  let mut groups: Vec<TargetGroup> = Vec::new();
  for spec in specs {
    let resolved = resolve_target(env, spec)?;
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
  Keep(InstallMode),
  Write(InstallMode),
}

/// Write the library file to `target`. Returns the effective mode (Windows falls back to a
/// copy when file symlinks are not permitted).
fn write_target(hub_file: &Path, target: &Path, mode: InstallMode) -> Result<InstallMode, String> {
  if target.exists() || is_symlink(target) {
    remove_path_any(target)?;
  }
  if let Some(parent) = target.parent() {
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
  }
  match mode {
    InstallMode::Symlink => {
      #[cfg(unix)]
      {
        std::os::unix::fs::symlink(hub_file, target).map_err(|e| e.to_string())?;
        Ok(InstallMode::Symlink)
      }
      #[cfg(windows)]
      {
        match std::os::windows::fs::symlink_file(hub_file, target) {
          Ok(()) => Ok(InstallMode::Symlink),
          Err(_) => {
            fs::copy(hub_file, target).map_err(|e| e.to_string())?;
            Ok(InstallMode::Copy)
          },
        }
      }
    },
    InstallMode::Copy => {
      fs::copy(hub_file, target).map_err(|e| e.to_string())?;
      Ok(InstallMode::Copy)
    },
  }
}

/// Decide what to do with a target file that has no install record of this instruction.
fn classify_new_target(
  hub_file: &Path,
  path: &Path,
  hub_hash: &str,
  mode: InstallMode,
  force: bool,
) -> Result<Plan, String> {
  if !path.exists() && !is_symlink(path) {
    return Ok(Plan::Write(mode));
  }
  if is_symlink(path) {
    if symlink_points_to(path, hub_file) {
      return Ok(if mode == InstallMode::Symlink {
        Plan::Keep(InstallMode::Symlink)
      } else {
        Plan::Write(mode)
      });
    }
    if !path.exists() || force {
      return Ok(Plan::Write(mode));
    }
    return Err(format!(
      "{} is a symlink to another location",
      path.to_string_lossy()
    ));
  }
  if !path.is_file() {
    return Err(format!(
      "{} exists and is not a file",
      path.to_string_lossy()
    ));
  }
  let current = hash_file(path).unwrap_or_default();
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
    "{} already exists with different content; import it into the library or use force to overwrite",
    path.to_string_lossy()
  ))
}

#[allow(clippy::too_many_arguments)]
fn upsert_install(
  record: &mut InstructionRecord,
  scope: InstallScope,
  project_path: Option<String>,
  path: &str,
  mode: InstallMode,
  agent_ids: &[String],
  hash: &str,
  timestamp: &str,
) {
  if let Some(existing) = record
    .installs
    .iter_mut()
    .find(|install| same_path(Path::new(&install.path), Path::new(path)))
  {
    existing.mode = mode;
    existing.hash = hash.to_string();
    for id in agent_ids {
      if !existing.agent_ids.contains(id) {
        existing.agent_ids.push(id.clone());
      }
    }
    return;
  }
  record.installs.push(InstallRecord {
    scope,
    project_path,
    path: path.to_string(),
    mode,
    hash: hash.to_string(),
    installed_at: timestamp.to_string(),
    agent_ids: agent_ids.to_vec(),
  });
}

pub fn install_instruction(
  env: &Env,
  request: InstallRequest,
) -> Result<InstructionActionResult, String> {
  let _ops = ops_guard();
  let name = request.name.trim().to_string();
  let lock = read_lock(env)?;
  let record = lock
    .instructions
    .get(&name)
    .cloned()
    .ok_or_else(|| format!("Instruction '{}' is not in the library", name))?;
  let hub_file = env.instruction_file(&name);
  if !hub_file.is_file() {
    return Err(format!(
      "Library copy is missing: {}",
      hub_file.to_string_lossy()
    ));
  }
  if request.targets.is_empty() {
    return Err("At least one install target is required".to_string());
  }
  let requested_mode = request.mode.unwrap_or_default();
  let hub_hash = hash_file(&hub_file)?;
  let groups = group_targets(env, &request.targets)?;

  let mut blockers = Vec::new();
  let mut plans: Vec<(usize, Plan)> = Vec::new();
  // (other instruction, path) pairs whose install record this install takes over.
  let mut displaced: Vec<(String, String)> = Vec::new();
  for (index, group) in groups.iter().enumerate() {
    let existing = record
      .installs
      .iter()
      .find(|install| same_path(Path::new(&install.path), &group.path));
    let plan = if let Some(existing) = existing {
      let (state, _) = target_state(env, &name, existing, Some(&hub_hash));
      let mode = request.mode.unwrap_or(existing.mode);
      if mode != existing.mode {
        if !request.force {
          blockers.push(format!(
            "{} is already installed as {:?}; use force to switch to {:?}",
            existing.path, existing.mode, mode
          ));
          continue;
        }
        if matches!(state, TargetState::Modified | TargetState::Conflict) && !request.force {
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
      // One file per agent: another instruction may already be deployed there.
      let occupant = lock.instructions.iter().find(|(other, other_record)| {
        *other != &name
          && other_record
            .installs
            .iter()
            .any(|install| same_path(Path::new(&install.path), &group.path))
      });
      if let Some((other, _)) = occupant {
        if !request.force {
          blockers.push(format!(
            "{} is occupied by '{}'; use force to replace it",
            group.path.to_string_lossy(),
            other
          ));
          continue;
        }
        displaced.push((other.clone(), path_to_string(&group.path)));
        Plan::Write(requested_mode)
      } else {
        match classify_new_target(
          &hub_file,
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
      }
    };
    plans.push((index, plan));
  }

  if !blockers.is_empty() {
    return Ok(InstructionActionResult::blocked(
      blockers,
      Some(build_view(env, &name, &record)),
    ));
  }

  let mut errors = Vec::new();
  let mut effective: BTreeMap<usize, InstallMode> = BTreeMap::new();
  for (index, plan) in plans {
    let group = &groups[index];
    match plan {
      Plan::Keep(mode) => {
        effective.insert(index, mode);
      },
      Plan::Write(mode) => match write_target(&hub_file, &group.path, mode) {
        Ok(mode) => {
          effective.insert(index, mode);
        },
        Err(err) => errors.push(format!("{}: {}", group.path.to_string_lossy(), err)),
      },
    }
  }

  let timestamp = now_rfc3339();
  update_lock(env, |lock| {
    for (other, path) in &displaced {
      if let Some(other_record) = lock.instructions.get_mut(other) {
        other_record
          .installs
          .retain(|install| !same_path(Path::new(&install.path), Path::new(path)));
      }
    }
    let record = record_mut(lock, &name)?;
    for (index, mode) in &effective {
      let group = &groups[*index];
      upsert_install(
        record,
        group.scope,
        group.project_path.clone(),
        &path_to_string(&group.path),
        *mode,
        &group.agent_ids,
        &hub_hash,
        &timestamp,
      );
    }
    Ok(())
  })?;

  if !errors.is_empty() {
    return Err(errors.join("\n"));
  }
  Ok(InstructionActionResult::applied(view_of(env, &name)?))
}

pub fn uninstall_instruction(
  env: &Env,
  request: UninstallRequest,
) -> Result<InstructionActionResult, String> {
  let _ops = ops_guard();
  let name = request.name.trim().to_string();
  let record = get_record(env, &name)?;
  let hub_hash = hash_file(&env.instruction_file(&name)).ok();

  // (install path, agent ids to drop)
  let mut removals: Vec<(String, Vec<String>)> = Vec::new();
  for spec in &request.targets {
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
  for path in &request.paths {
    let wanted = Path::new(path.trim());
    for install in &record.installs {
      let recorded = Path::new(&install.path);
      if recorded != wanted && !same_path(recorded, wanted) {
        continue;
      }
      match removals.iter_mut().find(|(p, _)| p == &install.path) {
        Some((_, ids)) => {
          for id in &install.agent_ids {
            if !ids.contains(id) {
              ids.push(id.clone());
            }
          }
        },
        None => removals.push((install.path.clone(), install.agent_ids.clone())),
      }
    }
  }

  let mut blockers = Vec::new();
  let mut files_to_remove: Vec<PathBuf> = Vec::new();
  for (path, ids) in &removals {
    let Some(install) = record.installs.iter().find(|i| &i.path == path) else {
      continue;
    };
    if install.agent_ids.iter().any(|id| !ids.contains(id)) {
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
    return Ok(InstructionActionResult::blocked(
      blockers,
      Some(build_view(env, &name, &record)),
    ));
  }

  let mut errors = Vec::new();
  for path in &files_to_remove {
    if let Err(err) = remove_path_any(path) {
      errors.push(format!("{}: {}", path.to_string_lossy(), err));
    }
  }
  update_lock(env, |lock| {
    let record = record_mut(lock, &name)?;
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
  Ok(InstructionActionResult::applied(view_of(env, &name)?))
}

// ---------------------------------------------------------------------------
// Sync
// ---------------------------------------------------------------------------

/// Push the library file to copy-mode targets. Without `force`, edited targets are left
/// alone and reported as blockers.
pub fn push_targets(
  env: &Env,
  name: &str,
  only: Option<&[String]>,
  force: bool,
) -> Result<InstructionActionResult, String> {
  let record = get_record(env, name)?;
  let hub_file = env.instruction_file(name);
  if !hub_file.is_file() {
    return Err(format!(
      "Library copy is missing: {}",
      hub_file.to_string_lossy()
    ));
  }
  let hub_hash = hash_file(&hub_file)?;

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
      if symlink_points_to(target, &hub_file) {
        pushed.push(install.path.clone());
      }
      continue;
    }
    let (state, _) = target_state(env, name, install, Some(&hub_hash));
    let overwrite = match state {
      TargetState::InSync => {
        pushed.push(install.path.clone());
        false
      },
      TargetState::Outdated | TargetState::Missing => true,
      TargetState::Modified | TargetState::Conflict => {
        if !force {
          blockers.push(format!("{} has local changes", install.path));
        }
        force
      },
      TargetState::BrokenLink => {
        errors.push(format!("{} is a broken symlink", install.path));
        false
      },
    };
    if overwrite {
      match write_target(&hub_file, target, InstallMode::Copy) {
        Ok(_) => pushed.push(install.path.clone()),
        Err(err) => errors.push(format!("{}: {}", install.path, err)),
      }
    }
  }

  if !pushed.is_empty() {
    update_lock(env, |lock| {
      let record = record_mut(lock, name)?;
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
    Ok(InstructionActionResult::applied(view))
  } else {
    Ok(InstructionActionResult::blocked(blockers, Some(view)))
  }
}

/// Take the content of one copy target as the new library version.
pub fn adopt_target(
  env: &Env,
  name: &str,
  path: &str,
  force: bool,
) -> Result<InstructionActionResult, String> {
  let record = get_record(env, name)?;
  let install = record
    .installs
    .iter()
    .find(|install| same_path(Path::new(&install.path), Path::new(path)))
    .ok_or_else(|| format!("{} is not an install target of '{}'", path, name))?;
  let target = Path::new(&install.path);
  if is_symlink(target) {
    return Err("Symlink targets already mirror the library".to_string());
  }
  if !target.is_file() {
    return Err(format!("{} does not exist", install.path));
  }
  let probe = probe_hub(env, name, &record);
  if probe.state == HubState::Modified && !force {
    return Ok(InstructionActionResult::blocked(
      vec![format!("Library copy of '{}' has local changes", name)],
      Some(build_view(env, name, &record)),
    ));
  }
  let adopted = install.path.clone();
  let new_hash = replace_hub_file(env, name, target)?;
  update_lock(env, |lock| {
    let record = record_mut(lock, name)?;
    record.hash = new_hash.clone();
    record.updated_at = now_rfc3339();
    if let Some(install) = record
      .installs
      .iter_mut()
      .find(|install| install.path == adopted)
    {
      install.hash = new_hash.clone();
    }
    Ok(())
  })?;
  Ok(InstructionActionResult::applied(view_of(env, name)?))
}

/// Replace the library file with `src` (the old copy goes to the trash); returns the hash.
fn replace_hub_file(env: &Env, name: &str, src: &Path) -> Result<String, String> {
  let hub_file = env.instruction_file(name);
  if hub_file.exists() || is_symlink(&hub_file) {
    trash_file(env, &hub_file)?;
  }
  fs::create_dir_all(&env.instructions_root).map_err(|e| e.to_string())?;
  fs::copy(src, &hub_file).map_err(|e| e.to_string())?;
  hash_file(&hub_file)
}

pub fn sync_instruction(
  env: &Env,
  name: &str,
  action: SyncAction,
) -> Result<InstructionActionResult, String> {
  match action {
    SyncAction::PushTargets { targets, force } => {
      let _ops = ops_guard();
      push_targets(env, name, targets.as_deref(), force)
    },
    SyncAction::AdoptTarget { path, force } => {
      let _ops = ops_guard();
      adopt_target(env, name, &path, force)
    },
    SyncAction::AcceptHub => accept_hub(env, name),
    SyncAction::PullSource { .. } | SyncAction::PushSource { .. } => {
      Err("Instructions have no source to sync with".to_string())
    },
  }
}

pub fn diff_instruction(env: &Env, name: &str, path: &str) -> Result<SkillDiff, String> {
  let hub_file = env.instruction_file(name);
  let target = Path::new(path);
  let rel = target
    .file_name()
    .map(|n| n.to_string_lossy().to_string())
    .unwrap_or_else(|| format!("{}.md", name));
  let file = diff_files(&rel, &hub_file, target)?;
  let unchanged = usize::from(file.is_none());
  Ok(SkillDiff {
    name: name.to_string(),
    left_label: path_to_string(&hub_file),
    right_label: path.to_string(),
    files: file.into_iter().collect(),
    unchanged,
  })
}

// ---------------------------------------------------------------------------
// Scan
// ---------------------------------------------------------------------------

/// Every instruction file that exists at an agent location: user level and each registered
/// project. Files already in the library are matched by install record or content.
pub fn scan_instruction_files(env: &Env) -> Result<Vec<InstructionScanItem>, String> {
  let lock = read_lock(env)?;
  let hub_hashes: Vec<(String, String)> = lock
    .instructions
    .keys()
    .filter_map(|name| {
      hash_file(&env.instruction_file(name))
        .ok()
        .map(|hash| (name.clone(), hash))
    })
    .collect();

  let mut candidates: Vec<(PathBuf, AgentRootMatch)> = Vec::new();
  let mut add = |path: PathBuf, location: AgentRootMatch| {
    if let Some((_, existing)) = candidates
      .iter_mut()
      .find(|(known, _)| same_path(known, &path))
    {
      for id in location.agent_ids {
        if !existing.agent_ids.contains(&id) {
          existing.agent_ids.push(id);
        }
      }
    } else {
      candidates.push((path, location));
    }
  };
  for app in &env.agent_apps {
    if let Ok(path) = env.agent_profile(app, InstallScope::User, None) {
      if path.is_file() {
        add(
          path,
          AgentRootMatch {
            scope: InstallScope::User,
            project_path: None,
            agent_ids: vec![app.id.clone()],
            registered_project: false,
          },
        );
      }
    }
  }
  for project in &env.projects {
    let root = Path::new(&project.path);
    for app in &env.agent_apps {
      if let Ok(path) = env.agent_profile(app, InstallScope::Project, Some(root)) {
        if path.is_file() {
          add(
            path,
            AgentRootMatch {
              scope: InstallScope::Project,
              project_path: Some(project.path.clone()),
              agent_ids: vec![app.id.clone()],
              registered_project: true,
            },
          );
        }
      }
    }
  }

  let mut items: Vec<InstructionScanItem> = candidates
    .into_iter()
    .map(|(path, location)| {
      let hash = hash_file(&path).ok();
      let tracked = lock
        .instructions
        .iter()
        .find(|(_, record)| {
          record
            .installs
            .iter()
            .any(|install| same_path(Path::new(&install.path), &path))
        })
        .map(|(name, _)| name.clone());
      let linked = is_symlink(&path)
        && fs::read_link(&path)
          .ok()
          .map(|raw| {
            let resolved = if raw.is_absolute() {
              raw
            } else {
              path.parent().map(|p| p.join(&raw)).unwrap_or(raw)
            };
            is_within(&resolved, &env.instructions_root)
          })
          .unwrap_or(false);
      let (status, hub_name, registered) = if linked {
        let name = tracked
          .clone()
          .or_else(|| fs::read_link(&path).ok().and_then(|raw| library_name(&raw)));
        (ScanStatus::Linked, name, tracked.is_some())
      } else if let Some(name) = tracked {
        let current = hub_hashes
          .iter()
          .find(|(known, _)| known == &name)
          .map(|(_, hash)| hash.clone());
        let status = if current.is_some() && current == hash {
          ScanStatus::Identical
        } else {
          ScanStatus::Different
        };
        (status, Some(name), true)
      } else if let Some((name, _)) = hub_hashes
        .iter()
        .find(|(_, known)| Some(known) == hash.as_ref())
      {
        (ScanStatus::Identical, Some(name.clone()), false)
      } else {
        (ScanStatus::New, None, false)
      };
      InstructionScanItem {
        name: String::new(),
        file_name: path
          .file_name()
          .map(|n| n.to_string_lossy().to_string())
          .unwrap_or_default(),
        path: path_to_string(&path),
        hash,
        status,
        hub_name,
        registered,
        location,
        error: None,
      }
    })
    .collect();

  items.sort_by(|a, b| {
    let rank = |item: &InstructionScanItem| match item.location.scope {
      InstallScope::User => 0,
      InstallScope::Project => 1,
    };
    rank(a).cmp(&rank(b)).then_with(|| a.path.cmp(&b.path))
  });
  suggest_names(&lock, &mut items);
  Ok(items)
}

/// Library names for new files: `user-<file>` at user level, the project folder name in a
/// project (plus the file stem when a project has several files), made unique.
fn suggest_names(lock: &InstructionLockFile, items: &mut [InstructionScanItem]) {
  let mut used: Vec<String> = lock.instructions.keys().map(|k| k.to_lowercase()).collect();
  let project_counts: BTreeMap<String, usize> =
    items.iter().fold(BTreeMap::new(), |mut map, item| {
      if let Some(project) = &item.location.project_path {
        *map.entry(project.clone()).or_insert(0) += 1;
      }
      map
    });
  for item in items.iter_mut() {
    if let Some(hub_name) = &item.hub_name {
      item.name = hub_name.clone();
      continue;
    }
    let stem = item.file_name.trim_end_matches(".md").to_lowercase();
    let base = match &item.location.project_path {
      None => format!("user-{}", stem),
      Some(project) => {
        let folder = Path::new(project)
          .file_name()
          .map(|n| n.to_string_lossy().to_string())
          .unwrap_or_else(|| "project".to_string());
        if project_counts.get(project).copied().unwrap_or(0) > 1 {
          format!("{}-{}", folder, stem)
        } else {
          folder
        }
      },
    };
    let base = if validate_instruction_name(&base).is_ok() {
      base
    } else {
      format!("instruction-{}", stem)
    };
    let mut candidate = base.clone();
    let mut counter = 2;
    while used.contains(&candidate.to_lowercase()) {
      candidate = format!("{}-{}", base, counter);
      counter += 1;
    }
    used.push(candidate.to_lowercase());
    item.name = candidate;
  }
}

/// Apply scan decisions: import new files (registering them as installs), register files
/// identical to a library entry, and adopt or overwrite files that drifted from theirs.
pub fn import_scanned_instructions(
  env: &Env,
  decisions: Vec<InstructionScanDecision>,
) -> Result<Vec<InstructionImportOutcome>, String> {
  let _ops = ops_guard();
  let mut outcomes = Vec::new();
  let mut errors = Vec::new();
  for decision in decisions {
    let path = Path::new(decision.path.trim());
    let result = match decision.resolution {
      ScanResolution::Skip => continue,
      ScanResolution::Import => match &decision.hub_name {
        Some(hub_name) => register_identical(env, hub_name, path, decision.register_install),
        None => import_one(env, &decision.name, path, false, decision.register_install),
      },
      ScanResolution::AdoptIntoHub => match &decision.hub_name {
        Some(hub_name) => adopt_scanned(env, hub_name, path),
        None => Err("adopting needs a library entry".to_string()),
      },
      ScanResolution::PushFromHub => match &decision.hub_name {
        Some(hub_name) => push_scanned(env, hub_name, path),
        None => Err("overwriting needs a library entry".to_string()),
      },
    };
    match result {
      Ok(outcome) => outcomes.push(outcome),
      Err(err) => errors.push(format!("{}: {}", decision.path, err)),
    }
  }
  finish_import(outcomes, errors)
}

fn outcome_for(env: &Env, name: &str, hash: String) -> InstructionImportOutcome {
  InstructionImportOutcome {
    name: name.to_string(),
    hub_path: path_to_string(&env.instruction_file(name)),
    hash,
    replaced: true,
  }
}

fn register_path(env: &Env, name: &str, path: &Path, hash: &str) -> Result<(), String> {
  let matched =
    classify_profile(env, path).ok_or_else(|| "not an agent instruction location".to_string())?;
  let timestamp = now_rfc3339();
  update_lock(env, |lock| {
    let record = record_mut(lock, name)?;
    upsert_install(
      record,
      matched.scope,
      matched.project_path.clone(),
      &path_to_string(path),
      InstallMode::Copy,
      &matched.agent_ids,
      hash,
      &timestamp,
    );
    Ok(())
  })
}

/// A file with the same content as a library entry only needs an install record.
fn register_identical(
  env: &Env,
  name: &str,
  path: &Path,
  register: bool,
) -> Result<InstructionImportOutcome, String> {
  let hub_hash = hash_file(&env.instruction_file(name))?;
  if hash_file(path)? != hub_hash {
    return Err(format!("content differs from '{}'", name));
  }
  if register {
    register_path(env, name, path, &hub_hash)?;
  }
  Ok(outcome_for(env, name, hub_hash))
}

fn adopt_scanned(env: &Env, name: &str, path: &Path) -> Result<InstructionImportOutcome, String> {
  get_record(env, name)?;
  if !path.is_file() {
    return Err("not a file".to_string());
  }
  let hash = replace_hub_file(env, name, path)?;
  update_lock(env, |lock| {
    let record = record_mut(lock, name)?;
    record.hash = hash.clone();
    record.updated_at = now_rfc3339();
    Ok(())
  })?;
  register_path(env, name, path, &hash)?;
  Ok(outcome_for(env, name, hash))
}

fn push_scanned(env: &Env, name: &str, path: &Path) -> Result<InstructionImportOutcome, String> {
  get_record(env, name)?;
  let hub_file = env.instruction_file(name);
  if !hub_file.is_file() {
    return Err("library copy is missing".to_string());
  }
  let hash = hash_file(&hub_file)?;
  write_target(&hub_file, path, InstallMode::Copy)?;
  register_path(env, name, path, &hash)?;
  Ok(outcome_for(env, name, hash))
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::models::{AgentApp, UserProject};

  fn app(id: &str, profile: &str, global_profile: &str) -> AgentApp {
    AgentApp {
      id: id.to_string(),
      display_name: id.to_string(),
      project_path: Some(format!(".{}/skills", id)),
      global_path: Some(format!("~/.{}/skills", id)),
      detect_path: None,
      profile_path: Some(profile.to_string()),
      global_profile_path: Some(global_profile.to_string()),
      is_user_custom: false,
    }
  }

  fn write(path: &Path, content: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
  }

  fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap()
  }

  /// Claude Code with its own file, Codex and the shared `agents` app both on `AGENTS.md`,
  /// and one registered project.
  fn env(tmp: &Path) -> Env {
    let project = tmp.join("proj");
    fs::create_dir_all(&project).unwrap();
    Env::for_test(
      tmp,
      vec![
        app("agents", "AGENTS.md", "~/.agents/AGENTS.md"),
        app("claude-code", "CLAUDE.md", "~/.claude/CLAUDE.md"),
        app("codex", "AGENTS.md", "~/.codex/AGENTS.md"),
      ],
      vec![UserProject {
        name: "proj".to_string(),
        path: project.to_string_lossy().to_string(),
        workspace_path: None,
      }],
    )
  }

  fn import(env: &Env, tmp: &Path, name: &str, body: &str) -> InstructionImportOutcome {
    let src = tmp.join("src").join(format!("{}.md", name));
    write(&src, body);
    let mut outcomes = import_instructions(
      env,
      vec![InstructionImportItem {
        name: name.to_string(),
        path: src.to_string_lossy().to_string(),
      }],
      false,
    )
    .unwrap();
    outcomes.remove(0)
  }

  fn spec(scope: InstallScope, project: Option<&Path>, agent: &str) -> InstallTargetSpec {
    InstallTargetSpec {
      scope,
      project_path: project.map(|p| p.to_string_lossy().to_string()),
      agent_id: agent.to_string(),
    }
  }

  fn install(
    env: &Env,
    name: &str,
    targets: Vec<InstallTargetSpec>,
    mode: InstallMode,
    force: bool,
  ) -> InstructionActionResult {
    install_instruction(
      env,
      InstallRequest {
        name: name.to_string(),
        targets,
        mode: Some(mode),
        force,
      },
    )
    .unwrap()
  }

  #[test]
  fn import_install_drift_adopt_and_push() {
    let tmp = tempfile::tempdir().unwrap();
    let env = env(tmp.path());
    let outcome = import(&env, tmp.path(), "rules", "# Team rules\n\nv1\n");
    assert!(!outcome.replaced);
    let view = get_instruction(&env, "rules").unwrap().unwrap();
    assert_eq!(view.description.as_deref(), Some("Team rules"));
    assert_eq!(view.hub_state, HubState::Ok);
    assert!(view.installs.is_empty());

    let result = install(
      &env,
      "rules",
      vec![spec(InstallScope::User, None, "claude-code")],
      InstallMode::Copy,
      false,
    );
    assert!(result.applied);
    let target = env.home.join(".claude/CLAUDE.md");
    assert!(read(&target).ends_with("v1\n"));
    let view = result.instruction.unwrap();
    assert_eq!(view.installs.len(), 1);
    assert_eq!(view.installs[0].state, TargetState::InSync);
    assert_eq!(view.installs[0].record.agent_ids, vec!["claude-code"]);

    // Edit the target: modified, then adopted into the library.
    write(&target, "# Team rules\n\nv2 edited\n");
    let view = get_instruction(&env, "rules").unwrap().unwrap();
    assert_eq!(view.installs[0].state, TargetState::Modified);
    assert!(view.has_drift);
    let adopted = sync_instruction(
      &env,
      "rules",
      SyncAction::AdoptTarget {
        path: path_to_string(&target),
        force: false,
      },
    )
    .unwrap();
    assert!(adopted.applied);
    assert!(read(&env.instruction_file("rules")).ends_with("v2 edited\n"));
    assert_eq!(
      adopted.instruction.unwrap().installs[0].state,
      TargetState::InSync
    );

    // Edit the library copy: accept it, the target is outdated, push it.
    write(&env.instruction_file("rules"), "# Team rules\n\nv3\n");
    let view = get_instruction(&env, "rules").unwrap().unwrap();
    assert_eq!(view.hub_state, HubState::Modified);
    let accepted = sync_instruction(&env, "rules", SyncAction::AcceptHub).unwrap();
    let view = accepted.instruction.unwrap();
    assert_eq!(view.hub_state, HubState::Ok);
    assert_eq!(view.installs[0].state, TargetState::Outdated);
    let pushed = sync_instruction(
      &env,
      "rules",
      SyncAction::PushTargets {
        targets: None,
        force: false,
      },
    )
    .unwrap();
    assert!(pushed.applied);
    assert!(read(&target).ends_with("v3\n"));
  }

  #[test]
  fn shared_project_file_groups_agents_and_uninstalls_by_agent() {
    let tmp = tempfile::tempdir().unwrap();
    let env = env(tmp.path());
    import(&env, tmp.path(), "rules", "rules\n");
    let project = tmp.path().join("proj");
    let result = install(
      &env,
      "rules",
      vec![
        spec(InstallScope::Project, Some(&project), "codex"),
        spec(InstallScope::Project, Some(&project), "agents"),
      ],
      InstallMode::Copy,
      false,
    );
    assert!(result.applied);
    let record = get_record(&env, "rules").unwrap();
    assert_eq!(record.installs.len(), 1);
    assert!(same_path(
      Path::new(&record.installs[0].path),
      &project.join("AGENTS.md")
    ));
    assert_eq!(record.installs[0].agent_ids, vec!["codex", "agents"]);
    // Claude Code reads CLAUDE.md, so it is not shown on the AGENTS.md install.
    let view = get_instruction(&env, "rules").unwrap().unwrap();
    assert_eq!(view.installs[0].record.agent_ids, vec!["agents", "codex"]);

    let result = uninstall_instruction(
      &env,
      UninstallRequest {
        name: "rules".to_string(),
        targets: vec![spec(InstallScope::Project, Some(&project), "codex")],
        paths: vec![],
        force: false,
      },
    )
    .unwrap();
    assert!(result.applied);
    assert!(project.join("AGENTS.md").is_file(), "agents still reads it");
    let result = uninstall_instruction(
      &env,
      UninstallRequest {
        name: "rules".to_string(),
        targets: vec![],
        paths: vec![path_to_string(&project.join("AGENTS.md"))],
        force: false,
      },
    )
    .unwrap();
    assert!(result.applied);
    assert!(!project.join("AGENTS.md").exists());
    assert!(get_record(&env, "rules").unwrap().installs.is_empty());
  }

  #[test]
  fn occupied_file_is_blocked_then_taken_over_with_force() {
    let tmp = tempfile::tempdir().unwrap();
    let env = env(tmp.path());
    import(&env, tmp.path(), "a", "A\n");
    import(&env, tmp.path(), "b", "B\n");
    let project = tmp.path().join("proj");
    assert!(
      install(
        &env,
        "a",
        vec![spec(InstallScope::Project, Some(&project), "codex")],
        InstallMode::Copy,
        false
      )
      .applied
    );

    let blocked = install(
      &env,
      "b",
      vec![spec(InstallScope::Project, Some(&project), "codex")],
      InstallMode::Copy,
      false,
    );
    assert!(!blocked.applied);
    assert!(blocked.blockers[0].contains("occupied by 'a'"));
    assert_eq!(read(&project.join("AGENTS.md")), "A\n");

    let forced = install(
      &env,
      "b",
      vec![spec(InstallScope::Project, Some(&project), "codex")],
      InstallMode::Copy,
      true,
    );
    assert!(forced.applied);
    assert_eq!(read(&project.join("AGENTS.md")), "B\n");
    assert!(get_record(&env, "a").unwrap().installs.is_empty());
    assert_eq!(get_record(&env, "b").unwrap().installs.len(), 1);
  }

  #[test]
  fn hand_written_file_blocks_install_until_forced() {
    let tmp = tempfile::tempdir().unwrap();
    let env = env(tmp.path());
    import(&env, tmp.path(), "rules", "rules\n");
    let target = env.home.join(".codex/AGENTS.md");
    write(&target, "mine\n");
    let blocked = install(
      &env,
      "rules",
      vec![spec(InstallScope::User, None, "codex")],
      InstallMode::Copy,
      false,
    );
    assert!(!blocked.applied);
    assert!(blocked.blockers[0].contains("different content"));
    assert_eq!(read(&target), "mine\n");
    let forced = install(
      &env,
      "rules",
      vec![spec(InstallScope::User, None, "codex")],
      InstallMode::Copy,
      true,
    );
    assert!(forced.applied);
    assert_eq!(read(&target), "rules\n");
  }

  #[test]
  fn importing_an_agent_file_registers_it_as_install() {
    let tmp = tempfile::tempdir().unwrap();
    let env = env(tmp.path());
    let source = env.home.join(".codex/AGENTS.md");
    write(&source, "# Codex\n");
    import_instructions(
      &env,
      vec![InstructionImportItem {
        name: "codex-rules".to_string(),
        path: path_to_string(&source),
      }],
      false,
    )
    .unwrap();
    let view = get_instruction(&env, "codex-rules").unwrap().unwrap();
    assert_eq!(view.installs.len(), 1);
    assert_eq!(view.installs[0].record.scope, InstallScope::User);
    assert_eq!(view.installs[0].record.agent_ids, vec!["codex"]);
    assert_eq!(view.installs[0].state, TargetState::InSync);
    assert!(source.is_file(), "the agent file stays in place");
    assert!(import_instructions(
      &env,
      vec![InstructionImportItem {
        name: "Codex-Rules".to_string(),
        path: path_to_string(&source),
      }],
      false,
    )
    .is_err());
  }

  #[test]
  fn scan_suggests_names_and_tracks_registered_files() {
    let tmp = tempfile::tempdir().unwrap();
    let env = env(tmp.path());
    let project = tmp.path().join("proj");
    write(&env.home.join(".claude/CLAUDE.md"), "# Claude\n");
    write(&project.join("AGENTS.md"), "# Proj\n");
    write(&project.join("CLAUDE.md"), "# Proj claude\n");

    let items = scan_instruction_files(&env).unwrap();
    assert_eq!(items.len(), 3);
    assert_eq!(items[0].name, "user-claude");
    assert_eq!(items[0].status, ScanStatus::New);
    assert_eq!(items[0].location.scope, InstallScope::User);
    assert_eq!(items[0].location.agent_ids, vec!["claude-code"]);
    let names: Vec<&str> = items[1..].iter().map(|item| item.name.as_str()).collect();
    assert_eq!(names, vec!["proj-agents", "proj-claude"]);
    assert_eq!(items[1].location.agent_ids, vec!["agents", "codex"]);
    assert!(items[1].location.registered_project);

    import_scanned_instructions(
      &env,
      vec![InstructionScanDecision {
        name: "user-claude".to_string(),
        path: items[0].path.clone(),
        resolution: ScanResolution::Import,
        register_install: true,
        hub_name: None,
      }],
    )
    .unwrap();
    let items = scan_instruction_files(&env).unwrap();
    assert_eq!(items[0].status, ScanStatus::Identical);
    assert_eq!(items[0].hub_name.as_deref(), Some("user-claude"));
    assert!(items[0].registered);

    // A second project file with the same content is only registered, not copied again.
    write(&project.join("CLAUDE.md"), "# Claude\n");
    let items = scan_instruction_files(&env).unwrap();
    let same = items
      .iter()
      .find(|item| item.file_name == "CLAUDE.md" && item.location.scope == InstallScope::Project)
      .unwrap();
    assert_eq!(same.status, ScanStatus::Identical);
    assert!(!same.registered);
    import_scanned_instructions(
      &env,
      vec![InstructionScanDecision {
        name: same.name.clone(),
        path: same.path.clone(),
        resolution: ScanResolution::Import,
        register_install: true,
        hub_name: Some("user-claude".to_string()),
      }],
    )
    .unwrap();
    assert_eq!(get_record(&env, "user-claude").unwrap().installs.len(), 2);

    // Edit the project file: different, then adopted.
    write(&project.join("CLAUDE.md"), "# Claude edited\n");
    let items = scan_instruction_files(&env).unwrap();
    let edited = items.iter().find(|item| item.path == same.path).unwrap();
    assert_eq!(edited.status, ScanStatus::Different);
    import_scanned_instructions(
      &env,
      vec![InstructionScanDecision {
        name: "user-claude".to_string(),
        path: edited.path.clone(),
        resolution: ScanResolution::AdoptIntoHub,
        register_install: true,
        hub_name: Some("user-claude".to_string()),
      }],
    )
    .unwrap();
    assert_eq!(
      read(&env.instruction_file("user-claude")),
      "# Claude edited\n"
    );
    let view = get_instruction(&env, "user-claude").unwrap().unwrap();
    let user = view
      .installs
      .iter()
      .find(|install| install.record.scope == InstallScope::User)
      .unwrap();
    assert_eq!(user.state, TargetState::Outdated);
  }

  #[cfg(unix)]
  #[test]
  fn create_symlink_and_remove_keeping_installs() {
    let tmp = tempfile::tempdir().unwrap();
    let env = env(tmp.path());
    let outcome = create_instruction(&env, "fresh", "").unwrap();
    assert_eq!(read(Path::new(&outcome.hub_path)), "# fresh\n");
    assert!(create_instruction(&env, "fresh", "again").is_err());

    let result = install(
      &env,
      "fresh",
      vec![spec(InstallScope::User, None, "claude-code")],
      InstallMode::Symlink,
      false,
    );
    assert!(result.applied);
    let target = env.home.join(".claude/CLAUDE.md");
    assert!(is_symlink(&target));
    assert_eq!(
      get_instruction(&env, "fresh").unwrap().unwrap().installs[0].state,
      TargetState::InSync
    );

    let diff = diff_instruction(&env, "fresh", &path_to_string(&target)).unwrap();
    assert!(diff.files.is_empty());
    assert_eq!(diff.unchanged, 1);

    remove_instruction(&env, "fresh", false).unwrap();
    assert!(!is_symlink(&target));
    assert_eq!(read(&target), "# fresh\n");
    assert!(get_instruction(&env, "fresh").unwrap().is_none());
    assert!(!env.instruction_file("fresh").exists());
  }

  #[test]
  fn untracked_library_files_are_adopted_on_list() {
    let tmp = tempfile::tempdir().unwrap();
    let env = env(tmp.path());
    write(&env.instruction_file("manual"), "# Manual\n");
    let list = list_instructions(&env).unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].name, "manual");
    assert_eq!(list[0].hub_state, HubState::Ok);
    assert!(!list[0].has_drift);
  }
}
