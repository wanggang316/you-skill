use crate::output::{report_action, Ctx};
use youskill_core::models::{SkillSource, SyncAction};
use youskill_core::services::env::Env;
use youskill_core::services::lock_service::store;
use youskill_core::services::{hub_service, source_service};

#[derive(Clone, Copy, clap::ValueEnum)]
pub enum Action {
  /// Replace the hub copy with the source (folder or GitHub).
  PullSource,
  /// Write the hub copy to every copy target, or to the `--target` paths.
  PushTargets,
  /// Take the content of one `--target` into the hub.
  AdoptTarget,
  /// Record the hub's current content as the new baseline.
  AcceptHub,
  /// Mirror the hub copy back into a folder source.
  PushSource,
}

#[derive(clap::Args)]
pub struct Args {
  /// Skill name as listed by `youskill list`.
  name: String,
  #[arg(value_enum)]
  action: Action,
  /// Install target path(s) as shown by `youskill show`. Required for `adopt-target`;
  /// narrows `push-targets`.
  #[arg(long)]
  target: Vec<String>,
  /// Discard local changes that would otherwise block the action.
  #[arg(long)]
  force: bool,
}

pub fn run(ctx: &Ctx, args: Args) -> Result<i32, String> {
  let env = Env::current()?;
  super::skill_or_error(&env, &args.name)?;
  let force = args.force;
  let (action, label) = match args.action {
    Action::PullSource => (SyncAction::PullSource { force }, "pulled from source"),
    Action::PushTargets => (
      SyncAction::PushTargets {
        targets: (!args.target.is_empty()).then(|| args.target.clone()),
        force,
      },
      "pushed to targets",
    ),
    Action::AdoptTarget => {
      let path = match args.target.as_slice() {
        [path] => path.clone(),
        _ => return Err("adopt-target needs exactly one --target <path>".to_string()),
      };
      (SyncAction::AdoptTarget { path, force }, "adopted target")
    },
    Action::AcceptHub => (SyncAction::AcceptHub, "accepted hub content"),
    Action::PushSource => (SyncAction::PushSource { force }, "pushed to source"),
  };

  let result = match &action {
    // A GitHub source has to be downloaded first; the rest works offline.
    SyncAction::PullSource { force } if is_github(&env, &args.name)? => super::block_on(
      source_service::pull_github_source(env.clone(), args.name.clone(), *force),
    )??,
    _ => hub_service::sync_skill(&env, &args.name, action)?,
  };
  report_action(ctx, &format!("'{}': {}", args.name, label), &result)
}

fn is_github(env: &Env, name: &str) -> Result<bool, String> {
  Ok(
    store(env)
      .get(name)?
      .map(|record| matches!(record.source, SkillSource::Github { .. }))
      .unwrap_or(false),
  )
}
