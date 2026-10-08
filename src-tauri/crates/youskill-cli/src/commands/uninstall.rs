use crate::output::{report_action, Ctx, EXIT_OK};
use youskill_core::models::{InstallScope, UninstallRequest};
use youskill_core::services::env::Env;
use youskill_core::services::install_service;
use youskill_core::utils::path::same_path;

#[derive(clap::Args)]
pub struct Args {
  /// Skill names as listed by `youskill list`.
  #[arg(required = true)]
  names: Vec<String>,
  /// Agent ids to remove the skill from (comma separated), or `all` for every agent that
  /// has it in this scope.
  #[arg(short, long, value_delimiter = ',', required = true)]
  agents: Vec<String>,
  /// Uninstall from a project instead of the user level. Without a value the current
  /// directory is used; put the flag last or write `--project=<path>`.
  #[arg(short, long, num_args = 0..=1, default_missing_value = ".")]
  project: Option<String>,
  /// Delete a target directory even when it has local changes.
  #[arg(long)]
  force: bool,
}

pub fn run(ctx: &Ctx, args: Args) -> Result<i32, String> {
  let env = Env::current()?;
  let scope = super::Scope::from_project_flag(args.project.as_deref())?;
  let mut code = EXIT_OK;
  for name in &args.names {
    let skill = super::skill_or_error(&env, name)?;
    // `all` means every agent recorded in this scope, whether or not it is still detected.
    let agent_ids: Vec<String> = if args.agents.iter().any(|id| id == "all") {
      let mut ids: Vec<String> = skill
        .installs
        .iter()
        .filter(|install| install.record.scope == scope.scope)
        .filter(
          |install| match (&install.record.project_path, &scope.project_path) {
            (Some(a), Some(b)) => same_path(std::path::Path::new(a), std::path::Path::new(b)),
            (None, None) => true,
            _ => false,
          },
        )
        .flat_map(|install| install.record.agent_ids.clone())
        .collect();
      ids.sort();
      ids.dedup();
      ids
    } else {
      args.agents.clone()
    };
    if agent_ids.is_empty() {
      ctx.say(&format!("'{}' is not installed at {}", name, scope.label()));
      continue;
    }
    let result = install_service::uninstall_skill(
      &env,
      UninstallRequest {
        name: name.clone(),
        targets: scope.targets(&agent_ids),
        paths: Vec::new(),
        force: args.force,
      },
    )?;
    let label = format!(
      "Uninstalled '{}' from {} ({})",
      name,
      match scope.scope {
        InstallScope::User => "user level".to_string(),
        InstallScope::Project => scope.label(),
      },
      agent_ids.join(",")
    );
    code = code.max(report_action(ctx, &label, &result)?);
  }
  Ok(code)
}
