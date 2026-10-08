pub mod agents;
pub mod diff;
pub mod list;
pub mod projects;
pub mod show;
pub mod status;

use std::time::Duration;
use youskill_core::models::{HubSkillView, TargetState};
use youskill_core::services::env::Env;
use youskill_core::services::{hub_service, migration_service};
use youskill_core::utils::folder::sweep_temp_dirs;

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
