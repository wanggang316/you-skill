use crate::output::{report_action, Ctx, EXIT_OK};
use youskill_core::config::load_config;
use youskill_core::models::{InstallMode, InstallRequest, UserProject};
use youskill_core::services::env::Env;
use youskill_core::services::{install_service, user_projects_service};

#[derive(Clone, Copy, clap::ValueEnum)]
pub enum Mode {
  Copy,
  Symlink,
}

impl From<Mode> for InstallMode {
  fn from(mode: Mode) -> Self {
    match mode {
      Mode::Copy => InstallMode::Copy,
      Mode::Symlink => InstallMode::Symlink,
    }
  }
}

#[derive(clap::Args)]
pub struct Args {
  /// Skill names as listed by `youskill list`.
  #[arg(required = true)]
  names: Vec<String>,
  /// Agent ids (comma separated, see `youskill agents`), or `all` for every detected agent.
  #[arg(short, long, value_delimiter = ',', required = true)]
  agents: Vec<String>,
  /// Install into a project instead of the user level. Without a value the current
  /// directory is used; put the flag last or write `--project=<path>`.
  #[arg(short, long, num_args = 0..=1, default_missing_value = ".")]
  project: Option<String>,
  /// Copy the skill or link it to the hub. Defaults to the app's sync mode setting.
  #[arg(long, value_enum)]
  mode: Option<Mode>,
  /// Overwrite a target that has different content, or switch its install mode.
  #[arg(long)]
  force: bool,
}

pub fn run(ctx: &Ctx, args: Args) -> Result<i32, String> {
  let env = Env::current()?;
  let scope = super::Scope::from_project_flag(args.project.as_deref())?;
  let agent_ids = super::resolve_agents(&env, &args.agents, &scope)?;
  let mode = match args.mode {
    Some(mode) => InstallMode::from(mode),
    None => InstallMode::from_setting(&load_config()?.sync_mode),
  };
  if let Some(path) = &scope.project_path {
    // The app lists installs per registered project, so a project installed to from the
    // terminal is registered too.
    user_projects_service::add_user_projects(vec![UserProject {
      name: String::new(),
      path: path.clone(),
      workspace_path: None,
    }])?;
  }

  let mut code = EXIT_OK;
  for name in &args.names {
    let result = install_service::install_skill(
      &env,
      InstallRequest {
        name: name.clone(),
        targets: scope.targets(&agent_ids),
        mode: Some(mode),
        force: args.force,
      },
    )?;
    let label = format!(
      "Installed '{}' to {} ({})",
      name,
      scope.label(),
      agent_ids.join(",")
    );
    code = code.max(report_action(ctx, &label, &result)?);
  }
  Ok(code)
}
