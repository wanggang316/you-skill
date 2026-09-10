//! The instruction library (`~/.youskill/instructions/<name>.md`): agent instruction files
//! kept once and deployed to the `AGENTS.md` / `CLAUDE.md` locations each agent reads.
//! Mirrors the skill hub (import, install, three-way drift, push / adopt) for single files;
//! install records share the skill types. Files found at agent locations are adopted into
//! the library automatically whenever it is listed.

use crate::models::{
  AgentRootMatch, DetectedInstruction, HubState, InstallMode, InstallRecord, InstallRequest,
  InstallScope, InstallTargetSpec, InstallView, InstructionActionResult, InstructionImportItem,
  InstructionImportOutcome, InstructionLockFile, InstructionRecord, InstructionSource,
  InstructionView, SkillDiff, SyncAction, TargetState, UninstallRequest, LOCK_VERSION,
};
use crate::services::diff_service::diff_files;
use crate::services::drift_service::compare_three_way;
use crate::services::env::Env;
use crate::services::install_service::ResolvedTarget;
use crate::services::lock_service::ops_guard;
use crate::utils::folder::create_temp_dir;
use crate::utils::github::GithubHelper;
use crate::utils::hash::hash_file;
use crate::utils::path::{
  expand_home_with, is_symlink, is_within, normalize_dir_path, path_to_string, remove_path_any,
  same_path, symlink_points_to, validate_instruction_name,
};
use crate::utils::time::{now_file_stamp, now_rfc3339};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use walkdir::WalkDir;

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
    source: record.source.clone(),
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
  adopt_agent_files(env)?;
  backfill_sources(env)?;
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
        source: InstructionSource::None,
        hash,
        imported_at: timestamp.clone(),
        updated_at: timestamp.clone(),
        installs: Vec::new(),
      });
    }
    Ok(())
  })
}

/// Entries adopted before sources were recorded: the agent file they were taken from is
/// their first install, the user-level one when there are several.
fn backfill_sources(env: &Env) -> Result<(), String> {
  let lock = read_lock(env)?;
  let missing: Vec<(String, String)> = lock
    .instructions
    .iter()
    .filter(|(_, record)| record.source == InstructionSource::None)
    .filter_map(|(name, record)| {
      let install = record
        .installs
        .iter()
        .find(|install| install.scope == InstallScope::User)
        .or_else(|| record.installs.first())?;
      Some((name.clone(), install.path.clone()))
    })
    .collect();
  if missing.is_empty() {
    return Ok(());
  }
  update_lock(env, |lock| {
    for (name, path) in missing {
      if let Some(record) = lock.instructions.get_mut(&name) {
        if record.source == InstructionSource::None {
          record.source = InstructionSource::Agent { path };
        }
      }
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
      item.source,
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
/// (when `register` is set) so the library entry and the file stay linked. Without an
/// explicit `source`, the file itself is recorded as one.
fn import_one(
  env: &Env,
  name: &str,
  src: &Path,
  source: Option<InstructionSource>,
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
  let source = source.unwrap_or_else(|| {
    if auto_install.is_some() {
      InstructionSource::Agent {
        path: src_text.clone(),
      }
    } else {
      InstructionSource::File {
        path: src_text.clone(),
      }
    }
  });
  update_lock(env, |lock| {
    let record = lock
      .instructions
      .entry(name.clone())
      .or_insert(InstructionRecord {
        source: InstructionSource::None,
        hash: hash.clone(),
        imported_at: timestamp.clone(),
        updated_at: timestamp.clone(),
        installs: Vec::new(),
      });
    record.source = source;
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
        source: InstructionSource::None,
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
// Agent locations
// ---------------------------------------------------------------------------

/// Every instruction file that exists at an agent location: user level and each registered
/// project whose folder exists. Keyed by path; agents that read the same file are merged.
fn agent_files(env: &Env) -> Vec<(PathBuf, AgentRootMatch)> {
  let mut found: Vec<(PathBuf, AgentRootMatch)> = Vec::new();
  let mut add = |path: PathBuf, location: AgentRootMatch| {
    if let Some((_, existing)) = found.iter_mut().find(|(known, _)| same_path(known, &path)) {
      for id in location.agent_ids {
        if !existing.agent_ids.contains(&id) {
          existing.agent_ids.push(id);
        }
      }
    } else {
      found.push((path, location));
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
    if !root.is_dir() {
      continue;
    }
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
  found
}

/// Files at agent locations become library entries without asking: a file with the same
/// content as an entry is registered as an install of it, anything else is imported under
/// a generated name. Symlinks are only registered when they point into the library, so a
/// file linked to a dotfiles repository is left alone.
fn adopt_agent_files(env: &Env) -> Result<(), String> {
  let lock = read_lock(env)?;
  let tracked = |path: &Path| {
    lock.instructions.values().any(|record| {
      record.installs.iter().any(|install| {
        let recorded = Path::new(&install.path);
        recorded == path || same_path(recorded, path)
      })
    })
  };
  let pending: Vec<(PathBuf, AgentRootMatch)> = agent_files(env)
    .into_iter()
    .filter(|(path, _)| !tracked(path))
    .collect();
  if pending.is_empty() {
    return Ok(());
  }

  let _ops = ops_guard();
  let mut hub_hashes: Vec<(String, String)> = lock
    .instructions
    .keys()
    .filter_map(|name| {
      hash_file(&env.instruction_file(name))
        .ok()
        .map(|hash| (name.clone(), hash))
    })
    .collect();
  let mut used: Vec<String> = lock
    .instructions
    .keys()
    .map(|key| key.to_lowercase())
    .collect();
  for (path, location) in pending {
    if is_symlink(&path) {
      let Some(name) = library_link_name(env, &path) else {
        continue;
      };
      if let Some((_, hash)) = hub_hashes.iter().find(|(known, _)| known == &name) {
        let hash = hash.clone();
        if let Err(err) =
          register_install(env, &name, &location, &path, InstallMode::Symlink, &hash)
        {
          tracing::warn!("registering {} failed: {}", path.to_string_lossy(), err);
        }
      }
      continue;
    }
    let Ok(hash) = hash_file(&path) else {
      continue;
    };
    let result = match hub_hashes.iter().find(|(_, known)| known == &hash) {
      Some((name, _)) => register_install(env, name, &location, &path, InstallMode::Copy, &hash),
      None => {
        let name = unique_name(&adopted_name(&path, &location), &used);
        used.push(name.to_lowercase());
        hub_hashes.push((name.clone(), hash));
        import_one(env, &name, &path, None, false, true).map(|_| ())
      },
    };
    if let Err(err) = result {
      tracing::warn!("adopting {} failed: {}", path.to_string_lossy(), err);
    }
  }
  Ok(())
}

/// Library entry a symlink points to, if it points into the library at all.
fn library_link_name(env: &Env, link: &Path) -> Option<String> {
  let raw = fs::read_link(link).ok()?;
  let resolved = if raw.is_absolute() {
    raw
  } else {
    link.parent().map(|parent| parent.join(&raw)).unwrap_or(raw)
  };
  if !is_within(&resolved, &env.instructions_root) {
    return None;
  }
  library_name(&resolved)
}

fn register_install(
  env: &Env,
  name: &str,
  location: &AgentRootMatch,
  path: &Path,
  mode: InstallMode,
  hash: &str,
) -> Result<(), String> {
  let timestamp = now_rfc3339();
  update_lock(env, |lock| {
    let record = record_mut(lock, name)?;
    upsert_install(
      record,
      location.scope,
      location.project_path.clone(),
      &path_to_string(path),
      mode,
      &location.agent_ids,
      hash,
      &timestamp,
    );
    Ok(())
  })
}

/// `user-<agent dir>` at user level (`~/.claude/CLAUDE.md` → `user-claude`) and
/// `<project>-<file>` in a project (`multica/AGENTS.md` → `multica-agents`).
fn adopted_name(path: &Path, location: &AgentRootMatch) -> String {
  let stem = file_stem(path);
  match &location.project_path {
    None => {
      let dir = path
        .parent()
        .and_then(Path::file_name)
        .map(|n| n.to_string_lossy().trim_start_matches('.').to_string())
        .unwrap_or_default();
      let tail = if is_upper(&stem) && !dir.is_empty() {
        dir
      } else {
        stem
      };
      slug(&format!("user-{}", tail))
    },
    Some(project) => {
      let folder = Path::new(project)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "project".to_string());
      slug(&format!("{}-{}", folder, stem))
    },
  }
}

fn file_stem(path: &Path) -> String {
  path
    .file_stem()
    .map(|s| s.to_string_lossy().to_string())
    .unwrap_or_default()
}

/// `AGENTS`, `CLAUDE`, `GEMINI`: conventional file names that say nothing on their own.
fn is_upper(stem: &str) -> bool {
  !stem.is_empty()
    && stem
      .chars()
      .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_' || c == '-')
}

/// A valid library name derived from free text: lower case, runs of other characters
/// become one `-`.
fn slug(text: &str) -> String {
  let mut out = String::new();
  let mut dash = false;
  for c in text.chars() {
    if c.is_ascii_alphanumeric() || c == '.' || c == '_' {
      out.push(c.to_ascii_lowercase());
      dash = false;
    } else if !dash && !out.is_empty() {
      out.push('-');
      dash = true;
    }
  }
  let out = out.trim_matches(|c| c == '-' || c == '.').to_string();
  if validate_instruction_name(&out).is_ok() {
    out
  } else {
    "instruction".to_string()
  }
}

fn unique_name(base: &str, used: &[String]) -> String {
  let mut candidate = base.to_string();
  let mut counter = 2;
  while used.contains(&candidate.to_lowercase()) {
    candidate = format!("{}-{}", base, counter);
    counter += 1;
  }
  candidate
}

// ---------------------------------------------------------------------------
// Detection: folders, files and GitHub repositories
// ---------------------------------------------------------------------------

const DETECT_MAX_DEPTH: usize = 6;
const DETECT_MAX_FILES: usize = 200;
const MARKDOWN_EXTENSIONS: [&str; 3] = ["md", "markdown", "mdc"];
/// Directories that never hold instruction files worth listing.
const SKIPPED_DIRS: [&str; 7] = [
  ".git",
  "node_modules",
  "target",
  "dist",
  "build",
  "vendor",
  ".youskill",
];

fn is_markdown(path: &Path) -> bool {
  path
    .extension()
    .map(|ext| {
      MARKDOWN_EXTENSIONS
        .iter()
        .any(|known| ext.eq_ignore_ascii_case(known))
    })
    .unwrap_or(false)
}

/// Hidden directories worth descending into: `.github` and whatever an agent's project
/// instruction file lives in. Other dotfolders (`.venv`, `.next`, ...) are skipped.
fn allowed_hidden_dirs(env: &Env) -> Vec<String> {
  let mut dirs = vec![".github".to_string()];
  for app in &env.agent_apps {
    let Some(profile) = app.profile_path.as_deref() else {
      continue;
    };
    let parents: Vec<String> = Path::new(profile)
      .parent()
      .map(|parent| {
        parent
          .components()
          .map(|c| c.as_os_str().to_string_lossy().to_string())
          .collect()
      })
      .unwrap_or_default();
    for dir in parents {
      if dir.starts_with('.') && dir.len() > 1 && !dirs.contains(&dir) {
        dirs.push(dir);
      }
    }
  }
  dirs
}

/// Markdown files under `root` (or `root` itself when it is a file), shallowest first.
fn find_markdown_files(env: &Env, root: &Path) -> Result<Vec<PathBuf>, String> {
  if root.is_file() {
    return Ok(vec![root.to_path_buf()]);
  }
  if !root.is_dir() {
    return Err(format!("Path does not exist: {}", root.to_string_lossy()));
  }
  let hidden = allowed_hidden_dirs(env);
  let mut out: Vec<PathBuf> = Vec::new();
  let mut walker = WalkDir::new(root)
    .follow_links(false)
    .max_depth(DETECT_MAX_DEPTH)
    .sort_by_file_name()
    .into_iter();
  while let Some(entry) = walker.next() {
    let Ok(entry) = entry else {
      continue;
    };
    let name = entry.file_name().to_string_lossy().to_string();
    if entry.file_type().is_dir() {
      let skip = entry.depth() > 0
        && (SKIPPED_DIRS.contains(&name.as_str())
          || (name.starts_with('.') && !hidden.contains(&name)));
      if skip {
        walker.skip_current_dir();
      }
      continue;
    }
    if !entry.file_type().is_file() || !is_markdown(entry.path()) {
      continue;
    }
    out.push(entry.path().to_path_buf());
    if out.len() >= DETECT_MAX_FILES {
      break;
    }
  }
  out.sort_by_key(|path| (path.components().count(), path.clone()));
  Ok(out)
}

/// `AGENTS.md` at the top of `repo` → `repo-agents`, `packages/x/CLAUDE.md` → `x-claude`,
/// `prompts/python.md` → `python`.
fn detected_name(file: &Path, base: &Path, base_label: &str) -> String {
  let stem = file_stem(file);
  if !is_upper(&stem) {
    return slug(&stem);
  }
  let parent = file.parent().unwrap_or(base);
  let label = if same_path(parent, base) {
    base_label.to_string()
  } else {
    parent
      .file_name()
      .map(|n| n.to_string_lossy().to_string())
      .unwrap_or_else(|| base_label.to_string())
  };
  slug(&format!("{}-{}", label, stem))
}

/// List the Markdown files under `root` as import candidates; `base` is where relative
/// paths start and `base_label` names it for suggested names.
fn detect_in(
  env: &Env,
  root: &Path,
  base: &Path,
  base_label: &str,
  used: &mut Vec<String>,
  source: impl Fn(&str) -> Option<InstructionSource>,
) -> Result<Vec<DetectedInstruction>, String> {
  let mut items = Vec::new();
  for file in find_markdown_files(env, root)? {
    let rel_path = file
      .strip_prefix(base)
      .map(|p| p.to_string_lossy().replace('\\', "/"))
      .unwrap_or_else(|_| path_to_string(&file));
    let name = unique_name(&detected_name(&file, base, base_label), used);
    used.push(name.to_lowercase());
    items.push(DetectedInstruction {
      name,
      path: path_to_string(&file),
      source: source(&rel_path),
      rel_path,
      file_name: file
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default(),
    });
  }
  Ok(items)
}

fn folder_label(path: &Path) -> String {
  path
    .file_name()
    .map(|n| n.to_string_lossy().trim_start_matches('.').to_string())
    .filter(|label| !label.is_empty())
    .unwrap_or_else(|| "instruction".to_string())
}

/// Markdown files in the given files and folders. Suggested names are unique across the
/// result but may match existing entries, which import can overwrite on request.
pub fn detect_instruction_files(
  env: &Env,
  paths: &[String],
) -> Result<Vec<DetectedInstruction>, String> {
  let mut used: Vec<String> = Vec::new();
  let mut items = Vec::new();
  for raw in paths {
    let path = expand_home_with(raw.trim(), &env.home);
    let base = if path.is_file() {
      path.parent().map(Path::to_path_buf).unwrap_or(path.clone())
    } else {
      path.clone()
    };
    let label = folder_label(&base);
    items.extend(detect_in(env, &path, &base, &label, &mut used, |_| None)?);
  }
  Ok(items)
}

/// Download a repository (optionally a `/tree/` or `/blob/` path inside it) and list its
/// Markdown files. The archive stays in a temp directory until the import copies from it.
pub async fn detect_instruction_github(
  env: &Env,
  github_path: &str,
) -> Result<Vec<DetectedInstruction>, String> {
  let reference = GithubHelper::parse_github_ref(github_path)?;
  let clone_dir = create_temp_dir(&format!(
    "detect-instructions-{}-{}",
    reference.owner, reference.repo
  ))?;
  let branch = GithubHelper::clone_repo_to(
    &reference.owner,
    &reference.repo,
    reference.branch.as_deref(),
    &clone_dir,
  )
  .await?;
  let repo = format!("{}/{}", reference.owner, reference.repo);
  let root = match &reference.subpath {
    Some(subpath) => {
      let inside = clone_dir.join(subpath);
      if !inside.exists() {
        return Err(format!("{} was not found in the repository", subpath));
      }
      inside
    },
    None => clone_dir.clone(),
  };
  let mut used: Vec<String> = Vec::new();
  detect_in(env, &root, &clone_dir, &reference.repo, &mut used, |rel| {
    Some(InstructionSource::Github {
      repo: repo.clone(),
      file_path: rel.to_string(),
      branch: Some(branch.clone()),
    })
  })
}

// ---------------------------------------------------------------------------
// Editing
// ---------------------------------------------------------------------------

/// Save edited content to the library file and push it to the copy targets that were in
/// sync. Targets with local changes are reported as blockers and left alone.
pub fn write_instruction(
  env: &Env,
  name: &str,
  content: &str,
) -> Result<InstructionActionResult, String> {
  let _ops = ops_guard();
  get_record(env, name)?;
  let hub_file = env.instruction_file(name);
  fs::create_dir_all(&env.instructions_root).map_err(|e| e.to_string())?;
  let body = if content.is_empty() || content.ends_with('\n') {
    content.to_string()
  } else {
    format!("{}\n", content)
  };
  fs::write(&hub_file, body).map_err(|e| e.to_string())?;
  let hash = hash_file(&hub_file)?;
  let timestamp = now_rfc3339();
  update_lock(env, |lock| {
    let record = record_mut(lock, name)?;
    record.hash = hash.clone();
    record.updated_at = timestamp.clone();
    Ok(())
  })?;
  push_targets(env, name, None, false)
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
        source: None,
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
    assert!(matches!(view.source, InstructionSource::File { .. }));
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
        source: None,
      }],
      false,
    )
    .unwrap();
    let view = get_instruction(&env, "codex-rules").unwrap().unwrap();
    assert_eq!(
      view.source,
      InstructionSource::Agent {
        path: path_to_string(&source)
      }
    );
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
        source: None,
      }],
      false,
    )
    .is_err());
  }

  #[test]
  fn agent_files_are_adopted_on_list() {
    let tmp = tempfile::tempdir().unwrap();
    let env = env(tmp.path());
    let project = tmp.path().join("proj");
    write(&env.home.join(".claude/CLAUDE.md"), "# Claude\n");
    write(&project.join("AGENTS.md"), "# Proj\n");
    // Same content as the user-level file: registered, not imported twice.
    write(&project.join("CLAUDE.md"), "# Claude\n");

    let list = list_instructions(&env).unwrap();
    let names: Vec<&str> = list.iter().map(|item| item.name.as_str()).collect();
    assert_eq!(names, vec!["proj-agents", "user-claude"]);
    let claude = list.iter().find(|item| item.name == "user-claude").unwrap();
    assert_eq!(
      claude.source,
      InstructionSource::Agent {
        path: path_to_string(&env.home.join(".claude/CLAUDE.md"))
      }
    );
    assert_eq!(claude.installs.len(), 2);
    assert!(claude
      .installs
      .iter()
      .all(|install| install.state == TargetState::InSync));
    let agents = list.iter().find(|item| item.name == "proj-agents").unwrap();
    assert_eq!(agents.installs.len(), 1);
    assert_eq!(agents.installs[0].record.agent_ids, vec!["agents", "codex"]);
    assert!(project.join("AGENTS.md").is_file(), "files stay in place");

    // Listing again changes nothing; an edited file drifts instead of being re-imported.
    write(&project.join("AGENTS.md"), "# Proj edited\n");
    let list = list_instructions(&env).unwrap();
    assert_eq!(list.len(), 2);
    let agents = list.iter().find(|item| item.name == "proj-agents").unwrap();
    assert_eq!(agents.installs[0].state, TargetState::Modified);
  }

  #[cfg(unix)]
  #[test]
  fn adoption_skips_symlinks_unless_they_point_into_the_library() {
    let tmp = tempfile::tempdir().unwrap();
    let env = env(tmp.path());
    let dotfiles = tmp.path().join("dotfiles/AGENTS.md");
    write(&dotfiles, "# Dotfiles\n");
    let codex = env.home.join(".codex/AGENTS.md");
    fs::create_dir_all(codex.parent().unwrap()).unwrap();
    std::os::unix::fs::symlink(&dotfiles, &codex).unwrap();
    import(&env, tmp.path(), "rules", "# Rules\n");
    let claude = env.home.join(".claude/CLAUDE.md");
    fs::create_dir_all(claude.parent().unwrap()).unwrap();
    std::os::unix::fs::symlink(env.instruction_file("rules"), &claude).unwrap();

    let list = list_instructions(&env).unwrap();
    assert_eq!(list.len(), 1, "the dotfiles link is not imported");
    assert_eq!(list[0].installs.len(), 1);
    assert_eq!(list[0].installs[0].record.mode, InstallMode::Symlink);
    assert_eq!(list[0].installs[0].record.agent_ids, vec!["claude-code"]);
    assert!(is_symlink(&codex));
  }

  #[test]
  fn detect_lists_markdown_files_with_suggested_names() {
    let tmp = tempfile::tempdir().unwrap();
    let env = env(tmp.path());
    let repo = tmp.path().join("repo");
    write(&repo.join("AGENTS.md"), "a\n");
    write(&repo.join("README.md"), "r\n");
    write(&repo.join("packages/x/CLAUDE.md"), "c\n");
    write(&repo.join("prompts/python.md"), "p\n");
    write(&repo.join(".github/copilot-instructions.md"), "g\n");
    write(&repo.join("node_modules/dep/AGENTS.md"), "n\n");
    write(&repo.join(".venv/lib/notes.md"), "v\n");
    write(&repo.join("src/main.rs"), "fn main() {}\n");

    let items = detect_instruction_files(&env, &[path_to_string(&repo)]).unwrap();
    let rel: Vec<&str> = items.iter().map(|item| item.rel_path.as_str()).collect();
    assert_eq!(
      rel,
      vec![
        "AGENTS.md",
        "README.md",
        ".github/copilot-instructions.md",
        "prompts/python.md",
        "packages/x/CLAUDE.md",
      ]
    );
    let names: Vec<&str> = items.iter().map(|item| item.name.as_str()).collect();
    assert_eq!(
      names,
      vec![
        "repo-agents",
        "repo-readme",
        "copilot-instructions",
        "python",
        "x-claude"
      ]
    );

    let single =
      detect_instruction_files(&env, &[path_to_string(&repo.join("prompts/python.md"))]).unwrap();
    assert_eq!(single.len(), 1);
    assert_eq!(single[0].name, "python");
    assert_eq!(single[0].rel_path, "python.md");
    assert!(detect_instruction_files(&env, &[path_to_string(&repo.join("missing"))]).is_err());
  }

  #[test]
  fn writing_content_updates_the_hash_and_pushes_copies() {
    let tmp = tempfile::tempdir().unwrap();
    let env = env(tmp.path());
    import(&env, tmp.path(), "rules", "v1\n");
    let target = env.home.join(".claude/CLAUDE.md");
    assert!(
      install(
        &env,
        "rules",
        vec![spec(InstallScope::User, None, "claude-code")],
        InstallMode::Copy,
        false
      )
      .applied
    );

    let result = write_instruction(&env, "rules", "v2").unwrap();
    assert!(result.applied);
    let view = result.instruction.unwrap();
    assert_eq!(view.hub_state, HubState::Ok);
    assert_eq!(view.installs[0].state, TargetState::InSync);
    assert_eq!(read(&target), "v2\n");
    assert_eq!(read_instruction(&env, "rules").unwrap(), "v2\n");

    // A target edited by hand is reported, not overwritten.
    write(&target, "mine\n");
    let result = write_instruction(&env, "rules", "v3\n").unwrap();
    assert!(!result.applied);
    assert!(result.blockers[0].contains("local changes"));
    assert_eq!(read(&target), "mine\n");
    assert_eq!(read_instruction(&env, "rules").unwrap(), "v3\n");
  }

  #[cfg(unix)]
  #[test]
  fn create_symlink_and_remove_keeping_installs() {
    let tmp = tempfile::tempdir().unwrap();
    let env = env(tmp.path());
    let outcome = create_instruction(&env, "fresh", "").unwrap();
    assert_eq!(read(Path::new(&outcome.hub_path)), "# fresh\n");
    assert_eq!(
      get_instruction(&env, "fresh").unwrap().unwrap().source,
      InstructionSource::None
    );
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
  fn records_without_source_take_their_install_location() {
    let tmp = tempfile::tempdir().unwrap();
    let env = env(tmp.path());
    let outcome = create_instruction(&env, "old", "# Old\n").unwrap();
    let target = env.home.join(".claude/CLAUDE.md");
    assert!(
      install(
        &env,
        "old",
        vec![spec(InstallScope::User, None, "claude-code")],
        InstallMode::Copy,
        false
      )
      .applied
    );
    let list = list_instructions(&env).unwrap();
    let old = list.iter().find(|item| item.name == "old").unwrap();
    assert_eq!(old.hub_path, outcome.hub_path);
    assert_eq!(
      old.source,
      InstructionSource::Agent {
        path: path_to_string(&target)
      }
    );
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
