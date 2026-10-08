use crate::output::{Ctx, EXIT_BLOCKED, EXIT_OK};
use youskill_core::services::env::Env;
use youskill_core::services::hub_service;

#[derive(clap::Args)]
pub struct Args {
  /// Skill names as listed by `youskill list`.
  #[arg(required = true)]
  names: Vec<String>,
  /// Leave copy targets on disk as unmanaged skills. Symlink targets are turned into
  /// copies first, because the hub copy they point to goes away.
  #[arg(long)]
  keep_targets: bool,
}

pub fn run(ctx: &Ctx, args: Args) -> Result<i32, String> {
  let env = Env::current()?;
  for name in &args.names {
    let skill = super::skill_or_error(&env, name)?;
    let question = if args.keep_targets || skill.installs.is_empty() {
      format!("Remove '{}' from the hub?", name)
    } else {
      format!(
        "Remove '{}' from the hub and delete its {} install target(s)?",
        name,
        skill.installs.len()
      )
    };
    if !ctx.confirm(&question) {
      return Ok(EXIT_BLOCKED);
    }
    hub_service::remove_hub_skill(&env, name, !args.keep_targets)?;
    ctx.say(&format!("Removed '{}'", name));
  }
  if ctx.json {
    println!("{}", serde_json::json!({ "removed": args.names }));
  }
  Ok(EXIT_OK)
}
