pub mod agents;
pub mod diff;
pub mod import;
pub mod install;
pub mod instructions;
pub mod list;
pub mod projects;
pub mod remove;
pub mod scan;
pub mod show;
pub mod status;
pub mod sync;
pub mod uninstall;
pub mod update;

use std::future::Future;
use std::path::{Path, PathBuf};
use std::time::Duration;
use youskill_core::models::{HubSkillView, InstallScope, InstallTargetSpec, TargetState};
use youskill_core::services::env::Env;
use youskill_core::services::{hub_service, migration_service};
use youskill_core::utils::folder::sweep_temp_dirs;
use youskill_core::utils::path::path_to_string;

/// The skill list as the app builds it: legacy layouts are migrated first and the trash
/// and temp directories are swept.
pub fn hub_skills(env: &Env) -> Result<Vec<HubSkillView>, String> {
  migration_service::ensure_migrated(env)?;
  hub_service::sweep_trash(env);
  sweep_temp_dirs(Duration::from_secs(24 * 60 * 60));
  hub_service::list_hub_skills(env)
}

pub fn skill_or_error(env: &Env, name: &str) -> Result<HubSkillView, String> {
  hub_service::get_hub_skill(env, name)?
    .ok_or_else(|| format!("Skill '{}' is not in the hub", name))
}

/// `3 in sync, 1 outdated`: the install targets of a skill grouped by state.
pub fn targets_summary(skill: &HubSkillView) -> String {
  if skill.installs.is_empty() {
    return "-".to_string();
  }
  let order = [
    TargetState::InSync,
    TargetState::Outdated,
    TargetState::Modified,
    TargetState::Conflict,
    TargetState::Missing,
    TargetState::BrokenLink,
  ];
  let parts: Vec<String> = order
    .iter()
    .filter_map(|state| {
      let count = skill
        .installs
        .iter()
        .filter(|install| install.state == *state)
        .count();
      (count > 0).then(|| format!("{} {}", count, super::output::state_str(state)))
    })
    .collect();
  parts.join(", ")
}

/// Run one async service call. The core is synchronous apart from GitHub access, so a
/// runtime is only built when a command needs the network.
pub fn block_on<F: Future>(future: F) -> Result<F::Output, String> {
  let runtime = tokio::runtime::Builder::new_multi_thread()
    .enable_all()
    .build()
    .map_err(|e| e.to_string())?;
  Ok(runtime.block_on(future))
}

/// A path argument made absolute against the current directory, without requiring that it
/// exists.
pub fn absolute(path: &str) -> Result<PathBuf, String> {
  let path = Path::new(path.trim());
  if path.is_absolute() {
    return Ok(path.to_path_buf());
  }
  let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
  Ok(cwd.join(path))
}

/// Where an install goes: the user level, or a project given by `-p [path]`.
pub struct Scope {
  pub scope: InstallScope,
  pub project_path: Option<String>,
}

impl Scope {
  pub fn from_project_flag(project: Option<&str>) -> Result<Scope, String> {
    match project {
      None => Ok(Scope {
        scope: InstallScope::User,
        project_path: None,
      }),
      Some(path) => {
        let dir = absolute(path)?;
        if !dir.is_dir() {
          return Err(format!(
            "Project directory does not exist: {}",
            dir.display()
          ));
        }
        Ok(Scope {
          scope: InstallScope::Project,
          project_path: Some(path_to_string(&dir)),
        })
      },
    }
  }

  pub fn label(&self) -> String {
    match &self.project_path {
      Some(path) => path.clone(),
      None => "user level".to_string(),
    }
  }

  pub fn targets(&self, agent_ids: &[String]) -> Vec<InstallTargetSpec> {
    agent_ids
      .iter()
      .map(|agent_id| InstallTargetSpec {
        scope: self.scope,
        project_path: self.project_path.clone(),
        agent_id: agent_id.clone(),
      })
      .collect()
  }
}

/// What an install writes: a skill directory or an agent instruction file.
#[derive(Clone, Copy)]
pub enum Unit {
  Skill,
  Instruction,
}

/// The agent ids of `-a`: explicit ids are checked against the detected apps, `all` means
/// every detected app that has a location for this unit in this scope.
pub fn resolve_agents(
  env: &Env,
  requested: &[String],
  scope: &Scope,
  unit: Unit,
) -> Result<Vec<String>, String> {
  if requested.iter().any(|id| id == "all") {
    let ids: Vec<String> = env
      .agent_apps
      .iter()
      .filter(|app| match (unit, scope.scope) {
        (Unit::Skill, InstallScope::User) => app.global_path.is_some(),
        (Unit::Skill, InstallScope::Project) => app.project_path.is_some(),
        (Unit::Instruction, InstallScope::User) => app.global_profile_path.is_some(),
        (Unit::Instruction, InstallScope::Project) => app.profile_path.is_some(),
      })
      .map(|app| app.id.clone())
      .collect();
    if ids.is_empty() {
      return Err("No agent apps detected on this machine".to_string());
    }
    return Ok(ids);
  }
  let mut ids = Vec::new();
  for id in requested {
    let id = id.trim();
    if id.is_empty() {
      continue;
    }
    if env.agent(id).is_none() {
      return Err(format!(
        "Unknown or undetected agent app '{}'; see `youskill agents --all`",
        id
      ));
    }
    if !ids.iter().any(|known| known == id) {
      ids.push(id.to_string());
    }
  }
  if ids.is_empty() {
    return Err("At least one agent id is required (-a <id>,... or -a all)".to_string());
  }
  Ok(ids)
}
